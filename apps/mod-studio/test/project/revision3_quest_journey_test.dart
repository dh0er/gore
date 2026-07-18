import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_journey.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

typedef _TranscriptBinding = ({int lineIndex, int? objectiveSlot});

const _otherQuestId = '60000000000000000000000000000000';
const _otherModuleId = '70000000000000000000000000000000';

void main() {
  test(
    'V4 composes stable objectives, root behavior, authored dialog order and sharing',
    () async {
      final parts = await _JourneyFixture(
        bindings: const <_TranscriptBinding>[
          (lineIndex: 1, objectiveSlot: 3),
          (lineIndex: 0, objectiveSlot: null),
          (lineIndex: 2, objectiveSlot: 1),
        ],
        sharedLineIndex: 1,
      ).load();

      final journey = parts.compose();

      expect(journey.projectId, revision3QuestOutlineProjectId);
      expect(journey.projectRevision, 7);
      expect(journey.checkpointIdentity, parts.head.canonicalJson);
      expect(journey.questId, revision3QuestOutlineQuestId);
      expect(journey.moduleId, revision3QuestOutlineModuleId);
      expect(
        journey.rootBehavior.orderedTransitions.map(
          (transition) => transition.edge,
        ),
        <AuthoringRevision3QuestTransitionEdgeV1>[
          AuthoringRevision3QuestTransitionEdgeV1.availability,
          AuthoringRevision3QuestTransitionEdgeV1.start,
        ],
      );
      expect(
        journey.objectives.map((objective) => objective.transitionSlot),
        <int>[1, 2, 3],
      );
      expect(
        journey.objectives.map((objective) => objective.stableObjectiveSlot),
        <int>[1, 2, 3],
      );
      expect(
        journey.objectives.map((objective) => objective.title),
        const <String>[
          'Ask Asghan about Homer',
          'Inspect the old gate',
          'Report the secured gate',
        ],
      );
      expect(journey.orderedDialogLines.map((line) => line.lineId), <String>[
        _lineId(1),
        _lineId(0),
        _lineId(2),
      ]);
      expect(
        journey.orderedDialogLines.map((line) => line.transcriptIndex),
        <int>[0, 1, 2],
      );
      expect(journey.generalDialogLines.single.lineId, _lineId(0));
      expect(
        journey.objectives
            .singleWhere((objective) => objective.stableObjectiveSlot == 3)
            .dialogLines
            .single
            .lineId,
        _lineId(1),
      );
      expect(
        journey.objectives
            .singleWhere((objective) => objective.stableObjectiveSlot == 1)
            .dialogLines
            .single
            .transcriptIndex,
        2,
      );
      final shared = journey.orderedDialogLines.first;
      expect(shared.isSharedAcrossQuests, isTrue);
      expect(shared.linkedQuestCount, 2);
      expect(identical(shared.row, parts.transcript.rows.first), isTrue);
    },
  );

  test('derives the two persisted Draft setup stages', () async {
    final completeParts = await _JourneyFixture(
      bindings: const <_TranscriptBinding>[(lineIndex: 0, objectiveSlot: null)],
    ).load();
    final complete = completeParts.compose().draftSetup;

    expect(complete.questDetailsComplete, isTrue);
    expect(complete.openingDialogComplete, isTrue);
    expect(complete.draftSetupComplete, isTrue);
    expect(complete.openingDialogLineCount, 1);
    expect(complete.openingTextLanguageCount, 1);
    expect(complete.openingVoiceTakeCount, 0);
    expect(complete.openingSelectedVoiceTakeCount, 0);
    expect(
      complete.recommendedStep,
      Revision3QuestDraftSetupStepKind.openingDialog,
      reason: 'complete Draft setup conservatively continues to dialog review',
    );

    final emptyParts = await _JourneyFixture(
      bindings: const <_TranscriptBinding>[],
    ).load();
    final empty = emptyParts.compose().draftSetup;
    expect(empty.questDetailsComplete, isTrue);
    expect(empty.openingDialogComplete, isFalse);
    expect(empty.draftSetupComplete, isFalse);
    expect(
      empty.recommendedStep,
      Revision3QuestDraftSetupStepKind.openingDialog,
    );
  });

  test(
    'fails closed on project, entity, module, seed and transcript drift',
    () async {
      final fixture = _JourneyFixture(
        bindings: const <_TranscriptBinding>[
          (lineIndex: 0, objectiveSlot: 1),
          (lineIndex: 1, objectiveSlot: null),
        ],
        sharedLineIndex: 0,
      );
      final parts = await fixture.load();
      final stale = isA<Revision3QuestJourneyStaleCheckpointException>();

      final laterSeed = _JourneyFixture(
        projectRevision: 8,
        bindings: fixture.bindings,
      ).seed();
      expect(() => parts.compose(seed: laterSeed), throwsA(stale));

      final changedModuleSeed = _JourneyFixture(
        moduleRevision: 6,
        bindings: fixture.bindings,
      ).seed();
      expect(() => parts.compose(seed: changedModuleSeed), throwsA(stale));

      final changedQuestSeed = _JourneyFixture(
        questRevision: 5,
        bindings: fixture.bindings,
      ).seed();
      expect(() => parts.compose(seed: changedQuestSeed), throwsA(stale));

      final clonedIndex = fixture.index();
      expect(
        () => parts.compose(
          quest: clonedIndex.entityById(revision3QuestOutlineQuestId),
        ),
        throwsA(stale),
      );
      expect(
        () => parts.compose(module: parts.index.entityById(_otherModuleId)),
        throwsA(stale),
      );

      final changedTitles = await _JourneyFixture(
        objectiveTitles: const <String>[
          'Ask Diego about Homer',
          'Inspect the old gate',
          'Report the secured gate',
        ],
        bindings: fixture.bindings,
      ).load();
      expect(() => changedTitles.compose(seed: parts.seed), throwsA(stale));

      final reversedTranscript = await _JourneyFixture(
        bindings: fixture.bindings.reversed.toList(growable: false),
      ).load();
      expect(
        () => parts.compose(transcript: reversedTranscript.transcript),
        throwsA(stale),
      );
    },
  );

  test('accepts exactly 256 rows without localization reads', () async {
    var localizationReads = 0;
    final parts = await _JourneyFixture(
      bindings: List<_TranscriptBinding>.generate(
        revision3QuestJourneyMaxDialogLines,
        (index) => (lineIndex: index, objectiveSlot: null),
      ),
    ).load(onLocalizationRead: () => localizationReads++);

    final journey = parts.compose();

    expect(journey.orderedDialogLines, hasLength(256));
    expect(journey.generalDialogLines, hasLength(256));
    expect(localizationReads, 0);
  });

  test('rejects 257 transcript rows at the exact ContentIndex boundary', () {
    final fixture = _JourneyFixture(
      bindings: List<_TranscriptBinding>.generate(
        revision3QuestJourneyMaxDialogLines + 1,
        (index) => (lineIndex: index, objectiveSlot: null),
      ),
    );

    expect(
      () => Revision3ContentIndex.fromJsonObject(fixture.contentIndexJson()),
      throwsFormatException,
    );
  });
}

