#!/usr/bin/env python3
"""Validate the active DXBOT plan package without third-party packages."""
from __future__ import annotations
import argparse, ast, re, shutil, sys, tempfile
from dataclasses import dataclass
from datetime import date
from pathlib import Path

SEMVER=re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
DATE=re.compile(r"^\d{4}-\d{2}-\d{2}$")
KEBAB=re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+)?$")
LINK=re.compile(r"\[[^\]]*\]\(([^)]+)\)")
REQ={"title","document_id","version","status","normative","priority","last_updated","depends_on"}
STAT={"Accepted","Normative Baseline","Reference Source","Reference Snapshot","Accepted Template"}
PRI={"P0","P1","P2"}; ASTAT={"Specified","Executable","Passed","Blocked"}; EXITS={0,*range(2,19)}

@dataclass(frozen=True)
class Doc:
    path:Path; rel:Path; meta:dict[str,object]; text:str
class VError(Exception): pass

def scalar(raw:str)->object:
    v=raw.strip()
    if v in {"true","false"}: return v=="true"
    if v.startswith("[") and v.endswith("]"):
        try: x=ast.literal_eval(v)
        except (SyntaxError,ValueError) as e: raise VError(f"invalid frontmatter list: {v}") from e
        if not isinstance(x,list): raise VError(f"frontmatter list expected: {v}")
        return x
    if len(v)>1 and v[0]==v[-1]=='"': return v[1:-1]
    if v.isdigit(): return int(v)
    return v

def frontmatter(path:Path)->tuple[dict[str,object],str]:
    text=path.read_text(encoding="utf-8"); lines=text.splitlines()
    if not lines or lines[0].strip()!="---": raise VError(f"{path}: missing frontmatter")
    try: end=next(i for i in range(1,len(lines)) if lines[i].strip()=="---")
    except StopIteration as e: raise VError(f"{path}: unterminated frontmatter") from e
    meta={}
    for line in lines[1:end]:
        if not line.strip() or line.lstrip().startswith("#"): continue
        if ":" not in line: raise VError(f"{path}: malformed frontmatter: {line}")
        k,v=line.split(":",1); meta[k.strip()]=scalar(v)
    return meta,text

def semver(v:str)->tuple[int,int,int]:
    if not SEMVER.fullmatch(v): raise VError(f"invalid semantic version: {v}")
    return tuple(map(int,re.split(r"[-+]",v,maxsplit=1)[0].split("."))) # type: ignore[return-value]

def active(repo:Path)->Path:
    found=[]
    for r in repo.glob("docs/plan/*-detailed-development-plan/readme.md"):
        try: m,_=frontmatter(r)
        except VError: continue
        if m.get("document_id")=="DXB-INDEX" and m.get("status")=="Accepted": found.append((semver(str(m.get("version"))),r.parent))
    if not found: raise VError("no Accepted DXB-INDEX package found")
    top=max(v for v,_ in found); wins=[p for v,p in found if v==top]
    if len(wins)!=1: raise VError(f"multiple active packages for version {top}: {wins}")
    return wins[0]

def docs(pkg:Path)->list[Doc]:
    out=[]
    for p in sorted(pkg.rglob("*.md")):
        m,t=frontmatter(p); out.append(Doc(p,p.relative_to(pkg),m,t))
    return out

def block(text:str,start:str,end:str)->str:
    if start not in text or end not in text: raise VError(f"registry markers missing: {start}")
    return text.split(start,1)[1].split(end,1)[0]

def listrows(text:str,start:str,end:str)->list[tuple[str,...]]:
    return [tuple(c.strip().strip("`") for c in l.strip()[2:].split("|")) for l in block(text,start,end).splitlines() if l.strip().startswith("- `")]

def tablerows(text:str,start:str,end:str,prefix:str)->list[tuple[str,...]]:
    return [tuple(c.strip().strip("`") for c in l.strip().strip("|").split("|")) for l in block(text,start,end).splitlines() if l.strip().startswith(f"| {prefix}")]

