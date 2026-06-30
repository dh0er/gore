# NPC Attributes & Inventory Editing — Design

**Date:** 2026-06-25
**Status:** Approved (design), pending implementation plan
**Branch base:** `chore/monorepo-restructure`

## Goal

Let the user edit **any character** in a Gothic 1 Remake save — the player *and* the ~1484 NPCs — through the existing editor UI, plus revive/respawn dead NPCs.

Two existing tabs gain a shared **actor selector** sidebar (player on top, NPCs below):

- **"Attribute"** tab (renamed from "Player"): edit the selected actor's attributes (health, mana, combat, resistances, …). For NPCs, also expose a **Dead** switch and a **KillBounty (killable-for-XP)** switch.
- **"Inventory"** tab: edit the selected actor's inventory (add / remove / count), now including NPC inventories.

Actor selection is **shared** across both tabs: picking an NPC in Attribute shows the same NPC in Inventory.

## Background (verified facts)

From a full typed parse of a real save (`work/decompressed/G1R-001.decompressed.bin`, 1484 NPCs, 47 dead). See memory `goresave-respawn-mechanism`.

- **Death is HP, not a flag.** A dead NPC has `Health` `BaseValue` + `CurrentValue` = `0`; `MaxHealth` keeps its real value. No `bIsDead` bool, no `GE_Death` effect exists (104 GE classes, none death/kill/bounty).
- **`State.Dead` and `State.KillBountyGranted` persist only as captured-tag residue** inside `CapturedActorTags` / `CapturedTargetTags` of `GameplayEffectSpec`s in each character's `CharacterStateSaveGameData_ActiveEffects`. No owned/granted/loose tag store holds them anywhere in the save.
- **`State.KillBountyGranted` is authoritative and independent of death.** Cross-tab found exactly one *alive* NPC (`OC_VLK_Herek_511`, full HP) carrying `State.KillBountyGranted` without `State.Dead` — a non-lethally defeated NPC whose XP bounty was already paid. So clearing the bounty (to make an NPC give XP again) requires removing that tag; resetting HP alone does **not**.
- **Player vs NPC live in different containers.** Player inventory is `m_SavedPlayers[i].m_Inventory` where `m_PlayerID == "Party ID 0"`. NPC inventories are entries of a `CharacterStateSaveGameData_Inventory` map keyed by GlobalId (e.g. `Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1`). NPC attributes are likewise in `CharacterStateSaveGameData_Attributes` map entries.

## Core feasibility (verified)

- **Typed edit engine is generic.** `private.typed.setValue` resolves a full property path from the private root via `parse_path` + `resolve_chain`, including `{key}` map-key segments. NPC attributes are therefore addressable with no new path-root machinery — only a way to *discover* the per-NPC paths is needed.
- **Tag containers are parsed but not editable.** The whole chain `ActiveEffects[] → ActiveGameplayEffect → GameplayEffectSpec → TagContainerAggregator → CapturedActorTags` parses to typed values (non-native structs → `StructValue::Properties`; the leaf `GameplayTagContainer` → editable `Vec<String>`). But `container_layout` / `patch_container` only accept `ArrayProperty` / `SetProperty`; a `GameplayTagContainer` is a native struct (`u32 count` + `count × fstring`). **There is no existing edit op to add/remove a tag from a `GameplayTagContainer`.** This is the one genuinely new core primitive.
- **HP editing already works.** `Health.BaseValue` / `CurrentValue` are `FloatProperty` scalars → `patch_scalar` handles them today.

## Approach

**Hybrid (chosen).** Attributes reuse the proven generic `private.typed.setValue`. Only the parts that are genuinely complex (per-NPC tag walking + HP bundle + inventory rooting) get dedicated, encapsulated commands. Rejected: a fully generic approach (frontend would have to know every captured-tag container path per NPC — brittle) and a fully dedicated approach (needlessly duplicates the player attribute/inventory machinery).

## Components

### 1. Shared actor model + selector (frontend)

- New `ActorSelector` sidebar widget: a pinned **Player** entry on top, then a **searchable, paginated** NPC list. With ~1484 NPCs this must reuse the existing memory-character pagination/search pattern (`loadMemoryCharacters`) and localized names (`localizedProgressionName`).
- `Actor` value: `{ kind: player | npc, id: String?, name: String }` (`id` is the GlobalId for NPCs, null for player).
- **Shared selection state** lifted into the editor notifier / editor state so both the Attribute and Inventory tabs read the same active actor. Selecting in one tab updates the other.

### 2. "Attribute" tab (renamed from "Player")

