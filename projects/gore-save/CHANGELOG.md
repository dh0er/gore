# Changelog

All notable changes to goresave are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] - 2026-06-17

- Codec host is now replaced with an in-process Oodle Kraken codec. This removes
  the dependency on the game executable and the codec helper executable. Now any
  game version should work.
- Any NPC can now be selected from catalog and be added to knowledge list.
- Fixed a bug where the Add button was missing in inventory tab.

## [0.2.1] - 2026-06-15

### Fixed

- Saves containing a `FieldPathProperty` no longer break every typed-parse tab.
  The typed property parser aborted on the first unsupported property type,
  which failed the whole typed parse and left the All data tab showing a parse
  error, the Progression tab locked, and the Player/Inventory edit controls
  gated off. `FieldPathProperty` is now kept as opaque bytes (read-only,
  round-tripped on save), like `TextProperty`.

## [0.2.0] - 2026-06-14

### Added

- The difficulty level of a profile can now be changed.
- The inventory view is organized by a category sidebar (weapons, ammunition,
  runes, scrolls, food, misc, amulets, rings, trophies, writings, mission
  items, keys, and other), matching the Player and Progression tabs.
- Users can add items not yet in the save via a searchable picker that browses
  the full bundled item catalog (Gothic 1 Remake item IDs) by category.
- Items can be removed from the inventory with a per-row delete button. Item
  counts are clamped to a minimum of 1; deleting an item removes its slot
  rather than leaving a count-0 ghost.
- New game builds are now adopted automatically. When the editor sees a
  `G1R-Win64-Shipping.exe` it does not recognize (e.g. a fresh patch), it
  silently verifies the embedded compression code against a built-in
  known-answer sample and, if it round-trips correctly, enables editing for
  that build without waiting for an editor update. The verified result is
  cached per executable, so the check runs only once per new build.

### Changed

- When a game version genuinely cannot be opened, the editor now shows a
  plain-language message ("This game version can't be opened yet") with a
  suggested next step, instead of internal codec details. The technical
  fields are still available behind a "Details" expander.

## [0.1.2] - 2026-06-12

### Fixed

- The G1R binary codec host now recognizes the 1.0.1 game patch
  (`G1R-Win64-Shipping.exe`). The patch shifted the embedded Oodle
  compress/decompress/dispatch functions, so the host fell back to pattern
  resolution and reported the executable as unsupported, disabling
  compression. Added a verified known profile (`g1r-99E4AF08`) with the new
  codec RVAs; compress and decompress were live round-trip tested against the
  patched executable.

## [0.1.1] - 2026-06-12

### Fixed

- Saving no longer fails with a codec timeout error on slower machines. The
  codec worker timeout now scales with save size (60s base + 1s per MiB)
  instead of the fixed 5 seconds that the quick selftest uses.

## [0.1.0] - 2026-06-11

### First Release

- Player: Edit stats, skills, location and much more
- Inventory: Change count of existing items. Adding new items is not yet implemented.
- Progression: Edit quest markers, NPC knowledge and events
- Almost all data can be changed by changing the value of the internal property. Only for experimental use.
- Automatic backup creation.
