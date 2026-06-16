# Knowledge tab: full NPC + entry catalogs

Date: 2026-06-15
Status: Approved (design)

## Problem

The editor's Knowledge tab only shows NPCs that already have a
`CharacterKnowledgeByUniqueName` entry in the loaded savegame, and the
entry-add field is free-text only. Two gaps:

1. No way to add knowledge for an NPC that is **not yet** in the save.
2. No discoverable list of **possible** knowledge entries to add.

The Inventory tab already solves the analogous problem with a bundled item
catalog (`assets/item_catalog.json`, built from a UE4SS object dump by
`tools/build_item_catalog.py`). This feature brings the same pattern to
knowledge.

## Data feasibility (established)

From the UE4SS object dump
(`G1R/Binaries/Win64/ue4ss/UE4SS_ObjectDump.txt`):

| Data | Source in dump | Count | Note |
|------|----------------|-------|------|
| NPC unique names | `ASClass …CharacterDefinition_Human_<key>` → `<key>` | 648 humans (1095 all CharacterDefinition_) | `<key>` is the exact `CharacterKnowledgeByUniqueName` map key, e.g. `OC_STT_Diego` |
| Topic tokens | `ASClass …Topic_*` | 1291 | dialog/quest knowledge |
| Choice tokens | `ASClass …Choice*` | 2479 | dialog choices |
| Info tokens | `ASClass …Info_*` | 143 | dialog infos |

**Voicelines are out of scope.** A real save's Knowledge set is ~80%
`Voiceline_*` tokens, but those are localization keys (suffix
`_AlkimiaLocalization`), not UObject classes — absent from the object dump and
only present in raw `.locres` chunks inside the 27 GB IoStore container with no
directory index. They are replay-tracking markers (suppress line repeats), not
dialog-unlock knowledge, so low edit value. Confirmed via `retoc` extraction
that no clean package-level source exists. The free-text entry field remains as
a manual fallback for anyone who needs a specific voiceline.

A real save's `KnowledgeSet` struct has exactly one field, `Knowledge`
(a `SetProperty<NameProperty>`); there is no sibling field. Verified against
`work/decompressed/G1R-001.host.bin` (47 characters, e.g. `OC_STT_Diego` =
105 entries: 84 Voiceline / 17 Choice / 4 Topic).

## Components

### 1. Catalog build scripts → Flutter assets

Two Python scripts modelled on `tools/build_item_catalog.py`:

- `tools/build_npc_catalog.py` → `apps/goresave/assets/npc_catalog.json`
  - Regex `ASClass /Script/Angelscript\.CharacterDefinition_(\S+)` → unique name.
  - Humans: the `<key>` after `CharacterDefinition_Human_` is the map-key form
    (`OC_STT_Diego`). Categorize by sub-prefix into `human` vs `creature`/other
    so the picker can group/filter. Non-human keys are kept for visibility but
    flagged (they are not known to be valid map keys; cosmetic).
  - Entry shape: `{ "id": "OC_STT_Diego", "class": "CharacterDefinition_Human_OC_STT_Diego", "category": "human" }`.
- `tools/build_knowledge_catalog.py` → `apps/goresave/assets/knowledge_catalog.json`
  - Regex over `ASClass /Script/Angelscript\.(Topic_\S+|Info_\S+|Choice\S+)`.
  - Entry shape: `{ "id": "Topic_Diego_209799", "category": "topic" }`
    (`topic` | `choice` | `info`).

Both are **frontend-only assets**. Unlike `item_catalog.json` they are **not**
embedded in the Rust core and impose **no allow-list** — they exist purely for
discovery/autocomplete. This keeps the core decoupled and preserves free-text.

Regenerate command (matches the item-catalog convention, see
`gothic-remake-ue4ss-dump` memory):

```
python tools/build_npc_catalog.py "<dump path>"
python tools/build_knowledge_catalog.py "<dump path>"
```

