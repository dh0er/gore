# gore-lua — shared UE4SS modding SDK

Common helpers for Gothic 1 Remake UE4SS mods. This README is the API reference;
`gore.help.list([filter])` returns the same registry programmatically.

## Use it
Deploy with `gore-cli deploy-shared`, then in a mod:
```lua
local ok, gore = pcall(require, "gorelib")
```
New mods scaffolded with `gore-cli scaffold <name>` get this loader wired in automatically.

## Namespaces
- `gore.obj` — `valid(o)`, `find(cls)`, `findAll(cls)`, `cdo(path)`, `prop(o,name[,d])`, `setProp(o,name,v)`
- `gore.player` — `pc()`, `pawn()`, `asc()`, `loc(a)`, `setLoc(a,x,y,z)`, `rot()`, `forward()`
- `gore.ui` — `ftext(s)`, `text(s)`/`notify(s)` (on-screen via the game's HUD message UI), `log(...)`
- `gore.gas` — `setAttr(setPath,name,v)`, `getAttr(setPath,name[,d])`, `heal()`, `buff(tbl)`
- `gore.cheat` — `god(on)`, `enableCheats()`
- `gore.cmd` — `command(name,fn)`, `keybind(key,fn)`, `onGameThread(fn)`
- `gore.help` — `register(ns,name,sig,doc)`, `list(filter)` (programmatic API registry)
- `gore.selftest()` — probe every namespace, log OK/FAIL

Every helper pcall-guards its reflection and returns `nil`/`false` on failure — it never
crashes the consuming mod.

## On-screen text
This shipping build strips UE's debug `AddOnScreenDebugMessage`, so `gore.ui.text` uses the
game's own `HUDSimpleTextMessageController:ShowSimpleTextMessage` (needs the gameplay HUD up;
no-ops at the main menu).
