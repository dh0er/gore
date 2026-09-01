import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';

void main() {
  test('maps known prefixes to the game inventory tabs', () {
    expect(itemCategoryFromId('ItMw_1H_Sword_01'), ItemCategory.meleeWeapon);
    expect(
      itemCategoryFromId('ItRw_Bow_Diego_Sleeper'),
      ItemCategory.rangedWeapon,
    );
    // Runes and scrolls share the game's Magic tab.
    expect(itemCategoryFromId('ItAr_Rune_FireBall_Base'), ItemCategory.magic);
    expect(itemCategoryFromId('ItAr_Scroll_Charm'), ItemCategory.magic);
    expect(itemCategoryFromId('ItFo_Apple'), ItemCategory.food);
    expect(itemCategoryFromId('ItFo_Potion_Health_01'), ItemCategory.potion);
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    // Hunting trophies are filed with the crafting materials in game.
    expect(itemCategoryFromId('ItAt_Wolf_Fur'), ItemCategory.material);
    expect(itemCategoryFromId('ItWr_Map_OldWorld'), ItemCategory.document);
    // Quest items and keys share the game's Artefacts tab.
    expect(itemCategoryFromId('ItMs_Ashes'), ItemCategory.artefact);
    expect(itemCategoryFromId('ItKe_Lockpick'), ItemCategory.artefact);
    expect(itemCategoryFromId('ItKeyDefault'), ItemCategory.artefact);
    expect(itemCategoryFromId('ItChestKey01'), ItemCategory.artefact);
    expect(itemCategoryFromId('ItDoorKey01'), ItemCategory.artefact);
    // Ammunition rides with the bows it is shot from.
    expect(itemCategoryFromId('ItAm_Arrow'), ItemCategory.rangedWeapon);
    expect(itemCategoryFromId('ItAm_Bolt'), ItemCategory.rangedWeapon);
    // Jewellery is worn, so the game files it with the armour.
    expect(itemCategoryFromId('ItAt_Amulet_OfDeath'), ItemCategory.wearable);
    expect(itemCategoryFromId('ItAt_Ring_OfLife'), ItemCategory.wearable);
  });

  test('armor classes categorize as wearables', () {
    expect(itemCategoryFromId('Ore_Armor_H'), ItemCategory.wearable);
    expect(itemCategoryFromId('Org_Armor'), ItemCategory.wearable);
    expect(itemCategoryFromId('Armor_OC_Gomez'), ItemCategory.wearable);
    expect(itemCategoryFromId('Org_Armor_Top_H_01'), ItemCategory.wearable);
    // non-armor unaffected
    expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
    expect(itemCategoryFromId('SomethingElse'), ItemCategory.other);
    // an "Armory" segment (room/building) is not armor
    expect(itemCategoryFromId('NC_Armory_Door'), ItemCategory.other);
  });

  test('unknown ids map to other', () {
    expect(itemCategoryFromId(''), ItemCategory.other);
    expect(itemCategoryFromId('ItIg_Worldsplitter'), ItemCategory.other);
  });

  group('with the bundled item stats', () {
    final stats = ItemStatsCatalog.fromJsonString('''
{
  "schema": 1,
  "filters": [
    {"id": "G1R_All", "itemTags": [], "nameKey": "Text_FilterAll",
     "icon": "T_Icon_AllItems", "sortOrder": 1},
    {"id": "G1R_Magic", "itemTags": ["Item_Weapon_Rune", "Item_Weapon_Scroll"],
     "nameKey": "Text_FilterMagic", "icon": "T_Icon_Magic", "sortOrder": 4},
    {"id": "G1R_Materials", "itemTags": ["Item_Ore", "Item_Trophy"],
     "nameKey": "Text_FilterMaterials", "icon": "T_Icon_Materials",
     "sortOrder": 8}
  ],
  "items": {
    "ItAr_Rune_BallLightning": {"itemType": "Item_Weapon_Rune_BallLightning"},
    "ItMi_Orenugget": {"itemType": "Item_Ore", "value": 1},
    "ItMi_Gold": {"itemType": "Item_Currency"}
  }
}
''');

    test('a child tag is claimed by its parent tag, as GameplayTags match', () {
      expect(stats.filterFor('ItAr_Rune_BallLightning')?.id, 'G1R_Magic');
      expect(
        itemCategoryFor('ItAr_Rune_BallLightning', stats: stats),
        ItemCategory.magic,
      );
    });

    test('the game type tag wins over the class-name prefix', () {
      // The prefix says misc; the game files it under Materials.
      expect(itemCategoryFromId('ItMi_Orenugget'), ItemCategory.misc);
      expect(
        itemCategoryFor('ItMi_Orenugget', stats: stats),
        ItemCategory.material,
      );
    });

    test(
      'a type tag no filter claims lands in Other, not in the prefix tab',
      () {
        // The filters and the item types come out of the same script cache, so a
        // type nothing claims means the game files it nowhere and shows it only
        // under "All". Reading the tab off the id prefix would contradict the
        // game's own answer for an item it has one for.
        expect(stats.filterFor('ItMi_Gold'), isNull);
        expect(itemCategoryFor('ItMi_Gold', stats: stats), ItemCategory.other);
      },
    );

    test('an id the stats do not know falls back to the prefix', () {
      expect(
        itemCategoryFor('ItMw_1H_Sword_01', stats: stats),
        ItemCategory.meleeWeapon,
      );
    });

    test('groups follow the stats when they are given', () {
      const items = [
        PrivateInventoryItem(id: 'ItMi_Orenugget', path: 'p1', count: 3),
        PrivateInventoryItem(id: 'ItMw_1H_Sword', path: 'p2', count: 1),
      ];
      expect(groupInventoryItems(items, stats: stats).map((g) => g.category), [
        ItemCategory.meleeWeapon,
        ItemCategory.material,
      ]);
    });
  });

  test('display name strips prefix', () {
    expect(itemDisplayNameFromId('ItMi_Orenugget'), 'Orenugget');
    expect(
      itemDisplayNameFromId('ItAr_Rune_FireBall_Base'),
      'Rune FireBall Base',
    );
    expect(itemDisplayNameFromId('ItChestKey01'), 'Chest Key 01');
    expect(itemDisplayNameFromId('ItDoorKey01'), 'Door Key 01');
    expect(
      itemDisplayNameFromId('ItFocusStoneBridgeItem'),
      'Focus Stone Bridge Item',
    );
    expect(itemDisplayNameFromId('ItKeyDefault'), 'Key Default');
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
    expect(groups.map((g) => g.category).toList(), [
      ItemCategory.meleeWeapon,
      ItemCategory.misc,
      ItemCategory.other,
    ]);
    expect(groups[1].items.map((i) => i.id).toList(), [
      'ItMi_Gold',
      'ItMi_Orenugget',
    ]); // sorted by id within group
  });

  test('an item no filter claims goes to Other, not to the prefix guess', () {
    // The game shows an orc two-hander under "All" and nowhere else: no filter
    // claims `Item_Weapon_Mace_TwoHand`. Reading `ItMw_` off the name filed it
    // among the melee weapons instead.
    final stats = ItemStatsCatalog.fromJsonString('''
{"schema": 1,
 "filters": [
   {"id": "G1R_All", "sortOrder": 1, "itemTags": []},
   {"id": "G1R_MeleeWeapons", "sortOrder": 2,
    "itemTags": ["Item_Weapon_Sword_OneHand", "Item_Weapon_Orc_TwoHand"]}
 ],
 "items": {
   "ItMw_2H_Mace_Orc_01_vOrc": {"itemType": "Item_Weapon_Mace_TwoHand"},
   "ItMw_1H_Sword_02": {"itemType": "Item_Weapon_Sword_OneHand"}
 }}
''');

    expect(
      itemCategoryFor('ItMw_2H_Mace_Orc_01_vOrc', stats: stats),
      ItemCategory.other,
    );
    expect(
      itemCategoryFor('ItMw_1H_Sword_02', stats: stats),
      ItemCategory.meleeWeapon,
    );
    // An id the catalog has never heard of still falls back to its prefix.
    expect(
      itemCategoryFor('ItMw_Something_New', stats: stats),
      ItemCategory.meleeWeapon,
    );
  });
}
