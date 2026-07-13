import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_workspace_bootstrap.dart';
import 'package:gore_mod/story/domain/story_workspace_launcher.dart';
import 'package:path/path.dart' as p;

const _projectId = '01010101010101010101010101010101';

typedef _GameFixture = ({
  Directory root,
  Directory g1r,
  File executable,
  File live,
  File binds,
});

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp('gore_story_launcher_');
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test('resolves only the exact root, G1R directory, and executable', () async {
    final game = await _createGame(fixture);
    final launcher = StoryWorkspaceLauncher(ModFfi(_LauncherCore()));

    for (final configured in <String>[
      game.root.path,
      game.g1r.path,
      game.executable.path,
    ]) {
      final inputs = await launcher.resolveGameInputs(configured);
      expect(inputs.gameRoot, p.normalize(p.absolute(game.root.path)));
      expect(inputs.executable, game.executable.path);
    }
  });

  test(
    'missing and ambiguous layouts fail with stable path-free errors',
    () async {
      final game = await _createGame(fixture);
      final launcher = StoryWorkspaceLauncher(ModFfi(_LauncherCore()));
      await game.executable.delete();
      final missing = await _launchFailure(
        launcher.resolveGameInputs(game.root.path),
      );
      expect(missing.code, StoryWorkspaceLaunchError.missingExecutable);
      expect(missing.message, isNot(contains(fixture.path)));

      final ambiguous = Directory(p.join(fixture.path, 'ambiguous', 'G1R'));
      await ambiguous.create(recursive: true);
      await Directory(p.join(ambiguous.path, 'G1R')).create();
      final ambiguousFailure = await _launchFailure(
        launcher.resolveGameInputs(ambiguous.path),
      );
      expect(
        ambiguousFailure.code,
        StoryWorkspaceLaunchError.ambiguousGameRoot,
      );
      expect(ambiguousFailure.message, isNot(contains(fixture.path)));
    },
  );

  test(
    'divergent deployed backup is delegated only to the native root command',
    () async {
      final game = await _createGame(fixture);
      final workspace = await _workspace(fixture);
      final backup = File('${game.live.path}.gore-bak');
      await backup.writeAsBytes(const <int>[1, 2, 3]);
      await game.live.writeAsBytes(const <int>[9, 8, 7]);
      await _writeValidDeployRecord(game: game, backup: backup);
      final core = _LauncherCore(useBackupShipping: true);

      final launched = await StoryWorkspaceLauncher(ModFfi(core)).create(
        configuredGamePath: game.root.path,
        workspaceRoot: workspace,
        metadata: _metadata('Native pristine selection'),
        projectIdSource: const _FixedProjectIdSource(_projectId),
      );
      expect(core.catalogBuildPayloads.single, <String, Object?>{
        'game_root': game.root.path,
      });
      expect(core.npcCatalogBuildPayloads.single, <String, Object?>{
        'game_root': game.root.path,
      });
      expect(core.selectedShippingPath, backup.path);
      await launched.close();
    },
  );

  test('layout resolution never inspects Shipping, Binds, or backup', () async {
    final game = await _createGame(fixture);
    await game.live.delete();
    await game.binds.delete();
    await File('${game.live.path}.gore-bak').writeAsBytes(const <int>[9]);

    final inputs = await StoryWorkspaceLauncher(
      ModFfi(_LauncherCore()),
    ).resolveGameInputs(game.root.path);
    expect(inputs.gameRoot, game.root.path);
  });

  test(
    'native hotfix/pristine failure stays path-free and acquires no lock',
    () async {
      final game = await _createGame(fixture);
      await game.live.writeAsBytes(const <int>[7, 7]);
      final workspace = await _workspace(fixture);
      final launcher = StoryWorkspaceLauncher(
        ModFfi(_LauncherCore(failCatalog: true)),
      );

      final failure = await _launchFailure(
        launcher.create(
          configuredGamePath: game.root.path,
          workspaceRoot: workspace,
          metadata: _metadata('Catalog failure'),
          projectIdSource: const _FixedProjectIdSource(_projectId),
        ),
      );
      expect(failure.code, StoryWorkspaceLaunchError.catalogBuildFailed);
      expect(failure.message, isNot(contains(game.root.path)));
      expect(failure.message, isNot(contains('secret native parser detail')));
      expect(
        await FileSystemEntity.type(
          p.join(workspace.path, '.gore', 'session.lock'),
          followLinks: false,
        ),
        FileSystemEntityType.notFound,
      );
    },
  );

  test(
    'native NPC catalog failure is path-free and acquires no lock',
    () async {
      final game = await _createGame(fixture);
      final workspace = await _workspace(fixture);
      final launcher = StoryWorkspaceLauncher(
        ModFfi(_LauncherCore(failNpcCatalog: true)),
      );

      final failure = await _launchFailure(
        launcher.create(
          configuredGamePath: game.root.path,
          workspaceRoot: workspace,
          metadata: _metadata('NPC catalog failure'),
          projectIdSource: const _FixedProjectIdSource(_projectId),
        ),
      );
      expect(failure.code, StoryWorkspaceLaunchError.npcCatalogBuildFailed);
      expect(failure.message, isNot(contains(game.root.path)));
      expect(failure.message, isNot(contains('secret NPC parser detail')));
      expect(
        await FileSystemEntity.type(
          p.join(workspace.path, '.gore', 'session.lock'),
          followLinks: false,
        ),
        FileSystemEntityType.notFound,
      );
    },
  );

  test(
    'create and open use only native game roots and production sessions',
    () async {
      final game = await _createGame(fixture);
      final workspace = await _workspace(fixture);
      final core = _LauncherCore();
      final launcher = StoryWorkspaceLauncher(ModFfi(core));

      final created = await launcher.create(
        configuredGamePath: game.executable.path,
        workspaceRoot: workspace,
        metadata: _metadata('Launcher test'),
        projectIdSource: const _FixedProjectIdSource(_projectId),
      );
      expect(
        created.workspace.session.profile,
        AuthoringValidationProfile.production,
      );
      expect(created.inputs.executable, game.executable.path);
      expect(core.catalogBuildPayloads.single, <String, Object?>{
        'game_root': game.root.path,
      });
      expect(core.npcCatalogBuildPayloads.single, <String, Object?>{
        'game_root': game.root.path,
      });
      expect(created.workspace.adapter.npcArchetypeIndex?.search('').length, 2);
      final expectedExecutableSeal = await _contentSeal(game.executable.path);
      expect(
        jsonDecode(created.workspace.session.projectJson),
        containsPair('target', <String, Object?>{
          'executable': expectedExecutableSeal,
        }),
      );
      await created.close();

      final opened = await launcher.open(
        configuredGamePath: game.root.path,
        workspaceRoot: workspace,
      );
      expect(
        opened.workspace.session.profile,
        AuthoringValidationProfile.production,
      );
      await opened.close();
      expect(core.catalogBuildPayloads, hasLength(2));
      expect(core.npcCatalogBuildPayloads, hasLength(2));
      expect(
        core.calls.map((call) => call.payload['profile']).whereType<String>(),
        everyElement('production'),
      );
    },
  );

  test('bootstrap failure is stable and releases the managed lock', () async {
    final game = await _createGame(fixture);
    final workspace = await _workspace(fixture);
    final core = _LauncherCore(failOpen: true);
    final launcher = StoryWorkspaceLauncher(ModFfi(core));

    final failure = await _launchFailure(
      launcher.create(
        configuredGamePath: game.root.path,
        workspaceRoot: workspace,
        metadata: _metadata('Failure'),
        projectIdSource: const _FixedProjectIdSource(_projectId),
      ),
    );
    expect(failure.code, StoryWorkspaceLaunchError.workspaceBootstrapFailed);
    expect(failure.message, isNot(contains(workspace.path)));

    core.failOpen = false;
    final retried = await launcher.create(
      configuredGamePath: game.root.path,
      workspaceRoot: workspace,
      metadata: _metadata('Failure retry'),
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );
    await retried.close();
  });

  test(
    'linked configured path or ancestor is rejected when supported',
    () async {
      final game = await _createGame(fixture);
      final alias = Link(p.join(fixture.path, 'linked-game'));
      try {
        await alias.create(game.root.path);
      } on FileSystemException {
        return;
      }
      final configuredThroughLink = p.join(
        alias.path,
        'G1R',
        'Binaries',
        'Win64',
        'G1R-Win64-Shipping.exe',
      );

      final failure = await _launchFailure(
        StoryWorkspaceLauncher(
          ModFfi(_LauncherCore()),
        ).resolveGameInputs(configuredThroughLink),
      );
      expect(failure.code, StoryWorkspaceLaunchError.unsafeFileType);
      expect(failure.message, isNot(contains(alias.path)));
    },
  );
}

