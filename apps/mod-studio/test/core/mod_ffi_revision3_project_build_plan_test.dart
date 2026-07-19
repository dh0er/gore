import 'dart:collection';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';

const _command = 'authoring_store_plan_revision3_project_build_v1';
const _root = r'C:\Projects\Readiness.goreproj';
const _stageMediaType =
    'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1';

String _headJson([String digit = 'b']) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 321,
    'sha256': List<String>.filled(64, digit).join(),
  },
});

Map<String, Object?> _sealBytes(List<int> bytes) => <String, Object?>{
  'byte_len': bytes.length,
  'sha256': crypto.sha256.convert(bytes).toString(),
};

Map<String, Object?> _sealJson(Object? value) =>
    _sealBytes(utf8.encode(jsonEncode(value)));

Map<String, Object?> _domain(
  String name, {
  int content = 0,
  int ready = 0,
  int blocked = 0,
}) => <String, Object?>{
  'domain': name,
  'status': content == 0
      ? 'not_present'
      : blocked == 0
      ? 'ready'
      : 'blocked',
  'content_count': content,
  'ready_count': ready,
  'blocked_count': blocked,
};

Map<String, Object?> _blocker(
  String category,
  String domain,
  String reason, {
  int affected = 1,
}) => <String, Object?>{
  'category': category,
  'domain': domain,
  'reason': reason,
  'affected_count': affected,
};

Map<String, Object?> _inputSeal(String projectJson) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final assetStore = (project['asset_store']! as Map).cast<String, Object?>();
  final assets = (assetStore['assets']! as Map).cast<String, Object?>();
  final stageDigests =
      assets.entries
          .where((entry) {
            final meta = (entry.value! as Map).cast<String, Object?>();
            return meta['media_type'] == _stageMediaType;
          })
          .map((entry) => entry.key)
          .toList(growable: false)
        ..sort();
  return _sealJson(<String, Object?>{
    'format': 'gore.authoring.revision3-project-build-input.v1',
    'project': _sealBytes(utf8.encode(projectJson)),
    'dataasset_stage_manifests': <Object?>[
      for (final digest in stageDigests)
        <String, Object?>{
          'byte_len': ((assets[digest]! as Map)['byte_len']! as int),
          'sha256': digest,
        },
    ],
  });
}

Map<String, Object?> _planProjection(Map<String, Object?> plan) =>
    <String, Object?>{
      'format': 'gore.authoring.revision3-project-build-plan.v1',
      'schema_revision': plan['schema_revision'],
      'project_id': plan['project_id'],
      'project_revision': plan['project_revision'],
      'outcome': plan['outcome'],
      'production_content_count': plan['production_content_count'],
      'input_seal': plan['input_seal'],
      'domains': plan['domains'],
      'blockers': plan['blockers'],
      'scope': plan['scope'],
      'build_authority': plan['build_authority'],
      'artifact_status': plan['artifact_status'],
      'deployment_status': plan['deployment_status'],
      'runtime_status': plan['runtime_status'],
      'publication_status': plan['publication_status'],
    };

void _reseal(Map<String, Object?> response) {
  final plan = (response['plan']! as Map).cast<String, Object?>();
  plan['plan_seal'] = _sealJson(_planProjection(plan));
}

