import 'dart:collection';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _command = 'authoring_store_prepare_revision3_dialog_line_v1';
const _projectId = '11111111111111111111111111111111';
const _lineId = '22222222222222222222222222222222';
const _localizationId = '33333333333333333333333333333333';
const _slotId = '44444444444444444444444444444444';
const _existingLocalizationId = '55555555555555555555555555555555';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

String _headJson(String byte) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 321,
    'sha256': List<String>.filled(64, byte).join(),
  },
});

String _projectJson({int revision = 7, String? existingGermanText}) {
  final entities = SplayTreeMap<String, Object?>();
  if (existingGermanText != null) {
    entities[_existingLocalizationId] = <String, Object?>{
      'id': _existingLocalizationId,
      'display_name': 'Existing managed text',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_EXISTING_TEXT',
      },
      'revision': 4,
      'payload': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': 'GORE_EXISTING_TEXT',
          'texts': SplayTreeMap<String, Object?>.from(<String, Object?>{
            'de': existingGermanText,
            'en': 'Existing text.',
          }),
        },
      },
    };
  }
  return jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': _projectId,
    'revision': revision,
    'meta': <String, Object?>{
      'name': 'Dialog-line core fixture',
      'version': '1.0.0',
      'author': 'tests',
    },
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256': _targetSha,
      },
    },
    'authoring_locales': <Object?>[],
    'entities': entities,
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
}

AuthoringRevision3DialogLineEntryRequestV1 _createRequest({
  required String projectJson,
  String? speakerHint,
  bool withSlot = true,
}) => AuthoringRevision3DialogLineEntryRequestV1.forProject(
  expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('a')),
  currentProjectJson: projectJson,
  lineId: _lineId,
  lineDisplayName: 'Asghan warning',
  lineAuthoredIdentity: 'GORE_DIALOG_ASGHAN_WARNING',
  speakerHint: speakerHint,
  localization: AuthoringRevision3DialogLocalizationCreateIntentV1(
    localizationId: _localizationId,
    displayName: 'Asghan warning text',
    locId: 'GORE_ASGHAN_WARNING',
    texts: const <String, String>{'de': 'Bleib stehen!'},
  ),
  voiceSlot: withSlot
      ? AuthoringRevision3DialogEmptyVoiceSlotIntentV1(
          slotId: _slotId,
          locale: 'de',
          displayName: 'Asghan warning German Voice',
        )
      : null,
);

AuthoringRevision3DialogLineEntryRequestV1 _reuseRequest({
  required String projectJson,
  bool withSlot = true,
}) => AuthoringRevision3DialogLineEntryRequestV1.forProject(
  expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('a')),
  currentProjectJson: projectJson,
  lineId: _lineId,
  lineDisplayName: 'Reuse existing text',
  lineAuthoredIdentity: 'GORE_DIALOG_REUSE_EXISTING',
  speakerHint: null,
  localization: AuthoringRevision3DialogLocalizationReuseExactIntentV1(
    localizationId: _existingLocalizationId,
    expectedLocalizationRevision: 4,
    expectedLocId: 'GORE_EXISTING_TEXT',
  ),
  voiceSlot: withSlot
      ? AuthoringRevision3DialogEmptyVoiceSlotIntentV1(
          slotId: _slotId,
          locale: 'de',
          displayName: 'Existing text German Voice',
        )
      : null,
);

Map<String, Object?> _typedRef(String id, String kind) => <String, Object?>{
  'project_id': _projectId,
  'id': id,
  'expected_kind': kind,
};

