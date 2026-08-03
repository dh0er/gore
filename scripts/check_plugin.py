#!/usr/bin/env python3
"""Verify the plugin manifests agree with each other and with what is on disk.

`claude plugin validate --strict` is the authority on whether a manifest is
well formed, and it should be run before publishing. It is not what this script
does, for one practical reason: it needs Claude Code installed, and CI does not
have it. What CI can check for free is the part that actually drifts.

A plugin here is described three times over — once for Claude Code, once for
Codex, once for Cursor — and each copy hand-carries a name and a version. Three
hand-kept copies of one fact is the shape every other triplicated fact in this
repository has already been wrong in at least once, which is why several of them
now have a test holding them together. This is that test, for the manifests.

Run from anywhere:

    python scripts/check_plugin.py

Exits non-zero if a manifest disagrees with its siblings or points at something
that is not there.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MARKETPLACE = ROOT / ".claude-plugin" / "marketplace.json"

# Every client's manifest for one plugin, relative to the plugin directory. The
# first is the one Claude Code reads and the one the marketplace entry points at;
# the others exist so Codex and Cursor can offer the same skill.
MANIFESTS = (
    Path(".claude-plugin") / "plugin.json",
    Path(".codex-plugin") / "plugin.json",
    Path(".cursor-plugin") / "plugin.json",
)


def load(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def check_plugin(plugin_dir: Path, problems: list[str]) -> None:
    """Every manifest in one plugin directory agrees, and its referents exist."""
    rel = plugin_dir.relative_to(ROOT).as_posix()
    seen: dict[str, set[str]] = {"name": set(), "version": set()}

    for manifest_rel in MANIFESTS:
        manifest = plugin_dir / manifest_rel
        if not manifest.is_file():
            problems.append(f"{rel}: missing {manifest_rel.as_posix()}")
            continue
        try:
            data = load(manifest)
        except json.JSONDecodeError as error:
            problems.append(f"{rel}/{manifest_rel.as_posix()}: not valid JSON: {error}")
            continue

        for field in seen:
            value = data.get(field)
            if not isinstance(value, str) or not value:
                problems.append(
                    f"{rel}/{manifest_rel.as_posix()}: `{field}` must be a non-empty string"
                )
                continue
            seen[field].add(value)

        # `skills` names a directory the client loads from. A typo here fails
        # silently at install time: the plugin loads, and the skill is absent.
        skills = data.get("skills")
        if isinstance(skills, str) and not (plugin_dir / skills).is_dir():
            problems.append(
                f"{rel}/{manifest_rel.as_posix()}: `skills` points at {skills!r}, "
                "which is not a directory"
            )

    for field, values in seen.items():
        if len(values) > 1:
            problems.append(
                f"{rel}: the manifests disagree about `{field}`: "
                + ", ".join(sorted(repr(value) for value in values))
            )

    # A plugin with no skill and no server is an empty install.
    if not (plugin_dir / ".mcp.json").is_file() and not (plugin_dir / "skills").is_dir():
        problems.append(f"{rel}: carries neither .mcp.json nor skills/")


def main() -> int:
    problems: list[str] = []

    if not MARKETPLACE.is_file():
        print(f"missing {MARKETPLACE.relative_to(ROOT).as_posix()}", file=sys.stderr)
        return 1

    try:
        marketplace = load(MARKETPLACE)
    except json.JSONDecodeError as error:
        print(f".claude-plugin/marketplace.json: not valid JSON: {error}", file=sys.stderr)
        return 1

    entries = marketplace.get("plugins")
    if not isinstance(entries, list) or not entries:
        print(".claude-plugin/marketplace.json: `plugins` must be a non-empty list", file=sys.stderr)
        return 1

    checked = 0
    for entry in entries:
        source = entry.get("source")
        if not isinstance(source, str):
            problems.append(f"marketplace entry {entry.get('name')!r} has no string `source`")
            continue
        plugin_dir = (ROOT / source).resolve()
        if not plugin_dir.is_dir():
            problems.append(f"marketplace entry {entry.get('name')!r}: {source} is not a directory")
            continue

        # The marketplace card and the plugin manifest are two names for one
        # thing, and `claude plugin install <name>@<marketplace>` uses the card's.
        manifest = plugin_dir / MANIFESTS[0]
        if manifest.is_file():
            declared = load(manifest).get("name")
            if declared != entry.get("name"):
                problems.append(
                    f"marketplace calls it {entry.get('name')!r} but "
                    f"{MANIFESTS[0].as_posix()} says {declared!r}"
                )

        check_plugin(plugin_dir, problems)
        checked += 1

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(f"OK: {checked} plugin(s) checked, manifests agree.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
