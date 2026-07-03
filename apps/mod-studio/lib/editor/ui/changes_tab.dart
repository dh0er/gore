import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/ui/game_path_scope.dart';
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../audio/ui/audio_tab.dart';
import '../../catalog/ui/items_tab.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../dialog/domain/dialog_catalog_provider.dart';
import '../../dialog/ui/dialoge_tab.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../scripts/domain/script_mods_notifier.dart';
import '../../scripts/domain/script_modules_provider.dart';
import '../../scripts/ui/script_tab.dart';
import '../../textures/domain/texture_index_provider.dart';
import '../../textures/domain/texture_replacements_notifier.dart';
import '../../textures/ui/texture_tab.dart';
import '../domain/overrides_notifier.dart';
import 'overrides_panel.dart';

/// Sidebar sections of the Changes tab.
enum _ChangesSection { all, items, dialogs, audio, textures, scripts }

/// The Änderungen/Changes main tab: a 230px sidebar with one entry per
/// change domain (plus "All") and a content pane showing either the flat
/// [OverridesPanel] or the matching main-tab view filtered down to the
/// staged changes of that domain.
class ChangesTab extends ConsumerStatefulWidget {
  const ChangesTab({super.key});

  @override
  ConsumerState<ChangesTab> createState() => _ChangesTabState();
}

class _ChangesTabState extends ConsumerState<ChangesTab> {
  _ChangesSection _section = _ChangesSection.all;

  /// Sections shown at least once — mirrors the visited semantics of the
  /// main tabs' TabReentryListener (home_page) for the embedded
  /// install-bound views. On FIRST entry a section's data providers are
  /// created fresh by that very build, so invalidating would double-fetch.
  /// On RE-entry they may have stayed alive the whole time (the keep-alive
  /// main tabs watch the same providers), so without a refresh a
  /// deploy/undeploy between visits would leave this tab showing a stale
  /// texture index / script module list.
  final Set<_ChangesSection> _visited = {_ChangesSection.all};

  void _selectSection(_ChangesSection section) {
    if (section == _section) return;
    if (!_visited.add(section)) {
      // Re-entry only (never per build, never on first display).
      if (section == _ChangesSection.textures) {
        ref.invalidate(textureIndexProvider);
      } else if (section == _ChangesSection.scripts) {
        ref.invalidate(scriptModulesProvider);
      }
      // Audio is deliberately NOT invalidated: main-tab re-entry
      // (TabReentryListener in home_page) doesn't refresh the audio
      // providers either — keep the two entry paths in parity.
    }
    setState(() => _section = section);
  }

  /// Distinct dialog loc ids among the staged loc edits — drives BOTH the
  /// Dialoge sidebar count and the [DialogeTab.onlyIds] filter, so the two
  /// can't disagree: non-dialog loc edits (e.g. item names staged via the
  /// field editor) are excluded from this section and stay reviewable under
  /// "All", and an id edited in several languages counts once.
  ///
  /// Intersected with the loaded loc catalog: the embedded [DialogeTab] can
  /// only render ids the catalog carries (its [buildDialogRows] iterates
  /// catalog keys), so a staged dialog id absent from the catalog — or any
  /// dialog edit while the catalog is still loading/empty — must not inflate
  /// the badge past the browsable rows. Such edits stay reviewable under "All".
  ///
  /// Cached in state behind a content compare: [locEditsProvider] emits a
  /// new edits map per keystroke even when the edited ID SET is unchanged,
  /// and a stable set identity lets the dialog browser's [DialogRowsMemo]
  /// skip re-scanning the catalog on those rebuilds.
  Set<String> _dialogIds = const {};

  Set<String> _dialogIdsFor(
    LocEditsState locState,
    Map<String, Map<String, String>> catalog,
  ) {
    final ids = <String>{
      for (final id in locState.edits.keys)
        if (isDialogLocId(id) && catalog.containsKey(id)) id,
    };
    if (!setEquals(ids, _dialogIds)) _dialogIds = ids;
    return _dialogIds;
  }

