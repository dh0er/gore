# Design: Add Items to Inventory (Searchable Catalog Picker)

Date: 2026-06-12
Status: Approved (design), implementation pending

## Problem

The Inventory tab can only edit the count of items that already exist in the
save (`private.inventory.setItemCount`). There is no way to add an item that
is not present. Users want to add arbitrary items, picked from a searchable
list of all items in the game.

## Background (current code)

- Inventory items live in the private GSAV payload as **plain struct array
  elements** (verified 2026-06-12 against three real decompressed payloads
  in `work/decompressed/`; an earlier draft of this spec wrongly assumed
  ObjectInstances). Addressable path:
  `m_GenericData{PlayersSavedData}.m_SavedPlayers[0].m_Inventory` →
  StructProperty `ReplicatedInventoryMap` with parallel arrays `m_Keys`
  (Enum `EInventoryTypes`, 14 entries) and `m_Values.Items`
  (Array<Struct `ContainerVirtualData`>). Player items are in the container
  whose key is `EInventoryTypes::MainContainer` (index 6 in observed saves;
  match by enum value, not index). Its `m_Slots` is a plain
  `ArrayProperty<StructProperty ItemVirtualData>`; each slot has `m_Id`
  (IntProperty, sequential per container — the uniqueness field),
  `m_InventoryType` (Enum), `m_SlotData` (Struct ItemSlot with
  `m_ItemDefinition` ObjectProperty asset path + `m_ItemCount` Int), and
  `m_Payload` (Struct ItemPayload, empty for ordinary stacks). FastArray
  replication ints are all -1 in saves.
- The typed parser (`properties::parse_private_root`) parses this fully;
  `resolve_chain` + `container_layout` already work on `m_Slots` (plain
  array — the ObjectInstances rejection at `crates/goresave_core/src/lib.rs:922`
  is never hit for inventory).
- The inventory summary shown in the UI is produced by an FString scan
  (`summarize_private_inventory_items`, `crates/goresave_core/src/lib.rs:3172`)
  bounded by `inventory_item_region` (`lib.rs:3221`).
- Count edits patch a fixed-size i32 in place; no size change involved.
- `ContainerEdit::ArrayDuplicate` (`crates/goresave_core/src/properties.rs:1076`)
  demonstrates the full size-fixup mechanics for inserting bytes into the
  payload: splice element bytes, bump the array element count, patch the
  ArrayProperty size field, and add the delta to every enclosing size field
  (`enclosing_size_fields` resolve chain), then re-validate by re-parsing.
- `patch_string` (`properties.rs:862`) demonstrates length-changing string
  replacement with parent size fixups.
- No item catalog exists in the repo; the app only knows items already in the
  save. No reusable searchable picker widget exists.

## Goals

- Add a new item (asset path + count) to the player inventory region of a
  GSAV save, backup-first, with full payload re-validation.
- Ship a searchable, categorized item catalog in the app so users can pick
  items they do not yet own.
- Categorize the **entire inventory view**: the existing item list in the
  Inventory tab is grouped by the same categories (derived from the item id
  prefix), not just the add-item picker.

## Non-goals

- Editing inventories other than the player inventory region.
- Shipping localized display names, stats, icons, or any asset content
  extracted from game files (legal posture: identifiers only).
- Removing items (existing count editor sets counts; removal semantics are a
  separate feature).

## Components

### 1. Catalog pipeline (`tools/build_item_catalog.py`)

- Input: UE4SS object dump from Gothic 1 Remake (one-time, maintainer-run).
  Dump created 2026-06-12 (with a loaded save):
  `D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt`,
  lines match `ASClass /Script/Angelscript.It...`. 896 unique `It*` classes.
- Filters UClasses in the `/Script/Angelscript` package by the empirical
  prefix set (verified against the dump):

  | Prefix | Count | Category |
  |---|---|---|
  | `ItMw_` | 148 | melee weapon |
  | `ItAr_Rune_` | — | rune |
  | `ItAr_Scroll_` | — | spell scroll |
  | `ItWr_` | 138 | writing (books/letters) |
  | `ItMi_` | 133 | misc |
  | `ItAt_` | 105 | animal trophy |
  | `ItFo_` | 87 | food & potions |
  | `ItMs_` | 45 | mission/special |
  | `ItKe_` | 34 | key |
  | `ItRw_` | 30 | ranged weapon |
  | `ItAm_` | 2 | amulet |

- Excluded: `ItemAnimConfig_*`, `ItemSpawnManagerConfig_*`,
  `ItemCollisionFX`, `ItemVisualWorldTargetConfig` (config classes),
  `ItAI_*` (AI props such as planks/boxes), classes ending in `_Base`
  (abstract rune/scroll bases). Unmatched `It*`-like classes are logged by
  the script, never silently dropped (singletons such as `ItKeyDefault`,
  `ItChestKey01`, `ItDoorKey01`, `ItIg_*`, `ItFocusStoneBridgeItem` get an
  explicit decision in the script: keys → `key`, rest → `special`).
