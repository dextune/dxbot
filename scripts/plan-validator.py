#!/usr/bin/env python3
from __future__ import annotations
import argparse, ast, re, shutil, sys, tempfile
from dataclasses import dataclass
from datetime import date
from pathlib import Path

SEMVER=re.compile(r'^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')
DATE=re.compile(r'^\d{4}-\d{2}-\d{2}$')
KEBAB=re.compile(r'^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+)?$')
LINK=re.compile(r'\[[^\]]*\]\(([^)]+)\)')
FIELD=re.compile(r'^[a-z0-9_]+:[A-Za-z0-9]+[!?*+]@(?:argv|stdin|file|artifact|local|oneof\{[^}]+\})(?:\{[^}]+\})?(?:=[A-Za-z0-9_-]+)?$')
REQ={'title','document_id','version','status','normative','priority','last_updated','depends_on'}
STAT={'Accepted','Normative Baseline','Reference Source','Reference Snapshot','Accepted Template'}
PRI={'P0','P1','P2'}; ASTAT={'Specified','Executable','Passed','Blocked'}
EXITS={0,*range(2,19)}; APPLIED={'task-cancel','task-suspend','task-resume','task-redirect'}
LOCAL={'ready_at','timeout','all','output','confirmation'}
MILES=('M0A','M1A','M1B','M2','M3','M4','M5','M6'); MORDER={v:i for i,v in enumerate(MILES)}
INVS={f'INV-{i:03d}' for i in range(1,16)}; ADRS={f'ADR-{i:04d}' for i in range(95,141)}

@dataclass(frozen=True)
class Doc:
    path:Path; rel:Path; meta:dict[str,object]; text:str
class VError(Exception): pass

def scalar(raw):
    v=raw.strip()
    if v in {'true','false'}: return v=='true'
    if v.startswith('[') and v.endswith(']'):
        try: x=ast.literal_eval(v)
        except (SyntaxError,ValueError) as e: raise VError(f'invalid frontmatter list: {v}') from e
        if not isinstance(x,list): raise VError(f'frontmatter list expected: {v}')
        return x
    if len(v)>1 and v[0]==v[-1]=='"': return v[1:-1]
    if v.isdigit(): return int(v)
    return v

def frontmatter(path):
    text=path.read_text(encoding='utf-8'); lines=text.splitlines()
    if not lines or lines[0].strip()!='---': raise VError(f'{path}: missing frontmatter')
    try: end=next(i for i in range(1,len(lines)) if lines[i].strip()=='---')
    except StopIteration as e: raise VError(f'{path}: unterminated frontmatter') from e
    meta={}
    for line in lines[1:end]:
        if not line.strip() or line.lstrip().startswith('#'): continue
        if ':' not in line: raise VError(f'{path}: malformed frontmatter: {line}')
        k,v=line.split(':',1); meta[k.strip()]=scalar(v)
    return meta,text

def semver(v):
    if not SEMVER.fullmatch(v): raise VError(f'invalid semantic version: {v}')
    return tuple(map(int,re.split(r'[-+]',v,maxsplit=1)[0].split('.')))

def active(repo):
    found=[]
    for readme in repo.glob('docs/plan/*-detailed-development-plan/readme.md'):
        try: meta,_=frontmatter(readme)
        except VError: continue
        if meta.get('document_id')=='DXB-INDEX' and meta.get('status')=='Accepted':
            found.append((semver(str(meta.get('version'))),readme.parent))
    if not found: raise VError('no Accepted DXB-INDEX package found')
    top=max(v for v,_ in found); wins=[p for v,p in found if v==top]
    if len(wins)!=1: raise VError(f'multiple active packages for version {top}: {wins}')
    return wins[0]

def docs(pkg):
    out=[]
    for path in sorted(pkg.rglob('*.md')):
        meta,text=frontmatter(path); out.append(Doc(path,path.relative_to(pkg),meta,text))
    return out
def block(text,start,end):
    if start not in text or end not in text: raise VError(f'registry markers missing: {start}')
    return text.split(start,1)[1].split(end,1)[0]
