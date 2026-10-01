# Separate NPC head with modular clothes

This is the focused head campaign after the successful 0.1.4 activity test.
`NpcHeadProbe` 0.1.5 and `NpcHeadVisibilityFix` 0.1.6 failed visually.
`NpcHeadRigidFix` 0.1.7 fixed the shape but left an open neck gap.
`NpcHeadNeckFix` 0.1.8 fixes the visible neck join, but crashes on option 15.
`NpcHeadRestoreFix` 0.1.9 passes the complete user checklist after excluding C
from the game's tick optimizer.
These target only C's head. B keeps his verified
activities; quest, dialogue, voice and clothing content are retained.

## Scope

Attach the shipped `SK_OC_IE_Flex_Head` mesh to C's existing modular Novice body.
The current experiment copies the body's animated pose into a separate poseable
mesh and resets strict head descendants to Flex's reference pose. Hide only original head-part material slots
identified through the actual material's parent chain. No slot ordering or
head-bone hiding is assumed. Existing material visibility is retained for the
restore option. This is an experiment, not a claim of arbitrary face editing.
The current [head controller source](head-component.as) is retained with this fixture.
Without a saved selection, the original Hero head stays visible until option 14.

A persistent-state controller watches visual-character spawn. C's scheduled
state reconstructs it on entry, including after loading, and refreshes it in
its two-second loop to cover later mesh generation. The current attachment
requires a head bone on both meshes and a valid Flex parent chain. The completed
0.1.9 user checklist below qualifies save/load and visual compatibility for this
particular head/body combination.

## User result: 0.1.5 failed visually (2026-09-08)

The user reports severe vertical stretching around the face/hair/neck, varying
as C turns. Both supplied screenshots show this. Option **15** successfully
restores the original Hero head. This does not qualify the replacement head.

Slot **43 `npc kopf - wechsel`** was read from a copy; its original was unchanged.
Saved diagnostics: `mode=1`, `loaded=1`, `bones=1`, `matched=31`, `hidden=6`,
`status=4`. Here `bones` means only that each head bone name exists on the body;
it does not check hierarchy, reference pose or the active LOD. `hidden` counts
material/LOD pairs, not complete head parts. The small count also requires
investigating section remapping before attributing everything to animation.
Save and screenshot fingerprints are retained in [runtime-result.json](runtime-result.json).
Full-restart appearance and duplicate prevention remain unverified.

## Next correction: 0.1.6

The controller now tests `SectionIndex=-1` when hiding/restoring the already
resolved material IDs, intended to bypass a section-zero LOD remap. The precise
native implementation is not available locally for this game build; this
argument's effect still needs the next runtime diagnostic. It also records
`gore_head_v2_expected`, and status 4 requires every requested material/LOD pair
to report hidden. Pose sharing is unchanged to isolate this visibility issue.
This correction is not yet evidence that the stretching is fixed.

