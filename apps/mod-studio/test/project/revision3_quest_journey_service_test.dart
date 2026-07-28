import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_journey.dart';
import 'package:gore_mod/project/revision3_quest_journey_service.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

void main() {
  test(
    'loads one exact V4 journey without invoking any secondary reads or writers',
    () async {
      final calls = _ForbiddenCalls();
      final fixture = Revision3QuestOutlineFixture();
      final index = _v4Index();
      final service = _service(
        fixture: fixture,
        transitionIndex: index,
        transcriptIndex: index,
        calls: calls,
      );

      final journey = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
      );

      expect(journey.projectId, revision3QuestOutlineProjectId);
      expect(journey.projectRevision, 7);
      expect(journey.questId, revision3QuestOutlineQuestId);
      expect(journey.moduleId, revision3QuestOutlineModuleId);
      expect(
        journey.objectives.map((objective) => objective.stableObjectiveSlot),
        <int>[1, 2, 3],
      );
      expect(journey.orderedDialogLines, isEmpty);
      expect(calls.seedReads, 1);
      expect(calls.transcriptIndexReads, 1);
      calls.expectNoForbiddenCalls();
    },
  );

  test(
    'normalizes a transcript checkpoint mismatch to journey-stale',
    () async {
      final calls = _ForbiddenCalls();
      final fixture = Revision3QuestOutlineFixture();
      final index = _v4Index();
      final service = _service(
        fixture: fixture,
        transitionIndex: index,
        transcriptIndex: _v4Index(projectRevision: 8),
        calls: calls,
      );

      await expectLater(
        service.load(
          index: index,
          quest: index.entityById(revision3QuestOutlineQuestId)!,
        ),
        throwsA(isA<Revision3QuestJourneyStaleCheckpointException>()),
      );

      expect(calls.seedReads, 1);
      expect(calls.transcriptIndexReads, 1);
      calls.expectNoForbiddenCalls();
    },
  );

  test(
    'normalizes a requires-reopen seed read and never falls through to writers',
    () async {
      final calls = _ForbiddenCalls();
      final fixture = Revision3QuestOutlineFixture();
      final index = _v4Index();
      final service = _service(
        fixture: fixture,
        transitionIndex: index,
        transcriptIndex: index,
        calls: calls,
        seedRequiresReopen: true,
      );

      await expectLater(
        service.load(
          index: index,
          quest: index.entityById(revision3QuestOutlineQuestId)!,
        ),
        throwsA(isA<Revision3QuestJourneyRequiresReopenException>()),
      );

      expect(calls.seedReads, 1);
      expect(calls.transcriptIndexReads, 1);
      calls.expectNoForbiddenCalls();
    },
  );

  test(
    'requires-reopen beats an earlier FormatException after both reads settle',
    () async {
      final calls = _ForbiddenCalls();
      final fixture = Revision3QuestOutlineFixture();
      final index = _v4Index();
      final transitionSeed =
          Completer<AuthoringRevision3QuestTransitionsSeed>();
      final transcriptIndex = Completer<Revision3ContentIndex>();
      final service = _service(
        fixture: fixture,
        transitionIndex: index,
        transcriptIndex: index,
        calls: calls,
        transitionSeedRead: transitionSeed.future,
        transcriptIndexRead: transcriptIndex.future,
      );

      final load = service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
      );
      var completed = false;
      final completion = load.then<void>(
        (_) => completed = true,
        onError: (Object _, StackTrace _) => completed = true,
      );
      final expectation = expectLater(
        load,
        throwsA(isA<Revision3QuestJourneyRequiresReopenException>()),
      );

      transcriptIndex.completeError(
        const FormatException('stale transcript read'),
      );
      await Future<void>.delayed(Duration.zero);
      expect(
        completed,
        isFalse,
        reason: 'the second parallel read must settle',
      );

      transitionSeed.completeError(
        const Revision3QuestTransitionsRequiresReopenException(),
      );
      await expectation;
      await completion;

      expect(calls.seedReads, 1);
      expect(calls.transcriptIndexReads, 1);
      calls.expectNoForbiddenCalls();
    },
  );

  test(
    'unknown error beats a later stale error after both reads settle',
    () async {
      final calls = _ForbiddenCalls();
      final fixture = Revision3QuestOutlineFixture();
      final index = _v4Index();
      final transitionSeed =
          Completer<AuthoringRevision3QuestTransitionsSeed>();
      final transcriptIndex = Completer<Revision3ContentIndex>();
      final service = _service(
        fixture: fixture,
        transitionIndex: index,
        transcriptIndex: index,
        calls: calls,
        transitionSeedRead: transitionSeed.future,
        transcriptIndexRead: transcriptIndex.future,
      );

      final load = service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
      );
      var completed = false;
      final completion = load.then<void>(
        (_) => completed = true,
        onError: (Object _, StackTrace _) => completed = true,
      );
      final expectation = expectLater(
        load,
        throwsA(isA<Revision3QuestJourneyRequiresReopenException>()),
      );

      transitionSeed.completeError(StateError('unknown transition failure'));
      await Future<void>.delayed(Duration.zero);
      expect(
        completed,
        isFalse,
        reason: 'the second parallel read must settle',
      );

      transcriptIndex.completeError(
        const Revision3QuestTranscriptStaleCheckpointException(),
      );
      await expectation;
      await completion;

      expect(calls.seedReads, 1);
      expect(calls.transcriptIndexReads, 1);
      calls.expectNoForbiddenCalls();
    },
  );

  test('rejects a cloned visible Quest before any read boundary', () async {
    final calls = _ForbiddenCalls();
    final fixture = Revision3QuestOutlineFixture();
    final index = _v4Index();
    final clonedQuest = _v4Index().entityById(revision3QuestOutlineQuestId)!;
    final service = _service(
      fixture: fixture,
      transitionIndex: index,
      transcriptIndex: index,
      calls: calls,
    );

    await expectLater(
      service.load(index: index, quest: clonedQuest),
      throwsA(isA<Revision3QuestJourneyStaleCheckpointException>()),
    );

    expect(calls.seedReads, 0);
    expect(calls.transcriptIndexReads, 0);
    calls.expectNoForbiddenCalls();
  });
}

