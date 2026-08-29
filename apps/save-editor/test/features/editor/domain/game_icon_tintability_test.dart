import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';

/// Every glyph the editor draws has to take the theme's colour.
///
/// A handful of the game's shared icons are finished pictures with their own
/// dark ring — `T_Icon_Trainer`, `T_Icon_Shop`, `T_Icon_Dead`. Recolouring one
/// flattens it to a blot, and leaving it uncoloured makes it the odd mark out
/// on a light theme. [gameIconsWithOwnColours] records which they are; this
/// test keeps them from creeping back into the UI.
void main() {
  test('no glyph the app names is one that cannot be tinted', () {
    final referenced = <String, List<String>>{};
    final glyph = RegExp(r"'(T_(?:Icon|Interact|Interaction)_[A-Za-z0-9_]+)'");
    for (final entity in Directory('lib').listSync(recursive: true)) {
      if (entity is! File || !entity.path.endsWith('.dart')) continue;
      var source = entity.readAsStringSync();
      // The set that RECORDS them has to name them; everything else must not.
      final declaration = source.indexOf('gameIconsWithOwnColours = ');
      if (declaration >= 0) {
        final end = source.indexOf('};', declaration);
        source = source.substring(0, declaration) + source.substring(end);
      }
      for (final match in glyph.allMatches(source)) {
        final name = match.group(1)!;
        if (!gameIconsWithOwnColours.contains(name)) continue;
        referenced.putIfAbsent(name, () => []).add(entity.path);
      }
    }
    expect(
      referenced,
      isEmpty,
      reason:
          'these glyphs carry their own colours and must not be drawn as UI '
          'icons — pick a plain white one instead',
    );
  });

  test('the glyphs the role and section maps hand out are all tintable', () {
    for (final name in [
      gameIconTrade,
      gameIconTeacher,
      gameIconArmorer,
      gameIconKnowledge,
      gameIconDead,
      gameIconCharacter,
      gameIconCreature,
      gameIconGenericBullet,
    ]) {
      expect(gameIconsWithOwnColours, isNot(contains(name)), reason: name);
    }
    // Everything that belongs to a camp takes that camp's crest, whether the
    // tag says so at the top level or deeper down.
    const crests = {
      'Guild.Human.OldCamp': 'T_Icon_OldCamp',
      'Guild.Human.OldCamp.Shadow': 'T_Icon_OldCamp',
      'Guild.Human.OldCamp.FireMage': 'T_Icon_OldCamp',
      'Guild.Human.NewCamp': 'T_Icon_NewCamp',
      'Guild.Human.NewCamp.Mercenary': 'T_Icon_NewCamp',
      'Guild.Human.NewCamp.WaterMage': 'T_Icon_NewCamp',
      // The tag tree files the bandits beside the camps, not under one.
      'Guild.Human.Bandit': 'T_Icon_NewCamp',
      'Guild.Human.SwampCamp': 'T_Icon_SwampCamp',
      'Guild.Human.SwampCamp.Novice': 'T_Icon_SwampCamp',
      'Guild.Human.SwampCamp.Templar': 'T_Icon_SwampCamp',
      'Guild.Orc.Warrior': 'T_Icon_OrcWeapon',
      'Guild.Undead': gameIconDead,
    };
    crests.forEach((guild, expected) {
      expect(gameIconForGuild(guild), expected, reason: guild);
      expect(gameIconsWithOwnColours, isNot(contains(expected)), reason: guild);
    });
    // The game draws no crest for these, so the editor keeps its own icon
    // rather than inventing one.
    for (final guild in [
      'Guild.Demon',
      'Guild.SleeperTemple.Xardas',
      'Other',
    ]) {
      expect(gameIconForGuild(guild), isNull, reason: guild);
    }
  });
}
