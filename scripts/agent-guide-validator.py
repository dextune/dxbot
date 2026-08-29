#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

KEBAB = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+)?$")
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
CATEGORIES = ("repository", "architecture", "rust", "runtime", "quality")
LEGACY = {
    "architecture-and-code-rules.md",
    "repository-and-git-rules.md",
    "testing-and-review-rules.md",
}


def local_links(path: Path, text: str):
    for raw in LINK.findall(text):
        target = raw.strip()
        if not target or target.startswith(("#", "http://", "https://", "mailto:")):
            continue
        target = target.split("#", 1)[0]
        if target:
            yield raw, (path.parent / target).resolve()


def validate(repo: Path) -> list[str]:
    errors: list[str] = []
    agent = repo / "docs/agent"
    root = repo / "AGENTS.md"
    guide = agent / "guide-index.md"

    if not root.is_file():
        return ["AGENTS.md missing"]
    if not guide.is_file():
        return ["docs/agent/guide-index.md missing"]

    root_text = root.read_text(encoding="utf-8")
    if "docs/agent/guide-index.md" not in root_text:
        errors.append("AGENTS.md must route through docs/agent/guide-index.md")

    guide_text = guide.read_text(encoding="utf-8")
    for category in CATEGORIES:
        index = agent / category / "index.md"
        if not index.is_file():
            errors.append(f"category index missing: {category}/index.md")
        if f"{category}/index.md" not in guide_text:
            errors.append(f"guide-index missing category route: {category}/index.md")

    for name in LEGACY:
        if not (agent / name).is_file():
            errors.append(f"legacy router missing: {name}")

    markdown = sorted(agent.rglob("*.md"))
    for path in markdown:
        rel = path.relative_to(agent)
        for part in rel.parts:
            if not KEBAB.fullmatch(part):
                errors.append(f"non-kebab agent path: {rel}")
        text = path.read_text(encoding="utf-8")
        for raw, target in local_links(path, text):
            try:
                target.relative_to(repo.resolve())
            except ValueError:
                errors.append(f"link escapes repository: {rel}: {raw}")
                continue
            if not target.exists():
                errors.append(f"broken relative link: {rel}: {raw}")

    # Every category detail must be reachable directly from its category index.
    for category in CATEGORIES:
        directory = agent / category
        index = directory / "index.md"
        if not index.is_file():
            continue
        text = index.read_text(encoding="utf-8")
        for detail in sorted(directory.glob("*.md")):
            if detail.name == "index.md":
                continue
            if detail.name not in text:
                errors.append(f"orphan detail guide: {category}/{detail.name}")

    # Top-level guide docs must be either the root router, documentation canonical guide, or legacy routers.
    allowed_top = {"guide-index.md", "documentation-rules.md", *LEGACY}
    for path in agent.glob("*.md"):
        if path.name not in allowed_top:
            errors.append(f"unrouted top-level agent guide: {path.name}")

    return errors


def expect_failure(repo: Path, contains: str, label: str):
    errors = validate(repo)
    if not any(contains in error for error in errors):
        raise RuntimeError(f"{label} negative fixture did not fail: {errors}")


def self_test(repo: Path):
    with tempfile.TemporaryDirectory(prefix="dxbot-agent-guide-validator-") as temp:
        copy = Path(temp) / "repo"
        shutil.copytree(repo / "docs", copy / "docs")
        shutil.copy2(repo / "AGENTS.md", copy / "AGENTS.md")
        if errors := validate(copy):
            raise RuntimeError(f"baseline failed: {errors}")

        guide = copy / "docs/agent/guide-index.md"
        original = guide.read_text(encoding="utf-8")
        guide.write_text(original.replace("](repository/index.md)", "](repository/missing-index.md)", 1), encoding="utf-8")
        expect_failure(copy, "broken relative link", "broken link")
        guide.write_text(original, encoding="utf-8")

        detail = copy / "docs/agent/rust/temporary-rule.md"
        detail.write_text("# Temporary Rule\n", encoding="utf-8")
        expect_failure(copy, "orphan detail guide", "orphan guide")
        detail.unlink()

        root = copy / "AGENTS.md"
        original_root = root.read_text(encoding="utf-8")
        root.write_text(original_root.replace("docs/agent/guide-index.md", "docs/agent/missing.md", 1), encoding="utf-8")
        expect_failure(copy, "AGENTS.md must route", "root router")

    print("agent-guide-validator self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--active", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--repo", default=".")
    args = parser.parse_args()
    if not args.active and not args.self_test:
        parser.error("select --active and/or --self-test")

    repo = Path(args.repo).resolve()
    try:
        if args.active:
            errors = validate(repo)
            if errors:
                for error in errors:
                    print("ERROR:", error, file=sys.stderr)
                return 1
            print("agent guide documents:", len(list((repo / "docs/agent").rglob("*.md"))))
            print("agent-guide-validator active validation: PASS")
        if args.self_test:
            self_test(repo)
    except RuntimeError as error:
        print("ERROR:", error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
