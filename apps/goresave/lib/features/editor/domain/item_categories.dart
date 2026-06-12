import 'package:goresave/features/editor/domain/editor_models.dart';

/// Item categories for Gothic 1 Remake inventory items, derived from the
/// Angelscript class-name prefix (e.g. `ItMi_Orenugget` -> misc).
///
/// Prefix set verified against the UE4SS object dump of 2026-06-12; see
/// docs/superpowers/specs/2026-06-12-inventory-add-item-design.md.
enum ItemCategory {
  meleeWeapon('Melee weapons'),
  rangedWeapon('Ranged weapons'),
  rune('Runes'),
  scroll('Spell scrolls'),
  food('Food & potions'),
  misc('Miscellaneous'),
  trophy('Animal trophies'),
  writing('Writings'),
  mission('Mission items'),
  key('Keys'),
  amulet('Amulets'),
  other('Other');

  const ItemCategory(this.label);

  final String label;
}

ItemCategory itemCategoryFromId(String id) {
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_')) return ItemCategory.rangedWeapon;
  if (id.startsWith('ItAr_Rune_')) return ItemCategory.rune;
  if (id.startsWith('ItAr_Scroll_')) return ItemCategory.scroll;
  if (id.startsWith('ItFo_')) return ItemCategory.food;
  if (id.startsWith('ItMi_')) return ItemCategory.misc;
  if (id.startsWith('ItAt_')) return ItemCategory.trophy;
  if (id.startsWith('ItWr_')) return ItemCategory.writing;
  if (id.startsWith('ItMs_')) return ItemCategory.mission;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.key;
  }
  if (id.startsWith('ItAm_')) return ItemCategory.amulet;
  return ItemCategory.other;
}

/// Human-readable name derived from the class id; never reads game
/// localization data (legal posture: identifiers only).
String itemDisplayNameFromId(String id) {
  const prefixes = ['ItMw_', 'ItRw_', 'ItAr_', 'ItFo_', 'ItMi_', 'ItAt_',
      'ItWr_', 'ItMs_', 'ItKe_', 'ItAm_'];
  var name = id;
  for (final prefix in prefixes) {
    if (name.startsWith(prefix)) {
      name = name.substring(prefix.length);
      break;
    }
  }
  final cleaned = name.replaceAll('_', ' ').trim();
  return cleaned.isEmpty ? id : cleaned;
}

class InventoryItemGroup {
  const InventoryItemGroup({required this.category, required this.items});

  final ItemCategory category;
  final List<PrivateInventoryItem> items;
}

/// Groups items by category. Groups appear in [ItemCategory] declaration
/// order, empty groups are omitted, items are sorted by id within a group.
List<InventoryItemGroup> groupInventoryItems(
  List<PrivateInventoryItem> items,
) {
  final byCategory = <ItemCategory, List<PrivateInventoryItem>>{};
  for (final item in items) {
    byCategory.putIfAbsent(itemCategoryFromId(item.id), () => []).add(item);
  }
  return [
    for (final category in ItemCategory.values)
      if (byCategory.containsKey(category))
        InventoryItemGroup(
          category: category,
          items: byCategory[category]!..sort((a, b) => a.id.compareTo(b.id)),
        ),
  ];
}
