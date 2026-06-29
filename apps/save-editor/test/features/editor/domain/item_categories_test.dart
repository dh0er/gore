import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';

void main() {
  test('maps known prefixes to categories', () {
    expect(itemCategoryFromId('ItMw_1H_Sword_01'), ItemCategory.meleeWeapon);
    expect(itemCategoryFromId('ItRw_Bow_Diego_Sleeper'), ItemCategory.rangedWeapon);
    expect(itemCategoryFromId('ItAr_Rune_FireBall_Base'), ItemCategory.rune);
    expect(itemCategoryFromId('ItAr_Scroll_Charm'), ItemCategory.scroll);
    expect(itemCategoryFromId('ItFo_Apple'), ItemCategory.food);
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    expect(itemCategoryFromId('ItAt_Wolf_Fur'), ItemCategory.trophy);
    expect(itemCategoryFromId('ItWr_Map_OldWorld'), ItemCategory.writing);
    expect(itemCategoryFromId('ItMs_Ashes'), ItemCategory.mission);
    expect(itemCategoryFromId('ItKe_Lockpick'), ItemCategory.key);
    expect(itemCategoryFromId('ItKeyDefault'), ItemCategory.key);
    expect(itemCategoryFromId('ItChestKey01'), ItemCategory.key);
    expect(itemCategoryFromId('ItDoorKey01'), ItemCategory.key);
    expect(itemCategoryFromId('ItAm_Arrow'), ItemCategory.ammunition);
    expect(itemCategoryFromId('ItAm_Bolt'), ItemCategory.ammunition);
    expect(itemCategoryFromId('ItAt_Amulet_OfDeath'), ItemCategory.amulet);
    expect(itemCategoryFromId('ItAt_Ring_OfLife'), ItemCategory.ring);
  });

  test('armor classes categorize as armor', () {
    expect(itemCategoryFromId('Ore_Armor_H'), ItemCategory.armor);
    expect(itemCategoryFromId('Org_Armor'), ItemCategory.armor);
    expect(itemCategoryFromId('Armor_OC_Gomez'), ItemCategory.armor);
    expect(itemCategoryFromId('Org_Armor_Top_H_01'), ItemCategory.armor);
    // non-armor unaffected
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    expect(itemCategoryFromId('SomethingElse'), ItemCategory.other);
  });

  test('unknown ids map to other', () {
    expect(itemCategoryFromId(''), ItemCategory.other);
    expect(itemCategoryFromId('ItIg_Worldsplitter'), ItemCategory.other);
  });

  test('display name strips prefix', () {
    expect(itemDisplayNameFromId('ItMi_Orenugget'), 'Orenugget');
    expect(itemDisplayNameFromId('ItAr_Rune_FireBall_Base'), 'Rune FireBall Base');
    expect(itemDisplayNameFromId('NoPrefix'), 'NoPrefix');
  });

  test('groups items by category in enum order, non-empty only', () {
    const items = [
      PrivateInventoryItem(id: 'ItMi_Orenugget', path: 'p1', count: 3),
      PrivateInventoryItem(id: 'ItMw_1H_Sword', path: 'p2', count: 1),
      PrivateInventoryItem(id: 'ItMi_Gold', path: 'p3', count: 9),
      PrivateInventoryItem(id: 'Weird_Thing', path: 'p4', count: 1),
    ];
    final groups = groupInventoryItems(items);
    expect(groups.map((g) => g.category).toList(),
        [ItemCategory.meleeWeapon, ItemCategory.misc, ItemCategory.other]);
    expect(groups[1].items.map((i) => i.id).toList(),
        ['ItMi_Gold', 'ItMi_Orenugget']); // sorted by id within group
  });
}
