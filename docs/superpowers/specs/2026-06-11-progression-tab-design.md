# Progression Tab v2 — Design

Date: 2026-06-11
Status: implemented (branch progression-tab-v2)

## Problem

The Progression tab shows a heuristic string dump: the core greps the
decompressed private payload for FStrings containing "quest", "dialog",
"knowledge", etc. (`summarize_private_progression_payload`,
`crates/goresave_core/src/lib.rs:2439`) and the UI renders flat
candidate/tag lists plus counts (`_PrivateProgressionSummaryCard`,
`apps/goresave/lib/features/editor/ui/editor_page.dart:1447`). Nothing is
editable, nothing is grouped by meaning, and the "sections" chips are
derived from hard-coded property names (`m_QuestLog`, `m_Knowledge`,
`m_ActiveQuests`, …) that do not exist in real saves — verified against
G1R-001: only `m_GeneratedEvents` occurs, and it belongs to trader
inventories, not progression. The tab makes no game-technical sense.

Goal: the Progression tab shows the real progression data (quests,
dialog knowledge, memory events) in a grouped, searchable UI following the
Player/Inventory tab pattern, and all three sections are editable.

## Findings (from a real save, G1R-001, 76.8 MB decoded payload)

- **Quests** live at
  `m_GenericData/{…}/SaveDataByStoryClass/{…}/QuestDataByClass` — a
  `MapProperty` with ObjectProperty keys (quest class paths such as
  `/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST`) and
  `SingleQuestSaveGameData` struct values:
  - `CurrentState`: `EnumProperty` `EQuestState` (ByteProperty form) —
    observed values `EQuestState::None|Available|Running|Succeeded|Failed`.
  - `StateReachedAtTime`: map `EQuestState → InGameTime` (`TotalSeconds`
    DoubleProperty).
  - `TimeLastViewedByPlayer`: map `Name → InGameTime`.
  - 707 distinct quest class names in the save; the class-name suffix
    encodes a hierarchy: `Quest_<Group>`, `Quest_<Group>_<QUEST>`,
    `Quest_<Group>_<QUEST>_<QUEST>_OBJ_<OBJECTIVE>`.
  - Unstarted quests are present as `Available`, so editing states never
    requires inserting new map entries.
- **Dialog knowledge** lives at
  `m_GenericData/{…}/CharacterKnowledgeByUniqueName` — a `MapProperty`
  keyed by NPC unique name (e.g. `OC_STT_Diego`) with `KnowledgeSet`
  struct values containing `Knowledge`: a `SetProperty` of `NameProperty`
  entries (voiceline ids like
  `Voiceline_info_diego_gamestart_11_00_AlkimiaLocalization`, dialog
  choice ids like `ChoiceDiegoGamestart`).
- **Memory events** live at
  `m_GenericData/{…}/AnyCharacterType/LongTermMemoryByGlobalId` — a
  `MapProperty` keyed by character global id with `MemorizedEvents`:
  an `ArrayProperty` of event structs:
  - `EventTags`: `StructProperty` `GameplayTagContainer` with tags from a
    32-tag taxonomy (`Memory.Quest.Started/Succeeded/Failed`,
    `Memory.Guild.Joined`, `Memory.Chapter.Completed`,
    `Memory.Character.Defeated`, `Memory.Item.Obtained`, …).
  - `Magnitude` (Float), `Payload` (`InstancedStruct`),
    `OptionalClass1/2` (SoftObjectProperty, e.g. quest/document classes),
    `position` (Vector), `Time`/`Duration` (`InGameTime`),
    `InstigatorGlobalId`/`AffectedCharacterGlobalId` (Name).
- The strict typed parser (`properties.rs`) already traverses all of this:
  maps by `{key}`, arrays/sets by `[index]`. `private.typed.setValue`
  already writes enum leaves with length change and auto-fixes the
  ancestor size chain — quest state editing needs no new write op.
- The decoded payload is cached after `inspect_save`
  (`decoded_private_payload_cached`), so per-section queries do not pay a
  second decode.

## Design

### 1. Core read side: `query_progression` command

New `execute_json` command, same shape as `search_typed_properties`
(requires codec backend, uses the decode cache, offset/limit paging):

- `section: "quests"` → walks `QuestDataByClass` and returns per entry:
  `classPath`, parsed `group`/`name`/`objective` display fields,
  `currentState`, `statePath` (the `private.typed.setValue`-addressable
  path to `CurrentState`), `writable`. Query filters on class path,
  case-insensitive. Sorted by class path for stable paging.
- `section: "knowledge"` → without `character` param: list of NPC unique
  names with entry counts. With `character`: that NPC's `Knowledge` set
  entries plus the set's typed path (for add/remove ops).
