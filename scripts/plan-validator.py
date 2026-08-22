#!/usr/bin/env python3
"""Validate the active DXBOT detailed development plan without external packages."""

from __future__ import annotations

import argparse
import ast
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
KEBAB_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+)?$")
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
AT_RE = re.compile(r"\bAT-[A-Z]+-\d+\b")


@dataclass(frozen=True)
class Document:
    path: Path
    rel: Path
    meta: dict[str, object]
    text: str


class ValidationError(Exception):
    pass


def parse_scalar(raw: str) -> object:
    value = raw.strip()
    if not value:
        return ""
    if value in {"true", "false"}:
        return value == "true"
    if value.startswith("[") and value.endswith("]"):
        try:
            parsed = ast.literal_eval(value)
        except (SyntaxError, ValueError) as exc:
            raise ValidationError(f"invalid frontmatter list: {value}") from exc
        if not isinstance(parsed, list):
            raise ValidationError(f"frontmatter list expected: {value}")
        return parsed
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return value[1:-1]
    if re.fullmatch(r"\d+", value):
        return int(value)
    return value


def parse_frontmatter(path: Path) -> tuple[dict[str, object], str]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        raise ValidationError(f"{path}: missing frontmatter")
    try:
        end = next(i for i in range(1, len(lines)) if lines[i].strip() == "---")
    except StopIteration as exc:
        raise ValidationError(f"{path}: unterminated frontmatter") from exc
    meta: dict[str, object] = {}
    for line in lines[1:end]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if ":" not in line:
            raise ValidationError(f"{path}: malformed frontmatter line: {line}")
        key, raw = line.split(":", 1)
        meta[key.strip()] = parse_scalar(raw)
    return meta, text


def semver_tuple(value: str) -> tuple[int, int, int]:
    if not SEMVER_RE.match(value):
        raise ValidationError(f"invalid semantic version: {value}")
    core = re.split(r"[-+]", value, maxsplit=1)[0]
    return tuple(int(x) for x in core.split("."))  # type: ignore[return-value]


def discover_active(repo: Path) -> Path:
    candidates: list[tuple[tuple[int, int, int], Path]] = []
    for readme in repo.glob("docs/plan/*-detailed-development-plan/readme.md"):
        try:
            meta, _ = parse_frontmatter(readme)
        except ValidationError:
            continue
        if meta.get("document_id") != "DXB-INDEX" or meta.get("status") != "Accepted":
            continue
        version = meta.get("version")
        if not isinstance(version, str):
            raise ValidationError(f"{readme}: missing version")
        candidates.append((semver_tuple(version), readme.parent))
    if not candidates:
        raise ValidationError("no Accepted DXB-INDEX package found")
    highest = max(v for v, _ in candidates)
    winners = [path for version, path in candidates if version == highest]
    if len(winners) != 1:
        raise ValidationError(f"multiple active packages for version {highest}: {winners}")
    return winners[0]


def path_is_kebab(rel: Path) -> bool:
    for part in rel.parts:
        if part in {"readme.md", "manifest.md"}:
            continue
        if not KEBAB_RE.fullmatch(part):
            return False
    return True


def load_documents(package: Path) -> list[Document]:
    docs: list[Document] = []
    for path in sorted(package.rglob("*.md")):
        meta, text = parse_frontmatter(path)
        docs.append(Document(path=path, rel=path.relative_to(package), meta=meta, text=text))
    return docs


def extract_registry(text: str, start: str, end: str) -> list[tuple[str, ...]]:
    if start not in text or end not in text:
        raise ValidationError(f"registry markers missing: {start}")
    body = text.split(start, 1)[1].split(end, 1)[0]
    rows: list[tuple[str, ...]] = []
    for line in body.splitlines():
        line = line.strip()
        if not line.startswith("- `"):
            continue
        cells = [cell.strip().strip("`") for cell in line[2:].split("|")]
        rows.append(tuple(cells))
    return rows


