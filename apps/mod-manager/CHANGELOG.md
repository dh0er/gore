# Changelog

All notable changes to gore-manager are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-07-03

- Early Alpha release, published as a GitHub prerelease rather than a stable
  general-use build.
- Mod library, load order, conflict detection, and declarative Apply/Undeploy.
- Windows installer with WinSparkle update checks; the portable zip remains
  self-contained and updater-free.
- Documented that uninstalling removes the app and its UI preferences but does
  not undeploy mods or erase the shared imported library, loadout, and GORE
  configuration.
- Included WinSparkle 0.8.1, Expat, and OpenSSL attributions in the shipped
  third-party notices.
