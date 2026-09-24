# Natural trespassing reaction on B

The user completed the sitting/watch campaign and reported that B ignored entry
in `sitzen` and `abend`, but threatened the player to leave in `wache` and
`morgen`. B was seated in the morning as well. This is an observed natural voice
trigger on the invented identity, separate from the passed object/routine test.

## Saved evidence

[trespassing-result.json](trespassing-result.json) compares copies of saves53–56.
All four contain a `Crime.Trespassing.OnPerson` record for Hero in
`Area.OldCamp.OuterRing.Hut.31`, whose victim/owner is
`OC_VLK_Digger26_531-OC_Spawnpoint_Digger26_531`. It is not B's house.

Only56 contains B's own relative-crime witness entries. Their two IDs join to
two additional global trespassing records for that hut. B's record stores
`Neutral` toward Hero and `ProtectFromDifferentGuild` toward the owner. That
supports B protecting another camp member's home; it does not show a permanent
hostile relationship toward Hero. The records are marked forgiven by save time.

The absence of B's saved witness entry in54 does not invalidate the user-heard
warning associated with `wache`: these saves are snapshots, not a transcript of
every perception or warning. The owner has effectively the same saved position
in53,54 and56, so owner position alone does not explain the different responses.

## Shipped script path

References below are paths within the verified shipped source tree, retained
locally under `work/npc-seat-guard/tree`; no original module was changed here.

- `AI/AIAgent/Human/Config/OC_VLK_Digger26_531/DailyRoutine_OC_VLK_Digger26_531.as:18`
  assigns Hut31 to Digger26_531. Its schedule changes eating, darts, smalltalk
  and sleeping; B sitting on this stool does not transfer that ownership.
- `GAS/PerceptionEventMixins.as:1037` configures sight-based area perception,
  periodic checks and updates when the observed character changes area.
- `AI/AssessmentResponseSystem/AssessmentResponseModules/Crime/AIARM_Crime_Trespassing.as:21`
  gates assessment on conversation, existing warnings and defeated/ignored
  states. Neither `Sit` nor `GuardWatch` is an explicit permission toggle.
- `AI/AssessmentResponseSystem/CrimeProcessingSubsystem/CrimeProcessingSubsystem.as:2994`
  implements `IsIgnoringHomeTrespassing`: an owner at home, not scheduled to
  sleep and outside conflict can tolerate entry. The context calculation also
  considers ownership, relationships and protection of that owner.
- `AI/States/FightAI/WarningState/AIState_Warning_Crime.as:681` requires visibility
  for warning speech and can yield to a closer visible warning-group leader.
  The trespassing state's `GetWarningVoicelineTag` at1185 selects
  `Sound_Voice_Reaction_Warning_Intruder`, then `Warning_InsistIntruder`.

B retains human AI, Old Camp affiliation and the Diego Voice05 subset. The
fixture schedules activities and exposes clock controls; it does not explicitly
request these intrusion lines. Hearing the warning therefore qualifies one
natural reaction case. The exact recording/subtitle ID was not captured.

## Interpretation and remaining limit

The behavior is consistent with the normal crime/perception system, rather
than evidence of a broken sitting routine. Visibility, event timing, owner
context and existing warning/conflict state can change the outcome. The exact
cause of the four-way difference is **not established** by these saved records;
do not attribute it solely to time of day, owner presence or B's posture.

No AI behavior was suppressed and no new package was deployed for this finding.
Natural everyday/combat lines, a second voice profile and their restart cases
remain separate work in the [NPC backlog](../../../docs/guide/npc-open-items.md).

## Next focused game check: drawn weapon

This used the already installed0.1.11 package. On2026-09-09 the user clarified
that the actual input was **bare fists**: first warning/voice/cleanup passed,
but lowering fists after the second completed line left B stuck in warning
and blocked saving until the user moved away. See the
[separate result and cache audit](../npc-weapon-warning/README.md).
The original proposed equipped-weapon checklist below remains historical;
equipped-weapon coverage is not claimed from this fists test.

1. Load54, `npc sitz-wache - wache`, without choosing setup21. Stay outside the
   hut, about4m in front of B with clear sight and no active conversation/warning.
2. Draw a melee weapon without attacking or approaching. Wait up to10 seconds.
   The intended natural reaction is a demand to put the weapon away. Identify
   whether B or another guard actually speaks; another speaker does not pass B.
   Sheathe after the first response, or after10 seconds if no response.
3. Save separately as `npc voice - waffenwarnung`. Reload the original54 and
   repeat once to check the reaction without a fresh setup. Report the speaker,
   audible response and whether the warning stops after sheathing.

The shipped rule is
`AI/AssessmentResponseSystem/AssessmentResponseModules/Crime/AIARM_Crime_WeaponOrFistsDrawn.as:54`;
it requires a perceived threatening weapon and a witness that pursues the crime.
The warning state at `AIState_Warning_Crime.as:728` selects
`Sound_Voice_Reaction_Warning_WeaponDown` and later `Warning_InsistWeaponDown`.
Relationships, existing warning groups and perception can affect the result.
This check does not establish combat lines, ambient lines or another profile.
