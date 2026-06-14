# Difficulty editor — profile-only, edited from the profile header

Date: 2026-06-14
Status: Design pending user spec review
Supersedes the per-save half of: `2026-06-14-profile-difficulty-tab-design.md`

## Problem / finding

The earlier difficulty editor wrote difficulty into three places (profile + each
save's public payload + each save's private payload), defaulting to a per-save
edit with two opt-in propagation checkboxes. Its "Authority note" flagged a risk
and asked for an empirical in-game check.

That check came back **inverted**: editing a save's own difficulty (public and
private) has **no in-game effect**. Only editing the **profile's** `ProfileData`
in `PersistentDataList.sav` changes difficulty — and it applies to **every save
in that profile**. The profile copy is the single authoritative, profile-wide
lever. The per-save copies are dead weight.

This redesign makes difficulty a **profile-level** edit, removes everything
per-save, and moves the editor to the profile header.

## Decisions

- **Profile-only write.** A difficulty edit writes only the active profile's
  `ProfileData`. The entire per-save public/private difficulty write path is
  deleted.
- **No per-save difficulty anywhere** — not read, written, or shown.
- **Editor moves to the profile header.** A prominent, clickable difficulty chip
  replaces the small `x saves | Quick y | Auto z` subtitle. Clicking it opens a
  modal **DifficultyDialog** that edits the profile's difficulty.
- **Self-contained dialog.** Own Save / Cancel. Save dispatches the profile-only
  write immediately (the core already takes a mandatory backup), then closes and
  refreshes. No confirmation step — a one-line hint states the scope.
- **Remove the three per-save difficulty render sites entirely** (sidebar
  subtitle, header info pill, diagnostics "Difficulty" metric). Do **not** restore
  the old map label; those sites are simply dropped.
- **Drop the two propagation checkboxes**, the `allSaves` / `alsoProfile` pending
  logic, the `PendingDifficulty` global-registry coupling, and the difficulty
  dirty-edit guard.

## Core (Rust) — `crates/goresave_core/src/lib.rs`

**Keep**
- `write_profile_difficulty` and its profile path (`parse_profile_file`,
  `profile_element`, `profile_difficulty_path`, the profile patch helpers).
- The bare-property-list PersistentDataList parse support
  (`parse_property_list_root_at` + dual-framing `parse_profile_file`) and the
  ordinal-fallback `profile_element` — these are what make profile writes work on
  real files.
- `ProfileSummary` difficulty fields (`difficulty_preset`,
  `custom_combat_settings`, `custom_resources_settings`,
  `custom_progression_settings`, `permanent_death`, `fake_sloppy_combos`, …).
  These already carry the full profile difficulty and become the only source for
  the chip label and the dialog seed.

**Delete**
- `apply_save_difficulty` and the per-save public+private difficulty splice
  plumbing it drove.
- The save-targeting branch of `write_difficulty_internal`. The command becomes
  profile-only: `{ difficulty, profile: { path, profileId } }` (no `saves[]`).
- The per-save `difficulty` field surfaced on `SaveInspection` / `SaveListItem`
  and the code that parses it during inspect/scan (`difficulty_for_gsav_bytes`
  and callers), if used only for the removed card/badges.
- Now-dead save-side helpers (`patch_difficulty_string`, `patch_difficulty_bool`,
  `write_permadeath_typed`, `validate_typed_reparse` for the save payload, …)
  **only if** no other feature uses them. Verify each before removing.

**Request shape (after)**
```
write_difficulty {
  difficulty: { preset, combat?, resources?, progression?, flowHelper?, permadeath? },
  profile:    { path: "<PersistentDataList.sav>", profileId },
  backup: true
}
```
Single target, single backup, atomic replace + rollback as today.

## Frontend (Flutter) — `apps/goresave/lib/features/editor/`

**Remove**
- `ui/difficulty_card.dart` (the whole `DifficultyCard`) and its placement in the
  Overview tab.
- The three per-save difficulty render sites in `ui/editor_page.dart`: the
  `_saveSlotSubtitle` difficulty text, the header `_InfoPill` difficulty, and the
  diagnostics "Difficulty" metric. Drop them outright (no map fallback).
- `PendingDifficulty` and the difficulty branch of the dirty-edit / profile-switch
  guard (`editor_notifier.dart`). Difficulty is no longer a global pending edit.

**Add — profile difficulty chip (`_ProfileHeader`)**
- Replace the `x saves | Quick y | Auto z` subtitle with a prominent difficulty
  **chip**: a filled pill, preset-tinted, with a fire icon
  (`Icons.local_fire_department_outlined`) and the difficulty label
  (Novice / Gothic / Hard / Custom; "Custom" when sub-settings are mixed).
  Clearly tappable (InkWell + hover/press state, pointer cursor). Disabled/greyed
  with an explanatory tooltip when the active profile has no resolvable
  difficulty.
- Save / Quick / Auto counts move into the chip's tooltip so no information is
  lost.

**Add — `DifficultyDialog` (modal)**
- Opened by tapping the chip. Seeded from the **active profile's**
  `DifficultySettings` (built from `ProfileSummary`).
- Same form as the old card: preset selector (Novice/Gothic/Hard/Custom), Custom
  block with Flow Helper + Permadeath toggles and Combat/Resources/Progression
  3-way pickers, governed by the existing editability matrix (Permadeath locked
  off for Novice; sub-settings editable only for Custom; etc.).
- One-line hint: **"Difficulty applies to all saves in this profile."**
- **Save**: assembles the profile-only `write_difficulty` request and dispatches
  it via `EditorNotifier`, closes, and refreshes. **Cancel**: closes, no write.
- Surfaces the codec/write error from the notifier inline (same error path as
  other writes); no new pending-registry coupling.

## Editability matrix (unchanged)

| Control | Novice | Gothic | Hard | Custom |
| --- | --- | --- | --- | --- |
| Flow Helper (`m_FakeSloppyCombos`) | editable | editable | editable | editable |
| Permadeath (`m_PermanentDeath`) | locked off | editable | editable | editable |
| Combat / Resources / Progression | locked = Novice | locked = Gothic | locked = Hard | editable |

Preset mapping: Novice=`Easy`, Gothic=`Standard`, Hard=`Hard`, Custom=`Custom`.

## Testing

**Core**
- Profile-only `write_difficulty` round-trip on both PersistentDataList framings
  (object-wrapped and bare property list): each preset switch, each bool toggle,
  each sub-setting; assert re-parse returns the written values and the file
  re-serializes byte-exact elsewhere. Induced-failure rollback restores the
  original file.
- Assert the removed save-write path is gone (no public/private difficulty edit).

**Frontend**
- Widget test: chip renders the active profile's difficulty label and tint; tap
  opens the dialog.
- Widget test: dialog editability matrix (enable/disable per preset), Save builds
  the profile-only request and dispatches it, Cancel writes nothing.
- Regression: Overview tab no longer contains a difficulty card; save rows no
  longer show a difficulty/map pill or metric.

## Out of scope

- Editing `m_MaxQuick` / `m_MaxAuto`, `m_Survival`, `m_PermanentDeathGameOver`.
- Per-save difficulty in any form.
- Creating / deleting / batch-editing profiles.
