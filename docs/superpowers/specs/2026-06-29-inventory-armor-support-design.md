# Inventory Armor Support — Design

Date: 2026-06-29
Status: Draft (pending user review)

## Problem

The save editor does not surface armor as armor and offers no way to add it.
Reported by the user: "the player can have multiple armors and switch them, so
armor must be carried in the bag too — but the editor shows nothing and offers
no way to add it."

Verified actual behavior (probed `inspect_save` on the user's G1R-019):

- **Display**: armor rows ARE returned by the core scan (the display filter
  `looks_item_definition_path` accepts any `/Script/Angelscript.*` path). All
  three of the save's armors appear in `private.inventory.items`. But the UI
  categorizes by class-name prefix (`itemCategoryFromId`), and armor classes
  match no `It*` prefix, so they fall into the **`Other`** category with an
  auto-generated name and no equipped indicator — effectively invisible to a
  user looking for an "Armor" section.
- **Add**: armor is genuinely not addable. The add picker and the core's
  addItem allow-list are both built from `item_catalog.json`, generated with a
  hard `name.starts_with("It")` filter — armor (`<Faction>_Armor_*`,
  `Armor_<Camp>_<NPC>_NNN`) is excluded, so it never appears in the picker and
  `addItem` rejects the path.
- **Equip / upgrades**: not represented at all.

Root cause for the catalog/category gaps: the item pipeline was defined as "the
`It*` class universe" and armor lives outside it. A scoping gap, not an
intentional exclusion.

## Verified data model

Reverse-engineered from real saves (`work/decompressed/G1R-016/019/020.host.bin`,
decoded from the user's `G1R-0{16,19,20}.sav`) and the UE4SS object dump.

### Armor is a normal inventory item

Armor occupies a standard inventory slot, structurally identical to every other
item:

```
m_SlotData (ItemSlot) → m_ItemDefinition (ObjectProperty) = /Script/Angelscript.<ArmorClass>
                      → m_ItemCount (IntProperty)
                      → m_Payload (ItemPayload)
```

The item-definition class carries gameplay tags `Item.Armor`,
`Item.Property.Wereable`, `Item.Property.NonDroppable`.

### Inventory is a map of typed containers

The player inventory is a `TMap<EInventoryTypes, Container>`:

- `m_Keys` — array of `EInventoryTypes` enum values
- `m_Values.Items[i].m_Slots` — that container's slot array

Container types observed: `MainContainer` (the bag), `ArmorSlot`, `MeleeSlot`,
`RangedSlot`, `RingLeft`, `RingRight`, `Amulet`, `TorchSlot`, `Pouch`,
`QuickItems`, `Trader`, `TradingBalance`.

### Equipped vs carried = which container

The worn armor is the item in the **`ArmorSlot`** container; carried armors sit
in **`MainContainer`**. Verified:

| Save | `ArmorSlot` container (worn) | `MainContainer` (bag) |
|------|------------------------------|------------------------|
| G1R-019 | `Ore_Armor_H` | `Crw_Armor_H`, `Ore_Armor_M` |
| G1R-016 | `Org_Armor` (no upgrades) | — |
| G1R-020 | `Org_Armor` (Heavy02 upgrades) | — |

`ArmorSlot` holds at most one armor. (Each slot also carries an inline
`m_InventoryType` enum field matching its container key; the container key is
the authoritative grouping the existing code resolves via `m_Keys`/`m_Values`.)

### Armor upgrades = string-map on the worn slot's payload

Upgrades are stored on the worn armor slot's `m_Payload.m_GenericData`
(`ReplicatedStringMap`):

- Keys (`m_Keys`, NameProperty): `m_CurrentUpperBodyUpgrade`,
  `m_CurrentMidBodyUpgrade`, `m_CurrentLowerBodyUpgrade`
- Values (`m_Values`, StrProperty):
  - G1R-016 (un-upgraded): all empty
  - G1R-020 (upgraded): `m_UpperBody_Heavy02_ArmorUpgrade`,
    `m_MidBody_Heavy02_ArmorUpgrade`, `m_LowerBody_Heavy02_ArmorUpgrade`

This reuses the existing generic `ItemPayload.m_GenericData` mechanism — no new
property type. The available upgrade variants per armor (e.g. the
`Org_Armor_Top/Mid/Bot_{L,M,H}_0{1,2}` set) are listed in a separate
`BoughtArmorUpgrades` struct, but the *applied* selection is the three
string-map values above.

### Armor item-class discriminator (for the catalog)

From the UE4SS object dump, armor classes appear as `ASClass` entries. The
clean discriminator:

- **Include**: classes whose name matches an armor family
  (`<Fac>_Armor[...]` such as Ore/Crw/Org/Vlk/Sfb/Dmb/Ebr/Gur/Kdf/Kdw/Grd/
  Nov/Sld/Stt/Tpl/Law/NH/Ryl/QA, with optional `_{Top,Mid,Bot}_{L,M,H}_NN`
  tier suffixes) **or** per-NPC `Armor_<CAMP>_<NPC>_NNN`.
- **Exclude**: the paired non-item classes — anything ending in
  `_VisualsDefinition` / `_VisualDefinition`, plus `ArmorVisualsDefinition_*`,
  `BaseArmorDefinition`, and the existing non-item noise families (`GE_*`,
  `GA_*`, `GC_*`, `CS_*`, `Choice*`, `Document*`, `Conversation*`,
  `DailyRoutine*`, `Module_*`, `AIAgent*`, `CharacterDefinition*`,
  `CharacterVisuals*`, `AllArmors*`, `Quest*`, `Memory*`, `Spawner*`).

This yields ~116 armor item classes. The filter MUST be validated against the
three known real-save armors (`Ore_Armor_H`, `Crw_Armor_H`, `Ore_Armor_M`,
`Org_Armor`) — they must all be present in the generated catalog.

## Scope

Full feature (user-approved A+B+C):

- **A. Display + add to bag** — show all armor (worn + carried) with an
  "equipped" indicator; add armor to `MainContainer`.
- **B. Equip / unequip** — move armor between `MainContainer` and `ArmorSlot`.
- **C. Upgrade editing** — view and edit the worn armor's three upgrade
  string-map values.

## Design

### Layer 1 — Catalog (`crates/gore-catalog`)

- `pipeline.rs`: extend `parse_item_classes` (or add a parallel armor pass) to
  collect armor item classes per the discriminator above; exclude the
  visual-definition pairs. Assign category `armor`.
- Add `Armor` to the category model: `ItemCategory` enum
  (`crates/gore-catalog/src/lib.rs`) + `item_category_from_id` /
  `category_for_id` mapping (armor classes → `Armor`).
- Regenerate `apps/save-editor/assets/item_catalog.json` (the Rust core embeds
  this via `include_str!`, so the core allow-list updates with it).
- Update existing catalog tests that assert armor → `Other`/`Unknown`; they
  now expect `Armor`.

### Layer 2 — Display scan (`crates/gore-save`)

- The display filter already passes armor (`looks_item_definition_path` accepts
  any `/Script/Angelscript.*`), so NO change is needed to make armor rows
  appear — verified: all three G1R-019 armors are returned today.
- `summarize_private_inventory_items` (`lib.rs:~3829`): surface each row's
  owning container type (`EInventoryTypes`) so the UI can mark the `ArmorSlot`
  item as equipped. The current FString-ref scan does not know the container;
  the equipped indicator requires deriving the container per row (either by
  extending the scan, or by a typed-tree pass that maps item path → container
  enum). This is the only core display change.

### Layer 3 — Add (existing path, now armor-eligible)

- Add validation (`is_item_definition_class` / `parse_private_inventory_add_item_edit`)
  is satisfied automatically once armor is in the embedded catalog. Adding
  armor targets `MainContainer` exactly like any other item — no new code, but
  add an armor case to the add tests.

### Layer 4 — Equip / unequip (new structural edit)

- New edit op that relocates an armor slot between the `MainContainer` and
  `ArmorSlot` containers' `m_Slots` arrays (the same typed-tree splice approach
  `addItem`/`removeItem` use on `MainContainer`).
- Invariants: `ArmorSlot` holds at most one armor. Equipping when one is
  already worn moves the previously-worn armor back to `MainContainer`
  (swap, no item loss). Unequipping moves the worn armor to `MainContainer`.
- Keep the slot's inline `m_InventoryType` field consistent with its new
  container key.

### Layer 5 — Upgrade editing (new payload edit)

- View: read the worn armor slot's `m_Payload.m_GenericData` map
  (`m_CurrentUpper/Mid/LowerBodyUpgrade`).
- Edit: set each of the three values to a chosen upgrade string (or empty =
  none). Validate values against the armor's available upgrade set where it can
  be derived; otherwise allow the known `m_<Part>_<Tier>_ArmorUpgrade` form.

### UI (`apps/save-editor`)

- New `Armor` category in the inventory grouping (`item_categories.dart`),
  rendered in the inventory panel.
- Armor rows show an equipped badge when in `ArmorSlot`.
- Equip / unequip control per armor row.
- Upgrade editor for the worn armor (three slots: upper / mid / lower).
- "Add item" dialog now lists armor (drawn from the regenerated catalog).

## Testing & validation

- **Catalog**: generated catalog includes the four known real-save armors and
  excludes every `*_VisualsDefinition`. Snapshot count sanity (~116 armor
  classes).
- **Display**: parsing G1R-019 yields three armor rows — one flagged equipped
  (`Ore_Armor_H`), two carried. G1R-016/020 each yield one equipped
  `Org_Armor`.
- **Add**: adding an armor to a fixture's `MainContainer` round-trips
  byte-consistently and re-parses.
- **Equip/unequip**: moving `Crw_Armor_H` from bag to `ArmorSlot` in G1R-019
  bumps `Ore_Armor_H` back to `MainContainer`; re-parse confirms exactly one
  armor in `ArmorSlot`.
- **Upgrades**: setting the three upgrade values on G1R-016's worn `Org_Armor`
  reproduces the G1R-020 Heavy02 string-map; clearing them reproduces G1R-016.
- All edits verified against real decompressed payloads
  (`work/decompressed/*.host.bin`) with `parse_private_root` re-parse, per the
  existing byte-faithfulness discipline.

## Out of scope

- Other equip slots (weapons, rings, amulet, torch) — same `m_InventoryType`
  mechanism, but not requested here.
- Editing the `BoughtArmorUpgrades` available-upgrade *catalog* (which upgrade
  variants are unlocked) — only the applied selection is edited.

## Open implementation question

The equip/unequip splice (Layer 4) operates on the `ArmorSlot` container's
`m_Slots`, which the current code only ever reads, never mutates. The first
implementation task is to confirm — against the real typed tree — that the
`ArmorSlot` container slot can be spliced with the same clone/relocate approach
`addItem` uses for `MainContainer`, including keeping `m_Keys`/`m_Values`
parallel and the inline `m_InventoryType` consistent.
