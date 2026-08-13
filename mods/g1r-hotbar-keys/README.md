# g1r-hotbar-keys

Rebinds the three hotbar slots that Gothic 1 Remake's Controls menu cannot reach.

## Why the menu cannot do it

`/Game/Inputs/Mappings/IMC_EquipItems_KBM` maps ten actions to the number row: melee,
ranged and eight quick slots, `IA_EquipItem_Quick0` .. `IA_EquipItem_Quick7`.

An action only appears in the Controls menu if it carries a `PlayerMappableKeySettings`
object. In the shipped build:

| Action | `PlayerMappableKeySettings` | rebindable in-game |
|---|---|---|
| `IA_EquipItem_Melee`, `IA_EquipItem_Ranged` | yes | yes |
| `IA_EquipItem_Quick0` .. `Quick4` | yes | yes |
| `IA_EquipItem_Quick5`, `Quick6`, `Quick7` | **missing** | **no** |

That object also supplies the *mapping name* under which Enhanced Input stores a user
rebind in `EnhancedInputUserSettings.sav`. Without it there is no name, so no rebind can
be stored — editing that save file cannot fix this. The mapping context has to change.

## What this mod does

At startup it waits for the mapping context to load, then for every entry in
`Config/config.lua` moves the action onto the configured key and asks Enhanced Input to
rebuild its control mappings.

The new key is mapped *before* the old one is removed, so if `MapKey` turns out not to be
callable from Lua on this build, the slot keeps its original key instead of ending up on
none. In that case the mod falls back to overwriting the key name on the existing entry
and says so in the log.

## Use

1. Copy the folder into the game's UE4SS mods directory:

```bash
cp -r mods/g1r-hotbar-keys "/c/Program Files (x86)/Steam/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/Mods/"
```

2. Edit `Config/config.lua` — the shipped values are the game's own defaults, so an
   unedited config changes nothing.
3. Start the game and check `ue4ss/UE4SS.log` for the `[g1r-hotbar-keys]` lines. They list
   every mapping of the context before and after, so the log alone shows whether it took.

Console commands (needs UE4SS's console): `hotbarkeys` prints the current mappings,
`hotbarkeys_apply` re-reads the config and applies it without a restart.

## Limits

- The keys are fixed by the config file; this does not add the three slots to the in-game
  Controls menu. Doing that means giving the three `IA_*` assets a `PlayerMappableKeySettings`
  object, which is Unreal Editor work, not a runtime edit.
- Keyboard/mouse only. There is no gamepad variant of this context in the build.
