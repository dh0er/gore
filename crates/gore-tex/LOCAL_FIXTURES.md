# gore-tex local texture fixture

A small cooked UE5 texture can be used as a **local, fast-iteration fixture**
for the `gore-tex` decode tests, so they do not have to crack open the
multi-gigabyte game container on every run.

## Important: the fixture files are never committed

The `.uasset`, `.uexp`, `.ubulk`, and `.usmap` files under
`work/tex-fixtures/` are game-derived, copyrighted data extracted from a local
Gothic 1 Remake installation. The entire `work/` tree is gitignored and must
never be force-added. The fixture files are regenerate-on-demand artifacts,
not source.

## Chosen asset

| Field | Value |
|---|---|
| Asset path | `/Game/UI/Textures/Common/T_HardwareCursor` |
| Class | `Texture2D` (UI hardware-cursor texture) |
| SizeX × SizeY | **128 × 128** |
| Pixel format | **PF_DXT5** (BC3) |

`SizeX`, `SizeY`, and the format were read from the cooked
`FTexturePlatformData` header at the start of `sample.uexp`.

## Local files

| File | Expected | Size (bytes) | Notes |
|---|:---:|---:|---|
| `sample.uasset` | yes | 748 | cooked zen-to-legacy `.uasset` header |
| `sample.uexp` | yes | 16,529 | exports plus inline mip data |
| `sample.ubulk` | no | — | the single mip is inline in `.uexp` |
| `mappings.usmap` | yes | 2,516,955 | generated UE mappings copied from the game |

## Regenerate the fixture locally

1. From the repository root, unpack
   `/Game/UI/Textures/Common/T_HardwareCursor` from a legally installed game
   with `gore_tex::container::unpack_asset` into a temporary directory.

   ```rust
   use std::path::PathBuf;
   use gore_tex::{container, paths};

   let game = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
   let utoc = paths::main_container(&game).unwrap();
   let usmap = paths::usmap(&game).unwrap();
   let out = std::env::temp_dir().join("gore-tex-fixture");
   let uasset = container::unpack_asset(
       &utoc,
       &usmap,
       "/Game/UI/Textures/Common/T_HardwareCursor",
       &out,
   )
   .unwrap();
   println!("unpacked: {}", uasset.display());
   ```

2. Copy the unpacked files into the ignored fixture directory, adjusting the
   game path when necessary:

   ```sh
   mkdir -p work/tex-fixtures
   cp "<temp>/T_HardwareCursor.uasset" work/tex-fixtures/sample.uasset
   cp "<temp>/T_HardwareCursor.uexp" work/tex-fixtures/sample.uexp
   cp "<game>/G1R/Binaries/Win64/ue4ss/<mappings>.usmap" \
      work/tex-fixtures/mappings.usmap
   ```

3. Confirm that Git ignores every file:

   ```sh
   git status --porcelain --ignored work/tex-fixtures/
   ```

The tests skip this optional fixture when the local files are absent.