Future<StoryWorkspaceLaunchException> _launchFailure(
  Future<Object?> operation,
) async {
  try {
    await operation;
  } on StoryWorkspaceLaunchException catch (error) {
    return error;
  }
  fail('expected StoryWorkspaceLaunchException');
}

StoryProjectMetadata _metadata(String name) => StoryProjectMetadata(
  name: name,
  version: '0.1.0',
  author: 'tests',
  authoringLocales: const <String>['de'],
);

Future<Directory> _workspace(Directory fixture) async =>
    Directory(p.join(fixture.path, 'workspace'))..createSync();

Future<_GameFixture> _createGame(Directory fixture) async {
  final root = Directory(p.join(fixture.path, 'game'));
  final g1r = Directory(p.join(root.path, 'G1R'));
  final executable = File(
    p.join(g1r.path, 'Binaries', 'Win64', 'G1R-Win64-Shipping.exe'),
  );
  final live = File(
    p.join(g1r.path, 'Script', 'PrecompiledScript_Shipping.Cache'),
  );
  final binds = File(p.join(g1r.path, 'Script', 'Binds.Cache'));
  await executable.parent.create(recursive: true);
  await live.parent.create(recursive: true);
  await executable.writeAsBytes(const <int>[1]);
  await live.writeAsBytes(const <int>[2]);
  await binds.writeAsBytes(const <int>[3]);
  return (
    root: root,
    g1r: g1r,
    executable: executable,
    live: live,
    binds: binds,
  );
}

