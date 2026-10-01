# Build evidence — 2026-09-06

Target: Gothic 1 Remake BuildID 24878692. Explicit clothing and C persistence
passed the user test. Scheduled noon walking passed on2026-09-07 with0.1.2;
reading, drinking, the evening return and restart continuation subsequently
passed with0.1.4. The earlier ambient-task fixture failed. See [RESULTS.md](RESULTS.md).

## AI event declaration correction — 2026-09-07

`NpcActivitiesEventFix` 0.1.4 changes only two declarations in B: explicit
`UFUNCTION(BlueprintOverride)` with unsuffixed `DoTask` and
`OnGracefulExitRequested` event names. All other source modules and all
voice/localization inputs are identical to0.1.3.

The old installed cache and original build were inspected at the exact
`UAIState_GoreScheduledActivity` class record. Both callbacks had suffixed Unreal
names and callable/override/event flags1/0/0. The corrected single-module native
compile produced44334 bytes and has unsuffixed Unreal names with flags0/1/1.
The checker links the methods to the exact class record; it does not infer event
metadata from emitted source. Reports are under `work/npc-activities-event-fix`.

A guarded incremental composition attempt stopped on a cache-wide function-ID
collision between B and A (`0x3c845`). Only local artifacts were written; the
guard was not bypassed. The package therefore uses the complete-graph standalone
compiler. That build passed:7319 modules,124581809-byte full cache and a
133139-byte four-module mini. Both final artifacts retain the corrected native
event names and override/event flags. Current CLI bundle inspection passed:
three components,ten files,330783 bytes. The older MCP inspector retains its
known multi-module limitation.

Manager entry `npcactivitieseventfix-de2d0063` is the sole enabled entry;0.1.3 is
disabled. Analyze reported no recognized conflicts. Dedicated preflight confirmed
a closed game and no recovery state. Apply succeeded; status is `in_sync`.
The installed cache passes the same exact-class event-metadata check as the
full and mini artifacts, and its emitted A/B code equals the build.

All33323 original voice entries retain CRC/size, all five added recordings match
source bytes, and all43904 localization rows match the passed voice campaign.
Original backup hashes and saves029–034,036–039 remain unchanged. No game was
launched by the agent. Reports include `installed-event-metadata.json`,
`deployment-evidence.json`, `artifact-hashes.json` and `verification.log` under
`work/npc-activities-event-fix`. These reports record the offline deployment
checks. The user subsequently confirmed the complete0.1.4 game test on2026-09-07;
runtime observations and saved state are recorded separately in [RESULTS.md](RESULTS.md).

| Event-fix artifact | SHA-256 |
|---|---|
| Full cache | `883c681feab5712ef3a860526284bfd6123956f9d1e707893c70f89acfcf238b` |
| Mini cache | `7bb4a7013860cb2d571983bc3e789989205de55607082c2fcd4e7af50ba0b28c` |
| Generation receipt | `fcf3617e35149f5185867cf7ab576ad8fdb84dc3322b7162b0c26a3fd164f0c1` |
| Bundle manifest | `fe1ab71ea3c363b6fcba9eb30e105b8fb1f5717285af391ce8661b41d24951c4` |
| Bundle tree | `ddfc82d66d3d9628724eaeb2e3ef98514f0c1bfcd45db4d70310183d1c09bf5c` |
| Installed shipping | `d345a65906817e23168ccd14070f033a1474c4ce2d697c14e7a84ecfaf4b284e` |

## Direct reading/drinking activities (runtime failed) — 2026-09-07

`NpcActivitiesProof` 0.1.3 changes only A/B among7319 source modules. B now
uses direct reading/drinking actions after reaching the same two outdoor
points. The final state includes the shipped graceful-exit override; A's evening
control advances to17:59. Quest, character identities, B/C clothing, localization
and all five voice inputs are unchanged.

Native combined precheck passed with a121274-byte mini before adding the small
graceful-exit override. The full build validates the final source, including that
override. The rebased optimized CLI uses the deployment-owned original backup,
explicitly pinned to `7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511`.
The walking package remains installed during compilation; no Reset is needed.

