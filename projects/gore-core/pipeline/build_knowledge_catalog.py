#!/usr/bin/env python3
"""Build projects/gore-save/app/assets/knowledge_catalog.json from a UE4SS object dump.

Usage: python tools/build_knowledge_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

Extracts Topic_/Info_/Choice* class identifiers (id, category). These are the
dialog-unlock knowledge tokens that appear in a character's Knowledge set.
Voiceline tokens are intentionally excluded (localization keys, not classes).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

# Order matters: Topic_/Info_ before bare Choice.
PATTERNS = [
    (re.compile(r"ASClass /Script/Angelscript\.(Topic_[A-Za-z0-9_]+)"), "topic"),
    (re.compile(r"ASClass /Script/Angelscript\.(Info_[A-Za-z0-9_]+)"), "info"),
    (re.compile(r"ASClass /Script/Angelscript\.(Choice[A-Za-z0-9_]+)"), "choice"),
]


def parse_dump_classes(lines) -> list[tuple[str, str]]:
    found: dict[str, str] = {}
    for line in lines:
        for rx, category in PATTERNS:
            match = rx.search(line)
            if match:
                found.setdefault(match.group(1), category)
                break
    return sorted(found.items())


def build_catalog(pairs: list[tuple[str, str]]) -> list[dict]:
    entries = [{"id": name, "category": cat} for name, cat in pairs]
    entries.sort(key=lambda e: e["id"])
    return entries


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parents[2]
        / "gore-save" / "app" / "assets" / "knowledge_catalog.json",
    )
    args = parser.parse_args()
    pairs = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries = build_catalog(pairs)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    counts: dict[str, int] = {}
    for e in entries:
        counts[e["category"]] = counts.get(e["category"], 0) + 1
    summary = ", ".join(f"{k}: {counts[k]}" for k in sorted(counts))
    print(f"wrote {len(entries)} knowledge tokens to {args.out}")
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
