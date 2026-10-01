# Fists warning: original failure and tested correction

The0.1.11 failure investigated below was corrected by0.1.12. On2026-09-10
the user confirmed cleanup after both warnings, nearby saving and persistent
escalation after reloading. See [the correction](RECOVERY.md) and
[saves59/60](recovery-runtime-result.json). The following audit preserves the
historical failure and its evidence.

On2026-09-09 the user tested the installed `NpcSeatGuardActivities`0.1.11.
The input was **bare fists**, lowered **after the second spoken line had fully
finished**. It was not an equipped-melee-weapon test. The user performs all
game testing; the agent only copied/read the resulting saves.

## Observed result

- B notices the threat and speaks naturally, although detection requires time
  and positioning. Other nearby NPCs also respond inconsistently.
- Lowering fists after the first warning correctly ends B's warning.
- After the second warning, lowering fists leaves B in the warning pose and
  prevents saving. Moving farther away allows saving; returning finds B normal.
- Another tested NPC recovered correctly. The user recalls similar vanilla
  failures, but that does not establish this particular failure as vanilla-only.

This is a **partial pass**: natural reaction voice and the first cleanup work;
the second cleanup is an open behavioral defect. Shipped code is not presumed
correct merely because it is original.

## Save evidence

[runtime-result.json](runtime-result.json) records exact fingerprints and joined
B crime records from unchanged original saves:

|Slot|Name|Saved clock|
|---|---|---|
|57|npc voice - waffenwarnung|day12 12:11:06|
|58|npc voice - waffenwarnung 2tes mal|day12 12:27:43|

Both retain one living B, the same watch position and
`DailyRoutine_GoreSeatGuardActivities`. Joined threat records use
`Crime.DirectThreat.Fists`, consistent with the user's clarification. Some
relative records in57 have no matching global record in that snapshot; their
crime type is not inferred. The two saves contain different crime histories,
so they are not a controlled before/after snapshot of one live warning state.

Neither save captures the stuck moment: saving was possible only after recovery
or moving away. They cannot establish the active state, warning count, group
leader, end-assessment flag or exact save-blocking condition at failure time.

## Decompiler exposure audit

The [fresh installed-cache audit](live-cache-audit.json) was rerun against the
actual game cache and original backup on2026-09-09. All7,315 unselected original
modules are byte-identical. This includes the human/Diego AI, threat perception,
crime assessment, warning state and warning-group machinery. Their behavior was
not replaced by recompiling the decompiler's output.

Only these four modules are authored in this package:

1. New `GORE_TEST_A` — dialogue/test controls and head support.
2. New `GORE_TEST_B` — NPC definition, appearance and routines.
3. Original `CharacterDefinition_OC_STT_Diego` — the existing health test edit.
4. Original `LevelScripts.XardasTower_AI` — the existing test-spawn hooks.

The installed/full-cache difference is confined to359 `Function.Id` fields;
every other byte, including all seven reference tables, is unchanged between
the compiled full cache and the installed cache. Original-to-full reference
preservation and selected-module findings are recorded in the
[reference audit](original-reference-preservation.json). All seven tables retain
their original entry regions as exact prefixes, with no duplicate keys in the
keyed tables. This closes a separate risk: unchanged bytecode must still
resolve to its original targets.

The two modified original modules were checked separately:

- Diego: the two intended health constants are540→1234. After canceling only
  those operands in an in-memory comparison, the byte-semantic oracle reports
  four identical functions and one benign reference/temporary-slot difference.
  No additional semantic difference remains in that comparison.
- Tower: fourteen functions are identical. The two changed callbacks each add
  the intended12-operation NPC spawn prefix, then retain the original13-operation
  warning-flock call sequence and arguments with temporary-slot renumbering.
  The [complete normalized diff](tower-normalized-diff.txt) retains both sides;
  its two semantic differences are intentional additions, not a clean-oracle claim.

All21 original functions in those two modules also retain exact function traits,
method-table membership and complete serialized UFunction metadata. The audit
found no relevant decompiler drift explaining the observed cleanup failure;
this is a scoped finding, not a blanket claim of decompiler correctness.

B uses the inherited Diego AI, Brave personality, ShadowLeader affiliation and
level100. B has no authored warning-state override. Those choices and its saved
history can affect reactions compared with another NPC. Preservation of the
original warning machinery does not prove every authored behavior correct or
identify the runtime cause by itself.

## Original-code cleanup candidate

The relevant emitted source is
`AI/States/FightAI/WarningState/AIState_Warning_Crime.as`. Fists direct-threat
responses select the same `AIState_Conflict_Warning_Threat` state as weapons;
the class name `UAIState_Warning_Crime_Human_WeaponDrawn` does not exclude fists.

The [original disassembly](original-warning-loop.disasm.txt), independently of
source reconstruction, contains this control-flow pattern:

- Word PCs393–401 dispatch `AIEvent_Threat_AllWarningsDone` through `AssessEvent`.
- PCs404–408 then write `true` to `bEndAssessmentDone`.
- PCs421–470 require that same flag to be false before the no-target/no-enemy
  branch can call `AssessEndWarning` at473.

Thus **if** the final assessment leaves the NPC in the same warning state, later
lowering fists could remove the target while the ordinary cleanup path remains
gated off. The too-close assessment has a similar source-level latch write.
This is a concrete candidate in original code, not proof that the user's live
NPC entered that exact path.

The warning count/timer update follows the speech request; the script does not
explicitly await completion of the recording. The next deadline is six real
seconds after the update. Finishing the spoken line therefore does not tell us
whether final assessment already ran. Separate perception/lost-awareness rules
can leave warning independently, consistent with distance recovery but not
proof of its exact mechanism here.

No speculative cleanup override, global crime-system rewrite or new deployment
was made. The next diagnostic must observe the stuck live state and whether
final assessment actually changed it; these post-recovery saves cannot supply
that information. Any fix must preserve normal warning escalation and avoid
recompiling unrelated original AI modules. Equipped weapons, combat lines,
everyday lines and another voice profile remain separate qualification work.

## Follow-up: concrete mod correction

After the user requested action,0.1.12 adds a narrowly guarded recovery in B's
own derived warning state. It retries the normal end assessment when the old
assessment latch remains set despite an empty warning state without sensed
enemies. C keeps its original AI. Two saved markers distinguish activation and
recovery attempts; neither substitutes for observing a successful exit.
The previous paragraph describes the completed audit stage, not the current
implementation status. See [the correction and test](RECOVERY.md); build and
runtime results are recorded there as they become available.