Source comparison and voice/localization input equivalence passed. Save036 was
copied and its recorded SHA-256 verified. All29 local documentation links
resolve; `git diff --check` found no whitespace errors. No Rust implementation
changed in this continuation.

Final standalone compilation passed:7319 modules,124581839-byte full cache,
133169-byte four-module mini (A/B added, Diego/XardasTower edited). Current CLI
inspection passed:three components,ten files,330810 bytes. The installed0.3.0
MCP inspector still rejects the supported multi-module shape; current CLI
inspection and Manager Apply both accept it.

Manager entry `npcactivitiesproof-67e41f1f` is the sole enabled entry; the walking
package is disabled. Analyze found no recognized conflicts. Dedicated preflight
confirmed a closed game and no recovery state before Apply. Apply succeeded and
status is `in_sync`. Installed A/B output matches the full build exactly,
including the activity states and graceful exit. All33323 original voice entries
retain CRC/size; all five additions match source bytes. All43904 localization
rows and the complete voice/localization hashes match the passed voice campaign.
Pristine backups match the known originals; saves029–034 and036 are unchanged.
No game was launched by the agent.

Reports and logs are under `work/npc-activities`, particularly
`deployment-evidence.json`, `source-evidence.json`, `artifact-hashes.json` and
`verification.log`. The subsequent game test failed: the activities and walking
did not run. The event-metadata correction above addresses that observed failure;
the original offline checks did not establish working gameplay.

| Activity artifact | SHA-256 |
|---|---|
| Full cache | `6a0c3ad204fd013293bd03f2c02b11b71008c617d8556a74f15a18ea6e1f32af` |
| Mini cache | `5749e58ba2b016e7b475c7a5e1c914df1c27616ad728ae5a4a53133d7692115e` |
| Generation receipt | `7b2657fcf56128f3287673316234d1743bb7bc1971b0515bab5e49fd60b1c67c` |
| Bundle manifest | `3a03b71c2ecc474d7885d293b5c7a4a24347badacf7c85c2e9bf1d1e88668abb` |
| Bundle tree | `798d521de92d9ea9ee52250de6647120ebb16651f15af2b1e6419ef694c7b097` |
| Installed shipping | `bedbb640ce3799680f7e2e8be2e2b1f16c78002a83647404fe542adda9b857c5` |

## Direct walking correction (superseded; noon walking passed)

`NpcRoutineWalkingProof` 0.1.2 replaces B's three ambient tasks with the shipped
`UAIState_TestGotoWP`. Its route393 → `FP_XT_WAIT_OUTSIDE` →393 uses two observed
outdoor points. A's changes are limited to three menu captions. No appearance,
quest, localization or voice input changed. Native combined precheck passed:
113872 bytes. Full graph compilation passed:7319 modules,124574103 bytes;
the deployable four-module mini is125433 bytes. Artifacts are under
`work/npc-routine-second`.

The first compiler attempt correctly refused to compile against the deployed
Shipping cache. Manager preflight confirmed a closed game/no recovery state;
Reset restored the pristine cache. The diagnostic module precheck explicitly
permits the fixture's new script symbols; strict remapping still applies.

Current CLI bundle inspection passed:three components,ten files,323078 bytes.
The older0.3.0 MCP inspector retains its known multi-module limitation.
Manager entry `npcroutinewalkingproof-839575d1` is the sole enabled entry;
the previous correction is disabled. Analyze found no recognized conflicts,
preflight confirmed a closed game/no recovery state, and Apply succeeded.
Status is `in_sync`.

Installed A/B output matches the new build exactly, including direct walking
and the updated09 caption. All33323 original voice entries retain CRC/size,
all five additions match their source bytes, and all43904 localization rows
match the passed voice campaign. Original backups and saves029–034 remain
byte-identical. No game was launched by the agent. The runtime checkpoint
Slot34 →09Aufbau(Lauftest) →10at11:59 →walking at noon passed; Slot36
`npc lauf - mittag` corroborates the resulting position and saved routine.

