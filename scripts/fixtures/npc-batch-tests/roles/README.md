# NPC role test sources

Two **sequential** fixtures built from tested `NpcWarningRecovery 0.1.12`.
Run `python scripts/fixtures/npc-batch-tests/roles/prepare.py` from the repository.
It writes `work/npc-batch-tests/roles/{economy,field}/GORE_TEST_{A,B}.as`
and source hashes. See the parent BUILD.md for compilation and packaging.

Both retain every old A class, hide its 29 previous choices, and append the new
menu to A. B's complete original source is an exact prefix; its equipment,
personality, AI mapping and existing fields are unchanged. Build and inspection
results are recorded in the parent batch; game results remain pending. Use the
prepared starts below with A and B already present. Reload the same clean input
between combat cases.

## Economy / teacher

Active as `NpcEconomyRolesTest 0.1.0` since2026-09-12; Manager reports `in_sync`.
The [deployment report](economy-deployment.json) records the existing verified
bundle. No source rebuild was needed. Natural-voice testing has finished its
restart check, and the completed Voice saves are archived outside the game.

Talk to A. This test uses genuine trader inventory and the stock learning helper.

Profil 4: **067 „NPC 03 Handel - LP fehlt“** (0 LP/50 Erz),
**068 „NPC 03 Lehrer - Erz fehlt“** (5 LP/0 Erz),
**069 „NPC 03 Lehrer - bereit“** (15 LP/200 Erz).
Bei 067 und 068 zuerst 03 ausprobieren, bevor du handelst oder 04 benutzt.
Bei 069 reicht das Geld auch beim zweiten Versuch noch aus; so wird die Sperre
fuer bereits gelerntes Tauchen geprueft. 04 ist fuer diese Starts nicht notwendig.
Ergebnisse bitte `lehrer-lp-fehlt`, `lehrer-erz-fehlt`, `lehrer-gelernt`,
`lehrer-neustart` und nach dem Handel `rolle-handel` nennen.

| Choice | Test / expected observation |
| --- | --- |
| 01 | Seed A's trade compartment once: 3 cheese, 10 arrows, 100 ore. |
| 02 | Open trade, buy 1 cheese, then sell it back. Check both inventories and ore changes; cancel a second pending transaction and confirm no transfer. |
| 03, before 04 | On an input without Diving and with fewer than 5 LP **or** 30 ore, learning must fail and charge nothing. For separate LP/ore gates, use inputs satisfying only the other requirement. The fixture never removes existing resources to manufacture this state. |
| 04 | Optional, once per save: explicitly grants 5 LP and 50 ore for the positive test. It does not reset resources or unlearn skills. |
| 03 | With Diving unknown and both resources sufficient, learn it: precisely 5 LP and 30 ore deducted. Repeat: already learned must cause no second charge. |
| 05 | Record current ore/LP, skill and requirement checks. Save, reload, talk again and repeat 05: Diving and changed stock must persist; 01 and 04 must stay unavailable if already used in that save. |

If the input already knows Diving, its positive learning case is inapplicable:
use an earlier test-save copy without the skill. Never count a skipped gate as
tested. Stock prices depend on shipped trading rules; compare displayed prices
and actual ore deltas, not a fabricated fixed price.

## Field roles / lifecycle / mixed weapons

Talk to A. B is the subject; keep A alive as the control menu. Start each combat,
faction, flee or death case from the same clean copied save, rather than carrying
hostility or injuries from a previous case.

Profil 4: **070 „NPC 04 Rollen - START“** fuer die unveraenderte Waffen-Diagnose
und Folgen/Warten. B steht auf der freien Flaeche vor dem Alten Lager; A steht
am Tor etwa 30 Meter hinter dir. Dort 30 einschalten, dann innerhalb der
60 Sekunden zu B zurueckgehen. **072 „NPC 04 Rollen - Kampf bereit“** hat einen
verstaerkten Helden fuer die positiven Niederlage-/Tod-Tests; Bs Werte bleiben
unveraendert. Zwischen den Kampf-, Gilden- und Fluchtzweigen frisch laden.

Ergebnisse bitte passend benennen: `rolle-waffenwahl`, `rolle-folgen`,
`rolle-warten`, `rolle-niederlage`, `rolle-feind-besiegt`, `rolle-gilde-neutral`,
`rolle-gilde-zurueck`, `rolle-flucht`, `rolle-tod`, `rolle-wiederbelebt`.
Falls Speichern wegen eines aktiven Konflikts gesperrt bleibt, diesen Befund
melden; der fehlende Spielstand ist dann kein Grund, den Test zu erzwingen.

| Choice | Test / expected observation |
| --- | --- |
| 20 / 21 / 20 | Follow, stop at current area, resume. Walk away after each; check B follows only while requested. Save/load once following and once waiting. |
| 22 | Stock training fight with a defeat goal, both weapons retained. Win and observe defeat/recovery without a death goal. |
| 23 | B becomes Hero's enemy until defeat. Observe actual hostility/combat and relationship reset after defeat. This is a personal modifier, not a guild change. |
| 24 / 25 | Change **B only** to guild None, then restore his original ShadowLeader guild. Inspect saved guild and relationships. Do this on a separate clean branch from 23 so its personal modifier cannot mask faction effects. |
| 26 / 27 | Change B's actual unfavorable-combat policy to Always-flee, introduce Hero as enemy, observe fleeing. 27 restores the captured original policy; it does not erase conflict memory. Reload the clean input afterward. |
| 28 | Stock training fight with a death goal. Kill B through combat/execution and verify the corpse/dead state persists through save/load. This is deliberately separate from ordinary defeat. |
| 29 | Available only while B is dead: explicitly enable his same-NPC native timed revival policy. Leave the area and allow at least one game hour; return and inspect. Verify the **same global ID**, living state, one B only, and retained inventory/state. Then use 21 to disable this policy again. No actor is spawned by this choice. Native simulation/distance timing is an open runtime test, not a guaranteed one-hour visible pop-in. |
| 30 | Arm a 60-second passive mixed-weapon trace. Close dialogue and reproduce warning sword → combat bow on the known input, keeping both weapons and arrows. Save promptly after the transition. |
| 31 | Record B's health, dead/defeated, relationship, follow and flee state before saving. |

Run weapon choice 30 on the unchanged combat input **before** faction/flee or
training controls. The trace reads B's actual selected item, equipped item,
conflict phase, character-of-interest, distance and the exact 100-unit grounded
reachability query used by the close-target multiplier. It never calls selection,
draw, inventory-update or scoring-manager functions. It captures changes and
the first ten combat samples, at most 40 rows. This tests a concrete cause of
the already-reproduced transition; it is not another generic terrain comparison.

World-float keys `gore_role_weapon_<row>_<field>` contain `seconds`, `phase`
(0 other / 1 warning / 2 combat), `selected` and `equipped` (0 none / 1 Diego
sword / 2 Diego bow / 3 other), `target` (0 none / 1 player / 2 other), `distance`,
`reachable` (-1 no target / 0 false / 1 true), and `close_gate`.
`gore_role_weapon_samples` bounds valid rows; older rows above it are ignored.
If no rows are produced, the tracer did not run: do not infer combat values.
The capture samples conditions, **not internal winning scores** or a proof of
vanilla/mod attribution. In particular, a true close gate does not by itself
prove which item ought to win after all other multipliers.

Readback must distinguish command markers from observed results. For example,
`gore_role_revive_enabled=1` only proves the policy was requested, and
`gore_role_follow_command=1` only proves the follow request was sent. Record
visible behavior, original/final saves and exact inventory/resource deltas.
