import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_fixture.dart';

String _validHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":321,'
    '"sha256":"${List.filled(64, 'a').join()}"}}';

String _validCandidateHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":654,'
    '"sha256":"${List.filled(64, 'b').join()}"}}';

String _validRevision3ProjectJson() =>
    '{"format":2,"schema_revision":3,'
    '"project_id":"00000000000000000000000000000003","revision":7,'
    '"meta":{"name":"Revision 3 Store","version":"1.0.0","author":"tests"},'
    '"target":{"executable":{"byte_len":1,'
    '"sha256":"${List.filled(64, '5').join()}"}},'
    '"authoring_locales":[],"entities":{},"asset_store":{"assets":{}}}';

Map<String, Object?> _validOpenedResponse() => <String, Object?>{
  'ok': true,
  'head_json': _validHeadJson(),
  'project_json': _validRevision3ProjectJson(),
};

Map<String, Object?> _validPreparedResponse() => <String, Object?>{
  'ok': true,
  'head_json': _validHeadJson(),
};

const _questId = '00000000000000000000000000000071';
const _scriptModuleId = '00000000000000000000000000000072';
const _questArtifactSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';

Map<String, Object?> _seal(int byteLength, String digit) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List.filled(64, digit).join(),
};

Map<String, Object?> _generation() => <String, Object?>{
  'executable': _seal(1, '5'),
};

Map<String, Object?> _resolvedQuestValue({
  required bool parent,
}) => <String, Object?>{
  'generation': _generation(),
  'source_seal': _seal(parent ? 11 : 12, parent ? '1' : '2'),
  'catalog_layer': parent ? 'base-game.quest-parent.v1' : 'base-game.npc.v1',
  'canonical_selector': parent ? 'SwampCamp_SCChapter2' : 'OM_GRD_Asghan_263',
  if (parent)
    'runtime_class': 'UQuest_SwampCamp_SCChapter2'
  else
    'runtime_unique_name': 'OM_GRD_Asghan_263',
};

Map<String, Object?> _validQuestInput({
  List<String> additionalObjectiveTitles = const <String>[],
}) => <String, Object?>{
  'target': _generation(),
  'quest_id': _questId,
  'module_namespace': 'GoreMods.Quests.Adapter',
  'technical_id': 'GORE_ADAPTER_QUEST',
  'text_helper': 'GoreAdapterQuestText',
  'parent_quest': _resolvedQuestValue(parent: true),
  'giver': _resolvedQuestValue(parent: false),
  'title': 'Adapter Quest',
  'description': 'Prepare one structural Quest candidate.',
  'objective_title': 'Finish Adapter Quest',
  if (additionalObjectiveTitles.isNotEmpty)
    'additional_objective_titles': additionalObjectiveTitles,
  'collision_catalog': <String, Object?>{
    'generation': _generation(),
    'catalog_layer':
        'base-game-plus-exact-revision3-project.story-collisions.v2',
    'artifact': _seal(123, 'e'),
    'source_seal': _seal(123, 'f'),
    'basis_snapshot': _seal(321, 'a'),
  },
};

String _questInputFingerprint(Map<String, Object?> input) {
  return revision3QuestInputFingerprint(input);
}

AuthoringRevision3QuestDraftIntentV3 _validQuestIntent({
  List<String> additionalObjectiveTitles = const <String>[],
}) => AuthoringRevision3QuestDraftIntentV3(
  moduleNamespace: 'GoreMods.Quests.Adapter',
  technicalId: 'GORE_ADAPTER_QUEST',
  textHelper: 'GoreAdapterQuestText',
  parentCatalogId: 'g1r:quest-parent:swampcamp_scchapter2',
  giverCatalogId: 'g1r:npc:om_grd_asghan_263',
  title: 'Adapter Quest',
  description: 'Prepare one structural Quest candidate.',
  objectiveTitle: 'Finish Adapter Quest',
  additionalObjectiveTitles: additionalObjectiveTitles,
);

