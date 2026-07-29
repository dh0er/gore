# Textures

Replace any `Texture2D` packed in the game's UE5 IoStore container. The output
is an **additive** Zen triplet (`.utoc`/`.ucas`/`.pak`) dropped into the game's
`~mods\` folder — no original game file is ever modified.

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
default**: uncompressed containers are the ones proven to load reliably in game.
The compressed path follows the base game's own writer conventions and uses a
pure-Rust Oodle implementation — no proprietary `oo2core` DLL is required.

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
