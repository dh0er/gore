import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/ui/actor_detail_header.dart';
import 'package:goresave/features/editor/ui/attribute_detail.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/features/editor/ui/inventory_detail.dart';
import 'package:goresave/features/editor/ui/progression_panel.dart'
    show KnowledgeDetail, EventsDetail;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// The unified "Charaktere" tab: a shared [CharacterMasterList] on the left and
/// four detail sub-tabs on the right (Attribute · Inventar · Wissen ·
/// Ereignisse). The selected character is the SHARED editor state
/// (`state.selectedActor` / `notifier.selectActor`), so switching sub-tabs keeps
/// the same actor. Each sub-tab body is kept alive across sub-tab switches so
/// pending edits (and their pending-registry entries) survive.
///
/// Orphan characters (knowledge-only, no spawned actor / GlobalId) have no
/// attributes, inventory, or events; for an orphan selection those three
/// sub-tabs show a clean empty state and only Wissen is wired up.
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
    final locCatalog =
        ref.watch(locCatalogProvider).asData?.value ?? const {};

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
          );

    // Wissen (knowledge) always works: the player's key is 'Hero' (which IS
    // selected.uniqueName for the player), an NPC's / orphan's key is its
    // uniqueName. Passing selected.uniqueName covers all three. The same
    // ActorDetailHeader the Attribute/Inventar bodies render sits above the
    // card (same Column + Expanded structure as AttributeDetail) so all four
    // sub-tabs identify the selection identically; it handles player (no id),
    // NPC (full GlobalId), and orphan (uniqueName-resolved, no id line).
    final Widget knowledgeBody = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ActorDetailHeader(
          actor: selected,
          locCatalog: locCatalog,
          lang: lang,
        ),
        Expanded(
          child: KnowledgeDetail(
            uniqueName: selected.uniqueName,
            notifier: notifier,
            editable: progressionEditable,
            reloadKey: inspection,
            theme: theme,
          ),
        ),
      ],
    );

    // Ereignisse (events) are keyed by GlobalId. The player's events live
    // under the save's own Hero ACTOR GlobalId, stashed in
    // `state.heroGlobalId` when the character index loads (null until then →
    // the detail's own empty state; EventsDetail re-selects when the id
    // arrives). Orphans have no GlobalId → null → empty state. The header
    // always renders (the detail below shows its own empty state when the
    // globalId is null), matching the other sub-tabs.
    final Widget eventsBody = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ActorDetailHeader(
          actor: selected,
          locCatalog: locCatalog,
          lang: lang,
        ),
        Expanded(
          child: EventsDetail(
            globalId: selected.isPlayer
                ? state.heroGlobalId
                : (selected.isOrphan ? null : selected.id),
            notifier: notifier,
            editable: progressionEditable,
            reloadKey: inspection,
            theme: theme,
          ),
        ),
      ],
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
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: DefaultTabController(
            length: 4,
            child: Column(
              children: [
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
                      icon: const Icon(Icons.school_outlined),
                      text: l10n.dialogKnowledge,
                    ),
                    Tab(
                      icon: const Icon(Icons.history_outlined),
                      text: l10n.sectionEvents,
                    ),
                  ],
                ),
                Expanded(
                  child: TabBarView(
                    children: [
                      _KeepAliveTab(child: attributeBody),
                      _KeepAliveTab(child: inventoryBody),
                      _KeepAliveTab(child: knowledgeBody),
                      _KeepAliveTab(child: eventsBody),
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