Map<String, Object?> _response({
  required String projectJson,
  required String outcome,
  required int productionContentCount,
  required List<Map<String, Object?>> domains,
  List<Map<String, Object?>> blockers = const <Map<String, Object?>>[],
}) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final inputSeal = _inputSeal(projectJson);
  final projection = <String, Object?>{
    'format': 'gore.authoring.revision3-project-build-plan.v1',
    'schema_revision': 1,
    'project_id': project['project_id'],
    'project_revision': project['revision'],
    'outcome': outcome,
    'production_content_count': productionContentCount,
    'input_seal': inputSeal,
    'domains': domains,
    'blockers': blockers,
    'scope': 'project_build_readiness_only',
    'build_authority': 'not_granted',
    'artifact_status': 'not_created',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
  return <String, Object?>{
    'ok': true,
    'basis_head_json': _headJson(),
    'plan': <String, Object?>{
      'schema_revision': 1,
      'project_id': project['project_id'],
      'project_revision': project['revision'],
      'outcome': outcome,
      'production_content_count': productionContentCount,
      'input_seal': inputSeal,
      'plan_seal': _sealJson(projection),
      'domains': domains,
      'blockers': blockers,
      'scope': 'project_build_readiness_only',
      'build_authority': 'not_granted',
      'artifact_status': 'not_created',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
  };
}

List<Map<String, Object?>> _emptyDomains() => <Map<String, Object?>>[
  _domain('localization'),
  _domain('dialog'),
  _domain('voice'),
  _domain('npc'),
  _domain('quest'),
  _domain('scripts'),
  _domain('items'),
  _domain('data_assets'),
];

Map<String, Object?> _copy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

String _authoredTextProjectJson() {
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  final entities = SplayTreeMap<String, Object?>.from(
    (project['entities']! as Map).cast<String, Object?>(),
  );
  final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
      .cast<String, Object?>();
  localization['origin'] = <String, Object?>{
    'type': 'new',
    'authored_runtime_id': 'authored.localization',
  };
  final line = (entities[revision3VoiceFixtureLineId]! as Map)
      .cast<String, Object?>();
  line['origin'] = <String, Object?>{
    'type': 'generated',
    'generator_id': 'test.generated-dialog',
    'generator_version': 1,
    'owner': <String, Object?>{
      'project_id': revision3VoiceFixtureProjectId,
      'id': revision3VoiceFixtureLocalizationId,
      'expected_kind': 'localization_entry',
    },
  };
  entities[revision3VoiceFixtureTakeId] = <String, Object?>{
    'id': revision3VoiceFixtureTakeId,
    'display_name': 'Ignored history take',
    'origin': <String, Object?>{
      'type': 'imported',
      'importer': 'tests',
      'source_seal': <String, Object?>{
        'byte_len': 10,
        'sha256': List<String>.filled(64, '9').join(),
      },
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'voice_take',
      'data': <String, Object?>{},
    },
  };
  project['entities'] = entities;
  return jsonEncode(project);
}

String _npcAndScriptProjectJson() {
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  final entities = SplayTreeMap<String, Object?>.from(
    (project['entities']! as Map).cast<String, Object?>(),
  );
  for (final item in <(String, String)>[
    ('00000000000000000000000000000051', 'npc_draft'),
    ('00000000000000000000000000000052', 'script_module'),
  ]) {
    entities[item.$1] = <String, Object?>{
      'id': item.$1,
      'display_name': item.$2,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'authored.${item.$2}',
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': item.$2,
        'data': <String, Object?>{},
      },
    };
  }
  project['entities'] = entities;
  return jsonEncode(project);
}

String _dataAssetProjectJson() {
  final project = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
      .cast<String, Object?>();
  project['asset_store'] = <String, Object?>{
    'assets': <String, Object?>{
      List<String>.filled(64, '7').join(): <String, Object?>{
        'byte_len': 1234,
        'media_type': _stageMediaType,
      },
    },
  };
  return jsonEncode(project);
}

String _voiceProjectWithNameOrLine({String? name, String? lineLabel}) {
  final project =
      (jsonDecode(revision3VoiceFixtureProjectWithVoiceSlotCountJson(1)) as Map)
          .cast<String, Object?>();
  if (name != null) {
    final meta = (project['meta']! as Map).cast<String, Object?>();
    meta['name'] = name;
  }
  if (lineLabel != null) {
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final line = (entities[revision3VoiceFixtureLineId]! as Map)
        .cast<String, Object?>();
    line['display_name'] = lineLabel;
  }
  return jsonEncode(project);
}