def parse_machine_manifest(text: str) -> dict[str, str]:
    start = "<!-- manifest-machine:start -->"
    end = "<!-- manifest-machine:end -->"
    if start not in text or end not in text:
        raise ValidationError("manifest machine block missing")
    body = text.split(start, 1)[1].split(end, 1)[0]
    result: dict[str, str] = {}
    for line in body.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            result[key.strip()] = value.strip()
    return result


def validate(package: Path, repo: Path | None = None) -> list[str]:
    errors: list[str] = []
    try:
        docs = load_documents(package)
    except ValidationError as exc:
        return [str(exc)]

    if not docs:
        return [f"{package}: no markdown documents"]

    by_id: dict[str, Document] = {}
    for doc in docs:
        if not path_is_kebab(doc.rel):
            errors.append(f"non-kebab path: {doc.rel}")
        doc_id = doc.meta.get("document_id")
        if not isinstance(doc_id, str) or not doc_id:
            errors.append(f"{doc.rel}: missing document_id")
            continue
        if doc_id in by_id:
            errors.append(f"duplicate document_id {doc_id}: {by_id[doc_id].rel}, {doc.rel}")
        else:
            by_id[doc_id] = doc

    graph: dict[str, list[str]] = {}
    for doc_id, doc in by_id.items():
        deps = doc.meta.get("depends_on", [])
        if not isinstance(deps, list) or not all(isinstance(x, str) for x in deps):
            errors.append(f"{doc.rel}: depends_on must be a string list")
            deps = []
        graph[doc_id] = list(deps)
        for dep in deps:
            if dep not in by_id:
                errors.append(f"{doc.rel}: missing dependency {dep}")

    state: dict[str, int] = {}
    stack: list[str] = []

    def visit(node: str) -> None:
        state[node] = 1
        stack.append(node)
        for dep in graph.get(node, []):
            if dep not in graph:
                continue
            if state.get(dep) == 1:
                cycle = " -> ".join(stack[stack.index(dep):] + [dep])
                errors.append(f"dependency cycle: {cycle}")
            elif state.get(dep, 0) == 0:
                visit(dep)
        stack.pop()
        state[node] = 2

    for node in graph:
        if state.get(node, 0) == 0:
            visit(node)

    index = by_id.get("DXB-INDEX")
    manifest = by_id.get("DXB-MANIFEST")
    source = by_id.get("DXB-SOURCE-000")
    cli = by_id.get("DXB-IFC-041")
    inputs = by_id.get("DXB-IFC-042")
    acceptance = by_id.get("DXB-DEL-061")

    if index is None:
        errors.append("DXB-INDEX missing")
    else:
        actual = package.as_posix()
        declared = index.meta.get("package_path")
        if not isinstance(declared, str) or not actual.endswith(declared):
            errors.append(f"package_path mismatch: declared={declared!r}, actual={actual}")
        version = index.meta.get("version")
        if not isinstance(version, str):
            errors.append("DXB-INDEX version missing")
        else:
            try:
                semver_tuple(version)
            except ValidationError as exc:
                errors.append(str(exc))

    if source is None:
        errors.append("DXB-SOURCE-000 missing")
    elif source.meta.get("normative") is not False:
        errors.append("DXB-SOURCE-000 must be normative:false")

    if manifest is None:
        errors.append("DXB-MANIFEST missing")
    else:
        try:
            mm = parse_machine_manifest(manifest.text)
            expected_path = index.meta.get("package_path") if index else None
            expected_version = index.meta.get("version") if index else None
            if mm.get("active_package_path") != expected_path:
                errors.append("manifest active_package_path drift")
            if mm.get("plan_version") != expected_version:
                errors.append("manifest plan_version drift")
            if mm.get("markdown_count") != str(len(docs)):
                errors.append(
                    f"manifest markdown_count drift: {mm.get('markdown_count')} != {len(docs)}"
                )
        except ValidationError as exc:
            errors.append(str(exc))

    if cli is None or inputs is None:
        errors.append("CLI command/input owner missing")
    else:
        try:
            command_rows = extract_registry(
                cli.text,
                "<!-- p0-command-registry:start -->",
                "<!-- p0-command-registry:end -->",
            )
            input_rows = extract_registry(
                inputs.text,
                "<!-- p0-input-registry:start -->",
                "<!-- p0-input-registry:end -->",
            )
            commands = [r[0] for r in command_rows if r]
            input_commands = [r[0] for r in input_rows if r]
            if len(commands) != len(set(commands)):
                errors.append("duplicate P0 command registry path")
            if len(input_commands) != len(set(input_commands)):
                errors.append("duplicate P0 input registry path")
            missing = sorted(set(commands) - set(input_commands))
            extra = sorted(set(input_commands) - set(commands))
            if missing or extra:
                errors.append(f"command/input registry mismatch missing={missing} extra={extra}")
            if acceptance:
                acceptance_ids = set(AT_RE.findall(acceptance.text))
                registry_ids = {r[3] for r in command_rows if len(r) >= 4}
                orphan = sorted(registry_ids - acceptance_ids)
                if orphan:
                    errors.append(f"command Acceptance orphan: {orphan}")
        except ValidationError as exc:
            errors.append(str(exc))

    for doc in docs:
        for raw_target in LINK_RE.findall(doc.text):
            target = raw_target.strip()
            if not target or target.startswith(("#", "http://", "https://", "mailto:")):
                continue
            target = target.split("#", 1)[0]
            if not target:
                continue
            resolved = (doc.path.parent / target).resolve()
            try:
                resolved.relative_to(package.resolve())
            except ValueError:
                errors.append(f"{doc.rel}: link escapes package: {raw_target}")
                continue
            if not resolved.exists():
                errors.append(f"{doc.rel}: broken link: {raw_target}")

    forbidden_path_terms = {"tui", "web", "bff", "frontend"}
    for doc in docs:
        parts = {p.lower() for p in doc.rel.parts}
        if parts & forbidden_path_terms:
            errors.append(f"active forbidden interface artifact: {doc.rel}")

    return errors


