-- gore-dump config — safe to edit your deployed copy.
return {
  -- Auto-dump when the game finishes loading (main menu is enough).
  auto = {
    enabled  = true,
    delay_ms = 15000,
    stats    = true,   -- write gore_game_data.json (item stat defaults)
    loc      = false,  -- loc dump freezes the game ~80s; run it on demand via
                       -- the `gore-dump loc` console command instead.
  },
  -- Loc dump scope.
  loc = {
    kinds    = {"item", "npc", "knowledge"},
    -- "all" = every culture the game ships (auto-discovered). Or pin a list,
    -- e.g. {"en", "de", "fr"} — useful if culture auto-switch misbehaves.
    cultures = "all",
  },
}
