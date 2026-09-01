# Textures

Replace any `Texture2D` packed in the game's UE5 IoStore container. The output
is an **additive** Zen triplet (`.utoc`/`.ucas`/`.pak`) dropped into the game's
`~mods\` folder — no original game file is ever modified.

Not everything you see on screen is one of those assets, and these commands
cannot tell you when it is not. Start with
[what these commands reach](#what-these-commands-reach).

## Find an asset

```powershell
gore texture list --game "$GAME"                        # every Texture2D
gore texture list --game "$GAME" --filter T_Hardware    # substring filter on the path
```

Asset paths look like `/Game/UI/Textures/Common/T_HardwareCursor`.

Not every picture the game shows is in that container. The glossary portraits,
the tutorial pictures, the writings and the loading-screen art are **loose**
PNG/JPG files under `G1R\Story\Conversation\images`, next to the localization
cache and the voice-over archives. `list` scans the container and will never
show them, however you filter — list those separately:

```powershell
gore texture story-images --game "$GAME"                            # all of them
gore texture story-images --game "$GAME" --filter Glossary/Creatures
gore texture story-images --game "$GAME" --absolute                 # full paths
```

They are ordinary image files: read them where they are, no extraction step.
Replacing one is a loose-file edit, not a container triplet, so none of the
commands below apply to them.

For a quick Diego smoke test, the armor atlas observed in the shipped game is
`/Game/Assets/Characters/Humans/Clothes/OC_Shadow/Textures/T_HM_OC_Atlas_02_Diego_D`.
Try an exact `extract` first. A successful extraction proves the asset identity
and avoids a slow full-container listing; use `list --filter Atlas_02_Diego`
only if the exact path is absent on the installed game version.

Resolving asset paths needs an asset→package-id index. It is built and cached
automatically, but you can (re)build it explicitly:

```powershell
gore texture index --game "$GAME"                  # cache into the shared dir
gore texture index --game "$GAME" -o my-index.json # or somewhere else
```

The default output is an immutable, generation-specific shared cache, so a game
update gets its own index rather than silently reusing a stale one.

## Extract, edit, replace

```powershell
gore texture extract --game "$GAME" /Game/UI/Textures/Common/T_HardwareCursor -o cur.png
# edit cur.png
gore texture replace --game "$GAME" /Game/UI/Textures/Common/T_HardwareCursor `
                     --image new.png --mod-dir moddir
```

- `extract` writes the texture's **top mip** as PNG.
- `replace` accepts RGBA8 or RGB8 PNG. The dimensions do **not** need to match
  the original.
- `replace` writes rewritten cooked files below `<mod-dir>\G1R\Content\…`; it
  does not touch the game.

Repeat `replace` with the same `--mod-dir` to collect several textures into one
mod.

## Pack and deploy

```powershell
gore texture pack   --game "$GAME" --mod-dir moddir --name zzz_MyMod_P -o out
gore texture deploy --game "$GAME" --triplet-dir out --name zzz_MyMod_P
gore texture undeploy --game "$GAME" --name zzz_MyMod_P
```

- `--name` is the triplet stem, e.g. `zzz_MyMod_P`. The `zzz_` prefix and the
  `_P` suffix are the usual UE convention for making a mod container mount last.
- `pack` needs `--game` for the global script-objects store.
- `deploy` copies `<name>.{utoc,ucas,pak}` into the game's `~mods\` folder;
  `undeploy` removes them again.

### Compression

