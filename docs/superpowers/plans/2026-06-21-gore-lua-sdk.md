# gore-lua SDK Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `projects/gore-lua/` — a shared UE4SS Lua modding SDK (`require("gorelib")` → namespaced `gore.*` table) with an in-game `gorehelp` command, a `gore-cli deploy-shared` command to install it, and `scaffold` wiring so new mods load it automatically.

**Architecture:** One deployable Lua file `gorelib.lua` assembling the whole `gore` table (namespaces `obj/player/ui/gas/cheat/cmd/help`), each helper pcall-guarded and self-registered into a help registry. A minimal example mod + `gore.selftest()` validate it in-game. `gore-cli deploy-shared` copies `projects/gore-lua/shared/` into the game's `ue4ss/Mods/shared/`; `scaffold` injects a robust loader snippet into new mods.

**Tech Stack:** Lua (UE4SS runtime; no interpreter on the dev box → Lua validated in-game, like the existing mods). Rust (gore-cli, clap/anyhow) with `tests/integration` (assert_cmd/predicates/tempfile) for the two CLI pieces — strict TDD.

**Spec:** `docs/superpowers/specs/2026-06-21-gore-lua-sdk-design.md`.

**Testing note:** The Lua SDK has no automated unit tests (no `lua` on the machine; the repo's existing mods are likewise validated in-game). Lua validation = Task 4 (deploy + run `gorehelp`/`goretest`/`gore.selftest()` in-game). The Rust tasks (5, 6) are strict TDD following `tests/integration/scaffold_test.rs` / `package_test.rs`.

---

### Task 1: The SDK — `gorelib.lua`

**Files:**
- Create: `projects/gore-lua/shared/gorelib/gorelib.lua`

- [ ] **Step 1: Write the complete SDK file**

Create `projects/gore-lua/shared/gorelib/gorelib.lua`:

```lua
-- gorelib.lua — shared UE4SS modding SDK for Gothic 1 Remake.  Load with:
--   local ok, gore = pcall(require, "gorelib")
-- Every helper pcall-guards its reflection and returns nil/false on failure (never throws).
-- In-game: `gorehelp [filter]` lists the API.  `gore.selftest()` probes every namespace.

local gore = { _VERSION = "0.1.0" }

-- ===== help registry =======================================================
gore.help = {}
local REG = {}
function gore.help.register(ns, name, sig, doc)
    REG[#REG + 1] = { ns = ns, name = name, sig = sig, doc = doc }
end
function gore.help.list(filter)
    local out = {}
    for _, e in ipairs(REG) do
        if not filter or e.ns:find(filter, 1, true) or e.name:find(filter, 1, true) then
            out[#out + 1] = e
        end
    end
    table.sort(out, function(a, b) return (a.ns .. a.name) < (b.ns .. b.name) end)
    return out
end
local function R(ns, name, sig, doc) gore.help.register(ns, name, sig, doc) end

-- ===== ui.log (needed early) ===============================================
gore.ui = {}
function gore.ui.log(...)
    local parts = {}
    for i = 1, select("#", ...) do parts[i] = tostring(select(i, ...)) end
    print("[gore] " .. table.concat(parts, " ") .. "\n")
end
R("ui", "log", "log(...)", "print a tagged line to the UE4SS log")

-- ===== obj: objects / CDOs / properties ====================================
gore.obj = {}
function gore.obj.valid(o)
    if not o or type(o) ~= "userdata" then return false end
    local ok, v = pcall(function() return o:IsValid() end)
    return ok and v == true
end
function gore.obj.find(cls)
    local ok, o = pcall(FindFirstOf, cls)
    if ok and gore.obj.valid(o) then return o end
    return nil
end
function gore.obj.findAll(cls)
    local ok, list = pcall(FindAllOf, cls)
    if ok and list then return list end
    return {}
end
function gore.obj.cdo(path)
    local ok, o = pcall(StaticFindObject, path)
    if ok and gore.obj.valid(o) then return o end
    return nil
end
function gore.obj.prop(o, name, default)
    if not gore.obj.valid(o) then return default end
    local ok, v = pcall(function() return o[name] end)
    if ok and v ~= nil then return v end
    return default
end
function gore.obj.setProp(o, name, v)
    if not gore.obj.valid(o) then return false end
    return (pcall(function() o[name] = v end))
end
R("obj", "valid", "valid(o)", "true if o is a live UObject")
R("obj", "find", "find(cls)", "FindFirstOf(cls) or nil")
R("obj", "findAll", "findAll(cls)", "FindAllOf(cls) as a list (never nil)")
R("obj", "cdo", "cdo(path)", "StaticFindObject of a /Script/...Default__X CDO, or nil")
R("obj", "prop", "prop(o,name[,default])", "safe property get")
R("obj", "setProp", "setProp(o,name,v)", "safe property set; returns ok")

-- ===== player: controller / pawn / world ===================================
gore.player = {}
function gore.player.pc() return gore.obj.find("PlayerController") end
function gore.player.pawn()
    local p = gore.obj.find("GothicPlayerCharacter")
    if p then return p end
    local pc = gore.player.pc()
    if pc then return gore.obj.prop(pc, "Pawn") end
    return nil
end
function gore.player.asc()
    local pawn = gore.player.pawn(); if not pawn then return nil end
    local ok, asc = pcall(function() return pawn:GetAbilitySystemComponent() end)
    if ok and gore.obj.valid(asc) then return asc end
    return nil
end
function gore.player.loc(actor)
    local x, y, z = 0, 0, 0
    if gore.obj.valid(actor) then
        pcall(function() local l = actor.RootComponent.RelativeLocation; x, y, z = l.X, l.Y, l.Z end)
    end
    return x, y, z
end
function gore.player.setLoc(actor, x, y, z)
    if not gore.obj.valid(actor) then return false end
    return (pcall(function() actor.RootComponent.RelativeLocation = { X = x, Y = y, Z = z } end))
end
function gore.player.rot()
    local pitch, yaw = 0, 0
    local pc = gore.player.pc()
    if pc then pcall(function() local r = pc.ControlRotation; pitch, yaw = r.Pitch, r.Yaw end) end
    return pitch, yaw
end
function gore.player.forward()
    local pitch, yaw = gore.player.rot()
    local pr, yr = math.rad(pitch), math.rad(yaw)
    return math.cos(pr) * math.cos(yr), math.cos(pr) * math.sin(yr), math.sin(pr)
end
R("player", "pc", "pc()", "the local PlayerController or nil")
R("player", "pawn", "pawn()", "the player pawn (GothicPlayerCharacter) or nil")
R("player", "asc", "asc()", "the player AbilitySystemComponent or nil")
R("player", "loc", "loc(actor)", "actor world location -> x,y,z")
R("player", "setLoc", "setLoc(actor,x,y,z)", "teleport actor; returns ok")
R("player", "rot", "rot()", "control rotation -> pitch,yaw")
R("player", "forward", "forward()", "unit look vector -> x,y,z")

-- ===== ui: on-screen text (game's own message UI) ==========================
function gore.ui.ftext(s)
    local ktl = gore.obj.cdo("/Script/Engine.Default__KismetTextLibrary")
    if ktl then local ok, t = pcall(function() return ktl:Conv_StringToText(s) end); if ok then return t end end
    return nil
end
function gore.ui.text(s)
    local txt = gore.ui.ftext(s); if not txt then gore.ui.log("ui.text: no FText"); return false end
    local function try(name, fn)
        local o = gore.obj.find(name)
        if o then return (pcall(fn, o)) end
        return false
    end
    if try("HUDSimpleTextMessageController", function(o) o:ShowSimpleTextMessage(txt) end) then return true end
    if try("W_SimpleTextMessage_C", function(o) o:ShowSimpleTextMessage(txt) end) then return true end
    if try("SettingsMessageWidget", function(o) o:AddMessage(txt, 5.0, 0) end) then return true end
    gore.ui.log("ui.text: no on-screen target (is the gameplay HUD up?)")
    return false
end
gore.ui.notify = gore.ui.text
R("ui", "ftext", "ftext(s)", "string -> FText")
R("ui", "text", "text(s)", "show s on screen via the game's HUD message UI; returns ok")
R("ui", "notify", "notify(s)", "alias of ui.text")

-- ===== gas: gameplay attributes ============================================
gore.gas = {}
function gore.gas.setAttr(setPath, name, v)
    local asc = gore.player.asc(); if not asc then return false end
    local set = gore.obj.cdo(setPath); if not set then return false end
    return (pcall(function() asc:SetAttributeBaseValue(set, FName(name), v) end))
end
function gore.gas.getAttr(setPath, name, default)
    local asc = gore.player.asc(); if not asc then return default end
    local set = gore.obj.cdo(setPath); if not set then return default end
    local ok, v = pcall(function() return asc:GetAttributeBaseValue(set, FName(name), default or 0.0) end)
    if ok and v ~= nil then return v end
    return default
end
function gore.gas.heal()
    local max = gore.gas.getAttr("/Script/G1R.AttributeSet_Health", "MaxHealth", 100.0)
    return gore.gas.setAttr("/Script/G1R.AttributeSet_Health", "Health", max)
end
function gore.gas.buff(tbl)
    local n = 0
    for setPath, attrs in pairs(tbl) do
        for name, v in pairs(attrs) do if gore.gas.setAttr(setPath, name, v) then n = n + 1 end end
    end
    return n
end
R("gas", "setAttr", "setAttr(setPath,name,v)", "set a GAS attribute base value; returns ok")
R("gas", "getAttr", "getAttr(setPath,name[,default])", "read a GAS attribute base value")
R("gas", "heal", "heal()", "set Health to MaxHealth; returns ok")
R("gas", "buff", "buff(tbl)", "apply {setPath={attr=val}}; returns count set")

-- ===== cheat: god / enable cheats ==========================================
gore.cheat = {}
function gore.cheat.god(on)
    local n = 0
    for _, o in ipairs(gore.obj.findAll("CombatConfig")) do
        if gore.obj.setProp(o, "m_GodMode", on) then n = n + 1 end
    end
    for _, p in ipairs({ "/Script/Angelscript.Default__CombatConfig", "/Script/G1R.Default__CombatConfig" }) do
        local c = gore.obj.cdo(p); if c and gore.obj.setProp(c, "m_GodMode", on) then n = n + 1 end
    end
    return n
end
function gore.cheat.enableCheats()
    local pc = gore.player.pc(); if not pc then return false end
    return (pcall(function() pc:EnableCheats() end))
end
R("cheat", "god", "god(on)", "set m_GodMode on live CombatConfig + CDOs; returns count")
R("cheat", "enableCheats", "enableCheats()", "call PlayerController:EnableCheats(); returns ok")

-- ===== cmd: console commands / keybinds / game thread ======================
gore.cmd = {}
function gore.cmd.onGameThread(fn)
    if type(ExecuteInGameThread) == "function" then ExecuteInGameThread(fn) else pcall(fn) end
end
function gore.cmd.command(name, fn)
    return (pcall(RegisterConsoleCommandHandler, name, function(_, params, ar)
        local ok, err = pcall(fn, params, ar)
        if not ok then gore.ui.log(name .. " error: " .. tostring(err)) end
        return true
    end))
end
function gore.cmd.keybind(key, fn)
    return (pcall(RegisterKeyBind, key, function() gore.cmd.onGameThread(fn) end))
end
R("cmd", "command", "command(name,fn)", "register console command; fn(params,ar)")
R("cmd", "keybind", "keybind(key,fn)", "bind a key; fn runs on the game thread")
R("cmd", "onGameThread", "onGameThread(fn)", "run fn on the game thread")

-- ===== selftest ============================================================
function gore.selftest()
    local results = {}
    local function probe(label, fn)
        local ok = pcall(fn)
        results[#results + 1] = (ok and "OK   " or "FAIL ") .. label
    end
    probe("obj.find(PlayerController)", function() gore.obj.find("PlayerController") end)
    probe("player.pawn", function() gore.player.pawn() end)
    probe("player.asc", function() gore.player.asc() end)
    probe("ui.ftext", function() gore.ui.ftext("x") end)
    probe("ui.text", function() gore.ui.text("gore.selftest()") end)
    probe("gas.getAttr", function() gore.gas.getAttr("/Script/G1R.AttributeSet_Health", "MaxHealth", 0) end)
    probe("cheat.god(false)", function() gore.cheat.god(false) end)
    gore.ui.log("selftest:")
    for _, r in ipairs(results) do gore.ui.log("  " .. r) end
    return results
end
R("", "selftest", "selftest()", "probe every namespace safely; logs OK/FAIL")

-- ===== register the in-game gorehelp command on load =======================
gore.cmd.command("gorehelp", function(params)
    local filter = params and params[1]
    gore.ui.log("gore SDK " .. gore._VERSION .. (filter and (" [" .. filter .. "]") or "") .. ":")
    for _, e in ipairs(gore.help.list(filter)) do
        local ns = (e.ns ~= "" and ("gore." .. e.ns .. ".") or "gore.")
        gore.ui.log(string.format("  %s%-26s -- %s", ns, e.sig, e.doc))
    end
end)

return gore
```

- [ ] **Step 2: Sanity-check structure (no interpreter; visual + paren balance)**

Run: `grep -c "^function \|R(\"" projects/gore-lua/shared/gorelib/gorelib.lua`
Expected: a non-zero count (functions + registrations present). Eyeball that every `function` has a matching `end` and the file ends with `return gore`.

- [ ] **Step 3: Commit**

```bash
git add projects/gore-lua/shared/gorelib/gorelib.lua
git commit -m "feat(gore-lua): shared UE4SS modding SDK (gorelib.lua)"
```

---

### Task 2: Example mod (reference consumer + in-game smoke test)

**Files:**
- Create: `projects/gore-lua/example/enabled.txt`
- Create: `projects/gore-lua/example/Scripts/main.lua`

- [ ] **Step 1: Create the enabled marker**

Create `projects/gore-lua/example/enabled.txt` (empty file — UE4SS load marker):

```
```

- [ ] **Step 2: Create the example mod**

Create `projects/gore-lua/example/Scripts/main.lua`:

```lua
-- gorelib example mod: loads the SDK and registers `goretest` (runs gore.selftest)
-- and `goresay <msg>` (on-screen text). Proves deploy + load + API end-to-end.

local ok, gore = pcall(require, "gorelib")
if not ok or not gore then
    -- robust fallback: load directly from the shared folder (require can be finicky)
    local base = [[\ue4ss\Mods\shared\gorelib\gorelib.lua]]
    for _, root in ipairs({ "ue4ss/Mods/shared/gorelib/gorelib.lua", "." .. base }) do
        local f = loadfile(root)
        if f then gore = f(); break end
    end
end

if not gore then
    print("[gorelib-example] FAILED to load gorelib\n")
    return
end

gore.ui.log("example mod loaded; gore SDK " .. gore._VERSION)

gore.cmd.command("goretest", function()
    gore.selftest()
end)

gore.cmd.command("goresay", function(params)
    local msg = (params and #params > 0) and table.concat(params, " ") or "hello from gorelib"
    gore.ui.text(msg)
end)

print("[gorelib-example] ready. Console: goretest | goresay <msg> | gorehelp\n")
```

- [ ] **Step 3: Commit**

```bash
git add projects/gore-lua/example
git commit -m "feat(gore-lua): example mod (smoke test for the SDK)"
```

---

### Task 3: README API reference

**Files:**
- Create: `projects/gore-lua/README.md`

- [ ] **Step 1: Write the README**

Create `projects/gore-lua/README.md`:

```markdown
# gore-lua — shared UE4SS modding SDK

Common helpers for Gothic 1 Remake UE4SS mods. Source of truth for the live API is the
in-game `gorehelp` command; this README mirrors it.

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
- `gore.help` — `register(ns,name,sig,doc)`, `list(filter)`; plus the `gorehelp [filter]` console command
- `gore.selftest()` — probe every namespace, log OK/FAIL

Every helper pcall-guards its reflection and returns `nil`/`false` on failure — it never
crashes the consuming mod.

## On-screen text
This shipping build strips UE's debug `AddOnScreenDebugMessage`, so `gore.ui.text` uses the
game's own `HUDSimpleTextMessageController:ShowSimpleTextMessage` (needs the gameplay HUD up;
no-ops at the main menu).
```

- [ ] **Step 2: Commit**

```bash
git add projects/gore-lua/README.md
git commit -m "docs(gore-lua): API reference README"
```

---

### Task 4: In-game validation

This is the Lua validation path (no interpreter on the dev box). Manual, on the real game.

- [ ] **Step 1: Deploy the SDK + example mod into the game**

```bash
g="/d/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/Mods"
mkdir -p "$g/shared/gorelib" "$g/gorelib-example/Scripts"
cp projects/gore-lua/shared/gorelib/gorelib.lua "$g/shared/gorelib/gorelib.lua"
cp projects/gore-lua/example/Scripts/main.lua "$g/gorelib-example/Scripts/main.lua"
cp projects/gore-lua/example/enabled.txt "$g/gorelib-example/enabled.txt"
```

- [ ] **Step 2: Launch the game (normal mode), load a save (so the gameplay HUD exists)**

- [ ] **Step 3: Open the console (`^`) and verify**

Type each and confirm:
- `gorehelp` → logs the full API grouped by namespace (check the UE4SS console window).
- `gorehelp ui` → logs only the `ui` entries.
- `goretest` → logs `selftest:` then `OK/FAIL` per helper; expect all `OK` in-game.
- `goresay gorelib works` → on-screen text "gorelib works" appears via the game's message UI.

Expected: all four behave as described. If `goresay` shows nothing, confirm you are in-game
(not the menu) — the HUD message UI needs gameplay.

- [ ] **Step 4: Record the result + clean up the deployed example**

```bash
rm -rf "$g/gorelib-example"
```
(Leave `shared/gorelib` deployed.) Note the in-game result in the commit message of Task 5
or a short note; no code change in this task.

---

### Task 5: `gore-cli deploy-shared` command (Rust, TDD)

**Files:**
- Create: `projects/gore-cli/crates/gore_cli/src/cmd/deploy_shared.rs`
- Modify: `projects/gore-cli/crates/gore_cli/src/cmd/mod.rs`
- Modify: `projects/gore-cli/crates/gore_cli/src/main.rs`
- Create: `projects/gore-cli/crates/gore_cli/tests/integration/deploy_shared_test.rs`
- Modify: `projects/gore-cli/crates/gore_cli/Cargo.toml` (register the `[[test]]`)

- [ ] **Step 1: Write the failing integration test**

Create `projects/gore-cli/crates/gore_cli/tests/integration/deploy_shared_test.rs`:

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn deploy_shared_copies_tree_into_mods_shared() {
    let src = tempdir().unwrap();
    // a fake projects/gore-lua/shared/ tree
    fs::create_dir_all(src.path().join("gorelib")).unwrap();
    fs::write(src.path().join("gorelib/gorelib.lua"), "return {}\n").unwrap();

    let game = tempdir().unwrap();
    let mods = game.path().join("ue4ss/Mods");
    fs::create_dir_all(&mods).unwrap();

    Command::cargo_bin("gore-cli")
        .unwrap()
        .args([
            "deploy-shared",
            "--src",
            src.path().to_str().unwrap(),
            "--game",
            game.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let dest = mods.join("shared/gorelib/gorelib.lua");
    assert!(dest.exists(), "gorelib.lua should be copied to Mods/shared/");
    assert_eq!(fs::read_to_string(dest).unwrap(), "return {}\n");
}
```

- [ ] **Step 2: Run it — expect failure (subcommand missing)**

Run: `cargo test -p gore_cli --test deploy_shared_test`
Expected: FAIL — `deploy-shared` is not a known subcommand / test binary not found.

- [ ] **Step 3: Implement the command**

Create `projects/gore-cli/crates/gore_cli/src/cmd/deploy_shared.rs`:

```rust
//! `gore-cli deploy-shared` — copy the gore-lua shared/ tree into the game's
//! `ue4ss/Mods/shared/`, so mods can `require("gorelib")`.

use anyhow::{bail, Context, Result};
use std::{fs, path::Path, path::PathBuf};

pub fn run(src: PathBuf, game: PathBuf) -> Result<()> {
    if !src.is_dir() {
        bail!("source '{}' is not a directory", src.display());
    }
    let dest_root = game.join("ue4ss").join("Mods").join("shared");
    if !game.join("ue4ss").join("Mods").is_dir() {
        bail!(
            "'{}' does not look like a game dir (no ue4ss/Mods)",
            game.display()
        );
    }
    let n = copy_dir(&src, &dest_root)?;
    println!("deployed {n} file(s) to {}", dest_root.display());
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    let mut count = 0;
    for entry in fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            count += copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).with_context(|| format!("copying {}", from.display()))?;
            count += 1;
        }
    }
    Ok(count)
}
```

- [ ] **Step 4: Register the module**

In `projects/gore-cli/crates/gore_cli/src/cmd/mod.rs`, add (keep the list alphabetical if it is):

```rust
pub mod deploy_shared;
```

- [ ] **Step 5: Wire the subcommand**

In `projects/gore-cli/crates/gore_cli/src/main.rs`, add a variant to the `Commands` enum (match the style of the existing `Scaffold`/`Package` variants):

```rust
    /// Deploy the gore-lua shared SDK into the game's ue4ss/Mods/shared.
    DeployShared {
        /// Source shared/ dir (default: projects/gore-lua/shared).
        #[arg(long, default_value = "projects/gore-lua/shared")]
        src: std::path::PathBuf,
        /// Game dir containing ue4ss/Mods.
        #[arg(long)]
        game: std::path::PathBuf,
    },
