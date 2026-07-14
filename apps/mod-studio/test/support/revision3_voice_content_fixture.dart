import 'package:gore_mod/project/revision3_content_index.dart';

const revision3VoiceContentProjectId = '11111111111111111111111111111111';
const revision3VoiceContentLocalizationId = '22222222222222222222222222222222';
const revision3VoiceContentLineId = '33333333333333333333333333333333';
const revision3VoiceContentSlotId = '44444444444444444444444444444444';
const revision3VoiceContentDuplicateLineId = '66666666666666666666666666666666';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

Revision3ContentIndex revision3VoiceContentIndexFixture({
  int revision = 7,
  bool existingDeSlot = true,
  int existingSlotCandidateCount = 0,
  bool existingSlotHasSelectedTake = false,
  String existingSlotTargetResolution = 'unresolved',
  bool omitExistingSlotCandidateCount = false,
  bool duplicateLine = false,
  Iterable<String> extraEntityIds = const <String>[],
  String lineDisplayName = 'Mine entrance question',
  String speaker = 'Asghan',
}) => Revision3ContentIndex.fromJsonObject(
  revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingDeSlot,
    existingSlotCandidateCount: existingSlotCandidateCount,
    existingSlotHasSelectedTake: existingSlotHasSelectedTake,
    existingSlotTargetResolution: existingSlotTargetResolution,
    omitExistingSlotCandidateCount: omitExistingSlotCandidateCount,
    duplicateLine: duplicateLine,
    extraEntityIds: extraEntityIds,
    lineDisplayName: lineDisplayName,
    speaker: speaker,
  ),
);

Map<String, Object?> revision3VoiceContentIndexJsonFixture({
  int revision = 7,
  bool existingDeSlot = true,
  int existingSlotCandidateCount = 0,
  bool existingSlotHasSelectedTake = false,
  String existingSlotTargetResolution = 'unresolved',
  bool omitExistingSlotCandidateCount = false,
  bool duplicateLine = false,
  Iterable<String> extraEntityIds = const <String>[],
  String lineDisplayName = 'Mine entrance question',
  String speaker = 'Asghan',
}) {
  final extras = extraEntityIds.toSet();
  final takeCount = existingDeSlot
      ? (existingSlotHasSelectedTake && existingSlotCandidateCount == 0
            ? 1
            : existingSlotCandidateCount)
      : 0;
  final takeIds = <String>[
    for (var index = 0; index < takeCount; index++) _candidateId(index),
  ];
  final entities =
      <Map<String, Object?>>[
        _localization(
          revision3VoiceContentLocalizationId,
          locId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        ),
        <String, Object?>{
          'id': revision3VoiceContentLineId,
          'kind': 'dialog_line',
          'display_name': lineDisplayName,
          'revision': 2,
          'origin': <String, Object?>{
            'type': 'new',
            'authored_runtime_id': 'fixture-dialog-line',
          },
          'summary': <String, Object?>{
            'kind': 'dialog_line',
            'data': <String, Object?>{
              'speaker_hint': speaker,
              'voice_slot_locales': <Object?>[if (existingDeSlot) 'de'],
            },
          },
          'references': <Object?>[
            _entityReference(
              role: 'dialog_localization',
              entityId: revision3VoiceContentLocalizationId,
              expectedKind: 'localization_entry',
            ),
            if (existingDeSlot)
              _entityReference(
                role: 'dialog_voice_slot',
                qualifier: 'de',
                entityId: revision3VoiceContentSlotId,
                expectedKind: 'voice_slot',
              ),
          ],
          'asset_references': <Object?>[],
        },
        if (existingDeSlot)
          <String, Object?>{
            'id': revision3VoiceContentSlotId,
            'kind': 'voice_slot',
            'display_name': 'Asghan German Voice',
            'revision': 1,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'fixture-voice-slot',
            },
            'summary': <String, Object?>{
              'kind': 'voice_slot',
              'data': <String, Object?>{
                'locale': 'de',
                'target_resolution': existingSlotTargetResolution,
                if (!omitExistingSlotCandidateCount)
                  'candidate_count': existingSlotCandidateCount,
                'has_selected_take': existingSlotHasSelectedTake,
              },
            },
            'references': <Object?>[
              for (final id in takeIds.take(existingSlotCandidateCount))
                _entityReference(
                  role: 'voice_candidate',
                  entityId: id,
                  expectedKind: 'voice_take',
                ),
              if (existingSlotHasSelectedTake)
                _entityReference(
                  role: 'voice_selected',
                  entityId: takeIds.first,
                  expectedKind: 'voice_take',
                ),
            ],
            'asset_references': <Object?>[],
          },
        for (final id in takeIds)
          _voiceTake(
            id,
            status: existingSlotHasSelectedTake && id == takeIds.first
                ? 'approved'
                : 'recorded',
          ),
        if (duplicateLine)
          <String, Object?>{
            'id': revision3VoiceContentDuplicateLineId,
            'kind': 'dialog_line',
            'display_name': lineDisplayName,
            'revision': 0,
            'origin': <String, Object?>{
              'type': 'new',
              'authored_runtime_id': 'fixture-duplicate-dialog-line',
            },
            'summary': <String, Object?>{
              'kind': 'dialog_line',
              'data': <String, Object?>{
                'speaker_hint': speaker,
                'voice_slot_locales': <Object?>[],
              },
            },
            'references': <Object?>[
              _entityReference(
                role: 'dialog_localization',
                entityId: revision3VoiceContentLocalizationId,
                expectedKind: 'localization_entry',
              ),
            ],
            'asset_references': <Object?>[],
          },
        for (final id in extras) _localization(id, locId: 'EXTRA_$id'),
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );

  return <String, Object?>{
    'schema_revision': 1,
    'project_id': revision3VoiceContentProjectId,
    'project_revision': revision,
    'project_name': 'Voice fixture',
    'project_version': '0.1.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256': _targetSha,
      },
    },
    'authoring_locales': <Object?>['de', 'en'],
    'entity_counts': <String, Object?>{
      'localization_entry': 1 + extras.length,
      'dialog_line': duplicateLine ? 2 : 1,
      if (existingDeSlot) 'voice_slot': 1,
      if (takeIds.isNotEmpty) 'voice_take': takeIds.length,
    },
    'entities': entities,
    'assets': <Object?>[],
  };
}

String _candidateId(int index) =>
    '55${index.toRadixString(16).padLeft(30, '0')}';

Map<String, Object?> _entityReference({
  required String role,
  String? qualifier,
  required String entityId,
  required String expectedKind,
  String resolution = 'resolved',
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': entityId,
    'expected_kind': expectedKind,
  },
  'resolution': resolution,
};

Map<String, Object?> _voiceTake(String id, {required String status}) =>
    <String, Object?>{
      'id': id,
      'kind': 'voice_take',
      'display_name': 'Asghan take',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'fixture-take-$id',
      },
      'summary': <String, Object?>{
        'kind': 'voice_take',
        'data': <String, Object?>{
          'locale': 'de',
          'status': status,
          'codec': 'vorbis',
          'channels': 1,
          'sample_rate': 48000,
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    };

Map<String, Object?> _localization(String id, {required String locId}) =>
    <String, Object?>{
      'id': id,
      'kind': 'localization_entry',
      'display_name': locId,
      'revision': 0,
      'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': locId},
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{'loc_id': locId, 'locales': <Object?>[]},
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    };
