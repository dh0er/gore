# Characters & World Tab Restructure — Design

**Date:** 2026-07-01
**Status:** Approved (design), pending implementation plan
**Branch base:** `main`

## Goal

Restructure the save editor's navigation from **aspect-first** (pick a data kind, then whose) to **entity-first** (pick a character, then the data kind) for all per-character data, and give world-global data its own home.

The save is *natively* entity-first: attributes, inventory, memory events and death/GAS tags are all stored in parallel maps keyed by **GlobalId**; dialog knowledge is keyed by **UniqueName**. The current UI fights this by rebuilding a character list four separate times (Attribute, Inventory, Knowledge, Events). This restructure aligns the UI with the data model.

**New top-level tabs:** `Übersicht · Charaktere · Welt · Alle Daten · Backups · Einstellungen` (was 7, now 6).

- **Charaktere** (new): one shared master list of all characters (Player pinned on top, then all NPCs). Selecting a character shows a detail area with four sub-tabs — **Attribute · Inventar · Wissen · Ereignisse** — all scoped to that character.
- **Welt** (new): the two world-global sections that have no character list — **Quests** and **Fraktionen** — moved out of the old "Fortschritt" tab.
- The old **Fortschritt** (Progression) tab is dissolved: Knowledge/Events become Charaktere sub-tabs; Quests/Factions become the Welt tab.

## Background (verified facts)

Measured against two real saves via the public IPC entry (`gore_save::execute_json`, same path the app uses) — a throwaway probe paging `private.npc.list`, `query_progression{knowledge}` and `query_progression{events}` and diffing the identity sets:

| Save | Actors (GlobalId) | unique charKeys | Knowledge (UniqueName) | Knowledge orphans | Events (GlobalId) | Event orphans |
|---|--:|--:|--:|--:|--:|--:|
| G1R-021 (late) | 1638 | 753 | 143 | **1** | 1609 | **0** |
| G1R-011 (mid) | 1496 | 733 | 106 | **0** | 1467 | **0** |

- **All NPCs are pre-spawned.** Every NPC has a GlobalId and appears in `private.npc.list` from the start, whether or not the player has met them. The master list is therefore the **complete** character set — nothing is ever "added" to it. (Matches the observed behaviour: an unmet NPC like Xardas already shows under Attribute/Inventory/Events.)
- **Knowledge is an acquired subset.** Only 106–143 of ~1500 NPCs have a `CharacterKnowledgeByUniqueName` entry. An NPC with no entry simply has empty knowledge until the first entry is added.
- **Join rule (proven): a knowledge `UniqueName` equals an actor's GlobalId prefix (`charKey` = substring before the first `-`), case-insensitive.** Example: knowledge `NC_ORG_Lares_801` ↔ actor `NC_ORG_Lares_801-WP_…`. 142/143 and 106/106 matched. No heuristic/fuzzy matching needed.
- **Events join by exact GlobalId, always clean** — 0 orphans in both saves. `LongTermMemoryByGlobalId` owners are all actors.
- **Orphans exist only for knowledge and are rare (0–1).** The single observed orphan, `ST_VLK_Mud_Sleeper`, is a creature template with no persistent actor row.
- **The player's knowledge is keyed `Hero`,** and a `Hero` actor also exists (the `Hero` knowledge matched an actor, so it was not an orphan). Player vs Hero-actor must be de-duplicated (see §5).

## Approach

**Core-computed unified character index (chosen).** A single new read command, `private.characters.list`, does the GlobalId↔UniqueName join in Rust — where all maps are already decoded — and emits one authoritative list with per-aspect availability flags. Rejected: composing the list client-side from three separate calls (`npc.list` + knowledge chars + event chars), which re-derives the proven join in Dart, triples the load path, and spreads identity logic across the frontend.

The four detail panels are **reused unchanged**; the restructure is mostly relocation plus the removal of three now-redundant character lists.

## Components

### 1. Core: `private.characters.list` (`crates/gore-save`)

One decode of the private root, then:

