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
      'Schadensmultiplikator',
    );
    expect(
      readableAttributeName('OxygenRecoveryRate', AppLocalizationsJa()),
      '酸素回復速度',
    );
    expect(
      readableAttributeName('MaxSwampweed', AppLocalizationsZh()),
      '最大沼泽草量',
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
}