String _validQuestCandidateProjectJson({
  List<String> additionalObjectiveTitles = const <String>[],
}) {
  final input = _validQuestInput(
    additionalObjectiveTitles: additionalObjectiveTitles,
  );
  final generatorVersion = additionalObjectiveTitles.isEmpty ? 2 : 3;
  final source = revision3QuestGeneratedSource(
    technicalId: 'GORE_ADAPTER_QUEST',
    textHelper: 'GoreAdapterQuestText',
    parentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
    giverRuntimeUniqueName: 'OM_GRD_Asghan_263',
    title: 'Adapter Quest',
    description: 'Prepare one structural Quest candidate.',
    objectiveTitle: 'Finish Adapter Quest',
    additionalObjectiveTitles: additionalObjectiveTitles,
  );
  return jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': '00000000000000000000000000000003',
    'revision': 8,
    'meta': <String, Object?>{
      'name': 'Revision 3 Store',
      'version': '1.0.0',
      'author': 'tests',
    },
    'target': _generation(),
    'authoring_locales': <Object?>[],
    'entities': <String, Object?>{
      _questId: <String, Object?>{
        'id': _questId,
        'display_name': 'Adapter Quest',
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_ADAPTER_QUEST',
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': 'quest_draft',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': generatorVersion,
            'input': input,
            'script_module': <String, Object?>{
              'project_id': '00000000000000000000000000000003',
              'id': _scriptModuleId,
              'expected_kind': 'script_module',
            },
          },
        },
      },
      _scriptModuleId: <String, Object?>{
        'id': _scriptModuleId,
        'display_name': 'Adapter Quest Script',
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': generatorVersion,
          'owner': <String, Object?>{
            'project_id': '00000000000000000000000000000003',
            'id': _questId,
            'expected_kind': 'quest_draft',
          },
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': generatorVersion,
            'owner': <String, Object?>{
              'project_id': '00000000000000000000000000000003',
              'id': _questId,
              'expected_kind': 'quest_draft',
            },
            'module_namespace': 'GoreMods.Quests.Adapter',
            'module_relative_path': 'GoreMods/Quests/Adapter.as',
            'source': source,
            'source_sha256': crypto.sha256
                .convert(utf8.encode(source))
                .toString(),
            'input_fingerprint': _questInputFingerprint(input),
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
      },
    },
    'asset_store': <String, Object?>{
      'assets': <String, Object?>{
        _questArtifactSha: <String, Object?>{
          'byte_len': 123,
          'media_type':
              'application/vnd.gore.quest-collision-capability+json;version=2',
        },
      },
    },
  });
}

String _mutatedQuestCandidateProjectJson(
  void Function(Map<String, Object?> project, Map<String, Object?> questInput)
  mutate,
) {
  final project =
      jsonDecode(_validQuestCandidateProjectJson()) as Map<String, Object?>;
  final entities = project['entities'] as Map<String, Object?>;
  final questEntity = entities[_questId] as Map<String, Object?>;
  final questPayload = questEntity['payload'] as Map<String, Object?>;
  final questData = questPayload['data'] as Map<String, Object?>;
  final questInput = questData['input'] as Map<String, Object?>;
  mutate(project, questInput);

  final moduleEntity = entities[_scriptModuleId] as Map<String, Object?>;
  final modulePayload = moduleEntity['payload'] as Map<String, Object?>;
  final moduleData = modulePayload['data'] as Map<String, Object?>;
  moduleData['input_fingerprint'] = _questInputFingerprint(questInput);
  return jsonEncode(project);
}

Map<String, Object?> _validQuestPreparedResponse({
  List<String> additionalObjectiveTitles = const <String>[],
}) => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_unpublished',
  'basis_head_json': _validHeadJson(),
  'head_json': _validCandidateHeadJson(),
  'project_json': _validQuestCandidateProjectJson(
    additionalObjectiveTitles: additionalObjectiveTitles,
  ),
  'revision': 8,
  'quest_id': _questId,
  'script_module_id': _scriptModuleId,
  'artifact_deduplicated': false,
  'build_status': 'blocked',
  'runtime_status': 'runtime_unqualified',
  'artifact_authority': 'not_granted',
  'source_inspection': 'fresh_capability_required',
  'publication_status': 'not_supported',
};

