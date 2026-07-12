import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/story/domain/story_workspace_bootstrap.dart';
import 'package:path/path.dart' as p;

const _catalogJson = '{"format":"story_catalog"}';
const _projectId = '01010101010101010101010101010101';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp('gore_story_bootstrap_');
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test('create, close, open, and reopen exact empty revision 2', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final core = _BootstrapCore();
    final ffi = ModFfi(core);
    final catalog = await _catalogSelections();
    final metadata = StoryProjectMetadata(
      name: 'My Story Mod',
      version: '0.1.0',
      author: 'Daniel',
      authoringLocales: const <String>['pt-BR', 'de'],
    );

    final created = await StoryWorkspaceBootstrap.create(
      root: root,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.production,
      metadata: metadata,
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );

    final expected = _expectedProjectJson(
      projectId: _projectId,
      metadata: metadata,
      executableByte: '1',
    );
    expect(created.session.projectJson, expected);
    expect(created.session.profile, AuthoringValidationProfile.production);
    expect(created.controller.current.revision, 0);
    expect(created.controller.current.drafts, isEmpty);
    expect(created.controller.current.blocksBuild, isTrue);
    expect(created.adapter.npcChoices, hasLength(2));
    expect(await created.session.headFile.exists(), isTrue);
    final publishedHead = created.session.head.canonicalJson;
    await created.close();
    await created.close();
    expect(created.isClosed, isTrue);

    final opened = await StoryWorkspaceBootstrap.open(
      root: root,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.production,
    );
    expect(opened.session.head.canonicalJson, publishedHead);
    expect(opened.session.projectJson, expected);
    expect(opened.controller.current.drafts, isEmpty);
    await opened.close();

    final reopened = await StoryWorkspaceBootstrap.open(
      root: root,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.production,
    );
    expect(reopened.session.projectJson, expected);
    await reopened.close();
    expect(
      core.calls
          .where((call) => call.payload.containsKey('verification'))
          .map((call) => call.payload['verification']),
      everyElement('full'),
    );
    expect(
      core.calls.map((call) => call.payload['profile']).whereType<String>(),
      everyElement('production'),
    );
  });

  test('same fixed inputs produce byte-identical canonical projects', () async {
    final rootA = Directory(p.join(fixture.path, 'a'));
    final rootB = Directory(p.join(fixture.path, 'b'));
    await rootA.create();
    await rootB.create();
    final core = _BootstrapCore();
    final ffi = ModFfi(core);
    final catalog = await _catalogSelections();
    final firstMetadata = StoryProjectMetadata(
      name: 'Deterministic',
      version: '1.0.0',
      author: 'tests',
      authoringLocales: const <String>['pt-BR', 'de'],
    );
    final secondMetadata = StoryProjectMetadata(
      name: 'Deterministic',
      version: '1.0.0',
      author: 'tests',
      authoringLocales: const <String>['de', 'pt-BR'],
    );

    final first = await StoryWorkspaceBootstrap.create(
      root: rootA,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.experimental,
      metadata: firstMetadata,
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );
    final second = await StoryWorkspaceBootstrap.create(
      root: rootB,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.experimental,
      metadata: secondMetadata,
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );

    expect(first.session.projectJson, second.session.projectJson);
    expect(
      first.session.projectJson,
      _expectedProjectJson(
        projectId: _projectId,
        metadata: firstMetadata,
        executableByte: '1',
      ),
    );
    await first.close();
    await second.close();
  });

  test('target mismatch closes and releases the managed lock', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final core = _BootstrapCore();
    final ffi = ModFfi(core);
    final correctCatalog = await _catalogSelections();
    final wrongCatalog = await _catalogSelections(executableByte: '9');
    final created = await StoryWorkspaceBootstrap.create(
      root: root,
      ffi: ffi,
      catalogSelections: correctCatalog,
      profile: AuthoringValidationProfile.experimental,
      metadata: StoryProjectMetadata(
        name: 'Mismatch',
        version: '1',
        author: 'tests',
      ),
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );
    await created.close();

    await expectLater(
      StoryWorkspaceBootstrap.open(
        root: root,
        ffi: ffi,
        catalogSelections: wrongCatalog,
        profile: AuthoringValidationProfile.experimental,
      ),
      throwsA(isA<StoryWorkspaceBootstrapException>()),
    );

    final reopened = await StoryWorkspaceBootstrap.open(
      root: root,
      ffi: ffi,
      catalogSelections: correctCatalog,
      profile: AuthoringValidationProfile.experimental,
    );
    expect(reopened.session.projectJson, isNotEmpty);
    await reopened.close();
  });

  test('create refuses an existing head and still releases its lock', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final core = _BootstrapCore();
    final ffi = ModFfi(core);
    final catalog = await _catalogSelections();
    final metadata = StoryProjectMetadata(
      name: 'Existing',
      version: '1',
      author: 'tests',
    );
    final created = await StoryWorkspaceBootstrap.create(
      root: root,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.experimental,
      metadata: metadata,
      projectIdSource: const _FixedProjectIdSource(_projectId),
    );
    await created.close();

    await expectLater(
      StoryWorkspaceBootstrap.create(
        root: root,
        ffi: ffi,
        catalogSelections: catalog,
        profile: AuthoringValidationProfile.experimental,
        metadata: metadata,
        projectIdSource: const _FixedProjectIdSource(_projectId),
      ),
      throwsA(isA<ManagedProjectAlreadyInitializedException>()),
    );
    final opened = await StoryWorkspaceBootstrap.open(
      root: root,
      ffi: ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.experimental,
    );
    await opened.close();
  });

  test(
    'invalid metadata, locales, and injected ProjectIds fail closed',
    () async {
      expect(
        () => StoryProjectMetadata(name: '', version: '1', author: 'tests'),
        throwsFormatException,
      );
      expect(
        () => StoryProjectMetadata(
          name: List<String>.filled(257, 'x').join(),
          version: '1',
          author: 'tests',
        ),
        throwsFormatException,
      );
      expect(
        () => StoryProjectMetadata(
          name: 'Bad\nName',
          version: '1',
          author: 'tests',
        ),
        throwsFormatException,
      );
      for (final locales in <List<String>>[
        <String>['PT-br'],
        <String>['en-us'],
        <String>['de', 'de'],
        <String>[for (var index = 0; index < 65; index++) 'de-$index'],
      ]) {
        expect(
          () => StoryProjectMetadata(
            name: 'Locales',
            version: '1',
            author: 'tests',
            authoringLocales: locales,
          ),
          throwsFormatException,
        );
      }

      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final ffi = ModFfi(_BootstrapCore());
      final catalog = await _catalogSelections();
      for (final projectId in <String>[
        '00000000000000000000000000000000',
        '0101010101010101010101010101010G',
        '01',
      ]) {
        await expectLater(
          StoryWorkspaceBootstrap.create(
            root: root,
            ffi: ffi,
            catalogSelections: catalog,
            profile: AuthoringValidationProfile.experimental,
            metadata: StoryProjectMetadata(
              name: 'Invalid ID',
              version: '1',
              author: 'tests',
            ),
            projectIdSource: _FixedProjectIdSource(projectId),
          ),
          throwsFormatException,
        );
        expect(
          await File(p.join(root.path, 'gore-project.json')).exists(),
          isFalse,
        );
      }
    },
  );

  test('secure default ProjectId retries are bounded', () {
    expect(
      () => SecureStoryProjectIdSource(
        random: const _ConstantRandom(0),
      ).nextProjectId(),
      throwsStateError,
    );
    expect(
      SecureStoryProjectIdSource(
        random: const _ConstantRandom(1),
      ).nextProjectId(),
      List<String>.filled(16, '01').join(),
    );
  });
}

