import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

String _draftSourceSha256(String source) =>
    crypto.sha256.convert(utf8.encode(source)).toString();

Map<String, Object?> _validNpcDraftResponse() => {
  'ok': true,
  'valid': true,
  'generated': <String, Object?>{
    'generator_id': 'gore-authoring.logical-npc-clone-draft',
    'generator_version': 1,
    'module_namespace': 'GoreMods.Probe.NpcLogicalCloneV1',
    'module_relative_path': 'GoreMods/Probe/NpcLogicalCloneV1.as',
    'unique_name': 'GORE_LOGICAL_ASGHAN_CLONE_V1',
    'classes': <String, Object?>{
      'character_definition':
          'UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1',
      'ai_agent_config': 'UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1',
      'spawn_definition':
          'USpawnAIAgentDefinition_GORE_LOGICAL_ASGHAN_CLONE_V1',
    },
    'source': 'class UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1\n',
    'source_sha256': _draftSourceSha256(
      'class UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1\n',
    ),
    'input_fingerprint': List.filled(64, 'b').join(),
    'status': <String, Object?>{
      'authoring': 'offline_draft',
      'runtime': 'runtime_unqualified',
    },
  },
  'diagnostics': <Object?>[],
};

Map<String, Object?> _draftGeneration(String byte) => {
  'executable': <String, Object?>{
    'byte_len': 1000000,
    'sha256': List.filled(32, byte).join(),
  },
};

Map<String, Object?> _draftSeal(String byte, int byteLength) => {
  'byte_len': byteLength,
  'sha256': List.filled(32, byte).join(),
};

Map<String, Object?> _validQuestDraftResponse() => {
  'ok': true,
  'valid': true,
  'generated': <String, Object?>{
    'target': _draftGeneration('11'),
    'quest_id': '0123456789abcdef0123456789abcdef',
    'generator_id': 'gore-authoring.draft-quest-skeleton',
    'generator_version': 1,
    'giver': <String, Object?>{
      'generation': _draftGeneration('11'),
      'source_seal': _draftSeal('22', 8192),
      'catalog_layer': 'base-game.g1r.characters',
      'canonical_selector': 'CatalogCharacter_00263',
      'runtime_unique_name': 'OM_GRD_Asghan_263',
    },
    'parent_quest': <String, Object?>{
      'generation': _draftGeneration('11'),
      'source_seal': _draftSeal('44', 4096),
      'catalog_layer': 'dependency.story-pack.quests',
      'canonical_selector': 'CatalogQuest_00263',
      'runtime_class': 'UQuest_SwampCamp_SCCHAPTER2',
    },
    'collision_catalog': <String, Object?>{
      'generation': _draftGeneration('11'),
      'source_seal': _draftSeal('33', 32768),
      'catalog_layer': 'resolved-loadout.scripts.v1',
    },
    'technical_names': <String, Object?>{
      'module_namespace': 'GoreMods.Probe.AsghanMiniQuest',
      'module_relative_path': 'GoreMods/Probe/AsghanMiniQuest.as',
      'root_class': 'UQuest_GORE_PROBE_ASGHAN_MINI',
      'objective_class': 'UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE',
      'text_helper': 'GoreProbeAsghanText',
      'root_getter': 'GetGoreProbeAsghanMini',
      'objective_getter': 'GetGoreProbeAsghanMiniObjective',
    },
    'fixed_shape': <String, Object?>{
      'quest_base_class': 'UG1RQuest',
      'root_kind': 'EQuestKind::Side',
      'objective_kind': 'EQuestKind::Subobjective',
      'root_external_start': true,
      'objective_external_start': true,
      'objective_external_success': true,
      'objective_succeeds_parent': true,
    },
    'source': 'class UQuest_GORE_PROBE_ASGHAN_MINI : UG1RQuest\n',
    'source_sha256': _draftSourceSha256(
      'class UQuest_GORE_PROBE_ASGHAN_MINI : UG1RQuest\n',
    ),
    'input_fingerprint': List.filled(64, 'd').join(),
    'status': <String, Object?>{
      'authoring': 'offline_draft',
      'discovery': 'runtime_unqualified',
      'transitions': 'transitions_runtime_unqualified',
    },
  },
  'diagnostics': <Object?>[],
};

