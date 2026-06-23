import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/item_categories.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';

void main() {
  group('itemCategoryFromId', () {
    test('classifies ItFo_Apple as food', () {
      expect(itemCategoryFromId('ItFo_Apple'), ItemCategory.food);
    });
    test('classifies ItMw_1H_Sword_01 as meleeWeapon', () {
      expect(itemCategoryFromId('ItMw_1H_Sword_01'), ItemCategory.meleeWeapon);
    });
    test('classifies ItAm_Arrow as ammunition (not amulet)', () {
      expect(itemCategoryFromId('ItAm_Arrow'), ItemCategory.ammunition);
    });
    test('classifies ItAt_Amulet_01 as amulet', () {
      expect(itemCategoryFromId('ItAt_Amulet_01'), ItemCategory.amulet);
    });
    test('classifies ItAt_Ring_01 as ring', () {
      expect(itemCategoryFromId('ItAt_Ring_01'), ItemCategory.ring);
    });
    test('falls back to other for unknown prefix', () {
      expect(itemCategoryFromId('Npc_Bloodfly'), ItemCategory.other);
    });
  });

  group('groupCatalogItems', () {
    final apple  = CatalogItem(id: 'ItFo_Apple',       displayName: 'Apple',  fields: []);
    final cheese = CatalogItem(id: 'ItFo_Cheese',      displayName: 'Cheese', fields: []);
    final sword  = CatalogItem(id: 'ItMw_1H_Sword_01', displayName: 'Sword',  fields: []);

    test('groups by category', () {
      final groups = groupCatalogItems([apple, cheese, sword]);
      expect(groups.map((g) => g.category), contains(ItemCategory.food));
      expect(groups.map((g) => g.category), contains(ItemCategory.meleeWeapon));
    });

    test('items sorted by id within group', () {
      final groups = groupCatalogItems([cheese, apple]);
      final food = groups.firstWhere((g) => g.category == ItemCategory.food);
      expect(food.items.map((i) => i.id), ['ItFo_Apple', 'ItFo_Cheese']);
    });

    test('empty groups are omitted', () {
      final groups = groupCatalogItems([apple]);
      expect(groups, hasLength(1));
    });

    test('preserves ItemCategory declaration order across groups', () {
      final groups = groupCatalogItems([apple, sword]);
      // meleeWeapon comes before food in the enum declaration
      expect(groups.first.category, ItemCategory.meleeWeapon);
    });
  });
}
