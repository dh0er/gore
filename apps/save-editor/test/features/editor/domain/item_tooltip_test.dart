import 'package:flutter/widgets.dart' show Locale;
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/domain/item_tooltip.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

/// A tiny stand-in for the extracted loc catalog: `id -> {set -> text}`.
Map<String, Map<String, String>> _catalog(Map<String, String> english) => {
  for (final entry in english.entries) entry.key: {'english': entry.value},
};

void main() {
  final l10n = lookupAppLocalizations(const Locale('en'));
  const lang = GameLang('en', 'English', Locale('en'), kEnglishLocSets);

  test('a weapon reads like the game\'s own hover block', () {
    const stats = ItemStats(
      itemType: 'Item_Weapon_Sword_OneHand',
      value: 31,
      damage: {'Item_Damage_Physical_Edge': 17},
      requires: {'Strength': 14},
    );
    final tooltip = buildItemTooltip(
      title: 'Battle Sword',
      stats: stats,
      catalog: _catalog({
        'item_weapon_sword_onehand': 'One-Handed Sword',
        'item_damage_physical_edge': 'Edge Dmg',
        'ui_inventory_requirements': 'Requirements:',
        'attributeset_strength_strength': 'Strength',
      }),
      lang: lang,
      l10n: l10n,
    );

    expect(tooltip.title, 'Battle Sword');
    expect(tooltip.subtitle, 'One-Handed Sword');
    expect(tooltip.stats.first.label, 'Edge Dmg');
    expect(tooltip.stats.first.value, '17');
    expect(tooltip.requirementsLabel, 'Requirements:');
    expect(tooltip.requirements.single.label, 'Strength');
    expect(tooltip.requirements.single.value, '14');
  });

  test('an unnamed type tag falls back to its named parent', () {
    // The game names `item_weapon_rune`, never the per-spell child tag, and its
    // own tooltip shows "Rune" for every rune.
    const stats = ItemStats(itemType: 'Item_Weapon_Rune_BallLightning');
    final tooltip = buildItemTooltip(
      title: 'Ball Lightning',
      stats: const ItemStats(
        itemType: 'Item_Weapon_Rune_BallLightning',
        value: 900,
      ),
      catalog: _catalog({'item_weapon_rune': 'Rune'}),
      lang: lang,
      l10n: l10n,
    );
    expect(stats.itemType, 'Item_Weapon_Rune_BallLightning');
    expect(tooltip.subtitle, 'Rune');
  });

  test('a rune lists damage and mana level by level', () {
    const stats = ItemStats(
      itemType: 'Item_Weapon_Rune_BallLightning',
      spellLevels: [
        {'Item_Damage_Elemental_Energy': 70},
        {'Item_Damage_Elemental_Energy': 90},
        {'Item_Damage_Elemental_Energy': 110},
        {'Item_Damage_Elemental_Energy': 150},
      ],
      spellMana: [
        {'initialMana': 5},
        {'initialMana': 6},
        {'initialMana': 7},
        {'initialMana': 9},
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Ball Lightning',
      stats: stats,
      catalog: _catalog({
        'item_damage_elemental_energy': 'Energy Dmg',
        'ui_stat_manacost_text': 'Mana cost',
      }),
      lang: lang,
      l10n: l10n,
    );
    expect(tooltip.stats[0].label, 'Energy Dmg');
    expect(tooltip.stats[0].value, '70/90/110/150');
    expect(tooltip.stats[1].label, 'Mana cost');
    expect(tooltip.stats[1].value, '5/6/7/9');
  });

  test('a continuous spell names its upkeep separately', () {
    const stats = ItemStats(
      itemType: 'Item_Weapon_Rune_IceBolt',
      spellLevels: [
        {'Item_Damage_Elemental_Ice': 20},
      ],
      spellMana: [
        {'initialMana': 1, 'manaPerSecond': 1},
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Ice Arrow',
      stats: stats,
      catalog: _catalog({
        'ui_stat_initialmanacost_text': 'Initial mana cost',
        'ui_stat_manaupkeep_text': 'Charge mana cost',
        'ui_stat_duration_measurement': '/sec',
      }),
      lang: lang,
      l10n: l10n,
    );
    final labels = tooltip.stats.map((row) => row.label).toList();
    expect(labels, contains('Initial mana cost'));
    expect(labels, contains('Charge mana cost'));
    expect(
      tooltip.stats.firstWhere((row) => row.label == 'Charge mana cost').value,
      '1/sec',
    );
  });

  test('armour lists its protection under the game\'s own heading', () {
    const stats = ItemStats(
      itemType: 'Item_Armor',
      value: 2900,
      onEquip: {'Resistance_Edge': 90, 'MaxSuperArmor': 35},
      descriptionKey: 'Ore_Armor_M_description',
    );
    final tooltip = buildItemTooltip(
      title: 'Heavy Ore Armour',
      stats: stats,
      catalog: _catalog({
        'item_armor': 'Armor',
        'ui_protection_protection': 'Protection',
        'attributeset_armor_resistance_edge': 'Edge',
        'ore_armor_m_description': 'Forged from raw ore.',
      }),
      lang: lang,
      l10n: l10n,
    );
    // Protection is its own labelled block, the way the game boxes it; the
    // poise bonus is not protection and stays with the plain numbers.
    expect(tooltip.protectionLabel, 'Protection');
    expect(tooltip.protection.single.label, 'Edge');
    expect(
      tooltip.protection.single.value,
      '+90',
      reason: 'an equip bonus reads as a bonus, not as a bare number',
    );
    expect(
      tooltip.protection.single.iconName,
      'T_Icon_Resistance_Edge',
      reason: 'the game marks each protection with its own shield',
    );
    expect(tooltip.stats.map((row) => row.label), contains('Maximum poise'));
    expect(tooltip.description, 'Forged from raw ore.');
  });

  test('nothing known about the item means no tooltip at all', () {
    expect(
      buildItemTooltip(
        title: 'Whatever',
        stats: null,
        catalog: const {},
        lang: lang,
        l10n: l10n,
      ).isEmpty,
      isTrue,
    );
    expect(
      buildItemTooltip(
        title: 'Whatever',
        stats: const ItemStats(),
        catalog: const {},
        lang: lang,
        l10n: l10n,
      ).isEmpty,
      isTrue,
    );
  });

  test('without an extracted catalog the labels stay the editor\'s own', () {
    const stats = ItemStats(
      itemType: 'Item_Weapon_Sword_OneHand',
      value: 31,
      requires: {'Strength': 14},
    );
    final tooltip = buildItemTooltip(
      title: 'Battle Sword',
      stats: stats,
      catalog: const {},
      lang: lang,
      l10n: l10n,
    );
    expect(tooltip.subtitle, isEmpty);
    expect(tooltip.requirementsLabel, l10n.itemTooltipRequirements);
    expect(
      tooltip.stats.map((row) => row.label),
      contains(l10n.itemTooltipValue),
    );
  });

  test('stack size is never shown, however the item declares it', () {
    // The game has no string for it and never displays it, and the editor does
    // not enforce it either — a count of 999 on a 99-stack item loads fine.
    final tooltip = buildItemTooltip(
      title: 'Cheese',
      stats: const ItemStats(itemType: 'Item_Food', value: 8, maxStack: 99),
      catalog: const {},
      lang: lang,
      l10n: l10n,
    );
    expect(tooltip.stats.map((row) => row.value), isNot(contains('99')));
    expect(
      tooltip.stats.map((row) => row.label),
      contains(l10n.itemTooltipValue),
      reason: 'trade value stays: it is what a save editor is for',
    );
  });

  test('food and drink name what they do to you', () {
    const stats = ItemStats(
      itemType: 'Item_Food',
      value: 4,
      onConsume: [
        ItemConsumeEffect(
          effect: 'Heal_Overtime',
          params: {'Heal': 1, 'Duration': 3},
        ),
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Apple',
      stats: stats,
      catalog: _catalog({
        'item_food': 'Food',
        'attributeset_health_health': 'Health',
        'ui_stat_duration_measurement': '/s',
      }),
      lang: lang,
      l10n: l10n,
    );

    // The game heals a point a second for three seconds, and says so — not a
    // single figure.
    expect(tooltip.stats.first.label, 'Health');
    expect(tooltip.stats.first.value, '+1/s · 3 s');
  });

  test('a brew lists every effect it carries, magnitude and percentage', () {
    const stats = ItemStats(
      itemType: 'Item_Food',
      onConsume: [
        ItemConsumeEffect(effect: 'Alcohol_Insta', params: {'Alcohol': 40}),
        ItemConsumeEffect(
          effect: 'Increase_Resistance_Fire',
          params: {'Duration': 30},
          percent: {'Resistance_Fire': 0.15},
        ),
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Strong Beer',
      stats: stats,
      catalog: _catalog({
        'attributeset_intoxication_alcohol': 'Alcohol',
        'attributeset_armor_resistance_fire': 'Fire Protection',
      }),
      lang: lang,
      l10n: l10n,
    );

    final values = {for (final row in tooltip.stats) row.label: row.value};
    expect(values['Alcohol'], '+40');
    // The effect class raises the resistance by a share of itself, for a span
    // it declares rather than one the item passes in.
    expect(values['Fire Protection'], '+15% · 30 s');
  });

  test('a spell that deals no damage still names its mana cost', () {
    // Light, Heal and every teleport rune cost mana and deal none. Their cost
    // used to hang off the damage block and so never showed.
    const stats = ItemStats(
      itemType: 'Item_Weapon_Rune_Light',
      requires: {'MagicianLevel': 1},
      spellMana: [
        {'initialMana': 5},
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Light',
      stats: stats,
      catalog: _catalog({'ui_stat_manacost_text': 'Mana Cost'}),
      lang: lang,
      l10n: l10n,
    );

    expect(tooltip.stats.map((row) => row.label), contains('Mana Cost'));
    expect(
      tooltip.stats.firstWhere((row) => row.label == 'Mana Cost').value,
      '5',
    );
  });

  test('a continuous spell without damage names cost and upkeep', () {
    const stats = ItemStats(
      itemType: 'Item_Weapon_Rune_Heal',
      spellMana: [
        {'initialMana': 2, 'manaPerSecond': 1},
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Heal',
      stats: stats,
      catalog: _catalog({
        'ui_stat_initialmanacost_text': 'Initial Mana',
        'ui_stat_manaupkeep_text': 'Upkeep',
        'ui_stat_duration_measurement': '/s',
      }),
      lang: lang,
      l10n: l10n,
    );

    final values = {for (final row in tooltip.stats) row.label: row.value};
    expect(values['Initial Mana'], '2');
    expect(values['Upkeep'], '1/s');
  });

  test('a blueprint lists the whole chain it teaches, with ingredients', () {
    const stats = ItemStats(
      itemType: 'Item_Writing',
      value: 100,
      teaches: [
        ItemRecipeStep(
          station: 'forge',
          needs: {'ItMi_Smith_Iron': 2},
          makes: {'ItMi_Smith_Blade_Arming': 1},
        ),
        ItemRecipeStep(
          station: 'workbench',
          needs: {'ItMi_Smith_Blade_Arming': 1, 'ItMi_Smith_Oak': 1},
          makes: {'ItMi_Smith_1H_Sword_02': 1},
        ),
        ItemRecipeStep(
          station: 'whetstone',
          needs: {'ItMi_Smith_1H_Sword_02': 1},
          makes: {'ItMw_1H_Sword_02': 1},
        ),
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Schematic of Judgement Sword',
      stats: stats,
      catalog: _catalog({
        'itmi_smith_iron': 'Iron',
        'itmi_smith_blade_arming': 'Arming Blade',
        'itmi_smith_oak': 'Oak',
        'itmi_smith_1h_sword_02': 'Dull Judgment Sword',
        'itmw_1h_sword_02': 'Judgment Sword',
      }),
      lang: lang,
      l10n: l10n,
    );

    // Each step is one readable line — what goes in, what comes out — in
    // production order, and the heading names what it is all for.
    expect(tooltip.recipe, hasLength(3));
    expect(tooltip.recipe[0].label, '2× Iron  →  Arming Blade');
    expect(
      tooltip.recipe[1].label,
      'Arming Blade + Oak  →  Dull Judgment Sword',
    );
    expect(tooltip.recipe[2].label, 'Dull Judgment Sword  →  Judgment Sword');
    expect(tooltip.recipeLabel, contains('Judgment Sword'));
    // Each step carries the mark of the bench it is worked at.
    expect(tooltip.recipe[0].iconName, 'T_Interact_Forge');
    expect(tooltip.recipe[2].iconName, 'T_Icon_Melee');
  });

  test('a raw material names what can be made from it', () {
    const stats = ItemStats(
      itemType: 'Item_Misc',
      value: 3,
      ingredientFor: ['ItMi_Smith_Blade_Arming', 'ItMi_Smith_Head_Broadaxe'],
    );
    final tooltip = buildItemTooltip(
      title: 'Iron',
      stats: stats,
      catalog: _catalog({
        'itmi_smith_blade_arming': 'Arming Blade',
        'itmi_smith_head_broadaxe': 'Broadaxe Head',
      }),
      lang: lang,
      l10n: l10n,
    );

    expect(tooltip.ingredientFor.map((row) => row.label), [
      'Arming Blade',
      'Broadaxe Head',
    ]);
    expect(tooltip.ingredientForLabel, isNotEmpty);
  });

  test('a writing shows its own text, and an item its own description key', () {
    const stats = ItemStats(
      itemType: 'Item_Writing',
      value: 160,
      writing: [
        ItemWritingPart(id: 'TEXT_TITLE', isHeading: true),
        ItemWritingPart(id: 'TEXT_BODY'),
      ],
    );
    final tooltip = buildItemTooltip(
      title: 'Book of Innos',
      // No descriptionKey on the class, but the game ships one under the id.
      itemId: 'ItMs_Book_Innos',
      stats: stats,
      catalog: _catalog({
        'text_title': 'The Circles of Magic',
        'text_body': 'Magic is the gift of the gods.',
        'itms_book_innos_description': 'A heavy tome.',
      }),
      lang: lang,
      l10n: l10n,
    );

    expect(tooltip.writing, [
      'The Circles of Magic',
      'Magic is the gift of the gods.',
    ]);
    expect(tooltip.description, 'A heavy tome.');
  });
}
