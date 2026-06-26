# Texture mod-bundle pipeline — end-to-end verification (2026-06-26)

Proves a texture replacement flows through the UNIFIED `gore mod build` → `gore mod deploy`
pipeline and the produced Zen triplet loads in-game without crashing. The cook+pack pipeline
itself was already proven via `gore texture`; this verifies the new gore-mod orchestration
(manifest `texture_patch` → deploy cook arm → repack → `~mods` + deploy record).

**Result: PASS.** Build clean, bundle correct, triplet deployed + recorded, game booted clean
(no new crash, no `T_HardwareCursor`/`Bad name index`), undeploy left the install pristine.

## Environment
- Worktree: `C:\sbx\goresave\.claude\worktrees\funny-noether-3efb13`
- Game: `D:\SteamLibrary\steamapps\common\Gothic 1 Remake` (appid 1297900)
- Shell: PowerShell (Git-Bash mangles `/Game/` paths)

## Commands (exact)

```powershell
# 1. Build
cargo build
cargo build -p gore-ffi

# 2. Texture index (one-time, ~5 min)
cargo run -q -p gore -- texture index --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake"
#  -> wrote C:\Users\Daniel\AppData\Local\gore-tools\texture_index.json (13480 textures)

# 3. BuildSpec authored at work\spike\tex-e2e\spec.json (cursor256.png already present, 256x256 RGBA)

# 4. Build the bundle
cargo run -q -p gore -- mod build --spec work\spike\tex-e2e\spec.json -o work\spike\tex-e2e\bundle
#  -> built bundle: work\spike\tex-e2e\bundle\TexE2E (1 components, 3 files)
#  NOTE: --out writes to <out>/<mod-name>, so bundle is at ...\bundle\TexE2E

# 5. Deploy (cook+pack at deploy time)
cargo run -q -p gore -- mod deploy --bundle work\spike\tex-e2e\bundle\TexE2E --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake"
#  -> deployed 'TexE2E' (0 backup(s))

# 6. Self-launch + verify (see boot result below)
Start-Process "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe" `
  -ArgumentList "-windowed","-ResX=640","-ResY=360" -PassThru
Start-Sleep -Seconds 100
# ...checked UE4SS.log + crash dirs... then Stop-Process

# 7. Undeploy
cargo run -q -p gore -- mod undeploy --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake"
#  -> undeployed 'TexE2E' (0 restored)
```

## BuildSpec (work\spike\tex-e2e\spec.json)

```json
{
  "meta": { "name": "TexE2E", "version": "1.0", "author": "test" },
  "texture": [
    { "asset": "/Game/UI/Textures/Common/T_HardwareCursor",
      "image_path": "C:\\sbx\\goresave\\.claude\\worktrees\\funny-noether-3efb13\\work\\spike\\up\\cursor256.png" }
  ]
}
```

## Bundle contents (work\spike\tex-e2e\bundle\TexE2E)

```
gore-mod.json
texture\manifest.json
texture\0__Game_UI_Textures_Common_T_HardwareCursor.png   (4455 bytes, 256x256 RGBA)
```

`gore-mod.json` — one `texture_patch` component:
```json
{
  "format": 1,
  "mod": { "name": "TexE2E", "version": "1.0", "author": "test" },
  "components": [
    { "type": "texture_patch", "path": "texture",
      "assets": ["/Game/UI/Textures/Common/T_HardwareCursor"] }
  ]
}
```

`texture\manifest.json`:
```json
{ "/Game/UI/Textures/Common/T_HardwareCursor": "texture/0__Game_UI_Textures_Common_T_HardwareCursor.png" }
```

## Deployed triplet + record

`...\G1R\Content\Paks\~mods\`:
```
zzz_TexE2E_tex_P.pak    (347 B)
zzz_TexE2E_tex_P.ucas   (66121 B)
zzz_TexE2E_tex_P.utoc   (535 B)
```

`gore-mod.deployed.json` (game root) — `texture_triplets` listed all 3:
```
\\?\D:\...\~mods\zzz_TexE2E_tex_P.utoc
\\?\D:\...\~mods\zzz_TexE2E_tex_P.ucas
\\?\D:\...\~mods\zzz_TexE2E_tex_P.pak
```

## Boot result — CLEAN (no crash)

- Newest crash dir BEFORE launch: `UECC-Windows-EB81BA07413A406F9F1FAF884C96430A_0000` @ 2026-06-26 10:33:45.
- Game launched (PID 1904), still alive after 100 s.
- `UE4SS.log` fresh (mtime 16:40:27, baseline was 15:50:17); tail showed the game running normally —
  mods loaded (gore-dump wrote FMOD key, ConsoleEnabler), GC active, `GoreAsErrorDump` poller at
  poll 41 with `errors found: false`. No `T_HardwareCursor` / `Bad name index`.
- After 100 s the newest crash dir was STILL the pre-launch baseline — **no new crash dir created**.
- `Stop-Process` cleanly terminated the game.

## Undeploy — CLEAN

- `~mods` empty (triplet gone).
- `gore-mod.deployed.json` removed.
- Base game `G1R-Windows.{pak,ucas,utoc}` untouched (mtime 2026-06-23 08:29:52, unchanged).

Game left UNDEPLOYED and CLOSED.
