import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:path/path.dart' as p;

Map<String, Object?> _compiledScriptReport(String miniPath) =>
    <String, Object?>{
      'ok': true,
      'outcome': 'compiled',
      'mini_path': miniPath,
      'module': 'GoreMods.Probe',
      'compile_error': null,
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
    };

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

const _storyCatalogRequest = '{"format":"story_catalog"}';
const _storyCatalogGameRoot = 'C:/Games/Gothic';
const _storyCatalogExecutable = r'C:\Game\Gothic1Remake.exe';
const _storyCatalogShippingCache =
    r'C:\Game\Alkimia\Content\Paks\Shipping-G1-Game.cache';
const _storyCatalogBindsCache =
    r'C:\Game\Alkimia\Content\Paks\Binds-G1-Game.cache';

Map<String, Object?> _catalogContentSeal(String byte, int byteLength) => {
  'byte_len': byteLength,
  'sha256': List.filled(64, byte).join(),
};

String _storyCatalogAlias(String catalogId, String role) {
  final bytes = <int>[
    ...utf8.encode('gore-story-catalog.authoring-selector-v1\u0000'),
  ];
  for (final value in <String>[catalogId, role]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

Map<String, Object?> _storyCatalogClass(
  String catalogId,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _storyCatalogAlias(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _catalogContentSeal(sealByte, 100),
};

Map<String, Object?> _storyCatalogNpc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper
      ? 'g1r:npc:om_stt_viper_302'
      : 'g1r:npc:om_grd_asghan_263';
  final character = _storyCatalogClass(
    catalogId,
    'character_definition',
    viper ? 'e' : 'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _storyCatalogClass(
      catalogId,
      'ai_agent_config',
      viper ? 'd' : 'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _storyCatalogClass(
      catalogId,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _storyCatalogAlias(catalogId, 'quest_giver'),
      'source_catalog_selector': character['source_catalog_selector'],
      'runtime_unique_name': runtime,
      'source_seal': character['source_seal'],
    },
    'discovery_status': 'sealed_cache_defaults_verified',
    'authoring_qualification': 'offline_qualified',
    'runtime_qualification': 'runtime_unqualified',
    'evidence_id': viper
        ? 'npc-logical-clone-v1:viper-current-v1'
        : 'npc-logical-clone-v1',
    'blocks_build': true,
  };
}

Map<String, Object?> _validStoryCatalogResponse({
  String catalogJson = _storyCatalogRequest,
}) => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': <String, Object?>{
      'edition': 'g1r-steam',
      'executable': _catalogContentSeal('1', 171698176),
      'shipping_cache': _catalogContentSeal('2', 123394250),
      'binds_cache': _catalogContentSeal('3', 5903938),
    },
    'catalog_seal': _catalogContentSeal('4', 5611),
    'npcs': <Object?>[
      _storyCatalogNpc(viper: false),
      _storyCatalogNpc(viper: true),
    ],
    'quest_parents': <Object?>[
      <String, Object?>{
        'catalog_id': 'g1r:quest-parent:swampcamp_scchapter2',
        'display_name': 'Swamp Camp — Chapter 2',
        'quest_class': _storyCatalogClass(
          'g1r:quest-parent:swampcamp_scchapter2',
          'quest_parent',
          'f',
          'UQuest_SwampCamp_SCCHAPTER2',
        ),
        'parent_class_name': 'UQuest_SwampCamp',
        'role': 'chapter',
        'qualification': 'curated_defaults_verified',
        'transition_qualification': 'runtime_unqualified',
        'evidence_id': 'current-cache-defaults-swampcamp-chapter2-20260712',
        'blocks_build': true,
      },
    ],
    'quest_collision_catalog': <String, Object?>{
      'status': 'inventory_unavailable',
      'catalog_layer': 'resolved-loadout.scripts.v1',
      'source_seal': _catalogContentSeal('2', 123394250),
      'blocks_draft_creation': true,
    },
    'blocks_build': true,
  },
};

Map<String, Object?> _storyCatalogGeneration() => <String, Object?>{
  'edition': 'g1r-steam',
  'executable': _catalogContentSeal('1', 171698176),
  'shipping_cache': _catalogContentSeal('2', 123394250),
  'binds_cache': _catalogContentSeal('3', 5903938),
};

String _storyCatalogBuildRaw() => jsonEncode(<String, Object?>{
  'format': 'story_catalog',
  'schema_revision': 1,
  'catalog': <String, Object?>{
    'generation': _storyCatalogGeneration(),
    'record_set_id': 'g1r-steam-1.0.3-curated-story-v1',
    'record_set_seal': _catalogContentSeal('5', 5499),
    'npcs': <Object?>[],
    'quest_parents': <Object?>[],
  },
  'catalog_seal': _catalogContentSeal('4', 5611),
});

String _storyCatalogBuildBinding({
  String executable = _storyCatalogExecutable,
  String shippingCache = _storyCatalogShippingCache,
  String bindsCache = _storyCatalogBindsCache,
}) {
  final bytes = <int>[
    ...utf8.encode(
      'gore-story-catalog.authoring-build-v1.request-binding\u0000',
    ),
  ];
  for (final value in <String>[executable, shippingCache, bindsCache]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return crypto.sha256.convert(bytes).toString();
}

Map<String, Object?> _validStoryCatalogBuildResponse({String? catalogJson}) =>
    <String, Object?>{
      'ok': true,
      'request_binding_sha256': _storyCatalogBuildBinding(),
      'catalog_json': catalogJson ?? _storyCatalogBuildRaw(),
      'generation': _storyCatalogGeneration(),
      'catalog_seal': _catalogContentSeal('4', 5611),
    };

String _storyCatalogGameRootBinding({String gameRoot = _storyCatalogGameRoot}) {
  final encoded = utf8.encode(gameRoot);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(
      'gore-story-catalog.authoring-build-for-game-root-v1.request-binding\u0000',
    ),
    ...length,
    ...encoded,
  ]).toString();
}

