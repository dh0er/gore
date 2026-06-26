# gore-tex local texture fixture

A small cooked UE5 texture used as a **local, fast-iteration fixture** for the
`gore-tex` decode tests (so they don't have to crack open the multi-GB game
container on every run).

## Important: the binaries are intentionally NOT committed

The `.uasset` / `.uexp` / `.ubulk` / `.usmap` files in this directory are
**game-derived, copyrighted data** extracted from a local Gothic 1 Remake
install. They are deliberately **gitignored** (see the repo `.gitignore`,
`work/tex-fixtures/*.uasset|*.uexp|*.ubulk|*.usmap`, and the whole `work/`
tree) and must **never** be committed. Only this `README.md` is tracked.

If the files are missing on your machine, **regenerate them locally** with the
steps below. They are regenerate-on-demand artifacts, not source.

## Chosen asset

| Field         | Value |
|---------------|-------|
| Asset path    | `/Game/UI/Textures/Common/T_HardwareCursor` |
| Class         | `Texture2D` (UI hardware-cursor texture) |
| SizeX × SizeY | **128 × 128** |
| Pixel format  | **PF_DXT5** (BC3) |

`SizeX`/`SizeY`/format were read straight from the cooked
`FTexturePlatformData` header at the start of `sample.uexp` (the three int32
`SizeX, SizeY, PackedData` immediately precede the length-prefixed
`"PF_DXT5"` `FString`). They can be re-confirmed once the T7 typed texture
parser lands.

## Files

| File             | Exists | Size (bytes) | Notes |
|------------------|:------:|-------------:|-------|
| `sample.uasset`  | yes    | 748          | cooked zen→legacy `.uasset` header |
| `sample.uexp`    | yes    | 16529        | exports + **inline** mip data |
| `sample.ubulk`   | **no** | –            | none: the single mip is inlined in `.uexp`, so there is no separate bulk file. This makes it a good test of the inline-mip decode path. |
| `mappings.usmap` | yes    | 2516955      | generated UE mappings (`.usmap`), copied verbatim from the game |

Total of the cooked texture files (`.uasset` + `.uexp` + `.ubulk`) = **17277
bytes** (~17 KB) — well under 1 MB, with no `.ubulk`.

## Game build

- Gothic 1 Remake on Unreal Engine **5.4.3**, CL **169686**.
- usmap source file name: `G1R-5.4.3-168781-272ce2f8.usmap`.
- Container: `G1R/Content/Paks/G1R-Windows.utoc` (IoStore / zen).

## Regenerate the fixture locally

Requires the game installed at
`D:\SteamLibrary\steamapps\common\Gothic 1 Remake` (adjust the path in the
commands if yours differs).

1. From the repo root, unpack the chosen asset from the real container into a
   temp dir. The quickest route is a tiny throwaway example (the same one used
   to originally pick this asset); equivalently, call
   `gore_tex::container::unpack_asset` from any small driver:

   ```rust
   use std::path::PathBuf;
   use gore_tex::{container, paths};

   let game = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
   let utoc = paths::main_container(&game).unwrap();
   let usmap = paths::usmap(&game).unwrap();
   let out = std::env::temp_dir().join("gore-tex-fixture");
   let uasset = container::unpack_asset(
       &utoc, &usmap,
       "/Game/UI/Textures/Common/T_HardwareCursor",
       &out,
   ).unwrap();
   println!("unpacked: {}", uasset.display());
   ```

   `unpack_asset` writes `T_HardwareCursor.uasset` and `.uexp` (no `.ubulk`
   for this asset) into the temp dir.

2. Copy the unpacked files into this directory with the fixture names, and
   copy the game's `.usmap` as `mappings.usmap`:

   ```sh
   cp "<temp>/T_HardwareCursor.uasset" work/tex-fixtures/sample.uasset
   cp "<temp>/T_HardwareCursor.uexp"   work/tex-fixtures/sample.uexp
   cp "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/G1R-5.4.3-168781-272ce2f8.usmap" \
      work/tex-fixtures/mappings.usmap
   ```

3. Confirm the binaries are gitignored (they must not show up here):

   ```sh
   git status --porcelain work/tex-fixtures/
   # -> should list nothing except README.md
   ```

To rediscover the *smallest* candidate from scratch (instead of using the
pinned asset above), scan with a narrowing filter and unpack candidates,
picking the smallest total — e.g. filter `"Cursor"` yields exactly this one
hardware-cursor texture, which is why it was chosen.