| Walking artifact | SHA-256 |
|---|---|
| Full cache | `1f5accda9d7aa8303aa6e643a52bafd2d6c365f700b8eb4f7162e2c36fe1f75c` |
| Mini cache | `b9a75f0ff5f61128dcdc1c68d0b869a027fee1522ff5291d4a679721182603a9` |
| Generation receipt | `d57ca35cb7e130b00aafd2ca90f8e0113ea058ecd98791cbc32ce31a6191949a` |
| Bundle manifest | `26286b870b291d1e487773188e20da9650d76333bcef75b5221d4deed5af69ad` |
| Bundle tree | `74f8281ffa784bf2adfcbc6872fd0f026162d1cb424df14977ff455bafa9df69` |
| Installed shipping | `59ccedd0a60c5eb8b4657422195409111179da383660e2fd0939c4861117068d` |

## CLI update and compilation with the mod installed — 2026-09-07

The branch was rebased onto `origin/main` at `f963ba17`; the existing40 commits
and all25 uncommitted/untracked files were preserved. The only manual conflict
combined the two additive changelog entries. The rebased HEAD is `20c05503`.

All107 targeted NPC/cache tests passed. The optimized workspace CLI was rebuilt
with the existing authenticated compiler catalog pinned at
`de2a2820193eb81f3032e3731d469a02dbd9ea05942f0ef48ebab58966a46e64`.
A strict standalone `compile-module` of the existing Diego fixture succeeded
while `NpcRoutineWalkingProof` remained deployed. The compiler selected the
deployment-owned original backup and matched the explicit original SHA-256.
The5249-byte diagnostic mini was not installed.

All seven live/backup/deployment-record files remained byte-identical and
Manager status remained `in_sync`. No Reset, Apply or game launch occurred in
this verification. This supersedes the historical requirement to undeploy the
fixture before compiling. Logs and reports are under `work/npc-rebase-main`,
including `smoke-evidence-release.json`; the updated executable is
`work/npc-continuation/runtime/gore-current.exe`.

## Setup failure and correction

User observed no visible action from09, with10/11/12 appearing afterward. This
proves the old ready marker was reached, excluding the A/B lookup, C spot guard
and null C spawn as the cause of this attempt. The old code placed
`EndConversation()` before all clock and teleport effects in all four choices.

Native read-only inspection and shipped call sites support treating that call
as a control-transfer boundary. `AdvanceToClockTime` computes/stores time and
returns; `EndConversation` invokes its bound context callback. A's own routine
exchange also updates active AI and invokes callbacks, so it now comes after
B/C placement, status storage and Hero teleport. EndConversation is last.

Current corrected source adds one saved setup status and two mutually exclusive
status labels. Phase visibility checks the current successful preparation,
rather than the old ready marker. No character lookup, appearance definition or
route was changed. Independent review passed. Native combined module check
passed: 113961 bytes. The complete corrected graph compiled successfully:
7319 modules, 124574192 bytes. The deployable mini contains the same four
modules as before, now 125522 bytes. Only A's source changed from the first
appearance/routine build.

Bundle `NpcAppearanceRoutineFix` 0.1.1 passed current CLI inspection: three
components, ten files, 323168 bytes. The older 0.3.0 MCP inspector still rejects
the supported multiple-module shape; current CLI inspection and Manager Apply
both succeeded. Preflight confirmed a closed game, no recovery state and one
enabled entry. The previous appearance package is disabled.

Manager entry `npcappearanceroutinefix-e62a3a10` is the sole enabled entry and
reports `in_sync`. Installed A/B output matches the new compiled output exactly,
including the new saved status and status topic. All 33323 original voice
entries retain their CRC and size; all five added recordings match their source
bytes. All 43904 localization rows and the complete voice/localization hashes
match the passed voice campaign. Pristine backups match the known originals;
saves 029/030 remain byte-identical. No game was launched by the agent.

Artifacts and `deployment-evidence.json` are under
`work/npc-appearance-routine-fix`. The subsequent user test confirmed this
setup correction and the clothing, but not correct target selection or walking;
its saves031–034 are recorded in [RESULTS.md](RESULTS.md).