### 2. Frontend (Flutter)

In `apps/goresave/lib/features/editor/`:

- New catalog models + loaders mirroring `domain/item_catalog.dart`:
  `domain/npc_catalog.dart`, `domain/knowledge_catalog.dart`
  (each: entry class, `loadBundled()` via `rootBundle`).
- New picker dialogs mirroring `ui/add_inventory_item_dialog.dart`:
  - **Add-NPC dialog**: full `npc_catalog`, category sidebar + search.
    Excludes NPCs already present in the save. On pick → call the new core
    add-character op, then refresh the character list and select the new NPC.
  - **Add-entry dialog**: full `knowledge_catalog`, category (topic/choice/info)
    + search, excludes entries already on the selected character. On pick →
    feed into the existing `_addEntry` path. Keep the existing free-text field
    as a fallback for non-catalog tokens (voicelines).
- Wire both into `ui/progression_panel.dart` `_KnowledgeDetail` (existing
  add/remove/dup-check logic at lines ~760–864 is reused unchanged for entries).

### 3. Backend (Rust core)

The current edit layer (`ContainerEdit` in `properties.rs:973`) only mutates
existing Sets/Arrays. Adding a brand-new NPC requires inserting a new entry into
the `CharacterKnowledgeByUniqueName` map, which has no precedent. New work:

- New `ContainerEdit::MapInsert { key_bytes, value_bytes }` variant
  (analogue of the existing `ArrayInsertBytes`).
- New `map_layout()` helper that walks inline map entries to find the byte
  boundary after the last entry and the count-field offset (maps lack the simple
  count+elements layout of arrays/sets — keys/values are inline-encoded).
- Extend `patch_container()` to handle `MapInsert`: splice `key_bytes ++
  value_bytes` at the end of the map body, increment the entry count, and fix
  the tag size + all enclosing size-chain fields (reuse the existing
  `ArrayInsertBytes` size-fixup logic at `properties.rs:1103-1149`).
- Build `value_bytes` = a minimal `KnowledgeSet { Knowledge: Set<Name>() }`
  (empty set), synthesized via the existing test helper
  `private_name_set_property` (`lib.rs:11143`) promoted to non-test code, or by
  cloning a donor entry's struct bytes and clearing its set. `key_bytes` = the
  NPC unique name encoded as an inline Name value.
- New IPC op `private.knowledge.addCharacter` (payload: NPC unique name) that
  resolves the map, rejects duplicates, applies the `MapInsert`, and
  strict-re-parses to validate (existing validation pattern).
- After insertion the new NPC has an empty `Knowledge` set; the user adds
  entries through the unchanged `private.typed.setAdd` path.

### 4. Policy

- **No allow-list** on entries or on NPC keys: catalogs are discovery only;
  free-text is always accepted. Voicelines stay reachable manually.
- **Dedup**: the NPC picker hides NPCs already in the save; the entry picker
  hides entries already on the selected character (server dup-check at
  `progression_panel.dart:798` still guards races).

## Testing

- **Core**: `MapInsert` unit tests in `properties.rs` (empty map → 1 entry;
  size-chain correctness; byte-exact re-parse round-trip). Integration test in
  `lib.rs` against a real `work/decompressed/*.host.bin`: add a new character,
  then `setAdd` an entry, verify both via `query_progression`.
- **Catalog scripts**: smoke test that each script parses the dump and emits a
  non-empty, schema-valid JSON with expected known IDs (e.g. `OC_STT_Diego`,
  `Topic_Diego_209799`).
- **Frontend**: widget tests for the two dialogs (loads catalog, filters by
  category/search, excludes existing, returns selection).

## Out of scope

- Voiceline-token catalog (localization extraction).
- Human-readable display names (would need DataTable/`.locres` parsing with the
  USMAP; the picker shows raw IDs, consistent with the existing inventory and
  knowledge UI).
- Removing an entire NPC from the map (only add is requested).