def write_doc(path: Path, doc_id: str, depends: list[str] | None = None, body: str = "") -> None:
    depends = depends or []
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "---\n"
        f'title: "{doc_id}"\n'
        f'document_id: "{doc_id}"\n'
        'version: "0.0.1"\n'
        'status: "Accepted"\n'
        "normative: true\n"
        'priority: "P0"\n'
        'last_updated: "2026-08-22"\n'
        "depends_on: [" + ", ".join(f'"{x}"' for x in depends) + "]\n"
        "---\n\n# " + doc_id + "\n\n" + body,
        encoding="utf-8",
    )


def make_self_test_package(base: Path) -> Path:
    package = base / "docs/plan/20260822-0000-v0-0-1-detailed-development-plan"
    package.mkdir(parents=True)
    write_doc(package / "readme.md", "DXB-INDEX", ["DXB-BASE-000"], "")
    index = (package / "readme.md").read_text(encoding="utf-8")
    index = index.replace(
        'depends_on: ["DXB-BASE-000"]',
        'depends_on: ["DXB-BASE-000"]\n'
        'package_path: "docs/plan/20260822-0000-v0-0-1-detailed-development-plan"',
    )
    (package / "readme.md").write_text(index, encoding="utf-8")
    write_doc(package / "00-governance/00-baseline.md", "DXB-BASE-000")
    write_doc(package / "00-governance/00-source.md", "DXB-SOURCE-000", body="")
    source_text = (package / "00-governance/00-source.md").read_text(encoding="utf-8")
    source_text = source_text.replace("normative: true", "normative: false")
    (package / "00-governance/00-source.md").write_text(source_text, encoding="utf-8")
    cli_body = (
        "<!-- p0-command-registry:start -->\n"
        "- `bot show` | `GetBot` | `Q` | `AT-CLI-001`\n"
        "<!-- p0-command-registry:end -->"
    )
    input_body = (
        "<!-- p0-input-registry:start -->\n"
        "- `bot show` | BotId\n"
        "<!-- p0-input-registry:end -->"
    )
    write_doc(package / "40-interfaces/41-cli.md", "DXB-IFC-041", body=cli_body)
    write_doc(package / "40-interfaces/42-cli-input.md", "DXB-IFC-042", body=input_body)
    write_doc(package / "60-delivery/61-acceptance.md", "DXB-DEL-061", body="AT-CLI-001")
    write_doc(package / "manifest.md", "DXB-MANIFEST")
    manifest = (package / "manifest.md").read_text(encoding="utf-8")
    manifest += (
        "\n<!-- manifest-machine:start -->\n"
        "active_package_path: docs/plan/20260822-0000-v0-0-1-detailed-development-plan\n"
        "plan_version: 0.0.1\n"
        "review_revision: 1\n"
        "markdown_count: 7\n"
        "source_baseline_commit: test\n"
        "<!-- manifest-machine:end -->\n"
    )
    (package / "manifest.md").write_text(manifest, encoding="utf-8")
    return package