Map<String, Object?> _validStoryCatalogGameRootBuildResponse({
  String? catalogJson,
}) => <String, Object?>{
  'ok': true,
  'request_binding_sha256': _storyCatalogGameRootBinding(),
  'catalog_json': catalogJson ?? _storyCatalogBuildRaw(),
  'generation': _storyCatalogGeneration(),
  'catalog_seal': _catalogContentSeal('4', 5611),
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

Map<String, Object?> _validVoiceOggInspectResponse() => {
  'ok': true,
  'codec': 'vorbis',
  'pages': 2,
  'streams': 1,
  'content_seal': <String, Object?>{
    'byte_len': 4096,
    'sha256': List.filled(64, 'a').join(),
  },
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

String _validCanonicalRevision2ProjectJson() =>
    '{"format":2,"schema_revision":2,'
    '"project_id":"00000000000000000000000000000002","revision":1,'
    '"meta":{"name":"Store document bridge","version":"1.0.0","author":"tests"},'
    '"target":{"executable":{"byte_len":1,'
    '"sha256":"${List.filled(64, '5').join()}"}},'
    '"authoring_locales":[],"entities":{},"asset_store":{"assets":{}}}';

Map<String, Object?> _revision2CombinedValidationDiagnostic() => {
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'entity': null,
  'property_path': 'schema_revision',
  'message':
      'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented',
  'related_entities': <Object?>[],
  'blocks_build': true,
};

Map<String, Object?> _validStoreOpenedResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'project_json': _validCanonicalProjectJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validRevision2StoreOpenedResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'project_json': _validCanonicalRevision2ProjectJson(),
  'diagnostics': <Object?>[_revision2CombinedValidationDiagnostic()],
  'blocks_build': true,
};

Map<String, Object?> _validCheckpointPreparationResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validRevision2CheckpointPreparationResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'diagnostics': <Object?>[_revision2CombinedValidationDiagnostic()],
  'blocks_build': true,
};

const _storyDraftId = '10101010101010101010101010101010';
const _storyScriptModuleId = '11111111111111111111111111111111';
const _storyProjectId = '01010101010101010101010101010101';

Map<String, Object?> _storyGeneration() => <String, Object?>{
  'executable': <String, Object?>{
    'byte_len': 1000000,
    'sha256': List.filled(64, '1').join(),
  },
};

Map<String, Object?> _storySeal(String byte, int byteLength) =>
    <String, Object?>{
      'byte_len': byteLength,
      'sha256': List.filled(32, byte).join(),
    };

String _validStoryBaseProjectJson({int revision = 7}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 2,
      'project_id': _storyProjectId,
      'revision': revision,
      'meta': <String, Object?>{
        'name': 'Story transaction',
        'version': '0.1',
        'author': 'tests',
      },
      'target': _storyGeneration(),
      'authoring_locales': <Object?>[],
      'entities': <String, Object?>{},
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

Map<String, Object?> _storyNpcParent(
  String sealByte,
  String selector,
  String runtimeClass,
) => <String, Object?>{
  'generation': _storyGeneration(),
  'source_seal': _storySeal(sealByte, 20000),
  'catalog_layer': 'base-game.g1r.characters',
  'canonical_selector': selector,
  'runtime_class': runtimeClass,
};

Map<String, Object?> _storyNpcMutationInput() => <String, Object?>{
  'module_namespace': 'GoreMods.Npcs.GateGuard',
  'unique_name': 'GoreGateGuard',
  'parent_character_definition': _storyNpcParent(
    '02',
    'CatalogCharacterDefinition_Asghan',
    'UCharacterDefinition_Human_OM_GRD_Asghan_263',
  ),
  'parent_ai_agent_config': _storyNpcParent(
    '03',
    'CatalogAiAgentConfig_Asghan',
    'UAIAgentConfig_Human_OM_GRD_Asghan_263',
  ),
  'parent_spawn_definition': _storyNpcParent(
    '04',
    'CatalogSpawnDefinition_Asghan',
    'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
  ),
};

String _validStoryMutationJson({bool quest = false, int revision = 7}) =>
    jsonEncode(<String, Object?>{
      'expected_project_id': _storyProjectId,
      'expected_revision': revision,
      'draft_id': _storyDraftId,
      'script_module_id': _storyScriptModuleId,
      'display_name': quest ? 'Quest GORE_GATE_TRIAL' : 'NPC GoreGateGuard',
      'draft': <String, Object?>{
        'kind': quest ? 'quest' : 'npc',
        'input': quest
            ? <String, Object?>{
                'module_namespace': 'GoreMods.Quests.GateTrial',
                'technical_id': 'GORE_GATE_TRIAL',
                'text_helper': 'GoreGateTrialText',
                'parent_quest': <String, Object?>{
                  'generation': _storyGeneration(),
                  'source_seal': _storySeal('05', 30000),
                  'catalog_layer': 'base-game.g1r.quests',
                  'canonical_selector': 'CatalogQuest_AsghanParent',
                  'runtime_class': 'UQuest_SwampCamp_SCCHAPTER2',
                },
                'giver': <String, Object?>{
                  'generation': _storyGeneration(),
                  'source_seal': _storySeal('06', 40000),
                  'catalog_layer': 'base-game.g1r.characters',
                  'canonical_selector': 'CatalogCharacter_Asghan',
                  'runtime_unique_name': 'OM_GRD_Asghan_263',
                },
                'title': "Asghan's Trial",
                'description': 'Prove that the gate is secure.',
                'objective_title': 'Report to Asghan',
                'collision_catalog': <String, Object?>{
                  'generation': _storyGeneration(),
                  'source_seal': _storySeal('07', 50000),
                  'catalog_layer': 'resolved-loadout.scripts.v1',
                  'modules': <Object?>[],
                  'relative_paths': <Object?>[],
                  'symbols': <Object?>[],
                },
              }
            : _storyNpcMutationInput(),
      },
    });

Map<String, Object?> _storyTypedRef(String id, String kind) =>
    <String, Object?>{
      'project_id': _storyProjectId,
      'id': id,
      'expected_kind': kind,
    };

String _validStoryCandidateProjectJson({bool quest = false, int revision = 8}) {
  final moduleNamespace = quest
      ? 'GoreMods.Quests.GateTrial'
      : 'GoreMods.Npcs.GateGuard';
  final source = '// generated ${quest ? 'quest' : 'npc'}\n';
  final draftInput = <String, Object?>{
    'target': _storyGeneration(),
    if (quest) 'quest_id': _storyDraftId,
    ...(quest
        ? ((jsonDecode(_validStoryMutationJson(quest: true)) as Map)['draft']
                  as Map)['input']
              as Map<String, Object?>
        : _storyNpcMutationInput()),
  };
  return jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 2,
    'project_id': _storyProjectId,
    'revision': revision,
    'meta': <String, Object?>{
      'name': 'Story transaction',
      'version': '0.1',
      'author': 'tests',
    },
    'target': _storyGeneration(),
    'authoring_locales': <Object?>[],
    'entities': <String, Object?>{
      _storyDraftId: <String, Object?>{
        'id': _storyDraftId,
        'display_name': quest ? 'Quest GORE_GATE_TRIAL' : 'NPC GoreGateGuard',
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': quest ? 'GORE_GATE_TRIAL' : 'GoreGateGuard',
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': quest ? 'quest_draft' : 'npc_draft',
          'data': <String, Object?>{
            'generator_id': quest
                ? 'gore-authoring.draft-quest-skeleton'
                : 'gore-authoring.logical-npc-clone-draft',
            'generator_version': 1,
            'input': draftInput,
            'script_module': _storyTypedRef(
              _storyScriptModuleId,
              'script_module',
            ),
          },
        },
      },
      _storyScriptModuleId: <String, Object?>{
        'id': _storyScriptModuleId,
        'display_name': moduleNamespace,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': quest
              ? 'gore-authoring.draft-quest-skeleton'
              : 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'owner': _storyTypedRef(
            _storyDraftId,
            quest ? 'quest_draft' : 'npc_draft',
          ),
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': quest
                ? 'gore-authoring.draft-quest-skeleton'
                : 'gore-authoring.logical-npc-clone-draft',
            'generator_version': 1,
            'owner': _storyTypedRef(
              _storyDraftId,
              quest ? 'quest_draft' : 'npc_draft',
            ),
            'module_namespace': moduleNamespace,
            'module_relative_path':
                '${moduleNamespace.replaceAll('.', '/')}.as',
            'source': source,
            'source_sha256': crypto.sha256
                .convert(utf8.encode(source))
                .toString(),
            'input_fingerprint': List.filled(64, 'a').join(),
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
      },
    },
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
}

