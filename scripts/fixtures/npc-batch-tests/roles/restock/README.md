# Late trader stock — focused follow-up

Completed2026-09-22 as `NpcTraderRestockTest 0.1.0` for Steam build25168047.
Runtime status: **passed**, confirmed by the user and five result saves in the
[runtime report](runtime-0.1.0.json). No repeat is needed. The [German checklist](FIELD-TEST.md)
is retained for reproduction.
The [build/deployment report](build-deploy.json) records the installed package;
the [START publication](start-save.json) records the original setup. All six
START/result saves are now [archived outside the game](../../profile-cleanup-after-restock-complete.json).
Profile4 is empty; other profiles and existing unrelated save files are unchanged.

This fixture retains every class from the passed economy0.1.3 A/B sources,
hides the completed teacher/test-funds menus and registers both stock batches
in the same `UTraderConfig_GoreRoleEconomy` from the start:

| Event | Cheese | Arrows | Ore |
|---|---:|---:|---:|
| `OnWorldStart` | 3 | 10 | 100 |
| `GoreNpcRestockTrialV1` | 2 | 5 | 20 |

The inherited ore/arrow difficulty multipliers remain1. Choice01 opens the
shop without a setup/grant. Choice02 dispatches the late global event once,
guarded by saved `gore_restock_trial_v1`. Choice03 deliberately dispatches the
**same event name again**, to test native batch deduplication independently of
the menu guard. `gore_restock_event_calls_v1` records dispatch count, not proof
that goods were granted. No NPC inventory seeding or manual stock-map repair.

The fresh START067 has no global shop row for A. Its archived private save
bytes are preserved, including the noon clock. Loading with this fixture must
initialize both current and default stock through `OnWorldStart`. The supplied
shop config supplies the later event before the first initialization as well.

## Evidence boundary

The previous [late-event failure](../economy-runtime-0.1.2.json) had populated
current stock, empty defaults, and duplicate goods after loading. The surviving
event ledger/once marker rule out a lost menu guard. They do **not** establish
whether missing initial stock or a general late-batch baseline delta caused it.
The shipped scripts register late chapter batches and dedicated events, e.g.
`OnOrcEnclaveFightStarted` for Cronos/BaalCadar. The event mechanism is valid;
its native saved-stock implementation is not present in the script sources.

Readback of A's `m_Items`, `m_DefaultItems`, `m_GeneratedEvents`, both test
markers and Hero inventory confirms:

| Save | Current cheese/arrows/ore | Defaults | Event calls |
|---|---|---|---:|
|003 `nachschub - vorher`|3/10/100|3/10/100|0|
|022 `nachschub-geliefert`|5/15/120|3/10/100|1|
|023 `nachschub-wiederholt`|5/15/120|3/10/100|2|
|024 `nachschub-gehandelt`|4/15/127|5/15/120|2|
|025 `nachschub-neustart`|4/15/127|5/15/120|2|

The ledger contains the late event once throughout022–025. Hero keeps50 ore
until the purchase, then1 cheese/43 ore across the second restart. The user
confirms both full restarts and the visible UI checks. Default reconstruction
does occur, but does not duplicate current stock in this path; that update
alone cannot explain the older failure. Its exact native cause remains open.
This qualifies the tested initial-plus-late batch, not periodic unlimited restock.

## Rebuild

`python scripts/fixtures/npc-batch-tests/roles/restock/prepare.py` regenerates
the two authored sources from the retained economy0.1.3 sources plus
`restock.as`. [sources.json](sources.json) records their hashes. Compile them
with the campaign's two existing spawn anchors through the strict standalone
FullGraph Core route. The current build uses the qualified25168047 pristine
cache and preserves all7315 unselected original module spans byte for byte.
Original trade/crime/AI scripts are not recompiled into the deployed mini-cache.
