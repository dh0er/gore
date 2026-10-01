# Invented NPC session proof

This fixture continues the two-NPC campaign on BuildID `24878692`. The user's
game campaign and read-only save verification passed on 2026-09-06; see
[results and evidence](RESULTS.md). Offline checks and generated artifacts alone
must not be reported as restart proof.

## Contents and expected state

- `GORE_TEST_A-WP_WarningFlock_XT_01`: Diego's borrowed appearance, first
  conversation, quest giver and owner of both knowledge markers.
- `GORE_TEST_B-WP_WarningFlock_XT_02`: the previous modular-appearance control;
  its observed player appearance is retained. It gets no new conversation or
  fixture knowledge.
- Diego's checkout edit sets initial Health and MaxHealth to 1234. Existing
  actor state in an older save can override initial defaults.
- `quest.as`: `G1R::Quest::UQuest_GORE_NPC_SESSION`, a side quest under
  `UQuest_ValleyOfMines`, with `GORE_TEST_A` as giver.
- `choices.as`: three direct choices beside the generated private root.
  Accepting starts the quest, writes `gore_npc_session_started` on NPC A and
  adds `UActivePersonalRelationshipModifier_Story(Hero, ERelationship(5))`.
  Returning succeeds the quest and adds `gore_npc_session_completed` on A.
  The final remembrance choice only ends the conversation.

The Story modifier has a class default weight of 1000. The legacy
`Story_Permanent` constructor-only weight is deliberately not used, because
that weight does not survive loading. Neither startup nor quest defaults
reapply markers or relationships. Hero, B and Diego must not own either marker.

## Preparing the source

Use the previously compiled TwoNpcs full cache, whose two actors already have
runtime evidence, as the dialog checkout base:

```powershell
gore dialog new-conversation GORE_TEST_A --cache $TWO_NPCS_CACHE `
  --caption 'Auftrag: Erinnere dich an mich' `
  --class UChoiceGoreNpcSessionBegin --mod-name NpcSessionProof -o $DIALOG
```

Retain the generated NPC declarations and `UTopic_Hero__GORE_TEST_A` root.
Replace the generated `UChoiceGoreNpcSessionBegin` class with `choices.as` and
append `quest.as`. Both fragments contain their own namespace blocks; close
the generated namespace before appending them. The three DebugIds in this
fixture belong to this exact checkout, and `dialog check` checks collisions.

```powershell
gore dialog check $DIALOG --cache $TWO_NPCS_CACHE
```

Copy the checked A source into the complete source tree at
`AI/AIAgent/Human/Config/GORE_TEST_A/GORE_TEST_A.as`. Keep the original B module
and both original Xardas world-point spawn lines, including each actor
Blueprint, without replacing the existing bird spawns. Retain the checkout
Health/MaxHealth edits in Diego's character definition. Start this graph from
the matching pristine Shipping cache, SHA-256
`7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511`.

Compile this graph together; an Edit-only A mini cannot be installed onto
pristine Shipping, where A does not yet exist:

Use a dedicated artifact directory. On Windows, putting outputs directly in an
ancestor of the running compiler/profile directory can conflict with the
compiler's retained directory handles during atomic publication.

```powershell
New-Item -ItemType Directory $COMPILE_WORK, $ARTIFACTS
gore as compile $TREE --backend standalone --game $GAME `
  --work-dir $COMPILE_WORK -o "$ARTIFACTS/NpcSessionProof.full.Cache" `
  --mini "$ARTIFACTS/NpcSessionProof.mini.Cache" `
  --generation-receipt "$ARTIFACTS/generation-receipt.json"
```

Package the resulting multi-module mini with this build spec beside it:

```json
{
  "meta": {"name": "NpcSessionProof", "version": "0.1.0", "author": ""},
  "scripts": [{
    "op": "edit",
    "module_name": "LevelScripts.XardasTower_AI",
    "mini_cache": "NpcSessionProof.mini.Cache"
  }]
}
```

Run `gore mod build --spec spec.json --out build` and
`gore mod inspect build/NpcSessionProof`. Installation and cleanup are separate
steps using the chosen deployment owner. Preserve the original save slots.

## User game campaign

Use a preserved earlier save near Xardas with both NPCs, or a separate test game
that reaches them. The initial campaign used `G1R-003.sav`, but the game later
reused that autosave slot; use a backed-up baseline when repeating the campaign.

1. Talk to A, the NPC who looks like Diego. Select **Auftrag: Erinnere dich an
   mich**. Check the journal for **Erinnere dich an mich**. Save to a free slot
   (`running-initial`). B should still have no quest choices.
2. Exit the game completely, start it again and load that slot. A must offer
   **Ich bin wieder da. Du erinnerst dich an mich.** The initial offer must be
   gone and the quest must still be active. Do not select the return option
   yet; leave the conversation and save to another slot (`running-reloaded`).
3. Select the return option. Check that the quest is completed. Save to another
   slot (`completed-initial`).
4. Exit the game completely, start it again and load that slot. A must offer
   **Gut, dass wir Freunde geblieben sind.** The quest must remain completed.
   Save to another slot (`completed-reloaded`).
5. Separately load the earlier ToughDiego checkout save (`G1R-002.sav`) and save
   it to a free slot. Its saved Health/MaxHealth 1234 provide the checkout
   reload check; do not apply that expectation to a different, older save.

Record the four filenames, the checkout reload filename, and whether both
exits were full process restarts. Stop and report the observed menu or journal
state if any step differs. The fixture does not require fighting or attacking
any NPC to assess friendship; the exact persisted relationship is read below.

## Read-only evidence

The verifier uses the existing `gore-save` DLL and only fixed read commands:

```powershell
python scripts/npc_session_proof.py --help
python scripts/npc_session_proof.py --save $RUNNING_INITIAL --dll $SAVE_DLL `
  --phase running --output running-initial.json
python scripts/npc_session_proof.py --save $RUNNING_RELOADED --dll $SAVE_DLL `
  --phase running --previous running-initial.json --output running-reloaded.json
python scripts/npc_session_proof.py --save $COMPLETED_INITIAL --dll $SAVE_DLL `
  --phase completed --previous running-reloaded.json --output completed-initial.json
python scripts/npc_session_proof.py --save $COMPLETED_RELOADED --dll $SAVE_DLL `
  --phase completed --previous completed-initial.json --output completed-reloaded.json
```

For each phase, retain the JSON report and compare the reload report with the
corresponding initial report. Require exact A/B identities, unchanged inventory
and attributes, the correct quest state, A-owned markers, and one stable Story
modifier targeting Hero with Friend. Report comparison is supplementary:
distinct save bytes alone cannot establish that the game process restarted.

The preserved initial `G1R-003.sav` was a negative control: A and B existed, but
the fixture quest, knowledge and Story relationship had not been created yet.
Its later replacement is not that baseline.
