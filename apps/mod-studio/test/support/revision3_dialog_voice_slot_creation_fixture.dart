import 'dart:convert';

import 'package:gore_mod/core/mod_ffi.dart';

import 'revision3_voice_fixture.dart';
import 'revision3_voice_selection_fixture.dart';

const revision3DialogVoiceSlotCreationSlotId =
    '00000000000000000000000000100000';

final class Revision3DialogVoiceSlotCreationFixture {
  Revision3DialogVoiceSlotCreationFixture()
    : projectJson = _basisProjectJson(),
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

  int get localizationRevision =>
      _entity(revision3VoiceFixtureLocalizationId)['revision']! as int;

  Map<String, Object?> _entity(String id) =>
      ((project['entities']! as Map)[id]! as Map).cast<String, Object?>();

  AuthoringRevision3DialogVoiceSlotCreationRequestV1 request() =>
      AuthoringRevision3DialogVoiceSlotCreationRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: projectJson,
        lineId: revision3VoiceFixtureLineId,
        expectedLineRevision: lineRevision,
        localizationId: revision3VoiceFixtureLocalizationId,
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        locale: 'de',
        slotId: revision3DialogVoiceSlotCreationSlotId,
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
    slots['de'] = <String, Object?>{
      'project_id': revision3VoiceFixtureProjectId,
      'id': revision3DialogVoiceSlotCreationSlotId,
      'expected_kind': 'voice_slot',
    };
    data['voice_slots'] = slots;
    payload['data'] = data;
    line['payload'] = payload;
    entities[revision3VoiceFixtureLineId] = line;
    entities[revision3DialogVoiceSlotCreationSlotId] = <String, Object?>{
      'id': revision3DialogVoiceSlotCreationSlotId,
      'display_name': 'Voice de',
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.voice-slot',
        'generator_version': 1,
        'owner': <String, Object?>{
          'project_id': revision3VoiceFixtureProjectId,
          'id': revision3VoiceFixtureLineId,
          'expected_kind': 'dialog_line',
        },
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': 'de',
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[],
        },
      },
    };
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
    'localization_revision': localizationRevision,
    'slot_id': revision3DialogVoiceSlotCreationSlotId,
    'slot_revision': 0,
    'locale': 'de',
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'target_resolution': 'unresolved',
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

String _basisProjectJson() {
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  project['authoring_locales'] = <Object?>['de'];
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
      .cast<String, Object?>();
  final payload = (localization['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['texts'] = <String, Object?>{'de': 'Niemand betritt die Mine.'};
  payload['data'] = data;
  localization['payload'] = payload;
  entities[revision3VoiceFixtureLocalizationId] = localization;
  project['entities'] = entities;
  return jsonEncode(project);
}