final class _FixedProjectIdSource implements StoryProjectIdSource {
  const _FixedProjectIdSource(this.value);

  final String value;

  @override
  String nextProjectId() => value;
}

final class _ConstantRandom implements Random {
  const _ConstantRandom(this.value);

  final int value;

  @override
  bool nextBool() => value.isOdd;

  @override
  double nextDouble() => value == 0 ? 0 : 0.5;

  @override
  int nextInt(int max) => value % max;
}

String _expectedProjectJson({
  required String projectId,
  required StoryProjectMetadata metadata,
  required String executableByte,
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 2,
  'project_id': projectId,
  'revision': 0,
  'meta': <String, Object?>{
    'name': metadata.name,
    'version': metadata.version,
    'author': metadata.author,
  },
  'target': <String, Object?>{'executable': _seal(executableByte, 171698176)},
  'authoring_locales': metadata.authoringLocales,
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

final class _BootstrapCore implements GoreCoreFfiService {
  final Map<String, String> _projectsByHead = <String, String>{};
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  String get description => 'Story bootstrap test core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const <String, Object?>{},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'authoring_store_prepare_document_checkpoint':
        return _prepare(payload);
      case 'authoring_store_open_head_bytes_document':
        return _openHead(payload['head_json']! as String);
      case 'authoring_store_open_document':
        final root = payload['root']! as String;
        final rawHead = await File(
          p.join(root, 'gore-project.json'),
        ).readAsString();
        return _openHead(rawHead);
      default:
        return <String, Object?>{
          'ok': false,
          'error': <String, Object?>{
            'code': 'UNKNOWN_COMMAND',
            'message': command,
          },
        };
    }
  }

  Future<Map<String, Object?>> _prepare(Map<String, Object?> payload) async {
    final root = payload['root']! as String;
    final expected = payload['expected_head_json'] as String?;
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expected) {
      return <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'AUTHORING_STORE_HEAD_CONFLICT',
          'message': 'test head conflict',
        },
      };
    }
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

  Map<String, Object?> _openHead(String headJson) {
    final projectJson = _projectsByHead[headJson];
    if (projectJson == null) {
      return <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'AUTHORING_STORE_MISSING_OBJECT',
          'message': 'unknown test head',
        },
      };
    }
    return <String, Object?>{
      'ok': true,
      'head_json': headJson,
      'project_json': projectJson,
      'diagnostics': <Object?>[_combinedDiagnostic()],
      'blocks_build': true,
    };
  }
}

