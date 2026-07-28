import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

import 'revision3_dataasset_fixture.dart';
import 'revision3_voice_content_fixture.dart';

const revision3ProjectProblemsNpcId = '77777777777777777777777777777777';
const revision3ProjectProblemsMissingModuleId =
    '88888888888888888888888888888888';
const revision3ProjectProblemsMissingAudioSha256 =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

/// A closed fixture for the project Problems surface.
///
/// [contentIndex] and every [dataAssetStages] entry always describe the same
/// exact project ID and published revision. Tests can therefore exercise
/// source availability independently without accidentally testing the view's
/// fail-closed checkpoint mismatch handling.
final class Revision3ProjectProblemsFixture {
  const Revision3ProjectProblemsFixture._({
    required this.contentIndex,
    required this.dataAssetStages,
  });

  final Revision3ContentIndex contentIndex;
  final List<AuthoringRevision3DataAssetStage> dataAssetStages;

  String get projectId => contentIndex.projectId;
  int get projectRevision => contentIndex.projectRevision;

  AuthoringRevision3DataAssetStage? get dataAssetStage =>
      dataAssetStages.isEmpty ? null : dataAssetStages.single;

  String? get dataAssetManifestSha256 => dataAssetStage?.manifestAsset.sha256;
}

/// A no-diagnostic project used to verify the honest empty boundary.
Revision3ProjectProblemsFixture revision3ProjectProblemsEmptyFixture({
  int projectRevision = 7,
}) => _fixture(
  projectRevision: projectRevision,
  includeDialogGraph: false,
  includeReferenceProblem: false,
  includeAssetProblem: false,
  includeVoiceProblems: false,
  includeDataAssetStage: false,
);

/// A healthy dialog graph with no unresolved references or staged DataAsset.
///
/// This differs from [revision3ProjectProblemsEmptyFixture] by retaining
/// realistic, searchable project content while still yielding no diagnostic.
Revision3ProjectProblemsFixture revision3ProjectProblemsCleanFixture({
  int projectRevision = 7,
}) => _fixture(
  projectRevision: projectRevision,
  includeDialogGraph: true,
  includeReferenceProblem: false,
  includeAssetProblem: false,
  includeVoiceProblems: false,
  includeDataAssetStage: false,
);

/// A multi-category report fixture for list, filter, search, and callback tests.
///
/// It contains:
///
/// * a VoiceSlot with one exact missing VoiceTake candidate;
/// * one missing Voice audio asset reference;
/// * one parsed blocked DataAsset stage at the same project revision.
Revision3ProjectProblemsFixture revision3ProjectProblemsFilterFixture({
  int projectRevision = 7,
}) => _fixture(
  projectRevision: projectRevision,
  includeDialogGraph: true,
  includeReferenceProblem: true,
  includeAssetProblem: true,
  includeVoiceProblems: true,
  includeDataAssetStage: true,
);

/// Configurable exact fixture for focused source and partial-load tests.
Revision3ProjectProblemsFixture revision3ProjectProblemsFixture({
  int projectRevision = 7,
  bool includeDialogGraph = true,
  bool includeReferenceProblem = true,
  bool includeAssetProblem = true,
  bool includeVoiceProblems = true,
  bool includeDataAssetStage = true,
}) {
  if (!includeDialogGraph &&
      (includeReferenceProblem ||
          includeAssetProblem ||
          includeVoiceProblems)) {
    throw ArgumentError(
      'Voice reference and asset problems require the dialog graph fixture.',
    );
  }
  return _fixture(
    projectRevision: projectRevision,
    includeDialogGraph: includeDialogGraph,
    includeReferenceProblem: includeReferenceProblem,
    includeAssetProblem: includeAssetProblem,
    includeVoiceProblems: includeVoiceProblems,
    includeDataAssetStage: includeDataAssetStage,
  );
}

Revision3ProjectProblemsFixture _fixture({
  required int projectRevision,
  required bool includeDialogGraph,
  required bool includeReferenceProblem,
  required bool includeAssetProblem,
  required bool includeVoiceProblems,
  required bool includeDataAssetStage,
}) {
  if (projectRevision < 1) {
    throw ArgumentError.value(
      projectRevision,
      'projectRevision',
      'must leave a non-negative basis revision for a matching DataAsset stage',
    );
  }

  final contentIndex = _contentIndex(
    projectRevision: projectRevision,
    includeDialogGraph: includeDialogGraph,
    includeReferenceProblem: includeReferenceProblem,
    includeAssetProblem: includeAssetProblem,
    includeVoiceProblems: includeVoiceProblems,
  );
  final stages = includeDataAssetStage
      ? _matchingDataAssetStages(contentIndex)
      : const <AuthoringRevision3DataAssetStage>[];
  return Revision3ProjectProblemsFixture._(
    contentIndex: contentIndex,
    dataAssetStages: List<AuthoringRevision3DataAssetStage>.unmodifiable(
      stages,
    ),
  );
}

