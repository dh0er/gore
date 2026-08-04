import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';

import 'revision3_voice_fixture.dart';

const revision3VoiceSelectionAlternateTakeId =
    '00000000000000000000000000001001';

AuthoringWorkingHead revision3VoiceSelectionManifestHead(
  String hex, {
  required int byteLength,
}) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, hex).join(),
    },
  }),
);

final class Revision3VoiceSelectionFixture {
  Revision3VoiceSelectionFixture({this.clear = false})
    : projectJson = revision3VoiceFixtureProjectWithExistingSlotJson(
        candidateCount: 2,
      ),
      head = revision3VoiceSelectionManifestHead('b', byteLength: 733),
      candidateHead = revision3VoiceSelectionManifestHead('c', byteLength: 911);

  final bool clear;
  final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringWorkingHead candidateHead;

  String get previousSelectedTakeId => revision3VoiceFixtureTakeId;

  String? get selectedTakeId =>
      clear ? null : revision3VoiceSelectionAlternateTakeId;

  int get projectRevision =>
      (jsonDecode(projectJson) as Map<String, Object?>)['revision']! as int;

  int get slotRevision {
    final project = jsonDecode(projectJson) as Map<String, Object?>;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
        .cast<String, Object?>();
    return slot['revision']! as int;
  }

  AuthoringRevision3VoiceTakeSelectionRequestV1 request() =>
      AuthoringRevision3VoiceTakeSelectionRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: projectJson,
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: previousSelectedTakeId,
        selectedTakeId: selectedTakeId,
      );

  String candidateProjectJson() {
    final project = jsonDecode(projectJson) as Map<String, Object?>;
    project['revision'] = projectRevision + 1;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
        .cast<String, Object?>();
    slot['revision'] = slotRevision + 1;
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    final selected = selectedTakeId;
    if (selected == null) {
      data.remove('selected');
    } else {
      data['selected'] = <String, Object?>{
        'project_id': revision3VoiceFixtureProjectId,
        'id': selected,
        'expected_kind': 'voice_take',
      };
    }
    payload['data'] = data;
    slot['payload'] = payload;
    entities[revision3VoiceFixtureSlotId] = slot;
    project['entities'] = entities;
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
    'slot_id': revision3VoiceFixtureSlotId,
    'slot_revision': slotRevision + 1,
    'locale': 'de',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'previous_selected_take_id': previousSelectedTakeId,
    'selected_take_id': selectedTakeId,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}
