import 'package:flutter/material.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';

/// One pane in a [GroupedAttributeSidebar]: a sidebar tile (icon + label) and
/// the detail content shown when that tile is selected. [id] identifies the
/// pane for selection bookkeeping and must be unique + stable within one
/// sidebar instance (e.g. a [HeroAttributeGroup] value or a sentinel for the
/// hero-transform slot).
class SidebarPane {
  const SidebarPane({
    required this.id,
    required this.label,
    required this.icon,
    required this.detail,
    this.gameIcon,
  });

  final Object id;
  final String label;
  final IconData icon;

  /// Shared game glyph shown instead of [icon] once the user's install has been
  /// read; null keeps [icon].
  final String? gameIcon;
  final Widget detail;
}

/// Reusable master-detail shell shared by the player's [HeroStatsCard] and the
/// [NpcAttributesPanel]: a slim left sidebar listing the [panes] and a right
/// detail area showing the selected pane. Every pane stays mounted (via
/// [Offstage]) so editor state — most importantly any unsaved field drafts that
/// back a registered pending edit — survives switching sidebar entries.
///
/// Selection is owned by the caller: [selected] is the currently selected pane
/// id and [onSelect] is called when the user taps a different tile. The caller
/// is responsible for keeping [selected] pointing at a pane that exists; if it
/// is null or stale the first pane is shown.
class GroupedAttributeSidebar extends StatelessWidget {
  const GroupedAttributeSidebar({
    super.key,
    required this.panes,
    required this.selected,
    required this.onSelect,
  });

  final List<SidebarPane> panes;
  final Object? selected;
  final void Function(Object id) onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    // Resolve the effective selection: the caller's choice when it still maps
    // to a present pane, else the first pane.
    final effective = panes.any((p) => p.id == selected)
        ? selected
        : panes.first.id;

    // CrossAxisAlignment.stretch makes both children fill the available height
    // so the sidebar background extends to the bottom regardless of content
    // length, and the detail side can scroll independently.
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Left sidebar: ~200px, fixed — never scrolls away with content.
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
                  for (final pane in panes)
                    _SidebarTile(
                      label: pane.label,
                      icon: pane.icon,
                      gameIcon: pane.gameIcon,
                      selected: pane.id == effective,
                      onTap: () => onSelect(pane.id),
                    ),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(width: 16),
        // Right detail area: scrolls independently while the sidebar stays put.
        // Every pane stays mounted (Offstage) so editor state survives switches.
        Expanded(
          child: Stack(
            children: [
              for (final pane in panes)
                Offstage(
                  offstage: pane.id != effective,
                  child: SingleChildScrollView(child: pane.detail),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

/// A slim sidebar tile echoing the save-list sidebar idiom (Material + InkWell,
/// selected highlight via primaryContainer).
class _SidebarTile extends StatelessWidget {
  const _SidebarTile({
    required this.label,
    required this.icon,
    required this.gameIcon,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final IconData icon;
  final String? gameIcon;
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
                    // A two-word heading like "Combat / movement" does not fit
                    // this sidebar in every language, so let it wrap rather
                    // than truncate; the ellipsis is the last resort.
                    maxLines: 2,
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