- Output: `apps/goresave/assets/item_catalog.json`, committed to the repo:

  ```json
  [{ "id": "ItMi_Orenugget", "path": "/Script/Angelscript.ItMi_Orenugget", "category": "misc" }]
  ```

- `category` is derived from the prefix. Display names are derived from `id`
  in the app (e.g. `ItMi_Orenugget` → `Orenugget`); no localization files are
  read or shipped.

### 2. Rust: new op `private.inventory.addItem`

Edit JSON: `{"path": "private.inventory.addItem", "value": {"path": "/Script/Angelscript.ItMi_X", "count": n}}`

Algorithm (uses existing typed machinery only — no new properties.rs API):

1. Require typed parse status `ok`; otherwise the op is not advertised in the
   `writable` list and the edit is rejected.
2. Locate the MainContainer: the `m_Inventory` entry whose `m_Keys` value is
   `EInventoryTypes::MainContainer` (match by enum value, not array index).
3. Reject if any slot's `m_ItemDefinition` already equals the target path
   (UI prevents this too).
4. Template: last `m_Slots` element of the MainContainer; empty container →
   error ("no template slot").
5. Duplicate the template slot via the existing `ArrayDuplicate` mechanics
   (splice, element count, size field, enclosing size-field chain), re-parse.
6. In the duplicate: `patch_string` its `m_ItemDefinition` to the target
   path (length-changing, fixes enclosing sizes), `patch_scalar` its
   `m_ItemCount` to the requested count and its `m_Id` to max existing id
   + 1 in that container. Verify the duplicated `m_Payload` is empty (it is
   for ordinary stacks; non-empty template payload → error rather than
   cloning item-specific state).
7. Re-parse the payload after the edit; on any failure the write is aborted
   and nothing is written. Backup-first like all writes.
8. Counts as a structural edit: at most one `addItem` per write batch, and
   not combinable with `arrayRemove`/`arrayDuplicate` in the same batch
   (indices/offsets shift).

Open question for the in-game verification (Phase 1): does the game accept
appended slots with FastArray ReplicationID -1? (All saved slots carry -1,
which suggests yes.)

Error cases: item already present; empty inventory (no template instance);
typed parse not ok; unknown/invalid path format; count < 1.

### 3. Flutter UI

- **Categorized inventory list:** the existing item list in
  `_PrivateInventorySummaryCard` is grouped by category. Category is derived
  from the item id prefix (same mapping as the catalog; shared Dart helper
  `itemCategoryFromId`). Items whose prefix is unknown go into an "other"
  group. Groups are collapsible section headers with item counts; the
  existing search field filters across all groups.
- `_InventoryPanel` gains an "Add item" button, enabled only when
  `scope == "player_inventory_region"` and the private payload is writable.
- Dialog: search field (substring match on id/path, same semantics as the
  existing filter), category-grouped list from the bundled catalog, items
  already present in the save are hidden (their counts are edited in the
  existing editor). Count field (default 1, integer ≥ 1), Add button.
- New model `InventoryItemAdd` with `toEditJson()` analogous to
  `InventoryItemCountChange` (`editor_models.dart:608`).
- After a successful write the inspection is re-run so the new item appears
  in the list.

## Phasing

- **Phase 1 — write-path verification (before any UI work):** implement the
  Rust op, exercise it against a real save (CLI/test harness), load the
  modified save in the game, confirm the item exists and the save is stable.
  Resolves V1 (template homogeneity), V2 (instance naming), and R3 below.
- **Phase 2 — catalog + UI:** dump-derived catalog JSON, picker dialog,
  wiring.

## Risks

- **R1:** a template slot may carry item-specific state in `m_Payload` →
  the op errors when the template payload is non-empty instead of cloning
  state; ordinary stacks have empty payloads (verified in 3 real saves).
- **R2:** slot `m_Id` must be unique per container → the duplicate gets
  max existing id + 1.
- **R3:** the game references stacks externally elsewhere in the save →
  in-game load test required before shipping.
- **R4:** prefix list incomplete → verify against the dump; the catalog
  script logs unmatched `It*`-like classes instead of silently dropping them.

## Testing

- Rust unit tests on fixture payloads: duplicate + size fixups, re-parse
  validation, error cases (item exists, empty inventory, parse not ok,
  invalid count).
- Catalog script test against a small dump fixture.
- Flutter widget test for the picker dialog (search filtering, hiding of
  already-present items).
- Manual integration note in `integration_test/`: game loads the modified
  save, added item present with the chosen count.
