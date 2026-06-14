# Difficulty editor tab — per-save, with optional profile / all-saves propagation

Date: 2026-06-14
Status: Design approved by user (pending spec review)

## Problem

The game lets the player choose a difficulty (Novice / Gothic / Hard / Custom) only
once, at profile creation, and never again in-game. We want an editor tab to change
the difficulty of an existing playthrough.

Investigation showed difficulty is **not** profile-only — it is stored in three places:

1. `PersistentDataList.sav` -> `/Script/G1R.ProfileData` (profile level).
2. Each savegame's **public payload** -> `/Script/G1R.SaveGamePublicData` (uncompressed
   GVAS, verified at `public_properties` in the inspect output).
3. Each savegame's **private payload** -> `/Script/G1R.SaveDataPayload` (inside the
   compressed stream; verified in `work/decompressed/*.bin`).

The savegame's own copy (private payload, driving `DifficultyManagerSubsystem` on load)
is the authoritative gameplay value. The profile copy is the menu default / display for
new saves. So **editing only the profile would not change an existing save's
difficulty** — the edit has to target the savegame itself.

## Decisions (from brainstorming)

- The editor is **per-save**: a new tab bound to the currently selected savegame
  (`SaveInspection`), like the other editor tabs. This dissolves the original cross-save
  concern — the authoritative edit is per-save.
- Tab **position 2**, immediately right of Overview (index 1); remaining tabs shift
  right. Working name **"Schwierigkeit"** (shows profile overview read-only as context
  plus the editable difficulty). Final tab label TBD with user; not blocking.
- Editing a save writes difficulty into **both** the save's public payload and its
  private payload (the authoritative copy), so menu display and gameplay stay
  consistent.
- **Two opt-in checkboxes**, both default OFF:
  - *"Auch das Profil aktualisieren"* — also write the difficulty into the profile's
    `ProfileData` in `PersistentDataList.sav`.
  - *"Auf ALLE Savegames dieses Profils anwenden"* — also write every other savegame
    belonging to the same profile.
- **UI must explain** the three-place storage and what each checkbox does, so the user
  understands why editing one save does not by itself change others or the profile.
- **Backups are mandatory** for every file written, under one shared backup suffix, with
  atomic replace and rollback if any target fails. No partial writes.
- Edit scope: difficulty only. Profile/save overview fields stay read-only.

## Verified facts

Difficulty fields are `ClassProperty` / `BoolProperty` members. Confirmed values:

- Live `PersistentDataList.sav`: profile 0 = `DifficultyPreset_Custom` (sub-settings
  `*_Standard`); profile 1 = `DifficultyPreset_Easy` (all sub-settings present and
  `*_Easy`, plus the bools). So **all difficulty properties are always serialized**
  on every profile and every save -> all edits are in-place splices, no structural
  property insertion needed.
- Inspect output of a real save: difficulty present in `public_properties`
  (uncompressed) and again in the decoded private payload.

Preset mapping (confirmed): Novice = `Easy`, Gothic = `Standard`, Hard = `Hard`,
Custom = `Custom`.

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

Left untouched (present but not in the in-game difficulty screen): `m_Survival`,
`m_PermanentDeathGameOver`, `m_MaxQuick`, `m_MaxAuto`.

## Editability behavior

Mirrors the in-game difficulty screen (confirmed from the four preset screenshots —
orange = editable, grey = locked):

| Control | Novice | Gothic | Hard | Custom |
| --- | --- | --- | --- | --- |
| Flow Helper (`m_FakeSloppyCombos`) | editable | editable | editable | editable |
| Permadeath (`m_PermanentDeath`) | locked off | editable | editable | editable |
| Combat / Resources / Progression | locked = Novice | locked = Gothic | locked = Hard | editable |

Rules:

- **Flow Helper** always editable, independent of preset (in-game default on). Never
  forced.
- **Permadeath** editable for Gothic / Hard / Custom (default off). For Novice it is
  locked off: when preset = Novice, force `m_PermanentDeath = false`.
- **Sub-settings** editable only for Custom. For Novice / Gothic / Hard, on save rewrite
  all three to the matching level (mirrors the game) and show them disabled. Custom
  defaults the three to Gothic (`*_Standard`) on first switch, otherwise preserves
  stored values.

