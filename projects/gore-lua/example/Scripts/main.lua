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
