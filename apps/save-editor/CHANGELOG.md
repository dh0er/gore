# Changelog

All notable changes to the GORE Save Editor are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- A Trade tab shows what a merchant offers for sale and how much ore he has to
  buy with. Stock counts and his ore can be changed, lines can be added and
  removed, and the restock baseline is editable next to the live stock.
- An NPC can be moved, with the same location picker the hero has.
- An NPC's daily routine can be switched off, so he stays where he was put
  instead of walking back within seconds. It can be switched back on again, as
  long as the savegame still holds what the move wrote.

### Changed

- Saving is much faster: a save with eight changed values took eleven seconds
  and now takes one.
- Opening a savegame is about four times faster.
- Tabs no longer load one by one. Everything they show is fetched in the
  background as soon as the savegame opens, so switching tabs is immediate.
- Going back to a savegame, or to a tab already visited, no longer reloads
  anything.

### Fixed

- Version 1.2.1 said an NPC's position cannot be changed because the game
  restores it from the level. That was wrong.
- Removing an item from a list and then editing another item in the same list
  could change the wrong one.

## [1.2.1] - 2026-08-03

### Added

- Backups can be given a name and deleted. The name heads the entry; the file
  name stays visible below it and is never changed.
- Added a "Position" tab to Characters. The hero's position moved there from the
  Attributes tab and can be picked from a list of 10,075 named locations from
  the game, grouped by region, instead of typing coordinates. Applying a
  location's facing is optional and off by default.
- An NPC's position and spawn position are shown but cannot be changed: the
  game restores an NPC's placement from the level, not from the savegame.
- A savegame damaged by an earlier version is now recognised on load: the
  overview and the inventory show a warning with the number of affected slots
  and a repair button, applied with the next save. Repairing can be taken back
  until then.

### Fixed

- Renaming a savegame could make the game drop it from the load list. The name
  is now written so the game can still read the save's metadata, and a rename
  that would leave it unreadable is refused instead of written.
- Items added to an inventory could not be dropped in the game afterwards, and
  dropping one made a different item vanish instead. Added items now go into a
  free inventory slot, and removing an item frees its slot instead of deleting
  it — the way the game does it. Adding or removing an item also repairs a
  savegame an earlier version left in that state.
- Large inventories were cut off at 200 entries, so a newly added item was
  missing from the list while the picker already refused to offer it again.
- Queued additions and removals now show the item's localized name.

## [1.2.0] - 2026-07-15

### Added

- Added an "Other saves" view for persistent management of profileless and
  external savegames.
- Savegames can now be assigned or removed from a profile.
- Added a searchable and filterable "Story state" section to the World tab. It
  combines values stored in the save with the complete catalog of persistent
  fields declared by the shipped game scripts, including fields that are not
  yet set.
- Story-state entries can be set, edited, removed, and undone before saving.
  Source-backed classifications and value suggestions, structured day/time
  editing, elapsed-time context, and exact Glossary matches help explain known
  values; unknown IDs from mods or newer game versions remain available as raw
  signed integers.

### Changed

- Reworked "All data" into a source-aware GSAV browser for metadata and the
  complete typed PUBLIC and PRIVATE property trees.
- Memory events now use localized, categorized cards with readable actions and
  subjects. Expandable details retain timing, participants, positions, payload
  fields, tags, and technical IDs, and multiple removals can be queued and
  individually undone before saving.
- Improved the quest layout and hierarchy.
- Moved tutorials to the Glossary.
- Almost all dialog knowledge IDs are now replaced with localized texts.
- Improved UI consistency and translations across the editor.

## [1.1.0] - 2026-07-14

### Added

- Added a dedicated Glossary section to the World tab for NPCs, creatures, and
  locations. NPCs are grouped by camp and can be filtered by traders, teachers,
  armorers, hostile relationships, or death entries.
- Glossary entries can be added or removed individually, including NPC
  discovery and portrait visibility. Entry text follows the selected game
  language and can be viewed in full from truncated rows.
- The stored NPC-to-player relationship can now be set to friend, neutral, or
  enemy from the Events tab.

### Fixed

- NPC status, relationship, and glossary controls no longer briefly revert to
  stale values while a saved game is refreshed.

## [1.0.0] - 2026-07-08

### Added

- NPCs are now fully editable, not just the player: select any NPC and change
  its stats/attributes and inventory the same way you edit your own character.
- Armor is supported in the inventory — equipped and carried armor pieces show
  up as regular items and can be edited and added like anything else.
- Faction crimes can be cleared, resetting the hostility an NPC's guild holds
  against you.
- NPCs can be revived — bring a dead NPC back to life (restores health and
  strips the death state).
- Inventories can be reset to a clean starting state.
- The in-game time (play clock) can now be set.
- All talents/skills can now be edited (previously only a subset was exposed).

### Changed

- Reworked the UI navigation: first pick who you're editing (an NPC or the
  player), then choose what to edit (attributes, inventory, …). The layout is
  clearer and more consistent across tabs.
- Improved performance, especially responsiveness when editing.

## [0.4.0] - 2026-06-24

### Added

- The editor now speaks 10 languages. The app UI is fully localized, and the
  chosen language also drives the in-game text shown for items, quests, and
  knowledge entries.
- Game text is extracted from your installation on first run (or on demand),
  so item, quest, and knowledge-entry names appear in your language instead of
  raw IDs.
- Inventory items are shown and searchable/filterable by their localized names.
- Quest and knowledge-entry names are localized; NPCs and quests can be searched
  by name, and the sidebar groups entries for easier navigation.
- The portable version can now check for updates too. It compares against the
  latest release and, when a newer version exists, offers to open the download
  page in your browser (the installed version still updates itself).

### Fixed

- Progression lists are fully paginated, so large lists load completely; the
  localization cache is resolved from the game executable, and a failed page
  fetch no longer leaves a partial list.
- Numerous localization edge cases (first-run prompt, language-dropdown
  normalization, cache/status handling, snake-case acronym preservation).
- Live profile state is read through public forwarders for correct values.

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
