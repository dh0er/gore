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

## What these commands reach

`gore texture` reaches cooked `Texture2D` assets **inside the IoStore
container**, and nothing else. Everything the engine opens through a real
filesystem path is a second world, and no `replace`, `pack` or `deploy` will
ever touch it.

That second world is small enough to enumerate:

```powershell
gore texture list --game "$GAME" --filter Cursor   # is it in the container?

Get-ChildItem -Recurse -File "$GAME\G1R\Content" |
  Where-Object { $_.Extension -notin '.pak', '.ucas', '.utoc' }
```

The second command prints about thirty files on a stock install, in four groups:

| Loose file | What it is |
|---|---|
| `G1R\Content\FMOD\Desktop\*.bank` | sounds and music — [Audio](audio.md) |
| `G1R\Content\Movies\*.bk2` (with `.srt`/`.uasset` sidecars) | pre-rendered Bink movies — [Voice-over](voice.md#the-intro-movie-brings-its-own-audio) |
| `G1R\Content\Slate\Cursors\Normal\*` | the mouse cursor — see below |
| `G1R\Content\Splash\Splash.bmp` | the startup splash |

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

One thing is **not settled**, and it decides whether editing those files works
at all: the same eight names are also in `G1R-Windows.pak`'s index, at the same
byte sizes. If the loader goes through the pak filesystem, the packed copy
shadows your edit; if it opens a raw OS path, the loose copy wins. That cannot
be answered offline. Edit the loose PNGs to something unmistakable and launch
once. If the cursor does not change, the packed copy is live and the change has
to ship as a loose-file `.pak` in `~mods\` — a different pipeline from this page
either way.

## What is proven, and by what

Worth being exact about, because the words carry weight the moment something
does not work. What the tests prove on every run is *structural*: a written
triplet is a well-formed container, and an asset read back out of it through
retoc is byte-identical to what went in, down to the pixels. What a human has
seen — once each, recorded in a commit message and nowhere else — is that such
a container mounts and renders in game: first an uncompressed one, and after the
compressed-writer fix a fully compressed multi-block one too. There is no
screenshot and no test behind those two sentences. A *deployed* triplet is
verified by SHA-256 and by nothing else: `deploy` records a hash per file and
confirms the bytes arrived. Nothing in this toolkit ever observes the screen, so
a successful deploy means the file is in place — never that anything changed.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--game <PATH>` | all | Install dir containing `G1R\Content\Paks\…`. Falls back to the configured path. |
| `--filter <TEXT>` | `list` | Keep only asset paths containing this substring. |
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