```

and in the `match` that dispatches commands, add:

```rust
        Commands::DeployShared { src, game } => cmd::deploy_shared::run(src, game),
```

- [ ] **Step 6: Register the test binary in Cargo.toml**

In `projects/gore-cli/crates/gore_cli/Cargo.toml`, add alongside the other `[[test]]` entries:

```toml
[[test]]
name = "deploy_shared_test"
path = "tests/integration/deploy_shared_test.rs"
```

- [ ] **Step 7: Run the test — expect pass**

Run: `cargo test -p gore_cli --test deploy_shared_test`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add projects/gore-cli/crates/gore_cli/src/cmd/deploy_shared.rs \
  projects/gore-cli/crates/gore_cli/src/cmd/mod.rs \
  projects/gore-cli/crates/gore_cli/src/main.rs \
  projects/gore-cli/crates/gore_cli/tests/integration/deploy_shared_test.rs \
  projects/gore-cli/crates/gore_cli/Cargo.toml
git commit -m "feat(gore-cli): deploy-shared command for the gore-lua SDK"
```

---

### Task 6: `scaffold` wires the gorelib loader into new mods (Rust, TDD)

**Files:**
- Modify: `projects/gore-cli/crates/gore_cli/src/cmd/scaffold.rs`
- Modify: `projects/gore-cli/crates/gore_cli/tests/integration/scaffold_test.rs`

