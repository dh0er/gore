# Changelog

All notable changes to gore-cli are documented here. The release workflow
publishes the section matching the released version as the GitHub release
notes, so every release needs an entry.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Added `gore mcp serve`: a Model Context Protocol server that exposes all 77
  CLI leaf commands to AI assistants over stdio JSON-RPC, as eleven
  command-family tools plus `gore_guide` and `gore_help`. The guide is compiled
  into the binary and served both as a search tool and as `gore://guide/<page>`
  resources.
- Commands that change the game installation or rewrite a file in place require
  `--allow-write`; `as compile` and `as compile-module` additionally require
  `--allow-game-launch`.
- Added `gore mcp tools`, which prints the advertised tool definitions as JSON
  for debugging a client integration.
- Added `gore guide html`, which renders the built-in guide into one
  self-contained HTML file — every page, stylesheet and script inlined, with a
  collapsible sidebar and a filter box. The release zip now ships a rendered
  `docs\guide.html` beside the Markdown pages, because Windows has no handler
  for `.md` and the guide is too table-heavy to read in Notepad.
- Split the documentation. `docs/guide/` is now a user guide and nothing else;
  the implementation contracts it used to carry — receipt semantics, seal
  guarantees, native transaction boundaries, wire formats, error-code tables —
  moved to `docs/reference/`, and the five Mod Studio pages were consolidated
  into one. `docs/guide/building.md` became `docs/development.md`. Only the
  guide ships in the release zip and is rendered to HTML; `gore.exe` embeds both
  bodies so the MCP server can serve the reference as `gore://reference/<page>`
  when it needs to explain why a command refused something.

## [0.1.0] - 2026-06-19

- Initial release: command-line companion for gore-mod (catalog sync, loc
  export/import, UE4SS dump-mod generation).
