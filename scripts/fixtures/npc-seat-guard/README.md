# Sitting and stationary watch

This follows the passed bed/alchemy campaign, saves49–52. Version0.1.11 adds
choices21–25 and two routine classes while retaining the exact previous A/B
source as a prefix. `NpcSeatGuardActivities` **0.1.11** was installed as the only
enabled Manager entry (`npcseatguardactivities-53a423a8`, `in_sync`). It has since
been replaced by [0.1.12's B-only warning correction](../npc-weapon-warning/RECOVERY.md),
which retains these activities. The user
completed the full game checklist successfully on2026-09-08. Saves53–56 and
the distinction between saved facts and observed behavior are recorded in
[runtime-result.json](runtime-result.json).

The [build result](build-result.json) records the successful standalone build,
exact four-module scope, source preservation and installed event bindings.
All7,315 unselected original modules retain their exact bytes. The Manager
changes only359 function-ID fields; [byte-level verification](installed-canonicalization-checks.json)
confirms executable code, metadata, reference tables and every other byte match
the compiled cache. Existing voice/localization, original backups and recorded
save baselines through52 remain unchanged.

## Targets and behavior

| Purpose | Spot | State |
|---|---|---|
| B's chair | `IO_OC_CHAIR_81` | Shipped `UAIState_Sit` |
| B's watch point and initial position | `BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5` | Shipped `UAIState_GuardWatch` |
| A's observation position | `WP_OC_NorthGate` | Navigation-only `UAIState_Stand` |
| Hero arrival | `WP_Guide_Path_FromTo_SwampCamp_2` | Shipped waypoint near the north gate |

Chair and watch point are6.5m apart; the player arrives about15m from A.
All four spots have no custom interaction restrictions or data layer. This does
not mean the surrounding room is public: the stool is **inside a hut** belonging
to Digger26_531 (`Area.OldCamp.OuterRing.Hut.31`). Save52 has no stored references to either activity spot; Guard10
is elsewhere and patrols this point only23:00–07:25. The saved evidence is in
[spot-selection.json](spot-selection.json). The user found the stool inside the
hut and confirmed successful use. No stock NPC is changed.

B sits until12:00, watches until18:00, then sits through the next morning. The
routine has zero random offsets and teleport mode `Never`; only setup teleports.
GuardWatch is a stationary watch/idle or crossed-arms posture with occasional
small gestures, not a patrol or aggression test. Continuous movement is not
expected. The shipped state runs bounded interactions with pauses.

Choice21 validates A/B/Hero and all four named spots before setup. It moves B,
then Hero, then conversation owner A. Setup marker `gore_npc_seat_guard_setup_v1`
is1 while incomplete and2 when setup calls finish; it does not prove object use.
Choices22–24 require2; choice25 reports incomplete setup. New callbacks explicitly
declare `UFUNCTION(BlueprintOverride)`.

The [new routines](seat-guard-routines.as) and [new choices](seat-guard-choices.as)
are retained here. Shipped references are `AIState_DailyRoutine.as`
(`UAIState_Sit`, `UAIState_GuardWatch`), `AbilityTask_Interaction_Human.as`
(`UAbilityTask_Interaction_Human_GuardWatch`) and Guard10's night-watch routine.

## Game checklist

1. Load slot52 **`npc objekte - geladen`**. At A choose **21**. At the north gate,
   walk past A through the gate into the camp, then turn left behind the gate
   wall. The target is a small stool roughly8m from A (about7.5m left and4m
   farther into the camp from the arrival direction), **inside the hut**. It is not visible from
   every approach to A. B should walk to the stool and sit.
   Save **`npc sitz-wache - sitzen`**.
2. Choose **22**, end conversation and wait for12:00. B should leave the chair,
   walk about6m and enter the watch posture. Save **`npc sitz-wache - wache`**.
3. Fully restart and load that watch save **without21**. Check continued watch,
   then choose **23** and wait for18:00. B should return to the chair.
   Save **`npc sitz-wache - abend`**.
4. Choose **24**. At08:00 the next morning B should **continue sitting**;
   this is a continuation check, not a new activity. Save **`npc sitz-wache - morgen`**.

If a step fails, save the observed state and report the option. The agent derives
slot numbers from save names. Game testing remains with the user.

## Setup observation

The user initially saw A at the gate but neither B nor the stool. Save53
(`npc sitz-wache - sitzen`) confirms setup marker2, B alive with the new routine,
and B's horizontal position within0.56m of the catalog stool spot. This supports
successful placement; it does not prove the visible sitting animation. The
initial direction advice was reversed and corrected to **left** behind the gate.
The user subsequently found the stool inside the hut and confirmed all steps.

## Passed game result

| Slot | Save suffix | Saved clock | User-confirmed state |
|---|---|---|---|
|53|sitzen|day12 08:49:01|B sits on the stool.|
|54|wache|day12 12:04:20|B walked to the watch point and watches.|
|55|abend|day12 18:02:16|After restart and the evening change, B returned to the stool.|
|56|morgen|day13 08:08:07|B continues sitting the next morning.|

All four saves retain one living B with `DailyRoutine_GoreSeatGuardActivities`.
The three seated positions match; the watch position matches its named spot.
Original saves were only copied/read and remain unchanged. No game code or
deployment change was needed to record this result.

The user also heard B warn about entering the hut during watch and the next
morning, but not during the first seated or evening phase. This is separately
recorded as a natural reaction-voice observation. [Trespassing analysis](TRESPASSING.md)
identifies the real owner and B's saved witness records; the exact reason for
the different reactions remains unresolved. Sitting does not disable that AI.
