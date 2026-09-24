# B-only warning recovery, 0.1.12

The user requested a concrete mod fix after B remained stuck when fists were
lowered after the second completed warning. Original-code preservation does
not exonerate B's authored configuration. The failure is treated as a defect
of the current mod scenario while its precise trigger remains unproven.

`NpcWarningRecovery`0.1.12 is installed as the sole enabled Manager entry,
`npcwarningrecovery-4e0db7e6`, with status `in_sync`. On2026-09-10 the user
confirmed that the entire focused game checklist passed.

## Runtime result

First- and second-warning cleanup, saving nearby, normal behavior after loading,
and retaining an ongoing threat all passed. The user additionally loaded the
second-warning save and raised fists again: B immediately attacked. This confirms
that successful cleanup does not erase the observed escalation across save/load.
It does not establish an exact serialized "third warning" counter.

|Slot|Save name|Active marker|Recovery marker|
|---|---|---|---|
|59|`npc warnfix - erste`|1|1|
|60|`npc warnfix - zweite`|1|1|

The [runtime result](recovery-runtime-result.json) retains save fingerprints,
B's preserved routine and crime records, and the distinction between saved
markers and user-observed behavior. Both markers are present in both saves.
The saves were copied/read only. The subsequent
[equipped-sword test](../npc-weapon-voice/README.md) confirmed attack escalation
and B's combat-start speech. Its close-range bow selection remains a separate
open observation.

## Change

Only B's character definition selects a new human AI with a replacement mapping
for `AIState_Conflict_Warning_Threat`. It uses Diego's own parent class and retains
Diego's sole extra default: zombie-target scoring with weight10000. C explicitly
selects the original Diego AI so it does not inherit B's replacement. A's source
is unchanged.

Direct inheritance from the shipped Diego leaf was rejected because its
`__InitDefaults` is final. Using the same supported parent avoids changing
Diego or the shared AI module. This compiler restriction does not prevent
authoring a new NPC AI.

The isolated strict-standalone admission passed. Readback confirms inherited
ordinary-method slots66/75, direct ancestor calls for the two `Super` hooks,
and the inherited `AssessEndWarning` target without a subclass override. The
new AI's first six initializer instructions exactly retain Diego's weighted
scoring setup. Full-package and installed-cache verification also passed.

The [build and deployment result](recovery-build-result.json) records the exact
four-module scope and preservation of all7315 unselected original modules.
The installed cache differs from the compiled cache only in370 normalized
function-ID fields; [byte-level comparison](recovery-installed-canonicalization.json)
verifies that executable code, metadata and reference tables are unchanged.
Voice, localization, original backups and the recorded save baselines through58
remain unchanged. The installed [new source](warning-recovery.as) is retained
alongside the report.

The new warning state inherits all normal warning speech, perception and
escalation behavior. Its ordinary script-loop hook retries the shipped
`AssessEndWarning` only when all these conditions hold:

- The state is not already exiting.
- End assessment was already marked done.
- There is no character of interest and the warned-character list is empty.
- The AI senses no living enemies.

All other cases run the inherited loop hook. This uses the existing end
assessment and normal conflict cleanup, rather than immediately teleporting,
resetting B or suppressing threat detection. No original crime/warning module
is rewritten. No activity schedule, warning deadline or voice recording changes.

Two saved markers support the next game result:

|Marker|Meaning|
|---|---|
|`gore_b_warning_fix_v1_active = 1`|The new warning state entered its `BeginWarning` hook.|
|`gore_b_warning_fix_v1_recovered = 1`|The guarded recovery path requested regular end assessment.|

The second marker does not alone prove successful cleanup. Its write is limited
to once per state instance; that limit does not suppress assessment retries.
Existing-save activation is now supported by those saved markers and the user's
successful test, rather than inferred from the changed definition alone.

## Focused game check

Completed by the user on2026-09-10. For reproduction, use original54,
`npc sitz-wache - wache`, and do not choose setup21.

1. Stand in front of B outside the hut and raise **bare fists**. After warning1,
   lower them. B should end the warning and saving should be possible while
   remaining nearby. Save separately as **`npc warnfix - erste`**.
2. Reload original54. Raise fists again and let warning2 finish completely,
   then lower them. Stay close enough to keep watching B; do not resolve it by
   walking away. Expect the warning to end and saving to become possible.
   Save separately as **`npc warnfix - zweite`**.
3. Load the second new save and confirm B resumes normal watch. For a short
   control from original54, leave fists raised through the warnings: the fix
   must not make B abandon an ongoing threat. Reload54 after that control.

If step2 still hangs, move away only after observing that failure, then save as
**`npc warnfix - weiterhin haenger`**. The markers help distinguish whether the
custom state loaded and whether its recovery condition was reached. The user
reports which NPC spoke; another nearby speaker does not establish B's result.
No specific recording ID or combat outcome is presumed.
