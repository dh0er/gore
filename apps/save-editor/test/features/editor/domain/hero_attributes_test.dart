import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';

TypedPropertyHit _heroHit(
  String setClass,
  String id,
  String leaf,
  String value, {
  String type = 'FloatProperty',
  bool editable = true,
}) {
  final path = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{Hero}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
    leaf,
  ];
  return TypedPropertyHit(
    path: path,
    display: path.join(' › '),
    type: type,
    value: value,
    editable: editable,
  );
}

void main() {
  test('pairs BaseValue and CurrentValue leaves into one attribute', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'BaseValue', '64'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'CurrentValue', '64'),
    ]);

    expect(attributes, hasLength(1));
    final attribute = attributes.single;
    expect(attribute.id, 'MaxHealth');
    expect(attribute.setClass, '/Script/G1R.AttributeSet_Health');
    expect(attribute.baseValue, 64);
    expect(attribute.currentValue, 64);
    expect(attribute.basePath, isNotNull);
    expect(attribute.basePath!.last, 'BaseValue');
    expect(attribute.currentPath!.last, 'CurrentValue');
  });

  test('keeps same-id attributes from different sets separate', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Health', 'RecoveryRatePerHourOfSleep',
          'BaseValue', '0.125'),
      _heroHit('/Script/G1R.AttributeSet_Mana', 'RecoveryRatePerHourOfSleep',
          'BaseValue', '-0.125'),
    ]);

    expect(attributes, hasLength(2));
    expect(attributes.map((a) => a.setClass).toSet(), hasLength(2));
  });

  test('skips non-attribute, non-editable and non-float hits', () {
    final nonHero = TypedPropertyHit(
      path: const ['m_GenericData', '{CharacterStates}', 'GlobalIDFormat'],
      display: 'GlobalIDFormat',
      type: 'StrProperty',
      value: 'x',
      editable: true,
    );
    final attributes = parseHeroAttributes([
      nonHero,
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'BaseValue', '35',
          editable: false),
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'CurrentValue', '35',
          type: 'StrProperty'),
    ]);

    expect(attributes, isEmpty);
  });

  test('assigns known ids to their groups and unknown ids to advanced', () {
    expect(heroAttributeGroup('MaxHealth'), HeroAttributeGroup.core);
    expect(heroAttributeGroup('SkillPoints'), HeroAttributeGroup.core);
    // The per-weapon crit values are hidden from the curated view now; the
    // classifier still buckets any leftover into advanced.
    expect(heroAttributeGroup('Critical_OneHand'), HeroAttributeGroup.advanced);
    expect(heroAttributeGroup('Resistance_Fire'), HeroAttributeGroup.resistances);
    expect(heroAttributeGroup('PickPocketing'), HeroAttributeGroup.thieving);
    expect(heroAttributeGroup('Swampweed'), HeroAttributeGroup.advanced);
    expect(heroAttributeGroup('SomeFutureAttribute'), HeroAttributeGroup.advanced);
  });

  test('sorts core attributes in display order before unknown ones', () {
    final attributes = parseHeroAttributes([
      _heroHit('/Script/G1R.AttributeSet_Strength', 'Strength', 'BaseValue', '10'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'MaxHealth', 'BaseValue', '64'),
      _heroHit('/Script/G1R.AttributeSet_Health', 'Health', 'BaseValue', '35'),
    ]);

    expect(attributes.map((a) => a.id).toList(),
        ['Health', 'MaxHealth', 'Strength']);
  });

  test('labels SkillPoints as learn points', () {
    expect(heroAttributeLabel('SkillPoints'), 'Skill points (LP)');
    expect(heroAttributeLabel('MaxHealth'), 'MaxHealth');
  });
}
