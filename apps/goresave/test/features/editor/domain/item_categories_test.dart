import 'package:flutter_test/flutter_test.dart';
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
    expect(itemCategoryFromId('ItAm_Amulet_01'), ItemCategory.amulet);
  });

  test('unknown ids map to other', () {
    expect(itemCategoryFromId('Armor_OC_EBR_Gomez_100'), ItemCategory.other);
    expect(itemCategoryFromId(''), ItemCategory.other);
    expect(itemCategoryFromId('ItIg_Worldsplitter'), ItemCategory.other);
  });

  test('display name strips prefix', () {
    expect(itemDisplayNameFromId('ItMi_Orenugget'), 'Orenugget');
    expect(itemDisplayNameFromId('ItAr_Rune_FireBall_Base'), 'Rune FireBall Base');
    expect(itemDisplayNameFromId('NoPrefix'), 'NoPrefix');
  });
}
