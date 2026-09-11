# Equipped weapon and combat-start voice

Prepared on2026-09-10 after the user passed0.1.12's bare-fists recovery tests,
including persistent escalation after reloading. No new mod build is required.

The first import of61 failed game visibility: the user could not find it in any
profile. Although the summary reported slot61/name/profile2, the actual public
`SaveGamePublicData` branch still named slot54 and its old title. Only the first
`SaveDataPayload` branch had changed. The cached profile entry also had blank
map and zeroed played-time fields. The preparation report below records that
first import, not a successful in-game load.
The actual central `m_SavedGamesNames` array also omitted61 entirely (60 entries,
including54/60). Registration in the per-profile array and public-data map alone
was therefore incomplete.

The [core/import repair](import-repair-result.json) was applied on2026-09-10,
after passing a detached preview and library tests (471 passed,22 ignored).
Both public identity copies now agree; the cached row copies the incoming public
metadata, and the central list contains61 exactly once. Existing registered slots
can be repaired through the same assignment API. The game was closed, backups
were created, private gameplay bytes and all other numbered saves were unchanged.
The user's subsequent weapon test confirms that the repaired input loaded.

Embedded `m_TimeLoaded` is0 in original54 as well as61;54's profile cache had a
later loaded-time value. The repair intentionally copies the embedded metadata.
The source date is retained, so61 may appear near54 rather than at the top.

## Prepared input

Slot61, **`npc voice - waffentest start`**, belongs to the same internal profile2
as original54. It is a separate copy of54 (`npc sitz-wache - wache`) with a new
save name and one unequipped `/Script/Angelscript.ItMw_1H_Sword_Old_01`.
The sword requires Strength5; the saved Hero has10. The original inventory had
only Letter and Glossary, so asking the player simply to equip a weapon would
not have been actionable.

The [preparation result](preparation-result.json) records original/output hashes,
item readback, unchanged B position/routine and unchanged numbered original saves.
The supported `write_save` route created a detached copy; `assign_save_profile`
registered it in the existing profile, backed up `PersistentDataList.sav` and
refused any existing destination. The game was closed. This generated save is
test input, not evidence of the tested gameplay outcome.

## Runtime result — combat entry and voice passed

On2026-09-10 the user confirmed that B attacks and speaks at combat onset, with
examples remembered as "jetzt gibts aufs maul" or "na dann mal los". This qualifies
the equipped-sword escalation/voice path. It does not identify an exact recording
or qualify all combat behavior. Save62 is named **`npc voice - schwertwarnung`**;
its existence records a saved warning-test checkpoint. This latest message did
not separately describe the cleanup sequence after sheathing.

The user also observed B frequently drawing his bow while the Hero stood next to
him, sometimes switching to the sword just before drawing the bowstring. This
is an open weapon-selection observation; the voice pass does not resolve it.
The [runtime record](runtime-result.json) separates user observations from saved
equipment and offline analysis. B retains Diego's archer personality and both
weapons. Actual saves61/62 contain Strength120/Dexterity180 and100 arrows, so the
known sword/bow requirements are satisfied. No combat tuning has been changed in
response to this report. See [weapon-selection analysis](WEAPON-SELECTION.md).

On2026-09-11 the user supplied63, `npc kampf - freie flaeche`. Separate test64,
**`npc kampf - B freie flaeche`**, now places B two metres ahead of the Hero and
disables only his daily routine in that copy. Weapons and combat AI are retained.
The [input record](flat-ground-input.json) records verified placement metadata,
unchanged originals, and profile backup/registration. The user then tested64:
B draws his sword during the warning and switches to his bow on combat escalation.
The problem therefore also occurs away from the hut, with its routine disabled.
See the [comparison result](flat-ground-runtime-result.json); runtime cause and
vanilla/mod attribution remain open.

## Test procedure used

1. Load **`npc voice - waffentest start`**, without running setup. Equip the
   added sword and visibly draw it near B, at the spot where fists worked.
   Wait for B's spoken warning, then sheathe it. Check that B ends his warning
   and that saving nearby works. Save as **`npc voice - schwertwarnung`**.
2. Reload the original prepared slot61. Draw the sword near B and leave it
   drawn through escalation; do not strike first. Observe whether B starts
   fighting and whether he speaks a distinct line at combat onset. Report
   B's reaction and any words heard separately from nearby NPC speech.
3. Reload slot61 after observing that transition. Winning the fight or saving
   during combat is unnecessary.

B uses `VoiceType_G1R_Voice05_Diego`. The shipped leader response to exhausted
weapon warnings requests `Sound_Voice_Reaction_Combat_Start` with
`VoicelineContext_AskedForIt` before entering combat
(`AIARM_Crime_WeaponOrFistsDrawn.as`,
`UAIARM_Crime_IgnoredWarning_WeaponDrawn_Responses`). The request is conditional:
another NPC leading the warning group, voice availability and concurrent speech
can affect who speaks or whether a separate line is audible. The exact archive
member is not established by merely hearing the reaction.

Ordinary/routine voice, an additional voice profile and further NPC roles remain
on the [open list](../../../docs/guide/npc-open-items.md).
