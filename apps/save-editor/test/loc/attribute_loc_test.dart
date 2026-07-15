import 'package:flutter_test/flutter_test.dart';
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
