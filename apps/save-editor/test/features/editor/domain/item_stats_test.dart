import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('the bundled catalog', () {
    late ItemStatsCatalog catalog;

    setUpAll(() async {
      catalog = await ItemStatsCatalog.loadBundled();
    });

    test('carries the game\'s own inventory rail, in its order', () {
      expect(catalog.filters.map((filter) => filter.id).toList(), const [
        'G1R_All',
        'G1R_MeleeWeapons',
        'G1R_RangedWeapons',
        'G1R_Magic',
        'G1R_Wereables',
        'G1R_Food',
        'G1R_Potions',
        'G1R_Materials',
        'G1R_Documents',
        'G1R_Miscellaneous',
        'G1R_Artefacts',
      ]);
      // Only the "All" tab claims no tags; every other tab must, or it would
      // silently collect nothing.
      expect(catalog.filters.where((f) => f.isAll).map((f) => f.id), [
        'G1R_All',
      ]);
      expect(
        catalog.filters.map((filter) => filter.icon),
        everyElement(startsWith('T_Icon_')),
      );
      expect(
        catalog.filters.map((filter) => filter.nameKey),
        everyElement(startsWith('Text_Filter')),
      );
    });

    test('a plain weapon carries its damage, price and requirement', () {
      final sword = catalog.statsFor('ItMw_1H_Sword_01')!;
      expect(sword.itemType, 'Item_Weapon_Sword_OneHand');
      expect(sword.value, 31);
      expect(sword.damage, {'Item_Damage_Physical_Edge': 17});
      expect(sword.requires, {'Strength': 14});
      expect(catalog.filterFor('ItMw_1H_Sword_01')?.id, 'G1R_MeleeWeapons');
    });

    test('a rune carries one damage and mana figure per spell level', () {
      // These are the four numbers the game's own tooltip prints for the
      // Ball Lightning rune: 70/90/110/150 damage at 5/6/7/9 mana.
      final rune = catalog.statsFor('ItAr_Rune_BallLightning')!;
      expect(rune.requires, {'MagicianLevel': 2});
      expect(
        rune.spellLevels.map((level) => level['Item_Damage_Elemental_Energy']),
        [70, 90, 110, 150],
      );
      expect(rune.spellMana.map((level) => level['initialMana']), [5, 6, 7, 9]);
      expect(catalog.filterFor('ItAr_Rune_BallLightning')?.id, 'G1R_Magic');
    });

    test('a continuous spell carries its per-second mana upkeep', () {
      final rune = catalog.statsFor('ItAr_Rune_IceBolt')!;
      expect(rune.spellMana.single, {'initialMana': 1, 'manaPerSecond': 1});
    });

    test('armour carries the protection it grants, without the zero rows', () {
      final armor = catalog.statsFor('Ore_Armor_M')!;
      expect(armor.itemType, 'Item_Armor');
      expect(armor.onEquip['Resistance_Edge'], 90);
      expect(armor.onEquip['Resistance_Fire'], 25);
      // The shipped effect lists Strength/Dexterity/MaxHealth at 0; a tooltip
      // must not claim those.
      expect(armor.onEquip.containsKey('Strength'), isFalse);
      expect(armor.descriptionKey, 'Ore_Armor_M_description');
      expect(catalog.filterFor('Ore_Armor_M')?.id, 'G1R_Wereables');
    });

    test('every rune and scroll carries a mana cost', () {
      // A container normally binds its spell configuration to the item's own
      // tag, but the heal, sleep, charm and telekinesis scrolls hand casting to
      // the RUNE's container and all seventeen transform scrolls share one
      // parent tag. Those used to reach nothing and showed no cost at all.
      final without = catalog.byItemId.entries
          .where(
            (entry) =>
                entry.key.startsWith('itar_') && entry.value.spellMana.isEmpty,
          )
          .map((entry) => entry.key)
          .toList();
      expect(without, isEmpty);
      // The heal scroll casts the heal rune's spell, so it costs what the rune
      // costs: 2 to start and 1 a second while it runs.
      final heal = catalog.statsFor('ItAr_Scroll_Heal')!;
      expect(heal.spellMana, [
        {'initialMana': 2, 'manaPerSecond': 1},
      ]);
    });

    test('lookup is case-insensitive and unknown ids stay null', () {
      expect(catalog.statsFor('itfo_apple')?.itemType, 'Item_Food');
      expect(catalog.statsFor('ItFo_Apple')?.itemType, 'Item_Food');
      expect(catalog.statsFor('Nonexistent_Item'), isNull);
      expect(catalog.filterFor('Nonexistent_Item'), isNull);
    });
  });

  test('a malformed document degrades to an empty catalog', () {
    expect(ItemStatsCatalog.fromJsonString('[]').filters, isEmpty);
    expect(ItemStatsCatalog.fromJsonString('{}').byItemId, isEmpty);
  });

  test('tag matching follows the GameplayTag hierarchy, not substrings', () {
    const filter = InventoryFilter(
      id: 'G1R_Magic',
      nameKey: 'Text_FilterMagic',
      icon: 'T_Icon_Magic',
      itemTags: ['Item_Weapon_Rune'],
      sortOrder: 4,
    );
    expect(filter.claims('Item_Weapon_Rune'), isTrue);
    expect(filter.claims('Item_Weapon_Rune_FireBall'), isTrue);
    // A tag that merely starts with the same letters is a different tag.
    expect(filter.claims('Item_Weapon_Runestone'), isFalse);
    expect(filter.claims('Item_Weapon_Scroll'), isFalse);
  });

  test('a forged blank is filed with the materials, not the weapons', () {
    // The game marks its smithing stock `Item_Property_Forge` and the Materials
    // tab claims that tag — but every blank still carries a weapon type, so
    // matching on the type alone left the criterion unreachable and filed all
    // seventy-six of them under Melee.
    final catalog = ItemStatsCatalog.fromJsonString('''
{"schema": 1,
 "filters": [
   {"id": "G1R_All", "sortOrder": 1, "itemTags": []},
   {"id": "G1R_MeleeWeapons", "sortOrder": 2,
    "itemTags": ["Item_Weapon_Sword_OneHand"]},
   {"id": "G1R_Materials", "sortOrder": 8,
    "itemTags": ["Item_Material", "Item_Property_Forge"]}
 ],
 "items": {
   "ItMi_Smith_Blade_Arming": {"itemType": "Item_Weapon_Sword_OneHand",
     "specs": ["Item_Property_Forge"]},
   "ItMw_1H_Sword_02": {"itemType": "Item_Weapon_Sword_OneHand"},
   "ItMi_Smith_Iron": {"itemType": "Item_Material"}
 }}
''');

    expect(catalog.filterFor('ItMi_Smith_Blade_Arming')?.id, 'G1R_Materials');
    // A finished weapon has no forge tag and stays where it was.
    expect(catalog.filterFor('ItMw_1H_Sword_02')?.id, 'G1R_MeleeWeapons');
    expect(catalog.filterFor('ItMi_Smith_Iron')?.id, 'G1R_Materials');
    expect(catalog.filterFor('NothingKnown'), isNull);
  });

  test(
    'the bundled catalog files every forged piece under Materials',
    () async {
      final catalog = await ItemStatsCatalog.loadBundled();
      final forged = catalog.byItemId.entries
          .where((e) => e.value.specs.contains('Item_Property_Forge'))
          .map((e) => e.key)
          .toList();
      expect(forged, isNotEmpty);
      for (final id in forged) {
        expect(catalog.filterFor(id)?.id, 'G1R_Materials', reason: id);
      }
    },
  );

  test('an item that carries only a description is not empty', () {
    // Two of the bundled entries have nothing but a description key. Counting
    // them as empty made `buildItemTooltip` return before it ever resolved the
    // text, so hovering them showed no card.
    const described = ItemStats(descriptionKey: 'FocusStoneBridgeItem');
    expect(described.isEmpty, isFalse);
    expect(const ItemStats().isEmpty, isTrue);
  });

  test('the bundled catalog has a card for every entry it describes', () async {
    final catalog = await ItemStatsCatalog.loadBundled();
    final silent = catalog.byItemId.entries
        .where((e) => e.value.descriptionKey.isNotEmpty && e.value.isEmpty)
        .map((e) => e.key)
        .toList();
    expect(silent, isEmpty);
  });
}