def manifest(text:str)->dict[str,str]:
    out={}
    for l in block(text,"<!-- manifest-machine:start -->","<!-- manifest-machine:end -->").splitlines():
        if ":" in l: k,v=l.split(":",1); out[k.strip()]=v.strip()
    return out

def evidence(text:str,start:str,end:str)->int:
    return sum(l.strip().startswith("- `") for l in block(text,start,end).splitlines())

def validate(pkg:Path)->list[str]:
    err=[]
    try: ds=docs(pkg)
    except VError as e: return [str(e)]
    by={}
    for d in ds:
        if any(part not in {"readme.md","manifest.md"} and not KEBAB.fullmatch(part) for part in d.rel.parts): err.append(f"non-kebab path: {d.rel}")
        miss=REQ-set(d.meta)
        if miss: err.append(f"{d.rel}: missing frontmatter keys {sorted(miss)}")
        i=d.meta.get("document_id")
        if not isinstance(i,str) or not i: err.append(f"{d.rel}: missing document_id"); continue
        if i in by: err.append(f"duplicate document_id {i}: {by[i].rel}, {d.rel}")
        else: by[i]=d
        if d.meta.get("status") not in STAT: err.append(f"{d.rel}: invalid status")
        if d.meta.get("priority") not in PRI: err.append(f"{d.rel}: invalid priority")
        if not isinstance(d.meta.get("normative"),bool): err.append(f"{d.rel}: normative must be boolean")
        u=d.meta.get("last_updated")
        if not isinstance(u,str) or not DATE.fullmatch(u): err.append(f"{d.rel}: invalid last_updated")
        else:
            try: date.fromisoformat(u)
            except ValueError: err.append(f"{d.rel}: impossible last_updated")
        if d.meta.get("normative") is True and not SEMVER.fullmatch(str(d.meta.get("version"))): err.append(f"{d.rel}: normative version must be semver")
    graph={}
    for i,d in by.items():
        dep=d.meta.get("depends_on",[])
        if not isinstance(dep,list) or not all(isinstance(x,str) for x in dep): err.append(f"{d.rel}: depends_on must be string list"); dep=[]
        graph[i]=dep
        for x in dep:
            if x not in by: err.append(f"{d.rel}: missing dependency {x}")
    state={}; stack=[]
    def visit(n:str):
        state[n]=1; stack.append(n)
        for x in graph.get(n,[]):
            if x not in graph: continue
            if state.get(x)==1: err.append("dependency cycle: "+" -> ".join(stack[stack.index(x):]+[x]))
            elif not state.get(x): visit(x)
        stack.pop(); state[n]=2
    for n in graph:
        if not state.get(n): visit(n)
    idx=by.get("DXB-INDEX"); man=by.get("DXB-MANIFEST"); src=by.get("DXB-SOURCE-000"); base=by.get("DXB-BASE-000")
    api=by.get("DXB-IFC-040"); cli=by.get("DXB-IFC-041"); inp=by.get("DXB-IFC-042"); acc=by.get("DXB-DEL-061"); risk=by.get("DXB-DEL-062")
    if not idx: err.append("DXB-INDEX missing")
    else:
        dec=idx.meta.get("package_path")
        if not isinstance(dec,str) or not pkg.as_posix().endswith(dec): err.append(f"package_path mismatch: {dec!r}")
    if not src or src.meta.get("normative") is not False: err.append("DXB-SOURCE-000 must exist and be normative:false")
    if not man: err.append("DXB-MANIFEST missing")
    else:
        try:
            mm=manifest(man.text); expected={"active_package_path":idx.meta.get("package_path") if idx else None,"plan_version":idx.meta.get("version") if idx else None,"review_revision":str(idx.meta.get("review_revision")) if idx else None,"adversarial_review_rounds":str(idx.meta.get("adversarial_review_rounds")) if idx else None,"final_rechecks":str(idx.meta.get("final_rechecks")) if idx else None,"markdown_count":str(len(ds))}
            for k,v in expected.items():
                if mm.get(k)!=v: err.append(f"manifest {k} drift: {mm.get(k)!r} != {v!r}")
            r=evidence(man.text,"<!-- review-evidence:start -->","<!-- review-evidence:end -->"); c=evidence(man.text,"<!-- final-recheck:start -->","<!-- final-recheck:end -->")
            if r!=int(mm.get("adversarial_review_rounds","-1")): err.append("review evidence count drift")
            if c!=int(mm.get("final_rechecks","-1")): err.append("final recheck count drift")
        except (VError,ValueError) as e: err.append(str(e))
    if base:
        try:
            rows=listrows(base.text,"<!-- canonical-owner-registry:start -->","<!-- canonical-owner-registry:end -->")
            if any(len(r)!=5 for r in rows): err.append("canonical owner registry row arity must be 5")
            keys=[r[0] for r in rows if len(r)==5]
            if len(keys)!=len(set(keys)): err.append("duplicate canonical owner key")
            for r in rows:
                if len(r)==5 and r[1] not in by: err.append(f"canonical owner document missing: {r[1]}")
        except VError as e: err.append(str(e))
    else: err.append("DXB-BASE-000 missing")
    if api:
        try:
            rows=listrows(api.text,"<!-- exit-registry:start -->","<!-- exit-registry:end -->")
            if any(len(r)!=3 for r in rows): err.append("exit registry row arity must be 3")
            codes={int(r[0]) for r in rows if len(r)==3 and r[0].isdigit()}
            if codes!=EXITS: err.append(f"exit registry mismatch: {sorted(codes)}")
        except (VError,ValueError) as e: err.append(str(e))
    else: err.append("DXB-IFC-040 missing")
    cr=[]; ir=[]
    if cli and inp:
        try:
            cr=listrows(cli.text,"<!-- p0-command-registry:start -->","<!-- p0-command-registry:end -->"); ir=listrows(inp.text,"<!-- p0-input-registry:start -->","<!-- p0-input-registry:end -->")
            if any(len(r)!=9 for r in cr): err.append("command registry row arity must be 9")
            if any(len(r)!=5 for r in ir): err.append("input registry row arity must be 5")
            cm={r[0]:r for r in cr if len(r)==9}; im={r[0]:r for r in ir if len(r)==5}
            if len(cm)!=sum(len(r)==9 for r in cr): err.append("duplicate command key")
            if len(im)!=sum(len(r)==5 for r in ir): err.append("duplicate input command key")
            if set(cm)!=set(im): err.append(f"command/input key mismatch missing={sorted(set(cm)-set(im))} extra={sorted(set(im)-set(cm))}")
            for k in set(cm)&set(im):
                if cm[k][4]!=im[k][1]: err.append(f"input schema drift for {k}")
                if cm[k][8]!=im[k][4]: err.append(f"Acceptance drift for {k}")
                if not im[k][3]: err.append(f"empty typed fields for {k}")
        except VError as e: err.append(str(e))
    else: err.append("CLI command/input owners missing")
    aids=set()
    if acc:
        try:
            rows=tablerows(acc.text,"<!-- acceptance-registry:start -->","<!-- acceptance-registry:end -->","AT-")
            if any(len(r)!=9 for r in rows): err.append("acceptance registry row arity must be 9")
            for r in rows:
                if len(r)==9:
                    aids.add(r[0])
                    if r[5] not in ASTAT: err.append(f"invalid Acceptance status {r[0]}={r[5]}")
                    if not all(r[i] for i in (6,7,8)): err.append(f"Acceptance evidence fields empty: {r[0]}")
        except VError as e: err.append(str(e))
    else: err.append("DXB-DEL-061 missing")
    for r in cr:
        if len(r)==9 and r[8] not in aids: err.append(f"command Acceptance orphan: {r[8]}")
    if risk:
        try:
            rows=tablerows(risk.text,"<!-- risk-registry:start -->","<!-- risk-registry:end -->","R-")
            if any(len(r)!=8 for r in rows): err.append("risk registry row arity must be 8")
            for r in rows:
                if len(r)==8 and not all(r[i] for i in (5,6,7)): err.append(f"Risk status/evidence/blocker empty: {r[0]}")
        except VError as e: err.append(str(e))
    else: err.append("DXB-DEL-062 missing")
    for d in ds:
        for raw in LINK.findall(d.text):
            t=raw.strip()
            if not t or t.startswith(("#","http://","https://","mailto:")): continue
            t=t.split("#",1)[0]
            if not t: continue
            p=(d.path.parent/t).resolve()
            try: p.relative_to(pkg.resolve())
            except ValueError: err.append(f"{d.rel}: link escapes package: {raw}"); continue
            if not p.exists(): err.append(f"{d.rel}: broken link: {raw}")
    return err

