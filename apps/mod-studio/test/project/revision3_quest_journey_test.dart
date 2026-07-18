import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_journey.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';

import '../support/revision3_quest_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';

typedef _TranscriptBinding = ({int lineIndex, int? objectiveSlot});

const _otherQuestId = '60000000000000000000000000000000';
const _otherModuleId = '70000000000000000000000000000000';

void main() {
  test(
    'V4 composes stable objectives, root behavior, authored dialog order and sharing',
    () async {
      final parts = await _JourneyFixture(
        generatorVersion: 4,
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
      expect(journey.legacySyntheticBehavior, isFalse);
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
        <int?>[1, 2, 3],
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

  test(
    'derives two persisted Draft stages plus conditional legacy migration',
    () async {
      final completeParts = await _JourneyFixture(
        generatorVersion: 4,
        bindings: const <_TranscriptBinding>[
          (lineIndex: 0, objectiveSlot: null),
        ],
      ).load();
      final complete = completeParts.compose().draftSetup;

      expect(complete.questDetailsComplete, isTrue);
      expect(complete.openingDialogComplete, isTrue);
      expect(complete.legacyBehaviorReviewRequired, isFalse);
      expect(complete.draftSetupComplete, isTrue);
      expect(complete.openingDialogLineCount, 1);
      expect(complete.openingTextLanguageCount, 1);
      expect(complete.openingVoiceTakeCount, 0);
      expect(complete.openingSelectedVoiceTakeCount, 0);
      expect(
        complete.recommendedStep,
        Revision3QuestDraftSetupStepKind.openingDialog,
        reason:
            'complete Draft setup conservatively continues to dialog review',
      );

      final emptyLegacyParts = await _JourneyFixture(
        generatorVersion: 3,
        bindings: const <_TranscriptBinding>[],
      ).load();
      final emptyLegacy = emptyLegacyParts.compose().draftSetup;
      expect(emptyLegacy.questDetailsComplete, isTrue);
      expect(emptyLegacy.openingDialogComplete, isFalse);
      expect(emptyLegacy.legacyBehaviorReviewRequired, isTrue);
      expect(emptyLegacy.draftSetupComplete, isFalse);
      expect(
        emptyLegacy.recommendedStep,
        Revision3QuestDraftSetupStepKind.openingDialog,
      );

      final dialogLegacyParts = await _JourneyFixture(
        generatorVersion: 3,
        bindings: const <_TranscriptBinding>[
          (lineIndex: 0, objectiveSlot: null),
        ],
      ).load();
      final dialogLegacy = dialogLegacyParts.compose().draftSetup;
      expect(dialogLegacy.openingDialogComplete, isTrue);
      expect(dialogLegacy.legacyBehaviorReviewRequired, isTrue);
      expect(
        dialogLegacy.recommendedStep,
        Revision3QuestDraftSetupStepKind.legacyBehavior,
      );
    },
  );

  for (final generatorVersion in <int>[2, 3]) {
    test(
      'generator V$generatorVersion keeps synthetic behavior but never invents transcript grouping',
      () async {
        final objectiveTitles = generatorVersion == 2
            ? const <String>['Find the gate key']
            : const <String>[
                'Ask Asghan about Homer',
                'Inspect the old gate',
                'Report the secured gate',
              ];
        final parts = await _JourneyFixture(
          generatorVersion: generatorVersion,
          objectiveTitles: objectiveTitles,
          bindings: const <_TranscriptBinding>[
            (lineIndex: 0, objectiveSlot: null),
            (lineIndex: 1, objectiveSlot: null),
          ],
        ).load();

        final journey = parts.compose();

        expect(journey.legacySyntheticBehavior, isTrue);
        expect(journey.objectives, hasLength(objectiveTitles.length));
        expect(
          journey.objectives.map((objective) => objective.stableObjectiveSlot),
          everyElement(isNull),
        );
        expect(
          journey.objectives.expand((objective) => objective.dialogLines),
          isEmpty,
        );
        expect(journey.generalDialogLines.map((line) => line.lineId), <String>[
          _lineId(0),
          _lineId(1),
        ]);
        expect(journey.objectives.first.behavior.availability.node.slot, 1);
      },
    );
  }

  test(
    'fails closed on project, entity, module, seed and transcript drift',
    () async {
      final fixture = _JourneyFixture(
        generatorVersion: 4,
        bindings: const <_TranscriptBinding>[
          (lineIndex: 0, objectiveSlot: 1),
          (lineIndex: 1, objectiveSlot: null),
        ],
        sharedLineIndex: 0,
      );
      final parts = await fixture.load();
      final stale = isA<Revision3QuestJourneyStaleCheckpointException>();

      final laterSeed = _JourneyFixture(
        generatorVersion: 4,
        projectRevision: 8,
        bindings: fixture.bindings,
      ).seed();
      expect(() => parts.compose(seed: laterSeed), throwsA(stale));

      final changedModuleSeed = _JourneyFixture(
        generatorVersion: 4,
        moduleRevision: 6,
        bindings: fixture.bindings,
      ).seed();
      expect(() => parts.compose(seed: changedModuleSeed), throwsA(stale));

      final changedQuestSeed = _JourneyFixture(
        generatorVersion: 4,
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
        generatorVersion: 4,
        objectiveTitles: const <String>[
          'Ask Diego about Homer',
          'Inspect the old gate',
          'Report the secured gate',
        ],
        bindings: fixture.bindings,
      ).load();
      expect(() => changedTitles.compose(seed: parts.seed), throwsA(stale));

      final reversedTranscript = await _JourneyFixture(
        generatorVersion: 4,
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
      generatorVersion: 4,
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
      generatorVersion: 4,
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
    required this.generatorVersion,
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

  final int generatorVersion;
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
        currentProjectJson: switch (generatorVersion) {
          2 => _legacyV2ProjectJson(outline),
          3 => outline.projectJson,
          4 => outline.semanticProjectJson,
          _ => throw StateError('unsupported test generator'),
        },
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
    if (generatorVersion < 4 &&
        bindings.any((binding) => binding.objectiveSlot != null)) {
      throw StateError('legacy test bindings cannot carry objective slots');
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
      if (generatorVersion == 4)
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
      if (generatorVersion == 4) 'objective_slots': <Object?>[1],
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
    origin: _generatedOrigin(
      ownerId: ownerId,
      generatorVersion: generatorVersion,
    ),
    summaryData: <String, Object?>{
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': generatorVersion,
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

String _legacyV2ProjectJson(Revision3QuestOutlineFixture fixture) {
  final project = fixture.projectObject();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  final questPayload = (quest['payload']! as Map).cast<String, Object?>();
  final questData = (questPayload['data']! as Map).cast<String, Object?>();
  questData['generator_version'] = 2;
  final input = (questData['input']! as Map).cast<String, Object?>();
  input.remove('additional_objective_titles');

  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  final origin = (module['origin']! as Map).cast<String, Object?>();
  origin['generator_version'] = 2;
  final modulePayload = (module['payload']! as Map).cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  moduleData['generator_version'] = 2;
  moduleData['input_fingerprint'] = revision3QuestInputFingerprint(input);
  return jsonEncode(project);
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

Map<String, Object?> _generatedOrigin({
  required String ownerId,
  required int generatorVersion,
}) => <String, Object?>{
  'type': 'generated',
  'generator_id': 'gore-authoring.draft-quest-skeleton',
  'generator_version': generatorVersion,
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
