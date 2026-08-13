import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/ui/actor_detail_header.dart';
import 'package:goresave/features/editor/ui/attribute_detail.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/features/editor/ui/inventory_detail.dart';
import 'package:goresave/features/editor/ui/position_detail.dart';
import 'package:goresave/features/editor/ui/trader_detail.dart';
import 'package:goresave/features/editor/ui/progression_panel.dart'
    show KnowledgeDetail, EventsDetail;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// The unified "Charaktere" tab: a shared [CharacterMasterList] on the left and
/// five detail sub-tabs on the right (Attribute · Inventar · Wissen ·
/// Ereignisse · Position). The selected character is the SHARED editor state
/// (`state.selectedActor` / `notifier.selectActor`), so switching sub-tabs keeps
/// the same actor. Each sub-tab body is kept alive across sub-tab switches so
/// pending edits (and their pending-registry entries) survive.
///
/// Orphan characters (knowledge-only, no spawned actor / GlobalId) have no
/// attributes, inventory, events, or stored position; for an orphan selection
/// those four sub-tabs show a clean empty state and only Wissen is wired up.
///
/// The [ActorDetailHeader] sits once above the secondary tab bar. It therefore
/// identifies the persistent character context instead of being duplicated in
/// each Attribute/Inventory/Knowledge/Events page. Every sub-tab body then uses
/// `Padding(EdgeInsets.fromLTRB(20, 8, 20, 20))` → one `Card` →
/// `Padding(EdgeInsets.all(16))` → content. Card titles are intentionally absent
/// because the sub-tab labels already name the views.
class CharactersTab extends ConsumerWidget {
  const CharactersTab({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.attributeEditable,
    required this.inventoryCanCompress,
    required this.progressionEditable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;

  /// Same gating expression the old `_AttributePanel` received
  /// (`inspection.privateEditable && state.codecCompressReady`).
  final bool attributeEditable;

  /// Same value the old `_InventoryPanel` received for `canCompress`
  /// (`state.codecCompressReady`).
  final bool inventoryCanCompress;

  /// Same gating the Welt (World) tab passes its quests/factions details — the
  /// old Progression tab's convention
  /// (`inspection.privateEditable && inspection.privateTypedVerified &&
  /// state.codecCompressReady`).
  final bool progressionEditable;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final state = ref.watch(editorProvider);
    final selected = state.selectedActor;
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);

    // Orphans have no actor-backed data: guard Attribute/Inventar/Ereignisse to
    // a clean empty state so they never issue an NPC load with the `orphan:`
    // sentinel id. Only Wissen works for an orphan (keyed by its uniqueName).
    final isOrphan = selected.isOrphan;

    final Widget attributeBody = isOrphan
        ? _MessagePane(
            icon: Icons.person_outline,
            title: l10n.tabAttribute,
            body: l10n.characterNoActorBody,
          )
        : AttributeDetail(
            inspection: inspection,
            notifier: notifier,
            editable: attributeEditable,
            actor: selected,
            showActorHeader: false,
          );

    final Widget inventoryBody = isOrphan
        ? _MessagePane(
            icon: Icons.inventory_2_outlined,
            title: l10n.tabInventory,
            body: l10n.characterNoActorBody,
          )
        : InventoryDetail(
            inspection: inspection,
            notifier: notifier,
            actor: selected,
            canCompress: inventoryCanCompress,
            showActorHeader: false,
          );

    // Handel: a merchant's shop, which is NOT his inventory — it lives in a
    // global array keyed by uniqueName alone. An orphan therefore gets it too:
    // the record does not depend on a spawned actor, and the panel already
    // shows a clean non-merchant state when no row matches the name.
    final Widget tradeBody = TraderPanel(
      inspection: inspection,
      notifier: notifier,
      actor: selected,
      editable: progressionEditable,
      // Carries the inspection, not just the actor: a save re-inspects the
      // file, and without that the kept-alive panel would keep showing the
      // pre-save stock.
      reloadKey: (inspection, selected.uniqueName),
    );

    // Position: the player's transform editor (its only home — it used to sit
    // in the Attribute tab's HeroStatsCard sidebar) and, for an NPC, the saved
    // pose from `private.npc.position` (editable again while the placement
    // question is open — see NpcPositionPanel). Orphans have no actor and
    // therefore no stored pose, so they get the same clean empty state as
    // Attribute/Inventar/Ereignisse.
    final Widget positionBody = isOrphan
        ? _MessagePane(
            icon: Icons.place_outlined,
            title: l10n.heroTransform,
            body: l10n.characterNoActorBody,
          )
        : PositionDetail(
            inspection: inspection,
            notifier: notifier,
            editable: attributeEditable,
            actor: selected,
          );

    // Wissen (knowledge) always works: the player's key is 'Hero' (which IS
    // selected.uniqueName for the player), an NPC's / orphan's key is its
    // uniqueName. Passing selected.uniqueName covers all three.
    final Widget knowledgeBody = Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: KnowledgeDetail(
        uniqueName: selected.uniqueName,
        notifier: notifier,
        editable: progressionEditable,
        reloadKey: inspection,
        theme: theme,
      ),
    );