Revision3QuestJourneyService _service({
  required Revision3QuestOutlineFixture fixture,
  required Revision3ContentIndex transitionIndex,
  required Revision3ContentIndex transcriptIndex,
  required _ForbiddenCalls calls,
  bool seedRequiresReopen = false,
  Future<AuthoringRevision3QuestTransitionsSeed>? transitionSeedRead,
  Future<Revision3ContentIndex>? transcriptIndexRead,
}) {
  final transitions = Revision3QuestTransitionsAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
        }) async {
          calls.seedReads++;
          if (seedRequiresReopen) {
            throw const Revision3QuestTransitionsRequiresReopenException();
          }
          if (transitionSeedRead != null) return transitionSeedRead;
          return AuthoringRevision3QuestTransitionsSeed.forProject(
            currentProjectJson: fixture.projectJson,
            questId: questId,
            expectedQuestRevision: expectedQuestRevision,
            expectedModuleId: expectedModuleId,
            expectedModuleRevision: expectedModuleRevision,
          );
        },
    publishTechnicalPlan: ({required plan}) async {
      calls.transitionPublications++;
      throw StateError('Journey loading must never publish transitions.');
    },
  );
  final transcript = Revision3QuestTranscriptAuthoringService(
    expectedHead: fixture.head,
    loadContentIndex: () async {
      calls.transcriptIndexReads++;
      if (transcriptIndexRead != null) return transcriptIndexRead;
      return transcriptIndex;
    },
    readExactLocalization:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required localizationId,
          required expectedLocalizationRevision,
          required expectedLocId,
        }) async {
          calls.localizationReads++;
          throw StateError('Journey loading must never read localization.');
        },
    publishReplace:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async {
          calls.transcriptReplacements++;
          throw StateError('Journey loading must never replace a transcript.');
        },
    publishCreate:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async {
          calls.transcriptCreations++;
          throw StateError(
            'Journey loading must never create transcript rows.',
          );
        },
  );
  expect(transitionIndex.projectId, revision3QuestOutlineProjectId);
  return Revision3QuestJourneyService(
    transitions: transitions,
    transcript: transcript,
  );
}

final class _ForbiddenCalls {
  int seedReads = 0;
  int transcriptIndexReads = 0;
  int transitionPublications = 0;
  int localizationReads = 0;
  int transcriptReplacements = 0;
  int transcriptCreations = 0;

  void expectNoForbiddenCalls() {
    expect(transitionPublications, 0);
    expect(localizationReads, 0);
    expect(transcriptReplacements, 0);
    expect(transcriptCreations, 0);
  }
}

Revision3ContentIndex _v4Index({
  int projectRevision = 7,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': revision3QuestOutlineProjectId,
  'project_revision': projectRevision,
  'project_name': 'Quest journey service fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256': revision3QuestOutlineTargetSha,
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{'quest_draft': 1, 'script_module': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': revision3QuestOutlineQuestId,
      'kind': 'quest_draft',
      'display_name': 'Find Homer',
      'revision': 4,
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
          'additional_objective_titles': <Object?>[
            'Inspect the old gate',
            'Report the secured gate',
          ],
          'objective_slots': <Object?>[1, 2, 3],
          'transcript_count': 0,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
          'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
        },
      },
      'references': <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: revision3QuestOutlineModuleId,
          expectedKind: 'script_module',
        ),
      ],
      'asset_references': <Object?>[
        <String, Object?>{
          'role': 'quest_collision_artifact',
          'sha256': revision3QuestOutlineArtifactSha,
          'byte_len': 123,
          'logical_name': null,
          'expected_media_type':
              'application/vnd.gore.quest-collision-capability+json;version=2',
          'resolution': 'resolved',
        },
      ],
    },
    <String, Object?>{
      'id': revision3QuestOutlineModuleId,
      'kind': 'script_module',
      'display_name': 'Find Homer Script',
      'revision': 5,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'entity_id': revision3QuestOutlineQuestId,
          'expected_kind': 'quest_draft',
        },
      },
      'summary': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
      'references': <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: revision3QuestOutlineQuestId,
          expectedKind: 'quest_draft',
        ),
        _reference(
          role: 'script_owner',
          targetId: revision3QuestOutlineQuestId,
          expectedKind: 'quest_draft',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[
    <String, Object?>{
      'sha256': revision3QuestOutlineArtifactSha,
      'byte_len': 123,
      'media_type':
          'application/vnd.gore.quest-collision-capability+json;version=2',
      'class': 'quest_collision_artifact',
    },
  ],
});

Map<String, Object?> _reference({
  required String role,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': revision3QuestOutlineProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
