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
