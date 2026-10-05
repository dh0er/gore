# Changelog

All notable development changes to the unreleased Mod Studio are documented
here. Once releases begin, the release workflow will publish the matching
version section as the GitHub release notes.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Retain exact, complete original `.as` inputs, including empty modules, by
  default in the script compiler, with fingerprints of replaced vanilla modules. Automatically
  include these sources alongside compiled scripts when building bundles.
- Let Manager rebuild source-backed script bundles against an updated pristine
  cache with its packaged standalone compiler. Rebuilds require compatible
  compiler support and native bindings and do not merge updated vanilla logic
  into the authored modules. Bundles without original sources remain binary-only.

- Compile against the deployment's pristine backup while a script mod stays
  installed; no undeploy is needed between iterations.
- Development baseline for the Gothic 1 Remake no-code modding GUI.
- Accept the exact 2026-08-27/28 game generation (Steam BuildID 24878692) in
  project creation and NPC authoring, and keep strict standalone compiler
  checks offline with native file/line/column/severity diagnostics.
- Windows installer and in-app update infrastructure are under development.
