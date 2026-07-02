import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/ui/game_path_scope.dart';
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../audio/ui/audio_tab.dart';
import '../../catalog/ui/items_tab.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../dialog/ui/dialoge_tab.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../scripts/domain/script_mods_notifier.dart';
import '../../scripts/ui/script_tab.dart';
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

  @override
  Widget build(BuildContext context) {
    final overridesState = ref.watch(overridesProvider);
    final locState = ref.watch(locEditsProvider);
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
        count: locState.entryCount,
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
                    onTap: () => setState(() => _section = e.section),
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
        Expanded(child: _buildContent(overridesState, locState)),
      ],
    );
  }

  Widget _buildContent(OverridesState overridesState, LocEditsState locState) {
    switch (_section) {
      case _ChangesSection.all:
        // The flat all-domains list, centred like the previous Changes tab.
        return Align(
          alignment: Alignment.topCenter,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 600),
            child: const OverridesPanel(),
          ),
        );
      case _ChangesSection.items:
        return ItemsTab(
          onlyIds: {for (final e in overridesState.entries) e.classId},
        );
      case _ChangesSection.dialogs:
        return DialogeTab(onlyIds: locState.edits.keys.toSet());
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
