# Write-path E2E: upscaled cursor mod deployed (2026-06-26)

Goal: drive the real `gore texture` CLI end-to-end to build an **UPSCALED** (rewritten
platform-data) texture mod and deploy it, proving the full write path beyond the
prior same-size magenta spike. The magenta spike proved a same-size cursor swap
renders; this proves a **256×256 rewritten-platform-data** texture mounts and
renders without crashing/black.

Target asset: `/Game/UI/Textures/Common/T_HardwareCursor` (mouse cursor,
originally 128×128 PF_DXT5). Reliable, always-on-screen visible proof.

Game: `D:\SteamLibrary\steamapps\common\Gothic 1 Remake`
`~mods` dir: `…\G1R\Content\Paks\~mods`

> NOTE on CLI shape: `texture replace` takes the asset as a **positional**
> `<ASSET>` argument, not `--asset`. Correct form:
> `gore texture replace --game <G> --image <PNG> --mod-dir <DIR> <ASSET>`.

All CLI calls run from PowerShell (Git-Bash mangles `/Game/...` into Windows paths)
using the freshly built `target\debug\gore.exe` (`cargo build -p gore`).

## Source image

Generated a 256×256 RGBA PNG — solid **cyan** (0,255,255) with a thick **magenta**
X — via a throwaway example `crates/gore/examples/gen_cursor256.rs`:

```
cargo run -p gore --example gen_cursor256 -- "work/spike/up/cursor256.png"
# wrote work/spike/up/cursor256.png (256x256 RGBA)
```

Verified: `work/spike/up/cursor256.png` = 256×256, bitdepth 8, colortype 6 (RGBA), 4455 B.
Deliberately obvious so it's unmistakably the mod, not the normal cursor.

## 1. Replace (upscale)

```
.\target\debug\gore.exe texture replace `
  --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake" `
  --image "work\spike\up\cursor256.png" `
  --mod-dir "work\spike\up\mod" `
  "/Game/UI/Textures/Common/T_HardwareCursor"
```

Printed:

```
wrote work\spike\up\mod\G1R/Content\UI/Textures/Common\T_HardwareCursor.uasset (256x256 PF_DXT5, was 128x128) [inline]
```

- Confirms the new dims **256×256 PF_DXT5** (was 128×128) → platform data was rewritten/upscaled.
- Elapsed ~3m56s (full container scan to unpack the source asset).
- Cooked output (cursor is fully **inline** → no `.ubulk`):
  - `…\mod\G1R\Content\UI\Textures\Common\T_HardwareCursor.uasset` — 748 B
  - `…\mod\G1R\Content\UI\Textures\Common\T_HardwareCursor.uexp` — 87681 B

## 2. Pack

```
.\target\debug\gore.exe texture pack `
  --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake" `
  --mod-dir "work\spike\up\mod" `
  --name "zzz_UpscaleCursor_P" `
  --out "work\spike\up\out"
```

Printed `wrote triplet:` + the 3 files. Sizes (all non-empty):

| file                          | bytes  |
|-------------------------------|--------|
| `zzz_UpscaleCursor_P.utoc`    | 503    |
| `zzz_UpscaleCursor_P.ucas`    | 88121  |
| `zzz_UpscaleCursor_P.pak`     | 347    |

`.ucas` ≈ 88 KB carries the upscaled inline texture payload — sizes look right.

## 3. Deploy

```
.\target\debug\gore.exe texture deploy `
  --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake" `
  --triplet-dir "work\spike\up\out" `
  --name "zzz_UpscaleCursor_P"
```

Printed:

```
deployed to ~mods; record: …\G1R/Content/Paks/~mods\zzz_UpscaleCursor_P.gore-deploy.json
launch the game to see it.
```

Deployed files in `…\G1R\Content\Paks\~mods\` (verified via dir listing):

- `zzz_UpscaleCursor_P.utoc` — 503 B
- `zzz_UpscaleCursor_P.ucas` — 88121 B
- `zzz_UpscaleCursor_P.pak` — 347 B
- `zzz_UpscaleCursor_P.gore-deploy.json` — 381 B (records the 3 triplet paths)

Sizes match the packed triplet exactly → copy is intact.

## HUMAN ACTION (finish the proof)

The mod is **DEPLOYED and left in place** for an in-game check. The agent did NOT
launch the game.

1. Launch `D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe`
   (verified to exist).
2. Look at the mouse cursor. If it is the **256×256 cyan square with a magenta X**
   (not the normal arrow/hand cursor), then the **upscaled** (rewritten
   platform-data) texture both **mounts AND renders** → upscale write-path proven
   end-to-end.
3. If the cursor is black, missing, or the game crashes on load → write path has a
   regression on rewritten/upscaled platform data; capture and report.

### Undeploy (after the check)

```
.\target\debug\gore.exe texture undeploy `
  --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake" `
  --name zzz_UpscaleCursor_P
```

Or simply delete the four `…\G1R\Content\Paks\~mods\zzz_UpscaleCursor_P.*` files
(`.utoc`, `.ucas`, `.pak`, `.gore-deploy.json`).

## Status

CLI ran clean end-to-end (replace → pack → deploy), no errors. Awaiting the single
in-game cursor check above to close the loop.
