#!/usr/bin/env python3
"""Verify the plugin manifests agree with each other and with what is on disk.

`claude plugin validate --strict` is the authority on whether a *Claude Code*
manifest is well formed, and it should be run before publishing. It is not what
this script does, for two reasons: it needs Claude Code installed, which CI does
not have, and it knows nothing about the other two clients.

This plugin is described five times over. Three clients each want their own
plugin manifest, and three want their own marketplace manifest in three
different shapes and three different places:

    .claude-plugin/marketplace.json     Claude Code    source: "./plugins/gore"
    .cursor-plugin/marketplace.json     Cursor         pluginRoot + source: "gore"
    .agents/plugins/marketplace.json    Codex          source.path: "./plugins/gore"

    plugins/gore/.claude-plugin/plugin.json
    plugins/gore/.codex-plugin/plugin.json
    plugins/gore/.cursor-plugin/plugin.json

    plugins/gore/.mcp.json    Claude Code and Codex read this name
    plugins/gore/mcp.json     Cursor's documented name for the same content

Every one of those hand-carries a name, and two carry a version. Nothing in any
client checks that they agree, and a disagreement does not fail loudly — it
installs a plugin that is missing a piece on one client only. That is what this
script is for.

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

# Every client's manifest for one plugin, relative to the plugin directory. The
# first is the one Claude Code reads and the one its marketplace entry points at;
# the others exist so Codex and Cursor can offer the same plugin.
MANIFESTS = (
    Path(".claude-plugin") / "plugin.json",
    Path(".codex-plugin") / "plugin.json",
    Path(".cursor-plugin") / "plugin.json",
)

# The MCP configuration, under the two names the three clients look for. Claude
# Code and Codex both document `.mcp.json`; Cursor's own marketplace template
# uses `mcp.json`. Same content, so they are kept byte-identical.
MCP_NAMES = (".mcp.json", "mcp.json")

# Marketplace manifests, as (path, human name). Each client only reads its own.
MARKETPLACES = (
    (Path(".claude-plugin") / "marketplace.json", "Claude Code"),
    (Path(".cursor-plugin") / "marketplace.json", "Cursor"),
    (Path(".agents") / "plugins" / "marketplace.json", "Codex"),
)


def load(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def check_mcp(plugin_dir: Path, rel: str, problems: list[str]) -> set[str]:
    """Both spellings of the MCP config agree, and say which user_config keys they use."""
    bodies: dict[str, str] = {}
    used: set[str] = set()

    for name in MCP_NAMES:
        path = plugin_dir / name
        if not path.is_file():
            problems.append(f"{rel}: missing {name} (one client looks for exactly this name)")
            continue
        bodies[name] = path.read_text(encoding="utf-8")
        try:
            data = json.loads(bodies[name])
        except json.JSONDecodeError as error:
            problems.append(f"{rel}/{name}: not valid JSON: {error}")
            continue

        # The wrapper is what all three document. A bare map works in Claude Code
        # and is not worth relying on across three clients.
        if "mcpServers" not in data:
            problems.append(f"{rel}/{name}: must wrap its servers in an `mcpServers` object")
            continue

        for server in data["mcpServers"].values():
            for value in (server.get("env") or {}).values():
                if isinstance(value, str) and value.startswith("${user_config."):
                    used.add(value[len("${user_config.") :].rstrip("}"))

    if len(bodies) == len(MCP_NAMES) and len(set(bodies.values())) > 1:
        problems.append(
            f"{rel}: {' and '.join(MCP_NAMES)} have drifted apart; they are the same "
            "configuration under the two names different clients look for"
        )

    return used


def check_plugin(plugin_dir: Path, problems: list[str]) -> None:
    """Every manifest in one plugin directory agrees, and its referents exist."""
    rel = plugin_dir.relative_to(ROOT).as_posix()
    seen: dict[str, set[str]] = {"name": set(), "version": set()}
    declared_config: set[str] = set()

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

        declared_config |= set((data.get("userConfig") or {}).keys())

    for field, values in seen.items():
        if len(values) > 1:
            problems.append(
                f"{rel}: the manifests disagree about `{field}`: "
                + ", ".join(sorted(repr(value) for value in values))
            )

    used_config = check_mcp(plugin_dir, rel, problems)

    # A `${user_config.X}` naming an option no manifest declares is never
    # substituted. The client passes the text through verbatim, and what reads it
    # sees a literal `${user_config.X}` rather than the setting someone expected.
    for missing in sorted(used_config - declared_config):
        problems.append(
            f"{rel}: the MCP config substitutes `${{user_config.{missing}}}`, which no "
            "plugin.json declares under `userConfig`"
        )

    if not (plugin_dir / "skills").is_dir():
        problems.append(f"{rel}: carries no skills/ directory")


def plugin_names(path: Path, problems: list[str]) -> set[str]:
    """The plugin names one marketplace manifest offers, whatever shape it uses."""
    try:
        data = load(path)
    except json.JSONDecodeError as error:
        problems.append(f"{path.relative_to(ROOT).as_posix()}: not valid JSON: {error}")
        return set()

    entries = data.get("plugins")
    if not isinstance(entries, list) or not entries:
        problems.append(
            f"{path.relative_to(ROOT).as_posix()}: `plugins` must be a non-empty list"
        )
        return set()

    names: set[str] = set()
    for entry in entries:
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            problems.append(f"{path.relative_to(ROOT).as_posix()}: an entry has no `name`")
            continue
        names.add(name)

        # Each client spells the location of the plugin folder its own way. All
        # three have to land on a directory that is really there.
        source = entry.get("source")
        if isinstance(source, dict):  # Codex: {"source": "local", "path": "./plugins/gore"}
            target = source.get("path")
        elif isinstance(source, str) and "pluginRoot" in json.dumps(data.get("metadata", {})):
            target = f"{data['metadata']['pluginRoot']}/{source}"  # Cursor
        else:  # Claude Code: a path relative to the repository root
            target = source
        if not isinstance(target, str) or not (ROOT / target).is_dir():
            problems.append(
                f"{path.relative_to(ROOT).as_posix()}: {name!r} points at {target!r}, "
                "which is not a directory"
            )

    return names


def main() -> int:
    problems: list[str] = []
    offered: dict[str, set[str]] = {}

    for rel, client in MARKETPLACES:
        path = ROOT / rel
        if not path.is_file():
            problems.append(f"missing {rel.as_posix()} — {client} would find no marketplace here")
            continue
        offered[client] = plugin_names(path, problems)

    if len(offered) == len(MARKETPLACES) and len(set(map(frozenset, offered.values()))) > 1:
        problems.append(
            "the three marketplaces offer different plugins: "
            + "; ".join(f"{client} {sorted(names)}" for client, names in offered.items())
        )

    checked = 0
    for name in sorted(set().union(*offered.values()) if offered else set()):
        plugin_dir = ROOT / "plugins" / name
        if not plugin_dir.is_dir():
            problems.append(f"marketplaces offer {name!r} but plugins/{name} is not a directory")
            continue
        # `claude plugin install <name>@<marketplace>` uses the marketplace's
        # name for the plugin, so it has to match what the manifest calls itself.
        manifest = plugin_dir / MANIFESTS[0]
        if manifest.is_file():
            declared = load(manifest).get("name")
            if declared != name:
                problems.append(
                    f"the marketplaces call it {name!r} but "
                    f"plugins/{name}/{MANIFESTS[0].as_posix()} says {declared!r}"
                )
        check_plugin(plugin_dir, problems)
        checked += 1

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(f"OK: {checked} plugin(s) checked across {len(MARKETPLACES)} marketplaces.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