- [ ] **Step 1: Add a failing assertion to the scaffold test**

In `projects/gore-cli/crates/gore_cli/tests/integration/scaffold_test.rs`, add a test (adapt the
existing test's mod-dir/output-reading helpers; this assumes scaffold writes `<mod>/Scripts/main.lua`):

```rust
#[test]
fn scaffold_main_lua_wires_gorelib_loader() {
    let dir = tempfile::tempdir().unwrap();
    assert_cmd::Command::cargo_bin("gore-cli")
        .unwrap()
        .args(["scaffold", "MyMod", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let main = std::fs::read_to_string(dir.path().join("MyMod/Scripts/main.lua")).unwrap();
    assert!(main.contains(r#"require("gorelib")"#), "should require gorelib");
    assert!(main.contains("loadfile"), "should have a loadfile fallback");
}
```

- [ ] **Step 2: Run it — expect failure**

Run: `cargo test -p gore_cli --test scaffold_test scaffold_main_lua_wires_gorelib_loader`
Expected: FAIL — the generated `main.lua` has no gorelib loader yet.

- [ ] **Step 3: Inject the loader into the scaffold template**

In `projects/gore-cli/crates/gore_cli/src/cmd/scaffold.rs`, prepend the loader to the generated
`main_lua` string (it is built with a `format!`/raw string — add these lines at the very top of the
template, before the existing body):

```rust
    let main_lua = format!(
        r#"-- {name} — generated by gore-cli scaffold

-- gore-lua SDK loader (require + robust loadfile fallback)
local ok, gore = pcall(require, "gorelib")
if not ok or not gore then
    local f = loadfile("ue4ss/Mods/shared/gorelib/gorelib.lua")
    if f then gore = f() end
end
-- use gore.* helpers below; see `gorehelp` in-game or projects/gore-lua/README.md

{body}
"#,
        name = mod_name,
        body = "-- your mod code here",
    );
```

If the existing `scaffold.rs` already builds `main_lua` with a specific body (the CDO example),
keep that body and only PREPEND the loader block — i.e. insert the `local ok, gore = ...` block
between the header comment and the existing CDO-pattern body, leaving the rest unchanged.

- [ ] **Step 4: Run the new test — expect pass**

Run: `cargo test -p gore_cli --test scaffold_test`
Expected: PASS (the new test + the pre-existing scaffold tests).

- [ ] **Step 5: Commit**

```bash
git add projects/gore-cli/crates/gore_cli/src/cmd/scaffold.rs \
  projects/gore-cli/crates/gore_cli/tests/integration/scaffold_test.rs
git commit -m "feat(gore-cli): scaffold wires the gorelib loader into new mods"
```

---

## Self-review

- **Spec coverage:** SDK file + 7 namespaces → Task 1; in-game `gorehelp` + `gore.selftest` → Task 1 (registered) + Task 4 (validated); README → Task 3; example/smoke mod → Task 2; `gore-cli deploy-shared` → Task 5; `scaffold` wiring → Task 6; "no existing-mod refactor" honored (none touched). Out-of-scope items (item-give helper, generated docs, multi-file split, mod refactors) correctly omitted.
- **No placeholders:** Task 1 ships the complete SDK; Tasks 5–6 ship complete Rust + tests. Task 4 is explicitly the manual in-game validation path (Lua has no interpreter here, matching the repo's mod practice) with exact commands/expected behavior — not a vague "test it".
- **Type/name consistency:** `gore.obj/player/ui/gas/cheat/cmd/help` namespaces and every function name match between the SDK (Task 1), README (Task 3), example mod (Task 2: `gore.ui.text`, `gore.cmd.command`, `gore.selftest`), and the `gorehelp` registry. `deploy-shared` flags `--src`/`--game` match between the command (Task 5 impl) and its test.
- **Assumption to verify during execution:** Task 6 assumes `scaffold.rs` builds `main.lua` via a `format!` string and writes it to `<mod>/Scripts/main.lua` (confirmed by reading the current `scaffold.rs` header in the spec exploration). If the variable name differs from `main_lua`/`mod_name`, adapt the prepend accordingly — the requirement is only that the generated `main.lua` contains `require("gorelib")` + a `loadfile` fallback.
