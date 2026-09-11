# Appearance/routine game results — 2026-09-06/07

## Reading, drinking and schedule passed — 2026-09-07

The user confirmed the complete `NpcActivitiesEventFix` 0.1.4 test worked as
expected: B read after setup, walked to the noon destination and drank, resumed
drinking after a full restart without setup, walked back and read in the evening,
and continued reading the next morning. The event-declaration correction below
therefore has both binary metadata verification and a successful game test.

Read-only inspection resolves the saved names and clocks:

| Slot | Saved name | Saved clock |
|---|---|---|
|040|`npc aktivitaet fix - mittag`|Day7,12:07:27|
|041|`npc aktivitaet fix - abend`|Day7,18:05:01|
|042|`npc aktivitaet fix - morgen`|Day8,08:03:33|

B carries `DailyRoutine_GORE_TEST_B_Start` in all three. His noon position is
`(124366.5795, -144728.8354, -778.9495)`; evening and morning share the returned
position `(124245.0800, -145123.6490, -721.4803)`. A remains in place; C remains
unique with the same global identity and effectively unchanged position
(less than0.2 cm vertical settling). Reports, SHA-256 fingerprints and
byte-identical save copies are under `work/npc-activities-event-fix/evidence`.
Saved positions corroborate the resulting state; visible animation, walking
and restart continuation are established by the user's test report.

The user also observed short repeated activities: B takes out the book or
bottle, uses it for about three seconds, puts it away and starts again. This
matches the authored loop in [npc-b-c.as](npc-b-c.as):
`TryInteractionWithoutSpot(..., 20.0f)` followed by `WaitSeconds(2.0f)`.
The native wrapper names the third parameter `TimeLimitSeconds`
(`AI/NativeAICommands.as:265–267`);20 seconds is a maximum per call, not a forced
animation length. The selected `Action_Conversation_ReadBook` and
`Action_Conversation_Drink` actions can finish sooner. The observed three-second
length is not a value hardcoded by this fixture. This pass proves these repeated
free activities, not a continuous long reading animation or furniture-based tasks.

## Direct activities failed; event binding corrected — 2026-09-07

`NpcActivitiesProof` 0.1.3 retains the confirmed outdoor route and adds reading
at393, drinking at `FP_XT_WAIT_OUTSIDE`, then reading back at393. Unlike the
failed ambient actions, `Action_Conversation_ReadBook` and
`Action_Conversation_Drink` explicitly inherit `bPossibleAnywhere=true` and
human-character registration in `AI/Interactions/AbilityTask_Interaction_Human.as`
(base57–61; reading105–108; drinking153–156). They do not declare inventory,
conversation-state or furniture requirements.

The state combines `GotoPreferredLocation` with `TryInteractionWithoutSpot`,
uses bounded20-second interactions and pauses, and follows the shipped
graceful-exit implementation from `AI/States/AIState_UseFreepoint.as:108–112`.
Simulated steps remain enabled, as on the shipped base and direct-interaction
state in `AI/AssessmentResponseSystem/EventResponses.as:757–777`. No persistent
executor or additional runtime infrastructure is introduced. The evening
control advances to17:59 for a natural18:00 switch.

The user completed the test and observed neither reading, drinking nor any
walking. B stayed at the setup point throughout, including after the restart.
Read-only inspection resolves the new saves:

| Slot | Saved name | Saved clock |
|---|---|---|
|037|`npc aktivitaet - trinken`|Day5,12:04:09|
|038|`npc aktivitaet - abend`|Day5,18:02:17|
|039|`npc aktivitaet - morgen`|Day6,08:04:02|

All three preserve B's routine and exactly the same location:
`(124206.85177647056, -145252.6775584904, -692.6238820446374)`.
A/C remain in place and C remains unique. Reports and byte-identical copies are
under `work/npc-activities/evidence`.

The compiled cache establishes the authoring error: newly authored
`UAIState_GoreScheduledActivity` declared bare `UFUNCTION()` methods named
`DoTask_Implementation` and `OnGracefulExitRequested_Implementation`. Both were
serialized with those suffixed Unreal names and flags
`BlueprintCallable=1, BlueprintOverride=0, BlueprintEvent=0`. They therefore had
no native event binding. The previously proven `UAIState_TestGotoWP` was an
existing shipping class whose original reflection metadata was restored.