`pack --compress` Oodle-compresses the `.ucas` blocks. It is **opt-in and off by
default**: the uncompressed path is the one with the most in-game mileage, so it
stays the default even though a fully compressed multi-block container has since
been seen to mount and render (see
[what is proven, and by what](#what-is-proven-and-by-what)). The compressed path
follows the base game's own writer conventions and uses a pure-Rust Oodle
implementation — no proprietary `oo2core` DLL is required.

`--compress` is reachable only from `gore texture pack`. A texture shipped inside
a [bundle](bundles.md) is packed uncompressed, whatever its size, so an in-game
sighting by way of `gore mod deploy` says nothing about the compressed writer.

## What these commands reach

`gore texture` reaches cooked `Texture2D` assets **inside the IoStore
container**, and nothing else. Everything the engine opens through a real
filesystem path is a second world, and no `replace`, `pack` or `deploy` will
ever touch it.

That second world is small enough to enumerate:

```powershell
gore texture list    --game "$GAME" --filter Cursor   # is it in the container?
gore texture paklist --game "$GAME" --filter Cursor   # is it also inside a pak?

Get-ChildItem -Recurse -File "$GAME\G1R\Content" |
  Where-Object { $_.Extension -notin '.pak', '.ucas', '.utoc' }
```

The `Get-ChildItem` line prints about thirty files on a stock install, in four
groups:

| Loose file | What it is |
|---|---|
| `G1R\Content\FMOD\Desktop\*.bank` | sounds and music — [Audio](audio.md) |
| `G1R\Content\Movies\*.bk2` (with `.srt`/`.uasset` sidecars) | pre-rendered Bink movies — [Voice-over](voice.md#the-intro-movie-brings-its-own-audio) |
| `G1R\Content\Slate\Cursors\Normal\*` | the mouse cursor — see below |
| `G1R\Content\Splash\Splash.bmp` | the startup splash |

Loose is not the same as reachable on disk. Checking every loose file in the
install against the file indexes of all six shipped paks yields exactly eight
collisions, and they are the eight cursors. The FMOD banks, the Bink movies with
their `.srt` subtitles, and `Splash.bmp` appear in no pak at all, so those three
groups are single-copy and a bundle's [`files` section](bundles.md#loose-files)
replaces them in place. The cursors are the exception, and they need
[`pak_files`](bundles.md#shadowed-destinations).

`G1R\Config` is the trap that never shows up as a collision: there is no loose
`Config` directory anywhere in the install. Every `.ini` the game reads,
`DefaultEngine.ini` included, exists **only** inside `G1R-Windows.pak`. Editing
one is a `pak_files` job by construction — `files` is replace-only, and here
there is nothing on disk to replace.

Spoken dialog is loose too, just not under `Content`: it lives in language ZIPs
under `G1R\Story\VoiceOver` ([Voice-over](voice.md)).

So the first question to ask about anything on screen is *which of the two
worlds does it live in*. Ask it early, because nothing downstream will answer
it for you: against a cooked asset the engine never samples, `replace`, `pack`
and `deploy` all succeed, the container is well formed, the deploy verifies —
and the screen does not change.

### The mouse cursor is not the cursor texture

`/Game/UI/Textures/Common/T_HardwareCursor` is the trap this rule was written
for. `list` finds it (it is the container's only cursor texture), `extract` and
`replace` work on it, the triplet packs and deploys, and the pointer does not
change.

It is imported source art that came along into the cook. It is 128×128
`PF_DXT5`, while the real UI brushes beside it in the same folder are
uncompressed — `T_Arrow` and `T_Checkbox_Checked` are both 64×64
`PF_B8G8R8A8`. And no Unreal API turns a cooked `Texture2D` into an OS cursor:
the hardware path (`UWidgetBlueprintLibrary::SetHardwareCursor`, present in the
shipping binary) takes a *Content-relative file path*, and the software path
takes a *widget class*. Neither consumes a texture.

What the player sees is a file-based hardware cursor: eight loose PNGs under
`G1R\Content\Slate\Cursors\Normal\`, selected by display scale.

| File | Size |
|---|---|
| `Normal.PNG` | 32×32 |
| `Normal@1.1x.png` | 36×36 |
| `Normal@1.25x.png` | 40×40 |
| `Normal@1.33x.png` | 44×44 |
| `Normal@1.50x.png` | 44×44 |
| `Normal@1.66x.png` | 50×50 |
| `Normal@1.75x.png` | 54×54 |
| `Normal@2x.png` | 60×60 |

Replace all eight — you cannot know which one a given player's display picks —
keep each file's dimensions, and keep the artwork cropped the way the shipped
set is, with the pointer tip on pixel (0, 0). The suffix ladder is UE's own DPI
convention; the shipping binary carries no `@2x`-style literal that would say
which file a given scale resolves to, which is one more reason to change all of
them. The `HardwareCursors=` line that names the path lives in a packed
`DefaultEngine.ini` that has not been read, so the hotspot is inferred from how
the art is cropped rather than quoted from the config.

Editing those eight files on disk, however, does nothing — and that part is
settled. The same eight names are also entries in `G1R-Windows.pak`, at the same
uncompressed sizes, and no offline reading could say which copy the loader
opens. One launch decided it: all eight loose PNGs were replaced and the pointer
did not change, while a cooked texture replaced by the same bundle in the same
launch was plainly visible. The packed copy is the live one. A new cursor has to
ship as a pak that overrides those eight entries, which is a bundle's
[`pak_files` section](bundles.md#shadowed-destinations). A second launch settled
that half too: all eight were shipped as an override pak and the pointer was
magenta. An override pak beats the base pak.

## What is proven, and by what

Worth being exact about, because the words carry weight the moment something
does not work. What the tests prove on every run is *structural*: a written
triplet is a well-formed container, and an asset read back out of it through
retoc is byte-identical to what went in, down to the pixels. Nothing in the test
suite has ever looked at a screen.

That a container also mounts and renders in game is something a human has seen.
The first two sightings were recorded in commit messages and nowhere else: an
uncompressed container, and after the compressed-writer fix a fully compressed
multi-block one. A verification pass on 2026-08-07 added four more observations,
all on Steam BuildID 24539464 with a `gore` binary built from commit `90940340`.
That is a *newer* build than the one the first two sightings were made on
(24340829): the game updated in between, so the pipeline is now known to have
survived a game update rather than only to have worked twice on one build. One person, one install, one
build, one sitting, and no screenshots.

- **The logo replacement reproduced.** `/Game/UI/Textures/Common/T_LogoRemake`
  — 512×180 `PF_DXT5` — was replaced with a solid magenta field carrying a black
  diagonal cross, shipped as a bundle and deployed with `gore mod deploy` as
  `zzz_GoreVerifyTexLogos_c5110c96_0_tex_P.{pak,ucas,utoc}`. The magenta field
  was on the main menu. So the claim that `~mods\` mounts on this build no longer
  rests on one observation: it has been seen twice, the second time with code
  newer than the first sighting's.
- **`T_Logo` is not drawn on the main menu.**
  `/Game/UI/Textures/Common/T_Logo` — 400×128 `PF_BC7` — was replaced in the
  *same* triplet and the same launch, and the magenta was nowhere on the main
  menu. Read that for exactly what it is: one person looked at that one screen
  for a short time and did not see it. It is evidence that the shipped main menu
  does not sample this asset; it is not proof the asset is dead everywhere in the
  game, which nobody has checked. It is worth recording because it is precisely
  the failure this page warns about — a flawless deploy that changes nothing
  because the asset is not the one in use — and what made it diagnosable rather
  than mysterious was the known-good logo riding in the same container.
- **A large multi-block container mounts and renders.**
  `/Game/UI/Textures/MainMenus/T_TempBackground_V2` — 3840×2160 `PF_BC7` — went
  out as its own separate bundle, deliberately not combined with the logos, so a
  malformed large container could not take the known-good control down with it.
  The deployed `.ucas` was 8,294,998 bytes, and in game the entire main-menu
  backdrop was the magenta field. Note what that does and does not extend.
  Bundles pack uncompressed, so this widens the *uncompressed* writer to 4K
  `PF_BC7` and to a container of that size. The compressed writer's in-game
  record is still the single unnamed sighting above; nothing in this pass was
  packed with `--compress`.
- **Undeploy was confirmed on screen.** In the launch that showed the magenta
  backdrop the logo was back to its shipped art, the logo triplet having been
  undeployed between runs. That is a stronger check than the file-level one this
  toolkit can otherwise offer — the game itself showed the removal — though it
  is again one look at one screen.

- **An override pak beats the base pak.** The
  [`pak_files`](bundles.md#shadowed-destinations) route was built during the first
  pass and deployed in a second one: the eight cursor PNGs went out as an override
  pak of 2,312 bytes into `~mods\`, and the pointer was magenta where replacing
  the same eight files loose had changed nothing. That settles a mechanism rather
  than a cursor, and the part that reaches furthest is not the cursor at all —
  `G1R\Config` has no loose copy anywhere, so every `.ini` the game reads,
  `DefaultEngine.ini` included, exists only inside a pak. This is the only route
  to any of them.

One corner the pass did not reach: nothing about texture replacement has been
checked on any build other than these two, 24340829 and 24539464.

A *deployed* triplet is verified by SHA-256 and by nothing else: `deploy` records
a hash per file and confirms the bytes arrived. Nothing in this toolkit ever
observes the screen, so a successful deploy means the file is in place — never
that anything changed.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--game <PATH>` | all | Install dir containing `G1R\Content\Paks\…`. Falls back to the configured path. |
| `--filter <TEXT>` | `list`, `paklist` | Keep only paths containing this substring. |
| `-o, --out <PATH>` | `extract`, `pack`, `index` | Output PNG, triplet output dir, or index path. |
| `--image <PNG>` | `replace` | Replacement PNG (RGBA8/RGB8). |
| `--mod-dir <DIR>` | `replace`, `pack` | Cooked-file staging dir laid out under its mount path. |
| `--name <NAME>` | `pack`, `deploy`, `undeploy` | Triplet base name, e.g. `zzz_MyMod_P`. |
| `--triplet-dir <DIR>` | `deploy` | Directory holding `<name>.{utoc,ucas,pak}`. |
| `--compress` | `pack` | Opt-in Oodle compression of `.ucas` blocks. |

## Related

- [Cooked DataAssets](dataassets.md) — the same additive Zen-triplet delivery
  for non-texture cooked packages.
- [Bundling & deploying](bundles.md) — shipping textures as part of a mod, where
  packing and deployment happen for you.
