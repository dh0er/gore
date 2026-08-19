# Changelog

All notable changes to gore-manager are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.2.0] - 2026-08-20

- Start the game from the Manager.
- Update settings: automatic checks can be turned off, and checked on demand.
  The portable build now checks too.
- New "Advanced details" setting, off by default. It holds the technical layer:
  per-component targets, how far the conflict check can be trusted, the import
  source and match reason, and the files GORE manages.
- UI improvements throughout: plain-language wording instead of engineer terms,
  one word per concept, real empty states, readable and copyable long paths,
  fewer permanent caveats, and the current UI scale in the title bar.

## [0.1.0] - 2026-08-18

- First experimental release. The real-install workflow has completed one
  acceptance campaign. Clean-Windows portable, installer, recovery, Reset, and
  uninstall acceptance remains a known limitation of this prerelease.
- Complete Manager lifecycle: import, inspect, enable, disable, reorder, remove,
  analyze, declarative Apply, deployment status, confirmed abandoned-operation
  recovery, Studio takeover, and Reset/Undeploy.
- Bounded import for GORE bundles and supported foreign zips/folders, loose
  `_P.pak`, IoStore `.utoc`/`.ucas` pairs with optional same-stem `.pak`, UE4SS
  Lua folders, mixed packages, and raw `.lcache`, `.bank`, and
  `PrecompiledScript*.Cache` replacements. `.7z`/`.rar`, multipart or incomplete
  IoStore, unknown, unsafe, and corrupt inputs fail without a partial import.
- Stable verified source/content identity keeps moved or unchanged reimports on
  one entry, updates changed input in place, and refuses ambiguous identity.
- Full-loadout deployment for item overrides, localization, audio, voice,
  textures, cooked assets, loose files, packed files, AngelScript, and dialog
  topics, with owned backups, drift/status evidence, and fail-closed recovery.
- Conflict and footprint reporting distinguishes Exact, Partial, Advisory, and
  Opaque knowledge instead of presenting an empty result as proof of safety.
  Voice archives and loose/packed-file claims are covered; opaque UE4SS and
  undeclared script targets remain visibly qualified.
- Numeric Unreal container priorities now follow the displayed order. A
  real-install campaign with Nexus mods #244, #512, #269, and Attack Input V4
  verified both #244/#512 order directions, new-game and existing-save loading,
  enable/disable/reorder/Reset, and byte-exact restoration of the captured
  loadout with no temporary campaign residue.
- Fixed prepared-mini AngelScript StaticNames remapping. A GORE-authored Viper
  probe using the PR #91 Core DLL rendered `[Gore probe] UI fixture` and logged
  `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with `exact_count=1`. This does not
  qualify a third-party AngelScript mod or a three-way script conflict. #269 was
  disabled for that probe after its separate off-game-thread UE4SS Lua crash.
- Actionable first-run diagnosis, compact localized mutation errors, intended
  winners/load-order direction, deployment ownership details, and truthful
  recovery guidance are available in the GUI.
- Windows installer with WinSparkle update checks; the portable zip remains
  self-contained and updater-free.
- Documented that uninstalling removes the app and its normal
  `%LOCALAPPDATA%` UI preferences, while an `%APPDATA%` fallback needs manual
  cleanup; uninstall does not undeploy mods or erase the shared imported
  library, loadout, and GORE configuration.
- Included WinSparkle 0.8.1, Expat, and OpenSSL attributions in the shipped
  third-party notices.