String _candidateProjectJson({
  required String basisProjectJson,
  required AuthoringRevision3DialogLineEntryRequestV1 request,
}) {
  final candidate = (jsonDecode(basisProjectJson) as Map)
      .cast<String, Object?>();
  candidate['revision'] = request.expectedRevision + 1;
  final entities = SplayTreeMap<String, Object?>.from(
    (candidate['entities']! as Map).cast<String, Object?>(),
  );
  final locales = (candidate['authoring_locales']! as List)
      .cast<String>()
      .toSet();

  final localization = request.localization;
  if (localization is AuthoringRevision3DialogLocalizationCreateIntentV1) {
    locales.addAll(localization.texts.keys);
    entities[localization.localizationId] = <String, Object?>{
      'id': localization.localizationId,
      'display_name': localization.displayName,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': localization.locId,
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': localization.locId,
          'texts': SplayTreeMap<String, Object?>.from(localization.texts),
        },
      },
    };
  } else if (localization
      is AuthoringRevision3DialogLocalizationReuseExactIntentV1) {
    final existing = (entities[localization.localizationId]! as Map)
        .cast<String, Object?>();
    final payload = (existing['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    locales.addAll((data['texts']! as Map).keys.cast<String>());
  } else {
    throw StateError('unsupported dialog localization fixture');
  }

  final voiceSlots = SplayTreeMap<String, Object?>();
  final slot = request.voiceSlot;
  if (slot != null) {
    locales.add(slot.locale);
    voiceSlots[slot.locale] = _typedRef(slot.slotId, 'voice_slot');
    entities[slot.slotId] = <String, Object?>{
      'id': slot.slotId,
      'display_name': slot.displayName,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.voice-slot',
        'generator_version': 1,
        'owner': _typedRef(request.lineId, 'dialog_line'),
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': slot.locale,
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[],
        },
      },
    };
  }
  entities[request.lineId] = <String, Object?>{
    'id': request.lineId,
    'display_name': request.lineDisplayName,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': request.lineAuthoredIdentity,
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'dialog_line',
      'data': <String, Object?>{
        'localization': _typedRef(
          request.localization.localizationId,
          'localization_entry',
        ),
        if (request.speakerHint != null) 'speaker_hint': request.speakerHint,
        'voice_slots': voiceSlots,
      },
    },
  };

  final sortedLocales = locales.toList(growable: false)..sort();
  candidate['authoring_locales'] = sortedLocales;
  candidate['entities'] = entities;
  return jsonEncode(candidate);
}

Map<String, Object?> _response({
  required String basisProjectJson,
  required AuthoringRevision3DialogLineEntryRequestV1 request,
  String? candidateProjectJson,
}) {
  final action =
      request.localization is AuthoringRevision3DialogLocalizationCreateIntentV1
      ? 'created'
      : 'reused_exact';
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': request.expectedHead.canonicalJson,
    'head_json': _headJson('b'),
    'project_json':
        candidateProjectJson ??
        _candidateProjectJson(
          basisProjectJson: basisProjectJson,
          request: request,
        ),
    'project_id': request.expectedProjectId,
    'revision': request.expectedRevision + 1,
    'line_id': request.lineId,
    'localization_id': request.localization.localizationId,
    'localization_action': action,
    'voice_slot_id': request.voiceSlot?.slotId,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'topic_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

Future<AuthoringRevision3DialogLineEntryPreparation> _call({
  required String basisProjectJson,
  required AuthoringRevision3DialogLineEntryRequestV1 request,
  required Map<String, Object?> response,
  void Function(FakeGoreCoreFfiService core)? inspectCore,
}) async {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_command: response},
  );
  final result = await ModFfi(core).authoringStorePrepareRevision3DialogLineV1(
    root: r'C:\Mods\Dialog.goreproj',
    currentProjectJson: basisProjectJson,
    request: request,
  );
  inspectCore?.call(core);
  return result;
}

Matcher get _throwsMalformed => throwsA(
  isA<ModFfiException>().having(
    (error) => error.code,
    'code',
    ModFfiException.malformedNativeResponseCode,
  ),
);

