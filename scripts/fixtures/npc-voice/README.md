# Invented NPC voice campaign

Passed on BuildID `24878692` on 2026-09-06, following the successful
[NPC session campaign](../npc-session/RESULTS.md). The user confirmed all game
steps; read-only checks of saves 029/030 passed. See the [runtime results](RESULTS.md)
and [build and installation checks](BUILD.md). Lip sync is excluded.

The fixture retains A/B, their spawn points, Diego's checkout health edit and
A's completed/running quest behavior. Eight additional direct choices on
`GORE_TEST_A` are always visible and repeatable. They end the conversation and
do not change the fixture quest, its knowledge markers, relationship or inventory.
The game may still record that an individual topic was selected.

## Cases

| Choice | Path under test | Expected observation |
|---|---|---|
| 01 Original | Shipped `LocText` recordings, A then Hero | A says “Ich bin Diego”; Hero says “Ich bin …”; speaker/camera association follows the corresponding actor. |
| 02 Stimmtyp | Generic Address_Call request via A's `VoiceTypeSubsets` | Diego's Voice05 “Hey!” from A. No explicit recording ID is passed by this choice. |
| 03 Neue Aufnahme | New text ID and new 48 kHz mono Vorbis member | Male synthetic voice says “Voice test three …”; matching German subtitle. |
| 04 Neue Aufnahme | New text ID and new 44.1 kHz mono Vorbis member | Male synthetic voice says “Voice test four …”; matching German subtitle. |
| 05 Neue Aufnahme | New text ID and new 48 kHz stereo Vorbis member | Male synthetic voice says “Voice test five …”; matching German subtitle. |
| 06 Neuer Dialog | A → Hero → A, all new recordings | Male → female → male synthetic test voices; each actor owns the correct turn; clean completion. |
| 07 Fehlende Aufnahme | New subtitle ID intentionally has no archive member | The user confirmed the silent subtitle and continuation to the subsequent recorded A line. This is fixture-specific behavior, not a general missing-audio guarantee. |
| 08 Stimmtyp | Two generic daily-routine Mumble requests | A uses Voice05 variants such as “Was haben wir hier?”, “So …”, “Also dann …”, or “Na, wenn er das so sagt …”. Repeated random variants are allowed. |

The explicit recording tests and generic selection tests exercise different
routes. Choosing a `LocText` recording does not prove inherited voice selection.
The generic tests call the shipped `Say(AI, SoundTag, ...)` overload, which checks
`CanSayGenericVoiceline` before executing. The user confirmed both generic
cases; native cooldown behavior and other selection contexts are not inferred
from this result.

The synthetic files are locally generated with Windows System.Speech. A uses
Microsoft David Desktop; Hero uses Microsoft Zira Desktop so turn assignment
is audible. These are deliberately distinct test voices, not imitations of the
game's actors. English speech and German subtitles are intentional. The stereo
payload has two identical channels; it tests format playback, not stereo imaging.

## Build inputs

- [choices.as](choices.as): append to the checked A conversation module, keeping
  its original declarations and existing quest choices.
- [recordings.json](recordings.json): exact synthetic voice, text, subtitle,
  sample rate and channel count for each new recording.
- Every new spoken ID has a localization row and one unique `.ogg` basename
  under `german_new/GoreMods/NpcVoiceProof/`. Hero and A use separate folders.
- `GORE_NPCVOICE_A_MISSING_01` has text only, intentionally.
- The bundle uses `voice` Add entries in `german_new.zip`, plus `loc_edits` and
  the complete four-module NPC mini-cache. It leaves shipped recordings intact.

Use `dialog checkout` / `check` against the preceding composed full cache with
`GORE_AS_BINDS` pointing to the matching Binds cache. Compile the complete source
graph with `as compile --backend standalone --mini ...`; the new A module and
world references must compose together onto pristine Shipping. Build and inspect
one complete bundle, then replace the previous test entry through Manager.

## User game procedure

1. Set spoken language to German and enable subtitles. Load a preserved completed
   campaign save, such as `G1R-027.sav`, and speak to A at Xardas' Tower.
2. Select 01–08 and record audible content, subtitles, speaker association and
   whether every choice returns control. Preserve the current quest state.
3. Repeat 03 and 06. Then skip a line in 06 with the normal dialogue-skip control;
   the next speaker should continue and the conversation should end normally.
4. Save in a new manual slot named `NPC Voice - vor Neustart`. Fully exit the
   game, start again and load that slot. Repeat 02, 03, 06 and 08, then save in
   another new slot named `NPC Voice - nach Neustart`.
5. Report the two filenames and any case that differs from the table. Screenshots
   establish text/speaker display; audio or the user's listening report establishes
   audible playback. Save inspection alone cannot prove sound or process restart.

Do not overwrite the original campaign saves. Stop on a hang or crash and report
the choice number; a silent generic request alone does not identify its cause.

## Qualification boundary

This campaign covers this invented actor, German archive selection, explicit
recordings, supported Vorbis layouts, inherited generic voice selection, repeated
playback, skipping and reopening across sessions. General voice requests are
invoked deliberately; this does not prove every natural combat/routine trigger.
Other languages, Opus, unrelated speakers and facial authoring are outside this
campaign. Opus was already silent in the earlier Diego format test.