The bone-name check remains deliberately limited. Epic's
[modular-character documentation](https://dev.epicgames.com/documentation/unreal-engine/working-with-modular-characters-in-unreal-engine?lang=en-US)
requires a matching bone structure for Leader Pose; sharing names and a skeleton
asset alone does not prove this. The extracted Flex asset has its own hair and
haircap material references, so the screenshots alone cannot establish whether
the stretched parts belong to Flex or the original Hero mesh.

## User result: 0.1.6 still distorted (2026-09-08)

Slot **44 `npc kopf - materialfix`** confirms `hidden=expected=186`, compared
with only six hidden pairs in 0.1.5. All intended visibility queries therefore
pass, but the replacement still stretches severely. Option **15** still restores
the Hero head. [visibility-runtime-result.json](visibility-runtime-result.json)
records the save and three screenshots; the original save was unchanged.

## Next correction: 0.1.7

Clear Leader Pose and enable `SetRenderStatic(true)` to render Flex in its own
reference pose. Accumulate Flex's local reference transforms from `head` through
its parents to the root; attach the component to the body's `head` bone with the
inverse accumulated transform. The offset comes from the asset, not an estimated
height. A bounded parent walk must reach the root before the head is shown.

This follows the documented [reference-pose rendering API](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/Components/USkinnedMeshComponent?application_version=5.5)
and [transform multiplication order](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Core/Math/TTransform/op_mul/2?application_version=5.5).
The three additional mesh API signatures and two transform operations are
qualified in [native-rigid-api-qualification-v2.json](native-rigid-api-qualification-v2.json).
The standalone B-module compilation passed. Its five emitted native identities,
all datatype flags and reference links match the sealed declarations; see
[rigid-compiled-native-qualification.json](rigid-compiled-native-qualification.json).
The face, neck and hair are rigid: no facial animation is transferred. Visual
shape, neck seams and load/restart behavior still need the user's test.

### Previous 0.1.7 game check

`NpcHeadRigidFix` 0.1.7 was installed as the only enabled Manager entry
(`npcheadrigidfix-1c3c4149`, `in_sync`). The installed script cache exactly matches
the composed full artifact. Four native-qualification tests and the guarded
four-module composition passed; A/Diego/Tower emit identically and all 7,315
unselected pristine modules retain their original bytes. Readback also confirms
event bindings, reference-transform accumulation/inversion, voice/localization,
original backups and save baselines through slot 44. See
[rigid-build-result.json](rigid-build-result.json). The user result below supersedes the pending appearance check.

1. Load **Slot 44 `npc kopf - materialfix`**. Its saved mode already selects the
   replacement. Inspect C from the front, side and back, including when turning.
   No repeated setup is needed.
2. Save as **`npc kopf - grundpose`**, even if the result is still wrong, then
   choose **15** to check restoration.
3. If shape and neck fit look normal, choose **14** twice and check for duplicate
   parts. Save again under that name, fully restart, and load without 09 or 14.

Expect a rigid face with the head following body movement. Facial animation is
outside this experiment; failure to align the head/neck is still a failure.

## User result: 0.1.7 shape fixed, neck join open (2026-09-08)

The user confirms normal head shape, successful option 15 restoration and two
applications of option 14 without duplicates. A visible gap separates the
replacement head/neck from the body, including while turning and gesturing.
Slot **45 `npc kopf - grundpose`** confirms `rigid=1` and
`hidden=expected=186`. The original save was unchanged. Evidence is retained in
[rigid-runtime-result.json](rigid-runtime-result.json). Full restart was not
established; the head/clothing combination is still not qualified.

## Next experiment: 0.1.8 neck skinning

Use a separately named `UPoseableMeshComponent` attached to the body component
with identity transform, hiding the old rigid component. Each controller Tick
copies the body pose, then resets only strict descendants of `head` to their own
reference transforms. The intended result is animated spine/neck/head bones with
an undeformed, non-lipsynced face. No guessed height or scale offset is applied.

Tick prerequisites order body animation, controller writes and poseable refresh.
Before the next copy, the controller compares the head/neck component-space
positions with the values expected from the previous frame. Saved `gore_head_v4_*`
values record activation, copied frames, facial-bone count and position errors.
These diagnose pose transfer; they cannot establish matching boundary geometry.
Epic's [PoseableMesh API](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/Components/UPoseableMeshComponent?application_version=5.5)
exposes copy/reset and local bone storage, but does not establish this particular
cross-mesh combination. Runtime behavior therefore still needs the user's test.

The eight additional native signatures and three type identities are recorded in
[native-poseable-api-qualification.json](native-poseable-api-qualification.json).
The final standalone B module passes their exact datatype/reference checks in
[neck-compiled-native-qualification.json](neck-compiled-native-qualification.json).
The existing controller fields retain their original order; new pose fields are
appended so replacement does not alias an old field's cache key.

### User result: 0.1.8 neck join passes, restoration crashes

The user confirms that the head/body connection now fits. Slot **46
`npc kopf - halsanimation`**, saved before restoration, records 2,286 copied
frames, 194 facial descendants and head/neck position errors below 2e-11.
All 186 expected material/LOD pairs remain hidden. The original save is intact.
These diagnostics support pose transfer; the supplied images and user report
establish the visible result.

Selecting **15** after saving crashes with an access violation reading `0xd50`.
Restoration, repeated apply and restart are therefore not qualified for 0.1.8.
[neck-runtime-result.json](neck-runtime-result.json) retains the save diagnostics
and local screenshot/crash fingerprints.

### Correction: 0.1.9 optimizer exclusion

Offline minidump analysis identifies a failed `SkeletalMeshComponent` cast
followed by an unchecked null dereference in the BFG interpolation path. Its
producer accepts registered skinned components, including a broader type set
than the consumer handles. Exact PoseHead instance identity is absent from the
dump, so it remains the leading candidate rather than a recovered object.
The native instruction/class-layout evidence and limits are retained in
[restore-crash-analysis.json](restore-crash-analysis.json).

The correction calls the shipped
`UBFGTickOptimizerSystem::SetTickOptimizationRuntime_Enabled(Character, false)`
for C's visual actor before creating its poseable component. Original scripts
already use this API when setting up special character props. This excludes
only C from the optimization during the test, including after restoring the
Hero head; other characters retain their normal optimization. The pose-copy,
facial resets, material restoration and controller field layout are unchanged.
Runtime crash prevention was subsequently confirmed by the user's test below.

### User result: 0.1.9 complete checklist passed

The user confirms that everything now works: the connected head, restoration
with 15 without a crash, two applications of 14 without duplicates, full restart
and restoration after loading. Slot **47 `npc kopf - wiederhergestellt`** has
saved mode 0; slot **48 `npc kopf - erneut`** has mode 1, active pose copies,
194 facial descendants and all 186 intended material/LOD pairs hidden.
Both original saves remain unchanged. See
[restore-runtime-result.json](restore-runtime-result.json).

This establishes the shipped Flex head on the tested Novice body. Independent
face/hair/beard/color selection and accurate lip sync are not established.
The next campaign covers object activities; additional appearance variants stay
in the [open-work list](../../../docs/guide/npc-open-items.md).

### Completed 0.1.9 game check

`NpcHeadRestoreFix` 0.1.9 is installed as the sole enabled Manager entry
(`npcheadrestorefix-4d4ff343`, `in_sync`). The installed full cache is byte-identical
to the guarded artifact; all 7,315 unselected pristine modules and the other
three authored module emissions are preserved. Readback confirms the exclusion
call before poseable creation, existing pose/event metadata, voice/localization,
original backups and recorded saves through slot 46. See
[restore-build-result.json](restore-build-result.json). The user result above
supersedes the original pending runtime status.

1. Load **Slot 46 `npc kopf - halsanimation`**, without 09 or 14. Check C's head
   and neck once more.
2. At A choose **15**. Expect the Hero head and no crash. Save as
   **`npc kopf - wiederhergestellt`**.
3. Choose **14** twice. Expect one correctly connected replacement head.
   Save as **`npc kopf - erneut`**.
4. Fully restart the game and load `npc kopf - erneut` without 09 or 14.
   Check the connection, then try **15** once more.

### Previous 0.1.8 game check

`NpcHeadNeckFix` 0.1.8 is installed as the sole enabled Manager entry
(`npcheadneckfix-531b1ef0`, `in_sync`). The guarded composition preserves all
7,315 unselected pristine modules byte-for-byte and the other three authored
module emissions exactly. Installed readback passes the new Tick metadata and
copy/reset calls, existing event bindings, voice/localization, original backups
and recorded save baselines through slot 45. The final artifacts and checks are
in [neck-build-result.json](neck-build-result.json). The user result above
supersedes its original pending runtime status.

1. Load **Slot 45 `npc kopf - grundpose`**. Do not choose 09 or 14: the saved
   selection already enables the different head.
2. Inspect C from the front and side, including while he turns and gestures.
   Check whether the head stays correctly shaped and the neck joins the body.
3. Save as **`npc kopf - halsanimation`**, even if the join is still wrong.
   Choose **15** and check restoration of the Hero head.
4. Only if the result looks normal: choose **14** twice, check for duplicate
   parts, save, fully restart the game, and load without 09 or 14.

Facial lip sync is excluded. The new saved diagnostics describe pose transfer;
the user's visual result decides whether this head/body combination works.

## Previous game checklist (0.1.6)

`NpcHeadVisibilityFix` 0.1.6 was installed as the sole enabled Manager entry;
status and installed readback passed. The selected four-module composition
preserves all 7,315 unselected original modules byte-for-byte. The historical
compiler receipt covers an intermediate with one extra stale test-module export;
the final selection excludes that module and restores its pristine bytes.
[visibility-build-result.json](visibility-build-result.json) records both stages,
the installed scripts, preserved voice/localization/backups and save baselines
through slot 43. The user result above supersedes the pending appearance check.

1. Load **Slot 42 `npc aktivitaet fix - morgen`**. At A choose **09 Aufbau**,
   then **14 C: anderen Kopf anlegen**. Inspect C from the front and side,
   including while he turns. Look for one correctly shaped head and unchanged
   body/clothing.
2. Save as **`npc kopf - materialfix`**, even if the head is still distorted.
   Then choose **15** and check that the Hero head returns.
3. Only if the replacement looks correct: fully quit and restart, load
   `npc kopf - materialfix` without choosing 09 or 14, and check C again.

The next save will distinguish complete material hiding from the separate pose
compatibility question. Its slot is derived from the save name by the agent.

## Original game checklist (0.1.5)

`NpcHeadProbe` 0.1.5 was built and installed with Manager state `in_sync`.
The user's result above supersedes the original pending visual status.

1. Load **Slot 42 `npc aktivitaet fix - morgen`**. At A choose **09 Aufbau**, then
   **14 C: anderen Kopf anlegen**. Inspect C from the front, side and back: a
   different face, one head, unchanged Novice clothing/body, no floating parts.
2. Choose **15 C: Heldenkopf wiederherstellen**: C should have his original Hero
   head again. Choose **14** twice: the different head should return without
   duplicate parts. Save as **`npc kopf - wechsel`**.
3. Fully quit and restart the game, then load that save. **Do not run 09 or 14.**
   Inspect C again and observe his head while he turns toward the player.
   Save as **`npc kopf - geladen`**.

If step 1 fails, save as **`npc kopf - auffaellig`** and report what is visible;
one screenshot is useful. Stop there. The agent will derive slots from saved
names and read the diagnostics; no additional console work is required.

## Offline evidence and limits

Exact head extraction and skeleton references are recorded in the
[preceding investigation](../npc-appearance-routine/RESULTS.md#heads-and-faces).
Local build inputs and reports are under `work/npc-head-probe`.

The prototype exposed a CLI limitation: native engine declarations unused by
the pristine scripts were rejected even when registered by this game build.
The required declarations are now qualified against the sealed native binding
profile; the [audit record](native-api-qualification.json) identifies each
approved declaration and its provenance. A standalone module build and a real
splice onto pristine passed, including the native event metadata. Four focused
qualification/composition tests and 85 existing sequential-cache guard tests
passed. The full graph and four-module mini compiled successfully and Manager
installed the bundle. Readback confirmed both activity/head event bindings,
the exact asset path in full/mini/installed caches, unchanged voice and
localization, original backups, and save baselines through slot 42. Artifact
hashes and the installed result are retained in [build-result.json](build-result.json).

The probe also exposed a decompiler arity collision: the global two-argument
`LoadObject` was confused with an unrelated zero-argument method in Binds.
The resolver now prefers the global's exact cache signature. Two arity regression
tests passed, and installed readback now includes both the null outer object and
the exact asset path. The deployed bytecode already retained these arguments.

The original 0.1.5 prototype used `ShowMaterialSection` with `SectionIndex=0`.
Its incomplete visibility result led to the 0.1.6 correction above.
Loaded-asset, material-match and visibility-query diagnostics are written as
`gore_head_v1_*` world values. Those values establish script progress, not
that the resulting character looks correct. Precise lip sync is excluded.

Remaining work is tracked in [NPC open work](../../../docs/guide/npc-open-items.md).
