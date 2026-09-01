#!/usr/bin/env python3
"""Build `assets/item_stats.json` from a decompiled AngelScript source tree.

The shipped script cache is the game's own authority on what an item is: every
`UItemDefinition` subclass carries its type tag, trade value, stack size,
damage, stat requirements, description key and icon texture as class defaults.
The save editor shows exactly those numbers, and groups the inventory by the
same `UInventoryFilter_*` tables the game's own inventory rail uses.

Regenerate after a game update:

    gore as emit-all "$GAME/G1R/Script/PrecompiledScript_Shipping.Cache" out_as
    python apps/save-editor/tools/build_item_stats.py out_as \
        --catalog apps/save-editor/assets/item_catalog.json \
        --out apps/save-editor/assets/item_stats.json

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
DEFAULT_RE = re.compile(r"^\s*default\s+(.*?);\s*$")

STRING_ASSIGN_RE = re.compile(r'^(m_\w+)\s*=\s*"(.*)"$')
NUMBER_ASSIGN_RE = re.compile(r"^(\w+)\s*=\s*(-?[\d.]+)f?$")
CLASS_ASSIGN_RE = re.compile(r"^(m_\w+)\s*=\s*(\w+)::StaticClass\(\)$")
SET_ITEM_TYPE_RE = re.compile(r"^SetItemType\(GameplayTag::(\w+)\)$")
ADD_ITEM_SPEC_RE = re.compile(r"^AddItemSpec\(GameplayTag::(\w+)\)$")
DAMAGE_RE = re.compile(r"^m_DamageBase\.Add\(GameplayTag::(\w+),\s*(-?[\d.]+)f?\)$")
REQUIRED_STAT_RE = re.compile(
    r'^m_RequiredStats\.Add\(GetAttribute\(TSubclassOf<UAttributeSet>\('
    r'\w+::StaticClass\(\)\),\s*n"(\w+)"\),\s*(?:int\()?(-?[\d.\w]+?)\)?f?\)$'
)
INCREASE_ATTRIBUTE_RE = re.compile(
    r'^IncreaseAttribute\(TSubclassOf<UAttributeSet>\('
    r'\w+::StaticClass\(\)\),\s*n"(\w+)",\s*(-?[\d.]+)f?\)$'
)
WEAPON_DEFINITION_RE = re.compile(
    r"^m_WeaponsDefinitions\.Add\(TSubclassOf<UWeaponDefinition>\((\w+)::StaticClass\(\)\)\)$"
)
WRITING_DOC_RE = re.compile(
    r"^m_WritingDocument\s*=\s*(?:[\w:]+::)?(\w+)(?:::StaticClass\(\))?$"
)
IN_DOCUMENT_RE = re.compile(r"^InDocument\s*=\s*(?:[\w:]+::)?(\w+)$")
ADDED_SEGMENT_RE = re.compile(r"^AddedSegments\.Add\((?:[\w:]+::)?(\w+)\)$")
# Loc ids are not all identifiers: the book texts use
# `Document-ItWr_Book_Arcanum-Text_1`, hyphens and all.
LOC_TEXT_RE = re.compile(
    r'(AddParagraph|AddChapterHeading)\(LocText\("([\w-]+)"\)\)'
)
LEARNS_RECIPE_RE = re.compile(
    r"^AddLearningRecipes\(TSubclassOf<UItemRecipeDefinition>\((\w+)::StaticClass\(\)\)\)$"
)
RECIPE_ITEM_RE = re.compile(
    r"^(m_RequiredIngredients|m_FinalItem)\.Add\(TSubclassOf<UItemDefinition>\("
    r"U(\w+)::StaticClass\(\)\),\s*(\d+)\)$"
)
ON_CONSUME_RE = re.compile(
    r"^AddOnConsume\(TSubclassOf<UGameplayEffect>\((\w+)::StaticClass\(\)\)\)(.*)$"
)
ADD_MAG_RE = re.compile(r"\.AddMag\(GameplayTag::GE_Param_(\w+),\s*(-?[\d.]+)f?\)")
INCREASE_PERCENT_RE = re.compile(
    r'^IncreaseAttributeByPercentage\(TSubclassOf<UAttributeSet>\('
    r'\w+::StaticClass\(\)\),\s*n"(\w+)",\s*(-?[\d.]+)f?\)$'
)
EFFECT_DURATION_RE = re.compile(
    r"^DurationMagnitude\.ScalableFloatMagnitude\.Value\s*=\s*(-?[\d.]+)f?$"
)
ADD_SPELL_LEVEL_RE = re.compile(
    r"^AddSpellLevel\((-?[\d.]+)f?,\s*(-?[\d.]+)f?,\s*(-?[\d.]+)f?,"
)
SPELL_TAG_RE = re.compile(r"^m_SpellTag\s*=\s*GameplayTag::(\w+)$")
FILTER_TAG_RE = re.compile(r"^m_FilterTag\s*=\s*GameplayTag::(\w+)$")
FILTER_NAME_RE = re.compile(r'^m_DisplayName\s*=\s*LocText\("(\w+)"\)$')
FILTER_ICON_RE = re.compile(r'^m_IconImage\s*=\s*TSoftObjectPtr<UTexture2D>\(n"([^"]+)"\)$')
FILTER_ITEM_TAG_RE = re.compile(r"^m_ItemTags\.AddTag\(GameplayTag::(\w+)\)$")
SORT_ORDER_RE = re.compile(r"^m_SortOrder\s*=\s*(\d+)$")

# Scalars copied straight through, under the name the editor uses.
SCALARS = {
    "m_Value": "value",
    "m_MaxStack": "maxStack",
    "m_Weight": "weight",
    "m_SuperArmorDamageBase": "superArmorDamage",
    "RequiredMagicCircleLevel": "magicCircle",
}


class ScriptClass:
    __slots__ = ("name", "parent", "defaults", "module", "text")

    def __init__(self, name: str, parent: str | None, module: str = "") -> None:
        self.name = name
        self.parent = parent
        self.defaults: list[str] = []
        # (kind, loc id) pairs a document segment builds its page from.
        self.text: list[tuple[str, str]] = []
        # The file the class was decompiled into. A spell configuration ships
        # in the same module as the equip effect it belongs to, and that
        # co-location is the only link between them.
        self.module = module


def parse_tree(root: Path) -> dict[str, ScriptClass]:
    classes: dict[str, ScriptClass] = {}
    for path in sorted(root.rglob("*.as")):
        current: ScriptClass | None = None
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            match = CLASS_RE.match(line)
            if match:
                current = ScriptClass(match.group(1), match.group(2), str(path))
                classes[current.name] = current
                continue
            if current is None:
                continue
            if line.startswith("}"):
                current = None
                continue
            default = DEFAULT_RE.match(line)
            if default:
                current.defaults.append(default.group(1).strip())
                continue
            # A document's own text lives in its segments' bodies, not in a
            # default — `AddParagraph(LocText("TEXT_…"))`, in reading order.
            for kind, loc_id in LOC_TEXT_RE.findall(line):
                current.text.append((kind, loc_id))
    return classes


def lineage(classes: dict[str, ScriptClass], name: str) -> list[ScriptClass]:
    """Root-first chain, so a subclass default overwrites its parent's."""
    chain: list[ScriptClass] = []
    seen: set[str] = set()
    cursor: str | None = name
    while cursor and cursor in classes and cursor not in seen:
        seen.add(cursor)
        chain.append(classes[cursor])
        cursor = classes[cursor].parent
    chain.reverse()
    return chain


