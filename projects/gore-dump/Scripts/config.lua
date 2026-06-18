-- gore-dump config — safe to edit your deployed copy.
return {
  -- Auto stats-dump when the game finishes loading (main menu is enough).
  auto = {
    enabled  = true,
    delay_ms = 15000,
    stats    = true,   -- write gore_game_data.json (item stat defaults)
  },
  -- Default kinds for the loc dump (override per call: `gore-dump loc de item`).
  -- The loc dump is manual: the engine language does not drive the Alkimia text,
  -- so set the language in the options menu, then run `gore-dump loc <lang>`.
  loc = {
    kinds = {"item", "npc", "knowledge"},
  },
}
