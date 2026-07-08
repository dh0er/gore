-- gore-lua example mod: loads the helpers and registers `goretest` (runs gore.selftest)
-- and `goresay <msg>` (on-screen text). Proves deploy + load + API end-to-end.

local ok, gore = pcall(require, "gore-lua")
if not ok then gore = nil end -- a failed pcall leaves the error string in `gore`; clear it
if not gore then
    -- robust fallback: load directly from the shared folder (require can be finicky)
    local base = [[\ue4ss\Mods\shared\gore-lua\gore-lua.lua]]
    for _, root in ipairs({ "ue4ss/Mods/shared/gore-lua/gore-lua.lua", "." .. base }) do
        local f = loadfile(root)
        if f then
            -- run the chunk under pcall: a load-time error in the helpers must not abort the mod
            local ok2, res = pcall(f)
            if ok2 then gore = res; break end
        end
    end
end

if not gore then
    print("[gore-lua-example] FAILED to load gore-lua\n")
    return
end

gore.ui.log("example mod loaded; gore-lua " .. gore._VERSION)

gore.cmd.command("goretest", function()
    gore.selftest()
end)

gore.cmd.command("goresay", function(params)
    local msg = (params and #params > 0) and table.concat(params, " ") or "hello from gore-lua"
    gore.ui.text(msg)
end)

print("[gore-lua-example] ready. Console: goretest | goresay <msg>\n")
