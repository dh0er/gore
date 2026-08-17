import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  final l10nDirectory = Directory('lib/l10n');
  final templateFile = File('${l10nDirectory.path}/app_en.arb');

  test('every locale contains exactly the template message keys', () {
    final template = _readArb(templateFile);
    final expectedKeys = _messageKeys(template);
    final localeFiles =
        l10nDirectory
            .listSync()
            .whereType<File>()
            .where((file) => RegExp(r'app_[\w]+\.arb$').hasMatch(file.path))
            .toList()
          ..sort((a, b) => a.path.compareTo(b.path));

    expect(localeFiles, isNotEmpty);
    for (final file in localeFiles) {
      final actualKeys = _messageKeys(_readArb(file));
      expect(
        actualKeys,
        expectedKeys,
        reason: '${file.path} must neither omit nor invent message keys',
      );
    }
  });

  test('every translation preserves the template placeholders', () {
    final template = _readArb(templateFile);
    final placeholders = <String, Set<String>>{
      for (final key in _messageKeys(template))
        key: _placeholderNames(template['@$key']),
    };
    final localeFiles = l10nDirectory.listSync().whereType<File>().where(
      (file) => RegExp(r'app_[\w]+\.arb$').hasMatch(file.path),
    );

    for (final file in localeFiles) {
      final arb = _readArb(file);
      for (final entry in placeholders.entries) {
        final message = arb[entry.key] as String;
        for (final placeholder in entry.value) {
          expect(
            message,
            contains('{$placeholder'),
            reason: '${file.path}:${entry.key} must preserve {$placeholder}',
          );
        }
      }
    }
  });

  test('feature translations are not copied from the English template', () {
    const mustBeLocalized = <String>{
      'allDataLockedBody',
      'allDataDescription',
      'allDataChildren',
      'allDataTagInputHint',
      'allDataTypedSource',
      'memoryEventCategory',
      'memoryEventAction',
      'memoryEventFact',
      'memoryEventGameTime',
      // Map areas the game has no string of its own for. These are the ones
      // that used to leak English into every other language's sidebar, so an
      // English copy here is the exact regression to catch.
      //
      // `locationAreaTundra` is deliberately NOT listed: German, Spanish,
      // Italian, Polish and Portuguese all spell the tundra "Tundra", and a
      // rule that forbids the correct word is worse than no rule.
      'locationAreaCavalornValley',
      'locationAreaEastForest',
      'locationAreaFogTower',
      'locationAreaIllegalWeedMixers',
      'locationAreaOrcArena',
      'locationAreaOrcGraveyard',
      'locationAreaShipwreck',
    };
    final template = _readArb(templateFile);
    final localeFiles = l10nDirectory.listSync().whereType<File>().where(
      (file) =>
          RegExp(r'app_[\w]+\.arb$').hasMatch(file.path) &&
          !file.path.endsWith('app_en.arb'),
    );

    for (final file in localeFiles) {
      final arb = _readArb(file);
      for (final key in mustBeLocalized) {
        expect(
          arb[key],
          isNot(template[key]),
          reason: '${file.path}:$key must not use the English fallback',
        );
      }
    }
  });

  test('every locale covers all known advanced attribute fallbacks', () {
    // Every value the curated attribute view can show. The Survival trio
    // (hunger/thirst/fatigue) and the Toughness quartet are deliberately
    // absent — they are hidden, see docs/reference/survival-mode.md.
    const advancedAttributeIds = <String>{
      'SuperArmor',
      'MaxSuperArmor',
      'DamageMultiplier',
      'SpeedModifier',
      'Oxygen',
      'MaxOxygen',
      'OxygenDepletionRate',
      'OxygenRecoveryRate',
      'CriticalLevelPercent',
      'SleepTime',
      'MaxSleepTime',
      'SleepTimeRecoveryAmount',
      'SleepTimeRecoveryPeriod',
      'MaxRestTime',
      'Health_RecoveryRatePerHourOfSleep',
      'Mana_RecoveryRatePerHourOfSleep',
      'Alcohol',
      'MaxAlcohol',
      'AlcoholDepletionRate',
      'Swampweed',
      'MaxSwampweed',
      'SwampweedDepletionRate',
      'XPExecutedBounty',
      'XPKillOrDefeatBounty',
    };
    final localeFiles = l10nDirectory.listSync().whereType<File>().where(
      (file) => RegExp(r'app_[\w]+\.arb$').hasMatch(file.path),
    );

    for (final file in localeFiles) {
      final message = _readArb(file)['attributeManualFallbackLabel'] as String;
      for (final attributeId in advancedAttributeIds) {
        expect(
          message,
          matches(RegExp('(?:^| )${RegExp.escape(attributeId)}\\{')),
          reason:
              '${file.path}:attributeManualFallbackLabel must cover '
              '$attributeId',
        );
      }
    }
  });
  test('attribute tooltips name no particular actor', () {
    // The same tooltip is shown for the player AND for a selected NPC, so a
    // sentence about "the hero" would claim the wrong thing on an NPC row.
    const heroWords = ['hero', 'Held', 'héroe', 'héros', 'eroe', 'bohater'];
    final template = _readArb(templateFile);
    final english = template['attributeManualTooltip'] as String;
    for (final word in heroWords) {
      expect(
        english.toLowerCase(),
        isNot(contains(word.toLowerCase())),
        reason: 'attributeManualTooltip must not name the hero',
      );
    }
    final localeFiles = l10nDirectory.listSync().whereType<File>().where(
      (file) => RegExp(r'app_[\w]+\.arb$').hasMatch(file.path),
    );
    for (final file in localeFiles) {
      final text = (_readArb(file)['attributeManualTooltip'] as String)
          .toLowerCase();
      for (final word in heroWords) {
        expect(
          text,
          isNot(contains(word.toLowerCase())),
          reason: '${file.path}:attributeManualTooltip names "$word"',
        );
      }
    }
  });
}

Map<String, Object?> _readArb(File file) =>
    (jsonDecode(file.readAsStringSync()) as Map).cast<String, Object?>();

Set<String> _messageKeys(Map<String, Object?> arb) =>
    arb.keys.where((key) => !key.startsWith('@')).toSet();

Set<String> _placeholderNames(Object? metadata) {
  if (metadata is! Map) return const {};
  final placeholders = metadata['placeholders'];
  if (placeholders is! Map) return const {};
  return placeholders.keys.whereType<String>().toSet();
}
