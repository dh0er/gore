import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _bindingDomain =
    'gore-story-build.authoring-plan-v1.request-binding\u0000';
const _projectId = '01010101010101010101010101010101';
const _ownerId = '02020202020202020202020202020202';
const _scriptModuleId = '03030303030303030303030303030303';
const _combinedMessage =
    'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented';

void main() {
  test(
    'Story build plan returns one strictly bound immutable blocked DTO',
    () async {
      final projectJson = _projectJson();
      final response = _validResponse(projectJson);
      final core = FakeGoreCoreFfiService(
        responses: {'authoring_story_build_plan_v1_generate': response},
      );

      final result = await ModFfi(core).authoringStoryBuildPlanV1Generate(
        projectJson: projectJson,
        profile: AuthoringValidationProfile.production,
      );

      expect(result.requestBindingSha256, _binding(projectJson, 'production'));
      expect(result.validationProfile, AuthoringValidationProfile.production);
      expect(result.project.projectId, _projectId);
      expect(result.project.projectRevision, 7);
      expect(
        result.runtimeQualification,
        AuthoringStoryBuildRuntimeQualification.runtimeUnqualified,
      );
      expect(
        result.publicationStatus,
        AuthoringStoryBuildPublicationStatus.notSupported,
      );
      expect(result.moduleCount, 0);
      expect(result.diagnosticCount, 1);
      expect(result.blockingDiagnosticIndexes, [0]);
      expect(result.blocksBuild, isTrue);
      expect(
        () => result.blockingDiagnosticIndexes.add(1),
        throwsUnsupportedError,
      );
      expect(
        core.calls.single.command,
        'authoring_story_build_plan_v1_generate',
      );
      expect(core.calls.single.payload, <String, Object?>{
        'project_json': projectJson,
        'profile': 'production',
      });
    },
  );

  test(
    'Story build plan rejects confusion, tampering, and numeric looseness',
    () async {
      final projectJson = _projectJson();
      final malformed = <Map<String, Object?> Function()>[
        () => _validResponse(projectJson)..['extra'] = true,
        () =>
            _validResponse(projectJson)
              ..['request_binding_sha256'] = List.filled(64, 'f').join(),
        () {
          final response = _validResponse(projectJson);
          response['plan_json'] = '${response['plan_json']}\n';
          return response;
        },
        () {
          final response = _validResponse(projectJson);
          response['plan_json'] = (response['plan_json'] as String)
              .replaceFirst(
                '"format":"story_build_plan"',
                '"format":"story_build_plan","format":"story_build_plan"',
              );
          return response;
        },
        () =>
            _rewritePlan(projectJson, (plan) => plan['schema_revision'] = 1.0),
        () => _rewritePlan(
          projectJson,
          (plan) => plan['validation_profile'] = 'experimental',
        ),
        () => _rewritePlan(
          projectJson,
          (plan) => plan['publication_status'] = 'supported',
        )..['publication_status'] = 'supported',
        () =>
            _rewritePlan(projectJson, (plan) => plan['blocks_build'] = false)
              ..['blocks_build'] = false,
        () =>
            _validResponse(projectJson)
              ..['runtime_qualification'] = 'runtime_qualified',
        () => _validResponse(projectJson)..['module_count'] = 0.0,
        () => _validResponse(projectJson)..['diagnostic_count'] = 2,
        () =>
            _validResponse(projectJson)
              ..['blocking_diagnostic_indexes'] = <Object?>[0.0],
        () =>
            _validResponse(projectJson)
              ..['blocking_diagnostic_indexes'] = <Object?>[],
        () {
          final response = _validResponse(projectJson);
          (response['plan_seal'] as Map<String, Object?>)['sha256'] =
              List.filled(64, 'e').join();
          return response;
        },
        () => _rewritePlan(projectJson, (plan) {
          (plan['project'] as Map<String, Object?>)['project_revision'] = 7.0;
        }),
        () => _rewritePlan(projectJson, (plan) {
          (plan['diagnostics'] as List).single['extra'] = true;
        }),
        () => _rewritePlan(projectJson, (plan) {
          (plan['diagnostics'] as List).single['blocks_build'] = false;
        })..['blocking_diagnostic_indexes'] = <Object?>[],
        () {
          final response = _validResponse(projectJson);
          final project = response['project'] as Map<String, Object?>;
          (project['canonical_document'] as Map<String, Object?>)['byte_len'] =
              1;
          return response;
        },
      ];

      for (final build in malformed) {
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {'authoring_story_build_plan_v1_generate': build()},
            ),
          ).authoringStoryBuildPlanV1Generate(
            projectJson: projectJson,
            profile: AuthoringValidationProfile.production,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test('Story build request and closed plan bounds fail before use', () async {
    final projectJson = _projectJson();
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_story_build_plan_v1_generate': _validResponse(projectJson),
      },
    );
    final ffi = ModFfi(core);
    for (final invalid in <String>[
      '',
      String.fromCharCode(0xd800),
      List.filled(16 * 1024 * 1024 + 1, 'x').join(),
      List.filled(11 * 1024 * 1024, '\u0001').join(),
    ]) {
      await expectLater(
        ffi.authoringStoryBuildPlanV1Generate(
          projectJson: invalid,
          profile: AuthoringValidationProfile.production,
        ),
        throwsArgumentError,
      );
    }
    expect(core.calls, isEmpty);

    final oversizedPlan = _validResponse(projectJson)
      ..['plan_json'] = List.filled(32 * 1024 * 1024 + 1, 'x').join();
    await expectLater(
      ModFfi(
        FakeGoreCoreFfiService(
          responses: {'authoring_story_build_plan_v1_generate': oversizedPlan},
        ),
      ).authoringStoryBuildPlanV1Generate(
        projectJson: projectJson,
        profile: AuthoringValidationProfile.production,
      ),
      throwsFormatException,
    );

    final oversizedMessage = _rewritePlan(projectJson, (plan) {
      (plan['diagnostics'] as List).single['message'] = List.filled(
        16 * 1024 + 1,
        'x',
      ).join();
    });
    await expectLater(
      ModFfi(
        FakeGoreCoreFfiService(
          responses: {
            'authoring_story_build_plan_v1_generate': oversizedMessage,
          },
        ),
      ).authoringStoryBuildPlanV1Generate(
        projectJson: projectJson,
        profile: AuthoringValidationProfile.production,
      ),
      throwsFormatException,
    );
  });

  test('Story build rejects a self-consistent module absent from project', () {
    final projectJson = _projectJson();
    final response =
        _rewritePlan(projectJson, (plan) {
            plan['modules'] = <Object?>[_forgedNpcModule()];
            plan['diagnostics'] = <Object?>[
              _combinedDiagnostic(),
              _runtimeDiagnostic(),
            ];
          })
          ..['module_count'] = 1
          ..['diagnostic_count'] = 2
          ..['blocking_diagnostic_indexes'] = <Object?>[0, 1];

    expect(
      ModFfi(
        FakeGoreCoreFfiService(
          responses: {'authoring_story_build_plan_v1_generate': response},
        ),
      ).authoringStoryBuildPlanV1Generate(
        projectJson: projectJson,
        profile: AuthoringValidationProfile.production,
      ),
      throwsFormatException,
    );
  });

  test(
    'Story build diagnostics are strict Rust-canonical wire values',
    () async {
      final projectJson = _projectJson();
      final malformed = <Map<String, Object?>>[
        _rewritePlan(projectJson, (plan) {
          (plan['diagnostics'] as List).single['related_entities'] =
              <Object?>[];
        }),
        _rewritePlan(projectJson, (plan) {
            plan['diagnostics'] = <Object?>[
              _combinedDiagnostic(),
              _combinedDiagnostic(),
            ];
          })
          ..['diagnostic_count'] = 2
          ..['blocking_diagnostic_indexes'] = <Object?>[0, 1],
        _rewritePlan(projectJson, (plan) {
            plan['diagnostics'] = <Object?>[
              _runtimeDiagnostic(),
              _combinedDiagnostic(),
            ];
          })
          ..['diagnostic_count'] = 2
          ..['blocking_diagnostic_indexes'] = <Object?>[0, 1],
      ];

      for (final response in malformed) {
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {'authoring_story_build_plan_v1_generate': response},
            ),
          ).authoringStoryBuildPlanV1Generate(
            projectJson: projectJson,
            profile: AuthoringValidationProfile.production,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'Story build rejects an omitted draft with only the combined blocker',
    () {
      final projectJson = _projectJson(
        entities: <String, Object?>{_ownerId: _omittedNpcDraftEntity()},
      );
      final response = _validResponse(projectJson);

      expect(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {'authoring_story_build_plan_v1_generate': response},
          ),
        ).authoringStoryBuildPlanV1Generate(
          projectJson: projectJson,
          profile: AuthoringValidationProfile.production,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'Story build accepts causal blockers on an omitted draft or module',
    () async {
      final projectJson = _projectJson(
        entities: <String, Object?>{_ownerId: _omittedNpcDraftEntity()},
      );
      for (final entityId in <String>[_ownerId, _scriptModuleId]) {
        final response =
            _rewritePlan(projectJson, (plan) {
                plan['diagnostics'] = <Object?>[
                  _combinedDiagnostic(),
                  _causalDiagnostic(entityId),
                ];
              })
              ..['diagnostic_count'] = 2
              ..['blocking_diagnostic_indexes'] = <Object?>[0, 1];

        final result =
            await ModFfi(
              FakeGoreCoreFfiService(
                responses: {'authoring_story_build_plan_v1_generate': response},
              ),
            ).authoringStoryBuildPlanV1Generate(
              projectJson: projectJson,
              profile: AuthoringValidationProfile.production,
            );
        expect(result.moduleCount, 0);
        expect(result.diagnosticCount, 2);
      }
    },
  );
}

String _projectJson({Map<String, Object?>? entities}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 2,
      'project_id': _projectId,
      'revision': 7,
      'meta': <String, Object?>{
        'name': 'Story plan',
        'version': '0.1',
        'author': 'tests',
      },
      'target': <String, Object?>{'executable': _seal('1', 1000000)},
      'authoring_locales': <Object?>[],
      'entities': entities ?? <String, Object?>{},
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

Map<String, Object?> _combinedDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'property_path': 'schema_revision',
  'message': _combinedMessage,
  'blocks_build': true,
};

Map<String, Object?> _runtimeDiagnostic() => <String, Object?>{
  'code': 'RUNTIME_UNQUALIFIED',
  'severity': 'error',
  'entity': _ownerId,
  'property_path': 'payload.data.input',
  'message': 'runtime qualification is unavailable',
  'blocks_build': true,
};

Map<String, Object?> _causalDiagnostic(String entityId) => <String, Object?>{
  'code': 'MISSING_REFERENCE',
  'severity': 'error',
  'entity': entityId,
  'property_path': 'payload.data.script_module',
  'message': 'the referenced ScriptModule is unavailable',
  'blocks_build': true,
};

Map<String, Object?> _omittedNpcDraftEntity() => <String, Object?>{
  'id': _ownerId,
  'display_name': 'Omitted NPC',
  'origin': <String, Object?>{
    'type': 'new',
    'authored_runtime_id': 'omitted_npc',
  },
  'revision': 3,
  'payload': <String, Object?>{
    'kind': 'npc_draft',
    'data': <String, Object?>{
      'generator_id': 'logical_npc_clone_v1',
      'generator_version': 1,
      'input': <String, Object?>{},
      'script_module': _typedRef(_scriptModuleId, 'script_module'),
    },
  },
};

Map<String, Object?> _forgedNpcModule() {
  const source = 'class Forged {}\n';
  final paths = <String>[
    'payload.data.input.parent_character_definition.generation.executable',
    'payload.data.input.parent_character_definition.source_seal',
    'payload.data.input.parent_ai_agent_config.generation.executable',
    'payload.data.input.parent_ai_agent_config.source_seal',
    'payload.data.input.parent_spawn_definition.generation.executable',
    'payload.data.input.parent_spawn_definition.source_seal',
  ]..sort();
  return <String, Object?>{
    'script_module': _typedRef(_scriptModuleId, 'script_module'),
    'draft_input': <String, Object?>{
      'provenance': _entityProvenance(
        _ownerId,
        3,
        'npc_draft',
        'payload.data.input',
      ),
      'content': _seal('a', 64),
    },
    'persisted_source': <String, Object?>{
      'provenance': _entityProvenance(
        _scriptModuleId,
        4,
        'script_module',
        'payload.data.source',
      ),
      'content': _bytesSeal(source),
    },
    'sealed_inputs': <Object?>[
      <String, Object?>{
        'provenance': <String, Object?>{
          'scope': 'project',
          'project_id': _projectId,
          'project_revision': 7,
          'property_path': 'target.executable',
        },
        'content': _seal('1', 1000000),
      },
      for (final path in paths)
        <String, Object?>{
          'provenance': _entityProvenance(_ownerId, 3, 'npc_draft', path),
          'content': _seal('b', 32),
        },
    ],
    'generated': <String, Object?>{
      'generator_id': 'forged_generator',
      'generator_version': 1,
      'owner': _typedRef(_ownerId, 'npc_draft'),
      'module_namespace': 'Forged',
      'module_relative_path': 'Story/Forged.as',
      'source': source,
      'source_sha256': crypto.sha256.convert(utf8.encode(source)).toString(),
      'input_fingerprint': List.filled(64, 'c').join(),
      'status': <String, Object?>{
        'authoring': 'offline_draft',
        'runtime': 'runtime_unqualified',
      },
    },
  };
}

Map<String, Object?> _typedRef(String id, String kind) => <String, Object?>{
  'project_id': _projectId,
  'id': id,
  'expected_kind': kind,
};

Map<String, Object?> _entityProvenance(
  String id,
  int revision,
  String kind,
  String path,
) => <String, Object?>{
  'scope': 'entity',
  'project_id': _projectId,
  'project_revision': 7,
  'entity_id': id,
  'entity_revision': revision,
  'entity_kind': kind,
  'property_path': path,
};

Map<String, Object?> _validResponse(String projectJson) {
  final projectSeal = _bytesSeal(projectJson);
  final project = <String, Object?>{
    'project_id': _projectId,
    'project_revision': 7,
    'canonical_document': projectSeal,
    'target_executable': _seal('1', 1000000),
  };
  final plan = <String, Object?>{
    'format': 'story_build_plan',
    'schema_revision': 1,
    'validation_profile': 'production',
    'project': project,
    'publication_status': 'not_supported',
    'modules': <Object?>[],
    'diagnostics': <Object?>[_combinedDiagnostic()],
    'blocks_build': true,
  };
  final planJson = jsonEncode(plan);
  return <String, Object?>{
    'ok': true,
    'request_binding_sha256': _binding(projectJson, 'production'),
    'plan_json': planJson,
    'plan_seal': _bytesSeal(planJson),
    'validation_profile': 'production',
    'project': _deepCopy(project),
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'module_count': 0,
    'diagnostic_count': 1,
    'blocking_diagnostic_indexes': <Object?>[0],
    'blocks_build': true,
  };
}

Map<String, Object?> _rewritePlan(
  String projectJson,
  void Function(Map<String, Object?> plan) mutate,
) {
  final response = _validResponse(projectJson);
  final plan = (jsonDecode(response['plan_json'] as String) as Map)
      .cast<String, Object?>();
  mutate(plan);
  final planJson = jsonEncode(plan);
  response['plan_json'] = planJson;
  response['plan_seal'] = _bytesSeal(planJson);
  return response;
}

Map<String, Object?> _bytesSeal(String value) {
  final bytes = utf8.encode(value);
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

Map<String, Object?> _seal(String byte, int length) => <String, Object?>{
  'byte_len': length,
  'sha256': List.filled(64, byte).join(),
};

Map<String, Object?> _deepCopy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

String _binding(String projectJson, String profile) {
  final bytes = <int>[...utf8.encode(_bindingDomain)];
  for (final value in <String>[projectJson, profile]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return crypto.sha256.convert(bytes).toString();
}
