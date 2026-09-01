import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';

/// A one-pixel PNG, so the widget under test has a real file to decode.
const _pngBytes = <int>[
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, //
  0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
  0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
  0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41,
  0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
  0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d,
  0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
  0x44, 0xae, 0x42, 0x60, 0x82,
];

void main() {
  group('the glyph mapping', () {
    test('names a glyph for every attribute the game shows itself', () {
      for (final id in const [
        'Health',
        'MaxHealth',
        'Mana',
        'MaxMana',
        'Strength',
        'Dexterity',
        'SkillPoints',
        'Resistance_Blunt',
        'Resistance_Edge',
        'Resistance_Point',
        'Resistance_Fire',
        'Resistance_Energy',
        'Resistance_Ice',
        'Resistance_Wind',
        'Resistance_Falling',
      ]) {
        expect(
          gameIconForAttribute(id),
          isNotNull,
          reason: '$id has no game glyph',
        );
      }
    });

    test('every row of a group the game never shows takes the ◆', () {
      for (final id in const [
        // Breath.
        'Oxygen', 'MaxOxygen', 'OxygenDepletionRate', 'OxygenRecoveryRate',
        'CriticalLevelPercent',
        // Sleep.
        'SleepTime', 'MaxSleepTime', 'SleepTimeRecoveryAmount',
        'SleepTimeRecoveryPeriod', 'MaxRestTime',
        // Booze and swampweed.
        'Alcohol', 'MaxAlcohol', 'AlcoholDepletionRate',
        'Swampweed', 'MaxSwampweed', 'SwampweedDepletionRate',
        // The catch-all, including the NPC-only bounties that land in it.
        'XPExecutedBounty', 'XPKillOrDefeatBounty', 'SomethingNew',
      ]) {
        expect(gameIconForAttribute(id), isNull, reason: '$id must take the ◆');
      }
      // Both sleep rows that only differ by their attribute set, too.
      for (final set in const ['Health', 'Mana']) {
        expect(
          gameIconForAttribute(
            'RecoveryRatePerHourOfSleep',
            '/Script/G1R.AttributeSet_$set',
          ),
          isNull,
        );
      }
    });

    test('an unmapped attribute has no glyph, so the row shows the ◆', () {
      expect(gameIconForAttribute('Level'), isNull);
      expect(gameIconForAttribute('SomethingNew'), isNull);
    });

    test('the groups the game does show keep their own glyphs', () {
      expect(gameIconForAttribute('Health'), 'T_Icon_Health');
      expect(gameIconForAttribute('Resistance_Fire'), 'T_Icon_Resistance_Fire');
      expect(gameIconForAttribute('SuperArmor'), isNotNull);
      expect(gameIconForAttribute('PickPocketing'), isNotNull);
    });

    test('every attribute group and inventory tab has a glyph', () {
      for (final group in HeroAttributeGroup.values) {
        expect(gameIconForAttributeGroup(group), isNotNull, reason: '$group');
      }
      for (final category in ItemCategory.values) {
        if (category == ItemCategory.other) continue;
        expect(
          gameIconForItemCategory(category),
          isNotNull,
          reason: '$category',
        );
      }
    });

    test('every hunting skill shares the hunting knife', () {
      expect(gameIconForSkill('Hunting_Claw'), 'T_Icon_Hunting');
      expect(gameIconForSkill('Hunting_TrollHorn'), 'T_Icon_Hunting');
      expect(gameIconForSkill('Melee_OneHanded'), 'T_Icon_1handed');
      expect(gameIconForSkill('Unknown_Skill'), isNull);
    });
  });

  group('GameIcon', () {
    late Directory directory;
    late String manaPath;

    setUp(() {
      directory = Directory.systemTemp.createTempSync('gore_game_icon');
      manaPath = '${directory.path}/mana.png';
      File(manaPath).writeAsBytesSync(_pngBytes);
    });

    tearDown(() => directory.deleteSync(recursive: true));

    ItemIconCatalog catalogWith(Map<String, String> entries) => ItemIconCatalog(
      buildId: 'test',
      manifestPath: '${directory.path}/manifest.json',
      pathByItemId: entries,
    );

    Future<void> pump(
      WidgetTester tester,
      Widget child, {
      required ItemIconCatalog icons,
    }) {
      return tester.pumpWidget(
        ProviderScope(
          overrides: [
            itemIconCatalogProvider.overrideWith((ref) async => icons),
          ],
          child: MaterialApp(
            home: Scaffold(body: Center(child: child)),
          ),
        ),
      );
    }

    testWidgets('shows the game glyph when this generation carries it', (
      tester,
    ) async {
      await pump(
        tester,
        const GameIcon(name: 'T_Icon_Mana', fallbackIcon: Icons.water_drop),
        icons: catalogWith({'ui:t_icon_mana': manaPath}),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Image), findsOneWidget);
      expect(find.byIcon(Icons.water_drop), findsNothing);
    });

    testWidgets('keeps the editor\'s own icon when the glyph is missing', (
      tester,
    ) async {
      await pump(
        tester,
        const GameIcon(name: 'T_Icon_Gone', fallbackIcon: Icons.water_drop),
        icons: catalogWith({'ui:t_icon_mana': manaPath}),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.water_drop), findsOneWidget);
      expect(find.byType(Image), findsNothing);
    });

    testWidgets('a row with no icon of its own falls back to the game ◆', (
      tester,
    ) async {
      await pump(
        tester,
        const GameIcon(),
        icons: catalogWith({
          'ui:${gameIconGenericBullet.toLowerCase()}': manaPath,
        }),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Image), findsOneWidget);
    });

    testWidgets('without any game images the row still shows its own icon', (
      tester,
    ) async {
      await pump(
        tester,
        const GameIcon(name: 'T_Icon_Mana', fallbackIcon: Icons.water_drop),
        icons: const ItemIconCatalog.empty(),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.water_drop), findsOneWidget);
    });

    testWidgets('a panel pumped without a ProviderScope still renders', (
      tester,
    ) async {
      // Widget tests pump individual panels bare; a label must not require the
      // whole app's provider container just to draw its icon.
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: Center(
              child: GameIcon(
                name: 'T_Icon_Mana',
                fallbackIcon: Icons.water_drop,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(find.byIcon(Icons.water_drop), findsOneWidget);
    });

    testWidgets('a glyph with its own colours is not recoloured', (
      tester,
    ) async {
      // The death mark is a dark ring around a red skull; tinting it leaves a
      // single blob where the skull was.
      await pump(
        tester,
        const GameIcon(name: 'T_Icon_Dead', tinted: false),
        icons: catalogWith({'ui:t_icon_dead': manaPath}),
      );
      await tester.pumpAndSettle();

      expect(tester.widget<Image>(find.byType(Image)).color, isNull);
    });

    testWidgets('a white glyph is recoloured so it reads on any theme', (
      tester,
    ) async {
      await pump(
        tester,
        const GameIcon(name: 'T_Icon_Mana'),
        icons: catalogWith({'ui:t_icon_mana': manaPath}),
      );
      await tester.pumpAndSettle();

      expect(tester.widget<Image>(find.byType(Image)).color, isNotNull);
    });
  });

  testWidgets('GameIconLabel puts the glyph in front of the text', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: Center(
            child: GameIconLabel(label: 'Mana', iconName: 'T_Icon_Mana'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Mana'), findsOneWidget);
    expect(find.byType(GameIcon), findsOneWidget);
  });
}
