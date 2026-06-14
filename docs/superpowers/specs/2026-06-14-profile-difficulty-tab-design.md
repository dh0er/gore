# Profil tab — view + edit profile difficulty

Date: 2026-06-14
Status: Design approved by user (pending spec review)

## Problem

The game lets the player choose a difficulty (Novice / Gothic / Hard / Custom) only
once, at profile creation, and never again. The difficulty actually lives in the
profile, which the editor already parses read-only (`ProfileSummary`). We want a new
**"Profil" tab** that displays profile overview data and lets the user edit the
difficulty settings of an existing profile.

## Decisions (from brainstorming)

- **Placement:** a new editor tab **"Profil"** at **position 2**, immediately right of
  Overview (tab index 1). The remaining tabs shift right.
- **Binding:** the tab is bound to the **effective profile**
  (`EditorState.activeProfile` / `effectiveProfileId`), NOT to the selected savegame.
  This is the smart resolution of the cross-save problem: the profile is already
  resolved app-wide (sidebar profile selector + the current save's profile), so the
  tab reads that shared state instead of `SaveInspection`. The other 7 tabs stay
  bound to the selected save; only Profil is profile-scoped.
- **Edit scope:** difficulty only (full). Profile overview fields (name, id, slot
  counts, MaxQuick/MaxAuto) are shown **read-only**. Editing those is out of scope v1.
- **Core write op:** a dedicated command `write_profile_difficulty` targeting
  `PersistentDataList.sav` (keeps `write_save`'s slot/private logic clean).

## Verified facts (from live PersistentDataList.sav + UE4SS dump)

The difficulty fields are `ClassProperty`/`BoolProperty` members of
`/Script/G1R.ProfileData`, stored per profile inside `PersistentDataList.sav` (GVAS),
in the profile's ref range located by `parse_profile_summaries`
(`crates/goresave_core/src/lib.rs:904`).

Confirmed against the user's live file (two profiles present):

- Profile 0 (Custom): `m_difficultyPreset = DifficultyPreset_Custom`, sub-settings all
  `*_Standard`.
- Profile 1 (Novice): `m_difficultyPreset = DifficultyPreset_Easy`, **and all three
  sub-settings are present and = `*_Easy`**, plus the bools.

Two assumptions therefore hold:

1. **Preset mapping** — Novice = `Easy`, Gothic = `Standard`, Hard = `Hard`, Custom =
   `Custom`.
2. **All difficulty properties are always serialized** on every profile, regardless of
   preset (offsets are contiguous `0x40–0x68` on `ProfileData`). So switching any
   profile to/from Custom is a pure **in-place edit** — no structural property
   insertion needed. This removes the main implementation risk.

The game mirrors the sub-settings to the preset level (a Novice profile stores all
`_Easy` sub-settings). We replicate that: see Behavior below.

## Field mapping

Asset-path prefix: `/Script/Angelscript.`

| UI control | property | type | values |
| --- | --- | --- | --- |
| Preset | `m_difficultyPreset` | ClassProperty | `DifficultyPreset_{Easy,Standard,Hard,Custom}` |
| Combat (Custom) | `m_customCombatSettings` | ClassProperty | `CombatDifficultySettings_{Easy,Standard,Hard}` |
| Resources (Custom) | `m_customResourcesSettings` | ClassProperty | `ResourcesDifficultySettings_{Easy,Standard,Hard}` |
| Progression (Custom) | `m_customProgressionSettings` | ClassProperty | `ProgressionDifficultySettings_{Easy,Standard,Hard}` |
| Close Combat Flow Helper | `m_FakeSloppyCombos` | BoolProperty | on/off |
| Permadeath | `m_PermanentDeath` | BoolProperty | on/off |

Sub-setting level labels follow the same Novice/Gothic/Hard naming as the preset
(`Easy`/`Standard`/`Hard`).

Left untouched (present on `ProfileData` but not in the in-game difficulty screen):
`m_Survival`, `m_PermanentDeathGameOver`, `m_MaxQuick`, `m_MaxAuto`.

## Behavior

- When **preset = Custom**: the Custom block (Flow Helper, Permadeath, Combat,
  Resources, Progression) is enabled and each field is independently editable.
- When **preset = Novice/Gothic/Hard**: the Custom block is shown disabled, and on save
  the three sub-settings are rewritten to the matching level (mirrors the game) while
  the bools are left as-is. (Confirm bool behavior against the fixture; if the game
  also resets bools for non-Custom presets, match that.)
- Editing difficulty rewrites the whole profile and affects all of its saves — show a
  one-line note to that effect near the Save action.

## Core (Rust) — `crates/goresave_core/src/lib.rs`

New command `write_profile_difficulty`:

- Payload: `{ path, profileId, difficulty: { preset, combat?, resources?,
  progression?, flowHelper?, permadeath? } }` where `path` resolves to the save
  directory / `PersistentDataList.sav`.
- Locate the profile's ref range via the same boundary logic as
  `parse_profile_summaries` (`m_ProfileName` start, next-profile / `SavedDataVersion`
  end). Match `profileId` via `m_ProfileId`.
- ClassProperty string edits: a ClassProperty-aware variant of
  `replace_str_property_fstring_in_range` (`lib.rs:3958`). Verify the on-disk
  ClassProperty layout against the fixture (TDD, byte-exact) — it may differ from
  `StrProperty`.
- Bool edits: in-place single byte (reuse the bool read path's offset logic).
- Safety pipeline mirrors `write_save_internal` (`lib.rs:3762`): backup, write tmp,
  re-parse + validate (round-trip the written values back through
  `parse_profile_summaries` and assert they match), atomic `begin_replace`/commit.
- Reject if a targeted property is not found in the profile range (defensive; not
  expected given the always-present finding).

## Frontend (Flutter / Riverpod)

- New `_ProfilePanel` widget, added to the `TabBar` tabs and `TabBarView` children at
  index 1 in `apps/goresave/lib/features/editor/ui/editor_page.dart` (bump the
  `TabController` length 7 -> 8). Wrap in `_KeepAliveTab`. Icon e.g.
  `Icons.badge_outlined` / `Icons.person_pin_outlined`, label "Profil".
- Reads `state.activeProfile`. Empty state ("Kein Profil ausgewählt") when none
  resolves.
- Layout mirrors the in-game difficulty screen: a preset selector (4 options) and a
  Custom block with two toggles (Flow Helper, Permadeath) and three 3-way pickers
  (Combat, Resources, Progression) that enable only for Custom. Read-only profile
  overview (name, id, slot counts) above.
- **Own edit buffer + Save/Reset** inside the tab, independent of the save-scoped
  pending-edit buffer. Save dispatches `_core.execute('write_profile_difficulty', …)`
  via `EditorNotifier`, then refreshes profiles.
- Extend the unsaved-edits / profile-switch guard
  (`editor_notifier.dart:347`) so Profil edits block profile/save switches the same way
  save edits do.

## Testing

- **Core:** parse + write round-trip test on a `PersistentDataList.sav` fixture (copy a
  sanitized snapshot of the live file into `fixtures/`). Cases: Custom→Novice,
  Novice→Custom, toggle each bool, change each sub-setting. Assert byte-exact GVAS
  re-serialization elsewhere and that re-parse returns the written values.
- **Frontend:** widget test for preset→Custom enable/disable, buffer Save/Reset, and
  the profile-switch guard with dirty Profil edits.

## Out of scope (v1)

- Editing `m_MaxQuick` / `m_MaxAuto`, `m_Survival`, `m_PermanentDeathGameOver`.
- Editing profile name / overview fields.
- Creating, deleting, or batch-editing profiles.