/// This is a complete current-format gore-mod record for the one in-place
/// script-cache write, including authenticated backup and deployed identities. Dart never parses
/// it; `_LauncherCore` represents the native gore-mod trust boundary selecting the backup.
Future<void> _writeValidDeployRecord({
  required _GameFixture game,
  required File backup,
}) async {
  final backupBytes = await backup.readAsBytes();
  final liveBytes = await game.live.readAsBytes();
  await File(p.join(game.root.path, 'gore-mod.deployed.json')).writeAsString(
    jsonEncode(<String, Object?>{
      'mod_name': 'launcher-test',
      'ue4ss_mod_dir': null,
      'backups': <Object?>[
        <Object?>[game.live.path, backup.path, true],
      ],
      'deployed_hashes': <String, Object?>{game.live.path: _fnv(liveBytes)},
      'backup_hashes': <String, Object?>{
        backup.path: 'sha256:${crypto.sha256.convert(backupBytes)}',
      },
      'phase': 'applied',
    }),
  );
}

String _fnv(List<int> bytes) {
  var value = 0xcbf29ce484222325;
  for (final byte in bytes) {
    value ^= byte;
    value = (value * 0x100000001b3) & 0xffffffffffffffff;
  }
  return value.toRadixString(16).padLeft(16, '0');
}

final class _FixedProjectIdSource implements StoryProjectIdSource {
  const _FixedProjectIdSource(this.value);

  final String value;

  @override
  String nextProjectId() => value;
}

