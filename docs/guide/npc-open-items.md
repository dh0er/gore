# NPC modding: open work

Updated 2026-09-11. The user sets this order: **heads, objects, voice triggers,
NPC roles**. Game tests are performed by the user. Complete each focused change,
then batch offline checks and provide a short game checklist. An untested behavior
is not automatically a missing CLI capability.

## 1. Modular heads and faces — tested path passed

- [x] Combine the shipped Flex head with the tested modular Novice clothing.
- [x] Qualify the required native mesh API references through compilation and
  Manager composition. Exact sealed native declarations now pass both admission
  and final cache validation; unknown signatures still fail.
- [ ] Establish which face, hair, beard and color choices actually work through
  that path; document any limits separately.
- [x] Verify animation alignment, save/load, full restart and absence of duplicate
  or overlapping heads.

Current result: complete borrowed NPC looks work. Explicit Guard/Novice clothing
works on `MO_Player`, but retains the Hero head. A separate shipped Flex head has
been extracted. The user's `NpcHeadProbe` 0.1.5 test **failed visually**: head
geometry stretches severely when turning. Option 15 restores the Hero head.
Slot 43 (`npc kopf - wechsel`) confirms the asset loads and bone names match;
those checks do not establish pose compatibility. The visibility diagnostic
also reports only six hidden material/LOD pairs from 31 matched materials.
`NpcHeadVisibilityFix` 0.1.6 also failed visually. Slot 44 (`npc kopf - materialfix`)
confirms all 186 expected material/LOD pairs report hidden; option 15 still
restores the Hero head. Material hiding alone therefore does not solve the
deformation. The following experiment kept Flex in its own reference pose and
attached it rigidly to the animated body head bone, without facial animation.
`NpcHeadRigidFix` 0.1.7 is a partial success: the user confirms normal head shape,
restoration and repeated option 14, but the neck has an open gap. Slot 45
(`npc kopf - grundpose`) confirms rigid mode and complete material hiding.
The 0.1.8 experiment uses a poseable mesh: copy the body pose, then restore only
strict head descendants to Flex's local reference pose. Tick-order and recorded
head/neck position comparisons are included; runtime compatibility and boundary
geometry needed a game test. The user confirms a correct neck join in 0.1.8;
slot 46 (`npc kopf - halsanimation`) records running pose copies and near-zero
head/neck position errors. However, option 15 crashes after that save.
That failure required fixing restoration before testing repeated application.
The 0.1.9 correction excludes C from BFG tick optimization before creating the
poseable head. The crash dump shows a failed skeletal-component cast in that
optimizer's interpolation path. The user now confirms the complete 0.1.9
checklist passes: correct connection, restoration without a crash, repeated
application without duplicates, full restart and restoration after loading.
Slots 47 (`npc kopf - wiederhergestellt`) and 48 (`npc kopf - erneut`) confirm
saved modes 0 and 1. This qualifies the tested Flex/Novice combination; arbitrary
independent face, hair, beard and color choices remain open. Continue with objects.
See the [head test](../../scripts/fixtures/npc-head/README.md) and
[head investigation](../../scripts/fixtures/npc-appearance-routine/RESULTS.md#heads-and-faces).

## 2. Activities using objects — tested pairs passed

- [x] Demonstrate sleeping in a real bed and a representative work activity
  using an alchemy table.
- [x] Demonstrate sitting at a compatible real chair or stool.
- [x] Verify a guard activity at a compatible spot.
- [x] Verify scheduled arrival, activity exit on a time change and continuation
  after save/load or restart for the bed/alchemy and sitting/watch pairs.

Current result: scheduled walking, free reading/drinking, evening return and
restart continuation passed with `NpcActivitiesEventFix` 0.1.4. The initial
ambient fixture failed because its targets did not provide suitable actions.
The successful free actions do not qualify furniture or work objects.
`NpcObjectActivities` 0.1.10 passed the user's full game checklist: Xardas'
bedroom bed and alchemy table, scheduled walking/entry/exit, restart and continued
sleep at08:00 the next day. Saves49–52 retain the routine and matching positions
and clock. Alchemy qualifies object use/animations, not recipes or crafted items.
See the [object fixture](../../scripts/fixtures/npc-objects/README.md).
The [NpcSeatGuardActivities0.1.11](../../scripts/fixtures/npc-seat-guard/README.md)
checklist also passed: stool use inside a hut near the Old Camp north gate,
walking to stationary GuardWatch at noon, return at18:00, full restart and
continued sitting the next morning. Saves53–56 retain matching positions and
routine. GuardWatch is a posture with small gestures, not a patrol test.
The absence of custom restrictions on an interaction spot does not make its
surrounding room public; the hut belongs to Digger26_531.

## 3. Natural voice triggers — in progress

- [ ] Demonstrate naturally triggered everyday/routine lines on a new NPC.
- [x] Observe a naturally triggered trespassing warning spoken by invented B.
- [x] Observe B's natural threat warning with voice and cleanup after the first warning (bare fists).
- [x] Resolve the stuck warning after lowering fists following the second spoken warning; nearby saving and save/load now pass in0.1.12.
- [x] Preserve escalation after save/load: raising fists again after loading the second-warning save makes B attack immediately.
- [x] Demonstrate equipped-sword escalation and B's spoken combat-start line.
- [ ] Check an additional voice profile and persistence after restart.

Current result: existing/new recordings, subtitles, A/Hero speaker changes,
explicit generic requests, repeat, skip and restart passed. Deliberately invoking
a generic request does not prove every natural trigger. See the
[voice campaign](../../scripts/fixtures/npc-voice/RESULTS.md).
During0.1.11 the user heard B warn about entering Hut31, including while seated
the next morning. Save56 records B as a witness protecting Digger26_531, with
two matching Hero trespassing records. This qualifies that natural reaction;
it does not qualify every trigger or a second profile. The different warning
behavior across saves is [documented](../../scripts/fixtures/npc-seat-guard/TRESPASSING.md),
with its exact cause still unresolved.
The next check on installed0.1.11 was performed with **bare fists**. B's natural
voice and cleanup after the first warning passed; cleanup after the second
warning failed, including blocked saving until the user moved farther away.
Saves57/58 were made after saving became possible and cannot show the stuck
live state. The [warning investigation](../../scripts/fixtures/npc-weapon-warning/README.md)
records this partial pass and audits actual cache/reference preservation.
Original warning modules were not recompiled; a cleanup gate exists in original
bytecode, but its role in the observed failure is not yet established. Vanilla
correctness and blanket decompiler correctness are not assumed.
The [0.1.12 correction](../../scripts/fixtures/npc-weapon-warning/RECOVERY.md)
is now installed: only B receives a derived warning state that retries normal
end assessment for a latched state with no target, warned characters or sensed
living enemies. C retains its original AI. The user confirmed all focused tests
on2026-09-10; saves59/60 both contain activation and recovery markers. Cleanup,
nearby saving and normal watch after loading pass. Raising fists again after
loading save60 produces an immediate attack, confirming that escalation remains.
The exact internal warning counter is not inferred from the saved crime rows.
The [equipped-weapon/combat-voice test](../../scripts/fixtures/npc-weapon-voice/README.md)
uses separate slot61, `npc voice - waffentest start`: a copy of54 with one usable
old sword. On2026-09-10 the user confirmed B attacks and speaks his combat line.
Save62 is `npc voice - schwertwarnung`. No mod rebuild was needed, and the original
saves were retained. The latest report did not separately describe cleanup after
sheathing; the recorded checkpoint is not proof of every cleanup step.
The user could not find the initially imported save. Core import defects in
duplicated public metadata and central slot registration were corrected, and61
was repaired with backups on2026-09-10. The subsequent game test confirms loading;
the retained source date may sort it beside54 rather than at the top.

## 4. NPC roles — queued

- [ ] Trader with actual stock and working buying/selling.
- [ ] Teacher with requirements, cost and persistent learned result.
- [ ] Companion/following behavior, including stopping and resuming.
- [ ] Combat, hostility/faction reactions and fleeing.
- [ ] Investigate B drawing a bow next to the Hero and sometimes switching to a
  sword before firing. Archer personality and both usable weapons are confirmed;
  the runtime cause remains open. See the
  [analysis](../../scripts/fixtures/npc-weapon-voice/WEAPON-SELECTION.md).
  Test64 (`npc kampf - B freie flaeche`) was prepared from the user's open-ground
  save63 on2026-09-11; B starts two metres ahead with his daily routine disabled.
  Weapons/combat AI are retained. The user reproduced it on2026-09-11: B draws
  the sword during the warning, then switches to his bow when combat starts.
  The hut is not required to reproduce it; runtime scoring and attribution remain open.
- [ ] Death/defeat and any explicitly configured respawn behavior.

These are focused role tests still to perform, not claims that the corresponding
engine features or script paths are absent. `npc new --trader` currently creates
an empty trader configuration; it does not by itself qualify a working shop.

## 5. Quest completion beyond the basic session fixture — pending

The [session campaign](../../scripts/fixtures/npc-session/RESULTS.md) passed
quest acceptance, active/completed journal presence, direct start/success calls,
stage-dependent dialogue and persistence across save/load and full restart.
It did not qualify every generated quest transition or a finished quest journal.

- [ ] Own questlog document and complete authored journal presentation. The
  session fixture retained `<GORE_TEST_A>` and unrelated vanilla letter text.
- [ ] Multiple objectives and automatic progression callbacks. The draft quest
  generator can emit subobjectives and availability/start/success/failure logic,
  but marks that broader generated behavior runtime-unqualified. The successful
  fixture invoked StartQuest/SucceedQuest directly from dialogue.
- [ ] Item hand-in and exactly-once quest rewards. A separately tested dialogue
  ore grant does not establish the complete quest reward workflow.
- [ ] Alternate branches and failure outcomes, with reload/restart checks at
  intermediate and terminal stages.

One focused quest with two objectives, item hand-in, a reward, an alternate
outcome and dedicated journal text can cover those related cases together.

## Dialog and voice boundaries

New conversations, same-module nested dialogue trees, rules, persisted effects,
speaker changes and new Vorbis recordings have game evidence. The current
[dialog guide](dialog-authoring.md#practical-limits-only) records constraints:
at most20 immediate choices per submenu; a required top-level Say between
successive new submenu transitions; a loaded settings anchor for first
conversations; and a complete-cache build for new cross-module dependencies.
These are authoring constraints, not evidence that ordinary dialogue creation
is missing. Other game builds and untested audio layouts are not qualified.
Voice publication uses Vorbis; line-specific lip sync is excluded by the user.

## Follow-up improvements

- [ ] CLI convenience for multiple schedule phases and activity selection. Today
  the CLI builds/deploys authored AngelScript; `--waypoint` generates one all-day
  task rather than a multi-phase routine. Add convenience only around proven paths.
- [ ] Optional longer, more natural activity animations. The current test repeats
  short reading/drinking actions with a two-second pause; this is expected.

Accurate line-specific lip sync is **excluded by the user**.

## Already verified

New NPC identity/body/spawn; existing complete appearances; modular clothing;
dialogue/quest progress; persistent knowledge and a Friend relationship; new
voice recordings and subtitle IDs; scheduled walks and free reading/drinking;
the tested separate Flex head with modular clothing, restore/reapply and restart;
the tested save/load and full-restart cases. A shipped Diego health edit also
passed. See the [NPC guide](npc-authoring.md#what-is-proven-and-what-is-not)
for scope and linked evidence.
