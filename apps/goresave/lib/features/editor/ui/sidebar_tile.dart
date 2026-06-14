import 'package:flutter/material.dart';

import 'package:goresave/features/editor/domain/item_categories.dart';

/// A selectable left-sidebar row, matching the Player/Progression tab style.
class SidebarTile extends StatelessWidget {
  const SidebarTile({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

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
                Icon(
                  icon,
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

/// Material icon for an item category, used by inventory sidebars.
IconData iconForItemCategory(ItemCategory category) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return Icons.gavel;
    case ItemCategory.rangedWeapon:
      return Icons.gps_fixed;
    case ItemCategory.ammunition:
      return Icons.arrow_outward;
    case ItemCategory.rune:
      return Icons.auto_awesome;
    case ItemCategory.scroll:
      return Icons.description_outlined;
    case ItemCategory.food:
      return Icons.restaurant;
    case ItemCategory.misc:
      return Icons.category_outlined;
    case ItemCategory.amulet:
      return Icons.diamond_outlined;
    case ItemCategory.ring:
      return Icons.radio_button_unchecked;
    case ItemCategory.trophy:
      return Icons.pets;
    case ItemCategory.writing:
      return Icons.menu_book_outlined;
    case ItemCategory.mission:
      return Icons.flag_outlined;
    case ItemCategory.key:
      return Icons.vpn_key_outlined;
    case ItemCategory.other:
      return Icons.help_outline;
  }
}