def run_self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="dxbot-plan-validator-") as td:
        base = Path(td)
        package = make_self_test_package(base)
        errors = validate(package, base)
        if errors:
            raise ValidationError(f"valid self-test fixture failed: {errors}")

        duplicate = package / "00-governance/01-duplicate.md"
        write_doc(duplicate, "DXB-BASE-000")
        if not any("duplicate document_id" in e for e in validate(package, base)):
            raise ValidationError("duplicate-ID negative fixture did not fail")
        duplicate.unlink()

        a = package / "00-governance/00-baseline.md"
        text = a.read_text(encoding="utf-8").replace("depends_on: []", 'depends_on: ["DXB-INDEX"]')
        a.write_text(text, encoding="utf-8")
        if not any("dependency cycle" in e for e in validate(package, base)):
            raise ValidationError("cycle negative fixture did not fail")
        a.write_text(text.replace('depends_on: ["DXB-INDEX"]', "depends_on: []"), encoding="utf-8")

        input_path = package / "40-interfaces/42-cli-input.md"
        input_text = input_path.read_text(encoding="utf-8").replace("- `bot show` | BotId\n", "")
        input_path.write_text(input_text, encoding="utf-8")
        if not any("registry mismatch" in e for e in validate(package, base)):
            raise ValidationError("missing-input negative fixture did not fail")
        input_path.write_text(input_text.replace(
            "<!-- p0-input-registry:end -->",
            "- `bot show` | BotId\n<!-- p0-input-registry:end -->",
        ), encoding="utf-8")

        manifest = package / "manifest.md"
        mtext = manifest.read_text(encoding="utf-8").replace("markdown_count: 7", "markdown_count: 999")
        manifest.write_text(mtext, encoding="utf-8")
        if not any("markdown_count drift" in e for e in validate(package, base)):
            raise ValidationError("manifest negative fixture did not fail")

    print("plan-validator self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--active", action="store_true", help="validate highest Accepted plan")
    parser.add_argument("--self-test", action="store_true", help="run built-in negative fixtures")
    parser.add_argument("--repo", default=".", help="repository root")
    args = parser.parse_args()
    if not args.active and not args.self_test:
        parser.error("select --active and/or --self-test")

    try:
        if args.self_test:
            run_self_test()
        if args.active:
            repo = Path(args.repo).resolve()
            package = discover_active(repo)
            errors = validate(package, repo)
            if errors:
                for error in errors:
                    print(f"ERROR: {error}", file=sys.stderr)
                return 1
            docs = len(list(package.rglob("*.md")))
            print(f"active plan: {package.relative_to(repo)}")
            print(f"markdown documents: {docs}")
            print("plan-validator active validation: PASS")
    except ValidationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
