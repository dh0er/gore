#!/usr/bin/env python3
"""Build `assets/glossary_images.json` from a decompiled AngelScript source tree.

The glossary portraits are NOT in the game's asset container: they are loose
PNGs under `G1R/Story/Conversation/images/Glossary/{Characters,Creatures,
Locations}/T_GlossaryImage_<name>_{M,S}.png`, next to the localization cache and
the voice-over archives. Each `Document_Glossary_*` class names its own pair:

    default m_BannerImage    = ...Assets:images/Glossary/Characters/T_GlossaryImage_Diego_M
    default m_ThumbnailImage = ...Assets:images/Glossary/Characters/T_GlossaryImage_Diego_S

This maps the document class the save editor already knows to that file name, so
the editor can read the portrait straight off the user's installation.

Regenerate after a game update:

    gore as emit-all "$GAME/G1R/Script/PrecompiledScript_Shipping.Cache" out_as
    python apps/save-editor/tools/build_glossary_images.py out_as \
        --out apps/save-editor/assets/glossary_images.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

KINDS = ("Characters", "Creatures", "Locations")
ARTWORK_DIRECTORY = "G1R/Story/Conversation/images/Glossary"
ARTWORK_RE = re.compile(r"^T_GlossaryImage_(?P<name>.+)_S\.png$", re.IGNORECASE)

CLASS_RE = re.compile(r"^class\s+(\w+)\s*(?::\s*(\w+))?\s*$")
IMAGE_RE = re.compile(
    r'^\s*default\s+(m_BannerImage|m_ThumbnailImage)\s*=\s*'
    r'TSoftObjectPtr<UTexture2D>\(n"[^"]*images/Glossary/'
    r'(?P<kind>Characters|Creatures|Locations)/T_GlossaryImage_(?P<name>\w+)_[MS]"\);\s*$'
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "tree",
        type=Path,
        nargs="?",
        help="`gore as emit-all` output directory",
    )
    parser.add_argument(
        "--game",
        type=Path,
        help="game installation root, to index the artwork files themselves",
    )
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    if args.tree is None and args.game is None:
        print("give a script tree, --game, or both", file=sys.stderr)
        return 2

    previous: dict[str, object] = {}
    if args.out.is_file():
        previous = json.loads(args.out.read_text(encoding="utf-8"))

    artwork = _artwork(args.game) if args.game else previous.get("artwork") or {}
    if args.tree is None:
        images = previous.get("images") or {}
        return _write(args.out, images, artwork)

    images: dict[str, dict[str, str]] = {}
    for path in sorted(args.tree.rglob("*.as")):
        current: str | None = None
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            match = CLASS_RE.match(line)
            if match:
                current = match.group(1)
                continue
            if current is None or not current.startswith("UDocument_Glossary_"):
                continue
            image = IMAGE_RE.match(line)
            if image:
                # Both fields name the same artwork; the editor picks the size.
                images[current[1:]] = {
                    "kind": image.group("kind"),
                    "name": image.group("name"),
                }

    return _write(args.out, images, artwork)


def _artwork(game: Path) -> dict[str, dict[str, str]]:
    """Every artwork file in the installation, keyed by its own name.

    About thirty of them belong to no glossary document at all.
    """
    out: dict[str, dict[str, str]] = {}
    root = game / Path(ARTWORK_DIRECTORY)
    for kind in KINDS:
        directory = root / kind
        if not directory.is_dir():
            continue
        for entry in sorted(directory.iterdir()):
            match = ARTWORK_RE.match(entry.name)
            if not match:
                continue
            # A detail view shows the banner; an artwork missing one would be
            # unusable there, so only complete pairs are indexed.
            banner = directory / f"T_GlossaryImage_{match.group('name')}_M.png"
            if not banner.is_file():
                continue
            out[match.group("name")] = {"kind": kind}
    return out


def _write(
    out: Path,
    images: dict[str, dict[str, str]],
    artwork: dict[str, dict[str, str]],
) -> int:
    if not images:
        print(
            "no glossary images in the tree: emit it with a gore-as build that "
            "writes `default` statements",
            file=sys.stderr,
        )
        return 2
    document = {
        "schema": 1,
        "images": dict(sorted(images.items())),
        "artwork": dict(sorted(artwork.items())),
    }
    out.write_text(
        json.dumps(document, ensure_ascii=False, indent=1, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    print(f"{len(images)} glossary portraits, {len(artwork)} artwork files -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
