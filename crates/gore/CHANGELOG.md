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
- Every MCP consent refusal now shows the command line it would have run. The
  refusal already told the assistant to ask the user directly and show them
  "the command line above", but that line only ever appeared in the elicitation
  dialog — and a client that reaches a refusal at all is one that answered that
  dialog itself, without showing anybody anything.
- Every `gore as` subcommand that takes a cache, and `gore catalog --kind
  knowledge --script-cache`, now checks the `0x9e377abe` module-cache magic
  before walking the container. Pointing `decompile` at `Binds.Cache` used to
  report `resolver: unexpected end of data at pos 28: needed 1919111983 more
  bytes`, a length read out of unrelated bytes; the failure now names the
  offending file, the real format mismatch, and which file to pass instead.
  `gore as walk` no longer accepts an arbitrary blob, and `decode-header` —
  which always checked the magic but named neither the file nor a reason — goes
  through the same gate.
- `gore audio` no longer reports the game's sample-free banks as damaged.
  `Master.bank` (mixer only), `Master.strings.bank` (string table only) and the
  four 506-byte placeholder banks all carry an empty SNDH chunk, which the
  decoder rejected as `SNDH too small`. That reading is taken only when the
  top-level LIST runs to the end of the file — true of exactly the banks that
  have no `SND ` chunk behind it — so a bank still carrying its FSB5 payload
  cannot claim it by having those four size bytes zeroed in place. The command
  still exits non-zero, because there is nothing to list. A genuinely truncated
  bank is still reported as truncated, including one cut off inside a nested
  SNDH chunk header, which used to panic instead.
- Bounded `gore voice list`, which printed every central-directory entry —
  287,581 characters for the shipped `foreign.zip`, past the MCP result limit,
  where the clip landed mid-array and left JSON that no longer parsed. It now
  prints at most `--max` entries (default 100), omits directory records unless
  `--directories` is passed, and takes a `--filter` substring that folds case
  the way `--basename` does, so `--filter MÜLLER` finds `DIA_Müller_01.ogg`.
  Both output modes say what they left out: the header names how many entries
  the filter kept and how many directory records were dropped, the table ends
  with a `… [truncated: …]` line, and the JSON document gained
  `directory_count`, `matched_count`, `listed_count`, `truncation_notice` and
  two booleans — `truncated` (did `--max` stop the listing) and `complete` (is
  the array the whole archive). The truncation notice deliberately does not name
  a `--max` that would list everything: on a 33,323-entry archive that is an
  ~11 MB document and the result limit cuts it inside the array again. `--max 0`
  lists nothing and reports only the counts. Note one breaking change for anyone
  scripting against that JSON: `entry_count` used to be the length of the
  `entries` array and is now the number of entries in the archive, which is no
  longer the same number once a page is printed. The array length is
  `listed_count`.

- Added a `files` section to the bundle build spec: a bundle can now replace a
  loose game file — one Unreal reads from disk rather than from the IoStore
  containers or an archive, such as the mouse cursor at
  `G1R\Content\Slate\Cursors\Normal\Normal.PNG`. Replacement only; the original
  is preserved as `*.gore-bak` and restored by `gore mod undeploy`, and
  destinations are limited to files under `G1R/Content` or `G1R/Config` that are
  not pak containers, backups, or one of the four files that already have their
  own deploy mechanism. `gore mgr` reports two mods replacing the same loose file
  as a hard conflict and applies them later-wins.
- `gore mod build` now resolves every asset path in a spec (`wav_path`,
  `ogg_path`, `image_path`, `mini_cache`, `source_path`) relative to the spec
  file's own directory instead of the process working directory, matching
  `gore audio replace --map`. Absolute paths are unchanged. A failure names the
  resolved path plus the section and index it came from. A spec whose relative
  assets sat in the working directory rather than beside the spec has to move
  them or spell them absolutely.
- Bounded `gore audio list`, which printed one line per sample with nothing
  stopping it: 458,589 bytes over 7,219 lines for `SFX.bank`. Through the MCP
  server that was cut at the 256 KiB stdout cap, mid-line inside sample #4122,
  so the back 43 per cent of the bank was simply absent from what the caller
  then searched — and the answer came back "not found" rather than "not shown".
  It now prints at most `--max` samples (default 100), takes a case-insensitive
  `--filter`, and has a `--json` mode carrying `sample_count`, `matched_count`,
  `listed_count`, `truncated`, `complete`, `truncation_notice` and the bank's
  codec.
- The guide now says which parts of the game the texture and voice commands
  cannot reach. The mouse cursor is a file-based Unreal hardware cursor — eight
  loose PNGs under `G1R\Content\Slate\Cursors\Normal\` — so replacing the cooked
  `T_HardwareCursor` texture is inert, and the pre-rendered intro movie carries
  four embedded audio tracks, so replacing the per-line Ogg files under
  `Cutscenes/Intro/` lands correctly in the archive and stays inaudible. Both
  were found by building a mod that changed neither.

## [0.1.0] - 2026-06-19

- Initial release: command-line companion for gore-mod (catalog sync, loc
  export/import, UE4SS dump-mod generation).
