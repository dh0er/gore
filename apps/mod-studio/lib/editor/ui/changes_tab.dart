import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart'; // StateProvider

import '../../app/domain/asset_entry_tracker.dart';
import '../../app/ui/game_path_scope.dart';
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../audio/ui/audio_tab.dart';
import '../../catalog/ui/items_tab.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../dialog/domain/dialog_catalog_provider.dart';
import '../../dialog/ui/dialoge_tab.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../project/dialog_topics_notifier.dart';
import '../../scripts/domain/script_mods_notifier.dart';
import '../../scripts/domain/script_modules_provider.dart';
import '../../scripts/ui/script_tab.dart';
import '../../textures/domain/texture_replacements_notifier.dart';
import '../domain/overrides_notifier.dart';
import 'overrides_panel.dart';

/// Sidebar sections of the Changes tab.
enum _ChangesSection { all, items, dialogs, audio, scripts }

/// The Changes-tab sections whose content is backed by an install-bound data
/// provider (the script module list) rather than staged state.
enum ChangesAssetSection { scripts }

/// The asset-backed section the (kept-alive) Changes tab currently embeds,
/// or null while a non-asset section (All/Items/Dialogs/Audio) is shown —
/// the default matches the tab's initial "All" section.
///
/// Published by [ChangesTab] on section selection so home_page's main-tab
/// entry handler can refresh exactly the provider backing the embedded
/// view, in parity with the standalone Scripts tab entry.
/// Without it, leaving the Changes MAIN tab parked on Scripts and re-entering
/// it later would keep showing a stale script module list after a deploy,
/// undeploy, or game patch until the user manually switched sections.
final changesAssetSectionProvider = StateProvider<ChangesAssetSection?>(
  (ref) => null,
);

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

  void _selectSection(_ChangesSection section) {
    if (section == _section) return;
    // Entering an install-bound asset section refreshes its shared data
    // provider — unless this is the very first time that asset kind is
    // shown ANYWHERE this session (then this very build creates the
    // provider fresh, and invalidating would double-fetch). The gate is the
    // session-wide tracker rather than a per-surface visited set on
    // purpose: the standalone Scripts main tab watches the same
    // autoDispose provider and, kept alive, can hold a value from before a
    // deploy, undeploy, or game patch — so even this tab's FIRST section
    // entry may hit a stale provider. (Runs on section taps only — never
    // during a build.)
    final tracker = ref.read(assetEntryTrackerProvider);
    if (section == _ChangesSection.scripts &&
        tracker.shouldInvalidateOnEntry(AssetKind.scriptModules)) {
      ref.invalidate(scriptModulesProvider);
    }
    // Audio is deliberately NOT invalidated: main-tab entry
    // (TabEntryListener in home_page) doesn't refresh the audio providers
    // either — keep the two entry paths in parity.
    setState(() {
      _section = section;
      // Publish the embedded asset section for home_page's main-tab
      // entry refresh. Safe to write here: sections change only via
      // sidebar taps, never during another consumer's build. (The initial
      // "All" needs no write — it matches the provider's null default.)
      ref.read(changesAssetSectionProvider.notifier).state = switch (section) {
        _ChangesSection.scripts => ChangesAssetSection.scripts,
        _ChangesSection.all ||
        _ChangesSection.items ||
        _ChangesSection.dialogs ||
        _ChangesSection.audio => null,
      };
    });
  }

  /// Distinct dialog loc ids among the staged loc edits. These drive the
  /// [DialogeTab.onlyIds] filter and the localization portion of the Dialogs
  /// sidebar count; staged runtime topics add to that count independently.
  /// Non-dialog loc edits remain excluded, and an id edited in several
  /// languages still counts once.
  ///
  /// Cached in state behind a content compare: [locEditsProvider] emits a
  /// new edits map per keystroke even when the edited ID SET is unchanged,
  /// and a stable set identity lets the dialog browser's [DialogRowsMemo]
  /// skip re-scanning the catalog on those rebuilds.
  Set<String> _dialogIds = const {};

  Set<String> _dialogIdsFor(LocEditsState locState) {
    final ids = <String>{
      for (final id in locState.edits.keys)
        if (isDialogLocId(id)) id,
    };
    if (!setEquals(ids, _dialogIds)) _dialogIds = ids;
    return _dialogIds;
  }

  @override
  Widget build(BuildContext context) {
    final overridesState = ref.watch(overridesProvider);
    final locState = ref.watch(locEditsProvider);
    final dialogIds = _dialogIdsFor(locState);
    final audioState = ref.watch(audioReplacementsProvider);
    final textureState = ref.watch(textureReplacementsProvider);
    final scriptState = ref.watch(scriptModsProvider);
    final dialogTopicsState = ref.watch(dialogTopicsProvider);

    // Same arithmetic as the OverridesPanel header count.
    final total =
        overridesState.count +
        locState.entryCount +
        audioState.count +
        textureState.count +
        scriptState.count +
        dialogTopicsState.count;

    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    final entries =
        <({_ChangesSection section, IconData icon, String label, int count})>[
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
            count: dialogIds.length + dialogTopicsState.count,
          ),
          (
            section: _ChangesSection.audio,
            icon: Icons.audiotrack_outlined,
            label: l10n.tabAudio,
            count: audioState.count,
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
        // IndexedStack would mount and build all five children up front,
        // paying hidden provider costs (audio bank loads and the script module
        // list) for sections the user may never open. The
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
      case _ChangesSection.scripts:
        return const GamePathScope(child: ScriptTab(onlyStaged: true));
    }
  }
}
