import 'package:flutter/material.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import 'glossary_panel.dart';
import 'progression_panel.dart' show QuestsDetail, FactionsDetail;
import 'story_state_panel.dart';

/// Sidebar section entries for the World tab. Knowledge and Events are
/// deliberately absent: they moved to detail-only panels (KnowledgeDetail /
/// EventsDetail) keyed by a shared character selection and are mounted from
/// the Characters tab, not from this sidebar.
enum _WorldSection { quests, glossary, factions, storyState }

/// World tab: structured quests, glossary, source-aware story state, and
/// faction crime records.
/// Full-height sidebar layout (no outer scroll). The reload key passed to the
/// details is the [SaveInspection] instance itself; identity comparison makes
/// every fresh inspection reload, while each detail decides which same-save
/// optimistic state must remain visible until its new snapshot arrives.
class WorldTab extends StatefulWidget {
  const WorldTab({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  @override
  State<WorldTab> createState() => _WorldTabState();
}

class _WorldTabState extends State<WorldTab> {
  // Keep selected section across save-triggered reloads (identity comparison
  // on reloadKey, not path comparison, so same pattern as hero_stats_card).
  _WorldSection _selected = _WorldSection.quests;
  // Glossary hydration joins several full-save datasets plus a bundled static
  // catalog. Defer that work until the section is opened for the first time;
  // once mounted it stays mounted so its pending switches survive navigation.
  bool _glossaryMounted = false;
  // Story state parses a large private map and joins optional glossary
  // context. Like the glossary, do that work only after its first selection.
  bool _storyStateMounted = false;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (!widget.inspection.privateDecoded) {
      return _MessagePane(
        icon: Icons.public,
        title: l10n.tabWorld,
        body: l10n.progressionLockedBody,
      );
    }
    if (!widget.inspection.privateProgression.available) {
      return _MessagePane(
        icon: Icons.public,
        title: l10n.tabWorld,
        body: l10n.progressionNeedsTyped,
      );
    }

    final reloadKey = widget.inspection;
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Left sidebar: same style as the Player tab (hero_stats_card).
          SizedBox(
            width: 200,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerLow,
                borderRadius: BorderRadius.circular(12),
              ),
              child: SingleChildScrollView(
                padding: const EdgeInsets.symmetric(vertical: 6),
                child: Column(
                  children: [
                    _SidebarTile(
                      icon: Icons.flag_outlined,
                      // The game's own quest marker.
                      gameIcon: 'T_Icon_StoryQuest',
                      label: l10n.sectionQuests,
                      selected: _selected == _WorldSection.quests,
                      onTap: () =>
                          setState(() => _selected = _WorldSection.quests),
                    ),
                    _SidebarTile(
                      icon: Icons.menu_book_outlined,
                      // The glossary's own book, not the speech bubble that
                      // marks captured dialogue.
                      gameIcon: 'T_Icon_Book',
                      label: l10n.sectionGlossary,
                      selected: _selected == _WorldSection.glossary,
                      onTap: () => setState(() {
                        _glossaryMounted = true;
                        _selected = _WorldSection.glossary;
                      }),
                    ),
                    _SidebarTile(
                      icon: Icons.gavel_outlined,
                      // The game draws no crest for "factions" as such; the
                      // Old Camp's stands for all of them.
                      gameIcon: 'T_Icon_OldCamp',
                      label: l10n.factionsSidebar,
                      selected: _selected == _WorldSection.factions,
                      onTap: () =>
                          setState(() => _selected = _WorldSection.factions),
                    ),
                    _SidebarTile(
                      icon: Icons.account_tree_outlined,
                      gameIcon: 'T_Icon_Commpleted',
                      label: l10n.storyStateSidebar,
                      selected: _selected == _WorldSection.storyState,
                      onTap: () => setState(() {
                        _storyStateMounted = true;
                        _selected = _WorldSection.storyState;
                      }),
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(width: 16),
          // Detail area — fills remaining width and full height.
          // Every section stays mounted (Offstage, same pattern as
          // hero_stats_card): a detail's local `_pending` map backs entries in
          // the global pending-edit registry, so disposing it on a section
          // switch would hide queued edits that Save still writes. Keys are
          // stable on purpose: a key derived from reloadKey would remount the
          // detail on every fresh inspection, disposing state and bypassing
          // the didUpdateWidget logic that preserves the selected character.
          Expanded(
            child: Stack(
              children: [
                Offstage(
                  offstage: _selected != _WorldSection.quests,
                  child: QuestsDetail(
                    key: const ValueKey('quests'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
                if (_glossaryMounted)
                  Offstage(
                    offstage: _selected != _WorldSection.glossary,
                    child: GlossaryDetail(
                      key: const ValueKey('glossary'),
                      notifier: widget.notifier,
                      editable: widget.editable,
                      reloadKey: reloadKey,
                      theme: theme,
                    ),
                  ),
                Offstage(
                  offstage: _selected != _WorldSection.factions,
                  child: FactionsDetail(
                    key: const ValueKey('factions'),
                    notifier: widget.notifier,
                    editable: widget.editable,
                    reloadKey: reloadKey,
                    theme: theme,
                  ),
                ),
                if (_storyStateMounted)
                  Offstage(
                    offstage: _selected != _WorldSection.storyState,
                    child: StoryStateDetail(
                      key: const ValueKey('story-state'),
                      notifier: widget.notifier,
                      editable: widget.editable,
                      reloadKey: reloadKey,
                      theme: theme,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Sidebar tile
// ---------------------------------------------------------------------------

class _SidebarTile extends StatelessWidget {
  const _SidebarTile({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.gameIcon,
  });

  /// The game's own glyph for this section, when it has one. Falls back to
  /// [icon] without a game installation.
  final String? gameIcon;

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      child: Material(
        color: selected ? scheme.primaryContainer : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: [
                GameIcon(
                  name: gameIcon,
                  fallbackIcon: icon,
                  size: 18,
                  color: selected ? scheme.primary : scheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: selected ? scheme.primary : scheme.onSurface,
                      fontWeight: selected ? FontWeight.w600 : null,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// _MessagePane (local helper, duplicated from editor_page.dart pattern)
// ---------------------------------------------------------------------------

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
