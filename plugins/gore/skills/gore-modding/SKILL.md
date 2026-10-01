---
name: gore-modding
description: Use when modding Gothic 1 Remake with GORE - changing textures, localized text, dialog, audio, voice-over, item values or AngelScript; authoring NPCs and quests; building a bundle; or importing, enabling, ordering, analyzing, preflighting, applying, checking status, recovering, removing, and resetting GORE or external mods with the Mod Manager. Covers the consent gate and what "deployed" does and does not prove.
---

# Modding Gothic 1 Remake with GORE

The `gore_*` MCP tools run the real `gore` CLI. This skill is the workflow around
them; the facts live in the guide, which ships with the binary and is always
current for the installed version. Read the guide rather than trusting anything
remembered.

## If the tools are not there

Missing tools can mean deferred discovery, pending startup, a disabled server
or a failed start. Search for exact GORE tool names before diagnosing a failure.
Check `Get-Command gore` and `gore --version` in the client's environment; missing
MCP tools alone do not prove the CLI is absent. For Codex, `codex plugin list`
and `codex mcp get gore --json` establish installation and registration, not
connection health. Use the client's MCP status or startup diagnostic to tell
pending startup from an error.

Codex may omit pending optional servers after its default one-second startup
grace. The current plugin marks GORE as required with a 30-second startup
timeout, so it waits for discovery or reports a startup failure. Update an older
plugin through its registered marketplace; see [plugin repair](../../README.md#updating-and-repairing-the-plugin).
Do not pin another executable or diagnose PATH without evidence.

When the current CLI is reachable, offline authoring can continue through that
same CLI and its shipped guide. Run `gore doctor` first and preserve the same
base guards and consent boundaries. Explain a missing MCP connection separately;
it does not establish a missing modding capability.

## Before you touch anything

For a new authoring session, run `gore_doctor` once before creating workspaces,
exporting script trees or compiling. Read the check verdicts: a successful tool
call only means the report was produced. For AngelScript, NPC, dialog or quest
work, resolve a `standalone_compiler` problem before expensive authoring steps.
Read the `native_api: ready` or `native_api: missing` item too: compiler
compatibility alone does not admit extended native references during composition.
An `ok` verdict establishes compiler/cache/API readiness; it does not prove that
the requested source compiles or its gameplay works. Keep native diagnostics
from a failed compile, including failures in unchanged shipped source.
Use the reported executable and pristine cache path; after replacing the CLI,
restart the MCP server and repeat Doctor once. Do not downgrade to an older CLI
or export another full tree to work around a missing embedded compiler catalog.

Call `gore_guide` for the page that covers your domain: `textures`, `audio`,
`voice`, `text-and-dialogs`, `dialog-trees`, `dialog-authoring`, `items`,
`npc-authoring`, `scripts`, `bundles`, `mod-manager`. Use
`action: "search"` once without `page` — it ranks single sections globally
across the guide and reference. Use a short mechanism query such as
`quest callbacks`, rather than the whole story, and read the returned section. If a
section lookup fails, use its returned section names or a focused search;
do not expand to the entire HTML guide. If the page and section are already
known, use `read` directly.
`gore_help` gives exact current flags; the guide gives the order to do things in
and what breaks when a step is skipped.

Do not preload every guide page or ask for help for every command in a planned
workflow. Read one ranked section for the step in front of you, and call
`gore_help` only when the tool schema leaves an argument unclear. `gore_help`
takes one CLI command path: use `{"command":"loc export"}`, never a `gore_loc`
tool name and never a separate `subcommand` field. Use the game path Doctor reports; call `config get` only when that path is
missing or contradictory. Prove each chosen target, build once,
inspect once, and leave Manager help/import/preflight until the user actually
chooses installation.

When an MCP client defers or hides tools, match only the needed tool names;
a description search for `GORE` matches almost the entire registry. Return names
first and load only the next tool's schema. For known NPCs, prefer filtered
`gore_npc list` with a small `max`, then one exact `show`, over broad `gore_find`
results. Use filtered dialog `list`, a shallow `tree`, then exact topic `show`.
Batch independent small reads; narrow a truncated result rather than raising
its output budget or searching every installed CLI version.

## Build new mods from supported capabilities

For NPC identity, appearance, routines and placement, start with `npc-authoring`
and `gore_npc`. Mod Studio's GUI coverage does not limit CLI or MCP authoring.
Check current commands and evidence before claiming a build or authoring limit.

Runtime fixtures demonstrate building blocks, not an allowlist of permitted
mods. Combine supported NPC, appearance, dialog, quest and reward mechanisms
for the user's request. Do not refuse because the exact character, clothing,
head, dialogue or quest combination has not appeared in a previous test.
Check the actual assets, APIs and documented technical constraints, then build
and validate the new mod. Report what its own game test still needs to check
without turning missing prior test coverage into an unsupported capability.

Discover mesh paths from shipped/native visual references or existing asset
metadata and confirm exact paths; do not guess names such as `XardasHead`.
`texture paklist` is not a generic IoStore mesh index. Use documented CLI readers;
report a discovery gap instead of writing an ad hoc IoStore parser.

For head swaps or newly introduced native mesh/material references, read
`scripts` / `probe-head-apis-before-assembling-the-mod`. After Doctor passes,
compile a minimal source using the planned native types, calls and properties
through strict standalone overlays before assembling the larger NPC/dialog/quest
mod. Check selective FullGraph composition as well as the compiler result.
Native API qualification is reusable across asset choices and exact supported
generation rows whose Binds bytes, ancestry, class and field profiles match.
Build 25414091 reuses authenticated snapshot evidence with its original source
provenance; signatures, datatype flags and property offsets remain exact.
Recompile for the target cache GUID after an update. A native-membership refusal
on a known compatible generation needs a toolkit correction; retain the target
hash/GUID and missing identity, and do not recommend `force` as a workaround.
Keep API admission separate from the chosen head's pose, materials, restoration
and persistence observed in game.

If a documented command or NPC guide is missing, check the executable path
printed by an MCP call against `Get-Command gore` and `gore --version`. An old
plugin installation can pin an old CLI; establish the installed version before
claiming the current toolkit cannot do the work. Keep the plugin's portable
`command: "gore"` wiring; do not pin it to a version directory as a workaround.

Offline NPC/quest examples are under `examples/npc-batch-tests/` beside the
running `gore.exe`. For quests, read `quest/README.md` and `quest/quest-content.as`;
`quest/GORE_TEST_A.as` is the complete surrounding overlay. Repository links
record historical source/evidence; use the packaged files for normal authoring.
If they are missing, report the CLI path/version and incomplete package instead
of crawling HTML, searching old distributions or fetching source with `curl`.

Use Doctor's pristine cache for every related workspace and source export.
Pass it as `cache` where supported and compare each manifest's `cache_sha256`
before combining overlays. Live-cache inspection describes the installed mod;
it may have a different hash. Preserve base guards and existing workspaces.
A mismatch needs a fresh checkout or toolkit correction, never edited manifest
hashes or an automatic reset; `scripts` explains the installed-mod case.

For a new NPC, use `npc stage <workspace>` without `--tree`. It snapshots only
the checked authored modules and prints the strict standalone overlay compile
with exact base and source hash guards. Run that command intact; stage again
after editing sources. No full-tree export is needed. Read `scripts` /
`compiling-while-a-script-mod-is-installed` when selecting a base with an active
deployment. A timeout is not proof that the child process stopped: establish
its state before retrying, and never rename its destination while it runs.

## Manage a loadout as one declarative deployment

For installing or managing mods, use `gore_mgr`, not a sequence of direct
`gore_mod deploy` calls. A Manager loadout is one owned deployment; Apply
rebuilds its complete enabled state from the pristine base.

Read `gore_guide` page `mod-manager` and the exact `gore_help` entry first. The
guide is the authority for the current accepted GORE bundles, external folders,
archives, loose files, containers, UE4SS mods, and mixed packages. Do not infer
support for a format from its extension or from where the mod came from.

Use this lifecycle:

1. If the setup is unknown or already looks wrong, run `gore_doctor` once; do
   not repeat it between ordinary Manager steps. `gore_mgr import` the package
   and keep the returned entry id. Import writes
   protected Manager library/loadout state, not the game installation; it still
   needs consent because a verified re-import may replace the stored payload.
2. `gore_mgr enable` the intended entries, use `order` when needed, then run
   `analyze`. Position 0 goes first and loses recognized ordered conflicts. An
   intended winner is evidence from the analyzer, not proof of runtime priority.
   `enable`, `disable`, and `order` update the reversible target loadout
   immediately and intentionally do not open the protected-write consent gate;
   state the intended edit before the call and report the resulting order.
3. Immediately before an installation change, run the dedicated
   `gore_mgr_preflight` tool for the exact read-only readiness/recovery report.
   Pass its typed arguments directly; it exists separately so MCP clients see a
   truthful read-only annotation instead of `gore_mgr`'s Apply/Reset worst case.
   An active Studio deployment, game drift, or interrupted operation is a state
   to resolve, not something to overwrite.
4. Before `apply`, summarize the enabled loadout and observable effects, give
   the exact Apply and Reset commands, and obtain consent. Apply writes the game
   installation. Follow it with `status` and then a concrete in-game checklist.
5. `remove` deletes the library entry and its loadout slot, but does not change
   bytes already deployed in the game. Apply the remaining loadout afterwards
   to make the installation match, then check `status` again.
6. `reset` is the terminal cleanup path for a Manager-owned deployment and must
   refuse rather than remove a Studio deployment. Check `status` afterwards;
   never treat a successful command as proof that the game displayed or executed
   every component.

`list`, `analyze`, and `status` are not advertised as read-only because opening
the authoritative Manager store may reconcile its loadout; list and analyze may
also finish recovery of an interrupted library replacement. They remain ungated,
but their result must still be treated as authoritative refreshed state.

Never delete a GORE installation lock or recovery directory by hand. If an
operation is still active, wait. If `preflight` identifies an abandoned Manager
operation, pass its exact opaque action token as `expected_guard_id` to
`gore_mgr recover` and obtain consent for that call. Recovery atomically rechecks
the token; never guess or reuse one after the state changes. Compiler-owned,
ambiguous, or invalid recovery state gets help, not an improvised reset. Re-run
`gore_mgr_preflight` and `status` after recovery before Apply or Reset.

## Pick a target that the engine actually reads

This is where mods silently fail, and the tools cannot warn you: they will
faithfully replace an asset nothing samples.

The game's content lives in two worlds. Most of it is cooked into the IoStore
containers, which is what `gore_texture` and `gore_asset` reach. A short,
enumerable set sits loose on disk under `G1R\Content` — the FMOD banks, the Bink
movies and their subtitles, the splash bitmap, and the mouse-cursor PNGs — and
none of those is reachable through the texture commands. Read
`gore_guide{action:"read",page:"textures"}` before choosing, and check which world your target
is in. A bundle's `files` section replaces a loose file; `texture` replaces a
cooked one. Using the wrong one deploys cleanly and changes nothing.

Three specific traps the guide documents, each found by shipping a mod that
changed nothing. The mouse cursor does not come from the cooked cursor texture.
The pre-rendered intro movie plays its own embedded audio, so replacing the
intro's voice lines is correct and inaudible — for a fast audible proof use a
real in-engine conversation line instead. And a sound sample is not a sound the
game triggers: the game plays events, several samples often share one trigger,
and near-identical names belong to different surfaces. Before you tell the user
where to listen, confirm the sample you replaced is one that surface plays and
whether it is one of a numbered set — the audio guide says how.

Then read your own work back. Every write path in this toolkit can be re-read:
a replaced sample lists as replaced, and `gore_mod_inspect` validates a built
bundle and hashes its exact manifest plus complete normalized tree. Do that
before reporting success, and say which items you could not check.

## Verify existing targets and define new identities

A reference to existing game content — a sample name, an archive path, a texture asset, a
localization id — has to be proved by an exact successful read or a listing you
actually ran, not from the pattern the neighbouring names suggested. An exact
texture extraction is proof; do not run the expensive full texture listing as a
second proof. For a fast Diego visual smoke test, try the documented armor atlas
`/Game/Assets/Characters/Humans/Clothes/OC_Shadow/Textures/T_HM_OC_Atlas_02_Diego_D`
directly and list only if that exact extraction says it is absent. The naming
looks regular enough to extrapolate from and is not: one session's spec named a
Diego line that appeared in no listing, and it happened to exist. The failure
mode when it does not is `mod build` accepting the spec and `mod deploy` refusing
it afterwards, which costs you the whole build.

New NPC names, topic classes, quest IDs and localization rows are authored by
the mod; they do not have to exist in the vanilla game or an earlier fixture.
Define them through the supported authoring path and verify the generated
source or payload and its references during the normal check/build steps.

## Author dialogs through checked same-module workspaces

Read `dialog-trees` to identify the exact participant, module, topic and menu
position, then read `dialog-authoring` before changing source. Its capability
table records runtime examples; "Practical limits only" describes concrete
technical restrictions. Missing an exact example is not a restriction on new
content assembled from the supported mechanisms.

Use `gore_dialog`, not a hand-built isolated Add module:

- `list`, `tree`, `show`, `text` and `export` inspect shipped conversations or
  prepare their separate localization edit. `export` requires an absent or
  empty output directory so an old snapshot cannot leak stale JSON into a new one.
- `checkout` -> edit -> `check` -> `stage` covers existing method bodies and
  complete reconstructed defaults, including `Caption`, `PriorityRank`, `Rules`
  and flags; new same-module topics may add their own fields and helpers.
- `new-topic` scaffolds a native root or one child of a shipped `Subdialog`;
  `subdialog_position` and `priority_rank` control placement and ordering.
- `new-conversation` requires an NPC with no root and one exact already-loaded
  per-NPC settings module. It edits that anchor; there is no discovered Add
  fallback.

Before layering quest/document/content helpers, check, stage and strictly
compile the minimal checkout plus scaffolded topic against the same pristine
base. This separates baseline emission/binding failures from new content. Then
add the requested content and check/compile that assembled source.

Deeper all-new trees stay in that generated module and must pass `check`, which
guards the private-base, same-module, shipped-ABI, 20-slot and action-bearing
tree contracts. Cross-module new-symbol dependencies use one coordinated
`gore_as_compile` call with `overlays: true` and `mini`, as the script guide
describes; independent mini-caches cannot supply each other's symbols. `caption_key`
only references localization; add the row separately. Voice is a third payload:
structural validation can inspect Vorbis or Opus, but playable publication is
fail-closed to Ogg/Vorbis because live Opus was silent. New lines receive the
game's generic facial placeholder; accurate line-specific lip sync needs
separate cooked facial-animation assets that GORE cannot yet author.
Compilation, bundle inspection, deployment and runtime proof remain separate
steps.

The dialog checker admits `UDocumentSegment` as a direct parent when shipped
classes in the target cache use it. It does not cover every native parent. For a remaining coverage
finding, read the quest recipe and use the script overlay route to establish
resolution while keeping topic privacy and source-preservation guards.

## Compile AngelScript offline unless the user chooses a game fallback

For ordinary authoring, use the dedicated `gore_as_compile` or
`gore_as_compile_module` tool. They select strict standalone themselves, do not
offer a game backend, and never need game-launch consent. Strict standalone uses
the bundled compiler, does not start the game, and does not stage anything in
the installation. Use a fresh unique `work_dir` whose generated `tree` is absent
and ordinary outputs outside the game installation; that normal path needs no
consent. An occupied generated tree or an output aimed into the installation
remains protected. Standalone also returns native compiler diagnostics; the
optional runtime diagnostics hook belongs only to the game backend.

For several changed/new modules, read `scripts` / `compile-only-authored-modules`
and call `gore_as_compile` with `overlays: true`. Put only those complete module
sources (at most 256) at canonical Script-relative paths in `src`; omitted
originals remain in the pristine cache. Do not export the whole tree first. `only_changes` binds
the exact allowed set and optional source hashes; it is a scope check, not a
file filter. Preserve `expect_base` or `expect_base_sha256` guards. Overlay mode
requires strict standalone and has no deletes or game fallback. A deliberate
complete-tree workflow remains available with `overlays` omitted/false.

Use the mixed `gore_as` compile routes only when the user knowingly chose
`game` or `standalone-then-game`; those may fall back to the embedded game
compiler and legitimately ask for both game-launch and install-write consent.
If a dedicated standalone call is refused with a claim that it launches the
game, do not relay that false question: report that the installed GORE MCP
server is older than this workflow and needs updating.

If a minimal source still fails after readiness passes, retain the matching CLI
package and diagnostics. Do not try an older EXE, rewrite emitted shipped calls
or guess default parameters to silence a native binding failure.

Compiler compatibility, one-module authoring feasibility, and default-patch
qualification are three separate answers. A native diagnostic such as
`Identifier 'UTopic_…' is not a data type in global namespace` means that the
shipped AngelScript class is private to another script module; it is not a game
version mismatch and does not make the standalone compiler generally
incompatible. Do not present a new Diego topic derived from
`UTopic_Hero__OC_STT_DIEGO` as an isolated `compile-module --op add` recipe.
Use `gore_dialog new-topic` so the new class is compiled inside the existing
conversation module, or `new-conversation` inside a qualified loaded settings
anchor. Coordinated overlay compilation resolves visible cross-module
dependencies when the requested mod needs them; it does not bypass module
privacy. A game fallback or an unrelated base class does not fix that error.

## The consent gate

A call asks first when it would change the game installation, or destroy
something outside it that this server can see is there — an output file that
already exists, an output directory that already holds files, a bundle folder
about to be cleared and rebuilt. Writing into a fresh or empty scratch directory
asks nobody. Explicit strict standalone AngelScript compilation needs no
game-launch consent, but its occupied generated tree and outputs aimed into the
installation remain protected like every other write. If a question does
arrive, it is about something real; read what it names rather than approving it
reflexively.

Many clients answer that question themselves in milliseconds without showing
anybody anything, so the call comes back refused even though nobody declined.

When that happens: do not resend the call unchanged, and do not tell the user they
said no — the server cannot see who answered. Show them the command line the
refusal prints and ask in the conversation. If they agree, send the exact same
call again with the refusal's `approval_request_id` and `user_approved` set to
their own words. The words need no ritual formula: a clear instruction to do the
displayed operation is enough. The opaque id expires, works once, and is bound
to the normalized call, so changing any argument or reusing it is refused. Never
invent either field or fill the words without having asked.

## Forcing after a game update

First distinguish a genuinely unqualified generation from a toolkit failure
on a known compatible generation. Supported equivalent rows reuse authenticated
native API evidence; a missing snapshot selection or native-membership refusal
there needs a toolkit correction, even if the error prints a generic force hint.
Do not request force approval to work around that defect.

For a genuinely unqualified generation, compile, decompile, `value inspect` or
`mod build` steps may offer `--force` when the specific check permits it.
The result then names `"force": true`. Ask the user **once** whether GORE should
force such steps for this game build, and say plainly that the result may be
broken. Do not ask again for every command: if they agree, pass
`"force": true` on every later call that needs it. The server confirms the
first forced call once and remembers the answer for the session. If they
decline, stop and point them to the game-updates reference instead of retrying.

Forcing does not replace testing: the warnings the forced command prints are
the checks it skipped. Tell the user which ones appeared, and put a new-game
check of the affected change on the in-game checklist.

## Two things that will cost you a build

Asset paths in a bundle spec (`wav_path`, `ogg_path`, `image_path`,
`mini_cache`, `source_path`) resolve against the **spec file's directory**, not
the working directory. Put the assets beside the spec, or spell them absolutely.

Listings are bounded on purpose. `gore_voice list` and `gore_audio list` print a
page and say how much they left out. Narrow with `--filter`; raise `--max` only as
far as you need. Asking for everything at once rebuilds the oversized result the
bound exists to prevent — and a cut-off JSON array does not parse.

## Hand the build over before you ask to install it

The moment `mod build` succeeds, call the dedicated read-only
`gore_mod_inspect` tool on the resulting bundle directory. It takes `bundle`
directly, needs no consent, and returns bounded root metadata, components,
manifest/tree hashes, and explicit proof limits. A failed inspection means the
build is not ready to import or deploy. Do not substitute Manager import as a
validator: import changes the protected library and legitimately needs consent.

After inspection succeeds, and **before** the deploy question is put to the user
in any form, tell them four things:

1. **What the mod changes**, domain by domain — for each one, what they will see
   or hear, and where. Not the spec; what it does.
2. **Where the bundle is**, as a full path.
3. **The installation route**, spelled out. For a Manager loadout this is
   `mgr import`, `enable`, and `apply`; use direct `mod deploy` only when the user
   explicitly chose the single-bundle route and no Manager deployment owns the
   installation.
4. **The matching cleanup route**: Manager `remove` plus Apply, or Manager Reset;
   direct `mod undeploy` only for a direct deployment.

Then ask.

This is the order that keeps the decision with them. Deploying writes into a game
installation they may have hours in, and "may I deploy?" is not a question anyone
can answer without already knowing what is in the bundle and how to get rid of
it. Sessions keep getting this backwards — asking first and explaining afterwards
— which turns the summary into a report on something already done.

It also stops the build from being trapped behind you. A person who has the path
and the two commands can install it next week, from a shell, without this
conversation.

## After deploying, be honest about what is proven

`gore mod deploy` and `gore mgr apply` verify that the bytes they wrote are the
bytes they meant to write, by hash. Nothing observes the screen. A successful
deploy or Apply is not evidence that the change is visible. A marker that exists
only in unreachable code is likewise not evidence for the visible or audible
behavior the user requested.

So follow it with a checklist the user can actually run: what to look at, in what
order, and what each item would look like if it worked. Mark the items you could
not verify offline as uncertain, and say why.

If something did not show up, the first question is not "did the tool fail" but
"does the engine read that asset". Check that before changing the pipeline.