String _storyRequestBinding(
  String projectJson,
  String mutationJson,
  String profile,
) {
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore-authoring.story-draft-insert-v1.request-binding\u0000'),
    );
  for (final part in <List<int>>[
    utf8.encode(projectJson),
    utf8.encode(mutationJson),
    utf8.encode(profile),
  ]) {
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, part.length, Endian.little);
    bytes
      ..add(length)
      ..add(part);
  }
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Map<String, Object?> _validStoryAppliedResponse({
  bool quest = false,
  String? base,
  String? mutation,
  String profile = 'experimental',
}) {
  base ??= _validStoryBaseProjectJson();
  mutation ??= _validStoryMutationJson(quest: quest);
  final baseRevision = (jsonDecode(base) as Map<String, Object?>)['revision'];
  if (baseRevision is! int) {
    throw StateError('test Story base revision is not an int');
  }
  final candidateRevision = baseRevision + 1;
  return <String, Object?>{
    'ok': true,
    'outcome': 'applied',
    'request_binding_sha256': _storyRequestBinding(base, mutation, profile),
    'project_json': _validStoryCandidateProjectJson(
      quest: quest,
      revision: candidateRevision,
    ),
    'revision': candidateRevision,
    'draft_id': _storyDraftId,
    'draft_kind': quest ? 'quest_draft' : 'npc_draft',
    'script_module_id': _storyScriptModuleId,
    'diagnostics': <Object?>[_revision2CombinedValidationDiagnostic()],
    'blocks_build': true,
  };
}

Map<String, Object?> _validStoryRejectedResponse({
  String? base,
  String? mutation,
  String profile = 'production',
}) {
  base ??= _validStoryBaseProjectJson();
  mutation ??= _validStoryMutationJson(revision: 6);
  return <String, Object?>{
    'ok': true,
    'outcome': 'rejected',
    'request_binding_sha256': _storyRequestBinding(base, mutation, profile),
    'diagnostics': <Object?>[
      <String, Object?>{
        'code': 'PROJECT_REVISION_CONFLICT',
        'severity': 'error',
        'entity': null,
        'property_path': 'expected_revision',
        'message':
            'story transaction expected project revision 6, but candidate is 7',
        'related_entities': <Object?>[],
        'blocks_build': true,
      },
    ],
  };
}