# The base every creature's built-in weapon derives from. A wolf's jaw, a
# harpy's claws and a minecrawler's leg are ItemDefinitions like any other and
# sit in the creature's MeleeSlot, but the game never shows them: they carry no
# name, no icon and no catalog entry, because the player can never hold one.
NATURAL_WEAPON_BASES = ("UMeleeWeaponBase_Creature", "URangedWeaponBase_Creature")


def natural_weapons(classes: dict[str, ScriptClass]) -> list[str]:
    """Every class whose lineage reaches a creature weapon base, id-first.

    Keyed the way the save writes them — the class name without its leading
    `U`, e.g. `ScavengerJaw`.
    """
    out = []
    for name in classes:
        chain = [entry.name for entry in lineage(classes, name)]
        if name in NATURAL_WEAPON_BASES or not any(
            base in chain for base in NATURAL_WEAPON_BASES
        ):
            continue
        out.append(name)
    return sorted(out)


def resolve_item(
    classes: dict[str, ScriptClass],
    spell_configs: tuple[dict[str, str], dict[str, str]],
    class_name: str,
) -> dict:
    out: dict = {}
    specs: list[str] = []
    damage: dict[str, float] = {}
    requirements: dict[str, float] = {}
    spells: list[str] = []
    equip_effect: str | None = None
    consume: list[dict] = []
    learns: list[str] = []
    writing: str | None = None

    for entry in lineage(classes, class_name):
        for statement in entry.defaults:
            if (m := SET_ITEM_TYPE_RE.match(statement)) is not None:
                out["itemType"] = m.group(1)
            elif (m := ADD_ITEM_SPEC_RE.match(statement)) is not None:
                if m.group(1) not in specs:
                    specs.append(m.group(1))
            elif (m := DAMAGE_RE.match(statement)) is not None:
                damage[m.group(1)] = float(m.group(2))
            elif (m := REQUIRED_STAT_RE.match(statement)) is not None:
                raw = m.group(2)
                try:
                    requirements[m.group(1)] = float(raw)
                except ValueError:
                    # `int(RequiredMagicCircleLevel)` — a scalar default that a
                    # later pass over this same lineage already captured.
                    requirements[m.group(1)] = raw
            elif (m := WRITING_DOC_RE.match(statement)) is not None:
                writing = m.group(1)
            elif (m := LEARNS_RECIPE_RE.match(statement)) is not None:
                if m.group(1) not in learns:
                    learns.append(m.group(1))
            elif (m := ON_CONSUME_RE.match(statement)) is not None:
                effect = m.group(1).removeprefix("UGE_Item_")
                params = {
                    key: _trim(float(value))
                    for key, value in ADD_MAG_RE.findall(m.group(2))
                }
                # Most items pass their own magnitudes in; the ones that do not
                # leave them on the effect class itself.
                declared = resolve_consume_effect(classes, m.group(1))
                for key, value in declared.get("params", {}).items():
                    params.setdefault(key, value)
                entry_effect = {"effect": effect}
                if params:
                    entry_effect["params"] = params
                if declared.get("percent"):
                    entry_effect["percent"] = declared["percent"]
                if entry_effect not in consume:
                    consume.append(entry_effect)
            elif (m := WEAPON_DEFINITION_RE.match(statement)) is not None:
                if m.group(1) not in spells:
                    spells.append(m.group(1))
            elif (m := STRING_ASSIGN_RE.match(statement)) is not None:
                field, value = m.group(1), m.group(2)
                if field == "m_Description":
                    out["descriptionKey"] = value
                elif field == "m_Icon":
                    out["iconAsset"] = value
            elif (m := CLASS_ASSIGN_RE.match(statement)) is not None:
                if m.group(1) == "m_OnEquipEffect":
                    equip_effect = m.group(2)
            elif (m := NUMBER_ASSIGN_RE.match(statement)) is not None:
                key = SCALARS.get(m.group(1))
                if key is not None:
                    number = float(m.group(2))
                    out[key] = int(number) if number.is_integer() else number

    # A requirement expressed as `int(RequiredMagicCircleLevel)` resolves against
    # the scalar of the same lineage.
    for stat, value in list(requirements.items()):
        if isinstance(value, str):
            resolved = out.get(SCALARS.get(value, ""), None)
            if resolved is None:
                requirements.pop(stat)
            else:
                requirements[stat] = resolved

    # A negative level is the game's "none needed" sentinel, not a requirement:
    # the teleport runes all carry RequiredMagicCircleLevel = -1. Kept it would
    # read as "Magic circle -1" on the card.
    for stat, value in list(requirements.items()):
        if isinstance(value, (int, float)) and value < 0:
            requirements.pop(stat)
    for key in ("magicCircle",):
        if isinstance(out.get(key), (int, float)) and out[key] < 0:
            out.pop(key)

    if damage:
        out["damage"] = {k: _trim(v) for k, v in sorted(damage.items())}
    if requirements:
        out["requires"] = {k: _trim(v) for k, v in sorted(requirements.items())}
    if specs:
        out["specs"] = sorted(specs)
    if spells:
        levels = [resolve_spell_level(classes, spell) for spell in spells]
        if any(levels):
            out["spellLevels"] = levels
    if consume:
        out["onConsume"] = consume
    if writing:
        text = resolve_document(classes, writing)
        if text:
            out["writing"] = text
    if learns:
        # Declaration order IS production order: the blank first, then the
        # rough piece, then the finished weapon.
        steps = [resolve_recipe(classes, name) for name in learns]
        steps = [step for step in steps if step]
        if steps:
            out["teaches"] = steps
    if equip_effect:
        effects = resolve_equip_effect(classes, equip_effect)
        if effects:
            out["onEquip"] = effects
    # The item's own tag first, since that is the binding the containers were
    # written for. Only where no container claims the tag — the heal, sleep,
    # charm and telekinesis scrolls, and every transform scroll — fall back to
    # the module the item's equip effect ships in.
    config = spell_configs[0].get(out["itemType"]) if "itemType" in out else None
    if config is None and equip_effect is not None:
        module = classes[equip_effect].module if equip_effect in classes else ""
        config = spell_configs[1].get(module)
    if config is not None:
        mana = resolve_spell_mana(classes, config)
        if mana:
            out["spellMana"] = mana
    return out