  @override
  Widget build(BuildContext context) {
    final overridesState = ref.watch(overridesProvider);
    final locState = ref.watch(locEditsProvider);
    final locCatalog =
        ref.watch(locCatalogProvider).value ?? const <String, Map<String, String>>{};
    final dialogIds = _dialogIdsFor(locState, locCatalog);
    final audioState = ref.watch(audioReplacementsProvider);
    final textureState = ref.watch(textureReplacementsProvider);
    final scriptState = ref.watch(scriptModsProvider);

    // Same arithmetic as the OverridesPanel header count.
    final total = overridesState.count +
        locState.entryCount +
        audioState.count +
        textureState.count +
        scriptState.count;

    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    final entries = <({
      _ChangesSection section,
      IconData icon,
      String label,
      int count,
    })>[
      (
        section: _ChangesSection.all,
        icon: Icons.pending_actions_outlined,
        label: l10n.changesAll,
        count: total,
      ),
      (
        section: _ChangesSection.items,
        icon: Icons.inventory_2_outlined,
        label: l10n.tabItems,
        count: overridesState.count,
      ),
      (
        section: _ChangesSection.dialogs,
        icon: Icons.forum_outlined,
        label: l10n.tabDialogs,
        count: dialogIds.length,
      ),
      (
        section: _ChangesSection.audio,
        icon: Icons.audiotrack_outlined,
        label: l10n.tabAudio,
        count: audioState.count,
      ),
      (
        section: _ChangesSection.textures,
        icon: Icons.texture,
        label: l10n.tabTextures,
        count: textureState.count,
      ),
      (
        section: _ChangesSection.scripts,
        icon: Icons.code,
        label: l10n.tabScripts,
        count: scriptState.count,
      ),
    ];

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Left: domain sidebar, mirroring the catalog browser sidebar styling.
        SizedBox(
          width: 230,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
            ),
            child: ListView(
              padding: const EdgeInsets.symmetric(vertical: 6),
              children: [
                for (final e in entries)
                  SidebarTile(
                    icon: e.icon,
                    label: l10n.categoryWithCount(e.label, e.count),
                    selected: e.section == _section,
                    onTap: () => _selectSection(e.section),
                  ),
              ],
            ),
          ),
        ),
        const VerticalDivider(width: 1),
        // Right: the selected section's view. Content is swapped per
        // selection rather than kept alive in an IndexedStack: an
        // IndexedStack would mount and build all six children up front,
        // paying hidden provider costs (audio bank loads, texture index,
        // script module list) for sections the user may never open. The
        // trade-off is that a filtered view's local UI state (search text,
        // tree expansion) resets on section switch — acceptable for a
        // review surface; the main tabs keep their own state independently.
        Expanded(child: _buildContent(overridesState, dialogIds)),
      ],
    );
  }

  Widget _buildContent(OverridesState overridesState, Set<String> dialogIds) {
    switch (_section) {
      case _ChangesSection.all:
        // The flat all-domains list: top-left aligned at a comfortable
        // reading width instead of stretching across the whole pane.
        return Align(
          alignment: Alignment.topLeft,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 720),
              child: const OverridesPanel(),
            ),
          ),
        );
      case _ChangesSection.items:
        return ItemsTab(
          onlyIds: {for (final e in overridesState.entries) e.classId},
        );
      case _ChangesSection.dialogs:
        return DialogeTab(onlyIds: dialogIds);
      // The install-bound views keep the main tabs' GamePathScope so a game
      // path change drops any subtree state tied to the previous install.
      case _ChangesSection.audio:
        return const GamePathScope(child: AudioTab(onlyStaged: true));
      case _ChangesSection.textures:
        return const GamePathScope(child: TextureTab(onlyStaged: true));
      case _ChangesSection.scripts:
        return const GamePathScope(child: ScriptTab(onlyStaged: true));
    }
  }
}