String _voiceUnqualifiedAddProjectJson() {
  final project =
      (jsonDecode(revision3VoiceFixtureBuildReadyProjectJson()) as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final slot = entities.values
      .map((value) => (value! as Map).cast<String, Object?>())
      .singleWhere(
        (entity) =>
            ((entity['payload']! as Map<String, Object?>)['kind']) ==
            'voice_slot',
      );
  final payload = (slot['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  final resolution = (data['target_resolution']! as Map)
      .cast<String, Object?>();
  final target = (resolution['target']! as Map).cast<String, Object?>();
  target['operation'] = 'add';
  return jsonEncode(project);
}

Future<Object?> _parse(
  Map<String, Object?> response, {
  required String projectJson,
}) {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_command: response},
  );
  return ModFfi(core).authoringStorePlanRevision3ProjectBuildV1(
    root: _root,
    currentProjectJson: projectJson,
    expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
  );
}

Future<void> _expectMalformed(
  Map<String, Object?> response, {
  required String projectJson,
}) => expectLater(
  _parse(response, projectJson: projectJson),
  throwsA(
    isA<ModFfiException>().having(
      (error) => error.code,
      'code',
      ModFfiException.malformedNativeResponseCode,
    ),
  ),
);

void main() {
  test('wrapper sends the exact read-only Store request', () async {
    final projectJson = revision3VoiceFixtureProjectJson();
    final response = _response(
      projectJson: projectJson,
      outcome: 'empty',
      productionContentCount: 0,
      domains: _emptyDomains(),
    );
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: response},
    );

    final result = await ModFfi(core).authoringStorePlanRevision3ProjectBuildV1(
      root: _root,
      currentProjectJson: projectJson,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
    );

    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload, <String, Object?>{
      'current_project_json': projectJson,
      'expected_head_json': _headJson(),
      'root': _root,
    });
    expect(core.calls.single.payload, isNot(contains('game_root')));
    expect(core.calls.single.payload, isNot(contains('output')));
    expect(result.basisHead.canonicalJson, _headJson());
    expect(result.plan.isEmpty, isTrue);
    expect(result.plan.productionContentCount, 0);
    expect(result.plan.blockers, isEmpty);
  });

  test(
    'only new/generated text roots count; VoiceTake history does not',
    () async {
      final projectJson = _authoredTextProjectJson();
      final domains = _emptyDomains();
      domains[0] = _domain('localization', content: 1, blocked: 1);
      domains[1] = _domain('dialog', content: 1, blocked: 1);
      final response = _response(
        projectJson: projectJson,
        outcome: 'blocked',
        productionContentCount: 2,
        domains: domains,
        blockers: <Map<String, Object?>>[
          _blocker(
            'toolkit_support',
            'localization',
            'localization_lowering_unavailable',
          ),
          _blocker('toolkit_support', 'dialog', 'dialog_lowering_unavailable'),
        ],
      );

      final result =
          await _parse(response, projectJson: projectJson)
              as AuthoringRevision3ProjectBuildPlanResult;
      expect(result.plan.productionContentCount, 2);
      expect(result.plan.domains[0].contentCount, 1);
      expect(result.plan.domains[1].contentCount, 1);
      expect(result.plan.domains[2].contentCount, 0);
    },
  );

  test('ScriptModule is visible but excluded from production count', () async {
    final projectJson = _npcAndScriptProjectJson();
    final domains = _emptyDomains();
    domains[3] = _domain('npc', content: 1, blocked: 1);
    domains[5] = _domain('scripts', content: 1, blocked: 1);
    final response = _response(
      projectJson: projectJson,
      outcome: 'blocked',
      productionContentCount: 1,
      domains: domains,
      blockers: <Map<String, Object?>>[
        _blocker('toolkit_support', 'npc', 'npc_lowering_unavailable'),
        _blocker('toolkit_support', 'scripts', 'script_lowering_unavailable'),
      ],
    );

    final result =
        await _parse(response, projectJson: projectJson)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(result.plan.productionContentCount, 1);
    expect(result.plan.domains[5].contentCount, 1);
  });

  test(
    'Voice domain is independently bound to exact Voice expectation',
    () async {
      final projectJson = revision3VoiceFixtureProjectWithVoiceSlotCountJson(1);
      final domains = _emptyDomains();
      domains[2] = _domain('voice', content: 1, blocked: 1);
      final response = _response(
        projectJson: projectJson,
        outcome: 'blocked',
        productionContentCount: 1,
        domains: domains,
        blockers: <Map<String, Object?>>[
          _blocker('author_project', 'voice', 'voice_target_unresolved'),
          _blocker('author_project', 'voice', 'voice_selected_take_missing'),
        ],
      );

      final result =
          await _parse(response, projectJson: projectJson)
              as AuthoringRevision3ProjectBuildPlanResult;
      expect(result.plan.isBlocked, isTrue);
      expect(result.plan.blockers, hasLength(2));
      expect(result.plan.domains[2].blockedCount, 1);
    },
  );

  test('Voice name and line-label blockers remain distinct', () async {
    final domains = _emptyDomains();
    domains[2] = _domain('voice', content: 1, blocked: 1);
    Map<String, Object?> response(String projectJson) => _response(
      projectJson: projectJson,
      outcome: 'blocked',
      productionContentCount: 1,
      domains: domains,
      blockers: <Map<String, Object?>>[
        _blocker('author_project', 'voice', 'voice_project_name_unsupported'),
      ],
    );

    final unsafeName = _voiceProjectWithNameOrLine(name: 'bad/name');
    final accepted =
        await _parse(response(unsafeName), projectJson: unsafeName)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(
      accepted.plan.blockers.single.reason,
      AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported,
    );

    final malformedGraph = _voiceProjectWithNameOrLine(
      lineLabel: ' invalid line label',
    );
    final lineLabelResponse = _response(
      projectJson: malformedGraph,
      outcome: 'blocked',
      productionContentCount: 1,
      domains: domains,
      blockers: <Map<String, Object?>>[
        _blocker('author_project', 'voice', 'voice_line_label_unsupported'),
      ],
    );
    final lineLabelAccepted =
        await _parse(lineLabelResponse, projectJson: malformedGraph)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(
      lineLabelAccepted.plan.blockers.single.reason,
      AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported,
    );
    await _expectMalformed(
      response(malformedGraph),
      projectJson: malformedGraph,
    );
  });

  test('empty project name remains exact Voice unsupported evidence', () async {
    final projectJson = _voiceProjectWithNameOrLine(name: '');
    final domains = _emptyDomains();
    domains[2] = _domain('voice', content: 1, blocked: 1);
    final response = _response(
      projectJson: projectJson,
      outcome: 'blocked',
      productionContentCount: 1,
      domains: domains,
      blockers: <Map<String, Object?>>[
        _blocker('author_project', 'voice', 'voice_project_name_unsupported'),
      ],
    );

    final result =
        await _parse(response, projectJson: projectJson)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(
      result.plan.blockers.single.reason,
      AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported,
    );
  });

  test(
    'project name over 1024 UTF-8 bytes remains Voice unsupported evidence',
    () async {
      final projectJson = _voiceProjectWithNameOrLine(
        name: List<String>.filled(1025, 'a').join(),
      );
      final domains = _emptyDomains();
      domains[2] = _domain('voice', content: 1, blocked: 1);
      final response = _response(
        projectJson: projectJson,
        outcome: 'blocked',
        productionContentCount: 1,
        domains: domains,
        blockers: <Map<String, Object?>>[
          _blocker('author_project', 'voice', 'voice_project_name_unsupported'),
        ],
      );

      final result =
          await _parse(response, projectJson: projectJson)
              as AuthoringRevision3ProjectBuildPlanResult;
      expect(
        result.plan.blockers.single.reason,
        AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported,
      );
    },
  );

  test('unqualified Voice add is a toolkit-support blocker', () async {
    final projectJson = _voiceUnqualifiedAddProjectJson();
    final domains = _emptyDomains();
    domains[2] = _domain('voice', content: 1, blocked: 1);
    final response = _response(
      projectJson: projectJson,
      outcome: 'blocked',
      productionContentCount: 1,
      domains: domains,
      blockers: <Map<String, Object?>>[
        _blocker('toolkit_support', 'voice', 'voice_add_unqualified'),
      ],
    );

    final result =
        await _parse(response, projectJson: projectJson)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(
      result.plan.blockers.single.category,
      AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport,
    );
  });

  test('DataAsset stage manifests bind input seal and coverage', () async {
    final projectJson = _dataAssetProjectJson();
    final domains = _emptyDomains();
    domains[7] = _domain('data_assets', content: 1, ready: 1);
    final response = _response(
      projectJson: projectJson,
      outcome: 'coverage_complete',
      productionContentCount: 1,
      domains: domains,
    );

    final result =
        await _parse(response, projectJson: projectJson)
            as AuthoringRevision3ProjectBuildPlanResult;
    expect(result.plan.hasCompleteCoverage, isTrue);
    expect(result.plan.domains[7].readyCount, 1);
    expect(result.plan.buildAuthority.name, 'notGranted');
    expect(result.plan.artifactStatus.name, 'notCreated');
  });

  test(
    'strict parser rejects forged basis, counts, order, and authority',
    () async {
      final projectJson = revision3VoiceFixtureProjectJson();
      Map<String, Object?> valid() => _response(
        projectJson: projectJson,
        outcome: 'empty',
        productionContentCount: 0,
        domains: _emptyDomains(),
      );

      final stale = valid()..['basis_head_json'] = _headJson('c');
      await _expectMalformed(stale, projectJson: projectJson);

      final foreign = valid();
      (foreign['plan']! as Map<String, Object?>)['project_id'] = 'f' * 32;
      _reseal(foreign);
      await _expectMalformed(foreign, projectJson: projectJson);

      final wrongCount = valid();
      final wrongPlan = wrongCount['plan']! as Map<String, Object?>;
      wrongPlan['production_content_count'] = 1;
      _reseal(wrongCount);
      await _expectMalformed(wrongCount, projectJson: projectJson);

      final wrongOrder = valid();
      final orderedDomains =
          ((wrongOrder['plan']! as Map<String, Object?>)['domains']! as List);
      final first = orderedDomains[0];
      orderedDomains[0] = orderedDomains[1];
      orderedDomains[1] = first;
      _reseal(wrongOrder);
      await _expectMalformed(wrongOrder, projectJson: projectJson);

      final authority = valid();
      (authority['plan']! as Map<String, Object?>)['build_authority'] =
          'granted';
      _reseal(authority);
      await _expectMalformed(authority, projectJson: projectJson);

      final legacy = valid();
      (legacy['plan']! as Map<String, Object?>)['migration'] =
          <String, Object?>{};
      await _expectMalformed(legacy, projectJson: projectJson);
    },
  );

  test(
    'strict parser rejects forged blockers even under a valid plan seal',
    () async {
      final projectJson = revision3VoiceFixtureProjectWithVoiceSlotCountJson(1);
      final domains = _emptyDomains();
      domains[2] = _domain('voice', content: 1, blocked: 1);
      Map<String, Object?> valid() => _response(
        projectJson: projectJson,
        outcome: 'blocked',
        productionContentCount: 1,
        domains: domains,
        blockers: <Map<String, Object?>>[
          _blocker('author_project', 'voice', 'voice_target_unresolved'),
          _blocker('author_project', 'voice', 'voice_selected_take_missing'),
        ],
      );

      final reversed = valid();
      final reversedPlan = reversed['plan']! as Map<String, Object?>;
      reversedPlan['blockers'] = (reversedPlan['blockers']! as List).reversed
          .toList();
      _reseal(reversed);
      await _expectMalformed(reversed, projectJson: projectJson);

      final duplicate = valid();
      final duplicatePlan = duplicate['plan']! as Map<String, Object?>;
      final duplicateBlockers = duplicatePlan['blockers']! as List;
      duplicateBlockers[1] = _copy(
        (duplicateBlockers[0]! as Map).cast<String, Object?>(),
      );
      _reseal(duplicate);
      await _expectMalformed(duplicate, projectJson: projectJson);

      final wrongCategory = valid();
      final wrongCategoryPlan = wrongCategory['plan']! as Map<String, Object?>;
      ((wrongCategoryPlan['blockers']! as List)[0]!
              as Map<String, Object?>)['category'] =
          'toolkit_support';
      _reseal(wrongCategory);
      await _expectMalformed(wrongCategory, projectJson: projectJson);
    },
  );

  test('both independently recomputed seals are mandatory', () async {
    final projectJson = revision3VoiceFixtureProjectJson();
    Map<String, Object?> valid() => _response(
      projectJson: projectJson,
      outcome: 'empty',
      productionContentCount: 0,
      domains: _emptyDomains(),
    );

    final input = valid();
    final inputPlan = input['plan']! as Map<String, Object?>;
    (inputPlan['input_seal']! as Map<String, Object?>)['sha256'] = '1' * 64;
    _reseal(input);
    await _expectMalformed(input, projectJson: projectJson);

    final plan = valid();
    ((plan['plan']! as Map<String, Object?>)['plan_seal']!
            as Map<String, Object?>)['sha256'] =
        '2' * 64;
    await _expectMalformed(plan, projectJson: projectJson);
  });
}