| Corrected artifact | SHA-256 |
|---|---|
| `NpcAppearanceRoutineFix.full.Cache` | `6575c9495f338f155d35aa74185b51f2d0569205a0f056108caa06a69c094a3f` |
| `NpcAppearanceRoutineFix.mini.Cache` | `0f296de7084a239675388d5cf4fb806b5960dc5cd89ac259ad0ce8bd7eeca522` |
| `generation-receipt.json` | `4a7141ccb9192705c63aa99fee297f41c9d618d7beaa1f81bc07fea266133c22` |
| Bundle manifest | `e58bff908a6865d70354b9c8ffa446c481a5dfed5493bab1ed8596e967565d69` |
| Bundle tree | `c2062a845437f6e7125a5cb1fcffa0991edac7fc223d848d04c95e9960c9854a` |
| Installed shipping | `a9f42f4a6a108ca059ec0958379d79a53e5c473a462139a6980422c0266adf96` |

## Initial package (superseded)

The initial build evidence below records the package that failed its first
runtime setup attempt; its successful offline checks were not runtime proof.

- Saves 029/030 backed up byte-identically before changes.
- Exact extraction of `MO_Player` succeeded. Exact `MO_Characters` extraction
  at its script-declared package path returned asset not found.
- Explicit clothing values were checked against the extracted cooked model and
  shipped armor definitions. Full fixed-width asset inspection cannot decode
  the native `MutableLODSettings` structure; no cooked asset was edited.
- Combined A/B/C native module precheck passed strict standalone compilation and
  remapping: 107342 bytes. This diagnostic mini is not the deployable package.
- Direct `SetCurrentClockTime` compiled but failed the strict reference-membership
  guard because the target generation does not carry its function reference.
  Final source uses the shipped `AdvanceToClockTime` call instead. No compiler
  guard was relaxed.
- Focused independent source review found no blocking issue. Natural noon
  transition, time-skip behavior and C persistence/deduplication remain explicit
  runtime questions.
- All 20 NPC generator tests passed after correcting the conditional teleport
  enum spelling and its description. All 22 local documentation links resolved.
- Source comparison confirms only A/B modules changed from the successful voice
  tree. All previous voice inputs and localization edits remain identical.

The full graph compiled successfully: 7319 modules, 124567573 bytes. The
118903-byte deployable mini contains A/B additions and Diego/XardasTower_AI
edits. C's definitions live in the B module; C is spawned by the setup choice.

The complete bundle passed current CLI inspection: 3 components, 10 files,
316551 bytes. The installed 0.3.0 MCP inspector still rejects the supported
multiple-module mini shape; the current workspace CLI validates it successfully.

Manager entry `npcappearanceroutineproof-8af86a9e` is the sole enabled entry and
reports `in_sync`. `npcvoiceproof-247d3668` remains disabled in the library.
Preflight confirmed a closed game and no recovery state before Reset and Apply.

Installed A/B output matches the compiled output exactly, including C and the
four controls. All 33323 original voice entries retain their CRC and size;
five added recordings match their source bytes. All 43904 localization rows
match the passed voice campaign, and voice/localization files also retain the
same full hashes as that campaign. Pristine backups match the known originals.
Saves 029/030 remain unchanged. No game launch occurred; runtime is pending.

Source and full artifacts remain under `work/npc-appearance-routine`.

| Artifact | SHA-256 |
|---|---|
| `NpcAppearanceRoutineProof.full.Cache` | `c50960f669eff96025d4d11e49c54f99719d06e667ec339d673f79a43c110cd0` |
| `NpcAppearanceRoutineProof.mini.Cache` | `99b029a66ae770a75393f9cf3fe36a2db41eb5b2fd1405535c9d8903d411cae1` |
| `generation-receipt.json` | `1a1d7282321df5f35ebf79835677ef4e6b12fc31e0905bfe667243593ca5d3ea` |
| Bundle manifest | `1a3b95aaa93ae30a98610abfef65b3bab8ef8059a66e8a51bdcb08be9780609a` |
| Bundle tree | `270683aa03be547769c3f290fd120a574cc4650bcb61e8f7b4a25d2cfb7776fb` |
| Installed shipping | `18714eaf14263b51eca68cfc940bd0b42e857b7677ebc7ab42fca52fb5bb946b` |