final class _LauncherCore implements GoreCoreFfiService {
  _LauncherCore({
    this.failOpen = false,
    this.failCatalog = false,
    this.failNpcCatalog = false,
    this.useBackupShipping = false,
  });

  bool failOpen;
  final bool failCatalog;
  final bool failNpcCatalog;
  final bool useBackupShipping;
  String? selectedShippingPath;
  final Map<String, String> _projectsByHead = <String, String>{};
  final List<({String command, Map<String, Object?> payload})> calls = [];
  final List<Map<String, Object?>> catalogBuildPayloads = [];
  final List<Map<String, Object?>> npcCatalogBuildPayloads = [];
  Map<String, Object?>? _generation;
  String? _catalogJson;

  @override
  String get description => 'Story launcher test core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const <String, Object?>{},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'authoring_story_catalog_v1_build_for_game_root':
        if (failCatalog) return _failure('secret native parser detail');
        return _buildCatalog(payload);
      case 'authoring_story_catalog_v1_read':
        return _readCatalog(payload);
      case 'authoring_npc_archetype_catalog_v1_build_for_game_root':
        if (failNpcCatalog) return _failure('secret NPC parser detail');
        return _buildNpcCatalog(payload);
      case 'authoring_store_prepare_document_checkpoint':
        return _prepare(payload);
      case 'authoring_store_open_head_bytes_document':
        if (failOpen) return _failure('injected open failure');
        return _openHead(payload['head_json']! as String);
      case 'authoring_store_open_document':
        if (failOpen) return _failure('injected open failure');
        final root = payload['root']! as String;
        final rawHead = await File(
          p.join(root, 'gore-project.json'),
        ).readAsString();
        return _openHead(rawHead);
      default:
        return _failure('unexpected command');
    }
  }

  Future<Map<String, Object?>> _buildCatalog(
    Map<String, Object?> payload,
  ) async {
    catalogBuildPayloads.add(Map<String, Object?>.from(payload));
    final root = payload['game_root']! as String;
    final g1r = p.join(root, 'G1R');
    final executable = p.join(
      g1r,
      'Binaries',
      'Win64',
      'G1R-Win64-Shipping.exe',
    );
    final live = p.join(g1r, 'Script', 'PrecompiledScript_Shipping.Cache');
    final shipping = useBackupShipping ? '$live.gore-bak' : live;
    selectedShippingPath = shipping;
    final binds = p.join(g1r, 'Script', 'Binds.Cache');
    final generation = <String, Object?>{
      'edition': 'g1r-steam',
      'executable': await _contentSeal(executable),
      'shipping_cache': await _contentSeal(shipping),
      'binds_cache': await _contentSeal(binds),
    };
    final catalogJson = _buildCatalogJson(generation);
    _generation = generation;
    _catalogJson = catalogJson;
    return <String, Object?>{
      'ok': true,
      'request_binding_sha256': _catalogGameRootBinding(root),
      'catalog_json': catalogJson,
      'generation': generation,
      'catalog_seal': _fixedSeal('4', 5611),
    };
  }

  Future<Map<String, Object?>> _readCatalog(
    Map<String, Object?> payload,
  ) async {
    final catalogJson = _catalogJson!;
    if (payload['catalog_json'] != catalogJson) {
      return _failure('catalog replay mismatch');
    }
    return _catalogResponse(catalogJson, _generation!);
  }

  Future<Map<String, Object?>> _buildNpcCatalog(
    Map<String, Object?> payload,
  ) async {
    npcCatalogBuildPayloads.add(Map<String, Object?>.from(payload));
    final root = payload['game_root']! as String;
    final g1r = p.join(root, 'G1R');
    final executable = p.join(
      g1r,
      'Binaries',
      'Win64',
      'G1R-Win64-Shipping.exe',
    );
    final live = p.join(g1r, 'Script', 'PrecompiledScript_Shipping.Cache');
    final shipping = useBackupShipping ? '$live.gore-bak' : live;
    final binds = p.join(g1r, 'Script', 'Binds.Cache');
    final generation = <String, Object?>{
      'edition': 'g1r-steam',
      'executable': await _contentSeal(executable),
      'shipping_cache': await _contentSeal(shipping),
      'binds_cache': await _contentSeal(binds),
    };
    final sourceIdentity = <String, Object?>{
      'shipping_cache': generation['shipping_cache'],
      'binds_cache': generation['binds_cache'],
    };
    final source = <String, Object?>{
      ...sourceIdentity,
      'source_pair_seal': _jsonSeal(sourceIdentity),
    };
    final records = <Object?>[
      _npcArchetype(viper: false),
      _npcArchetype(viper: true),
    ];
    final npcPayload = <String, Object?>{
      'extractor_records_sha256': List<String>.filled(64, '7').join(),
      'records': records,
      'rejections': <Object?>[],
    };
    final catalog = <String, Object?>{
      'generation': generation,
      'story_catalog_seal': _fixedSeal('4', 5611),
      'qualification': _npcQualification(),
      'source': source,
      'payload': npcPayload,
      'payload_seal': _jsonSeal(npcPayload),
    };
    final artifact = <String, Object?>{
      'format': 'npc_archetype_catalog',
      'schema_revision': 1,
      'catalog': catalog,
      'catalog_seal': _jsonSeal(catalog),
    };
    return <String, Object?>{
      'ok': true,
      'request_binding_sha256': _npcCatalogGameRootBinding(root),
      'catalog_json': jsonEncode(artifact),
      'generation': generation,
      'catalog_seal': artifact['catalog_seal'],
      'source': source,
      'payload_seal': catalog['payload_seal'],
      'record_count': records.length,
      'rejection_count': 0,
      'qualification': catalog['qualification'],
    };
  }

  Future<Map<String, Object?>> _prepare(Map<String, Object?> payload) async {
    final root = payload['root']! as String;
    final expected = payload['expected_head_json'] as String?;
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expected) return _failure('head conflict');
    final projectJson = payload['project_json']! as String;
    final headJson = jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{
        'byte_len': utf8.encode(projectJson).length,
        'sha256': crypto.sha256.convert(utf8.encode(projectJson)).toString(),
      },
    });
    _projectsByHead[headJson] = projectJson;
    return <String, Object?>{
      'ok': true,
      'head_json': headJson,
      'diagnostics': <Object?>[_combinedDiagnostic()],
      'blocks_build': true,
    };
  }

  Map<String, Object?> _openHead(String headJson) => <String, Object?>{
    'ok': true,
    'head_json': headJson,
    'project_json': _projectsByHead[headJson]!,
    'diagnostics': <Object?>[_combinedDiagnostic()],
    'blocks_build': true,
  };

  Map<String, Object?> _failure(String message) => <String, Object?>{
    'ok': false,
    'error': <String, Object?>{'code': 'TEST_FAILURE', 'message': message},
  };
}

