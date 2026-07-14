import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_content_library.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _questId = '33333333333333333333333333333333';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _assetSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

void main() {
  testWidgets('shows loading and exact-current content at desktop width', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final pending = Completer<Revision3ContentIndex>();

    await _pumpLibrary(tester, load: () => pending.future);

    expect(find.byKey(const Key('revision3-content-loading')), findsOneWidget);
    expect(find.text('Opening the exact current project...'), findsOneWidget);

    pending.complete(_fixture());
    await tester.pumpAndSettle();

    expect(find.text('Fixture project'), findsOneWidget);
    expect(find.text('2 entities / 1 assets / revision 7'), findsOneWidget);
    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsOneWidget);
    expect(
      find.byKey(Key('revision3-content-entity-$_questId')),
      findsOneWidget,
    );
    expect(
      find.text(
        'Read-only exact project view. Build readiness has not been evaluated; runtime behavior is unqualified.',
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsOneWidget,
    );
    expect(find.text('GORE_GATE_GUARD'), findsWidgets);
  });

  testWidgets('searches content and filters by semantic kind', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'characterdefinition_asghan',
    );
    await tester.pump();

    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsOneWidget);
    expect(find.byKey(Key('revision3-content-entity-$_questId')), findsNothing);

    await tester.tap(find.byTooltip('Clear search'));
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-content-filter-quest_draft')),
    );
    await tester.pump();

    expect(find.byKey(Key('revision3-content-entity-$_npcId')), findsNothing);
    expect(
      find.byKey(Key('revision3-content-entity-$_questId')),
      findsOneWidget,
    );
    expect(find.text('Find Homer'), findsWidgets);
  });

  testWidgets('switches to searchable read-only assets', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpLoadedLibrary(tester);

    await tester.tap(find.byKey(const Key('revision3-content-mode-assets')));
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-content-asset-list')),
      findsOneWidget,
    );
    expect(
      find.byKey(Key('revision3-content-asset-$_assetSha')),
      findsOneWidget,
    );
    expect(find.text('Voice audio'), findsWidgets);
    expect(find.text('audio/ogg'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-content-asset-details')),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-content-search')),
      'missing asset',
    );
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-content-asset-empty')),
      findsOneWidget,
    );
  });

  testWidgets('opens details from the compact one-pane layout', (tester) async {
    await _setSurfaceSize(tester, const Size(560, 760));
    await _pumpLoadedLibrary(tester);

    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsNothing,
    );

    await tester.tap(find.byKey(Key('revision3-content-entity-$_npcId')));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-content-entity-details')),
      findsOneWidget,
    );
    expect(find.text('Stable ID'), findsOneWidget);
    expect(find.text(_npcId), findsOneWidget);
  });

  testWidgets('shows a friendly error and retries the exact reopen', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var calls = 0;
    await _pumpLibrary(
      tester,
      load: () {
        calls += 1;
        if (calls == 1) return Future.error(StateError('fixture offline'));
        return Future.value(_fixture());
      },
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-content-error')), findsOneWidget);
    expect(find.textContaining('fixture offline'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-content-retry')));
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.byKey(const Key('revision3-content-error')), findsNothing);
    expect(find.text('Fixture project'), findsOneWidget);
  });

  testWidgets('rejects an index from another project checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    await _pumpLibrary(
      tester,
      projectRevision: 8,
      load: () async => _fixture(),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-content-error')), findsOneWidget);
    expect(
      find.text('Content index does not match the current project checkpoint.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-content-entity-list')),
      findsNothing,
    );
  });

  testWidgets('ignores loader closure identity but reloads a new checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var calls = 0;
    var revision = 7;
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3ContentLibrary(
                projectId: _projectId,
                projectRevision: revision,
                load: () async {
                  calls += 1;
                  return _fixture(revision: revision);
                },
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(calls, 1);

    rebuild(() {});
    await tester.pumpAndSettle();
    expect(calls, 1);

    rebuild(() => revision = 8);
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('2 entities / 1 assets / revision 8'), findsOneWidget);
  });
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pumpLoadedLibrary(WidgetTester tester) async {
  await _pumpLibrary(tester, load: () async => _fixture());
  await tester.pumpAndSettle();
}

Future<void> _pumpLibrary(
  WidgetTester tester, {
  required Revision3ContentIndexLoader load,
  int projectRevision = 7,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3ContentLibrary(
        projectId: _projectId,
        projectRevision: projectRevision,
        load: load,
      ),
    ),
  ),
);

Revision3ContentIndex _fixture({int revision = 7}) =>
    Revision3ContentIndex.fromJsonObject(<String, Object?>{
      'schema_revision': 1,
      'project_id': _projectId,
      'project_revision': revision,
      'project_name': 'Fixture project',
      'project_version': '0.1.0',
      'project_author': 'GORE',
      'target': <String, Object?>{
        'executable': <String, Object?>{'byte_len': 123, 'sha256': _targetSha},
      },
      'authoring_locales': <Object?>['de', 'en'],
      'entity_counts': <String, Object?>{'npc_draft': 1, 'quest_draft': 1},
      'entities': <Object?>[
        <String, Object?>{
          'id': _npcId,
          'kind': 'npc_draft',
          'display_name': 'Gate Guard',
          'revision': 0,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': 'GORE_GATE_GUARD',
          },
          'summary': <String, Object?>{
            'kind': 'npc_draft',
            'data': <String, Object?>{
              'unique_name': 'GORE_GATE_GUARD',
              'module_namespace': 'PROJECT.NPCS.GATEGUARD',
              'parent_character_definition': 'UCharacterDefinition_Asghan',
              'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
              'parent_spawn_definition': 'USpawnAIAgentDefinition_Asghan',
            },
          },
          'references': <Object?>[],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': _questId,
          'kind': 'quest_draft',
          'display_name': 'Find Homer',
          'revision': 1,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': 'GORE_FIND_HOMER',
          },
          'summary': <String, Object?>{
            'kind': 'quest_draft',
            'data': <String, Object?>{
              'technical_id': 'GORE_FIND_HOMER',
              'title': 'Find Homer',
              'objective_title': 'Ask Asghan about Homer',
              'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
              'parent_runtime_class': 'B_Quest_FindHomer_C',
              'giver_runtime_unique_name': 'ASGHAN',
            },
          },
          'references': <Object?>[],
          'asset_references': <Object?>[],
        },
      ],
      'assets': <Object?>[
        <String, Object?>{
          'sha256': _assetSha,
          'byte_len': 4096,
          'media_type': 'audio/ogg',
          'class': 'voice_audio',
        },
      ],
    });
