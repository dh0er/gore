# Magenta-override de-risk spike (Task 10)

**Goal:** prove the single biggest open write-path risk — does a `~mods` Zen
container actually *override* a base-container asset in **this** game (Gothic 1
Remake, UE 5.4.3, IoStore, unencrypted/unsigned)? Built a real "magenta cursor"
override mod, staged the verified triplet, and deployed it to `~mods`. The game
was **NOT launched** — a human confirms the visual result (see HUMAN ACTION).

This spike is independent of our product's encode path: it deliberately uses
**upstream** `trumank/retoc`'s `to-zen` (its own Oodle backend) to build the
override container. Our in-repo to-zen/encode write-path is separate, deferred
work and was **not** touched here.

## Result: STAGED + DEPLOYED, high confidence

Every step that can be verified *without* the game passed:

1. **Magenta BC3 bytes are correct.** A throwaway `gore-tex` example built
   16384 bytes of solid opaque-magenta BC3 (PF_DXT5) and decoded them with our
   own `gore_tex::decode::to_rgba8` — every one of 128×128 pixels =
   `R=255, G=0, B=255, A=255`.
   - One magenta block (16 bytes, repeated 1024× = 32×32 blocks):
     `FF FF 00 00 00 00 00 00  1F F8 1F F8 00 00 00 00`
     (BC4 alpha=255 block, then BC1 color block c0=c1=0xF81F = RGB565 magenta).
2. **Spliced into the real cursor and re-decoded to magenta.** Unpacked
   `/Game/UI/Textures/Common/T_HardwareCursor` (128×128 PF_DXT5, mip0 **inline**
   in `.uexp`, no `.ubulk`). `gore_tex::decode::parse` reported `mip0.len()==16384`;
   its exact byte range in the `.uexp` is **`[117, 16501)`**. Overwrote those
   16384 bytes with magenta (same length → no offset bookkeeping). Re-ran
   `parse` + `to_rgba8` on the edited files → solid magenta.
3. **Packed to a Zen triplet via upstream retoc `to-zen`.**
4. **Round-tripped the produced container back to legacy and re-decoded to
   magenta** — end-to-end proof the override container carries the magenta
   cursor under the correct package name/path.

The throwaway verification example (`crates/gore-tex/examples/magenta_spike.rs`)
was **deleted** after use and is intentionally not committed. All scratch lives
under `work/spike/` (gitignored).

## In-pak / in-container mount path (the load-bearing detail)

- **Package name** (embedded in the cooked `.uasset`):
  `/Game/UI/Textures/Common/T_HardwareCursor`
- **Cooked container path:** `G1R/Content/UI/Textures/Common/T_HardwareCursor`
  (`.uasset` + `.uexp`). `/Game/` → `<Project=G1R>/Content/`.

**Why this is correct (not a guess):** retoc's `to-zen` derives the Zen package
name from the **package name string inside the `.uasset` header**, not from the
on-disk file path (see `asset_conversion.rs:487`,
`legacy_package_summary.package_name`). The IoStore `FPackageId` is a hash of
that package name, so the override chunk gets the **same chunk id** as the base
container's cursor package — which is exactly what makes a `~mods` container
shadow the base asset. Confirmed empirically: converting the produced container
*back* to legacy (`retoc to-legacy`, with the game's `global.utoc` present for
script-object resolution) emitted the asset at
`G1R/Content/UI/Textures/Common/T_HardwareCursor.{uasset,uexp}` and the bytes
decoded to magenta.

## Exact commands run

retoc used = **upstream** `trumank/retoc` @ `d7b635039c3db60942efabcd29d49679f42ab089`,
cloned to `%TEMP%\retoc-src`, built with the full CLI + its own Oodle backend:

```
git clone https://github.com/trumank/retoc "%TEMP%\retoc-src"   # (already present)
cd %TEMP%\retoc-src && cargo build --release --bin retoc
```

(Note: the dataset is tiny, so `to-zen` did not actually invoke Oodle
compression — no `oo2core_9_win64.dll` was fetched in this run. Upstream would
fetch it for larger inputs; that is fine for this throwaway.)

Lay the edited cooked files at the cooked path, then to-zen the directory:

```
work/spike/pak_root/G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset
work/spike/pak_root/G1R/Content/UI/Textures/Common/T_HardwareCursor.uexp

retoc.exe to-zen `
  "<repo>\work\spike\pak_root" `
  "<repo>\work\spike\out\zzz_MagentaTest_P.utoc" `
  --version UE5_4
```

`to-zen` accepts a **directory** of cooked files directly (no separate `repak`
step needed) and emits the full triplet itself:
`zzz_MagentaTest_P.{utoc,ucas,pak}` (the `.pak` is a 347-byte stub mount entry).

`retoc info` on the output: 1 package, 2 chunks, `mount_point ../../../`,
`container_flags Indexed`, toc version `OnDemandMetaData`.

## Files now in `~mods` (DEPLOYED)

Copied into
`D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\Paks\~mods\`:

- `zzz_MagentaTest_P.utoc`  (491 bytes)
- `zzz_MagentaTest_P.ucas`  (16969 bytes)
- `zzz_MagentaTest_P.pak`   (347 bytes)

The `zzz_..._P` name sorts last alphabetically so it mounts with high priority
over the base `G1R-Windows` container.

---

## HUMAN ACTION (completes the de-risk)

1. Launch the game:
   `D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe`
   (or launch via Steam).
2. At the **main menu**, move the mouse and look at the **cursor**.
   - **Cursor is solid magenta** → the `~mods` Zen container **overrode** a
     base-container asset. **Write-path risk is killed.** Record the result.
   - **Cursor is normal** → the override did not mount; try the fallbacks below.

### Undeploy (reversible — do this when done, regardless of result)

Delete the three files from `~mods`:

```
del "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\Paks\~mods\zzz_MagentaTest_P.utoc"
del "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\Paks\~mods\zzz_MagentaTest_P.ucas"
del "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\Paks\~mods\zzz_MagentaTest_P.pak"
```

(The base game is untouched — only these three files were added.)

### Fallbacks if the cursor does NOT turn magenta

Per the design doc (`docs/superpowers/specs/2026-06-26-gore-texture-replacement-design.md`,
Risk #1): proven G1R texture mods on Nexus ship as exactly this kind of Zen
triplet in `~mods`, so the format is right; the gate is usually *mounting*, not
the container. Try, in order:

1. **Load order / priority:** ensure the name sorts last. It already starts
   `zzz_` and ends `_P`. If multiple mods exist, a later-sorting name wins.
2. **`SimpleModLoader` / a BP/UE4SS mod loader present + one UE4SS-registered
   launch.** UE4SS is already installed (`G1R/Binaries/Win64/ue4ss`). Some
   builds only scan `~mods` when a loader mod is enabled. Confirm UE4SS loads,
   and that a mod-loader entry that mounts `~mods` is enabled.
3. **`LogicMods/` folder** (sibling of `~mods`, also exists/empty) as an
   alternate mount location for IoStore mods.
4. Try the triplet *without* the `~` (a plain `mods` or directly in `Paks/`),
   or as a `pakchunk`-style name, if the loader expects a different convention.

### Note on confidence / staging

The triplet was placed in `~mods` (not merely staged) because the in-container
mount path was **verified by round-trip**, not guessed. If you prefer to test
from a clean staging spot first, an identical copy remains at
`work/spike/out/zzz_MagentaTest_P.{utoc,ucas,pak}`.