- Left: `ActorSelector`. Right: detail panel = the existing `HeroStatsCard` (keeps its inner attribute-group sidebar: Core / Combat / Resistances / Thieving / Transform / Advanced) parameterized by the selected actor.
- **Player selected** → existing player data source and edit path (unchanged behavior).
- **NPC selected** → new core read `private.npc.attributes(id)` returns attribute rows each with a ready-to-use `private.typed.setValue` path; edits flow through the existing `setValue` pending-edit machinery. The panel shows only the attribute groups the NPC actually has (a subset of the player's).
- **State section (NPC only)**, shown above the attribute groups:
  - HP / MaxHP readout.
  - `SwitchListTile` **Dead** (bundle, see §5).
  - `SwitchListTile` **KillBounty / killable-for-XP** (see §5).

### 3. "Inventory" tab

- Left: the same `ActorSelector` (shared selection). Right: the existing inventory card, parameterized by the selected actor.
- **Player selected** → existing path (unchanged).
- **NPC selected** → core resolves the NPC inventory via a new `npc_inventory_path(root, id)`; `addItem` / `removeItem` thread an `actorId` through to `resolve_inventory_path`. **NPC item-count edits route through typed `private.typed.setValue` on the slot's `m_ItemDefinition`/`m_ItemCount`** rather than the player-specific untyped FString-region scan, so the byte-region scoping logic does not need an NPC variant.
- NPC inventory for display is loaded on selection (extend `inspect_save` with an optional `actorId`, or a dedicated read command — decided in the plan).

### 4. Core changes (`crates/gore-save`)

- `properties.rs`: new primitives `tag_container_add` / `tag_container_remove` — splice an fstring in/out of a native `GameplayTagContainer`, adjust the `u32` count, and cascade enclosing size fields (reuse the `resolve_chain` enclosing-size-fields + `data_size_offset` cascade pattern already covered by the `patch_string_*_fixes_size_chain` tests).
- `lib.rs`:
  - `npc_inventory_path(root, global_id)` + refactor `resolve_inventory_path(root, actor_id: Option<&str>)` to dispatch player vs NPC. Add optional `actorId` to the inventory edit structs and thread it through the apply functions.
  - `private.npc.list` → paginated/searchable `[{ id, name, isDead, hasKillBounty, hp, maxHp }]`. `isDead` = HP 0; `hasKillBounty` = `State.KillBountyGranted` present in any captured container.
  - `private.npc.attributes(id)` → attribute rows + their `setValue` paths.
  - `private.npc.setDead(id, bool)` and `private.npc.setKillBounty(id, bool)` — encapsulate HP changes + tag add/remove across all of the NPC's captured containers + size cascade (built on the new tag primitive).

### 5. Switch semantics & edge cases

- **Dead switch (bundle HP + tag):**
  - OFF → ALIVE (revive): set `Health` `BaseValue` + `CurrentValue` = `MaxHealth.BaseValue`, **and** remove `State.Dead` from all of the NPC's captured containers.
  - ON → DEAD (kill): set `Health` `BaseValue` + `CurrentValue` = `0`. No tag-add — the game re-derives `State.Dead` from HP 0 on load; faking a captured snapshot is avoided.
- **KillBounty switch:** toggles `State.KillBountyGranted` across the NPC's captured containers.
- **Asymmetry (known limitation):** the OFF / revive / clear direction is always clean and is the real use case. The ON direction is partial: "Dead ON" relies on HP 0; "KillBounty ON" needs at least one existing captured container to host the tag — NPCs with no `ActiveEffects` entry (655 in the sample) cannot host it, so the op is a no-op with a surfaced warning. Setting these states ON is a niche direction; revive/clear is the goal.

### 6. Out of scope

- **Loot restore** (refilling `m_ItemDefinition` / `m_ItemCount` that the player looted off a corpse) is a separate feature: death does not null loot — looting does — so restoring it needs a fresh-save reference keyed by GlobalId. Not part of this work.

## Testing

- Revive roundtrip on real `G1R-001` (47 dead NPCs as fixtures): after `setDead(false)`, HP == MaxHealth and `State.Dead` gone from all captured containers; re-parse succeeds.
- `OC_VLK_Herek_511` `setKillBounty(false)`: `State.KillBountyGranted` removed from all his captured containers; HP untouched.
- NPC inventory add / remove / count edits roundtrip on a chosen NPC; player inventory edits unchanged (regression).
- Size-cascade correctness after a tag removal (enclosing struct/array/map size fields updated; whole-save re-parse consumes all bytes).
- Byte-identical roundtrip for actors that were not edited.

## Implementation phasing

One spec, two implementation phases:

1. Actor selector + shared selection + Attribute tab NPC support + Dead/KillBounty switches + the new tag primitive and `npc.*` commands.
2. Inventory tab NPC support (`npc_inventory_path`, `actorId` threading, typed count edits).
