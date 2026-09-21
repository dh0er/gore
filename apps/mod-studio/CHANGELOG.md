# Changelog

All notable development changes to the unreleased Mod Studio are documented
here. Once releases begin, the release workflow will publish the matching
version section as the GitHub release notes.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Compile against the deployment's pristine backup while a script mod stays
  installed; no undeploy is needed between iterations.
- Development baseline for the Gothic 1 Remake no-code modding GUI.
- Accept the exact 2026-08-27/28 game generation (Steam BuildID 24878692) in
  project creation and NPC authoring, and keep strict standalone compiler
  checks offline with native file/line/column/severity diagnostics.
- Windows installer and in-app update infrastructure are under development.
