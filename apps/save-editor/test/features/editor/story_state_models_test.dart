import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/story_state_models.dart';
import 'package:goresave/features/editor/domain/story_state_presentation.dart';
import 'package:goresave/loc/game_lang.dart';

void main() {
  test('story edits round-trip the atomic apply contract', () {
    const edits = [
      StoryStateEdit(
        id: 'Stone_OreArmor',
        present: true,
        rawValue: 1767047,
        expectedStored: false,
        expectedRawValue: null,
      ),
      StoryStateEdit(
        id: 'Mod_Custom',
        present: false,
        rawValue: null,
        expectedStored: true,
        expectedRawValue: -17,
        allowUnknownCreate: true,
      ),
    ];

    final wire = storyStateApplyEdit(edits);

    expect(wire['path'], storyStateApplyPath);
    expect(parseStoryStateApplyEdit(wire), edits);
    expect((wire['value'] as Map)['changes'], [
      {
        'id': 'Stone_OreArmor',
        'present': true,
        'rawValue': 1767047,
        'expected': {'stored': false},
      },
      {
        'id': 'Mod_Custom',
        'present': false,
        'expected': {'stored': true, 'rawValue': -17},
        'allowUnknownCreate': true,
      },
    ]);
  });

  test('story edit no-op distinguishes absent from a stored zero', () {
    const absentStillAbsent = StoryStateEdit(
      id: 'A',
      present: false,
      rawValue: null,
      expectedStored: false,
      expectedRawValue: null,
    );
    const absentBecomesZero = StoryStateEdit(
      id: 'A',
      present: true,
      rawValue: 0,
      expectedStored: false,
      expectedRawValue: null,
    );
    const storedZeroUnchanged = StoryStateEdit(
      id: 'A',
      present: true,
      rawValue: 0,
      expectedStored: true,
      expectedRawValue: 0,
    );

    expect(absentStillAbsent.isNoop, isTrue);
    expect(absentBecomesZero.isNoop, isFalse);
    expect(storedZeroUnchanged.isNoop, isTrue);
  });

  test('story edit parser rejects incomplete concurrency snapshots', () {
    expect(
      () => StoryStateEdit.fromJson({
        'id': 'A',
        'present': true,
        'expected': {'stored': false},
      }),
      throwsFormatException,
    );
    expect(
      () => StoryStateEdit.fromJson({
        'id': 'A',
        'present': false,
        'expected': {'stored': true},
      }),
      throwsFormatException,
    );
    expect(
      () => parseStoryStateApplyEdit({
        'path': storyStateApplyPath,
        'value': {
          'changes': [
            {
              'id': 'Stone_OreArmor',
              'present': true,
              'rawValue': 1,
              'expected': {'stored': false},
            },
            {
              'id': 'stone_orearmor',
              'present': true,
              'rawValue': 2,
              'expected': {'stored': false},
            },
          ],
        },
      }),
      throwsFormatException,
    );
  });

  test(
    'parses the core story projection without treating timestamps as ints',
    () {
      final page = StoryStatePage.fromJson({
        'storedTotal': 107,
        'writable': true,
        'offset': 0,
        'limit': 1000,
        'currentGameTimeSeconds': 1875587.9437,
        'storedSemanticTypeCounts': {
          'integer': 87,
          'timeMarker': 19,
          'chapter': 1,
        },
        'entries': [
          {
            'id': 'Stone_OreArmor',
            'rawValue': 1767047,
            'path': [
              'm_GenericData',
              '{Story}',
              'StoryPropertyValues',
              '{Stone_OreArmor}',
            ],
            'semanticType': 'timeMarker',
            'declaredType': 'FInGameTime',
            'catalogKnown': true,
          },
        ],
      });

      expect(page.total, 107);
      expect(page.storedTotal, 107);
      expect(page.catalogTotal, 107);
      expect(page.kindCounts[StorySemanticType.timeMarker], 19);
      expect(page.currentGameTimeSeconds, closeTo(1875587.9437, 0.0001));
      expect(page.writable, isTrue);
      expect(page.values.single.id, 'Stone_OreArmor');
      expect(page.values.single.value, 1767047);
      expect(page.values.single.stored, isTrue);
      expect(page.values.single.catalogKnown, isTrue);
      expect(page.values.single.semanticType, StorySemanticType.timeMarker);
      expect(page.values.single.declaredType, 'FInGameTime');
    },
  );

  test(
    'represents catalog values that are absent from the sparse save map',
    () {
      final page = StoryStatePage.fromJson({
        'total': 470,
        'storedTotal': 107,
        'catalogTotal': 470,
        'unsetTotal': 363,
        'entries': [
          {
            'id': 'AfterCinematic_Nyras',
            'rawValue': null,
            'stored': false,
            'path': <String>[],
            'semanticType': 'integer',
            'declaredType': 'int32',
            'catalogKnown': true,
          },
        ],
      });

      expect(page.total, 470);
      expect(page.catalogTotal, 470);
      expect(page.unsetTotal, 363);
      expect(page.values.single.value, isNull);
      expect(page.values.single.stored, isFalse);
      expect(page.values.single.catalogKnown, isTrue);
      expect(page.values.single.path, isEmpty);
    },
  );

  test(
    'links only an exact glossary-segment suffix and labels it as context',
    () {
      const segmentClass =
          '/Script/Angelscript.DocumentSegment_Glossary_OCR_GRD_STONE_OreArmor';
      const entries = [
        NpcGlossaryCatalogEntry(
          id: 'OCR_GRD_STONE',
          uniqueName: 'OCR_GRD_STONE_219',
          documentClass: '/Script/Angelscript.Document_Glossary_OCR_GRD_STONE',
          camp: NpcGlossaryCamp.oldCamp,
          segments: [
            NpcGlossaryCatalogSegment(
              id: 'OreArmor',
              segmentClass: segmentClass,
              label: 'Ore Armor',
            ),
          ],
        ),
      ];
      final link = findStoryGlossaryLink('Stone_OreArmor', entries, const {
        '/script/angelscript.documentsegment_glossary_ocr_grd_stone_orearmor': [
          'TEXT_STONE_ORE_ARMOR',
        ],
      });
      final catalog = {
        'ocr_grd_stone_219': {'german': 'Stone'},
        'text_stone_ore_armor': {
          'german': 'Er kann meine Erzrüstung verbessern.',
        },
      };

      expect(link, isNotNull);
      expect(link!.npcName(catalog, gameLangByCode('de')), 'Stone');
      expect(link.localizedParagraphs(catalog, gameLangByCode('de')), [
        'Er kann meine Erzrüstung verbessern.',
      ]);
      expect(findStoryGlossaryLink('Stone_Ore', entries, const {}), isNull);
    },
  );

  test('humanizes underscores and camel case without inventing semantics', () {
    expect(humanizeStoryId('Stone_OreArmor'), 'Stone Ore Armor');
    expect(
      humanizeStoryId('GuardPassageWarning_OC'),
      'Guard Passage Warning OC',
    );
  });

  test('does not invent a source type for an unknown stored id', () {
    final value = StoryStateValue.fromJson({
      'id': 'Mod_NewStoryValue',
      'rawValue': 123,
      'stored': true,
      'catalogKnown': false,
      'path': ['StoryPropertyValues', '{Mod_NewStoryValue}'],
      'semanticType': 'unknown',
      'declaredType': 'unknown',
    });

    expect(value.semanticType, StorySemanticType.unknown);
    expect(value.catalogKnown, isFalse);
    expect(value.value, 123);
  });
}
