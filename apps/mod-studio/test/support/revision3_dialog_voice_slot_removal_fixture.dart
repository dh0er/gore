import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';

import 'revision3_voice_fixture.dart';
import 'revision3_voice_selection_fixture.dart';

const revision3DialogVoiceSlotRemovalSlotId =
    '00000000000000000000000000100000';

final class Revision3DialogVoiceSlotRemovalFixture {
  Revision3DialogVoiceSlotRemovalFixture()
    : projectJson = revision3VoiceFixtureProjectWithVoiceSlotCountJson(
        1,
        generatedSlots: true,
      ),
      head = revision3VoiceSelectionManifestHead('7', byteLength: 701),
      candidateHead = revision3VoiceSelectionManifestHead('8', byteLength: 702);

  final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringWorkingHead candidateHead;

  Map<String, Object?> get project =>
      (jsonDecode(projectJson) as Map).cast<String, Object?>();

  int get projectRevision => project['revision']! as int;

  int get lineRevision =>
      _entity(revision3VoiceFixtureLineId)['revision']! as int;

  int get slotRevision =>
      _entity(revision3DialogVoiceSlotRemovalSlotId)['revision']! as int;

  Map<String, Object?> _entity(String id) =>
      ((project['entities']! as Map)[id]! as Map).cast<String, Object?>();

  AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request() =>
      AuthoringRevision3DialogVoiceSlotRemovalRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: projectJson,
        lineId: revision3VoiceFixtureLineId,
        expectedLineRevision: lineRevision,
        localizationId: revision3VoiceFixtureLocalizationId,
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        locale: 'de',
        slotId: revision3DialogVoiceSlotRemovalSlotId,
        expectedSlotRevision: slotRevision,
      );

  String candidateProjectJson() {
    final candidate = project;
    candidate['revision'] = projectRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final line = (entities[revision3VoiceFixtureLineId]! as Map)
        .cast<String, Object?>();
    line['revision'] = lineRevision + 1;
    final payload = (line['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    final slots = (data['voice_slots']! as Map).cast<String, Object?>();
    slots.remove('de');
    data['voice_slots'] = slots;
    payload['data'] = data;
    line['payload'] = payload;
    entities[revision3VoiceFixtureLineId] = line;
    entities.remove(revision3DialogVoiceSlotRemovalSlotId);
    candidate['entities'] = entities;
    return jsonEncode(candidate);
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
    'line_revision': lineRevision + 1,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': revision3DialogVoiceSlotRemovalSlotId,
    'removed_slot_revision': slotRevision,
    'locale': 'de',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'removed_target_resolution': 'unresolved',
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}
