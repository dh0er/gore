import 'dart:collection';
import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';

import 'revision3_voice_fixture.dart';
import 'revision3_voice_selection_fixture.dart';

const revision3VoiceRemovalSharedLineId = '00000000000000000000000000002001';
const revision3VoiceRemovalSharedSlotId = '00000000000000000000000000002002';

final class Revision3VoiceRemovalFixture {
  Revision3VoiceRemovalFixture({
    this.candidateCount = 2,
    String? takeId,
    this.shared = false,
  }) : takeId = takeId ?? revision3VoiceFixtureTakeId,
       head = revision3VoiceSelectionManifestHead('d', byteLength: 821),
       candidateHead = revision3VoiceSelectionManifestHead(
         'e',
         byteLength: 977,
       ) {
    var project = revision3VoiceFixtureProjectWithExistingSlotJson(
      candidateCount: candidateCount,
    );
    if (shared) project = _withSharedTake(project, this.takeId);
    projectJson = project;
  }

  final int candidateCount;
  final String takeId;
  final bool shared;
  late final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringWorkingHead candidateHead;

  int get projectRevision =>
      (jsonDecode(projectJson) as Map<String, Object?>)['revision']! as int;

  int get slotRevision =>
      _entity(takeProject, revision3VoiceFixtureSlotId)['revision']! as int;

  int get takeRevision => _entity(takeProject, takeId)['revision']! as int;

  Map<String, Object?> get takeProject =>
      (jsonDecode(projectJson) as Map).cast<String, Object?>();

  String? get previousSelectedTakeId => revision3VoiceFixtureTakeId;

  bool get selectionCleared => previousSelectedTakeId == takeId;

  bool get takeEntityRemoved => !shared;

  int get remainingCandidateCount => candidateCount - 1;

  AuthoringRevision3VoiceTakeRemovalRequestV1 request() =>
      AuthoringRevision3VoiceTakeRemovalRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: projectJson,
        lineId: revision3VoiceFixtureLineId,
        localizationId: revision3VoiceFixtureLocalizationId,
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        locale: 'de',
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: slotRevision,
        takeId: takeId,
        expectedTakeRevision: takeRevision,
        expectedSelectedTakeId: previousSelectedTakeId,
      );

  String candidateProjectJson() {
    final project = takeProject;
    project['revision'] = projectRevision + 1;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final slot = _entity(project, revision3VoiceFixtureSlotId);
    slot['revision'] = slotRevision + 1;
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    final candidates = (data['candidates']! as List)
        .cast<Map<String, Object?>>()
        .where((candidate) => candidate['id'] != takeId)
        .map<Object?>((candidate) => candidate)
        .toList(growable: false);
    data['candidates'] = candidates;
    if (selectionCleared) data.remove('selected');
    payload['data'] = data;
    slot['payload'] = payload;
    entities[revision3VoiceFixtureSlotId] = slot;
    if (takeEntityRemoved) entities.remove(takeId);
    project['entities'] = SplayTreeMap<String, Object?>.from(entities);
    return jsonEncode(project);
  }

  Map<String, Object?> response() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': head.canonicalJson,
    'head_json': candidateHead.canonicalJson,
    'project_json': candidateProjectJson(),
    'project_id': revision3VoiceFixtureProjectId,
    'revision': projectRevision + 1,
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': revision3VoiceFixtureSlotId,
    'slot_revision': slotRevision + 1,
    'locale': 'de',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'take_id': takeId,
    'take_revision': takeRevision,
    'previous_selected_take_id': previousSelectedTakeId,
    'selection_cleared': selectionCleared,
    'take_entity_removed': takeEntityRemoved,
    'remaining_candidate_count': remainingCandidateCount,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}

Map<String, Object?> _entity(Map<String, Object?> project, String id) =>
    ((project['entities']! as Map)[id]! as Map).cast<String, Object?>();

String _withSharedTake(String projectJson, String takeId) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final entities = SplayTreeMap<String, Object?>.from(
    (project['entities']! as Map).cast<String, Object?>(),
  );
  final line =
      jsonDecode(jsonEncode(entities[revision3VoiceFixtureLineId]))
          as Map<String, Object?>;
  line['id'] = revision3VoiceRemovalSharedLineId;
  line['display_name'] = 'Shared Asghan greeting';
  final linePayload = (line['payload']! as Map).cast<String, Object?>();
  final lineData = (linePayload['data']! as Map).cast<String, Object?>();
  lineData['voice_slots'] = <String, Object?>{
    'de': _typedRef(revision3VoiceRemovalSharedSlotId, 'voice_slot'),
  };
  linePayload['data'] = lineData;
  line['payload'] = linePayload;
  entities[revision3VoiceRemovalSharedLineId] = line;

  final sourceSlot =
      jsonDecode(jsonEncode(entities[revision3VoiceFixtureSlotId]))
          as Map<String, Object?>;
  sourceSlot['id'] = revision3VoiceRemovalSharedSlotId;
  sourceSlot['display_name'] = 'Shared German Voice slot';
  final origin = (sourceSlot['origin']! as Map).cast<String, Object?>();
  origin['owner'] = _typedRef(revision3VoiceRemovalSharedLineId, 'dialog_line');
  sourceSlot['origin'] = origin;
  final payload = (sourceSlot['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['candidates'] = <Object?>[_typedRef(takeId, 'voice_take')];
  data.remove('selected');
  payload['data'] = data;
  sourceSlot['payload'] = payload;
  entities[revision3VoiceRemovalSharedSlotId] = sourceSlot;
  project['entities'] = entities;
  return jsonEncode(project);
}

Map<String, Object?> _typedRef(String id, String kind) => <String, Object?>{
  'project_id': revision3VoiceFixtureProjectId,
  'id': id,
  'expected_kind': kind,
};