## Core (Rust) — `crates/goresave_core/src/lib.rs`

Single command **`write_difficulty`** so the all-or-nothing backup/rollback guarantee
lives in one place:

```
{
  difficulty: { preset, combat?, resources?, progression?, flowHelper?, permadeath? },
  targets: {
    saves:   ["<slot path>", ...],          // each gets public + private edits
    profile: { path: "<PersistentDataList.sav>", profileId } | null
  },
  backup: true
}
```

Per-save difficulty write (one helper, used for every entry in `targets.saves`):

- **Public payload:** ClassProperty / Bool splices on the uncompressed GVAS region.
  Extend `apply_public_edit` (`lib.rs:3994`) with the difficulty paths; reuse a
  ClassProperty-aware variant of the existing fstring splice.
- **Private payload:** same edits inside `SaveDataPayload`, applied through the existing
  private-edit + Kraken recompress pipeline (`apply_private_edits`, `lib.rs:3789`) that
  inventory edits already use.

Profile difficulty write (when `targets.profile` set): locate the profile range via the
same boundary logic as `parse_profile_summaries` (`lib.rs:904`), splice
`ProfileData` ClassProperty / Bool members in place.

Orchestration + safety:

- Validate ClassProperty on-disk layout against fixtures (TDD, byte-exact) — it may
  differ from `StrProperty`.
- Back up **every** target file under one `shared_backup_suffix` (`lib.rs` existing
  helper, today used for slot+companion). Write each to a `.tmp-goresave`, re-inspect /
  re-parse each to confirm the written value, then `begin_replace` all and commit; if
  any replace fails, roll back every already-committed target. Reuse the
  begin_replace / commit / rollback pattern from `write_save_internal` (`lib.rs:3854`).
- GSAV saves must still rebuild byte-identically except for the intended edit
  (`rebuild_gsav_preserving_stream`).

## Frontend (Flutter / Riverpod)

- New `_DifficultyPanel`, added to the `TabBar` tabs and `TabBarView` children at index 1
  in `apps/goresave/lib/features/editor/ui/editor_page.dart` (bump `TabController`
  length 7 -> 8, wrap in `_KeepAliveTab`).
- Bound to the selected save's `SaveInspection` (current save's difficulty is the edit
  subject). Shows the resolved profile overview (`state.activeProfile`) read-only as
  context.
- Layout mirrors the in-game screen: preset selector (4 options); a Custom block with
  two toggles (Flow Helper, Permadeath) and three 3-way pickers (Combat, Resources,
  Progression) enabled per the editability matrix.
- An **explanation block** stating: difficulty is stored in this save (gameplay-
  relevant), in the profile (menu default), and separately in every other save; this
  editor changes only the current save unless the checkboxes below are ticked.
- **Two checkboxes** (default off): update profile; apply to all saves of this profile.
- Own edit buffer + Save / Reset inside the tab. Save assembles `targets`
  (always the current save; + all profile slots if checked; + profile if checked) and
  dispatches `write_difficulty` via `EditorNotifier`, then refreshes.
- Extend the unsaved-edits / profile-switch guard (`editor_notifier.dart:347`) to cover
  dirty difficulty edits.

## Authority note (verify before relying on save edits)

Strongly inferred that the private `SaveDataPayload` drives loaded-game difficulty.
Recommend one empirical check during implementation: edit one save's difficulty, load
it in-game, confirm the change takes effect. If only public or only private matters,
narrow the per-save write accordingly.

## Testing

- **Core:** round-trip tests on (a) a `PersistentDataList.sav` fixture and (b) a real
  save fixture (`work/decompressed` payloads + a slot file). Cases: each preset switch,
  each bool toggle, each sub-setting; assert byte-exact re-serialization elsewhere and
  re-parse returns the written values, for public and private. Multi-target test:
  current save + profile + all saves all written and backed up; induced failure rolls
  everything back.
- **Frontend:** widget tests for the editability matrix (enable/disable per preset),
  checkbox-driven `targets` assembly, buffer Save/Reset, and the dirty-edit switch
  guard.

## Out of scope (v1)

- Editing `m_MaxQuick` / `m_MaxAuto`, `m_Survival`, `m_PermanentDeathGameOver`.
- Editing profile/save names or other overview fields.
- Creating, deleting, or batch-editing profiles.