    // Ereignisse (events) are keyed by GlobalId. Orphans have no GlobalId (and
    // so no events): like Attribute/Inventar they get the clean no-actor empty
    // state instead of the detail (whose null branch is the misleading
    // "select a character" prompt). The player's events live under the save's
    // own Hero ACTOR GlobalId, stashed in `state.heroGlobalId` when the
    // character index loads. While the index load is in flight
    // (`heroGlobalIdSettled` false) a spinner holds the pane; once it settles
    // WITHOUT an id (index failed or carried no hero row) the pane shows the
    // no-events empty state instead of spinning forever (the master list
    // separately surfaces a load error with retry); with the id, EventsDetail
    // mounts keyed by it.
    final Widget eventsBody = isOrphan
        ? _MessagePane(
            icon: Icons.history_outlined,
            title: l10n.sectionEvents,
            body: l10n.characterNoActorBody,
          )
        : selected.isPlayer && state.heroGlobalId == null
        ? (state.heroGlobalIdSettled
              // The index load completed without a hero id (error or no hero
              // row): no id is coming, so settle to the clean no-events empty
              // state — never an eternal spinner.
              ? _MessagePane(
                  icon: Icons.history_outlined,
                  title: l10n.sectionEvents,
                  body: l10n.characterNoEventsBody,
                )
              // Character-index load still in flight: the hero id is not known
              // yet, so show progress rather than mounting the detail with a
              // null id.
              : const Center(child: CircularProgressIndicator()))
        : Padding(
            padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
            child: EventsDetail(
              globalId: selected.isPlayer ? state.heroGlobalId : selected.id,
              notifier: notifier,
              editable: progressionEditable,
              reloadKey: inspection,
              theme: theme,
              relationshipNpcId: selected.isPlayer ? null : selected.id,
              relationshipEditable: attributeEditable,
            ),
          );

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          width: 365,
          child: CharacterMasterList(
            selected: selected,
            onSelect: notifier.selectActor,
            load: notifier.loadAllCharacters,
            // Reload the character list when the inspected save changes (new
            // inspection identity from a save/refresh), so it never shows the
            // previous file's characters/badges against the new save.
            reloadKey: inspection,
            locCatalog: locCatalog,
            lang: lang,
            showObjectIds: showObjectIds,
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: DefaultTabController(
            length: 6,
            child: Column(
              children: [
                ActorDetailHeader(
                  actor: selected,
                  locCatalog: locCatalog,
                  lang: lang,
                  showObjectIds: showObjectIds,
                ),
                const SizedBox(
                  key: ValueKey('actor-header-tab-gap'),
                  height: 12,
                ),
                TabBar(
                  tabs: [
                    Tab(
                      icon: const Icon(Icons.person_outline),
                      text: l10n.tabAttribute,
                    ),
                    Tab(
                      icon: const Icon(Icons.inventory_2_outlined),
                      text: l10n.tabInventory,
                    ),
                    Tab(
                      icon: const Icon(Icons.storefront_outlined),
                      text: l10n.tabTrade,
                    ),
                    Tab(
                      icon: const Icon(Icons.school_outlined),
                      text: l10n.dialogKnowledge,
                    ),
                    Tab(
                      icon: const Icon(Icons.history_outlined),
                      text: l10n.sectionEvents,
                    ),
                    Tab(
                      icon: const Icon(Icons.place_outlined),
                      text: l10n.heroTransform,
                    ),
                  ],
                ),
                Expanded(
                  child: TabBarView(
                    children: [
                      _KeepAliveTab(child: attributeBody),
                      _KeepAliveTab(child: inventoryBody),
                      _KeepAliveTab(child: tradeBody),
                      _KeepAliveTab(child: knowledgeBody),
                      _KeepAliveTab(child: eventsBody),
                      _KeepAliveTab(child: positionBody),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

/// Keeps a sub-tab's widget tree alive when the user switches to another sub-tab
/// so unsaved field state (and the matching pending-edit registry entries) stay
/// consistent. A private copy of the same widget in `editor_page.dart` (kept
/// per-file so this tab has no cross-file dependency on a private class).
class _KeepAliveTab extends StatefulWidget {
  const _KeepAliveTab({required this.child});

  final Widget child;

  @override
  State<_KeepAliveTab> createState() => _KeepAliveTabState();
}

class _KeepAliveTabState extends State<_KeepAliveTab>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context); // required by AutomaticKeepAliveClientMixin
    return widget.child;
  }
}

/// Centered icon + title + body message pane for the orphan empty states. A
/// private copy of the same widget in `editor_page.dart`.
class _MessagePane extends StatelessWidget {
  const _MessagePane({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  icon,
                  size: 48,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 12),
                Text(title, style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                Text(body, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
