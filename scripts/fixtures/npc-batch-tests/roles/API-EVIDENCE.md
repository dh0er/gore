# Source/API evidence

Pinned source baseline: `work/npc-warning-fix/tree`, exact pristine cache
`7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511`.
This verifies API shapes against shipped source; compiler admission and runtime
behavior remain separate checks. The fixture copies no decompiled combat loops.

| Fixture operation | Existing source evidence (relative to the tree) |
| --- | --- |
| Trader configuration | `Items/GenericItems/TradersGeneric.as`: named subclasses of `UTraderConfigBase`; generator `crates/gore/src/cmd/npc/generate.rs` uses this exact custom-NPC config pattern. |
| Stock seeding | `GAS/GASCharacterStateMixins.as:717`: `AddItemToInventory` forwards class/count/inventory to `AddItemOfClassInventory`. Exact build registration trace declares `EInventoryTypes::Trader=7` (do not confuse it with `EInventoryOpenedStates::TradingInventoryTrader=3`). |
| Trade UI | `AI/NativeAICommands.as:1140`: `StartTradingWith` wraps `TaskTradeWith`; `Story/G1R/Conversation/Conversation_OC_STT_FISK_311.as:711` calls it from a real trader choice. |
| Teacher requirements | `Story/Support/DialogTeaching.as:7`: `TryLearnSkill` checks LP, ore, already known, then calls `LearnSkill` with costs. `GAS/GASCharacterStateMixins.as:984` charges LP/ore only after successful effect application and memorizes the skill. |
| Skill/cost/persistence | `GAS/Effects/Skills/GE_Skills.as:907`: Diving has 5 LP / 30 ore and `GE_Persistent`. `HasLearnedSkill` checks the active skill effect. |
| Explicit test LP grant | `GAS/Effects/Skills/GE_Skills_Helpers.as:5`: `UGE_SkillPay_SP` is an additive set-by-caller modifier on SkillPoints. The fixture deliberately applies +5 through the existing effect; normal teaching uses negative cost. |
| Follow / stop / resume | `AI/DailyRoutines/DailyRoutines_Generic.as:5`: shipped `UDailyRoutine_Generic_FollowHero`; `GAS/GASCharacterStateMixins.as:1940` exchanges the NPC routine. Stand/Anywhere uses existing daily-routine and state APIs. |
| Combat and defeat/death goals | `AI/NativeAICommands.as:1519,1569`: `StartTrainingFight` vs `StartTrainingFightToDeath` use distinct stock events; no replaced conflict or damage implementation. |
| Personal hostility | `GAS/GASCharacterStateMixins.as:1088`: `SetRelationshipUntilDefeat`; `AI/NativeAICommands.as:1274` provides `ForceHearingPerception`. Registered `ERelationship::Enemy=1`. |
| Guild | `Story/Support/G1R/StoryFunctions.as:878`: `SetTrueGuild(UGE_Guild_None)`; B's unchanged definition explicitly supplies `UGE_Guild_Human_OldCamp_ShadowLeader`. |
| Flee policy | `AI/CharacterAI_Gothic.as:4,318`: enum and ordinary script field; `AI/AssessmentResponseSystem/AssessmentBits.as:1291` handles Always (4). The fixture saves/restores B's actual previous field. |
| Same-NPC timed revival | `AI/DailyRoutines/DailyRoutines_OG.as:437`: `RestoreAttributesAfterTimespan`, `TryReviveAfterTimespan`, `TryReviveIfDeathMemoryHasAnyOf`; `AI/States/FightAI/AIState_Conflict.as:243–265` records the three Hero-involved killed conflict tags. Application to a human via explicit routine exchange is the runtime experiment. |
| Weapon capture | `AI/CharacterAI_Gothic.as:264`: actual `SelectedItemAction`; `AI/States/FightAI/CombatState/AIState_OGCombat.as:1091`: equipped item read; `AI/AIItemScoring.as:223–238`: exact close-target prerequisite and reachability query. |
| Component tick | Existing tested B `UGoreFlexHeadProbeController : UActorComponent` provides the BlueprintOverride Tick/GetOrCreate pattern; the new tracer adds its own class and never moves existing fields. |

No novel engine registration, signature allowlist, template, game install or save
is changed by source preparation. Inherited native members may still need an
exact compiler admission check; source existence alone does not waive remap guards.
