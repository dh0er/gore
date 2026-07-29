# gore-lua — shared UE4SS helper library

Common helpers for Gothic 1 Remake UE4SS mods. This README is the API reference;
`gore.help.list([filter])` returns the same registry programmatically.

It is a thin convenience layer, not a full SDK: a handful of `pcall`-guarded
wrappers over UE4SS reflection, for behavior the override generator cannot
express (hooks, keybinds, console commands, live attribute tweaks). Override
mods produced by [`gore gen`](../docs/guide/items.md) or Mod Studio do **not**
use it — it is for hand-written mods.

## Use it

```powershell
gore deploy-shared --game "$GAME"       # copy gore-lua into ue4ss\Mods\shared (once)
gore scaffold MyMod -o "$GAME\...\Mods" # new mod with the loader wired in
```

Then, in a mod:

```lua
local ok, gore = pcall(require, "gore-lua")

gore.cheat.god(true)                        -- toggle god mode on the live CombatConfig + CDOs
gore.gas.heal()                             -- set Health to MaxHealth
gore.ui.text("hello from my mod")           -- on-screen message via the game's HUD
gore.cmd.command("mycmd", function() end)   -- register a console command
```

UE4SS loads any mod folder containing an `enabled.txt`.

## API reference

Each function below mirrors the one-line doc registered in the source (`gore.help.list()`
returns the same).

### `gore.obj` — objects / CDOs / properties
| function | does |
|---|---|
| `valid(o)` | true if `o` is a live UObject |
| `find(cls)` | `FindFirstOf(cls)` or nil |
| `findAll(cls)` | `FindAllOf(cls)` as a list (never nil) |
| `cdo(path)` | `StaticFindObject` of a `/Script/...Default__X` CDO, or nil |
| `prop(o,name[,default])` | safe property get |
| `setProp(o,name,v)` | safe property set; returns ok |

### `gore.player` — controller / pawn / world
| function | does |
|---|---|
| `pc()` | the local PlayerController or nil |
| `pawn()` | the player pawn (GothicPlayerCharacter) or nil |
| `asc()` | the player AbilitySystemComponent or nil |
| `loc(actor)` | actor world location → x,y,z |
| `setLoc(actor,x,y,z)` | teleport actor; returns ok |
| `rot()` | control rotation → pitch,yaw |
| `forward()` | unit look vector → x,y,z |

### `gore.ui` — on-screen text + log
| function | does |
|---|---|
| `ftext(s)` | string → FText |
| `text(s)` | show `s` on screen via the game's HUD message UI; returns ok |
| `notify(s)` | alias of `text` |
| `log(...)` | print a tagged line to the UE4SS log |

### `gore.gas` — gameplay attributes
| function | does |
|---|---|
| `setAttr(setPath,name,v)` | set a GAS attribute base value; returns ok |
| `getAttr(setPath,name[,default])` | read a GAS attribute base value |
| `heal()` | set Health to MaxHealth; returns ok |
| `buff(tbl)` | apply `{setPath={attr=val}}`; returns count set |

### `gore.cheat` — cheats
| function | does |
|---|---|
| `god(on)` | set `m_GodMode` on live CombatConfig + CDOs; returns count |
| `enableCheats()` | call `PlayerController:EnableCheats()`; returns ok |

### `gore.cmd` — console commands / keybinds / game thread
| function | does |
|---|---|
| `command(name,fn)` | register console command; `fn(params,ar)` |
| `keybind(key,fn)` | bind a key; `fn` runs on the game thread |
| `onGameThread(fn)` | run `fn` on the game thread |

### `gore.help` / misc
| function | does |
|---|---|
| `gore.help.register(ns,name,sig,doc)` | add an entry to the API registry |
| `gore.help.list([filter])` | return the API registry (optionally filtered) |
| `gore.selftest()` | probe every namespace safely; logs OK/FAIL |

Every helper pcall-guards its reflection and returns `nil`/`false` on failure — it never
crashes the consuming mod.

## On-screen text
This shipping build strips UE's debug `AddOnScreenDebugMessage`, so `gore.ui.text` uses the
game's own `HUDSimpleTextMessageController:ShowSimpleTextMessage` (needs the gameplay HUD up;
no-ops at the main menu).