Version0.1.4 changes only those two declarations to
`UFUNCTION(BlueprintOverride) void DoTask()` and
`UFUNCTION(BlueprintOverride) void OnGracefulExitRequested()`.
The compiler appends the implementation suffix itself and emits the correct
unsuffixed Unreal names with override/event flags set. Activity bodies, route,
subclasses, setup, clocks, clothing and voice remain unchanged. The new mini
passed a direct binary check of the exact class and both linked method records.
This check catches the original error that emitted-source equality missed.

The corrected version subsequently passed the game test recorded above. The reproducible checker
and old/new metadata reports are under `work/npc-activities-event-fix`;
the general authoring rule is now in the [script guide](../../../docs/guide/scripts.md#engine-callbacks-in-new-classes).

## Direct walking passed — 2026-09-07

With `NpcRoutineWalkingProof` 0.1.2, the user confirms B visibly walked to the
expected point at the natural12:00 transition and then saved. This establishes
physical scheduled walking for the tested noon transition, independently of
direct clock-skip simulation.

The save is `G1R-036.sav`, public name `npc lauf - mittag`,1079167 bytes,
SHA-256 `0b6d330902d9477ded365f78a1778d63bcba42d2408901fe6908880d0502691d`.
Read-only inspection records Day4,12:02:43, B's
`DailyRoutine_GORE_TEST_B_Start`, assigned at08:00, and B at
`(124371.4656, -144720.4820, -781.1130)`, displaced from its recorded393
starting point. A remains at392 and exactly one C remains at391 with the same
identity as saves031–034. The user's observation establishes walking/arrival;
save coordinates and routine fields corroborate the resulting saved state.

Evidence and a byte-identical save copy are under
`work/npc-routine-second/evidence/036.json` and `evidence/saves/G1R-036.sav`.
This pass does not separately qualify evening/morning return behavior or a
restart from036. Guard/drink animations and independent NPC head/clothing
recombination remain outside this proof. No game was launched by the agent.

## Previous ambient-task fixture — 2026-09-06

BuildID 24878692, package `NpcAppearanceRoutineFix` 0.1.1. The user performed
all game actions. Four screenshots show C's Novice clothes outside and B's
Guard clothes inside Xardas' tower, both with Hero heads. The user confirms
the appearance, loading, a single C after repeated setup, and an old voice
option. This is positive evidence for the explicit `MO_Player` clothing path.

Scheduled walking did **not** pass. After setup, B stood inside; noon caused
no observed walking; evening moved B instantly outside. Loading preserved
that state; the morning control caused no visible move. Repeating setup moved
B again. Clock changes and routine assignment do persist, but those facts do
not establish correct task selection or physical navigation.

Read-only inspection resolves the saves by their actual public names:

| Slot | Saved name | Saved game clock | B's saved horizontal location |
|---|---|---|---|
| 031 | `NPC Routine - Mittag` | Day 1, 12:09:50 | NavigationSupport395 |
| 032 | `npc routine - abend` | Day 1, 18:07:16 | NavigationSupport394 |
| 033 | `npc routine - morgens` | Day 2, 08:02:35 | NavigationSupport394 |
| 034 | `aufbau erneut` | Day 3, 08:08:25 | NavigationSupport393 |

Every save has B's `DailyRoutine_GORE_TEST_B_Start` and exactly one C, with
the same C GlobalId:
`GORE_TEST_C-Character_GORE_TEST_A-WP_WarningFlock_XT_01`.
A remains at392 and C at391. B's `UsedSpot` is `None`; its routine's recorded
starting position is393. Save bytes alone cannot prove a full process restart;
this report only attributes loading to the latest user account.

The live `InteractionSpots.json` gives393/394/395 no supported actions. The
original Guard/StandAround/Drink tasks search ambient interaction spots.
`FP_XT_WAIT_OUTSIDE` supports StandAround but is about125 cm from394, outside
the configured100 cm search radius. This is a concrete fixture mismatch;
it does not fully explain the stale/wrong targets in the saves.

The subsequent focused fixture uses the shipped `UAIState_TestGotoWP`, which calls
`GotoPreferredLocation` repeatedly, and the outdoor route393 →
`FP_XT_WAIT_OUTSIDE` →393. Its natural noon walking proof passed on2026-09-07,
as recorded above. Guard/drink animations are not claimed by this test.

## Heads and faces

The test explicitly selects `MO_Player`, `Person=Hero`, `Head=Head_01`.
It therefore does not demonstrate independent NPC face customization.
Shipped `NpcHumansVisual.as` exposes eye/skin/hair color, hair, eyebrows,
beard, body and head fields. `NpcVisualLibrary.as` contains generic and named
NPC presets, but these usually select a prebaked complete character mesh.

The base game's UTOC Directory Index contains `MO_Player`,
`/Game/Assets/Characters/GTO/GTO_Male64`, `GTO_OC_STT_Diego` and
`GTO_NC_ORG_Drax_819`. It has no indexed `MO_Characters` path. This was a
metadata-only read of the header and Directory Index. A subsequent exact
extraction of `GTO_Male64` succeeded and identified `/Script/GateShadow.Fggto`.
The inspector stopped at an unsupported `FilePath` structure (byte14), with
no decoded/editable fields. Its contents therefore do not establish independent
head/hair control. Existing complete NPC looks are reusable; combining an
independently changed NPC face/hair with the tested modular clothes remains
unproven. Exact extraction evidence is in
`work/npc-routine-second/head-inspect/gore-asset-extract.json`.

Further native-header inspection identifies `UFggto` as facial rig/animation
data (expression/joint maps, rig path, skeleton and quaternion rotations),
consumed by the GateShadow animation node. It is not evidence of a head mesh.

An exact extraction of
`/Game/Assets/Characters/Humans/TierC/Flex/SK_OC_IE_Flex_Head` succeeded. Export560
is an `Engine.SkeletalMesh`; the package also has559 facial morph targets. Both
this package and `MO_Player` name
`/Game/Assets/Characters/Humans/Main/NHero/SK_NH_Ch1_Skeleton`. This supports a
separate-head compatibility experiment, but does not prove matching geometry
or bone mappings. Root mesh inspection stops at unsupported nonzero
`SkeletalMeshLODInfo` at byte18, so generated material-section IDs are unknown.

The cooked player model lists `Head_01` and `Head_02` for `Head`; no explicit
disabled/None choice was found. Its script default `Head="None"` alone does
not prove head removal. Body/head/hair/eye material references are distinguishable,
but section order must not be inferred from those names. Current script bindings
expose component creation, skeletal-mesh assignment, leader-pose synchronization,
material-section visibility and skeleton/material inspection. A future probe
must identify the actual generated head sections, preserve the body, and handle
character respawns/Mutable mesh updates before combining Flex with the clothes.
Hiding a head bone would also collapse a follower's corresponding bones and is
not a valid substitute. No head replacement was deployed in0.1.3.

The bindings expose `ShowMaterialSection(MaterialID, SectionIndex, bShow,
LODIndex)` but no complete skeletal section/material map. This missing map alone
does not prove that head hiding is blocked: Epic's
[API description](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/Components/USkinnedMeshComponent/ShowMaterialSection?application_version=5.5)
describes material-ID visibility and leaves `SectionIndex` undocumented. Whether
that parameter matters in this game's implementation remains unverified.
Current compiler stubs have no native addresses, and an older wrapper address
lacks a matching module/build mapping. No native behavior is inferred from it.

Exact package/provenance files are under
`work/npc-routine-second/head-inspect/flex-head`; source lifecycle references
include `Gameplay/Magic/Components/Spell/Summon/SummonManagerComponent.as:36/88`
(`OnCharacterSpawned`) and the bound Mutable instance `UpdatedDelegate`.

Local save copies, compact reports and hashes are under
`work/npc-routine-second/evidence`. Screenshot sources are the four clipboard
images attached to the user's result message. No game was launched by the agent.