Future<Map<String, Object?>> _contentSeal(String path) async {
  final bytes = await File(path).readAsBytes();
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

String _buildCatalogJson(Map<String, Object?> generation) =>
    jsonEncode(<String, Object?>{
      'format': 'story_catalog',
      'schema_revision': 1,
      'catalog': <String, Object?>{
        'generation': generation,
        'record_set_id': 'g1r-steam-1.0.3-curated-story-v1',
        'record_set_seal': _fixedSeal('5', 5499),
        'npcs': <Object?>[],
        'quest_parents': <Object?>[],
      },
      'catalog_seal': _fixedSeal('4', 5611),
    });

String _catalogGameRootBinding(String root) {
  final encoded = utf8.encode(root);
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

String _npcCatalogGameRootBinding(String root) {
  final encoded = utf8.encode(root);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(
      'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000',
    ),
    ...length,
    ...encoded,
  ]).toString();
}

Map<String, Object?> _catalogResponse(
  String catalogJson,
  Map<String, Object?> generation,
) => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': generation,
    'catalog_seal': _fixedSeal('4', 5611),
    'npcs': <Object?>[_npc(viper: false), _npc(viper: true)],
    'quest_parents': <Object?>[_questParent()],
    'quest_collision_catalog': <String, Object?>{
      'status': 'inventory_unavailable',
      'catalog_layer': 'resolved-loadout.scripts.v1',
      'source_seal': generation['shipping_cache'],
      'blocks_draft_creation': true,
    },
    'blocks_build': true,
  },
};

