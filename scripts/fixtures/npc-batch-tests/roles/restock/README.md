# Late trader stock — focused follow-up

Prepared2026-09-22 as `NpcTraderRestockTest 0.1.0` for Steam build25168047.
Runtime status: **pending user test**. The completed initial-stock test and
teacher tests do not need repeating. See the [German checklist](FIELD-TEST.md).
The [build/deployment report](build-deploy.json) records the installed package;
the [START publication](start-save.json) verifies that profile4 contains only067
and all other profiles and existing save files remain unchanged.

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

Read back A's `m_Items`, `m_DefaultItems`, `m_GeneratedEvents`, the two test
markers and Hero inventory in all named result saves. Before the late event,
both stock maps must contain3/10/100. Do not infer the late default-map behavior
in advance. One diagnostic pattern would be defaults3/10/100 immediately after
delivery, then defaults5/15/120 on load while current stock incorrectly rises
to7/20/140. That would support a reload delta, not prove its native cause.

The user checks the visible shop before/after dispatch, repeated dispatch,
two full restarts and one purchase. Only that observation plus save readback
can qualify persistence. A compiled/deployed fixture is not a runtime pass.

## Rebuild

`python scripts/fixtures/npc-batch-tests/roles/restock/prepare.py` regenerates
the two authored sources from the retained economy0.1.3 sources plus
`restock.as`. [sources.json](sources.json) records their hashes. Compile them
with the campaign's two existing spawn anchors through the strict standalone
FullGraph Core route. The current build uses the qualified25168047 pristine
cache and preserves all7315 unselected original module spans byte for byte.
Original trade/crime/AI scripts are not recompiled into the deployed mini-cache.
