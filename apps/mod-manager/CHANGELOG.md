# Changelog

All notable changes to gore-manager are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - Unreleased

- First Early Alpha release candidate. Publication remains blocked on the
  clean-machine and real-install acceptance passes.
- Mod library, load order, conflict detection, and declarative Apply/Undeploy.
- Windows installer with WinSparkle update checks; the portable zip remains
  self-contained and updater-free.
- Documented that uninstalling removes the app and its normal
  `%LOCALAPPDATA%` UI preferences, while an `%APPDATA%` fallback needs manual
  cleanup; uninstall does not undeploy mods or erase the shared imported
  library, loadout, and GORE configuration.
- Included WinSparkle 0.8.1, Expat, and OpenSSL attributions in the shipped
  third-party notices.