def selftest(repo:Path)->None:
    src=active(repo)
    with tempfile.TemporaryDirectory(prefix="dxbot-plan-validator-") as td:
        pkg=Path(td)/"docs/plan"/src.name; pkg.parent.mkdir(parents=True); shutil.copytree(src,pkg)
        if (e:=validate(pkg)): raise VError(f"active package failed self-test baseline: {e}")
        dup=pkg/"00-governance/99-duplicate.md"; dup.write_text((pkg/"00-governance/00-normative-baseline.md").read_text(),encoding="utf-8")
        if not any("duplicate document_id" in x for x in validate(pkg)): raise VError("duplicate-ID fixture did not fail")
        dup.unlink()
        b=pkg/"00-governance/00-normative-baseline.md"; s=b.read_text(); b.write_text(s.replace("depends_on: []",'depends_on: ["DXB-INDEX"]',1))
        if not any("dependency cycle" in x for x in validate(pkg)): raise VError("cycle fixture did not fail")
        b.write_text(s)
        i=pkg/"40-interfaces/42-cli-input-contract.md"; s=i.read_text(); line=next(x for x in s.splitlines() if x.startswith("- `runtime-start`")); i.write_text(s.replace(line+"\n","",1))
        if not any("command/input key mismatch" in x for x in validate(pkg)): raise VError("input fixture did not fail")
        i.write_text(s)
        c=pkg/"40-interfaces/41-cli.md"; s=c.read_text(); line=next(x for x in s.splitlines() if x.startswith("- `runtime-start`")); c.write_text(s.replace(line,line.rsplit(" | ",1)[0],1))
        if not any("command registry row arity" in x for x in validate(pkg)): raise VError("command fixture did not fail")
        c.write_text(s)
        m=pkg/"manifest.md"; s=m.read_text(); m.write_text(s.replace("markdown_count: 49","markdown_count: 999",1))
        if not any("markdown_count" in x for x in validate(pkg)): raise VError("manifest fixture did not fail")
        m.write_text(s.replace("- `R5-FLOW`","- R5-FLOW",1))
        if not any("review evidence count drift" in x for x in validate(pkg)): raise VError("review fixture did not fail")
        m.write_text(s)
        a=pkg/"40-interfaces/40-api-protocols.md"; s=a.read_text(); line=next(x for x in s.splitlines() if x.startswith("- `18`")); a.write_text(s.replace(line+"\n","",1))
        if not any("exit registry mismatch" in x for x in validate(pkg)): raise VError("exit fixture did not fail")
    print("plan-validator self-test: PASS")

def main()->int:
    p=argparse.ArgumentParser(); p.add_argument("--active",action="store_true"); p.add_argument("--self-test",action="store_true"); p.add_argument("--repo",default="."); a=p.parse_args()
    if not a.active and not a.self_test: p.error("select --active and/or --self-test")
    repo=Path(a.repo).resolve()
    try:
        if a.active:
            pkg=active(repo); e=validate(pkg)
            if e:
                for x in e: print(f"ERROR: {x}",file=sys.stderr)
                return 1
            print(f"active plan: {pkg.relative_to(repo)}"); print(f"markdown documents: {len(list(pkg.rglob('*.md')))}"); print("plan-validator active validation: PASS")
        if a.self_test: selftest(repo)
    except VError as e: print(f"ERROR: {e}",file=sys.stderr); return 1
    return 0
if __name__=="__main__": raise SystemExit(main())
