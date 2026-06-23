import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('DifficultySettings.fromJson maps fields and label', () {
    final d = DifficultySettings.fromJson({
      'preset': 'DifficultyPreset_Custom',
      'combat': 'CombatDifficultySettings_Hard',
      'flowHelper': true,
      'permadeath': false,
    });
    expect(d.presetLabel, 'Custom');
    expect(d.combatLabel, 'Hard');
    expect(d.flowHelper, true);
    expect(d.permadeath, false);
  });

  test('presetLabel maps Easy to Novice and Standard to Gothic', () {
    expect(DifficultySettings(preset: 'DifficultyPreset_Easy').presetLabel, 'Novice');
    expect(DifficultySettings(preset: 'DifficultyPreset_Standard').presetLabel, 'Gothic');
  });
}
