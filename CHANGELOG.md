# Changelog

All notable changes to goresave are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-06-11

### Added

- Browse Gothic Remake save slots, inspect GSAV container metadata and
  PersistentDataList details.
- Edit supported save metadata with automatic backups and slot-name sync.
- Overview, player, inventory, progression, and JSON views into parsed saves.
- List and restore backups created by the editor.
- Optional out-of-process helper for advanced private payload support.
- Automatic updates: the installed app checks GitHub Releases on startup,
  downloads new versions in the background, and applies them on restart.
