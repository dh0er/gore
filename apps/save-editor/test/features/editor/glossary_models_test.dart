import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_models.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/glossary_segment_text_catalog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('GlossaryPage parses quest documents and raw NPC segment unlocks', () {
    final page = GlossaryPage.fromJson({
      'total': 1,
      'heroMemoryArrayPath': [
        'LongTermMemoryByGlobalId',
        '{Hero}',
        'MemorizedEvents',
      ],
      'writable': ['private.glossary.setSegment'],
      'categories': [
        {
          'id': 'creatures',
          'group': 'CreaturesGlossary',
          'total': 1,
          'entries': [
            {
              'id': 'MeatbugGlossary',
              'name': 'Meatbug',
              'documentClass': '/Script/Angelscript.Document_Glossary_Meatbug',
              'questClass':
                  '/Script/Angelscript.Quest_CreaturesGlossary_MeatbugGlossary',
              'unlocked': true,
              'writable': true,
              'segments': [
                {
                  'id': 'MeatbugUnlock',
                  'name': 'Unlock',
                  'segmentClass':
                      '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Unlock',
                  'currentState': 'EQuestState::Succeeded',
                  'statePath': [
                    'QuestDataByClass',
                    '{MeatbugUnlock}',
                    'CurrentState',
                  ],
                  'eventIndices': [17],
                  'viewedEventIndices': [19],
                  'unlocked': true,
                  'writable': true,
                },
              ],
            },
          ],
        },
      ],
      'segmentUnlocks': [
        {
          'documentClass': '/Script/Angelscript.Document_Glossary_SC_NOV_CAINE',
          'segmentClass':
              '/Script/Angelscript.DocumentSegment_Glossary_SC_NOV_CAINE_Introduction',
          'unlockedEventIndices': [4],
          'viewedEventIndices': [5],
        },
      ],
    });

    expect(page.total, 1);
    expect(page.heroMemoryArrayPath, hasLength(3));
    expect(page.canSetSegment, isTrue);
    final creature = page.category('creatures')!.entries.single;
    expect(creature.name, 'Meatbug');
    expect(creature.segments.single.unlocked, isTrue);
    expect(creature.segments.single.eventIndices, [17]);
    expect(creature.segments.single.statePath.last, 'CurrentState');
    expect(page.segmentUnlocks.single.unlocked, isTrue);
  });

  test('GlossarySegmentEdit emits the atomic core operation', () {
    const edit = GlossarySegmentEdit(
      documentClass: '/Script/Angelscript.Document_Glossary_Meatbug',
      segmentClass:
          '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Entry2',
      unlocked: false,
      questStatePath: [
        'QuestDataByClass',
        '{/Script/Angelscript.Quest_CreaturesGlossary_MeatbugGlossary_MeatbugEntry2}',
        'CurrentState',
      ],
    );

    expect(edit.toEditJson(), {
      'path': 'private.glossary.setSegment',
      'value': {
        'documentClass': '/Script/Angelscript.Document_Glossary_Meatbug',
        'segmentClass':
            '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Entry2',
        'unlocked': false,
        'questStatePath': [
          'QuestDataByClass',
          '{/Script/Angelscript.Quest_CreaturesGlossary_MeatbugGlossary_MeatbugEntry2}',
          'CurrentState',
        ],
      },
    });
  });

  test('NPC catalog segments parse camp and filter roles', () {
    final entry = NpcGlossaryCatalogEntry.fromJson({
      'id': 'NC_ORG_BUSTER',
      'uniqueName': 'NC_ORG_BUSTER_833',
      'documentClass': '/Script/Angelscript.Document_Glossary_NC_ORG_BUSTER',
      'camp': 'newCamp',
      'segments': [
        {
          'id': 'Introduction_2',
          'class':
              '/Script/Angelscript.DocumentSegment_Glossary_NC_ORG_BUSTER_Introduction_2',
          'label': 'Introduction 2',
          'roles': ['portrait'],
        },
        {
          'id': 'Introduction',
          'class':
              '/Script/Angelscript.DocumentSegment_Glossary_NC_ORG_BUSTER_Introduction',
          'label': 'Introduction',
          'roles': ['portrait'],
        },
        {
          'id': 'Teacher',
          'class':
              '/Script/Angelscript.DocumentSegment_Glossary_NC_ORG_BUSTER_Teacher',
          'label': 'Teacher',
          'roles': ['teacher'],
        },
      ],
    });

    expect(entry.camp, NpcGlossaryCamp.newCamp);
    expect(entry.portraitSegment?.id, 'Introduction');
    expect(entry.segments.last.roles, contains(NpcGlossaryRole.teacher));
  });

  test('bundled NPC catalog is complete and collision-free', () async {
    final entries = await loadGlossaryNpcCatalog();
    expect(entries, hasLength(160));
    expect(
      entries.where((entry) => entry.camp == NpcGlossaryCamp.oldCamp),
      hasLength(63),
    );
    expect(
      entries.where((entry) => entry.camp == NpcGlossaryCamp.newCamp),
      hasLength(41),
    );
    expect(
      entries.where((entry) => entry.camp == NpcGlossaryCamp.swampCamp),
      hasLength(34),
    );
    expect(
      entries.where((entry) => entry.camp == NpcGlossaryCamp.outsiders),
      hasLength(22),
    );

    final documents = <String>{};
    final segments = <String>{};
    var segmentCount = 0;
    for (final entry in entries) {
      expect(documents.add(entry.documentClass), isTrue);
      for (final segment in entry.segments) {
        segmentCount++;
        expect(
          segments.add('${entry.documentClass}\u0000${segment.segmentClass}'),
          isTrue,
        );
      }
    }
    expect(segmentCount, 590);
  });

  test('bundled segment text catalog covers every glossary segment', () async {
    final catalog = await loadGlossarySegmentTextCatalog();
    expect(catalog, hasLength(734));
    expect(
      catalog.values.fold<int>(0, (total, ids) => total + ids.length),
      759,
    );
    expect(
      catalog['/script/angelscript.documentsegment_glossary_wolf_entry2'],
      ['TEXT_WIP_OLOSEG_20250819_112959'],
    );

    final textIds = catalog.values.expand((ids) => ids).toList();
    expect(textIds.toSet(), hasLength(textIds.length));

    final npcCatalog = await loadGlossaryNpcCatalog();
    for (final document in npcCatalog) {
      for (final segment in document.segments) {
        expect(
          catalog,
          contains(segment.segmentClass.toLowerCase()),
          reason: 'Missing text for ${segment.segmentClass}',
        );
      }
    }
  });
}
