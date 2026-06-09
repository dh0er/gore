# Profile And Savegame Screenshot Design

## Goal

Show Gothic Remake profile and savegame information, including save screenshots,
inside the existing GoReSave editor. The display should make the same facts a
player expects from the in-game profile/savegame selection visible in the app:
profile identity, quick-save and auto-save grouping, saved slot membership,
slot metadata, and screenshots.

## Current State

The Flutter app scans a selected save directory for `.sav` files and shows a
text-only save list in the left sidebar. The Rust core already enriches slots
from `PersistentDataList.sav` with public metadata such as save name, chapter,
map, played time, quick-save flag, auto-save flag, and profile ID.

Local research artifacts confirm the remaining data source:

- `PersistentDataList.sav` contains `m_Profiles`, `m_QuickSaveName`,
  `m_AutoSaveName`, and `m_SavedSlotsNames`.
- `Profile_0_Screenshots.sav` contains `m_Screenshots` keyed by slot names such
  as `G1R-001`.
- The screenshot payloads are JPEG bytes. A local extraction found screenshots
  for `G1R-001` through `G1R-006`.

Reference screenshots mentioned in the original request are not present as
files in the workspace. This design targets data and interaction parity with
the in-game profile/savegame selection rather than pixel-perfect replication.

## Proposed Experience

Keep the existing editor shell: app bar, status strip, left save browser, and
right tabbed workspace. Upgrade the save browser and Overview tab so profile
and screenshot information are first-class.

The left sidebar becomes a wider save browser with:

- A profile header showing the selected profile name, profile ID, number of
  saved slots, quick-save slots, and auto-save slots.
- Save cards instead of plain rows.
- Each card shows a screenshot thumbnail when available.
- Each card shows slot ID, save display name, save kind, chapter, map, played
  time, and file status.
- Selected cards remain visually distinct and keep the current click-to-inspect
  workflow.

The Overview tab gains a large screenshot hero for the selected save:

- Screenshot fills a stable aspect-ratio area.
- Text overlays or adjacent compact facts show save name, slot ID, kind,
  chapter, map, played time, profile name or ID, file size, and compression.
- Existing metadata editing remains below the hero.

If screenshots are unavailable, the UI shows a stable placeholder with the slot
name and keeps all metadata visible.

## Data Model

Add typed profile and screenshot data to the core JSON contract:

- `profiles`: list of profile summaries returned by `scan_save_dir`.
- `activeProfileId`: selected or inferred profile ID, initially derived from
  slot metadata. For the current single-profile files this will be `0`.
- `SaveListItem.screenshot`: optional screenshot summary containing MIME type,
  byte length, and base64 JPEG bytes.
- `SaveInspection.screenshot`: optional screenshot summary for the selected
  save.
- `ProfileSummary`: profile ID, profile name, quick-save slot names, auto-save
  slot names, saved slot names, difficulty object paths, survival/permadeath
  flags, max quick saves, and max auto saves when present.

The Flutter domain layer mirrors these structures with immutable models. The
existing `SaveSlot` remains the primary list item, extended with optional
screenshot and profile context.

## Core Parsing

Extend the Rust core in small, testable units:

1. Parse profile summaries from `PersistentDataList.sav`.
2. Locate the matching screenshot GSAV file for each profile:
   `Profile_<profileId>_Screenshots.sav`.
3. Decode the screenshot save private stream through the existing codec backend
   path when a backend is configured.
4. Parse `m_Screenshots` and extract JPEG bytes keyed by slot name.
5. Attach screenshots to scanned slots and inspected saves by slot key.

The core must continue to work when any companion file is missing, malformed,
or undecodable. These failures should be represented as missing optional
screenshot/profile details, not as a failed save-directory scan.

## UI Design

The app remains a utility/editor, not a landing page. The screenshot display is
functional and dense:

- Sidebar width increases enough for legible thumbnails and metadata.
- Save cards use a fixed thumbnail aspect ratio to avoid layout shifts.
- Cards keep 8px-or-less radii, consistent with the existing theme.
- Icons are used for refresh, folder selection, save kind, and missing
  screenshot states.
- Text stays compact and wraps or ellipsizes inside fixed constraints.
- The palette remains close to the current neutral/teal/gold theme and avoids a
  one-note game-menu skin.

The design should feel recognizably informed by the game save selection, but
still belong to GoReSave as an editor.

## Error Handling

- Missing `PersistentDataList.sav`: show the existing save list with no profile
  header details beyond an "Unknown profile" state.
- Missing screenshot save: show profile metadata and screenshot placeholders.
- Codec unavailable for screenshot save: keep save scanning usable and mark
  screenshots unavailable.
- Screenshot bytes malformed: omit that slot's image and show a placeholder.
- Large screenshots: pass JPEG bytes as base64 only for visible metadata scale;
  avoid writing extracted image files as part of normal app behavior.

## Testing

Core tests should cover:

- Profile parsing from a synthetic `PersistentDataList.sav`.
- Screenshot extraction from a synthetic screenshot GSAV/private payload.
- Scan results attaching screenshots to matching slots.
- Missing or malformed companion files do not fail `scan_save_dir`.

Flutter tests should cover:

- Domain models parse profile and screenshot fields.
- Sidebar renders profile header, screenshot thumbnails, and fallback
  placeholders.
- Overview renders the selected save screenshot hero and key metadata.
- Existing edit, backup, and settings flows remain visible and usable.

Manual verification should include:

- Running the full repository test suite.
- Running the Flutter widget tests.
- Opening the app against a local save directory with
  `Profile_0_Screenshots.sav` and verifying screenshot cards and the Overview
  hero render correctly.

## Non-Goals

- Pixel-perfect recreation of the game menu.
- Editing screenshots.
- Creating or deleting profiles.
- Changing the existing private player, inventory, progression, or backup edit
  behavior.
- Persisting extracted screenshots as files during normal app use.

## Open Decisions Resolved

- Use the hybrid UI: keep the existing editor and enrich the left save browser
  plus Overview.
- Treat screenshots as optional data.
- Prefer data parity with the game selection over a separate game-menu clone.