# The bench a recipe is worked at, from the base class it derives from.
RECIPE_STATIONS = {
    "UForgeRecipe": "forge",
    "UWhetstoneRecipe": "whetstone",
    "UWorkbenchRecipe": "workbench",
    "UInscriptionRecipe": "inscription",
    "UAlchemyRecipe": "alchemy",
    "UCauldronRecipe": "cauldron",
}


def index_document_segments(
    classes: dict[str, ScriptClass],
) -> dict[str, list[ScriptClass]]:
    """Document class -> the segments that declare themselves part of it."""
    out: dict[str, list[ScriptClass]] = {}
    for entry in classes.values():
        for statement in entry.defaults:
            m = IN_DOCUMENT_RE.match(statement)
            if m:
                out.setdefault(m.group(1), []).append(entry)
                break
    return out


DOCUMENT_SEGMENTS: dict[str, list[ScriptClass]] = {}


def resolve_document(classes: dict[str, ScriptClass], document: str) -> list[dict]:
    """What a book or letter actually says, in reading order.

    Two thirds of the writings carry no description at all — their content IS
    the item. The text is not a default: each segment builds its page in a
    function body out of `LocText` ids, which is where this reads it from.
    """
    segments = DOCUMENT_SEGMENTS.get(document, [])
    if not segments:
        return []
    listed: list[str] = []
    for entry in lineage(classes, document):
        for statement in entry.defaults:
            m = ADDED_SEGMENT_RE.match(statement)
            if m and m.group(1) not in listed:
                listed.append(m.group(1))
    # A segment the document lists comes first, in the order it lists them; the
    # rest keep the order they were declared in.
    order = {name: i for i, name in enumerate(listed)}
    segments = sorted(segments, key=lambda e: order.get(e.name, len(order)))
    out: list[dict] = []
    for entry in segments:
        for kind, loc_id in entry.text:
            out.append({
                "kind": "heading" if kind == "AddChapterHeading" else "text",
                "id": loc_id,
            })
    return out


