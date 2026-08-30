# Bundling & deploying

A **bundle** combines every deployable domain — item overrides, localized text,
audio, voice archives, textures/assets, loose or packed files, scripts, and
low-level dialog-topic registration adapters — into one mod that deploys and
undeploys as a unit. This is the same engine
[Mod Studio](../../apps/mod-studio/README.md) drives.

## The build spec

Write a `spec.json`:

```json
{
  "meta": { "name": "MyMod", "version": "1.0.0", "author": "you" },
  "overrides": [ { "class": "ItFo_Apple", "field": "m_Value", "value_int": 500 } ],
  "loc_edits": { "ch1_bringlist_entry_3": { "german": "…", "german_new": "…" } },
  "audio":   [ { "bank": "SFX.bank", "sample": "Foo", "wav_path": "foo.wav" } ],
  "voice":   [ { "archive": "german_new.zip", "op": "replace", "archive_path": "NPC/Hero/DIA_Foo.ogg", "ogg_path": "DIA_Foo.ogg" } ],
  "texture": [ { "asset": "/Game/UI/.../T_Foo", "image_path": "foo.png" } ],
  "files":   [ { "game_path": "G1R/Content/Splash/Splash.bmp", "source_path": "Splash.bmp" } ],
  "pak_files": [ { "game_path": "G1R/Content/Slate/Cursors/Normal/Normal.PNG", "source_path": "Normal.PNG" } ],
  "scripts": [ { "op": "add", "module_name": "MyModule", "mini_cache": "MyModule.cache" } ]
}
```

**Asset paths are resolved relative to the spec file's own directory.**
`wav_path`, `ogg_path`, `image_path`, `mini_cache` and `source_path` may be
written as bare filenames beside the spec, whatever directory you run `gore mod
build` from; absolute paths are used as written. This is the same rule
[`gore audio replace --map`](audio.md) uses for its WAVs.

