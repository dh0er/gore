#!/usr/bin/env python3
"""Verify every relative Markdown link and anchor in the repository's docs.

Checks the root README, everything under docs/ (except the gitignored
docs/superpowers/ and docs/internal/ areas), and the component READMEs. External
links (http, https, mailto) are ignored; relative targets must exist on disk, and
`#anchor` fragments must match a heading in the target document.

Run from anywhere:

    python scripts/check_docs_links.py

Exits non-zero if any link is broken.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Files whose links are checked.
GLOBS = (
    "README.md",
    "VISION.md",
    "docs/**/*.md",
    "apps/*/README.md",
    "lua/README.md",
    "mods/*/README.md",
    "crates/*/*.md",
    # The plugin ships to users through a marketplace, so its prose is as public as the guide's —
    # and its skill is read by a model rather than a person, which makes a dead link cheaper to
    # introduce and dearer to notice.
    "plugins/*/README.md",
    "plugins/*/skills/*/SKILL.md",
)

# Skipped entirely: local-only areas, not part of the published docs.
EXCLUDED_DIRS = ("docs/superpowers", "docs/internal")

MD_LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
HTML_SRC = re.compile(r"<img[^>]*\ssrc=\"([^\"]+)\"")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$", flags=re.MULTILINE)
EXTERNAL = re.compile(r"^(https?:|mailto:|#|<)")


def slug(heading: str) -> str:
    """GitHub's heading-to-anchor rule, close enough for our docs."""
    text = heading.strip().lower()
    text = re.sub(r"[`*_~]", "", text)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)  # links keep their text
    text = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE)
    return re.sub(r"\s+", "-", text).strip("-")


def anchors_of(path: Path) -> set[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return set()
    found: set[str] = set()
    for _, heading in HEADING.findall(text):
        base = slug(heading)
        if not base:
            continue
        candidate, n = base, 1
        while candidate in found:
            candidate = f"{base}-{n}"
            n += 1
        found.add(candidate)
    return found


def collect_files() -> list[Path]:
    files: list[Path] = []
    for pattern in GLOBS:
        for path in sorted(ROOT.glob(pattern)):
            rel = path.relative_to(ROOT).as_posix()
            if any(rel.startswith(prefix) for prefix in EXCLUDED_DIRS):
                continue
            if path.is_file():
                files.append(path)
    return files


def check_file(path: Path, anchor_cache: dict[Path, set[str]]) -> list[str]:
    rel = path.relative_to(ROOT).as_posix()
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    problems: list[str] = []

    for lineno, line in enumerate(lines, start=1):
        targets = MD_LINK.findall(line) + HTML_SRC.findall(line)
        for target in targets:
            if EXTERNAL.match(target):
                if target.startswith("#"):
                    anchor = target[1:]
                    known = anchor_cache.setdefault(path, anchors_of(path))
                    if anchor and anchor not in known:
                        problems.append(
                            f"{rel}:{lineno}: same-document anchor '#{anchor}' not found"
                        )
                continue

            file_part, _, anchor = target.partition("#")
            if not file_part:
                continue

            resolved = (path.parent / file_part).resolve()
            if not resolved.exists():
                problems.append(f"{rel}:{lineno}: missing target '{target}'")
                continue

            if anchor and resolved.suffix.lower() == ".md":
                known = anchor_cache.setdefault(resolved, anchors_of(resolved))
                if anchor not in known:
                    problems.append(
                        f"{rel}:{lineno}: anchor '#{anchor}' not found in {file_part}"
                    )

    return problems


def main() -> int:
    files = collect_files()
    anchor_cache: dict[Path, set[str]] = {}
    problems: list[str] = []
    for path in files:
        problems.extend(check_file(path, anchor_cache))

    if problems:
        for problem in problems:
            print(problem)
        print(f"\n{len(problems)} broken link(s) in {len(files)} file(s).")
        return 1

    print(f"OK: {len(files)} file(s) checked, no broken links.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