def resolve_recipe(classes: dict[str, ScriptClass], recipe: str) -> dict:
    """One step of a crafting chain: what it consumes and what it yields.

    A blueprint teaches a whole chain — two iron into a blade, the blade into a
    rough sword, the rough sword over the whetstone into the finished one — so
    every step carries its own ingredients.
    """
    needs: dict[str, int] = {}
    makes: dict[str, int] = {}
    station: str | None = None
    for entry in lineage(classes, recipe):
        if entry.parent in RECIPE_STATIONS:
            station = RECIPE_STATIONS[entry.parent]
        for statement in entry.defaults:
            m = RECIPE_ITEM_RE.match(statement)
            if not m:
                continue
            target = needs if m.group(1) == "m_RequiredIngredients" else makes
            target[m.group(2)] = int(m.group(3))
    if not makes:
        return {}
    step: dict = {"makes": dict(sorted(makes.items()))}
    if needs:
        step["needs"] = dict(sorted(needs.items()))
    if station:
        step["station"] = station
    return step


def resolve_consume_effect(classes: dict[str, ScriptClass], effect: str) -> dict:
    """What a consume effect declares about itself.

    An item normally passes its magnitudes in through `AddMag`, but a few
    effects carry their own — a percentage bonus and how long it lasts — and
    the item names nothing but the class.
    """
    percent: dict[str, float] = {}
    params: dict[str, float] = {}
    for entry in lineage(classes, effect):
        for statement in entry.defaults:
            if (m := INCREASE_PERCENT_RE.match(statement)) is not None:
                percent[m.group(1)] = float(m.group(2))
            elif (m := EFFECT_DURATION_RE.match(statement)) is not None:
                params["Duration"] = float(m.group(1))
    out: dict = {}
    if percent:
        out["percent"] = {k: _trim(v) for k, v in sorted(percent.items())}
    if params:
        out["params"] = {k: _trim(v) for k, v in sorted(params.items())}
    return out