Revision3ContentIndex _contentIndex({
  required int projectRevision,
  required bool includeDialogGraph,
  required bool includeReferenceProblem,
  required bool includeAssetProblem,
  required bool includeVoiceProblems,
}) {
  final json = includeDialogGraph
      ? revision3VoiceContentIndexJsonFixture(
          revision: projectRevision,
          existingDeSlot: true,
          existingSlotCandidateCount: 1,
          existingSlotHasSelectedTake:
              !includeVoiceProblems && !includeReferenceProblem,
          existingSlotTargetResolution:
              includeVoiceProblems || includeReferenceProblem
              ? 'unresolved'
              : 'resolved',
          lineDisplayName: 'Mine entrance question',
          speaker: 'Asghan',
        )
      : _emptyContentJson(projectRevision);

  final entities = (json['entities']! as List).cast<Map<String, Object?>>();

  if (includeReferenceProblem) {
    _addVoiceCandidateReferenceProblem(entities);
  }

  if (includeAssetProblem) {
    final take = entities.singleWhere(
      (entity) => entity['kind'] == 'voice_take',
    );
    take['asset_references'] = <Object?>[
      <String, Object?>{
        'role': 'voice_audio',
        'sha256': revision3ProjectProblemsMissingAudioSha256,
        'byte_len': 4096,
        'logical_name': 'asghan_mine_entrance_problem_fixture.ogg',
        'expected_media_type': 'audio/ogg',
        'resolution': 'missing_asset',
      },
    ];
  }

  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  return Revision3ContentIndex.fromJsonObject(json);
}

Map<String, Object?> _emptyContentJson(
  int projectRevision,
) => <String, Object?>{
  'schema_revision': 1,
  'project_id': revision3VoiceContentProjectId,
  'project_revision': projectRevision,
  'project_name': 'Problems fixture',
  'project_version': '0.1.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256':
          'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    },
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
};

void _addVoiceCandidateReferenceProblem(List<Map<String, Object?>> entities) {
  final line = entities.singleWhere(
    (entity) => entity['kind'] == 'dialog_line',
  );
  final slotReference = (line['references']! as List)
      .cast<Map<String, Object?>>()
      .singleWhere((reference) => reference['role'] == 'dialog_voice_slot');
  (slotReference['target']! as Map<String, Object?>)['entity_id'] =
      revision3ProjectProblemsNpcId;

  final slot = entities.singleWhere((entity) => entity['kind'] == 'voice_slot');
  slot['id'] = revision3ProjectProblemsNpcId;
  slot['display_name'] = 'Gate Guard';
  final candidate = (slot['references']! as List)
      .cast<Map<String, Object?>>()
      .singleWhere((reference) => reference['role'] == 'voice_candidate');
  (candidate['target']! as Map<String, Object?>)['entity_id'] =
      revision3ProjectProblemsMissingModuleId;
  candidate['resolution'] = 'missing_entity';
}

List<AuthoringRevision3DataAssetStage> _matchingDataAssetStages(
  Revision3ContentIndex contentIndex,
) {
  final basisProjectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': contentIndex.projectId,
    'revision': contentIndex.projectRevision - 1,
    'meta': <String, Object?>{
      'name': contentIndex.projectName,
      'version': contentIndex.projectVersion,
      'author': contentIndex.projectAuthor,
    },
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': contentIndex.targetExecutableByteLength,
        'sha256': contentIndex.targetExecutableSha256,
      },
    },
    'authoring_locales': contentIndex.authoringLocales,
    'entities': <String, Object?>{},
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
  final basisHead = revision3DataAssetHeadForProject(basisProjectJson);
  final fixture = Revision3DataAssetFixture.fromBasis(
    basisHead: basisHead,
    basisProjectJson: basisProjectJson,
  );
  final parsed = AuthoringRevision3DataAssetStageListResult.fromJson(
    fixture.listResponse(),
    expectedHead: fixture.stagedHead,
  );
  if (parsed.revision != contentIndex.projectRevision ||
      parsed.stages.any(
        (stage) =>
            stage.projectId != contentIndex.projectId ||
            stage.stagedProjectRevision != contentIndex.projectRevision,
      )) {
    throw StateError(
      'Problems fixture produced a cross-project DataAsset stage.',
    );
  }
  return parsed.stages;
}
