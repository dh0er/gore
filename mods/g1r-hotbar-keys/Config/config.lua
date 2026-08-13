-- g1r-hotbar-keys configuration.
--
-- The game ships eight hotbar slots (IA_EquipItem_Quick0 .. Quick7) mapped to the
-- number row.  Only Quick0..Quick4 carry a PlayerMappableKeySettings object, so only
-- those show up in the in-game "Controls" menu; Quick5, Quick6 and Quick7 cannot be
-- rebound there at all.  This mod rewrites their keys in the mapping context instead.
--
-- Key names are Unreal FKey names, for example:
--   One Two Three Four Five Six Seven Eight Nine Zero
--   F1 .. F12
--   NumPadOne .. NumPadNine, NumPadZero
--   Q W E R T Z U I O P, SpaceBar, LeftShift, LeftControl, LeftAlt, Tab
--   ThumbMouseButton, ThumbMouseButton2, MiddleMouseButton
--
-- Change `key` to whatever you want and restart the game (or run `hotbarkeys_apply`
-- in the console).  The values below are the game's own defaults, so an unedited
-- config changes nothing.

return {
    enabled = true,

    -- Log every mapping of the context before and after applying.  Cheap, and the
    -- only way to see what actually took effect; leave it on until you trust it.
    log_mappings = true,

    -- Seconds to keep looking for the mapping context after the mod loads.  The
    -- asset only exists once the game has loaded its input data.
    search_timeout_s = 120,

    context = "/Game/Inputs/Mappings/IMC_EquipItems_KBM.IMC_EquipItems_KBM",

    -- Only list actions you actually want to move.  Anything left out keeps the
    -- key the game shipped with.
    --
    -- Quick5/Quick6/Quick7 are the three slots the Controls menu cannot reach.
    -- Quick0..Quick4, Melee and Ranged CAN be rebound in-game -- remapping those
    -- here as well works, but it drops them out of the in-game rebinding menu,
    -- so prefer the menu for them.
    bindings = {
        { action = "IA_EquipItem_Quick5", key = "Eight" },
        { action = "IA_EquipItem_Quick6", key = "Nine" },
        { action = "IA_EquipItem_Quick7", key = "Zero" },
    },

    -- Where the IA_* assets live; used to expand the short names above.
    action_package = "/Game/Inputs/Actions/EquipItems/",
}