def resolve_spell_level(classes: dict[str, ScriptClass], definition: str) -> dict:
    """Damage of one rune/scroll spell level, as its projectile definition
    declares it. The game's tooltip lists these level by level."""
    damage: dict[str, float] = {}
    for entry in lineage(classes, definition):
        for statement in entry.defaults:
            m = DAMAGE_RE.match(statement)
            if m:
                damage[m.group(1)] = float(m.group(2))
    return {k: _trim(v) for k, v in sorted(damage.items())}


def index_spell_configs(
    classes: dict[str, ScriptClass],
) -> tuple[dict[str, str], dict[str, str]]:
    """Two ways to reach a spell configuration: by tag, and by module.

    A `USpellConfigurationContainer_*` normally binds its configuration to the
    very tag a rune or scroll declares as its item type. Some do not: the heal,
    sleep, charm and telekinesis scrolls hand their casting to the RUNE's
    container, and all seventeen transform scrolls share the one parent tag. For
    those the link is co-location — the container ships in the same module as
    the equip effect the item names — which is what the module index is for.
    """
    by_tag: dict[str, str] = {}
    by_module: dict[str, str] = {}
    for name, entry_class in classes.items():
        if not name.startswith("USpellConfigurationContainer_"):
            continue
        tag: str | None = None
        config: str | None = None
        for entry in lineage(classes, name):
            for statement in entry.defaults:
                if (m := SPELL_TAG_RE.match(statement)) is not None:
                    tag = m.group(1)
                elif (m := CLASS_ASSIGN_RE.match(statement)) is not None:
                    if m.group(1) == "m_SpellConfigClass":
                        config = m.group(2)
        if not config:
            continue
        if tag:
            # Several containers claim the same tag for the fire spells; the
            # last one wins, which is the pairing the game's own tooltip shows.
            by_tag[tag] = config
        # A module carrying two containers cannot say which belongs to the
        # effect, so it is left out rather than guessed at.
        by_module[entry_class.module] = (
            None if entry_class.module in by_module else config
        )
    return by_tag, {k: v for k, v in by_module.items() if v}


def resolve_spell_mana(
    classes: dict[str, ScriptClass],
    config: str | None,
) -> list[dict]:
    """Mana per spell level: `AddSpellLevel(initial, chargeTime, upkeep, …)`.

    The first argument is what THIS level ADDS to the initial cost, which is why
    the game shows a rising 5/6/7/9 for a spell declared as 5/1/1/2. The third
    is the per-second upkeep of a continuous spell.
    """
    if config is None:
        return []

    levels: list[dict] = []
    initial = 0.0
    for entry in lineage(classes, config):
        for statement in entry.defaults:
            m = ADD_SPELL_LEVEL_RE.match(statement)
            if not m:
                continue
            initial += float(m.group(1))
            level = {"initialMana": _trim(initial)}
            upkeep = float(m.group(3))
            if upkeep:
                level["manaPerSecond"] = _trim(upkeep)
            levels.append(level)
    return levels