Map<String, Object?> _invalidNpcDraftResponse() => {
  'ok': true,
  'valid': false,
  'generated': null,
  'diagnostics': <Object?>[_draftDiagnostic()],
};

Map<String, Object?> _draftDiagnostic() => {
  'code': 'NPC_INVALID_IDENTIFIER_CHARACTER',
  'field': 'unique_name',
  'message': 'technical identity contains invalid character',
};

Map<String, Object?> _draftGenerated(Map<String, Object?> response) =>
    (response['generated'] as Map).cast<String, Object?>();

Map<String, Object?> _draftGeneratedObject(
  Map<String, Object?> response,
  String field,
) => (_draftGenerated(response)[field] as Map).cast<String, Object?>();

Map<String, Object?> _firstDraftDiagnostic(Map<String, Object?> response) =>
    ((response['diagnostics'] as List<Object?>).single as Map)
        .cast<String, Object?>();

Map<String, Object?> _validVoiceMatchResponse() => {
  'ok': true,
  'archive': r'C:\Game\VoiceOver\german.zip',
  'archive_size': 4096,
  'archive_sha256': List.filled(32, 'ab').join(),
  'loc_id': 'LINE_ONE',
  'expected_basename': 'LINE_ONE.ogg',
  'resolution': 'unique',
  'match_count': 1,
  'matches': <Object?>[
    <String, Object?>{
      'index': 7,
      'path': 'Voices/Hero/line_one.OGG',
      'basename': 'line_one.OGG',
      'compressed_size': 100,
      'uncompressed_size': 128,
      'crc32': 0x12345678,
      'compression': 'stored',
      'compression_code': 0,
      'last_modified': <String, Object?>{
        'year': 2026,
        'month': 7,
        'day': 12,
        'hour': 13,
        'minute': 14,
        'second': 16,
      },
      'unix_mode': 0x81a4,
      'is_directory': false,
      'is_symlink': false,
      'encrypted': false,
    },
  ],
};

Map<String, Object?> _firstVoiceMatch(Map<String, Object?> response) =>
    ((response['matches'] as List<Object?>).single as Map)
        .cast<String, Object?>();

Map<String, Object?> _voiceTimestamp(Map<String, Object?> response) =>
    (_firstVoiceMatch(response)['last_modified'] as Map)
        .cast<String, Object?>();

Map<String, Object?> _validAuthoringCheckResponse() => {
  'ok': true,
  'canonical_project_json':
      '{"format":2,"schema_revision":1,"project_id":"00000000000000000000000000000001"}',
  'diagnostics': <Object?>[
    <String, Object?>{
      'code': 'INVALID_GENERATION_ANCHOR',
      'severity': 'error',
      'entity': null,
      'property_path': 'target.executable.byte_len',
      'message':
          'game generation executable seal must have a non-zero byte length',
      'related_entities': <Object?>[],
      'blocks_build': true,
    },
    <String, Object?>{
      'code': 'UNQUALIFIED_VOICE_ADD',
      'severity': 'warning',
      'entity': '00000000000000000000000000000001',
      'property_path': 'payload.data.target_resolution.target.operation',
      'message': 'new voice-member runtime binding is not qualified',
      'related_entities': <Object?>[
        '00000000000000000000000000000002',
        '00000000000000000000000000000003',
      ],
      'blocks_build': false,
    },
  ],
  'blocks_build': true,
};

String _validWorkingHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":321,"sha256":"${List.filled(64, 'a').join()}"}}';

String _validCanonicalProjectJson() =>
    '{"format":2,"schema_revision":1,'
    '"project_id":"00000000000000000000000000000001","revision":0,'
    '"meta":{"name":"Store bridge","version":"1.0.0","author":"tests"},'
    '"target":{"executable":{"byte_len":1,'
    '"sha256":"${List.filled(64, '4').join()}"}},'
    '"authoring_locales":[],"entities":{},"asset_store":{"assets":{}}}';

Map<String, Object?> _validStoreOpenedResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'project_json': _validCanonicalProjectJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validCheckpointPreparationResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validImportedOggResponse() => {
  'ok': true,
  'asset': <String, Object?>{
    'sha256': List.filled(64, 'b').join(),
    'byte_len': 4096,
    'logical_name': 'voice/asghan.ogg',
  },
  'ogg': <String, Object?>{
    'codec': 'vorbis',
    'channels': 1,
    'sample_rate': 48000,
    'pages': 3,
    'logical_streams': 1,
  },
  'deduplicated': false,
};

