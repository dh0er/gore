import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';

Map<String, Object?> _row(String setClass, String id) {
  final prefix = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{NPC-1}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
  ];
  return {
    'key': id,
    'base': 1.0,
    'current': 1.0,
    'basePath': [...prefix, 'BaseValue'],
    'currentPath': [...prefix, 'CurrentValue'],
  };
}

void main() {
  test('recovers the attribute set class from the typed path', () {
    final row = NpcAttributeRow.fromJson(
      _row('/Script/G1R.AttributeSet_Fatigue', 'RecoveryRatePerHourOfSleep'),
    );
    expect(row.setClass, '/Script/G1R.AttributeSet_Fatigue');
  });

  test('setClass is null when the path carries no attribute set', () {
    final row = NpcAttributeRow.fromJson({
      'key': 'Health',
      'base': 1.0,
      'current': 1.0,
      'basePath': const ['m_GenericData', 'Whatever'],
      'currentPath': const <String>[],
    });
    expect(row.setClass, isNull);
  });

  test('an NPC hides the same derived and unused values the hero does', () {
    // The set matters: RecoveryRatePerHourOfSleep is inert on Fatigue (the
    // unreachable survival mode) but real on Health and Mana. Filtering by the
    // bare id would have taken all three, or none.
    final result = NpcAttributesResult.fromJson({
      'attributes': [
        _row('/Script/G1R.AttributeSet_Fatigue', 'RecoveryRatePerHourOfSleep'),
        _row('/Script/G1R.AttributeSet_Health', 'RecoveryRatePerHourOfSleep'),
        _row('/Script/G1R.AttributeSet_Mana', 'RecoveryRatePerHourOfSleep'),
        _row('/Script/G1R.AttributeSet_Hunger', 'Hunger'),
        _row('/Script/G1R.AttributeSet_Strength', 'Critical_OneHand'),
        _row('/Script/G1R.AttributeSet_LevelProgression', 'Toughness'),
        _row('/Script/G1R.AttributeSet_Health', 'MaxHealth'),
      ],
    });

    expect(result.attributes.map((a) => a.setClass?.split('.').last), [
      'AttributeSet_Health',
      'AttributeSet_Mana',
      'AttributeSet_Health',
    ]);
    expect(result.attributes.map((a) => a.key), [
      'RecoveryRatePerHourOfSleep',
      'RecoveryRatePerHourOfSleep',
      'MaxHealth',
    ]);
  });

  test('the surviving sleep rates group with Sleep, not Advanced', () {
    for (final set in ['AttributeSet_Health', 'AttributeSet_Mana']) {
      final row = NpcAttributeRow.fromJson(
        _row('/Script/G1R.$set', 'RecoveryRatePerHourOfSleep'),
      );
      expect(
        heroAttributeGroup(row.key, row.setClass),
        HeroAttributeGroup.sleep,
        reason: set,
      );
    }
  });
}