Map<String, Object?> _combinedDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'entity': null,
  'property_path': 'schema_revision',
  'message':
      'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented',
  'related_entities': <Object?>[],
  'blocks_build': true,
};

Future<AuthoringStoryCatalogSelections> _catalogSelections({
  String executableByte = '1',
}) => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': _catalogResponse(executableByte),
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogJson);

Map<String, Object?> _catalogResponse(String executableByte) =>
    <String, Object?>{
      'ok': true,
      'request_catalog_sha256': crypto.sha256
          .convert(utf8.encode(_catalogJson))
          .toString(),
      'selections': <String, Object?>{
        'schema_revision': 1,
        'generation': <String, Object?>{
          'edition': 'g1r-steam',
          'executable': _seal(executableByte, 171698176),
          'shipping_cache': _seal('2', 123394250),
          'binds_cache': _seal('3', 5903938),
        },
        'catalog_seal': _seal('4', 5611),
        'npcs': <Object?>[_npc(viper: false), _npc(viper: true)],
        'quest_parents': <Object?>[
          <String, Object?>{
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
          },
        ],
        'quest_collision_catalog': <String, Object?>{
          'status': 'inventory_unavailable',
          'catalog_layer': 'resolved-loadout.scripts.v1',
          'source_seal': _seal('2', 123394250),
          'blocks_draft_creation': true,
        },
        'blocks_build': true,
      },
    };

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(64, byte).join(),
};

Map<String, Object?> _npc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper
      ? 'g1r:npc:om_stt_viper_302'
      : 'g1r:npc:om_grd_asghan_263';
  final character = _classSelection(
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
    'ai_agent_config': _classSelection(
      catalogId,
      'ai_agent_config',
      viper ? 'd' : 'b',
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
  'source_seal': _seal(sealByte, 100),
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