Map<String, Object?> _authoringDiagnostic(
  Map<String, Object?> response,
  int index,
) => ((response['diagnostics'] as List<Object?>)[index] as Map)
    .cast<String, Object?>();

Future<ModFfiException> _captureModFfiException(Future<Object?> call) async {
  try {
    await call;
  } on ModFfiException catch (error) {
    return error;
  }
  fail('expected ModFfiException');
}

class _MalformedJsonCoreService extends GoreCoreFfiService {
  @override
  String get description => 'malformed response fake';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) => throw const FormatException('hostile undecodable native response');
}

void main() {
  test('normal success response is returned to the command wrapper', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'find_game': {
          'ok': true,
          'found': true,
          'exe': r'C:\Game\GothicRemake.exe',
        },
      },
    );

    expect(await ModFfi(core).findGameExe(), r'C:\Game\GothicRemake.exe');
    expect(core.calls.single.command, 'find_game');
  });

  test(
    'structured native error preserves command, code, and message',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'audio_extract': {
            'ok': false,
            'error': {
              'code': 'NOT_FOUND',
              'message': 'sample not found: DIA_HERO_1',
            },
          },
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).audioExtract('speech.fsb', 'DIA_HERO_1'),
      );

      expect(error.command, 'audio_extract');
      expect(error.code, 'NOT_FOUND');
      expect(error.message, 'sample not found: DIA_HERO_1');
      expect(
        error.toString(),
        'audio_extract: sample not found: DIA_HERO_1 [NOT_FOUND]',
      );
    },
  );

  test(
    'malformed native error fields use one bounded local identity',
    () async {
      final oversizedCode = List.filled(129, 'A').join();
      final oversizedMessage = List.filled(64 * 1024 + 1, 'x').join();
      final multibyteCode = List.filled(65, 'Ä').join();
      final multibyteMessage = List.filled(32 * 1024 + 1, 'é').join();
      final malformedResponses = <Map<String, Object?>>[
        const {},
        const {'ok': 'false'},
        const {'ok': false},
        const {'ok': false, 'error': 'bad'},
        const {'ok': false, 'error': <String, Object?>{}},
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 'failure'},
          'extra': true,
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 'failure', 'extra': true},
        },
        const {
          'ok': false,
          'error': {'code': 7, 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': '', 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': 'bad_code', 'message': 'failure'},
        },
        {
          'ok': false,
          'error': {'code': oversizedCode, 'message': 'failure'},
        },
        {
          'ok': false,
          'error': {'code': multibyteCode, 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': 'IO'},
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 7},
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': '  \n'},
        },
        {
          'ok': false,
          'error': {'code': 'IO', 'message': oversizedMessage},
        },
        {
          'ok': false,
          'error': {'code': 'IO', 'message': multibyteMessage},
        },
      ];

      for (final response in malformedResponses) {
        final core = FakeGoreCoreFfiService(responses: {'find_game': response});
        final error = await _captureModFfiException(ModFfi(core).findGameExe());

        expect(error.command, 'find_game');
        expect(error.code, ModFfiException.malformedNativeResponseCode);
        expect(error.message, startsWith('malformed native response:'));
        expect(error.message.length, lessThan(128));
        expect(error.toString(), isNot(contains(oversizedMessage)));
        expect(error.toString(), isNot(contains(oversizedCode)));
        expect(error.toString(), isNot(contains(multibyteMessage)));
        expect(error.toString(), isNot(contains(multibyteCode)));
      }
    },
  );

  test(
    'undecodable response gets the stable malformed response code',
    () async {
      final error = await _captureModFfiException(
        ModFfi(_MalformedJsonCoreService()).findGameExe(),
      );

      expect(error.command, 'find_game');
      expect(error.code, ModFfiException.malformedNativeResponseCode);
      expect(
        error.toString(),
        'find_game: malformed native response: response could not be decoded '
        '[MALFORMED_NATIVE_RESPONSE]',
      );
      expect(error.toString(), isNot(contains('hostile undecodable')));
    },
  );

  test('scriptCompile propagates the new-symbol opt-in', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'script_compile': {
          'ok': true,
          'mini_path': 'mini.cache',
          'module': 'GoreMods.Probe',
        },
      },
    );

    await ModFfi(core).scriptCompile(
      gameDir: r'C:\Game',
      op: 'add',
      moduleName: 'GoreMods.Probe',
      relPath: 'GoreMods/Probe.as',
      asPath: r'C:\Source\Probe.as',
      workDir: r'C:\Temp\compile',
      allowNewSymbols: true,
    );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, 'script_compile');
    expect(core.calls.single.payload['allow_new_symbols'], isTrue);
  });

  test(
    'voiceArchiveMatchLine sends the command and parses a strict result',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_archive_match_line': _validVoiceMatchResponse()},
      );

      final result = await ModFfi(core).voiceArchiveMatchLine(
        archive: r'C:\Game\german.zip',
        locId: 'LINE_ONE',
      );

      expect(result.resolution, VoiceArchiveLineResolution.unique);
      expect(result.archiveSize, 4096);
      expect(result.matches.single.path, 'Voices/Hero/line_one.OGG');
      expect(core.calls.single.command, 'voice_archive_match_line');
      expect(core.calls.single.payload, {
        'archive': r'C:\Game\german.zip',
        'loc_id': 'LINE_ONE',
      });
    },
  );

  test(
    'authoringProjectCheck preserves raw JSON and uses a closed profile',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'authoring_project_check': _validAuthoringCheckResponse()},
      );
      const rawProject = '{"revision":0,"revision":1}';

      final result = await ModFfi(core).authoringProjectCheck(
        projectJson: rawProject,
        profile: AuthoringValidationProfile.experimental,
      );

      expect(core.calls, hasLength(1));
      expect(core.calls.single.command, 'authoring_project_check');
      expect(core.calls.single.payload, {
        'project_json': rawProject,
        'profile': 'experimental',
      });
      expect(result.blocksBuild, isTrue);
      expect(result.diagnostics, hasLength(2));
      expect(
        result.diagnostics.first.severity,
        AuthoringDiagnosticSeverity.error,
      );
      expect(
        result.diagnostics.last.entity,
        '00000000000000000000000000000001',
      );
      expect(
        () => result.diagnostics.clear(),
        throwsA(isA<UnsupportedError>()),
      );
      expect(
        () => result.diagnostics.last.relatedEntities.clear(),
        throwsA(isA<UnsupportedError>()),
      );
    },
  );

  test(
    'draft preview wrappers preserve raw JSON and parse typed results',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_logical_npc_clone_draft_v1_generate':
              _validNpcDraftResponse(),
          'authoring_draft_quest_skeleton_v1_generate':
              _validQuestDraftResponse(),
        },
      );
      const duplicateNpcInput =
          '{"unique_name":"FIRST","unique_name":"SECOND"}';
      const rawQuestInput = '{"technical_id":"GORE_TEST"}';

      final npc = await ModFfi(
        core,
      ).authoringLogicalNpcCloneDraftV1Generate(inputJson: duplicateNpcInput);
      final quest = await ModFfi(
        core,
      ).authoringDraftQuestSkeletonV1Generate(inputJson: rawQuestInput);

      expect(npc.valid, isTrue);
      expect(npc.generated?.generatorVersion, 1);
      expect(
        npc.generated?.classes.spawnDefinition,
        'USpawnAIAgentDefinition_GORE_LOGICAL_ASGHAN_CLONE_V1',
      );
      expect(npc.diagnostics, isEmpty);
      expect(
        npc.generated?.status.runtime,
        AuthoringDraftRuntimeStatus.runtimeUnqualified,
      );
      expect(() => npc.diagnostics.clear(), throwsUnsupportedError);
      expect(quest.valid, isTrue);
      expect(quest.generated?.questId, '0123456789abcdef0123456789abcdef');
      expect(
        quest.generated?.generatorId,
        'gore-authoring.draft-quest-skeleton',
      );
      expect(quest.generated?.target.executable.byteLength, 1000000);
      expect(quest.generated?.giver.runtimeUniqueName, 'OM_GRD_Asghan_263');
      expect(
        quest.generated?.parentQuest.runtimeClass,
        'UQuest_SwampCamp_SCCHAPTER2',
      );
      expect(
        quest.generated?.collisionCatalog.catalogLayer,
        'resolved-loadout.scripts.v1',
      );
      expect(quest.generated?.fixedShape.objectiveSucceedsParent, isTrue);
      expect(quest.generated?.fixedShape.questBaseClass, 'UG1RQuest');
      expect(
        quest.generated?.status.transitions,
        AuthoringDraftQuestTransitionStatus.transitionsRuntimeUnqualified,
      );
      expect(core.calls[0].payload, {'input_json': duplicateNpcInput});
      expect(core.calls[1].payload, {'input_json': rawQuestInput});
    },
  );

  test('draft invalid result remains typed and immutable', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_logical_npc_clone_draft_v1_generate':
            _invalidNpcDraftResponse(),
      },
    );

    final result = await ModFfi(
      core,
    ).authoringLogicalNpcCloneDraftV1Generate(inputJson: '{}');

    expect(result.valid, isFalse);
    expect(result.generated, isNull);
    expect(result.diagnostics.single.code, 'NPC_INVALID_IDENTIFIER_CHARACTER');
    expect(result.diagnostics.single.field, 'unique_name');
    expect(() => result.diagnostics.clear(), throwsUnsupportedError);
  });

  test(
    'draft DTOs reject loose, qualified, oversized, and inconsistent data',
    () {
      final badNpc = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) => response['valid'] = false,
        (response) => response['diagnostics'] = <Object?>[_draftDiagnostic()],
        (response) => _draftGenerated(response)['generator_id'] = 'other',
        (response) => _draftGenerated(response)['generator_version'] = 2,
        (response) =>
            _draftGenerated(response)['module_relative_path'] = '../escape.as',
        (response) =>
            (_draftGenerated(response)['classes'] as Map)['spawn_definition'] =
                'USpawnAIAgentDefinition_OTHER',
        (response) => (_draftGenerated(response)['status'] as Map)['runtime'] =
            'runtime_qualified',
        (response) => _draftGenerated(response)['source_sha256'] = List.filled(
          64,
          'A',
        ).join(),
        (response) => _draftGenerated(response)['source'] =
            '${_draftGenerated(response)['source']} ',
        (response) => _draftGenerated(response)['source'] = List.filled(
          1024 * 1024 + 1,
          'x',
        ).join(),
      ];
      for (final mutate in badNpc) {
        final response = _validNpcDraftResponse();
        mutate(response);
        expect(
          () => AuthoringLogicalNpcCloneDraftResult.fromJson(response),
          throwsFormatException,
        );
      }

      final badInvalid = <void Function(Map<String, Object?>)>[
        (response) => response['diagnostics'] = <Object?>[],
        (response) => _firstDraftDiagnostic(response)['code'] = 'FUTURE_CODE',
        (response) => _firstDraftDiagnostic(response)['field'] = '',
        (response) => _firstDraftDiagnostic(response)['message'] = List.filled(
          4097,
          'x',
        ).join(),
        (response) =>
            response['generated'] = _validNpcDraftResponse()['generated'],
      ];
      for (final mutate in badInvalid) {
        final response = _invalidNpcDraftResponse();
        mutate(response);
        expect(
          () => AuthoringLogicalNpcCloneDraftResult.fromJson(response),
          throwsFormatException,
        );
      }

      final badQuest = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) => _draftGenerated(response)['quest_id'] =
            '0123456789ABCDEF0123456789ABCDEF',
        (response) => _draftGenerated(response)['quest_id'] =
            '00000000000000000000000000000000',
        (response) => _draftGenerated(response)['generator_id'] = 'other',
        (response) =>
            (_draftGenerated(response)['target'] as Map)['unexpected'] = true,
        (response) =>
            _draftGeneratedObject(response, 'giver')['canonical_selector'] =
                'class',
        (response) =>
            _draftGeneratedObject(response, 'giver')['canonical_selector'] =
                '__hidden',
        (response) =>
            _draftGeneratedObject(response, 'giver')['catalog_layer'] =
                'Base Game',
        (response) =>
            ((_draftGenerated(response)['giver'] as Map)['source_seal']
                    as Map)['byte_len'] =
                0,
        (response) =>
            (((_draftGenerated(response)['giver'] as Map)['generation']
                    as Map)['executable']
                as Map)['sha256'] = List.filled(
              64,
              '9',
            ).join(),
        (response) => _draftGeneratedObject(
          response,
          'technical_names',
        )['module_relative_path'] = '../escape.as',
        (response) {
          final names = _draftGeneratedObject(response, 'technical_names');
          names['module_namespace'] = 'GoreMods.CON.Bad';
          names['module_relative_path'] = 'GoreMods/CON/Bad.as';
        },
        (response) => _draftGeneratedObject(
          response,
          'technical_names',
        )['objective_class'] = 'UQuest_OTHER',
        (response) =>
            _draftGeneratedObject(response, 'technical_names')['root_getter'] =
                'GetWrong',
        (response) =>
            _draftGeneratedObject(response, 'parent_quest')['runtime_class'] =
                'UG1RQuest',
        (response) =>
            _draftGeneratedObject(response, 'parent_quest')['runtime_class'] =
                'UQuest_GORE_PROBE_ASGHAN_MINI',
        (response) => _draftGeneratedObject(
          response,
          'fixed_shape',
        )['objective_succeeds_parent'] = false,
        (response) => _draftGeneratedObject(response, 'status')['discovery'] =
            'runtime_qualified',
      ];
      for (final mutate in badQuest) {
        final response = _validQuestDraftResponse();
        mutate(response);
        expect(
          () => AuthoringDraftQuestSkeletonResult.fromJson(response),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'draft preview wrappers enforce UTF-8 request limits before FFI',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_logical_npc_clone_draft_v1_generate':
              _validNpcDraftResponse(),
          'authoring_draft_quest_skeleton_v1_generate':
              _validQuestDraftResponse(),
        },
      );
      final ffi = ModFfi(core);

      await expectLater(
        ffi.authoringLogicalNpcCloneDraftV1Generate(
          inputJson: String.fromCharCodes(
            Uint16List(8193)..fillRange(0, 8193, 0x00e9),
          ),
        ),
        throwsArgumentError,
      );
      await expectLater(
        ffi.authoringDraftQuestSkeletonV1Generate(
          inputJson: String.fromCharCodes(
            Uint16List(10 * 1024 * 1024 + 1)
              ..fillRange(0, 10 * 1024 * 1024 + 1, 0x00e9),
          ),
        ),
        throwsArgumentError,
      );
      await expectLater(
        ffi.authoringDraftQuestSkeletonV1Generate(
          inputJson: String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    },
  );

  test('authoring DTO rejects malformed and inconsistent wire data', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['canonical_project_json'] = '',
      (response) => response['diagnostics'] = <String, Object?>{},
      (response) => (response['diagnostics'] as List<Object?>)[0] = 'bad',
      (response) => _authoringDiagnostic(response, 0)['code'] = 'bad_code',
      (response) => _authoringDiagnostic(response, 0)['severity'] = 'fatal',
      (response) => _authoringDiagnostic(response, 1)['entity'] =
          '0000000000000000000000000000000A',
      (response) => _authoringDiagnostic(response, 0).remove('entity'),
      (response) => _authoringDiagnostic(response, 0)['property_path'] = '',
      (response) => _authoringDiagnostic(response, 0)['message'] = '',
      (response) =>
          _authoringDiagnostic(response, 1)['related_entities'] = <Object?>[
            '00000000000000000000000000000003',
            '00000000000000000000000000000002',
          ],
      (response) => _authoringDiagnostic(response, 0)['blocks_build'] = 'true',
      (response) => response['blocks_build'] = false,
      (response) => response['blocks_build'] = 1,
    ];

    for (final mutate in malformed) {
      final response = _validAuthoringCheckResponse();
      mutate(response);
      expect(
        () => AuthoringProjectCheckResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'voice match DTO rejects fractional, negative, and out-of-range integers',
    () {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['match_count'] = 1.5,
        (response) => response['archive_size'] = -1,
        (response) => _firstVoiceMatch(response)['index'] = -1,
        (response) => _firstVoiceMatch(response)['compressed_size'] = 1.5,
        (response) => _firstVoiceMatch(response)['crc32'] = 0x100000000,
        (response) => _firstVoiceMatch(response)['compression_code'] = 0x10000,
        (response) => _firstVoiceMatch(response)['unix_mode'] = -1,
        (response) => _voiceTimestamp(response)['month'] = 13,
        (response) {
          _voiceTimestamp(response)['month'] = 2;
          _voiceTimestamp(response)['day'] = 31;
        },
      ];

      for (final mutate in malformed) {
        final response = _validVoiceMatchResponse();
        mutate(response);
        expect(
          () => VoiceArchiveMatchLineResult.fromJson(response),
          throwsFormatException,
        );
      }
    },
  );

  test('voice match DTO rejects inconsistent or ineligible match metadata', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['expected_basename'] = 'OTHER.ogg',
      (response) => response['loc_id'] = 'LÍNE_ONE',
      (response) => _firstVoiceMatch(response)['basename'] = 'OTHER.ogg',
      (response) =>
          _firstVoiceMatch(response)['path'] = 'Voices/Hero/OTHER.ogg',
      (response) =>
          _firstVoiceMatch(response)['path'] = r'Voices\Hero\line_one.OGG',
      (response) => _firstVoiceMatch(response)['is_symlink'] = true,
      (response) => _firstVoiceMatch(response)['encrypted'] = true,
      (response) => _firstVoiceMatch(response)['compression_code'] = 12,
      (response) => _firstVoiceMatch(response)['compression'] = 'deflated',
    ];

    for (final mutate in malformed) {
      final response = _validVoiceMatchResponse();
      mutate(response);
      expect(
        () => VoiceArchiveMatchLineResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'working-store wrappers preserve raw CAS/project bytes and typed payloads',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_open': _validStoreOpenedResponse(),
          'authoring_store_prepare_checkpoint':
              _validCheckpointPreparationResponse(),
          'authoring_store_open_head_bytes': _validStoreOpenedResponse(),
          'authoring_store_import_ogg': _validImportedOggResponse(),
          'authoring_store_verify_asset': {'ok': true},
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(
        _validWorkingHeadJson(),
      );
      const rawProject = '{"revision":0,"revision":1}';

      final opened = await ffi.authoringStoreOpen(
        root: r'C:\Mods\MyMod.goreproj',
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      );
      final prepared = await ffi.authoringStorePrepareCheckpoint(
        root: r'C:\Mods\MyMod.goreproj',
        expectedHead: null,
        projectJson: rawProject,
        profile: AuthoringValidationProfile.experimental,
      );
      final candidate = await ffi.authoringStoreOpenHeadBytes(
        root: r'C:\Mods\MyMod.goreproj',
        head: head,
        verification: AuthoringAssetVerification.structural,
        profile: AuthoringValidationProfile.experimental,
      );
      final imported = await ffi.authoringStoreImportOgg(
        root: r'C:\Mods\MyMod.goreproj',
        source: r'C:\Recordings\asghan.ogg',
        logicalName: 'voice/asghan.ogg',
        expectedHead: head,
      );
      await ffi.authoringStoreVerifyAsset(
        root: r'C:\Mods\MyMod.goreproj',
        asset: imported.asset,
        verification: AuthoringAssetVerification.full,
      );

      expect(opened.head.canonicalJson, _validWorkingHeadJson());
      expect(prepared.head.snapshotByteLength, 321);
      expect(candidate.projectJson, _validCanonicalProjectJson());
      expect(imported.ogg.codec, AuthoringOggCodec.vorbis);
      expect(imported.asset.logicalName, 'voice/asghan.ogg');
      expect(core.calls, hasLength(5));
      expect(core.calls[1].command, 'authoring_store_prepare_checkpoint');
      expect(core.calls[1].payload['project_json'], rawProject);
      expect(core.calls[1].payload['expected_head_json'], isNull);
      expect(core.calls[2].payload['head_json'], _validWorkingHeadJson());
      expect(
        core.calls[3].payload['expected_head_json'],
        _validWorkingHeadJson(),
      );
      expect(core.calls[4].payload['asset'], imported.asset.toJson());
    },
  );

  test('working-head DTO accepts only exact canonical bounded bytes', () {
    final valid = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson(),
    );
    expect(valid.snapshotByteLength, 321);
    expect(valid.snapshotSha256, List.filled(64, 'a').join());

    final malformed = <String>[
      '{}',
      ' ${_validWorkingHeadJson()}',
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":2',
      ),
      _validWorkingHeadJson().replaceFirst('"byte_len":321', '"byte_len":0'),
      _validWorkingHeadJson().replaceFirst(
        List.filled(64, 'a').join(),
        List.filled(64, 'A').join(),
      ),
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":1,"store_format":1',
      ),
      List.filled(64 * 1024 + 1, 'x').join(),
    ];
    for (final value in malformed) {
      expect(
        () => AuthoringWorkingHead.fromCanonicalJson(value),
        throwsFormatException,
      );
    }
  });

  test('working-store response DTOs reject loose or inconsistent data', () {
    final badOpen = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => response['head_json'] = ' ${_validWorkingHeadJson()}',
      (response) => response['project_json'] = '[]',
      (response) =>
          response['project_json'] = ' ${_validCanonicalProjectJson()}',
      (response) => response['project_json'] = _validCanonicalProjectJson()
          .replaceFirst('"revision":0', '"revision":0,"revision":0'),
      (response) =>
          response['project_json'] = _validCanonicalProjectJson().replaceFirst(
            '"format":2,"schema_revision":1',
            '"schema_revision":1,"format":2',
          ),
      (response) => response['diagnostics'] = <Object?>[
        _validAuthoringCheckResponse()['diagnostics'] as List<Object?>,
      ],
      (response) => response['blocks_build'] = true,
    ];
    for (final mutate in badOpen) {
      final response = _validStoreOpenedResponse();
      mutate(response);
      expect(
        () => AuthoringStoreOpenedResult.fromJson(response),
        throwsFormatException,
      );
    }

    final preparation = _validCheckpointPreparationResponse()
      ..['unexpected'] = true;
    expect(
      () => AuthoringCheckpointPreparation.fromJson(preparation),
      throwsFormatException,
    );

    final badImports = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => (response['asset'] as Map<String, Object?>)['byte_len'] = 0,
      (response) => (response['ogg'] as Map<String, Object?>)['codec'] = 'mp3',
      (response) => response['deduplicated'] = 0,
    ];
    for (final mutate in badImports) {
      final response = _validImportedOggResponse();
      mutate(response);
      expect(
        () => AuthoringImportedOgg.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test('working-store request bounds reject locally before FFI', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_open': _validStoreOpenedResponse(),
        'authoring_store_prepare_checkpoint':
            _validCheckpointPreparationResponse(),
        'authoring_store_import_ogg': _validImportedOggResponse(),
      },
    );
    final ffi = ModFfi(core);

    await expectLater(
      ffi.authoringStoreOpen(
        root: List.filled(32 * 1024 + 1, 'x').join(),
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringStorePrepareCheckpoint(
        root: 'root',
        expectedHead: null,
        projectJson: List.filled(16 * 1024 * 1024 + 1, 'x').join(),
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringStoreImportOgg(
        root: 'root',
        source: 'voice.ogg',
        logicalName: List.filled(1025, 'x').join(),
        expectedHead: null,
      ),
      throwsArgumentError,
    );
    expect(core.calls, isEmpty);
  });

  test('asset references enforce the phase-one 64 MiB blob limit', () {
    final sha256 = List.filled(64, 'c').join();
    final atLimit = AuthoringAssetRef(
      sha256: sha256,
      byteLength: 64 * 1024 * 1024,
      logicalName: 'voice.ogg',
    );
    expect(atLimit.byteLength, 64 * 1024 * 1024);
    expect(
      () => AuthoringAssetRef(
        sha256: sha256,
        byteLength: 64 * 1024 * 1024 + 1,
        logicalName: 'voice.ogg',
      ),
      throwsFormatException,
    );
  });

  test('verify wrapper rejects a success response with extra fields', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_verify_asset': {'ok': true, 'ignored': true},
      },
    );
    final asset = AuthoringAssetRef(
      sha256: List.filled(64, 'c').join(),
      byteLength: 1,
      logicalName: 'voice.ogg',
    );

    await expectLater(
      ModFfi(core).authoringStoreVerifyAsset(
        root: 'root',
        asset: asset,
        verification: AuthoringAssetVerification.full,
      ),
      throwsFormatException,
    );
  });
}
