# NPC activities using objects

The user confirmed the complete 0.1.10 object checklist: B sleeps in a real bed,
uses an alchemy table and walks between them at the scheduled transitions.
Sitting and guarding are next. Gameplay validation is performed by the user.

`NpcObjectActivities` **0.1.10** was installed as the only enabled Manager entry
(`npcobjectactivities-34ee7ff4`, `in_sync`) for the passed game checklist below.
It is now superseded by the [sitting/watch fixture](../npc-seat-guard/README.md),
which retains this source and behavior.
The [build result](build-result.json) records the successful standalone compile,
four authored modules, native event bindings and installation readback. All
7,315 unselected pristine modules retain their original bytes. Manager changes
only 318 serialized function IDs; the [byte-level comparison](installed-canonicalization-checks.json)
confirms every other byte, including executable code and reference tables, is
identical to the compiled cache. Voice, localization, original backups and the
recorded save baselines through slot48 are unchanged.

## Runtime result (2026-09-08)

The user reports that the whole checklist worked, including continued sleep
after option19. This is expected: 19 advances to08:00 the next day, and B sleeps
from18:00 until12:00. It is a morning/restart continuation check, not a wake-up
command. The next alchemy phase starts at noon.

| Slot | Save name | Saved clock | B's recorded position |
|---|---|---|---|
| 49 | `npc objekte - bett` | Day10 08:06:43 | Bed |
| 50 | `npc objekte - alchemie` | Day10 12:04:47 | Alchemy table |
| 51 | `npc objekte - abend` | Day10 18:05:35 | Bed |
| 52 | `npc objekte - geladen` | Day11 08:04:51 | Bed |

All four saves retain B's exact identity and `DailyRoutine_GoreObjectActivities`.
Their originals were unchanged. The [runtime result](runtime-result.json)
distinguishes stored positions/routine/clock from the user's observations of
walking, animations and full restart. Crafting recipes and produced items were
not part of this test.

## Exact targets

| Purpose | Named spot | State/action |
|---|---|---|
| Bed | `Interactive_Bed_Xardas472597` | Shipped `UAIState_Sleep`; `Action.Ambient.Sleep.High.Right` |
| Alchemy table | `IO_XT_POTION_ALCHEMY_01` | Shipped `UAIState_PotionAlchemy`; `Action.Ambient.PotionAlchemy` |
| Initial B position | `IO_XT_TAKENOTES_02` | Position only; no note-taking action requested |
| Player arrival | `WP_OW_TP_XARDASTOWER_BEDROOM` | Shipped bedroom teleport waypoint |
| A's observation position | `FP_XT_STANDAROUND_01` | `UAIState_Stand`, position only; the Xardas-only interaction is not requested |

Xardas' shipped `UDailyRoutine_XT_DMB_Xardas_404_Bedroom` pairs the same bed and
table with those states. The bed and table are real object actors with no custom
spot restrictions. In the test's source save 48, Xardas has his `Start` routine
and is on the lower study floor; the mod does not change his routine.
Object streaming, occupancy and walking remain runtime checks.

## Behavior

Option **16** prepares B, A and the player in the bedroom at 08:00. B begins with
sleep, changes to alchemy at 12:00 and returns to bed at 18:00. The schedule has
zero random time offsets and teleport mode `Never`. Only initial setup teleports;
options 17–19 change the clock. A remains available for those controls.

The original A/B source, including all head, free-activity, quest and voice logic,
is retained. Options 01–15 retain their meanings. C remains at the earlier test
location. The new setup records `gore_npc_objects_setup_v1`: 1 means incomplete,
2 means setup calls completed, not proof of successful object use. If a required
NPC or spot cannot be found, option20 reports the failure.

The new [routine definitions](object-routines.as) and [dialogue choices](object-choices.as)
are retained here. This uses shipped object states rather than a free-action
animation played beside furniture. The alchemy task includes object and character
animations; it does not qualify crafting output or recipe/ingredient processing.

## Game checklist

1. Load **slot48 `npc kopf - erneut`** and talk to A. Choose **16**. After arriving
   upstairs, watch B enter the bed. Save **`npc objekte - bett`**.
2. At A choose **17**. End the conversation and wait for noon. Watch B leave the
   bed, walk to the table and use it. Save **`npc objekte - alchemie`**.
3. Choose **18** and watch the evening return to bed. Save
   **`npc objekte - abend`**.
4. Fully restart the game, load that save without16, and check continued sleep.
   Choose **19** and check the next morning; save **`npc objekte - geladen`**.

If B remains idle, save the observed state and report which option was chosen.
The agent derives save slots from names. No console work is needed.
