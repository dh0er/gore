# Quest: Proviant fuer die Wache

Focused game test for the callback paths that the basic session quest did not
cover. The user performs the game campaign. Compilation and bundle inspection
do not establish that these callbacks execute in the game.

`GORE_TEST_A.as` replaces A's previous test menu with seven numbered quest
choices. It retains A's established character, appearance, routine and private
conversation root. All new quest, document and topic classes are in that same
module. Every original A class, helper, field and default remains available for
references in existing saves, including `UQuest_GORE_NPC_SESSION`. Only the
29 original choice visibility methods return false, hiding the old menu. B and C
are unchanged.
Load this bundle by itself, using **Profil 4, Slot 071, NPC 05 Quest - START**,
where A exists and this quest has not been accepted. Its journal title,
description, objective titles and four journal paragraphs are authored here;
they do not reference a vanilla letter or a vanilla localization row.

The text is intentionally German and embedded through the already tested
`FName -> FString -> FText` helper. No localization patch is required for the
quest text. A's displayed NPC name is supplied by the batch's name localization
payload; its technical identity remains `GORE_TEST_A`.

## Game campaign

Use free save slots. Keep the starting save and every named checkpoint below.
After each choice, let the conversation close and wait a few seconds for the
quest system before checking the journal or opening A's menu again. If a stage
does not advance, record that result; do not use a console to force completion.

1. Talk to A and select **Q1 Auftrag: Proviant fuer die Wache**. The active
   journal must show that title, the promised **25 Erz**, and objective
   **Sprich die Lieferung mit dem Proviantmeister ab**. There must be no unrelated
   vanilla letter. Q1 must disappear and Q2 must appear. Save `quest-absprache`.
2. Exit the game completely, restart and load `quest-absprache`. The active
   journal and Q2 must persist. Select **Q2 Abgemacht: zwei Kaese fuer 25 Erz**.
   The first objective must succeed automatically, the second objective
   **Gib dem Proviantmeister zwei Kaese** must start, and the journal must gain
   the paragraph beginning **Die Lieferung ist abgesprochen**.
3. Select **Q3 Testvorrat: zwei Kaese nehmen (einmalig)**. Check that Hero's
   cheese count rises by exactly two and Q3 disappears. This explicit test helper
   is available once per campaign; it gives no ore and completes no objective.
   Record Hero's cheese and ore counts, then save `quest-lieferung`.
4. Exit completely, restart and load `quest-lieferung`. The second objective
   must remain active, Q3 must stay absent, and the recorded counts must match.
   Select **Q4 Hier sind die zwei Kaese**. Hero must lose exactly two cheese;
   A receives those two. Both objectives and the root quest must complete
   automatically. Hero must receive exactly **25 ore**, and the journal must
   gain the completed delivery paragraph. Q4/Q5 disappear; Q6 appears.
   Save `quest-erfolg`.
5. Select **Q6 Die Lieferung ist bezahlt. Danke.** several times. Inventory
   counts must remain unchanged. Exit completely, restart and load
   `quest-erfolg`; check the completed journal, Q6, inventory counts and another
   Q6 selection. Save `quest-erfolg-neustart`.
6. Load the preserved `quest-lieferung` checkpoint to test the alternate branch.
   Select **Q5 Ich sage die Lieferung ab (Auftrag scheitert)**. The root and
   active second objective must fail automatically. The first objective remains
   completed. The journal must gain the cancellation paragraph. Cheese and ore
   counts must remain those recorded in step 3; there is no hand-in or reward.
   Q3/Q4/Q5 disappear and Q7 appears. Save `quest-abbruch`.
7. Exit completely, restart and load `quest-abbruch`. Failure, the journal,
   Q7 and unchanged inventory must persist. Select Q7 again; nothing is granted
   or consumed. Save `quest-abbruch-neustart`.

Report the checkpoint filenames and any differing journal, menu or inventory
result. Distinct save files do not themselves prove a full process restart.

## Source and evidence

Root class: `G1R::Quest::UQuest_GORE_BATCH_PROVISIONS`. Its children end in
`_AGREE` and `_DELIVER`. Journal class:
`G1R::Document::UDocument_GORE_BATCH_PROVISIONS`.

Dialogue only writes persistent Hero knowledge and performs the guarded item
transaction. `ShouldStart_Implementation` starts the root after acceptance;
`HandleQuestStarted_Implementation` starts the first objective and unlocks the
first journal segment. The first objective's success predicate recognizes the
agreement. The second objective's start predicate follows the first objective's
success; its start handler unlocks the next paragraph. Hand-in knowledge drives
the second objective's success predicate, which drives the root's success
predicate. The root success handler is the sole reward site and guards it with
Hero knowledge. The root failure predicate and handler implement cancellation,
fail a running child, and unlock the failure paragraph. These are the same
native callback names and transition operations used by generator v5; this
handwritten fixture is not proof of every generated transition plan.

All knowledge below belongs to Hero. The prefix is `gore_batch_quest_`:

| Checkpoint | Present suffixes | Absent suffixes |
| --- | --- | --- |
| `quest-absprache` | `accepted` | `agreed`, `supplied`, `handed_in`, `rewarded`, `cancelled` |
| `quest-lieferung` | `accepted`, `agreed`, `supplied` | `handed_in`, `rewarded`, `cancelled` |
| Success and reloaded success | `accepted`, `agreed`, `supplied`, `handed_in`, `rewarded` | `cancelled` |
| Failure and reloaded failure | `accepted`, `agreed`, `supplied`, `cancelled` | `handed_in`, `rewarded` |

Existing source references were read in the BuildID 24878692 source tree at
`work/npc-warning-fix/tree`:

- `Story/G1R/Quest/Quest_FreeMine_BALOROS_WAFFE.as`:
  `SetQuestlogDocumentClass`, root quest kind and failure/success callbacks.
- `Story/G1R/Document/Misc/Document_CH1_BALOROS_WAFFE.as`:
  own `UQuestLogDocument`, `UDocumentSegment`, `InDocument`, `BuildSegment` and
  `AddParagraph` declarations.
- `Story/G1R/Quest/Quest_ValleyOfMines_VoMCHAPTER1_BESTIARY_1_BESTIARY_1_OBJ_FIND.as`:
  availability predicate and `UnlockDocumentSegment` in the start callback.
- `Story/G1R/Quest/Quest_FreeMine_BALOROS_WAFFE_BALOROS_WAFFE_OBJ_DELIVER.as`:
  child start predicate over preceding objectives' `HasSucceeded()` states.
- `Story/G1R/Conversation/Conversation_FM_SLD_BALORO_753.as`:
  `HasItem(Hero, UItFo_Cheese, 2)`, removal from Hero, addition to the giver and
  document-segment unlocking.
- `Story/G1R/Conversation/Conversation_Generic_AboutCamp_NC_ORG.as`:
  `AddItemToInventory(Hero, UItMi_Orenugget, count, EInventoryTypes(1))`.
- `crates/gore-authoring/src/quest.rs`, generator v5:
  explicit availability override, `Should*` predicates, `HandleQuest*` effects,
  guarded `StartQuest`/`FailQuest` and the FName text helper.

`quest-content.as` is the new same-module section; `GORE_TEST_A.as` is the
complete overlay used for compilation. The work copy is at
`work/npc-batch-tests/quest/GORE_TEST_A.as`. Keep any build diagnostics and
inspection receipts with that work directory. Compilation and runtime result
status is tracked by the parent batch checklist.
