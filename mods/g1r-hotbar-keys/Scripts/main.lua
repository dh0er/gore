-- g1r-hotbar-keys — rebind the three hotbar slots the Controls menu cannot reach.
--
-- Gothic 1 Remake maps ten actions in /Game/Inputs/Mappings/IMC_EquipItems_KBM:
-- melee, ranged and eight quick slots (IA_EquipItem_Quick0 .. Quick7).  Seven of
-- them carry a PlayerMappableKeySettings object and are therefore listed in the
-- Controls menu; Quick5, Quick6 and Quick7 do not carry one, which is why their
-- keys cannot be changed in-game and why no save file can fix it: Enhanced Input
-- stores user rebinds under the mapping *name* that object provides, and for
-- those three actions no such name exists.
--
-- So we edit the mapping context itself, in memory, at runtime.
--
-- Console commands (needs UE4SS's console):
--   hotbarkeys        print every mapping of the context
--   hotbarkeys_apply  re-apply the config (after editing Config/config.lua)

local MOD_NAME = "g1r-hotbar-keys"
local CONFIG_PATH = "ue4ss/Mods/g1r-hotbar-keys/Config/config.lua"
local RETRY_MS = 2000

local function log(msg)
    print(string.format("[%s] %s\n", MOD_NAME, tostring(msg)))
end

-- ===== config ==============================================================

local defaults = {
    enabled = true,
    log_mappings = true,
    search_timeout_s = 120,
    context = "/Game/Inputs/Mappings/IMC_EquipItems_KBM.IMC_EquipItems_KBM",
    action_package = "/Game/Inputs/Actions/EquipItems/",
    bindings = {},
}

local function load_config()
    local ok, loaded = pcall(dofile, CONFIG_PATH)
    if not ok or type(loaded) ~= "table" then
        log("config not loaded (" .. tostring(loaded) .. "); using defaults")
        loaded = {}
    end
    for k, v in pairs(defaults) do
        if loaded[k] == nil then loaded[k] = v end
    end
    return loaded
end

local Config = load_config()

-- A short name like "IA_EquipItem_Quick5" becomes the full object path the
-- engine knows it by; a value that already looks like a path is left alone.
local function action_path(name)
    if name:sub(1, 1) == "/" then return name end
    return Config.action_package .. name .. "." .. name
end

-- ===== key names ===========================================================

-- Unreal's FKey names for keyboard and mouse, read out of G1R-Win64-Shipping.exe.
-- A misspelt name yields an invalid key and leaves the slot on nothing, so warn
-- about anything unrecognised.  See Config/key-names.md for the annotated list.
local KNOWN_KEYS = {}
do
    local function add(list)
        for name in list:gmatch("%S+") do KNOWN_KEYS[name] = true end
    end
    add([[
        AnyKey MouseX MouseY Mouse2D MouseScrollUp MouseScrollDown MouseWheelAxis
        LeftMouseButton RightMouseButton MiddleMouseButton ThumbMouseButton ThumbMouseButton2
        BackSpace Tab Enter Pause CapsLock Escape SpaceBar PageUp PageDown End Home
        Left Up Right Down Insert Delete
        Zero One Two Three Four Five Six Seven Eight Nine
        Multiply Add Subtract Decimal Divide NumLock ScrollLock
        LeftShift RightShift LeftControl RightControl LeftAlt RightAlt LeftCommand RightCommand
        Semicolon Equals Comma Underscore Hyphen Period Slash Tilde
        LeftBracket LeftParantheses Backslash RightBracket RightParantheses
        Apostrophe Quote Asterix Ampersand Caret Dollar Exclamation Colon
        A_AccentGrave E_AccentGrave E_AccentAigu C_Cedille
    ]])
    for c = string.byte("A"), string.byte("Z") do KNOWN_KEYS[string.char(c)] = true end
    for i = 1, 12 do KNOWN_KEYS["F" .. i] = true end
    add([[NumPadZero NumPadOne NumPadTwo NumPadThree NumPadFour
          NumPadFive NumPadSix NumPadSeven NumPadEight NumPadNine]])
end

local function known_key(name)
    -- Gamepad/VR names are legitimate but too numerous to list; accept by prefix.
    return KNOWN_KEYS[name] == true or name:match("^Gamepad_") ~= nil
end

-- ===== reflection helpers ==================================================

local function valid(o)
    if not o or type(o) ~= "userdata" then return false end
    local ok, v = pcall(function() return o:IsValid() end)
    return ok and v == true
end

local function find(path)
    local ok, o = pcall(StaticFindObject, path)
    if ok and valid(o) then return o end
    return nil
end

local function object_name(o)
    if not valid(o) then return "<none>" end
    local ok, n = pcall(function() return o:GetFullName() end)
    if ok and n then return n end
    return "<unnamed>"
end

-- UE4SS exposes TArray properties in more than one shape depending on version,
-- so read the mapping list through whichever accessor answers.
local function each_mapping(imc, fn)
    local ok, arr = pcall(function() return imc.Mappings end)
    if not ok or arr == nil then return false end

    local ok_foreach = pcall(function()
        arr:ForEach(function(index, elem)
            local e = elem
            local ok_get, got = pcall(function() return elem:get() end)
            if ok_get and got ~= nil then e = got end
            fn(index, e)
        end)
    end)
    if ok_foreach then return true end

    local ok_num, num = pcall(function() return arr:GetArrayNum() end)
    if not ok_num or type(num) ~= "number" then return false end
    for i = 1, num do
        local ok_el, el = pcall(function() return arr[i] end)
        if ok_el and el ~= nil then fn(i, el) end
    end
    return true
end

local function key_name_of(mapping)
    local ok, name = pcall(function() return mapping.Key.KeyName:ToString() end)
    if ok and name then return name end
    return "?"
end

local function action_name_of(mapping)
    local ok, name = pcall(function() return mapping.Action:GetFName():ToString() end)
    if ok and name then return name end
    return "?"
end

local function dump_mappings(imc, label)
    log(label .. " — " .. object_name(imc))
    local shown = 0
    local ok = each_mapping(imc, function(index, mapping)
        shown = shown + 1
        log(string.format("  [%s] %-24s <- %s", tostring(index),
            action_name_of(mapping), key_name_of(mapping)))
    end)
    if not ok then log("  (could not read the Mappings array)") end
    return shown
end

-- ===== the actual work =====================================================

-- Which keys is this action currently on?  Returns a list of key names.
local function keys_of(imc, action_short)
    local keys = {}
    each_mapping(imc, function(_, mapping)
        if action_name_of(mapping) == action_short then
            keys[#keys + 1] = key_name_of(mapping)
        end
    end)
    return keys
end

-- Last resort: overwrite the key name on the existing entries.  The engine caches
-- key details inside FKey, and this leaves that cache pointing at the old key, so
-- the button works but its on-screen glyph may be wrong.  Only used when MapKey
-- is not callable from Lua on this build.
local function rebind_in_place(imc, action_short, key_name)
    local hits = 0
    each_mapping(imc, function(_, mapping)
        if action_name_of(mapping) == action_short then
            if pcall(function() mapping.Key.KeyName = FName(key_name) end) then
                hits = hits + 1
            end
        end
    end)
    return hits > 0
end

-- Move `action` onto `key_name`.  Map the new key BEFORE dropping the old ones:
-- if MapKey turns out not to be callable, the action is still on its original key
-- rather than on none at all.
local function rebind(imc, action, action_short, key_name)
    local old = keys_of(imc, action_short)
    if #old == 1 and old[1] == key_name then
        return true -- already there
    end

    local ok_map = pcall(function() imc:MapKey(action, { KeyName = FName(key_name) }) end)
    if not ok_map then
        log("  MapKey unavailable for " .. action_short .. "; overwriting the key in place")
        return rebind_in_place(imc, action_short, key_name)
    end

    for _, k in ipairs(old) do
        if k ~= key_name then
            if not pcall(function() imc:UnmapKey(action, { KeyName = FName(k) }) end) then
                log("  warning: " .. action_short .. " is still also on " .. k)
            end
        end
    end
    return true
end

local function rebuild(imc)
    local lib = find("/Script/EnhancedInput.Default__EnhancedInputLibrary")
    if not lib then return false end
    return (pcall(function() lib:RequestRebuildControlMappingsUsingContext(imc, true) end))
end

local function apply()
    local imc = find(Config.context)
    if not imc then return false end

    if Config.log_mappings then dump_mappings(imc, "before") end

    local applied, failed = 0, 0
    for _, b in ipairs(Config.bindings or {}) do
        local path = action_path(b.action)
        local action = find(path)
        if not known_key(b.key) then
            log(string.format("warning: '%s' is not a known key name -- check the spelling " ..
                "against Config/key-names.md, or the slot ends up on no key at all", tostring(b.key)))
        end
        if not action then
            log("action not found: " .. path)
            failed = failed + 1
        elseif rebind(imc, action, path:match("([^/.]+)$") or b.action, b.key) then
            log(string.format("bound %s -> %s", b.action, b.key))
            applied = applied + 1
        else
            failed = failed + 1
        end
    end

    if applied > 0 and not rebuild(imc) then
        log("note: could not request a control-mapping rebuild; " ..
            "the new keys take effect on the next context change")
    end

    if Config.log_mappings then dump_mappings(imc, "after") end
    log(string.format("done: %d applied, %d failed", applied, failed))
    return true
end

-- ===== startup =============================================================

-- The mapping context only exists once the game has loaded its input assets, so
-- keep looking until it shows up, then stop.
local function start()
    local waited = 0
    local function attempt()
        if apply() then return end
        waited = waited + RETRY_MS / 1000
        if waited >= (Config.search_timeout_s or 120) then
            log("gave up: " .. Config.context .. " never showed up")
            return
        end
        ExecuteWithDelay(RETRY_MS, attempt)
    end
    ExecuteWithDelay(RETRY_MS, attempt)
end

if not Config.enabled then
    log("disabled via config")
    return
end

if #(Config.bindings or {}) == 0 then
    log("no bindings configured; nothing to do")
end

RegisterConsoleCommandHandler("hotbarkeys", function()
    local imc = find(Config.context)
    if imc then dump_mappings(imc, "current") else log("context not loaded yet") end
    return true
end)

RegisterConsoleCommandHandler("hotbarkeys_apply", function()
    Config = load_config()
    if not apply() then log("context not loaded yet") end
    return true
end)

start()
log("loaded; console: hotbarkeys | hotbarkeys_apply")