def resolve_equip_effect(classes: dict[str, ScriptClass], effect: str) -> dict:
    """Attribute deltas an equipped item grants (armour protection, rings…).

    Zero deltas are dropped: the shipped effects list every attribute they could
    touch and leave the untouched ones at 0.
    """
    values: dict[str, float] = {}
    for entry in lineage(classes, effect):
        for statement in entry.defaults:
            m = INCREASE_ATTRIBUTE_RE.match(statement)
            if m:
                values[m.group(1)] = float(m.group(2))
    return {k: _trim(v) for k, v in sorted(values.items()) if v != 0.0}


def _trim(value):
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return value


def resolve_filters(classes: dict[str, ScriptClass]) -> list[dict]:
    filters: list[dict] = []
    for name, entry in classes.items():
        if not name.startswith("UInventoryFilter_"):
            continue
        row: dict = {"id": name[len("UInventoryFilter_") :], "itemTags": []}
        for statement in entry.defaults:
            if (m := FILTER_TAG_RE.match(statement)) is not None:
                row["filterTag"] = m.group(1)
            elif (m := FILTER_NAME_RE.match(statement)) is not None:
                row["nameKey"] = m.group(1)
            elif (m := FILTER_ICON_RE.match(statement)) is not None:
                row["icon"] = m.group(1).split("/")[-1].split(".")[0]
            elif (m := FILTER_ITEM_TAG_RE.match(statement)) is not None:
                row["itemTags"].append(m.group(1))
            elif (m := SORT_ORDER_RE.match(statement)) is not None:
                row["sortOrder"] = int(m.group(1))
        if "filterTag" in row:
            filters.append(row)
    filters.sort(key=lambda row: row.get("sortOrder", 1 << 20))
    return filters


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tree", type=Path, help="`gore as emit-all` output directory")
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    classes = parse_tree(args.tree)
    if not any(entry.defaults for entry in classes.values()):
        print(
            "no class defaults in the tree: emit it with a gore-as build that "
            "writes `default` statements",
            file=sys.stderr,
        )
        return 2

    spell_configs = index_spell_configs(classes)
    DOCUMENT_SEGMENTS.update(index_document_segments(classes))
    catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
    items: dict[str, dict] = {}
    missing: list[str] = []
    for entry in catalog:
        item_id = entry["id"]
        resolved = resolve_item(classes, spell_configs, f"U{item_id}")
        if not resolved:
            missing.append(item_id)
            continue
        items[item_id] = resolved

    # What a raw material is FOR. Two thirds of the smithing stock carries no
    # description and no numbers, so the only useful thing to say about a lump
    # of iron is what can be made from it.
    used_in: dict[str, list[str]] = {}
    for owner in items.values():
        for step in owner.get("teaches", []):
            for product in step.get("makes", {}):
                for ingredient in step.get("needs", {}):
                    made = used_in.setdefault(ingredient, [])
                    if product not in made:
                        made.append(product)
    for item_id, made in used_in.items():
        if item_id in items and not items[item_id].get("teaches"):
            items[item_id]["ingredientFor"] = sorted(made)

    # The creatures' own weapons, which no item catalog lists. Without them a
    # save's `WolfJaw` row had no name, no icon and no card at all.
    for class_name in natural_weapons(classes):
        item_id = class_name[1:] if class_name.startswith("U") else class_name
        if item_id in items:
            continue
        resolved = resolve_item(classes, spell_configs, class_name)
        if not resolved:
            continue
        resolved["naturalWeapon"] = True
        items[item_id] = resolved

    filters = resolve_filters(classes)
    document = {
        "schema": 1,
        "filters": filters,
        "items": dict(sorted(items.items())),
    }
    args.out.write_text(
        json.dumps(document, ensure_ascii=False, indent=1, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    print(f"{len(items)} items, {len(filters)} inventory filters -> {args.out}")
    if missing:
        print(f"{len(missing)} catalog ids had no class defaults: {missing[:10]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
