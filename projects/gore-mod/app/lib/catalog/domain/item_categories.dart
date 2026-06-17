import 'item_entry.dart';

// ItemCategory and itemCategoryFromId/itemDisplayNameFromId are identical
// to goresave's version — same prefix table, same enum order.
enum ItemCategory {
  meleeWeapon('Melee weapons'),
  rangedWeapon('Ranged weapons'),
  ammunition('Ammunition'),
  rune('Runes'),
  scroll('Spell scrolls'),
  food('Food & potions'),
  misc('Miscellaneous'),
  amulet('Amulets'),
  ring('Rings'),
  trophy('Animal trophies'),
  writing('Writings'),
  mission('Mission items'),
  key('Keys'),
  other('Other');

  const ItemCategory(this.label);
  final String label;
}

ItemCategory itemCategoryFromId(String id) {
  if (id.startsWith('ItMw_'))         return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_'))         return ItemCategory.rangedWeapon;
  if (id.startsWith('ItAm_'))         return ItemCategory.ammunition;
  if (id.startsWith('ItAr_Rune_'))    return ItemCategory.rune;
  if (id.startsWith('ItAr_Scroll_'))  return ItemCategory.scroll;
  if (id.startsWith('ItFo_'))         return ItemCategory.food;
  if (id.startsWith('ItMi_'))         return ItemCategory.misc;
  if (id.startsWith('ItAt_Amulet_'))  return ItemCategory.amulet;
  if (id.startsWith('ItAt_Ring_'))    return ItemCategory.ring;
  if (id.startsWith('ItAt_'))         return ItemCategory.trophy;
  if (id.startsWith('ItWr_'))         return ItemCategory.writing;
  if (id.startsWith('ItMs_'))         return ItemCategory.mission;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.key;
  }
  return ItemCategory.other;
}

class CatalogItemGroup {
  const CatalogItemGroup({required this.category, required this.items});
  final ItemCategory category;
  final List<CatalogItem> items;
}

/// Groups [CatalogItem] list by category, in [ItemCategory] declaration order.
/// Empty groups are omitted; items are sorted by id within each group.
List<CatalogItemGroup> groupCatalogItems(List<CatalogItem> items) {
  final byCategory = <ItemCategory, List<CatalogItem>>{};
  for (final item in items) {
    byCategory.putIfAbsent(itemCategoryFromId(item.id), () => []).add(item);
  }
  return [
    for (final category in ItemCategory.values)
      if (byCategory.containsKey(category))
        CatalogItemGroup(
          category: category,
          items: byCategory[category]!..sort((a, b) => a.id.compareTo(b.id)),
        ),
  ];
}
