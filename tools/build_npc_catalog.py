#!/usr/bin/env python3
"""Build apps/goresave/assets/npc_catalog.json from a UE4SS object dump.

Usage: python tools/build_npc_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

Extracts CharacterDefinition_* class identifiers only (id, class, category).
The `id` of a Human definition is the exact CharacterKnowledgeByUniqueName map
key (e.g. OC_STT_Diego). No localized names or stats are extracted.

Id derivation: Human_ entries are stripped to the map-key form (e.g.
OC_STT_Diego), while non-human entries keep their sub-prefix (e.g.
Creature_Biter). Anything matching neither Human_ nor Creature_ is
categorised as "other".
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

CLASS_RE = re.compile(r"ASClass /Script/Angelscript\.(CharacterDefinition_[A-Za-z0-9_]+)")

CATEGORY_BY_SUBPREFIX = [
    ("Human_", "human"),
    ("Creature_", "creature"),
]


def parse_dump_classes(lines) -> list[str]:
    names: set[str] = set()
    for line in lines:
        match = CLASS_RE.search(line)
        if match:
            names.add(match.group(1))
    return sorted(names)


def build_catalog(class_names: list[str]) -> tuple[list[dict], list[str]]:
    entries: list[dict] = []
    skipped: list[str] = []
    for cls in class_names:
        rest = cls[len("CharacterDefinition_"):]
        category = "other"
        unique = rest
        for sub, cat in CATEGORY_BY_SUBPREFIX:
            if rest.startswith(sub):
                category = cat
                if cat == "human":
                    unique = rest[len(sub):]  # map-key form
                break
        if not unique:
            skipped.append(cls)
            continue
        entries.append({"id": unique, "class": cls, "category": category})
    entries.sort(key=lambda e: e["id"])
    seen: set[str] = set()
    deduped = []
    for e in entries:
        if e["id"] in seen:
            continue
        seen.add(e["id"])
        deduped.append(e)
    return deduped, skipped


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parent.parent
        / "apps" / "goresave" / "assets" / "npc_catalog.json",
    )
    args = parser.parse_args()
    names = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries, skipped = build_catalog(names)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} npcs to {args.out}")
    from collections import Counter
    counts = Counter(e["category"] for e in entries)
    print(", ".join(f"{cat}: {counts[cat]}" for cat in sorted(counts)))
    others = [e["class"] for e in entries if e["category"] == "other"]
    if others:
        print(f"other category ({len(others)} classes):")
        for cls in others:
            print(f"  - {cls}")
    if skipped:
        print(f"skipped {len(skipped)} classes:")
        for name in skipped:
            print(f"  - {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
