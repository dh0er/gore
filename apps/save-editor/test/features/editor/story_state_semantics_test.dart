import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/story_state_semantics.dart';

void main() {
  test('catalog contains the exact researched integer-kind counts', () {
    final counts = {
      for (final kind in StoryIntegerKind.values)
        kind: storyIntegerSemanticsCatalog
            .where((value) => value.kind == kind)
            .length,
    };

    expect(storyIntegerSemanticsCatalog, hasLength(419));
    expect(counts, {
      StoryIntegerKind.binaryFlag: 192,
      StoryIntegerKind.finiteState: 23,
      StoryIntegerKind.counterOrScore: 31,
      StoryIntegerKind.calendarDay: 1,
      StoryIntegerKind.derivedOrOpaqueInteger: 3,
      StoryIntegerKind.readOnlyInSourceInteger: 20,
      StoryIntegerKind.dormantOrLegacyInteger: 149,
    });
  });

  test('lookup is case-insensitive and exposes representative meanings', () {
    expect(
      storyIntegerSemantics('  stone_improvedorearmor  ')?.kind,
      StoryIntegerKind.binaryFlag,
    );
    expect(
      storyIntegerSemantics('BLACKMAILER_PERMISSION')?.kind,
      StoryIntegerKind.finiteState,
    );
    expect(
      storyIntegerSemantics('nc_jointsdistributed')?.kind,
      StoryIntegerKind.counterOrScore,
    );
    expect(
      storyIntegerSemantics('Whistler_BuyMySword_Day')?.kind,
      StoryIntegerKind.calendarDay,
    );
    expect(
      storyIntegerSemantics('Diego_Notes_DEX')?.kind,
      StoryIntegerKind.derivedOrOpaqueInteger,
    );
    expect(
      storyIntegerSemantics('FireMagesDead')?.kind,
      StoryIntegerKind.readOnlyInSourceInteger,
    );
    expect(
      storyIntegerSemantics('armor')?.kind,
      StoryIntegerKind.dormantOrLegacyInteger,
    );
  });

  test('finite-state values remain source-evidence suggestions', () {
    expect(storyIntegerSemantics('Blackmailer_Permission')?.knownValues, [
      0,
      1,
      2,
      3,
    ]);
    expect(storyIntegerSemantics('ChromaninReaded')?.knownValues, [
      1,
      2,
      3,
      4,
      5,
      6,
    ]);
    expect(
      storyIntegerSemantics('Mud_Nerve')?.knownValues,
      List<int>.generate(20, (index) => index),
    );
    expect(storyIntegerSemantics('Stone_ImprovedOreArmor')?.knownValues, [
      0,
      1,
    ]);
    expect(
      storyIntegerSemantics('Blackmailer_Permission')?.confidence,
      'high-source-evidence',
    );
  });

  test('unknown IDs, time markers, and Chapter are not integer guesses', () {
    expect(storyIntegerSemantics('Mod_NewStoryValue'), isNull);
    expect(storyIntegerSemantics('Stone_OreArmor'), isNull);
    expect(storyIntegerSemantics('Chapter'), isNull);
    expect(storyIntegerSemantics(''), isNull);
  });

  test('catalog has no duplicate IDs under case-insensitive lookup', () {
    final normalized = storyIntegerSemanticsCatalog
        .map((value) => value.id.toLowerCase())
        .toList();

    expect(normalized.toSet(), hasLength(normalized.length));
    for (final value in storyIntegerSemanticsCatalog) {
      expect(storyIntegerSemantics(value.id), same(value));
    }
  });
}
