import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/l10n/app_localizations_de.dart';
import 'package:goresave/l10n/app_localizations_ja.dart';
import 'package:goresave/l10n/app_localizations_zh.dart';
import 'package:goresave/loc/attribute_loc.dart';
import 'package:goresave/loc/game_lang.dart';

void main() {
  final catalog = <String, Map<String, String>>{
    'attributeset_health_health': {
      'english': 'Health',
      'german': 'Lebenspunkte',
    },
    'attributeset_health_maxhealth': {
      'english': 'Maximum health',
      'german': 'Maximale Lebenspunkte',
    },
    'attributeset_levelprogression_skillpoints': {
      'english': 'Learning Points',
      'german': 'Lernpunkte',
    },
    'attributeset_future_newstat': {
      'english': 'New stat',
      'german': 'Neuer Wert',
    },
  };

  test('reconstructs the exact game catalog key from attribute set and id', () {
    expect(
      localizedAttributeName(
        catalog,
        gameLangByCode('de'),
        'MaxHealth',
        setClass: '/Script/G1R.AttributeSet_Health',
      ),
      'Maximale Lebenspunkte',
    );
  });

  test('uses known set mapping when a legacy attribute has no set class', () {
    expect(
      localizedAttributeName(catalog, gameLangByCode('de'), 'SkillPoints'),
      'Lernpunkte',
    );
  });

  test('discovers a unique future catalog entry by attribute suffix', () {
    expect(
      localizedAttributeName(catalog, gameLangByCode('de'), 'NewStat'),
      'Neuer Wert',
    );
  });

  test('falls back to a readable label when localization is unavailable', () {
    expect(
      localizedAttributeName(
        const {},
        gameLangByCode('de'),
        'DamageMultiplier',
      ),
      'Damage multiplier',
    );
    expect(readableAttributeName('Resistance_Fire'), 'Resistance fire');
    expect(readableAttributeName('SkillPoints'), 'Skill points (LP)');
  });

  test('localizes manual fallbacks absent from the game catalog', () {
    expect(
      localizedAttributeName(
        const {},
        gameLangByCode('de'),
        'DamageMultiplier',
        l10n: AppLocalizationsDe(),
      ),
      'Erlittener Schaden',
    );
    // Set-qualified: the same id means something different per attribute set,
    // so the label has to follow the set, not the bare id.
    expect(
      localizedAttributeName(
        const {},
        gameLangByCode('de'),
        'RecoveryRatePerHourOfSleep',
        setClass: '/Script/G1R.AttributeSet_Health',
        l10n: AppLocalizationsDe(),
      ),
      'Leben je Schlafstunde',
    );
    expect(
      localizedAttributeName(
        const {},
        gameLangByCode('de'),
        'RecoveryRatePerHourOfSleep',
        setClass: '/Script/G1R.AttributeSet_Mana',
        l10n: AppLocalizationsDe(),
      ),
      'Mana je Schlafstunde',
    );
    // The tooltip explains the value; unknown ids get none.
    expect(
      attributeTooltip(
        'Oxygen',
        setClass: '/Script/G1R.AttributeSet_Oxygen',
        l10n: AppLocalizationsDe(),
      ),
      startsWith('Verbleibende Sekunden Luft'),
    );
    expect(
      attributeTooltip('SomeFutureAttribute', l10n: AppLocalizationsDe()),
      '',
    );
    expect(
      readableAttributeName('OxygenRecoveryRate', AppLocalizationsJa()),
      '息の回復（毎秒）',
    );
    expect(
      readableAttributeName('MaxSwampweed', AppLocalizationsZh()),
      '最大沼泽草值',
    );
  });

  test('manual localization keeps the readable fallback for unknown ids', () {
    expect(
      readableAttributeName('FutureUnknownValue', AppLocalizationsDe()),
      'Future unknown value',
    );
  });

  test('an exact game catalog value takes priority over a manual fallback', () {
    final gameValue = <String, Map<String, String>>{
      'attributeset_combat_damagemultiplier': {
        'german': 'Schadensfaktor aus dem Spiel',
      },
    };
    expect(
      localizedAttributeName(
        gameValue,
        gameLangByCode('de'),
        'DamageMultiplier',
        setClass: '/Script/G1R.AttributeSet_Combat',
        l10n: AppLocalizationsDe(),
      ),
      'Schadensfaktor aus dem Spiel',
    );
  });

  test('does not guess when future suffix matches are ambiguous', () {
    final ambiguous = <String, Map<String, String>>{
      'attributeset_one_shared': {'german': 'Eins'},
      'attributeset_two_shared': {'german': 'Zwei'},
    };
    expect(
      localizedAttributeName(ambiguous, gameLangByCode('de'), 'Shared'),
      'Shared',
    );
  });

  group('attribute tooltips', () {
    final descriptions = <String, Map<String, String>>{
      'attributeset_mana_mana_description': {
        'english': 'Mana fuels your magical abilities.',
        'german': 'Mana ermöglicht dir den Einsatz magischer Fähigkeiten.',
      },
      'attributeset_armor_resistance_fire_description': {
        'english': 'Shields against the intense heat of flames.',
        'german': 'Schützt vor der intensiven Hitze der Flammen.',
      },
    };

    test("prefers the game's own description, in the chosen language", () {
      expect(
        attributeTooltip(
          'Mana',
          setClass: '/Script/G1R.AttributeSet_Mana',
          l10n: AppLocalizationsDe(),
          catalog: descriptions,
          lang: gameLangByCode('de'),
        ),
        'Mana ermöglicht dir den Einsatz magischer Fähigkeiten.',
      );
    });

    test(
      'reaches a description through the known-set map without a set class',
      () {
        expect(
          attributeTooltip(
            'Mana',
            l10n: AppLocalizationsDe(),
            catalog: descriptions,
            lang: gameLangByCode('en'),
          ),
          'Mana fuels your magical abilities.',
        );
      },
    );

    test('resistances resolve against their armour set', () {
      expect(
        gameAttributeDescription(
          descriptions,
          gameLangByCode('en'),
          'Resistance_Fire',
          setClass: '/Script/G1R.AttributeSet_Armor',
        ),
        'Shields against the intense heat of flames.',
      );
    });

    test(
      'falls back to the manual table where the game has no description',
      () {
        // Poise is not on the game's own character screen, so only the editor
        // has anything to say about it.
        expect(
          gameAttributeDescription(
            descriptions,
            gameLangByCode('en'),
            'SuperArmor',
          ),
          isNull,
        );
        expect(
          attributeTooltip(
            'SuperArmor',
            l10n: AppLocalizationsDe(),
            catalog: descriptions,
            lang: gameLangByCode('de'),
          ),
          isNotEmpty,
        );
      },
    );

    test('an attribute nobody explains gets no tooltip at all', () {
      expect(
        attributeTooltip(
          'SomeFutureStat',
          l10n: AppLocalizationsDe(),
          catalog: descriptions,
          lang: gameLangByCode('de'),
        ),
        isEmpty,
      );
    });
  });
}
