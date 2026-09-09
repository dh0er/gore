#!/usr/bin/env python3
"""Build `assets/lock_catalog.json` from a decompiled AngelScript source tree.

A save records only the locks the player has already opened, as a set of names
in `LockPersistentData.m_UnlockedLocks`. To show every lock in the game — and
to let one be locked again — the editor needs the full list from somewhere
else, and the shipped script cache is the game's own authority on it.

Every lockable chest and door is an `InteractiveObjects/*` class whose defaults
carry the lock:

    class UIO_OC_CHEST_DEXTER : UIoChestDefault
    {
        default m_UniqueName = n"IO_OC_CHEST_DEXTER";
        default m_Lock = n"OC_Chest_Dexter_Lock";
        default m_LockDifficulty = 3;
    }

`m_UniqueName` — not `m_Lock` — is what the save writes, so that is the key
this catalog is addressed by. A lock counts as one when the class carries any
of `m_Lock`, `m_LockDifficulty` or `m_Keys`, or when `RandomLockPools`
registers the chest for the randomized-lock subsystem; the last source is the
only reason `IO_AM_CHEST_05` appears in real saves at all.

Regenerate after a game update:

    gore as emit-all "$GAME/G1R/Script/PrecompiledScript_Shipping.Cache" out_as
    python apps/save-editor/tools/build_lock_catalog.py out_as \
        --locations apps/save-editor/assets/location_catalog.json \
        --out apps/save-editor/assets/lock_catalog.json

`gore as emit-all` must be a build that emits class `default` statements;
without them this script has nothing to read and refuses to write a file.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

CLASS_RE = re.compile(r"^class\s+(\w+)\s*(?::\s*(\w+))?\s*$")
UNIQUE_NAME_RE = re.compile(r'^\s*default\s+m_UniqueName\s*=\s*n"([^"]+)";')
LOCK_RE = re.compile(r'^\s*default\s+m_Lock\s*=\s*n"([^"]+)";')
DIFFICULTY_RE = re.compile(r"^\s*default\s+m_LockDifficulty\s*=\s*(\d+);")
KEY_RE = re.compile(r'^\s*default\s+m_Keys\.Add\(n"([^"]+)"\);')
INTERACTION_RE = re.compile(
    r"^\s*default\s+m_DefaultInteraction\s*=\s*GameplayTag::(\w+);"
)
REGISTER_CHEST_RE = re.compile(
    r"^\s*default\s+RegisterChest\(TSubclassOf<UIoChestDefault>\("
    r"(\w+)::StaticClass\(\)\)\);"
)

# The scan is limited to this subtree: every lockable chest and door lives in
# it, and reading the whole emitted tree would mean parsing ~7300 files to find
# them.
SOURCE_DIR = "InteractiveObjects"

# Dev fixtures that ship in the retail cache but stand behind no placed actor.
# `TestChests` is a whole module of them; `UExampleDoor` is the template the
# door classes are cloned from, and its "key" is the literal `LockExampleA`.
SKIPPED_MODULES = {"TestChests"}
SKIPPED_CLASSES = {"UExampleDoor"}

# A chest is one by base class, a door by the interaction the game offers on
# it. Both are stated in the defaults, so neither has to be guessed from a name.
CHEST_BASES = {"UIoChestDefault", "UIoLootDefault"}
DOOR_INTERACTION_PREFIX = "Action_Door"

# A handful of doors spell their area out instead of using its code, and sit at
# a placement the location catalog does not carry. These are the game's own
# long forms for areas that do have a code.
AREA_ALIASES = {
    "ABANDONEDMINE": "AM",
    "ORCGRAVEYARD": "OG",
    "SUNKENTOWER": "SNT",
}


class LockClass:
    """One `InteractiveObjects` class, reduced to what a lock needs."""

    def __init__(self, name: str, base: str | None, module: str) -> None:
        self.name = name
        self.base = base
        self.module = module
        self.unique_name: str | None = None
        self.lock: str | None = None
        self.difficulty: int | None = None
        self.keys: list[str] = []
        self.interaction: str | None = None

    @property
    def carries_lock(self) -> bool:
        return bool(self.lock or self.difficulty or self.keys)

    @property
    def kind(self) -> str | None:
        if self.base in CHEST_BASES:
            return "chest"
        if self.interaction and self.interaction.startswith(DOOR_INTERACTION_PREFIX):
            return "door"
        return None


def parse_module(path: Path, module: str) -> list[LockClass]:
    """Every class in one emitted `.as` file, with its lock defaults."""
    classes: list[LockClass] = []
    current: LockClass | None = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = CLASS_RE.match(line)
        if match:
            current = LockClass(match.group(1), match.group(2), module)
            classes.append(current)
            continue
        if current is None:
            continue
        if match := UNIQUE_NAME_RE.match(line):
            current.unique_name = match.group(1)
        elif match := LOCK_RE.match(line):
            current.lock = match.group(1)
        elif match := DIFFICULTY_RE.match(line):
            current.difficulty = int(match.group(1))
        elif match := KEY_RE.match(line):
            current.keys.append(match.group(1))
        elif match := INTERACTION_RE.match(line):
            current.interaction = match.group(1)
    return classes


def parse_random_pool(path: Path) -> set[str]:
    """Class names the randomized-lock subsystem registers as lockable."""
    if not path.is_file():
        return set()
    registered: set[str] = set()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if match := REGISTER_CHEST_RE.match(line):
            registered.add(match.group(1))
    return registered


def load_areas(catalog_path: Path | None) -> tuple[dict[str, str], list[str]]:
    """`{upper-cased spot name: area code}` plus the known area codes.

    The location catalog is built from the game's own `InteractionSpots.json`,
    so a lock's own placement is usually already in it; the area codes are the
    fallback for the rest.
    """
    if catalog_path is None:
        return {}, []
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    spots = {
        spot["n"].upper(): spot.get("a") or ""
        for spot in catalog.get("spots", [])
        if spot.get("n")
    }
    # Longest first, so `OTOWN` wins over `OT` and `SNT` over `SC`.
    codes = sorted(
        (area["id"] for area in catalog.get("areas", [])),
        key=len,
        reverse=True,
    )
    return spots, codes


def resolve_area(name: str, spots: dict[str, str], codes: list[str]) -> str:
    """The area a lock sits in: its own placement first, then its name prefix."""
    placed = spots.get(name.upper())
    if placed:
        return placed
    # `IO_OC_CHEST_DEXTER` and `OC_Santino_Door` both carry the code as the
    # first name part after an optional `IO_`.
    stem = name[3:] if name.upper().startswith("IO_") else name
    upper = stem.upper()
    for code in codes:
        if upper.startswith(f"{code}_"):
            return code
    for long_form, code in AREA_ALIASES.items():
        if upper.startswith(f"{long_form}_"):
            return code
    return ""


def build(
    source_root: Path, location_catalog: Path | None
) -> tuple[dict[str, object], dict[str, int]]:
    source_dir = source_root / SOURCE_DIR
    if not source_dir.is_dir():
        raise SystemExit(
            f"no {SOURCE_DIR}/ under {source_root} — pass the root of an "
            f"`gore as emit-all` tree"
        )

    classes: list[LockClass] = []
    for path in sorted(source_dir.glob("*.as")):
        module = path.stem
        if module in SKIPPED_MODULES:
            continue
        classes.extend(parse_module(path, module))

    registered = parse_random_pool(source_dir / "RandomLockPools.as")
    spots, codes = load_areas(location_catalog)

    locks: list[dict[str, object]] = []
    for entry in classes:
        if entry.name in SKIPPED_CLASSES or not entry.unique_name:
            continue
        randomized = entry.name in registered
        if not entry.carries_lock and not randomized:
            continue
        kind = entry.kind
        if kind is None:
            continue
        row: dict[str, object] = {
            "n": entry.unique_name,
            "k": kind,
            "a": resolve_area(entry.unique_name, spots, codes),
        }
        if entry.difficulty is not None:
            row["d"] = entry.difficulty
        if entry.lock:
            row["l"] = entry.lock
        if entry.keys:
            row["keys"] = entry.keys
        if randomized:
            row["r"] = True
        locks.append(row)

    locks.sort(key=lambda row: str(row["n"]).upper())
    seen: set[str] = set()
    for row in locks:
        name = str(row["n"])
        if name in seen:
            raise SystemExit(f"duplicate lock name in the source tree: {name}")
        seen.add(name)

    report = {
        "classes": len(classes),
        "locks": len(locks),
        "chests": sum(1 for row in locks if row["k"] == "chest"),
        "doors": sum(1 for row in locks if row["k"] == "door"),
        "with_difficulty": sum(1 for row in locks if "d" in row),
        "with_key": sum(1 for row in locks if "keys" in row),
        "randomized": sum(1 for row in locks if "r" in row),
        "without_area": sum(1 for row in locks if not row["a"]),
    }
    return {"version": 1, "locks": locks}, report


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "source",
        type=Path,
        help="root of an `gore as emit-all` tree (the directory holding InteractiveObjects/)",
    )
    parser.add_argument(
        "--locations",
        type=Path,
        default=None,
        help="location_catalog.json, to resolve each lock's area",
    )
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)

    catalog, report = build(args.source, args.locations)
    if not catalog["locks"]:
        raise SystemExit(
            "no locks found — was the tree emitted by a build that writes "
            "class `default` statements?"
        )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(catalog, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    for key, value in report.items():
        print(f"{key}: {value}")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