**`audio.bank` is a bare file name, never a path.** It names one bank in the
install's `G1R\Content\FMOD\Desktop` — `SFX.bank`, `Music.bank` — and deploy
resolves it against that directory itself. `gore mod build` refuses anything
else and names the rule, so an absolute path fails while you are still building
instead of surviving to a deploy that was always going to reject it.
`voice.archive` has the same shape for `G1R\Story\VoiceOver` — see
[below](#voice-packaging-details).

**`loc_edits` keys are language keys, and German has two of them.** The cache
carries `german` (the original 1998 text) and `german_new` (the remake's
rewrite), plus three English generations; where an id has both German keys the
game displays `german_new` — observed once, on BuildID 24539464. That is why
the example above writes both. An edit whose key a given id does not carry is
**still not written, and `gore mod deploy` now names it**: the bundle deploys,
the `.lcache` is rewritten, the `*.gore-bak` backup is taken, the line in game is
unchanged, and the command prints the id and language it could not write. Unlike
`audio.bank`, `build` cannot refuse it up front, because whether a key fits an id
is a property of the install's cache rather than of the spec — so the report at
deploy time is where you find out. Check the id in a `gore loc export` and see
[which language key to write](text-and-dialogs.md#which-language-key-to-write).

**`overrides` class and field names are checked only if you ask.** Pass
`--model model.json` and `build` rejects unknown classes, unknown fields and
type mismatches before writing anything — the same check `gore gen --model`
runs, through the same code. Without it the names go unchecked and the build
says so on stderr. Nothing in the release zip is a model; building one is
covered in [Catalogs & models](catalogs-and-models.md). An unchecked typo costs
you a play session: the bundle builds, deploys, and its Lua polls once a second
for 120 attempts before writing one "gave up" line to `UE4SS.log`.

```powershell
gore mod build --spec spec.json -o build --model model.json
```

Every section is optional; `delay_ms` may be set alongside `overrides` to defer
the CDO patch. Each section maps to the domain guide of the same name:
[items](items.md), [text](text-and-dialogs.md), [audio](audio.md),
[voice](voice.md), [textures](textures.md), [scripts](scripts.md).

## Bundle format and reader contract

The build spec is authoring input. The built bundle's root `gore-mod.json` has
a separate, closed `format` contract that tells a consumer which component set
it must understand:

| `gore-mod.json` format | Valid component shape | When the current writer uses it |
|---|---|---|
| `1` | every current component except `pak_file_patch` | the spec has no `pak_files` entries |
| `2` | the format-1 set plus at least one `pak_file_patch` | the spec has one or more `pak_files` entries |

Formats 1 and 2 are both current. A newer writer does not upgrade an ordinary
format-1 bundle merely because it knows format 2; it writes format 2 if and
only if the bundle actually carries `pak_file_patch`.

Direct deploy and Mod Manager import validate this relationship before component
paths or payload contracts are interpreted or anything is activated. They reject
an unknown format, format 1 containing `pak_file_patch`, and format 2 without it.
A rejected bundle is not migrated, downgraded, stripped of the unknown
component, or retried as a foreign mod. Use a consumer that supports the
declared format, or rebuild from the source spec; do not hand-edit the format
number. The version inside `voice/manifest.json` is an independent
voice-payload contract.

## Validate a built bundle before installing it

`inspect` is the canonical read-only check for a built GORE bundle. It accepts
the bundle directory or a ZIP containing one supported bundle root, performs
the same bounded materialization and component parsing as Manager import, then
opens every declared payload. WAV, image, AngelScript mini-cache, and Ogg
payloads use their existing offline structural validators. Nothing is imported,
no game path is resolved, and the selected source is not changed.

```powershell
gore mod inspect build\MyMod
gore mod inspect MyMod.zip --json
```

The bounded report includes root mod metadata, manifest format, component type,
path, target count and footprint coverage, file/byte totals, the exact
`gore-mod.json` SHA-256, and a SHA-256 over the complete normalized bundle tree.
It deliberately does not print unbounded target lists. A directory and a ZIP
containing the same bundle tree produce the same tree hash.

This proves the selected package is internally readable and structurally valid
under the current offline GORE contracts. It does **not** prove that its named
assets, samples, localization ids, or script edit targets exist in a particular
installation; that other enabled mods do not conflict; or that the game will
display, play, or execute the intended effect. Those limits are also carried in
the JSON report's `evidence.not_verified` field.

## Build, deploy, undeploy

```powershell
gore mod build    --spec spec.json -o build      # → build\MyMod\ (manifest + payloads)
gore mod inspect  build\MyMod                     # validate offline; no install or consent
gore mod deploy   --bundle build\MyMod --game "$GAME"
gore mod undeploy --game "$GAME"                 # restore everything
```

What deploy does per domain:

| Section | Deployment |
|---|---|
| `overrides` | a generated UE4SS Lua mod into `ue4ss\Mods\` |
| `loc_edits` | in-place `.lcache` rewrite, original backed up to `*.gore-bak` |
| `audio` | in-place bank rewrite, original backed up to `*.gore-bak` |
| `voice` | transactional ZIP rewrite under `G1R\Story\VoiceOver` |
| `texture` | cooks + packs a Zen triplet into `~mods\` (additive) |
| `files` | in-place replacement of a loose game file, original backed up to `*.gore-bak` |
| `pak_files` | packs the same files into an override `.pak` in `~mods\` (additive) |
| `scripts` | splices the mini-caches into the script cache, backed up to `*.gore-bak` |
| `dialog_topics` | low-level legacy UE4SS topic-registration adapter |

`gore mod undeploy` restores every backup and removes every additive container.

### What is proven, and by what

The offline half is routine and re-checked on every test run against a temporary
game root: a deploy writes the files it names, and an undeploy restores the
backups and deletes the containers it owns. That boundary was first checked by
hand on 2026-08-07 — Gothic 1 Remake at Steam BuildID 24539464, `gore` built
from commit `90940340`, with each mod's effect picked so it could not be
misread, and a person looking or listening. A later Manager campaign added
genuine third-party, numeric-container-order, and live composed-script evidence
described [below](#running-several-mods-at-once).

One bundle carrying `overrides`, `loc_edits` and a `texture` deployed as a unit
and took effect in a single launch. The main-menu logo was magenta, the edited
menu string was on screen, and the override's line was in `UE4SS.log`. That is
three unrelated deploy mechanisms landing together — an additive Zen triplet in
`~mods\`, an in-place `.lcache` rewrite with its `*.gore-bak` beside it, and a
generated UE4SS Lua mod folder — from one spec and one `gore mod deploy`.

Undeploy came back **byte-exact** for the in-place mechanisms, which is more
than "the backup was restored". Over the day's runs the `.lcache` returned to
exactly 37,093,440 bytes and the 915 MB `german_new.zip` to exactly
915,670,575 — the sizes they had before any of it started.

Undeploy also left nothing behind *between* runs, and that is the most useful
single thing the session established. Seven bundles were deployed and undeployed
in sequence on that install, several of them overriding the same class. The
seventh one's line in `UE4SS.log` read:

```
[GoreVerifyCombo] ItFo_Apple.m_Value 4 -> 12345
```

The `4` is the value read off the CDO before the write: the vanilla one, not the
`99999` or the `1000` that earlier bundles in the same sequence had set. Nothing
carried over.
Every other observation from that day rests on it: in a sequence of deploys and
undeploys where residue survived, no result would mean anything, because each
run would be reading what the previous ones left rather than the mod under test.
Each run also carried a control whose effect was already established, so a run
that showed nothing could be told apart from a tool that did nothing.

Read it for exactly what it is. One person, one install, one build, one sitting,
a screen and a log file: more evidence than existed before, and not a test
suite. Nothing re-checks it, and nothing in this toolkit ever observes the
screen — a deploy that reports success still says only that the bytes are in
place.

A later Diego campaign on BuildID `24878692` separately crossed the live
boundary for current same-module dialog bundles: a new native root, a new
direct sub-topic, persisted inventory and explicit knowledge/quest effects, a
new localization/voice pair, and a manual rebuild of an existing four-child
sub-menu were each observed in game. Those fixtures prove their exact
script-plus-payload combinations on that build; build, inspection and deploy
success alone still prove none of those runtime results. The precise scope and
remaining limits are kept on [AngelScript dialog authoring](dialog-authoring.md)
rather than generalized here.

## Loose files

Most game content lives in the IoStore containers (use `texture`) or in an
archive (use `audio` / `voice`). A few things Unreal reads through a real
filesystem path — the mouse cursor at
`G1R\Content\Slate\Cursors\Normal\Normal.PNG` and its DPI variants are the
standard example. Two sections reach those, and which one you need is a property
of the destination, not of your mod.

### Shadowed destinations

A file on disk is not necessarily the copy the engine reads. `G1R-Windows.pak`
carries its own copy of some of those same paths, and Unreal consults a mounted
pak before it falls through to the filesystem. Where both exist the packed copy
wins and rewriting the file on disk is **inert**: the bytes land, the backup is
correct, `undeploy` puts the original back, and nothing on screen changes. The
eight cursor PNGs are exactly that case — see
[the mouse cursor](textures.md#the-mouse-cursor-is-not-the-cursor-texture).

- **`files`** writes the destination on disk, in place. It reaches a path only
  the filesystem carries.
- **`pak_files`** packs your files into an additive `.pak` of your own, installs
  it in `~mods\`, and declares the virtual game path each entry claims. The
  destination need not exist as a loose file on disk — everything under
  `G1R/Config` is packed and has no loose copy at all. Offline verification proves
  the archive and deployment receipt, not which entry the game selects at runtime.

One command tells you which you are looking at:

```powershell
gore texture paklist --game "$GAME" --filter Cursors/Normal
```

A hit means a pak carries that path and `files` cannot win there. Deploy asks
the same question and refuses a shadowed `files` destination by name rather than
writing something that cannot work.

It refuses instead of quietly switching sections, because the two do not
undeploy alike: `files` records a backup and restores it, `pak_files` adds a
container and deletes it. Choosing for you would make a bundle's undeploy
contract a property of the machine that deployed it rather than of the mod.

### What each section does

- `game_path` is forward-slash and relative to the **game install root** (the
  directory that contains `G1R`), and both sections accept the same
  destinations: files under `G1R/Content` or `G1R/Config`, excluding
  `G1R/Content/Paks` (that is what `texture` and the mod manager's paks own),
  any `*.gore-bak` backup, and the four files that already have their own deploy
  mechanism — the `.lcache`, an FMOD `.bank`, the precompiled script cache, and
  a voice `.zip`. Everything else, including `G1R/Binaries`, is refused when the
  bundle is built.
- `files` is **replace-only**: the file must already exist in the install.
  Deploy refuses a `game_path` this install does not ship rather than creating
  it.
- `files` preserves the original as `<file>.gore-bak`, and `gore mod undeploy`
  restores it. If the game updates underneath a deployed bundle, the stale
  backup is dropped and the newer file becomes the pristine one, exactly as for
  the `.lcache` and the banks.
- `pak_files` never touches the file it overrides. Its whole footprint is one
  added `.pak` under `~mods\`, which `gore mod undeploy` deletes.

Two mods replacing the same loose file is a **hard** conflict in
[`gore mgr analyze`](mod-manager.md): the loser keeps nothing. An apply still
succeeds — the later mod in load order wins the whole file. Two mods claiming
one path through `pak_files` is milder: the manager keeps both additive paks,
orders their filenames by loadout position, and reports the later claimant as
the intended winner for that entry.

Whether the game honors a replaced loose file at runtime is still a per-file
question wherever no pak shadows it; the toolkit only guarantees the replacement
and its restore.

The deterministic pak filenames, filesystem changes, and deploy receipts are
offline ownership evidence only. They do **not** prove Unreal's mount priority,
that the game reads the intended entry, or any runtime/gameplay result. The
2026-08-07 session did not close that gap either. It launched the game with a
triplet of its own in `~mods\` and looked at the result, but it never put two
mods' containers on the same path, so which of those the engine picks is still
unobserved. The receipts also grant no authority for a real installation, game
launch, save access, or save mutation; each of those needs its own qualified
safety gate.

## Dialog topics

Current `gore dialog new-topic` workspaces do **not** use `dialog_topics`.
Both a same-module root and a direct sub-topic stage as one script-only
`--op edit --allow-new-symbols` mini-cache. On BuildID `24878692`, the new Diego
root was discovered and selected with no UE4SS proxy present; a prior run with
the legacy adapter installed had skipped before insertion. A sub-topic is
reached through the authored `Subdialog` call in the same module.

`dialog_topics` remains a separate low-level compatibility surface for old
workspaces and explicitly hand-authored specs. Such an entry asks the generated
UE4SS adapter to register an authored AngelScript topic at the target
conversation's natural UI boundary. It needs explicit identities: the
participant, the authored `topic_class`, and a vanilla `sentinel_class`:

```json
{
  "dialog_topics": [
    {
      "id": "legacy-diego-test",
      "participant_name": "oc_stt_diego",
      "topic_class": "/Script/Angelscript.ChoiceMyModDiego",
      "sentinel_class": "/Script/Angelscript.ChoiceDiegoExitGamestart"
    }
  ]
}
```

For a state-dependent choice, add `"allow_hidden": true`. A clean zero-match
after `IsVisible_Implementation` is then accepted as conditional, while
duplicates and mixed identity/class matches still fail closed. The default
remains strict: the registered topic must reach both UI proof stages.

That `allow_hidden` flag belongs to this low-level adapter schema; it is not a
`gore dialog new-topic` CLI option. Compilation and script-only packaging do
not require UE4SS. The adapter's older runtime evidence does not replace the
native current-path evidence or qualify other builds.

Full template, runtime evidence, and safe test order:
[AngelScript dialog authoring](dialog-authoring.md).

## Voice packaging details

Voice entries are packaged into a versioned format-1 `voice/manifest.json` with
bundle-relative, validated Ogg payloads.

- `archive` must be one `.zip` filename under `G1R\Story\VoiceOver`.
- `archive_path` is a forward-slash `.ogg` member path.
- `replace` requires that member's exact, case-sensitive stored path.
- `add` requires that the path does **not** exist.

`add` is archive-safe; actual playback still depends on a matching authored
line. One new Diego member and `Say` identity was audibly resolved on BuildID
`24878692`, with system-loopback correlation `0.763` to its source recording.
That is bounded evidence for that fixture, while replacements retain their
separate established path. See [Voice-over archives](voice.md) for the exact
member and proof boundary.

Direct deploy and manager apply group edits into one verified rewrite per ZIP
and always rebuild from the pristine or prior-backup archive. A referenced
archive missing from the install is a hard preflight error: deployment refuses
to create a partial voice patch. All manifests, payload paths, files, and Oggs
are validated before an active loadout is transactionally replaced.

### Disk space

Each candidate ZIP is written and verified beside the archive it replaces before
anything is published, so the game volume needs temporary free space comparable
to the archives being rewritten. Running out of space or memory fails before a
live archive is changed.

## Running several mods at once

`gore mod deploy` deploys **one** bundle. For a library of mods with load order
and conflict detection, use [`gore mgr`](mod-manager.md) or the
[Mod Manager](../../apps/mod-manager/README.md) app, which consume the same
bundles.

That side now has two manual evidence points on the same maintainer's install.
The 2026-08-07 run checked a soft localization clash in both directions. The
2026-08-18 run used genuine Nexus mods #244, #512, #269, and Attack Input V4;
numeric #244/#512 container priority matched the displayed order in both
directions, a new game and an existing save loaded, and the tested
enable/disable/reorder/Reset paths behaved as expected. A GORE-authored Viper
AngelScript fixture also composed and rendered with `ARMED`, `CHOICE_PASS`, and
`RENDER_PASS` plus `exact_count=1`.

The details and restoration boundary are in
[the manager's evidence boundary](mod-manager.md#evidence-boundary). The script
probe used the PR #91-fixed app-local Core DLL and was not a third-party
AngelScript mod; no three-way script conflict was checked. #269 was disabled
for that probe after a separate off-game-thread crash in its own UE4SS Lua loop.

## Other helpers

```powershell
gore scaffold MyMod -o "$GAME\...\Mods"   # empty hand-written gore-lua mod skeleton
gore deploy-shared --game "$GAME"         # install the gore-lua helpers (for custom Lua mods)
gore package mod_dir/ -o MyMod.zip        # zip a Lua mod for sharing
```

`deploy-shared` takes an optional `--src` for unusual layouts; by default it
locates the shared tree relative to the `gore` executable, independent of the
working directory.
