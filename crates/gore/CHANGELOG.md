# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

First release. Command-line toolkit for modding Gothic 1 Remake.

- `mod` and `mgr` — build a bundle from one spec, deploy or undeploy it, and run
  several mods together with a conflict report.
- `loc`, `audio`, `voice`, `texture`, `asset`, `as` — edit localized text, FMOD
  banks, voice-over archives, IoStore textures, cooked DataAssets and the
  AngelScript cache.
- `mcp serve` — all 78 commands over the Model Context Protocol, with every
  installation change confirmed first.
- `guide` — the manual, built into the binary and rendered to one HTML file.
