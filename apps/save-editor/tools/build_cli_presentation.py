#!/usr/bin/env python3
"""Extract the Editor's declarative presentation tables for the Rust CLI.

Run with --check in CI. Source seals make drift fail the CLI's tests too.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / "apps/save-editor"
OUT = ROOT / "crates/gore/src/cmd/save/editor_presentation.json"


def read(relative, sources):
    # Git's autocrlf setting must not change the generated source seals.
    data = (APP / relative).read_bytes().replace(b"\r\n", b"\n")
    sources[f"apps/save-editor/{relative}"] = hashlib.sha256(data).hexdigest()
    return data.decode("utf-8")


def select_arms(text):
    match = re.match(r"^\{\w+,\s*select,\s*", text)
    if not match:
        return {}
    result = {}
    at = match.end()
    while at < len(text):
        arm = re.match(r"\s*([\w-]+)\{", text[at:])
        if not arm:
            break
        start = at + arm.end()
        depth, end = 1, start
        while end < len(text) and depth:
            depth += (text[end] == "{") - (text[end] == "}")
            end += 1
        if depth:
            raise ValueError("unbalanced ICU select")
        result[arm[1]] = text[start:end - 1]
        at = end
    return result


def build():
    sources = {}
    story = read("lib/features/editor/domain/story_state_semantics.dart", sources)
    known = {key: [int(v) for v in re.findall(r"-?\d+", values)]
             for key, values in re.findall(r"'([^']+)'\s*:\s*\[([^]]*)\]", story)}
    entries = {}
    table = story.split("_storyIntegerIdsByKind =", 1)[1].split("_knownValuesById =", 1)[0]
    for kind, values in re.findall(r"StoryIntegerKind\.(\w+)\s*:\s*('''.*?'''|'[^']*')", table, re.S):
        for name in values.strip("'").split():
            confidence = ("high-source-evidence" if kind in ["binaryFlag", "finiteState", "counterOrScore", "calendarDay"]
                          else "medium-source-evidence" if kind == "derivedOrOpaqueInteger"
                          else "medium-no-script-write" if kind == "readOnlyInSourceInteger"
                          else "no-live-script-reference")
            entries[name.lower()] = {"id": name, "kind": kind, "confidence": confidence,
                                     "knownValues": known.get(name, [0, 1] if kind == "binaryFlag" else [])}
    if len(entries) != 419:
        raise ValueError(f"expected all 419 story semantics, got {len(entries)}")
    hero = read("lib/features/editor/domain/hero_attributes.dart", sources)
    lists = {name: re.findall(r"'([^']+)'", body)
             for name, body in re.findall(r"const (\w+)\s*=\s*\[([^]]*)\]", hero, re.S)}
    groups = {group: lists[name] for group, name in re.findall(r"HeroAttributeGroup\.(\w+): (\w+)", hero)}
    hidden_body = hero.split("heroHiddenAttributeIds =", 1)[1].split("};", 1)[0]
    unused = hero.split("_heroUnusedAttributeIds =", 1)[1].split("};", 1)[0]
    hidden = re.findall(r"^\s*'([^']+)'", hidden_body + unused, re.M)
    # Inline lists can have several ids on one line; ignore comments first.
    hidden = re.findall(r"'([^']+)'", re.sub(r"//[^\n]*", "", hidden_body + unused))
    languages = read("lib/loc/game_lang.dart", sources).split("const List<UiLang> kUiLangs =", 1)[1].split("];", 1)[0]
    defaults = dict(re.findall(r"UiLang\(\s*'([^']+)'.*?,\s*'([^']+)'\s*,?\s*\)", languages, re.S))
    if len(defaults) < 10:
        raise ValueError("could not extract the Editor interface language table")
    ui, selects = {}, {}
    for lang in defaults:
        file = lang.replace("-", "_")
        strings = json.loads(read(f"lib/l10n/app_{file}.arb", sources))
        ui[lang] = {key: value for key, value in strings.items() if not key.startswith("@")}
        selects[lang] = {key: arms for key, value in ui[lang].items() if (arms := select_arms(value))}
    return {"schema": 1, "sources": sources, "story": entries,
            "attributeGroups": groups, "hiddenAttributes": sorted(set(hidden)), "ui": ui,
            "gameTextDefaults": defaults, "selects": selects}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            parser.exit(1, "CLI presentation metadata is stale; run build_cli_presentation.py\n")
    else:
        OUT.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