void main() {
  test('request wire omits absent speaker and VoiceSlot canonically', () {
    final projectJson = _projectJson();
    final request = _createRequest(
      projectJson: projectJson,
      speakerHint: null,
      withSlot: false,
    );
    final decoded = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();

    expect(decoded.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'line_display_name',
      'line_authored_identity',
      'localization',
    ]);
    expect(decoded, isNot(contains('speaker_hint')));
    expect(decoded, isNot(contains('voice_slot')));
    expect((decoded['localization']! as Map).keys, <String>[
      'mode',
      'localization_id',
      'display_name',
      'loc_id',
      'texts',
    ]);
    expect(
      AuthoringRevision3DialogLineEntryRequestV1.fromCanonicalJson(
        request.canonicalJson,
        currentProjectJson: projectJson,
      ).canonicalJson,
      request.canonicalJson,
    );
    expect(requiredStudioCoreCommands, contains(_command));
  });

  test(
    'Create prepares only the exact blocked unpublished candidate',
    () async {
      final projectJson = _projectJson();
      final request = _createRequest(
        projectJson: projectJson,
        speakerHint: 'Asghan',
      );
      final response = _response(
        basisProjectJson: projectJson,
        request: request,
      );

      final result = await _call(
        basisProjectJson: projectJson,
        request: request,
        response: response,
        inspectCore: (core) {
          expect(core.calls.single.command, _command);
          expect(core.calls.single.payload.keys, <String>[
            'current_project_json',
            'dialog_line_request_json',
            'root',
          ]);
          expect(core.calls.single.payload, isNot(contains('game_root')));
        },
      );

      expect(
        result.localizationAction,
        AuthoringRevision3DialogLocalizationAction.created,
      );
      expect(result.lineId, _lineId);
      expect(result.localizationId, _localizationId);
      expect(result.voiceSlotId, _slotId);
      expect(result.buildStatus, AuthoringRevision3DialogBuildStatus.blocked);
      expect(
        result.runtimeStatus,
        AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified,
      );
      expect(
        result.topicAuthority,
        AuthoringRevision3DialogTopicAuthority.notGranted,
      );
      expect(
        result.publicationStatus,
        AuthoringRevision3DialogPublicationStatus.notSupported,
      );
    },
  );

  test('ReuseExact preserves the localization entity byte-for-byte', () async {
    final projectJson = _projectJson(existingGermanText: 'Vorhanden.');
    final request = _reuseRequest(projectJson: projectJson);
    final candidateJson = _candidateProjectJson(
      basisProjectJson: projectJson,
      request: request,
    );
    final result = await _call(
      basisProjectJson: projectJson,
      request: request,
      response: _response(
        basisProjectJson: projectJson,
        request: request,
        candidateProjectJson: candidateJson,
      ),
    );

    final basis = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    final candidate = (jsonDecode(candidateJson) as Map)
        .cast<String, Object?>();
    final basisEntities = (basis['entities']! as Map).cast<String, Object?>();
    final candidateEntities = (candidate['entities']! as Map)
        .cast<String, Object?>();
    expect(
      jsonEncode(candidateEntities[_existingLocalizationId]),
      jsonEncode(basisEntities[_existingLocalizationId]),
    );
    expect(
      result.localizationAction,
      AuthoringRevision3DialogLocalizationAction.reusedExact,
    );
    expect(candidate['authoring_locales'], <Object?>['de', 'en']);
  });

  test(
    'ReuseExact Voice slot uses the native FEFF whitespace contract',
    () async {
      final projectJson = _projectJson(existingGermanText: '\ufeff');
      final request = _reuseRequest(projectJson: projectJson);
      final result = await _call(
        basisProjectJson: projectJson,
        request: request,
        response: _response(basisProjectJson: projectJson, request: request),
      );

      expect(
        result.localizationAction,
        AuthoringRevision3DialogLocalizationAction.reusedExact,
      );
      expect(result.voiceSlotId, _slotId);
    },
  );

  test('response rejects forged authority and semantic deltas', () async {
    final projectJson = _projectJson();
    final request = _createRequest(projectJson: projectJson);
    final valid = _response(basisProjectJson: projectJson, request: request);
    for (final mutation in <String, Object?>{
      'build_status': 'ready',
      'runtime_status': 'runtime_qualified',
      'topic_authority': 'granted',
      'publication_status': 'published',
    }.entries) {
      final response = Map<String, Object?>.from(valid)
        ..[mutation.key] = mutation.value;
      await expectLater(
        _call(
          basisProjectJson: projectJson,
          request: request,
          response: response,
        ),
        _throwsMalformed,
      );
    }

    final changed = (jsonDecode(valid['project_json']! as String) as Map)
        .cast<String, Object?>();
    final meta = (changed['meta']! as Map).cast<String, Object?>();
    meta['name'] = 'Forged unrelated delta';
    final changedResponse = Map<String, Object?>.from(valid)
      ..['project_json'] = jsonEncode(changed);
    await expectLater(
      _call(
        basisProjectJson: projectJson,
        request: request,
        response: changedResponse,
      ),
      _throwsMalformed,
    );
  });

  test('ReuseExact slot rejects whitespace-only exact locale text', () async {
    final projectJson = _projectJson(existingGermanText: '   ');
    final request = _reuseRequest(projectJson: projectJson);
    final response = _response(basisProjectJson: projectJson, request: request);

    await expectLater(
      _call(
        basisProjectJson: projectJson,
        request: request,
        response: response,
      ),
      _throwsMalformed,
    );
  });
}