- Build three lookup sets from the decoded maps: inventory GlobalIds (`CharacterStateSaveGameData_Inventory`), knowledge UniqueNames (`CharacterKnowledgeByUniqueName`), event GlobalIds (`LongTermMemoryByGlobalId`).
- For each actor from the existing NPC enumeration, emit:
  ```
  { globalId, uniqueName, displayName, isDead,
    hasInventory, hasKnowledge, hasEvents }
  ```
  where `uniqueName = charKey(globalId)`, `hasKnowledge = knowledgeSet.contains(charKey)`, `hasEvents = eventSet.contains(globalId)`, `hasInventory = invSet.contains(globalId)`.
- **Append orphan knowledge rows**: any knowledge UniqueName with no matching actor becomes `{ globalId: null, uniqueName, displayName, isDead: false, hasInventory: false, hasKnowledge: true, hasEvents: false }`. (Event orphans never occur, but the same append is harmless if one ever does.)
- Paginated + id/name filterable, mirroring `private.npc.list` (`{ path, query?, offset?, limit? }` → `{ characters, total, offset, limit }`). Reuse the existing decode-cache prelude (`decode_private_root`).

`isDead` reuses the HP-0 rule already in `private.npc.list`. No new write ops — all edits still flow through the existing per-aspect commands.

### 2. Frontend: top-level tab restructure (`editor_page.dart`)

- `DefaultTabController` length 7 → 6. Replace the `Attribute / Inventar / Fortschritt` tabs with `Charaktere / Welt`. Keep `Übersicht / Alle Daten / Backups / Einstellungen`.
- The shared Save/Reset action bar and pending-edit machinery are unchanged (all sub-tabs continue to register pending edits through the same registry).

### 3. Charaktere tab

- **Left: master list** — the existing `ActorSelector`, re-backed by `private.characters.list` instead of `loadAllNpcActors`, so rows carry availability flags. Player pinned on top (unchanged), searchable + paginated (unchanged). Rows show small badges for the meaningful aspects (📖 Wissen, ⚡ Ereignisse); inventory is near-universal, so no inventory badge.
- **Right: detail** — a four-way sub-tab bar (`Attribute · Inventar · Wissen · Ereignisse`) over the character selected in the master list. Selection is the shared `state.selectedActor` (extended to also carry `uniqueName` for the knowledge key).
- The four sub-tab bodies are the **existing detail widgets, with their own character lists removed**:
  - **Attribute** = today's `_AttributePanel` detail (player `HeroStatsCard` / NPC `NpcAttributesPanel`), minus its embedded `ActorSelector`.
  - **Inventar** = today's `_InventoryPanel` detail, minus its embedded `ActorSelector`.
  - **Wissen** = `_KnowledgeDetail`, **left character pane deleted**; keyed by `selectedActor.uniqueName`. Empty state + "Wissen hinzufügen" when the character has no entry (see §6).
  - **Ereignisse** = `_EventsDetail`, **left character pane deleted**; keyed by `selectedActor.id` (GlobalId).
- A sub-tab with no data renders a clean empty state, not an error (e.g. Xardas → Attribute/Inventar/Ereignisse populated, Wissen empty + add button).

### 4. Welt tab

- Reuse the old Fortschritt sidebar shell (`Quests / Fraktionen` only) with the existing `_QuestsDetail` (group picker, no character list) and `_FactionsDetail` (global guild matrix). These move **unchanged** — they never had a character list, which is exactly why they belong here rather than under Charaktere.
- Game time stays in Übersicht (unchanged).

### 5. Player / Hero de-duplication

- The master list pins **Player** (the `Actor.player()`, backed by the `m_SavedPlayers` player data source). The `Hero` actor row from the character index must be **suppressed** from the NPC portion so the player is not listed twice.
- **Verify in the plan:** whether `private.npc.list` currently returns the `Hero` actor at all (the probe's `Hero` knowledge matched *some* actor charKey). If it does, exclude it by charKey in `private.characters.list`; if it does not, no action needed. Player knowledge (`Hero`) routes to the pinned Player's Wissen sub-tab via `uniqueName = "Hero"`.

### 6. Knowledge add-flow simplification

- The old **"NPC hinzufügen"** button + bundled `NpcCatalog` picker in the knowledge tab is **removed**: every NPC is already in the master list, so there is nothing to add to a list.
- Adding the first knowledge entry to a character who has none is handled in-place: the Wissen sub-tab's "Wissen hinzufügen" action calls the existing `private.knowledge.addCharacter` (using the character's `uniqueName`) to create the map entry if absent, then the existing entry-add path. This preserves the current core write ops; only the UI entry point changes.
- **Non-actor knowledge targets** (creature templates with no actor row) are out of scope for *new* knowledge: they can't be selected from an all-actors list. Existing orphan knowledge stays editable via the appended orphan rows (§7).

