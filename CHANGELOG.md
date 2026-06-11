# Changelog

All notable changes to goresave are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- New Windows installer (Inno Setup): the setup wizard now lets you choose
  the install directory, including per-user installs without admin rights.
- Auto-updates now use WinSparkle with signed update feeds. **Users of
  v0.1.0 must download and run the new installer manually once** — the old
  updater's feed is no longer published.

## [0.1.0] - 2026-06-11

### First Release

- Player: Edit stats, skills, location and much more
- Inventory: Change count of existing items. Adding new items is not yet implemented.
- Progression: Edit quest markers, NPC knowledge and events
- Almost all data can be changed by changing the value of the internal property. Only for experimental use.
- Automatic backup creation.
