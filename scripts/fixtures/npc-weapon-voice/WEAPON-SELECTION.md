# B's close-range bow selection

Reported on2026-09-10 with NpcWarningRecovery0.1.12: B attacks and speaks his
combat-start line, but often draws his bow while the Hero stands beside him.
Sometimes he changes to his sword just before drawing the bowstring. The user
does not identify this as conclusively vanilla or mod behavior.

## Confirmed configuration and saved state

- B's authored definition retains Diego's
  `UGothicCharacterPersonality_Brave_Archer_Patient`, bow, sword and master weapon
  skills. The personality includes a1.5 bow/crossbow preference.
- Source defaults are Strength80/Dexterity120; actual saves61 and62 contain
  Strength120/Dexterity180 under the saved Hard settings. Both retain
  `ItRw_Bow_Diego`, `ItMw_1H_Sword_04_Diego_Sleeper` and100 arrows.
- The known bow requirement is Dexterity55; the sword requires Strength13.
  Missing weapon attributes therefore do not explain the observed switching.
- The [0.1.12 build proof](../npc-weapon-warning/recovery-build-result.json)
  preserves the original shared modules and Diego's extra zombie-target scoring
  initialization. B's custom AI changes the threat-warning state mapping.
  Original byte preservation does not establish correct runtime behavior.

Readback is in [runtime-result.json](runtime-result.json). Saves were copied for
inspection and their hashes were checked unchanged. They establish inventory
and attributes, not the transient target, selected action or weapon scores.

## Candidate explanation and limits

The emitted original `AI.AIItemScoring` source contains a close-target multiplier
that requires a current target, distance below400 game units and a successful
grounded-target reachability query. It favors melee and penalizes ranged items
only when those conditions hold. A separate entry penalizes unreachable melee
targets. Thus physical proximity alone is not the complete selection rule.

`AI.States.FightAI.CombatState.AIState_OGCombat` includes repeated weapon selection
during combat preparation and the opening taunt, plus timing/animation gates on
switching. Updating target or reachability state could explain a bow-first,
sword-afterward sequence. The actual values during the user's attempts were not
captured, so this remains a hypothesis. Emitted source alone is insufficient
given the known decompiler limitations.

### Bytecode verification

Product disassembly and resolved reference tables were checked against full
cache SHA256 `85c183da343ec3501fb4696064cc671e424a62fda75b991017f79e5140c5a0b9`:

- `UAIItemScoringEntry_Mlt_Weapon_CloseToTarget` constructor sets Distance400.
  Its eligibility method returns false for a null target (PC7–13), distance at
  least Distance (comparison PC75/branch77), or failed reachability (native call
  PC115, false return PC119–126). The reachability call receives100 and a height
  calculated from twice self collision half-height plus the target half-height.
- `UAIState_OGCombat::DoPrepareCombatTick` updates available items (PC7), checks
  whether weapons can be equipped (PC11), and selects an action with an
  `Item_Weapon` or Empty filter depending on `State_Combat` (PC47/75). It draws
  the selected weapon at PC121 when the selected action is a valid weapon.
- The module-span checker was rerun: all7315 unselected modules remain byte
  identical to the pristine cache. `AI.AIItemScoring` and
  `AI.States.FightAI.CombatState.AIState_OGCombat` are among them. The retained
  installed-cache proof allows only the previously verified Function.Id
  canonicalization. Thus these shared functions did not acquire decompiler
  changes from this mod build.

These checks establish the branches and module preservation, not the runtime
query results, weapon scores or the correctness of all shared combat behavior.

On2026-09-11 the warning selection was also checked in cached bytecode:
`UAIState_Warning_Crime::SelectAndDrawWeapon` gates on `CanEquipWeapons` (PC1),
updates available items (PC17), and calls `SelectBestItemActionFromInventory`
(PC36) with `FGameplayTag::Empty`, empty required tags and `bForceUpdate=false`.
It draws the non-null result at PC65. The WeaponDrawn subclass does not override
that method. There is no explicit melee-only filter at this warning call.
Therefore the warning sword proves usable melee equipment, but not which action
wins after the separate combat selection and changed combat context.

## Open-ground comparison result

The original instruction to move B away from his hut was impractical for a
bow-using NPC. The user saved the Hero on open ground on2026-09-11 as slot63,
`npc kampf - freie flaeche`. A separate **slot64, `npc kampf - B freie flaeche`**,
now places B200 game units ahead of the saved Hero, facing him. In this copy,
`DailyRoutine_Empty` prevents the hut routine from sending B back. B's equipment,
attributes, spawn pose and the Hero's state remain unchanged; normal conflict AI
is retained. The original63 and all other numbered saves were preserved, with
a profile backup before registration. See [prepared input](flat-ground-input.json).

The user tested64 on2026-09-11: B initially draws his sword when the Hero draws
the sword, then switches to his bow when the warning escalates into combat.
This reproduces the problem away from the hut and with the hut routine disabled.
The transition is now identified more precisely as **warning sword -> combat
bow**. The hut's immediate geometry is not required to reproduce the observation;
the game still did not expose the target, reachability result or item scores.
The disabled routine is an additional controlled change used for placement.
See [runtime record](flat-ground-runtime-result.json).

Further generic terrain tests are not proposed. A useful next diagnostic would
capture target, reachability and weapon scores across the warning-to-combat
transition, or compare against original Diego under controlled conditions.
Neither comparison has been performed; vanilla/mod attribution remains open.
Removing the bow or forcing melee would change the test configuration without
explaining the original mixed-weapon behavior. No such tuning was applied.