AuthoringStoryDraftInsertResult _decodeStoryResponse(
  Map<String, Object?> response, {
  bool quest = false,
  String? base,
  String? mutation,
  AuthoringValidationProfile profile = AuthoringValidationProfile.experimental,
}) => AuthoringStoryDraftInsertResult.fromJson(
  response,
  projectJson: base ?? _validStoryBaseProjectJson(),
  mutationJson: mutation ?? _validStoryMutationJson(quest: quest),
  profile: profile,
);

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
    'scriptCompileReportV1 returns compiler failure as structured data',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': {
            'ok': true,
            'outcome': 'failed',
            'mini_path': null,
            'module': null,
            'compile_error': {
              'code': 'COMPILER_REGEN_FAILED',
              'message': 'compiler rejected the source',
            },
            'compiler_diagnostics': {
              'capture': 'captured',
              'messages': [
                {
                  'file': 'GoreMods/Probe.as',
                  'line': 5,
                  'column': 9,
                  'severity': 'error',
                  'message': 'Expected expression',
                },
              ],
              'omitted': 0,
            },
            'install_restore': 'restored_exact',
            'recovery_required': false,
          },
        },
      );

      final report = await ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: r'C:\Temp\compile',
        allowNewSymbols: true,
      );

      expect(report.compiled, isFalse);
      expect(report.failure!.code, 'COMPILER_REGEN_FAILED');
      expect(report.diagnostics!.messages.single.line, 5);
      expect(core.calls.single.command, 'script_compile_report_v1');
      expect(core.calls.single.payload['allow_new_symbols'], isTrue);
    },
  );

  test('scriptCompileReportV1 rejects a malformed success envelope', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'script_compile_report_v1': {
          'ok': true,
          'outcome': 'compiled',
          'mini_path': 'mini.cache',
          'module': 'GoreMods.Probe',
          'compile_error': null,
          'compiler_diagnostics': null,
          'install_restore': 'restored_exact',
          'recovery_required': false,
        },
      },
    );

    final error = await _captureModFfiException(
      ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: r'C:\Temp\compile',
      ),
    );

    expect(error.code, ModFfiException.malformedNativeResponseCode);
    expect(
      error.message,
      'malformed native response: compile report schema is invalid',
    );
  });

  test(
    'scriptCompileReportV1 accepts only a marked direct owned output',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      File(
        p.join(owned.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1, 2, 3]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final report = await ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: work.path,
      );

      expect(report.compiled, isTrue);
      expect(report.miniPath, mini.path);
    },
  );

  test(
    'scriptCompileReportV1 maps missing ownership evidence to malformed',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(error.code, ModFfiException.malformedNativeResponseCode);
    },
  );

  test(
    'scriptCompileReportV1 rejects an extended-prefix output response',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      File(
        p.join(owned.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(
            '\\\\?\\${mini.path}',
          ),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(error.code, ModFfiException.malformedNativeResponseCode);
    },
  );

  test('scriptCompileReportV1 rejects non-native owned-child shapes', () async {
    final work = Directory.systemTemp.createTempSync('gore-owned-output-test-');
    addTearDown(() => work.deleteSync(recursive: true));
    final candidates = <Directory>[
      Directory(p.join(work.path, 'nested', 'gore-owned-compile-a1b2c3d4e5f6')),
      Directory(p.join(work.path, 'gore-owned-compile-A1b2c3d4e5f6')),
    ];
    for (final candidate in candidates) {
      candidate.createSync(recursive: true);
      File(
        p.join(candidate.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(candidate.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(
        error.code,
        ModFfiException.malformedNativeResponseCode,
        reason: candidate.path,
      );
    }
  });

  test(
    'scriptCompileInstallStateV1 sends the root and parses safety',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_install_state_v1': <String, Object?>{
            'ok': true,
            'disposition': 'safe_to_compile',
            'safe_to_compile': true,
            'game_process': 'not_running',
            'artifacts': <Object?>[],
            'issues': <Object?>[],
          },
        },
      );

      final state = await ModFfi(
        core,
      ).scriptCompileInstallStateV1(gameDir: r'C:\Game');

      expect(state.safeToCompile, isTrue);
      expect(core.calls.single.command, 'script_compile_install_state_v1');
      expect(core.calls.single.payload, <String, Object?>{
        'game_dir': r'C:\Game',
      });
    },
  );

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
    'voiceOggInspectV1 sends only the selected path and parses facts',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
      );

      final result = await ModFfi(
        core,
      ).voiceOggInspectV1(oggPath: r'C:\Recordings\viper.ogg');

      expect(result.codec, VoiceOggCodec.vorbis);
      expect(result.pages, 2);
      expect(result.streams, 1);
      expect(result.contentSeal.byteLength, 4096);
      expect(result.contentSeal.sha256, List.filled(64, 'a').join());
      expect(core.calls.single.command, 'voice_ogg_inspect_v1');
      expect(core.calls.single.payload, {
        'ogg_path': r'C:\Recordings\viper.ogg',
      });
    },
  );

  test('voice Ogg request bounds fail locally before core execution', () async {
    final core = FakeGoreCoreFfiService(
      responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
    );
    final ffi = ModFfi(core);

    for (final path in <String>[
      '',
      'bad\u0000path.ogg',
      List.filled(32 * 1024 + 1, 'x').join(),
      List.filled(10923, '\u20ac').join(),
      String.fromCharCode(0xd800),
    ]) {
      await expectLater(
        ffi.voiceOggInspectV1(oggPath: path),
        throwsArgumentError,
      );
    }
    expect(core.calls, isEmpty);
  });

  test(
    'voice Ogg request accepts the native escaped-envelope boundary',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
      );
      final boundaryPath = List.filled(32 * 1024, '\u0001').join();

      await ModFfi(core).voiceOggInspectV1(oggPath: boundaryPath);

      expect(core.calls.single.payload, {'ogg_path': boundaryPath});
    },
  );

  test('voice Ogg inspection DTO rejects non-exact or implausible facts', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => response.remove('streams'),
      (response) => response['ok'] = false,
      (response) => response['codec'] = 'mp3',
      (response) => response['pages'] = 0,
      (response) => response['pages'] = 1.5,
      (response) => response['pages'] = 0x100000000,
      (response) => response['streams'] = 0,
      (response) => response['streams'] = 3,
      (response) => response['content_seal'] = 'not-an-object',
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['extra'] = true,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['byte_len'] = 26,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['byte_len'] =
              64 * 1024 * 1024 + 1,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['sha256'] =
              List.filled(64, 'A').join(),
      (response) {
        response['pages'] = 2;
        (response['content_seal'] as Map<String, Object?>)['byte_len'] = 53;
      },
    ];

    for (final mutate in malformed) {
      final response = _validVoiceOggInspectResponse();
      mutate(response);
      expect(
        () => VoiceOggInspectionResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test('voice Ogg inspection DTO accepts the closed Opus codec', () {
    final response = _validVoiceOggInspectResponse()..['codec'] = 'opus';

    expect(
      VoiceOggInspectionResult.fromJson(response).codec,
      VoiceOggCodec.opus,
    );
  });

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
    'Story catalog wrapper preserves raw input and parses immutable choices',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(),
        },
      );

      final result = await ModFfi(
        core,
      ).authoringStoryCatalogV1Read(catalogJson: _storyCatalogRequest);

      expect(core.calls, hasLength(1));
      expect(core.calls.single.command, 'authoring_story_catalog_v1_read');
      expect(core.calls.single.payload, {'catalog_json': _storyCatalogRequest});
      expect(result.schemaRevision, 1);
      expect(result.generation.edition, 'g1r-steam');
      expect(result.npcs.map((entry) => entry.displayName), [
        'Asghan',
        'Viper',
      ]);
      expect(result.npcs.first.runtimeUniqueName, 'OM_GRD_Asghan_263');
      expect(
        result.npcs.first.authoringQualification,
        AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
      );
      expect(
        result.npcs.first.runtimeQualification,
        AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
      );
      expect(
        result.questParents.single.questClass.runtimeClass,
        'UQuest_SwampCamp_SCCHAPTER2',
      );
      expect(
        result.questCollisionCatalog.status,
        AuthoringStoryCatalogCollisionStatus.inventoryUnavailable,
      );
      expect(result.questCollisionCatalog.blocksDraftCreation, isTrue);
      expect(result.blocksBuild, isTrue);
      expect(() => result.npcs.clear(), throwsUnsupportedError);
      expect(() => result.questParents.clear(), throwsUnsupportedError);
    },
  );

  test(
    'Story catalog build binds exact paths and can feed the pinned reader',
    () async {
      expect(
        _storyCatalogBuildBinding(
          executable: 'A/game.exe',
          shippingCache: 'B/Shipping-G1-Game.cache',
          bindsCache: 'C/Binds.cache',
        ),
        '86c32f29c17846499a62e6acf9778610fe25b445930519e6e055aa427519cb37',
      );
      final rawCatalog = _storyCatalogBuildRaw();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build': _validStoryCatalogBuildResponse(
            catalogJson: rawCatalog,
          ),
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(
            catalogJson: rawCatalog,
          ),
        },
      );
      final ffi = ModFfi(core);

      final built = await ffi.authoringStoryCatalogV1Build(
        executable: _storyCatalogExecutable,
        shippingCache: _storyCatalogShippingCache,
        bindsCache: _storyCatalogBindsCache,
      );
      final selections = await ffi.authoringStoryCatalogV1BuildAndRead(
        executable: _storyCatalogExecutable,
        shippingCache: _storyCatalogShippingCache,
        bindsCache: _storyCatalogBindsCache,
      );

      expect(built.catalogJson, rawCatalog);
      expect(built.generation.edition, 'g1r-steam');
      expect(built.catalogSeal.sha256, List.filled(64, '4').join());
      expect(selections.npcs, hasLength(2));
      expect(core.calls.map((call) => call.command), <String>[
        'authoring_story_catalog_v1_build',
        'authoring_story_catalog_v1_build',
        'authoring_story_catalog_v1_read',
      ]);
      expect(core.calls.first.payload, <String, Object?>{
        'executable': _storyCatalogExecutable,
        'shipping_cache': _storyCatalogShippingCache,
        'binds_cache': _storyCatalogBindsCache,
      });
      expect(core.calls.last.payload, <String, Object?>{
        'catalog_json': rawCatalog,
      });
    },
  );

  test(
    'Story catalog build rejects response confusion and loose JSON',
    () async {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) =>
            response['request_binding_sha256'] = List.filled(64, 'a').join(),
        (response) => response['request_binding_sha256'] =
            _storyCatalogBuildBinding().toUpperCase(),
        (response) => response['catalog_json'] = '${_storyCatalogBuildRaw()}\n',
        (response) =>
            response['catalog_json'] = _storyCatalogBuildRaw().replaceFirst(
              '"format":"story_catalog"',
              '"format":"story_catalog","format":"story_catalog"',
            ),
        (response) =>
            (response['generation'] as Map<String, Object?>)['edition'] =
                'g1r-other',
        (response) =>
            ((response['generation'] as Map<String, Object?>)['executable']
                    as Map<String, Object?>)['byte_len'] =
                1,
        (response) =>
            (response['catalog_seal'] as Map<String, Object?>)['sha256'] =
                List.filled(64, 'f').join(),
        (response) {
          final raw =
              jsonDecode(response['catalog_json'] as String)
                  as Map<String, Object?>;
          (raw['catalog'] as Map<String, Object?>)['extra'] = true;
          response['catalog_json'] = jsonEncode(raw);
        },
        (response) => response['catalog_json'] = '[]',
      ];

      for (final mutate in malformed) {
        final response = _validStoryCatalogBuildResponse();
        mutate(response);
        final core = FakeGoreCoreFfiService(
          responses: {'authoring_story_catalog_v1_build': response},
        );
        await expectLater(
          ModFfi(core).authoringStoryCatalogV1Build(
            executable: _storyCatalogExecutable,
            shippingCache: _storyCatalogShippingCache,
            bindsCache: _storyCatalogBindsCache,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test('Story catalog build bounds paths before FFI', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_story_catalog_v1_build': _validStoryCatalogBuildResponse(),
      },
    );
    final ffi = ModFfi(core);
    for (final executable in <String>[
      '',
      'bad\u0000path',
      List.filled(32 * 1024 + 1, 'x').join(),
      String.fromCharCode(0xd800),
    ]) {
      await expectLater(
        ffi.authoringStoryCatalogV1Build(
          executable: executable,
          shippingCache: _storyCatalogShippingCache,
          bindsCache: _storyCatalogBindsCache,
        ),
        throwsArgumentError,
      );
    }
    expect(core.calls, isEmpty);
  });

  test(
    'Story catalog game-root build is root-bound and feeds the strict reader',
    () async {
      expect(
        _storyCatalogGameRootBinding(),
        '208d76c5754bc4457ea54b30605d1081b21894d3d8ea925c5e925257da370f7b',
      );
      final rawCatalog = _storyCatalogBuildRaw();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build_for_game_root':
              _validStoryCatalogGameRootBuildResponse(catalogJson: rawCatalog),
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(
            catalogJson: rawCatalog,
          ),
        },
      );
      final result = await ModFfi(core)
          .authoringStoryCatalogV1BuildAndReadForGameRoot(
            gameRoot: _storyCatalogGameRoot,
          );

      expect(result.npcs, hasLength(2));
      expect(core.calls.map((call) => call.command), <String>[
        'authoring_story_catalog_v1_build_for_game_root',
        'authoring_story_catalog_v1_read',
      ]);
      expect(core.calls.first.payload, <String, Object?>{
        'game_root': _storyCatalogGameRoot,
      });
      expect(core.calls.last.payload, <String, Object?>{
        'catalog_json': rawCatalog,
      });
    },
  );

  test(
    'Story catalog game-root build rejects wrong binding and bad roots',
    () async {
      final confused = _validStoryCatalogGameRootBuildResponse()
        ..['request_binding_sha256'] = _storyCatalogBuildBinding();
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_story_catalog_v1_build_for_game_root': confused,
            },
          ),
        ).authoringStoryCatalogV1BuildForGameRoot(
          gameRoot: _storyCatalogGameRoot,
        ),
        throwsFormatException,
      );

      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build_for_game_root':
              _validStoryCatalogGameRootBuildResponse(),
        },
      );
      for (final root in <String>[
        '',
        'bad\u0000root',
        List.filled(32 * 1024 + 1, 'x').join(),
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ModFfi(core).authoringStoryCatalogV1BuildForGameRoot(gameRoot: root),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    },
  );

  test(
    'Story catalog DTO rejects unbound, loose, and inconsistent data',
    () async {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) =>
            response['request_catalog_sha256'] = List.filled(64, 'a').join(),
        (response) => response['request_catalog_sha256'] = crypto.sha256
            .convert(utf8.encode(_storyCatalogRequest))
            .toString()
            .toUpperCase(),
        (response) =>
            (response['selections'] as Map<String, Object?>)['extra'] = true,
        (response) =>
            (response['selections']
                    as Map<String, Object?>)['schema_revision'] =
                2,
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final reversed = npcs.reversed.toList();
          npcs.setAll(0, reversed);
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          final second = npcs[1] as Map<String, Object?>;
          (second['character_definition']
                  as Map<String, Object?>)['authoring_selector'] =
              (first['character_definition']
                  as Map<String, Object?>)['authoring_selector'];
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['authoring_selector'] =
              'Catalog_${List.filled(64, '0').join()}';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          final character =
              first['character_definition'] as Map<String, Object?>;
          final ai = first['ai_agent_config'] as Map<String, Object?>;
          final alias = character['authoring_selector'];
          character['authoring_selector'] = ai['authoring_selector'];
          ai['authoring_selector'] = alias;
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['source_catalog_selector'] =
              'script-class:Trusted/UCharacterDefinition_Human_OTHER';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['catalog_layer'] =
              'base-game.g1r.other';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          (npcs[0] as Map<String, Object?>)['runtime_qualification'] =
              'runtime_qualified';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          (npcs[0] as Map<String, Object?>)['display_name'] = ' Asghan';
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final collision =
              selections['quest_collision_catalog'] as Map<String, Object?>;
          collision['source_seal'] = _catalogContentSeal('f', 123394250);
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final collision =
              selections['quest_collision_catalog'] as Map<String, Object?>;
          collision['blocks_draft_creation'] = false;
        },
        (response) =>
            (response['selections'] as Map<String, Object?>)['blocks_build'] =
                false,
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final parents = selections['quest_parents'] as List;
          final questClass =
              (parents.single as Map<String, Object?>)['quest_class']
                  as Map<String, Object?>;
          questClass['source_catalog_selector'] = r'script-class:Bad\Path';
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final generation = selections['generation'] as Map<String, Object?>;
          (generation['executable'] as Map<String, Object?>)['byte_len'] = 1.0;
        },
      ];

      for (final mutate in malformed) {
        final response = _validStoryCatalogResponse();
        mutate(response);
        final core = FakeGoreCoreFfiService(
          responses: {'authoring_story_catalog_v1_read': response},
        );
        await expectLater(
          ModFfi(
            core,
          ).authoringStoryCatalogV1Read(catalogJson: _storyCatalogRequest),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'Story catalog wrapper bounds raw and escaped inputs before FFI',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(),
        },
      );
      final ffi = ModFfi(core);

      await expectLater(
        ffi.authoringStoryCatalogV1Read(
          catalogJson: String.fromCharCodes(Uint8List(16 * 1024 * 1024 + 1)),
        ),
        throwsArgumentError,
      );
      await expectLater(
        ffi.authoringStoryCatalogV1Read(
          catalogJson: String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    },
  );

  test(
    'Story Draft insert preserves both raw strings and parses applied',
    () async {
      final rawProject = _validStoryBaseProjectJson();
      final rawMutation = _validStoryMutationJson();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_project_story_draft_insert_v1':
              _validStoryAppliedResponse(),
        },
      );

      final result = await ModFfi(core).authoringProjectStoryDraftInsertV1(
        projectJson: rawProject,
        mutationJson: rawMutation,
        profile: AuthoringValidationProfile.experimental,
      );

      expect(result, isA<AuthoringStoryDraftInsertApplied>());
      final applied = result as AuthoringStoryDraftInsertApplied;
      expect(applied.projectJson, _validStoryCandidateProjectJson());
      expect(applied.revision, 8);
      expect(applied.draftId, _storyDraftId);
      expect(applied.draftKind, AuthoringStoryDraftKind.npcDraft);
      expect(applied.scriptModuleId, _storyScriptModuleId);
      expect(
        applied.requestBindingSha256,
        _storyRequestBinding(rawProject, rawMutation, 'experimental'),
      );
      expect(applied.blocksBuild, isTrue);
      expect(
        applied.diagnostics.single.code,
        'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
      );
      expect(
        () => applied.diagnostics.add(applied.diagnostics.single),
        throwsUnsupportedError,
      );
      expect(
        core.calls.single.command,
        'authoring_project_story_draft_insert_v1',
      );
      expect(core.calls.single.payload['project_json'], rawProject);
      expect(core.calls.single.payload['mutation_json'], rawMutation);
      expect(core.calls.single.payload['profile'], 'experimental');
    },
  );

  test('Story Draft rejection is typed and cannot carry a candidate', () async {
    final rawProject = _validStoryBaseProjectJson();
    final rawMutation = _validStoryMutationJson(revision: 6);
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_project_story_draft_insert_v1':
            _validStoryRejectedResponse(),
      },
    );
    final result = await ModFfi(core).authoringProjectStoryDraftInsertV1(
      projectJson: rawProject,
      mutationJson: rawMutation,
      profile: AuthoringValidationProfile.production,
    );

    expect(result, isA<AuthoringStoryDraftInsertRejected>());
    final rejected = result as AuthoringStoryDraftInsertRejected;
    expect(rejected.diagnostics.single.code, 'PROJECT_REVISION_CONFLICT');
    expect(
      rejected.requestBindingSha256,
      _storyRequestBinding(rawProject, rawMutation, 'production'),
    );

    final leaked = _validStoryRejectedResponse()
      ..['project_json'] = _validStoryCandidateProjectJson();
    expect(
      () => _decodeStoryResponse(
        leaked,
        mutation: rawMutation,
        profile: AuthoringValidationProfile.production,
      ),
      throwsFormatException,
    );
  });

  test(
    'Story Draft invalid generator input remains a typed rejection',
    () async {
      final rawProject = _validStoryBaseProjectJson();
      final rawMutation = _validStoryMutationJson().replaceFirst(
        'GoreMods.Npcs.GateGuard',
        'module namespace with spaces',
      );
      final response = _validStoryRejectedResponse(
        base: rawProject,
        mutation: rawMutation,
        profile: 'experimental',
      );
      final diagnostic = _authoringDiagnostic(response, 0)
        ..['code'] = 'INVALID_STORY_MUTATION'
        ..['property_path'] = 'draft.input.module_namespace'
        ..['message'] = 'module namespace is invalid';
      expect(diagnostic['blocks_build'], isTrue);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_project_story_draft_insert_v1': response,
        },
      );

      final result = await ModFfi(core).authoringProjectStoryDraftInsertV1(
        projectJson: rawProject,
        mutationJson: rawMutation,
        profile: AuthoringValidationProfile.experimental,
      );

      expect(result, isA<AuthoringStoryDraftInsertRejected>());
      final rejected = result as AuthoringStoryDraftInsertRejected;
      expect(rejected.diagnostics.single.code, 'INVALID_STORY_MUTATION');
      expect(
        rejected.requestBindingSha256,
        _storyRequestBinding(rawProject, rawMutation, 'experimental'),
      );
    },
  );

  test(
    'Story Draft applied DTO binds candidate, IDs, kind, revision, and gate',
    () {
      expect(
        () => _decodeStoryResponse(_validStoryAppliedResponse()),
        returnsNormally,
      );
      final quest =
          _decodeStoryResponse(
                _validStoryAppliedResponse(quest: true),
                quest: true,
              )
              as AuthoringStoryDraftInsertApplied;
      expect(quest.draftKind, AuthoringStoryDraftKind.questDraft);

      final maximumBase = _validStoryBaseProjectJson(
        revision: 0x7ffffffffffffffe,
      );
      final maximumMutation = _validStoryMutationJson(
        revision: 0x7ffffffffffffffe,
      );
      final maximumApplied =
          _decodeStoryResponse(
                _validStoryAppliedResponse(
                  base: maximumBase,
                  mutation: maximumMutation,
                ),
                base: maximumBase,
                mutation: maximumMutation,
              )
              as AuthoringStoryDraftInsertApplied;
      expect(maximumApplied.revision, 0x7fffffffffffffff);

      for (final invalidMutation in <String>[
        _validStoryMutationJson().replaceFirst(
          '"expected_revision":7',
          '"expected_revision":7.0',
        ),
        _validStoryMutationJson().replaceFirst(
          '"expected_revision":7',
          '"expected_revision":9223372036854775807',
        ),
      ]) {
        expect(
          () => _decodeStoryResponse(
            _validStoryAppliedResponse(mutation: invalidMutation),
            mutation: invalidMutation,
          ),
          throwsFormatException,
        );
      }
      final fractionalBase = _validStoryBaseProjectJson().replaceFirst(
        '"revision":7',
        '"revision":7.0',
      );
      final fractionalBaseResponse = _validStoryAppliedResponse();
      fractionalBaseResponse['request_binding_sha256'] = _storyRequestBinding(
        fractionalBase,
        _validStoryMutationJson(),
        'experimental',
      );
      expect(
        () =>
            _decodeStoryResponse(fractionalBaseResponse, base: fractionalBase),
        throwsFormatException,
      );
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) =>
            response['request_binding_sha256'] = List.filled(64, 'f').join(),
        (response) => response['revision'] = 9,
        (response) => response['draft_id'] = _storyScriptModuleId,
        (response) => response['draft_kind'] = 'dialog_line',
        (response) => response['blocks_build'] = false,
        (response) => response['diagnostics'] = <Object?>[],
        (response) =>
            (_authoringDiagnostic(response, 0)['severity'] = 'warning'),
        (response) =>
            (_authoringDiagnostic(response, 0)['entity'] = _storyDraftId),
        (response) =>
            (_authoringDiagnostic(response, 0)['property_path'] = 'revision'),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"schema_revision":2', '"schema_revision":1'),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"revision":8', '"revision":9'),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"kind":"npc_draft"', '"kind":"quest_draft"'),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"authored_runtime_id":"GoreGateGuard"',
              '"authored_runtime_id":"Other"',
            ),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"display_name":"NPC GoreGateGuard"',
              '"display_name":"Other"',
            ),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"revision":0', '"revision":1'),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"unique_name":"GoreGateGuard"',
              '"unique_name":"Other"',
            ),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"display_name":"GoreMods.Npcs.GateGuard"',
              '"display_name":"Other.Module"',
            ),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"module_namespace":"GoreMods.Npcs.GateGuard"',
              '"module_namespace":"Other.Module"',
            ),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"module_relative_path":"GoreMods/Npcs/GateGuard.as"',
              '"module_relative_path":"Other.as"',
            ),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('// generated npc\\n', '// corrupted\\n'),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              List.filled(64, 'a').join(),
              List.filled(64, 'g').join(),
            ),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"authoring":"offline_draft"',
              '"authoring":"qualified"',
            ),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"name":"Story transaction"', '"name":"Other"'),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"byte_len":1000000', '"byte_len":1000000.0'),
        (response) =>
            response['project_json'] = _validStoryCandidateProjectJson()
                .replaceFirst('"byte_len":20000', '"byte_len":20000.0'),
        (response) => response['project_json'] =
            _validStoryCandidateProjectJson().replaceFirst(
              '"expected_kind":"npc_draft"',
              '"expected_kind":"quest_draft"',
            ),
        (response) =>
            response['project_json'] = ' ${_validStoryCandidateProjectJson()}',
        (response) {
          final candidate =
              (jsonDecode(response['project_json']! as String) as Map)
                  .cast<String, Object?>();
          (candidate['entities']
                  as Map<String, Object?>)['22222222222222222222222222222222'] =
              <String, Object?>{'unexpected': true};
          response['project_json'] = jsonEncode(candidate);
        },
      ];
      for (final mutate in malformed) {
        final response = _validStoryAppliedResponse();
        mutate(response);
        expect(() => _decodeStoryResponse(response), throwsFormatException);
      }

      final emptyRejection = _validStoryRejectedResponse()
        ..['diagnostics'] = <Object?>[];
      expect(
        () => _decodeStoryResponse(
          emptyRejection,
          mutation: _validStoryMutationJson(revision: 6),
          profile: AuthoringValidationProfile.production,
        ),
        throwsFormatException,
      );

      final warningRejection = _validStoryRejectedResponse();
      _authoringDiagnostic(warningRejection, 0)['severity'] = 'warning';
      expect(
        () => _decodeStoryResponse(
          warningRejection,
          mutation: _validStoryMutationJson(revision: 6),
          profile: AuthoringValidationProfile.production,
        ),
        throwsFormatException,
      );

      final baseMap = (jsonDecode(_validStoryBaseProjectJson()) as Map)
          .cast<String, Object?>();
      (baseMap['entities']
              as Map<String, Object?>)['22222222222222222222222222222222'] =
          <String, Object?>{'preserved': true};
      final baseWithEntity = jsonEncode(baseMap);
      expect(
        () => _decodeStoryResponse(
          _validStoryAppliedResponse(base: baseWithEntity),
          base: baseWithEntity,
        ),
        throwsFormatException,
      );
    },
  );

  test('Story Draft wrapper bounds raw and escaped inputs before FFI', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_project_story_draft_insert_v1': _validStoryAppliedResponse(),
      },
    );
    final ffi = ModFfi(core);

    await expectLater(
      ffi.authoringProjectStoryDraftInsertV1(
        projectJson: String.fromCharCodes(Uint8List(16 * 1024 * 1024 + 1)),
        mutationJson: '{}',
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringProjectStoryDraftInsertV1(
        projectJson: '{}',
        mutationJson: String.fromCharCodes(Uint8List(20 * 1024 * 1024 + 1)),
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringProjectStoryDraftInsertV1(
        // Eleven MiB of NUL is within the raw project limit but JSON escaping would exceed the
        // bounded 64 MiB native transport envelope.
        projectJson: String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
        mutationJson: '{}',
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    expect(core.calls, isEmpty);
  });

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

  test(
    'document working-store wrappers preserve raw bytes and accept revision 2',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_open_document': _validRevision2StoreOpenedResponse(),
          'authoring_store_prepare_document_checkpoint':
              _validRevision2CheckpointPreparationResponse(),
          'authoring_store_open_head_bytes_document':
              _validRevision2StoreOpenedResponse(),
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(
        _validWorkingHeadJson(),
      );
      const rawProject = '{"schema_revision":2,"revision":0,"revision":1}';

      final opened = await ffi.authoringStoreOpenDocument(
        root: r'C:\Mods\Story.goreproj',
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      );
      final prepared = await ffi.authoringStorePrepareDocumentCheckpoint(
        root: r'C:\Mods\Story.goreproj',
        expectedHead: head,
        projectJson: rawProject,
        profile: AuthoringValidationProfile.experimental,
      );
      final candidate = await ffi.authoringStoreOpenHeadBytesDocument(
        root: r'C:\Mods\Story.goreproj',
        head: head,
        verification: AuthoringAssetVerification.structural,
        profile: AuthoringValidationProfile.experimental,
      );

      expect(opened.projectJson, _validCanonicalRevision2ProjectJson());
      expect(opened.blocksBuild, isTrue);
      expect(
        opened.diagnostics.single.code,
        'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
      );
      expect(prepared.blocksBuild, isTrue);
      expect(candidate.projectJson, _validCanonicalRevision2ProjectJson());
      expect(core.calls.map((call) => call.command), <String>[
        'authoring_store_open_document',
        'authoring_store_prepare_document_checkpoint',
        'authoring_store_open_head_bytes_document',
      ]);
      expect(core.calls[1].payload['project_json'], rawProject);
      expect(
        core.calls[1].payload['expected_head_json'],
        _validWorkingHeadJson(),
      );
      expect(core.calls[2].payload['head_json'], _validWorkingHeadJson());
    },
  );

  test('revision-2 store response requires its blocking combined gate', () {
    expect(
      () => AuthoringStoreOpenedResult.fromJson(
        _validRevision2StoreOpenedResponse(),
      ),
      returnsNormally,
    );

    final missingDiagnostic = _validRevision2StoreOpenedResponse()
      ..['diagnostics'] = <Object?>[]
      ..['blocks_build'] = false;
    expect(
      () => AuthoringStoreOpenedResult.fromJson(missingDiagnostic),
      throwsFormatException,
    );

    final nonblockingDiagnostic = _validRevision2StoreOpenedResponse();
    ((nonblockingDiagnostic['diagnostics'] as List<Object?>).single
            as Map<String, Object?>)['blocks_build'] =
        false;
    nonblockingDiagnostic['blocks_build'] = false;
    expect(
      () => AuthoringStoreOpenedResult.fromJson(nonblockingDiagnostic),
      throwsFormatException,
    );

    final falseTopLevel = _validRevision2StoreOpenedResponse()
      ..['blocks_build'] = false;
    expect(
      () => AuthoringStoreOpenedResult.fromJson(falseTopLevel),
      throwsFormatException,
    );
  });

  test('working-head DTO accepts only exact canonical bounded bytes', () {
    final valid = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson(),
    );
    expect(valid.snapshotByteLength, 321);
    expect(valid.snapshotSha256, List.filled(64, 'a').join());
    final maximumRevision3 = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson().replaceFirst(
        '"byte_len":321',
        '"byte_len":${17 * 1024 * 1024}',
      ),
    );
    expect(maximumRevision3.snapshotByteLength, 17 * 1024 * 1024);

    final malformed = <String>[
      '{}',
      ' ${_validWorkingHeadJson()}',
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":2',
      ),
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":1.0',
      ),
      _validWorkingHeadJson().replaceFirst('"byte_len":321', '"byte_len":0'),
      _validWorkingHeadJson().replaceFirst(
        '"byte_len":321',
        '"byte_len":${17 * 1024 * 1024 + 1}',
      ),
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
      (response) => response['project_json'] = _validCanonicalProjectJson()
          .replaceFirst('"format":2', '"format":3'),
      (response) => response['project_json'] = _validCanonicalProjectJson()
          .replaceFirst('"schema_revision":1', '"schema_revision":3'),
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
      ffi.authoringStorePrepareDocumentCheckpoint(
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