Map<String, Object?> _combinedDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'entity': null,
  'property_path': 'schema_revision',
  'message': 'combined validation unavailable',
  'related_entities': <Object?>[],
  'blocks_build': true,
};

Map<String, Object?> _npc({required bool viper}) {
  final catalogId = viper
      ? 'g1r:npc:om_stt_viper_302'
      : 'g1r:npc:om_grd_asghan_263';
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final character = _classSelection(
    catalogId,
    'character_definition',
    'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _classSelection(
      catalogId,
      'ai_agent_config',
      'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _classSelection(
      catalogId,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _selectorAlias(catalogId, 'quest_giver'),
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

Map<String, Object?> _npcArchetype({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final spawn = 'USpawnAIAgentDefinition_$runtime';
  final ai = 'UAIAgentConfig_Human_$runtime';
  final character = 'UCharacterDefinition_Human_$runtime';
  final actor = viper ? 'BP_Viper' : 'BP_Asghan';
  return <String, Object?>{
    'spawn': _npcArchetypeClass(spawn, 'USpawnAIAgentDefinition', 'c'),
    'ai_config': _npcArchetypeClass(ai, 'UAIAgentConfig', 'b'),
    'character_definition': _npcArchetypeClass(
      character,
      'UCharacterDefinition',
      'a',
    ),
    'actor_blueprint': actor,
    'blueprint_family': 'human_base',
    'spawn_ai_edge': _npcArchetypeEdge(spawn, 'AIAgentConfigClass', ai, '1'),
    'spawn_blueprint_edge': _npcArchetypeEdge(
      spawn,
      'AIAgentCharacterClass',
      actor,
      '2',
    ),
    'ai_character_edge': _npcArchetypeEdge(
      ai,
      'm_CharacterDefinition',
      character,
      '3',
    ),
    'evidence_sha256': List<String>.filled(64, '8').join(),
  };
}

Map<String, Object?> _npcArchetypeClass(
  String name,
  String parent,
  String sealByte,
) => <String, Object?>{
  'class_name': name,
  'super_class': parent,
  'module_name': 'World',
  'relative_path': 'World/$name.as',
  'source_seal': _fixedSeal(sealByte, 100),
};

Map<String, Object?> _npcArchetypeEdge(
  String owner,
  String field,
  String assigned,
  String sealByte,
) => <String, Object?>{
  'owner_class': owner,
  'field_name': field,
  'assigned_value': assigned,
  'instruction_offset_dwords': 1,
  'init_defaults_bytecode_seal': _fixedSeal(sealByte, 20),
  'evidence_sha256': List<String>.filled(64, sealByte).join(),
};

Map<String, Object?> _npcQualification() => <String, Object?>{
  'linkage': 'sealed_linkage_verified',
  'runtime': 'runtime_unqualified',
  'build': 'not_supported',
  'deploy': 'not_supported',
  'publication': 'not_supported',
};

Map<String, Object?> _jsonSeal(Map<String, Object?> value) {
  final bytes = utf8.encode(jsonEncode(value));
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

Map<String, Object?> _questParent() => <String, Object?>{
  'catalog_id': 'g1r:quest-parent:swampcamp_scchapter2',
  'display_name': 'Swamp Camp Chapter 2',
  'quest_class': _classSelection(
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
};

Map<String, Object?> _classSelection(
  String catalogId,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _selectorAlias(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _fixedSeal(sealByte, 100),
};

String _selectorAlias(String catalogId, String role) {
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

Map<String, Object?> _fixedSeal(String byte, int byteLength) =>
    <String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, byte).join(),
    };
