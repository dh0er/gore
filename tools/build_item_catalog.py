#!/usr/bin/env python3
"""Build apps/goresave/assets/item_catalog.json from a UE4SS object dump.

Usage:
    python tools/build_item_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

The dump must come from Gothic 1 Remake with UE4SS's DumpAllObjects().
Only class identifiers are extracted (id, path, category) - no localized
names, stats, or other game content.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

CLASS_RE = re.compile(r"ASClass /Script/Angelscript\.(It[A-Za-z0-9_]+)")

CATEGORY_BY_PREFIX = [
    ("ItMw_", "melee_weapon"),
    ("ItRw_", "ranged_weapon"),
    ("ItAr_Rune_", "rune"),
    ("ItAr_Scroll_", "scroll"),
    ("ItFo_", "food"),
    ("ItMi_", "misc"),
    ("ItAt_", "trophy"),
    ("ItWr_", "writing"),
    ("ItMs_", "mission"),
    ("ItKe_", "key"),
    ("ItAm_", "amulet"),
]

# Non-inventory classes that match the It* scan.
EXCLUDE_PREFIXES = (
    "ItemAnimConfig",
    "ItemSpawnManagerConfig",
    "ItemCollisionFX",
    "ItemVisualWorldTargetConfig",
    "ItAI_",
)

# Known singletons that carry no category prefix.
EXPLICIT = {
    "ItKeyDefault": "key",
    "ItChestKey01": "key",
    "ItDoorKey01": "key",
    "ItIg_Worldsplitter": "special",
    "ItFocusStoneBridgeItem": "special",
}


def parse_dump_classes(lines) -> list[str]:
    names: set[str] = set()
    for line in lines:
        match = CLASS_RE.search(line)
        if match:
            names.add(match.group(1))
    return sorted(names)


def build_catalog(names: list[str]) -> tuple[list[dict], list[str]]:
    entries: list[dict] = []
    skipped: list[str] = []
    for name in names:
        if name.startswith(EXCLUDE_PREFIXES) or name.endswith("_Base"):
            skipped.append(name)
            continue
        category = EXPLICIT.get(name)
        if category is None:
            for prefix, cat in CATEGORY_BY_PREFIX:
                if name.startswith(prefix):
                    category = cat
                    break
        if category is None:
            category = "special"
            skipped.append(f"{name} (unmatched prefix -> special)")
        entries.append({
            "id": name,
            "path": f"/Script/Angelscript.{name}",
            "category": category,
        })
    entries.sort(key=lambda e: e["id"])
    return entries, skipped


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parent.parent
        / "apps" / "goresave" / "assets" / "item_catalog.json",
    )
    args = parser.parse_args()

    names = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries, skipped = build_catalog(names)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} items to {args.out}")
    if skipped:
        print(f"skipped {len(skipped)} classes:")
        for name in skipped:
            print(f"  - {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
