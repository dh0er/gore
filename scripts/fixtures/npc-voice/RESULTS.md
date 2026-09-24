# NPC voice results — 2026-09-06

**All eight voice cases and the repeat/skip/restart campaign passed on BuildID
`24878692`, as confirmed by the user.** Both resulting saves independently pass
all 23 read-only state checks. Lip sync was excluded.

## User observations

After receiving the [test procedure](README.md#user-game-procedure), the user
reported: “alles erledigt. alles hat perfekt funktioniert. saves sind in slot
29 und 30.” This confirms the requested tests, including full exit/restart.
No new audio/video recording was supplied, and no loopback correlation was
measured. Audible playback, subtitle display, speaker association and restart
are user-reported evidence; the agent did not operate the game.

| Case | Confirmed behavior |
|---|---|
| 01 | A played the shipped Diego recording; Hero played the corresponding Hero recording, with correct speaker association. |
| 02 | A's assigned Diego Voice05 subset supplied the generic `Address_Call` recording (“Hey!”). |
| 03 | A played a new 48 kHz mono Vorbis recording with its new subtitle ID. |
| 04 | A played a new 44.1 kHz mono Vorbis recording with its new subtitle ID. |
| 05 | A played a new 48 kHz stereo Vorbis recording with its new subtitle ID. |
| 06 | A → Hero → A played all three new lines with correct turns and distinct synthetic voices. |
| 07 | The intentionally absent recording produced the silent subtitle and continued to the next recorded line. |
| 08 | Generic daily-routine Mumble requests used A's assigned Diego voice. |

The user also confirmed repeating 03/06, skipping a line in 06, and replaying
02/03/06/08 after fully restarting and loading the new save. The quest stayed
completed and B remained outside the voice menu. English synthetic speech with
German subtitles was intentional; the female Hero test voice distinguishes its
turn from A's male test voice.

## Independent save inspection

Names and profile below were read from public save metadata. The existing
[NPC state verifier](../../npc_session_proof.py) inspected the private state
through fixed read commands and left both save files unchanged.

| Slot | Player save name | Profile | Quest state |
|---|---|---|---|
| `G1R-029.sav` | NPC Voice - vor Neustart | 2 | `EQuestState::Succeeded` |
| `G1R-030.sav` | NPC Voice - nach Neustart | 2 | `EQuestState::Succeeded` |

- `027 → 029`: both actor identities, inventories and attributes survive the
  voice campaign; the prior quest remains completed.
- `029 → 030`: the same state survives the reported restart and repeated voice tests.
- A remains `GORE_TEST_A-WP_WarningFlock_XT_01`; B remains
  `GORE_TEST_B-WP_WarningFlock_XT_02`.
- A retains `gore_npc_session_started`, `gore_npc_session_completed`, and exactly
  one `ActivePersonalRelationshipModifier_Story` towards Hero with Friend.
- Both saves contain all eight `ChoiceGoreNpcVoice*` topic IDs on A and Hero,
  the two conversation participants. B and Diego have none. These are recorded
  topic selections, not evidence of sound output or the fixture's custom markers.
- Neither B, Hero nor Diego owns A's two fixture knowledge markers.
- Each report passes 23 checks with zero errors; the two save hashes differ.

The verifier's `runtimeRestartProven` remains `false`, as it must: save bytes
alone cannot prove a separate process. The combined qualification above uses
the user's completed game campaign as that evidence.

## Fingerprints

Raw reports and save files remain local; no user saves are included in this
fixture. Artifact and installation hashes are in [BUILD.md](BUILD.md).

| Evidence | SHA-256 |
|---|---|
| `G1R-029.sav` | `43818f92425d0804fe71c2331a1a7d5ec34dc2bb6c3d6e10168c50bf2e69e24d` |
| `029-proof.json` | `720c7898bca4e8ae8c46fd969e90e24ba1333d7585645df857feb7c292c71c48` |
| `G1R-030.sav` | `93fac2808953b9c184bcd8e5817e601fd6f02ef1407d38c400fa04e47510bfbf` |
| `030-proof.json` | `2f4f208bf7e476156c5d95c66f7de9153651073ff96ef8dc8e6692c715a47985` |

## Scope

This qualifies the tested invented actor, new explicit recordings and subtitle
IDs in the German archive, three Vorbis layouts, inherited Diego generic voice
selection, speaker turns, repeat/skip behavior and replay after restart. The
stereo payload has two identical channels, so no stereo imaging claim is made.
The missing-recording result applies to that diagnostic line. Generic requests
were deliberately invoked; every natural combat/routine trigger, other voice
profiles, other archives/languages, Opus and lip sync remain outside this test.