### 7. Orphans ("Weitere" group)

- Orphan rows from `private.characters.list` (knowledge UniqueName with no actor — typically 0–1) render in a **"Weitere" group at the bottom of the master list that only appears when non-empty.** Selecting one enables the Wissen sub-tab only; the other three show a "kein Actor" empty state.
- This guarantees no existing knowledge becomes unreachable while adding ~zero visual weight in the common case.

## Reuse vs new

- **Reused unchanged:** `HeroStatsCard`, `NpcAttributesPanel`, inventory detail, `_QuestsDetail`, `_FactionsDetail`, the pending-edit registry, all core write ops, `decode_private_root` prelude.
- **Reused, trimmed:** `ActorSelector` (new data source + badges), `_KnowledgeDetail` / `_EventsDetail` (character list panes removed, keyed by shared selection).
- **New:** `private.characters.list` core command; the Charaktere master+sub-tab shell; the Welt tab shell; `uniqueName` on the shared `Actor`.
- **Removed:** three redundant character lists (knowledge, events, and the standalone progression tab shell); the knowledge "Add NPC" dialog + bundled `NpcCatalog` use.

## Edge cases

- **Ambiguous charKey.** 1638 actors share only 753 charKeys, so a charKey can map to multiple actor instances; but knowledge only ever targets uniquely-numbered named NPCs (`NC_*_<n>`), so in practice each knowledge row maps to exactly one actor. If a knowledge charKey ever matches several actors, the knowledge shows on each — acceptable, since it *is* that character's knowledge.
- **Selection identity.** `Actor` equality is by `(kind, id)`; adding `uniqueName` must not change equality (it's a label/key carrier, like `name`/`isDead` already are).
- **Save switch / refresh.** The master list resets and re-fetches on `reloadKey` change (existing `ActorSelector` behaviour); sub-tabs re-key off the preserved/reset selection exactly as the current shared-selection tabs do.

## Out of scope

- Any change to *what* attributes/inventory/knowledge/events edits do (write ops unchanged).
- Bulk "same aspect across many NPCs" workflows — the user confirmed single-character run-through is the dominant use; aspect-first bulk editing is not a goal.
- Seeding knowledge onto non-actor creature templates (rare power-user case).

## Testing

- `private.characters.list` on real `G1R-021` / `G1R-011`: character count == actor count (+ orphan tail); `hasKnowledge`/`hasEvents`/`hasInventory` flags match the standalone `knowledge`/`events`/inventory sets; the `ST_VLK_Mud_Sleeper` orphan appears as a `globalId: null` row on 021 and is absent on 011.
- Join correctness: every `hasKnowledge` row's `uniqueName` resolves to a knowledge entry; every non-orphan `uniqueName` equals its `charKey`.
- Frontend: selecting a character shows the same data across all four sub-tabs; Xardas (no knowledge) shows empty Wissen + add button; adding a first entry roundtrips (creates the map entry then the entry).
- Player pinned once (no duplicate `Hero` row); player attribute/inventory/knowledge unchanged (regression).
- Welt: quests + factions behave exactly as under the old Fortschritt tab (regression).

## Implementation phasing

One spec, three phases:

1. **Core** — `private.characters.list` (join + availability flags + orphan append) with tests. No UI change yet.
2. **Charaktere tab** — master list re-backed by the new command; four detail sub-tabs wired to shared selection; `_KnowledgeDetail`/`_EventsDetail` character panes removed; knowledge add-flow simplified; Player/Hero dedup.
3. **Welt tab** — relocate Quests + Factions; delete the dissolved Fortschritt shell; final tab-bar cleanup.
