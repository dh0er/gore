import 'package:flutter/material.dart';

import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';

/// A selectable left-sidebar row, matching the Player/Progression tab style.
class SidebarTile extends StatelessWidget {
  const SidebarTile({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.gameIcon,
  });

  final IconData icon;

  /// Shared game glyph shown instead of [icon] when the user's install has been
  /// read. Null (or a glyph this game build lacks) keeps [icon].
  final String? gameIcon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  /// The label, ellipsized to one line, wrapped in a [Tooltip] ONLY when it
  /// actually does not fit. A tooltip that repeats text the user can already
  /// read in full is noise, so the row is measured against its own width first
  /// (`TextPainter.didExceedMaxLines`) with the same style, scale and direction
  /// the `Text` will use — otherwise the measurement and the render disagree.
  Widget _label(BuildContext context, TextStyle? style) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final text = Text(
          label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: style,
        );
        final painter = TextPainter(
          text: TextSpan(text: label, style: style),
          maxLines: 1,
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        if (!painter.didExceedMaxLines) return text;
        return Tooltip(message: label, child: text);
      },
    );
  }

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
                  child: _label(
                    context,
                    Theme.of(context).textTheme.bodyMedium?.copyWith(
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

/// Material icon for an item category, used by inventory sidebars.
IconData iconForItemCategory(ItemCategory category) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return Icons.gavel;
    case ItemCategory.rangedWeapon:
      return Icons.gps_fixed;
    case ItemCategory.magic:
      return Icons.auto_awesome;
    case ItemCategory.wearable:
      return Icons.shield_outlined;
    case ItemCategory.food:
      return Icons.restaurant;
    case ItemCategory.potion:
      return Icons.science_outlined;
    case ItemCategory.material:
      return Icons.diamond_outlined;
    case ItemCategory.document:
      return Icons.menu_book_outlined;
    case ItemCategory.misc:
      return Icons.category_outlined;
    case ItemCategory.artefact:
      return Icons.vpn_key_outlined;
    case ItemCategory.other:
      return Icons.help_outline;
  }
}
