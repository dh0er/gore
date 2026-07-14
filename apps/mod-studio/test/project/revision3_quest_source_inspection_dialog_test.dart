import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_quest_source_inspection_dialog.dart';

const _projectId = '11111111111111111111111111111111';
const _questId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _source = 'class TestQuest : Quest {}\n';

void main() {
  testWidgets('shows friendly closed checks and generated source', (
    tester,
  ) async {
    await _openInspectionDialog(
      tester,
      inspect: ({required gameRoot, required questId}) async {
        expect(gameRoot, r'C:\Game');
        expect(questId, _questId);
        return _inspection();
      },
    );

    expect(find.text('Saved source verified'), findsOneWidget);
    expect(find.text('Source inputs verified'), findsOneWidget);
    expect(find.text('Exact project version checked'), findsOneWidget);
    final result = find.byKey(
      const Key('revision3-quest-source-inspection-result'),
    );
    final scrollable = find.descendant(
      of: result,
      matching: find.byType(Scrollable),
    );
    await tester.scrollUntilVisible(
      find.text('Build is still blocked'),
      180,
      scrollable: scrollable,
    );
    expect(find.text('Compilation was not run'), findsOneWidget);
    expect(find.text('Build is still blocked'), findsOneWidget);
    expect(find.text('In-game behavior is not qualified'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.byKey(const Key('revision3-quest-source-inspection-advanced')),
      180,
      scrollable: scrollable,
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-source-inspection-advanced')),
    );
    await tester.pumpAndSettle();

    expect(find.text('GoreMods.Quests.Test'), findsOneWidget);
    expect(find.text(_questId), findsOneWidget);
    expect(find.text(_moduleId), findsOneWidget);
    expect(find.text(_source), findsOneWidget);
  });

  testWidgets('retries a transient native input failure without closing', (
    tester,
  ) async {
    var calls = 0;
    await _openInspectionDialog(
      tester,
      inspect: ({required gameRoot, required questId}) async {
        calls++;
        if (calls == 1) {
          throw const ModFfiException(
            command: 'authoring_store_inspect_revision3_quest_source_v1',
            code: 'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_CHANGED',
            message: 'the game input changed during inspection',
          );
        }
        return _inspection();
      },
    );

    expect(
      find.byKey(const Key('revision3-quest-source-inspection-retry')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-quest-source-inspection-close-refresh')),
      findsNothing,
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-source-inspection-retry')),
    );
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('Saved source verified'), findsOneWidget);
  });

  testWidgets('stale and requires-reopen errors close for a fresh checkpoint', (
    tester,
  ) async {
    for (final error in <Object>[
      const Revision3QuestSourceInspectionStaleCheckpointException(),
      const Revision3QuestSourceInspectionRequiresReopenException(),
    ]) {
      var calls = 0;
      await _openInspectionDialog(
        tester,
        inspect: ({required gameRoot, required questId}) async {
          calls++;
          throw error;
        },
      );

      expect(
        find.byKey(const Key('revision3-quest-source-inspection-retry')),
        findsNothing,
      );
      final closeAndRefresh = find.byKey(
        const Key('revision3-quest-source-inspection-close-refresh'),
      );
      expect(closeAndRefresh, findsOneWidget);
      expect(find.text('Close and refresh'), findsOneWidget);
      if (error is Revision3QuestSourceInspectionStaleCheckpointException) {
        expect(
          find.textContaining('project changed after this Quest was selected'),
          findsOneWidget,
        );
      } else {
        expect(find.textContaining('must be reopened'), findsOneWidget);
      }

      await tester.tap(closeAndRefresh);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-source-inspection-dialog')),
        findsNothing,
      );
      expect(calls, 1);
    }
  });
}

Future<void> _openInspectionDialog(
  WidgetTester tester, {
  required Revision3QuestSourceInspectionLoader inspect,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            key: const Key('open-revision3-quest-source-inspection'),
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => Revision3QuestSourceInspectionDialog(
                questTitle: 'Secure the gate',
                questId: _questId,
                gameRoot: r'C:\Game',
                inspect: inspect,
              ),
            ),
            child: const Text('Open inspection'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(
    find.byKey(const Key('open-revision3-quest-source-inspection')),
  );
  await tester.pumpAndSettle();
}

AuthoringRevision3QuestSourceInspectionResult _inspection() {
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _sealJson(41, 'a'),
    }),
  );
  final sourceBytes = utf8.encode(_source);
  final sourceSha = crypto.sha256.convert(sourceBytes).toString();
  final projectSeal = _sealJson(2048, 'b');
  final questRef = <String, Object?>{
    'project_id': _projectId,
    'id': _questId,
    'expected_kind': 'quest_draft',
  };
  final plan = <String, Object?>{
    'format': 'revision3_quest_source_inspection_plan',
    'schema_revision': 3,
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': _projectId,
      'project_revision': 7,
      'target_executable': _sealJson(4096, 'c'),
      'canonical_project': projectSeal,
      'collision_basis_head': <String, Object?>{
        'store_format': 1,
        'snapshot': _sealJson(1024, 'd'),
      },
      'collision_basis_project': _sealJson(1024, 'e'),
      'collision_nonquest_project': _sealJson(900, 'f'),
      'collision_prior_quest_count': 2,
      'collision_prior_quest_evidence': _sealJson(64, '1'),
      'collision_artifact': _sealJson(8192, '2'),
      'collision_source': _sealJson(8192, '3'),
    },
    'module': <String, Object?>{
      'quest': questRef,
      'script_module': <String, Object?>{
        'project_id': _projectId,
        'id': _moduleId,
        'expected_kind': 'script_module',
      },
      'draft_input': _sealJson(512, '4'),
      'persisted_source': <String, Object?>{
        'byte_len': sourceBytes.length,
        'sha256': sourceSha,
      },
      'generated': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': questRef,
        'module_namespace': 'GoreMods.Quests.Test',
        'module_relative_path': 'GoreMods/Quests/Test.as',
        'source': _source,
        'source_sha256': sourceSha,
        'input_fingerprint': '5' * 64,
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  };
  final planJson = jsonEncode(plan);
  final planBytes = utf8.encode(planJson);
  return AuthoringRevision3QuestSourceInspectionResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'inspection_only',
      'head_json': head.canonicalJson,
      'project_id': _projectId,
      'project_revision': 7,
      'project_seal': projectSeal,
      'quest_id': _questId,
      'plan_json': planJson,
      'plan_seal': <String, Object?>{
        'byte_len': planBytes.length,
        'sha256': crypto.sha256.convert(planBytes).toString(),
      },
      'scope': 'source_inspection_only',
      'build_status': 'blocked',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedQuestId: _questId,
  );
}

Map<String, Object?> _sealJson(int byteLength, String hex) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': hex * 64,
};
