# Player Tab Hero Editing — Design

Date: 2026-06-10
Status: approved by user (chat), pending spec review

## Problem

The Player tab's "Private FString editor" lists raw strings scanned from the
decompressed private payload. Most entries are property names, script paths,
and tags — not values — so the card is useless for actual editing and invites
save-corrupting edits (renaming a property name breaks the structure). The
real hero values (attributes) are shown by the hero attributes card, but it
only covers 9 hard-coded IDs read via a byte-scan heuristic
(`private_player_attribute_refs`, whitelist in `hero_attribute_supported`,
`crates/goresave_core/src/lib.rs:4200`). Learn points, combat skills,
resistances, thieving stats etc. exist in the save but are invisible.

Goal: the Player tab edits everything player-related in a friendly, grouped
UI, and the useless FString card disappears.

## Findings (from a real save, G1R-007, 482k typed properties)

- Hero stats live at
  `m_GenericData/{CharacterStates}/AnyCharacterType/AttributesByGlobalId/{Hero}/AttributeSetsByClass/{...}/Attributes/{<Id>}/BaseValue|CurrentValue`.
  All are `FloatProperty` and editable via the existing
  `private.typed.setValue` write path.
- ~55 attribute IDs observed (each with BaseValue + CurrentValue), including: Health/MaxHealth, Mana/MaxMana,
  Strength, Dexterity, Level, Experience, SkillPoints (= learn points),
  MagicianLevel, Critical_Fists/OneHand/TwoHand/Orc, Resistance_Blunt/Edge/
  Point/Fire/Energy/Ice/Wind/Falling, LockpickDurability, LockpickPrecision,
  PickPocketing, Oxygen/MaxOxygen, Fatigue, SleepTime, Alcohol, Swampweed,
  SpeedModifier, DamageMultiplier, Toughness(A/B/C), SuperArmor, …
- Learned talents and guild membership are `ActiveEffects` entries
  (`GE_Skill_*`, `GE_Guild_*`) — complex object structures.
- Player equipment/inventory slots live under
  `{PlayersSavedData}/m_SavedPlayers` (separate concern, Inventory tab).
- `search_typed_properties` matches whitespace-separated, case-insensitive
  terms against the display path (segments joined with `" › "`), so the fixed
  two-term query `AttributesByGlobalId {Hero}` returns exactly the hero
  attribute leaves.
- **Core gap found during feasibility probing:** `AttributeSetsByClass` is a
  map with ObjectProperty keys. `map_key_to_string` in
  `crates/goresave_core/src/properties.rs` cannot stringify Object keys, so
  search labels them `{?}` and `private.typed.setValue` fails with
  `map key "?" not found` — the hero attributes are reported editable but are
  not actually writable today. Extending `map_key_to_string` with
  `PropertyValue::Object` fixes label and resolve in lockstep; verified on a
  real save (MaxHealth 64→65 written and read back).
- The full private decode is cached after `inspect_save`
  (`store_decoded_payload_cache`), so the typed search in the Player tab does
  not pay a second ~20s decode.

## Design

### 1. Hero stats card (new, typed-backed)

Replaces the heuristic attributes card in the Player tab whenever the core
reports `private.typed.setValue` as writable (strict typed parse OK).

- Data: `searchTypedProperties('AttributesByGlobalId/{Hero}', limit: 1000)`.
- Dart parser folds hits into
  `HeroAttribute { id, basePath, currentPath, base, current }` (pairs the
  `.../{Id}/BaseValue` and `.../{Id}/CurrentValue` leaves).
- Grouped UI sections:
  - **Hauptwerte:** Health/MaxHealth, Mana/MaxMana, Strength, Dexterity,
    Level, Experience, SkillPoints (labelled "Lernpunkte"), MagicianLevel
  - **Kampf-Skills:** Critical_Fists, Critical_OneHand, Critical_TwoHand,
    Critical_Orc
  - **Resistenzen:** the 8 `Resistance_*` IDs
  - **Diebeskunst:** LockpickDurability, LockpickPrecision, PickPocketing
  - **Erweitert** (collapsed by default): every remaining ID, including IDs
    unknown to the app — unmapped attributes land here automatically so
    nothing is silently dropped.
- Editing: numeric fields for base/current; writes go through
  `private.typed.setValue`. A new batch write variant sends all dirty fields
  of one card as a single `write` request (one backup per save action).
- Fallback: when the typed parse is not OK, the existing heuristic attributes
  card stays (today's behaviour, byte-scan + `private.player.setAttribute`).

### 2. Unchanged editors

Player name, profile name, and transform editors stay as they are.

### 3. FString editor removal

`_PrivateFStringEditor` is deleted from the UI entirely (Player tab and
class). The core write path `private.replaceFString` stays for API
compatibility; the "All data" browser covers string edits in the UI.

### 4. Out of scope

- Talents/guild via `ActiveEffects` (`GE_Skill_*`, `GE_Guild_*`): not
  editable in this iteration — nested object specs, corruption risk. Possible
  later follow-up.
- Player equipment/inventory under `{PlayersSavedData}` (Inventory tab
  concern).
- NPC attributes (same shape under other GlobalIds) — Player tab is hero-only.

## Error handling

- Typed search returns an error or zero hero hits → fall back to the
  heuristic attributes card; no hard failure.
- Write failures surface through the existing `_runWrite` snackbar/messaging
  path, unchanged.
- Non-numeric input is rejected in the field validator before any write.

## Testing

- Dart unit tests: TypedSearchResult hits → `HeroAttribute` pairing, group
  assignment (known IDs to their groups, unknown IDs to "Erweitert").
- Widget test: hero stats card renders groups, edits a value, dispatches the
  batched write with the correct typed paths.
- Rust core: one targeted change — Object map keys become addressable in
  typed property paths (`map_key_to_string`), with a unit test that a search
  hit under an Object-keyed map round-trips through `resolve`. Everything
  else reuses existing commands; existing tests stay green.

## Decisions log

- Combined approach (curated cards + generic coverage) chosen by user; the
  "Erweitert" group is the generic coverage.
- Typed paths over extending the byte-scan whitelist or embedding the raw
  browser: chosen by user.
- FString editor: delete from UI completely (user choice over moving it to
  the Advanced tab).