def listrows(text,start,end):
    return [tuple(c.strip().strip('`') for c in l.strip()[2:].split('|'))
            for l in block(text,start,end).splitlines() if l.strip().startswith('- `')]
def tablerows(text,start,end,prefix):
    return [tuple(c.strip().strip('`') for c in l.strip().strip('|').split('|'))
            for l in block(text,start,end).splitlines() if l.strip().startswith(f'| {prefix}')]
def manifest(text):
    out={}
    for l in block(text,'<!-- manifest-machine:start -->','<!-- manifest-machine:end -->').splitlines():
        if ':' in l: k,v=l.split(':',1); out[k.strip()]=v.strip()
    return out
def evidence(text,start,end):
    return sum(l.strip().startswith('- `') for l in block(text,start,end).splitlines())

def validate(pkg):
    err=[]
    try: ds=docs(pkg)
    except VError as e: return [str(e)]
    by={}
    for d in ds:
        if any(p not in {'readme.md','manifest.md'} and not KEBAB.fullmatch(p) for p in d.rel.parts):
            err.append(f'non-kebab path: {d.rel}')
        miss=REQ-set(d.meta)
        if miss: err.append(f'{d.rel}: missing frontmatter keys {sorted(miss)}')
        ident=d.meta.get('document_id')
        if not isinstance(ident,str) or not ident: err.append(f'{d.rel}: missing document_id'); continue
        if ident in by: err.append(f'duplicate document_id {ident}: {by[ident].rel}, {d.rel}')
        else: by[ident]=d
        if d.meta.get('status') not in STAT: err.append(f'{d.rel}: invalid status')
        if d.meta.get('priority') not in PRI: err.append(f'{d.rel}: invalid priority')
        if not isinstance(d.meta.get('normative'),bool): err.append(f'{d.rel}: normative must be boolean')
        updated=d.meta.get('last_updated')
        if not isinstance(updated,str) or not DATE.fullmatch(updated): err.append(f'{d.rel}: invalid last_updated')
        else:
            try: date.fromisoformat(updated)
            except ValueError: err.append(f'{d.rel}: impossible last_updated')
        if d.meta.get('normative') is True and not SEMVER.fullmatch(str(d.meta.get('version'))):
            err.append(f'{d.rel}: normative version must be semver')

    graph={}
    for ident,d in by.items():
        dep=d.meta.get('depends_on',[])
        if not isinstance(dep,list) or not all(isinstance(x,str) for x in dep):
            err.append(f'{d.rel}: depends_on must be string list'); dep=[]
        graph[ident]=dep
        for x in dep:
            if x not in by: err.append(f'{d.rel}: missing dependency {x}')
    state={}; stack=[]
    def visit(node):
        state[node]=1; stack.append(node)
        for x in graph.get(node,[]):
            if x not in graph: continue
            if state.get(x)==1: err.append('dependency cycle: '+' -> '.join(stack[stack.index(x):]+[x]))
            elif not state.get(x): visit(x)
        stack.pop(); state[node]=2
    for node in graph:
        if not state.get(node): visit(node)

    idx=by.get('DXB-INDEX'); man=by.get('DXB-MANIFEST'); src=by.get('DXB-SOURCE-000')
    base=by.get('DXB-BASE-000'); adr=by.get('DXB-GOV-003'); api=by.get('DXB-IFC-040')
    cli=by.get('DXB-IFC-041'); inp=by.get('DXB-IFC-042'); road=by.get('DXB-DEL-060')
    acc=by.get('DXB-DEL-061'); risk=by.get('DXB-DEL-062')

    if not idx: err.append('DXB-INDEX missing')
    else:
        declared=idx.meta.get('package_path')
        if not isinstance(declared,str) or not pkg.as_posix().endswith(declared):
            err.append(f'package_path mismatch: {declared!r}')
    if not src or src.meta.get('normative') is not False:
        err.append('DXB-SOURCE-000 must exist and be normative:false')

    if man:
        try:
            mm=manifest(man.text)
            exp={'active_package_path':idx.meta.get('package_path') if idx else None,
                 'plan_version':idx.meta.get('version') if idx else None,
                 'review_revision':str(idx.meta.get('review_revision')) if idx else None,
                 'adversarial_review_rounds':str(idx.meta.get('adversarial_review_rounds')) if idx else None,
                 'final_rechecks':str(idx.meta.get('final_rechecks')) if idx else None,
                 'markdown_count':str(len(ds))}
            for k,v in exp.items():
                if mm.get(k)!=v: err.append(f'manifest {k} drift: {mm.get(k)!r} != {v!r}')
            if evidence(man.text,'<!-- review-evidence:start -->','<!-- review-evidence:end -->')!=int(mm.get('adversarial_review_rounds','-1')):
                err.append('review evidence count drift')
            if evidence(man.text,'<!-- final-recheck:start -->','<!-- final-recheck:end -->')!=int(mm.get('final_rechecks','-1')):
                err.append('final recheck count drift')
        except (VError,ValueError) as e: err.append(str(e))
    else: err.append('DXB-MANIFEST missing')

    if base:
        try:
            inv={r[0] for r in listrows(base.text,'<!-- product-invariants:start -->','<!-- product-invariants:end -->') if len(r)==2}
            if inv!=INVS: err.append(f'product invariant coverage mismatch: {sorted(inv)}')
            owners=listrows(base.text,'<!-- canonical-owner-registry:start -->','<!-- canonical-owner-registry:end -->')
            if any(len(r)!=5 for r in owners): err.append('canonical owner registry row arity must be 5')
            keys=[r[0] for r in owners if len(r)==5]
            if len(keys)!=len(set(keys)): err.append('duplicate canonical owner key')
            for r in owners:
                if len(r)==5 and r[1] not in by: err.append(f'canonical owner document missing: {r[1]}')
        except VError as e: err.append(str(e))
    else: err.append('DXB-BASE-000 missing')

    if adr:
        try:
            got={r[0] for r in listrows(adr.text,'<!-- architecture-decision-registry:start -->','<!-- architecture-decision-registry:end -->') if len(r)==2}
            if got!=ADRS: err.append(f'architecture decision coverage mismatch: {sorted(got)}')
        except VError as e: err.append(str(e))
    else: err.append('DXB-GOV-003 missing')

    if api:
        try:
            rows=listrows(api.text,'<!-- exit-registry:start -->','<!-- exit-registry:end -->')
            codes={int(r[0]) for r in rows if len(r)==3 and r[0].isdigit()}
            if any(len(r)!=3 for r in rows): err.append('exit registry row arity must be 3')
            if codes!=EXITS: err.append(f'exit registry mismatch: {sorted(codes)}')
        except (VError,ValueError) as e: err.append(str(e))
    else: err.append('DXB-IFC-040 missing')

    cr=[]; ir=[]; command_mile={}
    if cli and inp:
        try:
            cr=listrows(cli.text,'<!-- p0-command-registry:start -->','<!-- p0-command-registry:end -->')
            ir=listrows(inp.text,'<!-- p0-input-registry:start -->','<!-- p0-input-registry:end -->')
            if any(len(r)!=9 for r in cr): err.append('command registry row arity must be 9')
            if any(len(r)!=5 for r in ir): err.append('input registry row arity must be 5')
            cm={r[0]:r for r in cr if len(r)==9}; im={r[0]:r for r in ir if len(r)==5}
            if len(cm)!=sum(len(r)==9 for r in cr): err.append('duplicate command key')
            if len(im)!=sum(len(r)==5 for r in ir): err.append('duplicate input command key')
            if set(cm)!=set(im): err.append('command/input key mismatch')
            for key in set(cm)&set(im):
                c,i=cm[key],im[key]
                if c[4]!=i[1]: err.append(f'input schema drift for {key}')
                if c[8]!=i[4]: err.append(f'Acceptance drift for {key}')
                if not i[2].startswith('tgt-'): err.append(f'target schema invalid for {key}')
                if '*?' in i[3] or '+?' in i[3]: err.append(f'invalid field modifier for {key}')
                for fld in [x for x in i[3].split(';') if x]:
                    if not FIELD.fullmatch(fld): err.append(f'invalid field DSL for {key}: {fld}')
                    name=fld.split(':',1)[0]
                    if name in LOCAL and '@local' not in fld:
                        err.append(f'local field must use @local for {key}: {name}')
                    if 'ContentSource' in fld and '@oneof{' not in fld:
                        err.append(f'ContentSource must use @oneof for {key}: {fld}')
                if 'target-terminal' in c[5]: err.append(f'target-terminal wait forbidden: {key}')
                if c[3]=='C' and c[6]!='out-operation-v1':
                    err.append(f'Application Command output must be out-operation-v1: {key}')
                if c[3]=='C' and 'applied' in c[5] and key not in APPLIED:
                    err.append(f'applied wait has no Directive owner: {key}')
            freeze=listrows(cli.text,'<!-- milestone-freeze:start -->','<!-- milestone-freeze:end -->')
            seen=[]
            for r in freeze:
                if len(r)!=2: err.append('milestone freeze row arity must be 2'); continue
                if r[0] not in MORDER: err.append(f'invalid command milestone: {r[0]}')
                for key in [x for x in r[1].split(',') if x]:
                    seen.append(key); command_mile[key]=r[0]
            if set(seen)!=set(cm) or len(seen)!=len(set(seen)):
                err.append('milestone freeze coverage/drift')
        except VError as e: err.append(str(e))
    else: err.append('CLI command/input owners missing')

    aids=set(); amile={}
    if acc:
        try:
            rows=tablerows(acc.text,'<!-- acceptance-registry:start -->','<!-- acceptance-registry:end -->','AT-')
            if any(len(r)!=9 for r in rows): err.append('acceptance registry row arity must be 9')
            for r in rows:
                if len(r)!=9: continue
                ident=r[0]
                if ident in aids: err.append(f'duplicate Acceptance ID: {ident}')
                aids.add(ident)
                if r[4] not in MORDER: err.append(f'Acceptance must have one valid milestone: {ident}={r[4]}')
                else: amile[ident]=r[4]
                if r[5] not in ASTAT: err.append(f'invalid Acceptance status {ident}={r[5]}')
                if not all(r[i] for i in (6,7,8)): err.append(f'Acceptance evidence fields empty: {ident}')
        except VError as e: err.append(str(e))
    else: err.append('DXB-DEL-061 missing')

    for r in cr:
        if len(r)!=9: continue
        key,aid=r[0],r[8]
        if aid not in aids: err.append(f'command Acceptance orphan: {aid}'); continue
        cm,am=command_mile.get(key),amile.get(aid)
        if cm in MORDER and am in MORDER and MORDER[am]>MORDER[cm]:
            err.append(f'command milestone precedes Acceptance: {key}={cm} < {aid}={am}')

    if road:
        try:
            rows=listrows(road.text,'<!-- milestone-exit-registry:start -->','<!-- milestone-exit-registry:end -->')
            scheduled={}; seen=set()
            for r in rows:
                if len(r)!=2: err.append('milestone exit row arity must be 2'); continue
                mile=r[0]
                if mile not in MORDER: err.append(f'invalid milestone exit key: {mile}'); continue
                if mile in seen: err.append(f'duplicate milestone exit key: {mile}')
                seen.add(mile)
                for aid in [x for x in r[1].split(',') if x]:
                    if aid in scheduled: err.append(f'Acceptance scheduled more than once: {aid}')
                    scheduled[aid]=mile
            if seen!=set(MILES): err.append(f'milestone exit coverage mismatch: {sorted(seen)}')
            if set(scheduled)!=aids: err.append(f'milestone/Acceptance coverage drift: scheduled={sorted(scheduled)} acceptance={sorted(aids)}')
            for aid,mile in amile.items():
                if scheduled.get(aid)!=mile: err.append(f'milestone schedule drift: {aid} {scheduled.get(aid)!r} != {mile!r}')
        except VError as e: err.append(str(e))
    else: err.append('DXB-DEL-060 missing')

    if risk:
        try:
            rows=tablerows(risk.text,'<!-- risk-registry:start -->','<!-- risk-registry:end -->','R-')
            if any(len(r)!=8 for r in rows): err.append('risk registry row arity must be 8')
        except VError as e: err.append(str(e))
    else: err.append('DXB-DEL-062 missing')

    for d in ds:
        for raw in LINK.findall(d.text):
            target=raw.strip()
            if not target or target.startswith(('#','http://','https://','mailto:')): continue
            target=target.split('#',1)[0]
            if not target: continue
            path=(d.path.parent/target).resolve()
            try: path.relative_to(pkg.resolve())
            except ValueError: err.append(f'{d.rel}: link escapes package: {raw}'); continue
            if not path.exists(): err.append(f'{d.rel}: broken link: {raw}')
    return err