void main() {
  test('revision-3 Store commands are mandatory Studio capabilities', () {
    expect(
      requiredStudioCoreCommands.where(
        (command) => command.contains('revision3'),
      ),
      <String>[
        'authoring_store_build_revision3_reviewed_dataasset_v1',
        'authoring_store_build_revision3_voice_v1',
        'authoring_store_check_revision3_npc_compiler_v1',
        'authoring_store_check_revision3_quest_compiler_v1',
        'authoring_store_export_revision3_exact_snapshot_v1',
        'authoring_store_export_revision3_exact_snapshot_v2',
        'authoring_store_import_revision3_exact_snapshot_v2',
        'authoring_store_inspect_revision3_exact_snapshot_v2',
        'authoring_store_inspect_revision3_installed_dataasset_v1',
        'authoring_store_inspect_revision3_npc_source_v1',
        'authoring_store_inspect_revision3_quest_source_v1',
        'authoring_store_inspect_revision3_voice_take_media_v1',
        'authoring_store_list_revision3_dataasset_stages_v1',
        'authoring_store_list_revision3_history_v1',
        'authoring_store_materialize_revision3_voice_take_preview_v1',
        'authoring_store_open_revision3',
        'authoring_store_open_revision3_head_bytes',
        'authoring_store_plan_revision3_voice_v1',
        'authoring_store_prepare_remove_revision3_dataasset_stage_v1',
        'authoring_store_prepare_remove_revision3_story_draft_v1',
        'authoring_store_prepare_revision3_checkpoint',
        'authoring_store_prepare_revision3_dataasset_edit_v1',
        'authoring_store_prepare_revision3_dataasset_stage_v1',
        'authoring_store_prepare_revision3_dialog_line_v1',
        'authoring_store_prepare_revision3_dialog_localization_edit_v1',
        'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1',
        'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1',
        'authoring_store_prepare_revision3_history_restore_v1',
        'authoring_store_prepare_revision3_installed_dataasset_edit_v1',
        'authoring_store_prepare_revision3_npc_draft_v1',
        'authoring_store_prepare_revision3_npc_greeting_v1',
        'authoring_store_prepare_revision3_npc_profile_edit_v1',
        'authoring_store_prepare_revision3_quest_context_edit_v1',
        'authoring_store_prepare_revision3_quest_draft_v3',
        'authoring_store_prepare_revision3_quest_outline_edit_v1',
        'authoring_store_prepare_revision3_quest_outline_edit_v2',
        'authoring_store_prepare_revision3_quest_transcript_v1',
        'authoring_store_prepare_revision3_quest_transitions_edit_v1',
        'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1',
        'authoring_store_prepare_revision3_voice_take_removal_v1',
        'authoring_store_prepare_revision3_voice_take_selection_v1',
        'authoring_store_prepare_revision3_voice_take_status_v1',
        'authoring_store_prepare_revision3_voice_take_v1',
        'authoring_store_prepare_revision3_voice_target_v1',
        'authoring_store_read_revision3_content_index_v1',
        'authoring_store_read_revision3_dataasset_package_index_v1',
        'authoring_store_read_revision3_dialog_localization_edit_seed_v1',
        'authoring_store_read_revision3_dialog_localization_v1',
        'authoring_store_register_revision3_voice_take_preview_v1',
        'authoring_store_release_revision3_voice_take_preview_v1',
      ],
    );
    expect(
      requiredStudioCoreCommands,
      contains('authoring_read_dataasset_extract_receipt_v2'),
    );
  });

  test(
    'revision-3 Store wrappers preserve exact nested strings and payloads',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _validOpenedResponse(),
          'authoring_store_open_revision3_head_bytes': _validOpenedResponse(),
          'authoring_store_prepare_revision3_checkpoint':
              _validPreparedResponse(),
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(_validHeadJson());
      const root = 'C:\\Mods\\Revision "Three".goreproj';
      const rawDuplicateProject =
          '{"schema_revision":3,"revision":0,"revision":1}';

      final opened = await ffi.authoringStoreOpenRevision3(
        root: root,
        verification: AuthoringAssetVerification.structural,
      );
      final reopened = await ffi.authoringStoreOpenRevision3HeadBytes(
        root: root,
        head: head,
        verification: AuthoringAssetVerification.full,
      );
      final preparedAbsent = await ffi.authoringStorePrepareRevision3Checkpoint(
        root: root,
        expectedHead: null,
        projectJson: rawDuplicateProject,
      );
      final preparedCas = await ffi.authoringStorePrepareRevision3Checkpoint(
        root: root,
        expectedHead: head,
        projectJson: rawDuplicateProject,
      );

      expect(opened.projectJson, _validRevision3ProjectJson());
      expect(opened.projectId, '00000000000000000000000000000003');
      expect(opened.projectRevision, 7);
      expect(opened.head.canonicalJson, _validHeadJson());
      expect(reopened.projectJson, _validRevision3ProjectJson());
      expect(preparedAbsent.head.canonicalJson, _validHeadJson());
      expect(preparedCas.head.canonicalJson, _validHeadJson());
      expect(core.calls, hasLength(4));
      expect(core.calls[0].command, 'authoring_store_open_revision3');
      expect(core.calls[0].payload, <String, Object?>{
        'root': root,
        'verification': 'structural',
      });
      expect(
        core.calls[1].command,
        'authoring_store_open_revision3_head_bytes',
      );
      expect(core.calls[1].payload, <String, Object?>{
        'root': root,
        'head_json': _validHeadJson(),
        'verification': 'full',
      });
      expect(
        core.calls[2].command,
        'authoring_store_prepare_revision3_checkpoint',
      );
      expect(core.calls[2].payload, <String, Object?>{
        'root': root,
        'expected_head_json': null,
        'project_json': rawDuplicateProject,
      });
      expect(core.calls[3].payload, <String, Object?>{
        'root': root,
        'expected_head_json': _validHeadJson(),
        'project_json': rawDuplicateProject,
      });
    },
  );

  test('revision-3 open DTO rejects loose fields, types, and claims', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('ok'),
      (response) => response.remove('head_json'),
      (response) => response.remove('project_json'),
      (response) => response['ok'] = false,
      (response) => response['ok'] = 'true',
      (response) => response['head_json'] = 1,
      (response) => response['project_json'] = true,
      (response) => response['unknown'] = true,
      (response) => response['diagnostics'] = <Object?>[],
      (response) => response['blocks_build'] = false,
      (response) => response['readiness'] = 'ready',
      (response) => response['publication_status'] = 'supported',
    ];
    for (final mutate in mutations) {
      final response = _validOpenedResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3StoreOpenedResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'revision-3 open DTO accounts exact UTF-8 bounds and canonical bytes',
    () {
      final badNestedStrings = <void Function(Map<String, Object?>)>[
        (response) => response['head_json'] = String.fromCharCodes(
          Uint8List(64 * 1024 + 1),
        ),
        (response) => response['project_json'] = String.fromCharCodes(
          Uint8List(16 * 1024 * 1024 + 1),
        ),
        (response) => response['head_json'] = String.fromCharCode(0xd800),
        (response) => response['project_json'] = String.fromCharCode(0xd800),
        (response) =>
            response['project_json'] = ' ${_validRevision3ProjectJson()}',
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst('"schema_revision":3', '"schema_revision":2'),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst('"revision":7', '"revision":7,"revision":7'),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"name":"Revision 3 Store"',
              '"name":"Revision 3 Store","name":"shadow"',
            ),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"project_id":"00000000000000000000000000000003"',
              '"project_id":"00000000000000000000000000000000"',
            ),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"format":2,"schema_revision":3',
              '"schema_revision":3,"format":2',
            ),
      ];
      for (final mutate in badNestedStrings) {
        final response = _validOpenedResponse();
        mutate(response);
        expect(
          () => AuthoringRevision3StoreOpenedResult.fromJson(response),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'revision-3 project DTO rejects signed-unsafe numbers at every depth',
    () {
      final signedBoundary = _validRevision3ProjectJson().replaceFirst(
        '"byte_len":1',
        '"byte_len":9223372036854775807',
      );
      final accepted = _validOpenedResponse()
        ..['project_json'] = signedBoundary;
      expect(
        AuthoringRevision3StoreOpenedResult.fromJson(accepted).projectJson,
        signedBoundary,
      );

      for (final number in <String>[
        '-1',
        '1.0',
        '1e0',
        '9223372036854775808',
      ]) {
        final response = _validOpenedResponse()
          ..['project_json'] = _validRevision3ProjectJson().replaceFirst(
            '"byte_len":1',
            '"byte_len":$number',
          );
        expect(
          () => AuthoringRevision3StoreOpenedResult.fromJson(response),
          throwsFormatException,
          reason: 'nested JSON number $number must fail closed',
        );
      }

      final candidate = _validQuestPreparedResponse()
        ..['project_json'] = _validQuestCandidateProjectJson().replaceFirst(
          '"byte_len":123,"media_type"',
          '"byte_len":9223372036854775808,"media_type"',
        );
      expect(
        () => AuthoringRevision3QuestDraftPreparation.fromJson(candidate),
        throwsFormatException,
      );
    },
  );

  test('revision-3 prepare DTO is exact and exposes no authority claims', () {
    expect(
      AuthoringRevision3CheckpointPreparation.fromJson(
        _validPreparedResponse(),
      ).head.canonicalJson,
      _validHeadJson(),
    );

    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('ok'),
      (response) => response.remove('head_json'),
      (response) => response['ok'] = false,
      (response) => response['ok'] = 1,
      (response) => response['head_json'] = false,
      (response) => response['head_json'] = String.fromCharCodes(
        Uint8List(64 * 1024 + 1),
      ),
      (response) => response['head_json'] = String.fromCharCode(0xd800),
      (response) => response['unknown'] = true,
      (response) => response['diagnostics'] = <Object?>[],
      (response) => response['blocks_build'] = false,
      (response) => response['readiness'] = 'ready',
      (response) => response['publication_status'] = 'supported',
      (response) => response['project_json'] = _validRevision3ProjectJson(),
    ];
    for (final mutate in mutations) {
      final response = _validPreparedResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3CheckpointPreparation.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'revision-3 Quest wrapper preserves exact transports and parses only blocked preparation',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_quest_draft_v3':
              _validQuestPreparedResponse(),
        },
      );
      final ffi = ModFfi(core);
      final request = AuthoringRevision3QuestDraftRequestV3(
        expectedHead: AuthoringWorkingHead.fromCanonicalJson(_validHeadJson()),
        expectedProjectId: '00000000000000000000000000000003',
        expectedRevision: 7,
        questId: _questId,
        scriptModuleId: _scriptModuleId,
        displayName: 'Adapter Quest',
        intent: _validQuestIntent(),
      );
      const root = r'C:\Mods\Quest.goreproj';
      const gameRoot = r'D:\Games\Gothic Remake';
      final prepared = await ffi.authoringStorePrepareRevision3QuestDraftV3(
        root: root,
        gameRoot: gameRoot,
        currentProjectJson: _validRevision3ProjectJson(),
        questRequestJson: request.canonicalJson,
      );

      expect(prepared.basisHead.canonicalJson, _validHeadJson());
      expect(prepared.head.canonicalJson, _validCandidateHeadJson());
      expect(prepared.projectJson, _validQuestCandidateProjectJson());
      expect(prepared.projectId, '00000000000000000000000000000003');
      expect(prepared.revision, 8);
      expect(prepared.questId, _questId);
      expect(prepared.scriptModuleId, _scriptModuleId);
      expect(prepared.artifactDeduplicated, isFalse);
      expect(prepared.buildStatus, AuthoringRevision3QuestBuildStatus.blocked);
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3QuestRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.artifactAuthority,
        AuthoringRevision3QuestArtifactAuthority.notGranted,
      );
      expect(
        prepared.sourceInspection,
        AuthoringRevision3QuestSourceInspection.freshCapabilityRequired,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3QuestNativePublicationStatus.notSupported,
      );
      expect(core.calls, hasLength(1));
      expect(
        core.calls.single.command,
        'authoring_store_prepare_revision3_quest_draft_v3',
      );
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': _validRevision3ProjectJson(),
        'game_root': gameRoot,
        'quest_request_json': request.canonicalJson,
        'root': root,
      });

      final malformedCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_quest_draft_v3':
              _validQuestPreparedResponse()..['build_status'] = 'ready',
        },
      );
      await expectLater(
        ModFfi(malformedCore).authoringStorePrepareRevision3QuestDraftV3(
          root: root,
          gameRoot: gameRoot,
          currentProjectJson: _validRevision3ProjectJson(),
          questRequestJson: request.canonicalJson,
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );
    },
  );

  test('revision-3 Quest request DTO is exact, canonical, and basis-bound', () {
    final request = AuthoringRevision3QuestDraftRequestV3(
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_validHeadJson()),
      expectedProjectId: '00000000000000000000000000000003',
      expectedRevision: 7,
      questId: _questId,
      scriptModuleId: _scriptModuleId,
      displayName: 'Adapter Quest',
      intent: _validQuestIntent(),
    );
    final reopened = AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
      request.canonicalJson,
    );
    expect(reopened.expectedHead.canonicalJson, _validHeadJson());
    expect(reopened.expectedProjectId, '00000000000000000000000000000003');
    expect(reopened.expectedRevision, 7);
    expect(reopened.questId, _questId);
    expect(reopened.scriptModuleId, _scriptModuleId);
    expect(reopened.intent.technicalId, 'GORE_ADAPTER_QUEST');

    final raw = jsonDecode(request.canonicalJson) as Map<String, Object?>;
    final reordered = <String, Object?>{
      'expected_project_id': raw['expected_project_id'],
      for (final entry in raw.entries)
        if (entry.key != 'expected_project_id') entry.key: entry.value,
    };
    final malformed = <String>[
      ' ${request.canonicalJson}',
      '${request.canonicalJson}\n',
      request.canonicalJson.replaceFirst(
        '"expected_revision":7',
        '"expected_revision":7,"expected_revision":7',
      ),
      jsonEncode(<String, Object?>{...raw, 'authority': 'forged'}),
      jsonEncode(<String, Object?>{...raw}..remove('expected_head')),
      jsonEncode(<String, Object?>{...raw, 'quest_id': _scriptModuleId}),
      request.canonicalJson.replaceFirst(
        '"expected_revision":7',
        '"expected_revision":9223372036854775807',
      ),
      request.canonicalJson.replaceFirst(_questId, List.filled(32, '0').join()),
      jsonEncode(reordered),
    ];
    for (final value in malformed) {
      expect(
        () => AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(value),
        throwsFormatException,
        reason: value.substring(0, value.length.clamp(0, 120)),
      );
    }
  });

  test(
    'revision-3 Quest DTO persists a bounded ordered multi-objective extension',
    () {
      final objectives = <String>['Inspect the gate', 'Report to Asghan'];
      final request = AuthoringRevision3QuestDraftRequestV3(
        expectedHead: AuthoringWorkingHead.fromCanonicalJson(_validHeadJson()),
        expectedProjectId: '00000000000000000000000000000003',
        expectedRevision: 7,
        questId: _questId,
        scriptModuleId: _scriptModuleId,
        displayName: 'Adapter Quest',
        intent: _validQuestIntent(additionalObjectiveTitles: objectives),
      );
      expect(
        request.canonicalJson,
        contains(
          '"objective_title":"Finish Adapter Quest",'
          '"additional_objective_titles":["Inspect the gate","Report to Asghan"]',
        ),
      );
      final reopened = AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
        request.canonicalJson,
      );
      expect(reopened.intent.additionalObjectiveTitles, objectives);

      final prepared = AuthoringRevision3QuestDraftPreparation.fromJson(
        _validQuestPreparedResponse(additionalObjectiveTitles: objectives),
      );
      expect(prepared.additionalObjectiveTitles, objectives);
      expect(
        () => prepared.additionalObjectiveTitles.add('Mutate'),
        throwsUnsupportedError,
      );

      final decoded = jsonDecode(request.canonicalJson) as Map<String, Object?>;
      final invalidLists = <List<Object?>>[
        <Object?>[],
        <Object?>['finish adapter quest'],
        <Object?>[' Inspect the gate'],
        List<Object?>.filled(8, 'Too many'),
        <Object?>['Inspect the gate', 7],
      ];
      for (final invalid in invalidLists) {
        final candidate =
            jsonDecode(jsonEncode(decoded)) as Map<String, Object?>;
        final intent = candidate['intent'] as Map<String, Object?>;
        intent['additional_objective_titles'] = invalid;
        expect(
          () => AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
            jsonEncode(candidate),
          ),
          throwsFormatException,
        );
      }
    },
  );

  test('revision-3 Quest preparation rejects loose claims and broken pairs', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('basis_head_json'),
      (response) => response['unknown'] = true,
      (response) => response['outcome'] = 'published',
      (response) => response['head_json'] = _validHeadJson(),
      (response) => response['revision'] = 7,
      (response) => response['quest_id'] = _scriptModuleId,
      (response) => response['artifact_deduplicated'] = 'false',
      (response) => response['build_status'] = 'ready',
      (response) => response['runtime_status'] = 'qualified',
      (response) => response['artifact_authority'] = 'granted',
      (response) => response['source_inspection'] = 'available',
      (response) => response['publication_status'] = 'supported',
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            '"expected_kind":"script_module"',
            '"expected_kind":"quest_draft"',
          ),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            '"authoring":"offline_draft"',
            '"authoring":"published"',
          ),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst('"generator_version":2', '"generator_version":3'),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            '"authored_runtime_id":"GORE_ADAPTER_QUEST"',
            '"authored_runtime_id":"GORE_WRONG_QUEST"',
          ),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            '"basis_snapshot":{"byte_len":321',
            '"basis_snapshot":{"byte_len":322',
          ),
      (response) => response['project_json'] =
          _mutatedQuestCandidateProjectJson((project, questInput) {
            final collision =
                questInput['collision_catalog'] as Map<String, Object?>;
            final sourceSeal = collision['source_seal'] as Map<String, Object?>;
            sourceSeal['byte_len'] = 124;
          }),
      (response) => response['project_json'] =
          _mutatedQuestCandidateProjectJson((project, questInput) {
            const overLimit = 24 * 1024 * 1024 + 1;
            final collision =
                questInput['collision_catalog'] as Map<String, Object?>;
            final artifact = collision['artifact'] as Map<String, Object?>;
            final sourceSeal = collision['source_seal'] as Map<String, Object?>;
            artifact['byte_len'] = overLimit;
            sourceSeal['byte_len'] = overLimit;
            final assetStore = project['asset_store'] as Map<String, Object?>;
            final assets = assetStore['assets'] as Map<String, Object?>;
            final artifactMeta =
                assets[_questArtifactSha] as Map<String, Object?>;
            artifactMeta['byte_len'] = overLimit;
          }),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            'application/vnd.gore.quest-collision-capability+json;version=2',
            'application/octet-stream',
          ),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            RegExp(r'"source_sha256":"[0-9a-f]{64}"'),
            '"source_sha256":"${List.filled(64, '0').join()}"',
          ),
      (response) => response['project_json'] = _validQuestCandidateProjectJson()
          .replaceFirst(
            '"input_fingerprint":"${_questInputFingerprint(_validQuestInput())}"',
            '"input_fingerprint":"${List.filled(64, '0').join()}"',
          ),
    ];
    for (final mutate in mutations) {
      final response = _validQuestPreparedResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3QuestDraftPreparation.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'revision-3 Store requests fail locally on unsafe or oversized strings',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _validOpenedResponse(),
          'authoring_store_prepare_revision3_checkpoint':
              _validPreparedResponse(),
          'authoring_store_prepare_revision3_quest_draft_v3':
              _validQuestPreparedResponse(),
        },
      );
      final ffi = ModFfi(core);

      for (final root in <String>[
        '',
        String.fromCharCodes(Uint8List(32 * 1024 + 1).map((_) => 0x78)),
        'root\u0000tail',
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ffi.authoringStoreOpenRevision3(
            root: root,
            verification: AuthoringAssetVerification.full,
          ),
          throwsArgumentError,
        );
      }

      for (final projectJson in <String>[
        '',
        String.fromCharCodes(Uint8List(16 * 1024 * 1024 + 1)),
        String.fromCharCode(0xd800),
        // Raw size is valid, but conservative JSON escaping exceeds the 64 MiB transport cap.
        String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
      ]) {
        await expectLater(
          ffi.authoringStorePrepareRevision3Checkpoint(
            root: 'root',
            expectedHead: null,
            projectJson: projectJson,
          ),
          throwsArgumentError,
        );
      }
      for (final badPath in <String>[
        '',
        'path\u0000tail',
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ffi.authoringStorePrepareRevision3QuestDraftV3(
            root: badPath,
            gameRoot: 'game',
            currentProjectJson: _validRevision3ProjectJson(),
            questRequestJson: '{}',
          ),
          throwsArgumentError,
        );
        await expectLater(
          ffi.authoringStorePrepareRevision3QuestDraftV3(
            root: 'root',
            gameRoot: badPath,
            currentProjectJson: _validRevision3ProjectJson(),
            questRequestJson: '{}',
          ),
          throwsArgumentError,
        );
      }
      for (final questRequestJson in <String>[
        '',
        String.fromCharCode(0xd800),
        String.fromCharCodes(Uint8List(64 * 1024 + 1)),
      ]) {
        await expectLater(
          ffi.authoringStorePrepareRevision3QuestDraftV3(
            root: 'root',
            gameRoot: 'game',
            currentProjectJson: _validRevision3ProjectJson(),
            questRequestJson: questRequestJson,
          ),
          throwsArgumentError,
        );
      }
      await expectLater(
        ffi.authoringStorePrepareRevision3QuestDraftV3(
          root: 'root',
          gameRoot: 'game',
          currentProjectJson: _validRevision3ProjectJson().replaceFirst(
            '"byte_len":1',
            '"byte_len":9223372036854775808',
          ),
          questRequestJson: '{}',
        ),
        throwsFormatException,
      );
      expect(core.calls, isEmpty);
    },
  );
}
