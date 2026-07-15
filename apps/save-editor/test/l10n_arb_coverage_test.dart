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

  test('every locale covers all known advanced attribute fallbacks', () {
    const advancedAttributeIds = <String>{
      'Alcohol',
      'AlcoholDepletionRate',
      'MaxAlcohol',
      'MaxSuperArmor',
      'SuperArmor',
      'Fatigue',
      'FillRatio',
      'FillRatioPeriod',
      'MaxFatigue',
      'MaxThresholdIndex',
      'RecoveryRatePerHourOfSleep',
      'DamageMultiplier',
      'Toughness',
      'ToughnessA',
      'ToughnessB',
      'ToughnessC',
      'XPExecutedBounty',
      'XPKillOrDefeatBounty',
      'SpeedModifier',
      'CriticalLevelPercent',
      'MaxOxygen',
      'Oxygen',
      'OxygenDepletionRate',
      'OxygenRecoveryRate',
      'MaxRestTime',
      'MaxSleepTime',
      'SleepTime',
      'SleepTimeRecoveryAmount',
      'SleepTimeRecoveryPeriod',
      'MaxSwampweed',
      'Swampweed',
      'SwampweedDepletionRate',
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