final class _JourneyParts {
  const _JourneyParts({
    required this.index,
    required this.quest,
    required this.module,
    required this.seed,
    required this.transcript,
    required this.head,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final Revision3ContentEntity module;
  final AuthoringRevision3QuestTransitionsSeed seed;
  final Revision3QuestTranscriptProjection transcript;
  final AuthoringWorkingHead head;

  Revision3QuestJourneyProjection compose({
    Revision3ContentEntity? quest,
    Revision3ContentEntity? module,
    AuthoringRevision3QuestTransitionsSeed? seed,
    Revision3QuestTranscriptProjection? transcript,
  }) => Revision3QuestJourneyProjection.compose(
    index: index,
    quest: quest ?? this.quest,
    module: module ?? this.module,
    transitionSeed: seed ?? this.seed,
    transcript: transcript ?? this.transcript,
  );
}

final class _JourneyFixture {
  const _JourneyFixture({
    required this.bindings,
    this.projectRevision = 7,
    this.questRevision = 4,
    this.moduleRevision = 5,
    this.objectiveTitles = const <String>[
      'Ask Asghan about Homer',
      'Inspect the old gate',
      'Report the secured gate',
    ],
    this.sharedLineIndex,
  });

  final int projectRevision;
  final int questRevision;
  final int moduleRevision;
  final List<String> objectiveTitles;
  final List<_TranscriptBinding> bindings;
  final int? sharedLineIndex;

  Revision3QuestOutlineFixture get outline => Revision3QuestOutlineFixture(
    projectRevision: projectRevision,
    questRevision: questRevision,
    moduleRevision: moduleRevision,
    objectiveTitles: objectiveTitles,
  );

  AuthoringRevision3QuestTransitionsSeed seed() =>
      AuthoringRevision3QuestTransitionsSeed.forProject(
        currentProjectJson: outline.projectJson,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: moduleRevision,
      );

  Future<_JourneyParts> load({void Function()? onLocalizationRead}) async {
    final exactIndex = index();
    final head = outline.head;
    final service = Revision3QuestTranscriptAuthoringService(
      expectedHead: head,
      loadContentIndex: () async => exactIndex,
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            onLocalizationRead?.call();
            throw StateError('Journey composition must not read text');
          },
      publishReplace:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async => throw StateError('Journey composition is read-only'),
      publishCreate:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async => throw StateError('Journey composition is read-only'),
    );
    final transcript = await service.load(
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: questRevision,
    );
    return _JourneyParts(
      index: exactIndex,
      quest: exactIndex.entityById(revision3QuestOutlineQuestId)!,
      module: exactIndex.entityById(revision3QuestOutlineModuleId)!,
      seed: seed(),
      transcript: transcript,
      head: head,
    );
  }

  Revision3ContentIndex index() =>
      Revision3ContentIndex.fromJsonObject(contentIndexJson());

  Map<String, Object?> contentIndexJson() {
    final lineCount = bindings.isEmpty
        ? 0
        : 1 +
              bindings
                  .map((binding) => binding.lineIndex)
                  .reduce(
                    (largest, current) => largest > current ? largest : current,
                  );
    if (bindings.any(
      (binding) => binding.lineIndex < 0 || binding.lineIndex >= lineCount,
    )) {
      throw StateError('invalid test line index');
    }
    final hasSharedLine = sharedLineIndex != null;
    if (hasSharedLine &&
        (sharedLineIndex! < 0 || sharedLineIndex! >= lineCount)) {
      throw StateError('invalid shared test line');
    }
    return <String, Object?>{
      'schema_revision': 1,
      'project_id': revision3QuestOutlineProjectId,
      'project_revision': projectRevision,
      'project_name': 'Quest journey fixture',
      'project_version': '1.0.0',
      'project_author': 'tests',
      'target': _target(),
      'authoring_locales': <Object?>['de'],
      'entity_counts': <String, Object?>{
        if (lineCount > 0) 'localization_entry': lineCount,
        if (lineCount > 0) 'dialog_line': lineCount,
        'quest_draft': hasSharedLine ? 2 : 1,
        'script_module': hasSharedLine ? 2 : 1,
      },
      'entities': <Object?>[
        _questEntity(),
        _moduleEntity(
          id: revision3QuestOutlineModuleId,
          ownerId: revision3QuestOutlineQuestId,
          namespace: 'PROJECT.QUESTS.FINDHOMER',
        ),
        for (var index = 0; index < lineCount; index++) _lineEntity(index),
        for (var index = 0; index < lineCount; index++)
          _localizationEntity(index),
        if (hasSharedLine) _otherQuestEntity(sharedLineIndex!),
        if (hasSharedLine)
          _moduleEntity(
            id: _otherModuleId,
            ownerId: _otherQuestId,
            namespace: 'PROJECT.QUESTS.SHAREDREVIEW',
          ),
      ],
      'assets': <Object?>[],
    };
  }

  Map<String, Object?> _questEntity() => _entity(
    id: revision3QuestOutlineQuestId,
    kind: 'quest_draft',
    displayName: 'Find Homer',
    revision: questRevision,
    summaryData: <String, Object?>{
      'technical_id': 'GORE_FIND_HOMER',
      'title': 'Find Homer',
      'objective_title': objectiveTitles.first,
      if (objectiveTitles.length > 1)
        'additional_objective_titles': objectiveTitles.skip(1).toList(),
      'objective_slots': List<int>.generate(
        objectiveTitles.length,
        (index) => index + 1,
      ),
      'transcript_count': bindings.length,
      'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
      'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
      'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
    },
    references: <Object?>[
      _reference(
        role: 'draft_script_module',
        targetId: revision3QuestOutlineModuleId,
        expectedKind: 'script_module',
      ),
      for (final binding in bindings)
        _reference(
          role: 'quest_transcript_line',
          qualifier: binding.objectiveSlot?.toString(),
          targetId: _lineId(binding.lineIndex),
          expectedKind: 'dialog_line',
        ),
    ],
  );

  Map<String, Object?> _otherQuestEntity(int lineIndex) => _entity(
    id: _otherQuestId,
    kind: 'quest_draft',
    displayName: 'Review shared report',
    revision: 1,
    summaryData: <String, Object?>{
      'technical_id': 'GORE_SHARED_REVIEW',
      'title': 'Review shared report',
      'objective_title': 'Review the report',
      'objective_slots': <Object?>[1],
      'transcript_count': 1,
      'module_namespace': 'PROJECT.QUESTS.SHAREDREVIEW',
      'parent_runtime_class': 'UQuest_SwampCamp',
      'giver_runtime_unique_name': 'OM_GRD_Diego_251',
    },
    references: <Object?>[
      _reference(
        role: 'draft_script_module',
        targetId: _otherModuleId,
        expectedKind: 'script_module',
      ),
      _reference(
        role: 'quest_transcript_line',
        targetId: _lineId(lineIndex),
        expectedKind: 'dialog_line',
      ),
    ],
  );

  Map<String, Object?> _moduleEntity({
    required String id,
    required String ownerId,
    required String namespace,
  }) => _entity(
    id: id,
    kind: 'script_module',
    displayName: '$namespace Script',
    revision: id == revision3QuestOutlineModuleId ? moduleRevision : 1,
    origin: _generatedOrigin(ownerId: ownerId),
    summaryData: <String, Object?>{
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': 4,
      'module_namespace': namespace,
      'module_relative_path': '${namespace.replaceAll('.', '/')}.as',
      'status': <String, Object?>{
        'authoring': 'offline_draft',
        'runtime': 'runtime_unqualified',
      },
    },
    references: <Object?>[
      _reference(
        role: 'origin_owner',
        targetId: ownerId,
        expectedKind: 'quest_draft',
      ),
    ],
  );

  Map<String, Object?> _lineEntity(int index) => _entity(
    id: _lineId(index),
    kind: 'dialog_line',
    displayName: 'Dialog line ${index + 1}',
    revision: 1,
    summaryData: <String, Object?>{
      'speaker_hint': 'Asghan',
      'voice_slot_locales': <Object?>[],
    },
    references: <Object?>[
      _reference(
        role: 'dialog_localization',
        targetId: _localizationId(index),
        expectedKind: 'localization_entry',
      ),
    ],
  );

  Map<String, Object?> _localizationEntity(int index) => _entity(
    id: _localizationId(index),
    kind: 'localization_entry',
    displayName: 'Dialog text ${index + 1}',
    revision: 1,
    summaryData: <String, Object?>{
      'loc_id': 'DIA_JOURNEY_${index + 1}',
      'locales': <Object?>['de'],
    },
  );
}

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summaryData,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
}) => <String, Object?>{
  'id': id,
  'kind': kind,
  'display_name': displayName,
  'revision': revision,
  'origin':
      origin ??
      <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'AUTHORED_${kind.toUpperCase()}_$id',
      },
  'summary': <String, Object?>{'kind': kind, 'data': summaryData},
  'references': references,
  'asset_references': <Object?>[],
};

Map<String, Object?> _generatedOrigin({required String ownerId}) =>
    <String, Object?>{
      'type': 'generated',
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': 4,
      'owner': <String, Object?>{
        'project_id': revision3QuestOutlineProjectId,
        'entity_id': ownerId,
        'expected_kind': 'quest_draft',
      },
    };

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3QuestOutlineProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Map<String, Object?> _target() => <String, Object?>{
  'executable': <String, Object?>{
    'byte_len': 171698176,
    'sha256': revision3QuestOutlineTargetSha,
  },
};

String _lineId(int index) =>
    '4${(index + 1).toRadixString(16).padLeft(31, '0')}';

String _localizationId(int index) =>
    '5${(index + 1).toRadixString(16).padLeft(31, '0')}';