- `section: "events"` → without `character` param: list of character
  global ids with event counts. With `character`: paged `MemorizedEvents`
  entries: `index`, `tags`, `timeTotalSeconds`, `instigator`, `affected`,
  `optionalClass1/2`, plus the array's typed path and per-element scalar
  leaf paths (for duplicate-then-edit).
- `inspect_save` replaces `summarize_private_progression_payload` with a
  small structured overview computed from the typed tree: quest counts by
  state, NPC-with-knowledge count, total memorized event count. The
  heuristic string-grep summary and its UI fields are removed.

### 2. Core write side: generic container edit ops

Four new ops dispatched in `apply_private_edits`, all path-addressed with
the same path grammar as `private.typed.setValue`:

- `private.typed.setAdd` `{path, value}` — path resolves to a
  `SetProperty` with `NameProperty`/`StrProperty` inner type; encodes the
  FString, splices it at the end of the set payload, bumps the element
  count, fixes the property size and ancestor size chain (reuse the
  setValue length-change machinery). Rejects duplicates.
- `private.typed.setRemove` `{path, value}` — inverse; errors if the
  value is not present.
- `private.typed.arrayRemove` `{path, index}` — path resolves to an
  `ArrayProperty`; splices the element's byte range out, decrements the
  count, fixes sizes.
- `private.typed.arrayDuplicate` `{path, index}` — copies the element's
  byte range in place (insert after the source element), increments the
  count, fixes sizes.

Quest state changes use the existing `private.typed.setValue` with values
like `EQuestState::Succeeded` — no new op.

Out of scope for v1:

- Inserting new map entries (not needed: all quests already exist as
  `Available`).
- Composing a memory event from scratch. "Add event" is
  `arrayDuplicate` + editing the duplicate's scalar leaves (time,
  magnitude, instigator/affected names).
- Rewriting tags inside a duplicated event's `GameplayTagContainer`.
  Stretch goal (v1.1): if the typed parser can address the tag FStrings
  inside the container, the same splice machinery applies; otherwise the
  duplicate keeps the source tags.

### 3. App side (Flutter, Inventory-card pattern)

- Models in `editor_models.dart` (or a new `progression_models.dart`):
  `ProgressionOverview`, `ProgressionQuest`, `KnowledgeCharacter`,
  `KnowledgeEntry`, `MemoryCharacter`, `MemoryEvent`, plus edit-intent
  types with `toEditJson()` mapping to the new op paths (mirroring
  `InventoryItemCountChange`).
- Repository/notifier: `queryProgression(section, {query, character,
  offset, limit})`; pending edits flow through the existing edit-JSON
  apply/save pipeline; after a successful write the affected section
  reloads.
- `_ProgressionPanel` rewrite, gated like today on decoded private
  payload plus writable flags from the core:
  - **Overview card**: quest counts by state, knowledge/NPC count, event
    count (from the new inspect summary).
  - **Quests card**: search field, list grouped by parsed quest group,
    state dropdown (`None/Available/Running/Succeeded/Failed`) per entry,
    "modified" chip on pending changes, paging ("load more").
  - **Knowledge card**: NPC search/list with counts, expansion loads
    entries lazily, per-entry delete (pending `setRemove`), add field
    with non-empty/duplicate validation (pending `setAdd`).
  - **Events card**: character selector with counts, paged event list
    showing tags and in-game time, per-event delete (`arrayRemove`) and a
    duplicate dialog (`arrayDuplicate` + scalar edits on the copy).
- Read-only fallback: when the typed parse failed or the codec backend
  cannot compress, cards render without edit affordances (same gating
  pattern as the Inventory tab's `canCompress`).

### 4. Error handling

- All ops validate path resolution, container type, and inner element
  type before touching bytes; failures return `UNSUPPORTED_EDIT` or
  `PARSE_ERROR` without modifying the payload.
- Multi-edit batches apply sequentially to the decompressed payload;
  index-addressed edits (`arrayRemove`/`arrayDuplicate`) shift later
  indices, so the app submits at most one structural array edit per
  apply round (UI enforces this; core documents it).
- The UI surfaces op errors per pending edit, as the Inventory card does.

### 5. Testing

- Rust unit tests per op on synthetic GSAV containers (existing test
  style): byte-exact assertions on count fields, size chains, and
  payload length for setAdd/setRemove/arrayRemove/arrayDuplicate,
  including rejection cases (duplicate add, missing value, bad index,
  wrong container type).
- `query_progression` tests on synthetic payloads with quest map,
  knowledge set, and event array fixtures.
- Local (not committed) roundtrip verification against the real saves in
  `work/roundtrip_gsav`: apply each op, re-parse strictly, confirm the
  typed parse stays `ok` and unrelated bytes are untouched.
- Dart tests for model JSON parsing and `toEditJson()` shapes.
