import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Item categories for Gothic 1 Remake inventory items, derived from the
/// Angelscript class-name prefix (e.g. `ItMi_Orenugget` -> misc).
///
/// Prefix set verified against the UE4SS object dump of 2026-06-12; see
/// docs/superpowers/specs/2026-06-12-inventory-add-item-design.md.
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

/// Returns the localized display label for [category]. The English
/// [ItemCategory.label] is kept as a stable identifier / fallback; this is what
/// should be shown to the user.
String localizedItemCategoryLabel(AppLocalizations l10n, ItemCategory category) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return l10n.itemCategoryMeleeWeapon;
    case ItemCategory.rangedWeapon:
      return l10n.itemCategoryRangedWeapon;
    case ItemCategory.ammunition:
      return l10n.itemCategoryAmmunition;
    case ItemCategory.rune:
      return l10n.itemCategoryRune;
    case ItemCategory.scroll:
      return l10n.itemCategoryScroll;
    case ItemCategory.food:
      return l10n.itemCategoryFood;
    case ItemCategory.misc:
      return l10n.itemCategoryMisc;
    case ItemCategory.amulet:
      return l10n.itemCategoryAmulet;
    case ItemCategory.ring:
      return l10n.itemCategoryRing;
    case ItemCategory.trophy:
      return l10n.itemCategoryTrophy;
    case ItemCategory.writing:
      return l10n.itemCategoryWriting;
    case ItemCategory.mission:
      return l10n.itemCategoryMission;
    case ItemCategory.key:
      return l10n.itemCategoryKey;
    case ItemCategory.other:
      return l10n.itemCategoryOther;
  }
}

ItemCategory itemCategoryFromId(String id) {
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_')) return ItemCategory.rangedWeapon;
  // ItAm_ is ammunition (ItAm_Arrow/ItAm_Bolt); amulets live under ItAt_.
  if (id.startsWith('ItAm_')) return ItemCategory.ammunition;
  if (id.startsWith('ItAr_Rune_')) return ItemCategory.rune;
  if (id.startsWith('ItAr_Scroll_')) return ItemCategory.scroll;
  if (id.startsWith('ItFo_')) return ItemCategory.food;
  if (id.startsWith('ItMi_')) return ItemCategory.misc;
  if (id.startsWith('ItAt_Amulet_')) return ItemCategory.amulet;
  if (id.startsWith('ItAt_Ring_')) return ItemCategory.ring;
  if (id.startsWith('ItAt_')) return ItemCategory.trophy;
  if (id.startsWith('ItWr_')) return ItemCategory.writing;
  if (id.startsWith('ItMs_')) return ItemCategory.mission;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.key;
  }
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