def expect(pkg,contains,name):
    errors=validate(pkg)
    if not any(contains in x for x in errors): raise VError(f'{name} fixture did not fail as expected: {errors}')

def selftest(repo):
    src=active(repo)
    with tempfile.TemporaryDirectory(prefix='dxbot-plan-validator-') as td:
        pkg=Path(td)/'docs/plan'/src.name; pkg.parent.mkdir(parents=True); shutil.copytree(src,pkg)
        if errors:=validate(pkg): raise VError(f'active package failed self-test baseline: {errors}')

        path=pkg/'40-interfaces/41-cli.md'; original=path.read_text()
        path.write_text(original.replace('default=committed;allowed=accepted,committed` | `out-operation-v1`','default=committed;allowed=accepted,committed,target-terminal` | `out-operation-v1`',1))
        expect(pkg,'target-terminal','target-terminal'); path.write_text(original)

        path=pkg/'40-interfaces/42-cli-input-contract.md'; original=path.read_text()
        path.write_text(original.replace('evidence:EvidenceRef*@argv','evidence:EvidenceRef*?@argv',1))
        expect(pkg,'invalid field modifier','DSL modifier'); path.write_text(original)
        path.write_text(original.replace('all:Bool?@local','all:Bool?@argv',1))
        expect(pkg,'local field must use @local','local field leak'); path.write_text(original)

        path=pkg/'00-governance/00-normative-baseline.md'; original=path.read_text()
        path.write_text('\n'.join(l for l in original.splitlines() if not l.startswith('- `INV-015`'))+'\n')
        expect(pkg,'product invariant coverage mismatch','invariant coverage'); path.write_text(original)

        path=pkg/'40-interfaces/41-cli.md'; original=path.read_text()
        moved=original.replace('runtime-doctor,approval-list','runtime-doctor,version,approval-list',1).replace('`M3` | `version,bot-create','`M3` | `bot-create',1)
        path.write_text(moved); expect(pkg,'command milestone precedes Acceptance','milestone ordering'); path.write_text(original)

        path=pkg/'manifest.md'; original=path.read_text()
        changed,count=re.subn(r'(?m)^markdown_count:\s*\d+\s*$','markdown_count: 999',original,count=1)
        if count!=1 or changed==original: raise VError('manifest fixture failed to mutate current markdown_count')
        path.write_text(changed); expect(pkg,'markdown_count','manifest')
    print('plan-validator self-test: PASS')

def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--active',action='store_true'); parser.add_argument('--self-test',action='store_true'); parser.add_argument('--repo',default='.')
    args=parser.parse_args()
    if not args.active and not args.self_test: parser.error('select --active and/or --self-test')
    repo=Path(args.repo).resolve()
    try:
        if args.active:
            pkg=active(repo); errors=validate(pkg)
            if errors:
                for error in errors: print('ERROR:',error,file=sys.stderr)
                return 1
            print(f'active plan: {pkg.relative_to(repo)}'); print(f'markdown documents: {len(list(pkg.rglob("*.md")))}'); print('plan-validator active validation: PASS')
        if args.self_test: selftest(repo)
    except VError as e: print('ERROR:',e,file=sys.stderr); return 1
    return 0
if __name__=='__main__': raise SystemExit(main())
