import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_catalog_adapter.dart';

const _catalogJson = '{"format":"story_catalog"}';
const _asghanId = 'g1r:npc:om_grd_asghan_263';
const _viperId = 'g1r:npc:om_stt_viper_302';

void main() {
  late StoryCatalogAdapter adapter;

  setUp(() async {
    adapter = StoryCatalogAdapter.fromSelections(await _selections());
  });

  test('projects exact qualified NPC catalog rows', () {
    expect(adapter.npcChoices.map((choice) => choice.catalogId), <String>[
      _asghanId,
      _viperId,
    ]);
    expect(adapter.npcChoices.map((choice) => choice.displayName), <String>[
      'Asghan',
      'Viper',
    ]);
    expect(
      adapter.npcChoices,
      everyElement(
        isA<StoryCatalogNpcChoice>()
            .having(
              (choice) => choice.authoringQualification,
              'authoring qualification',
              AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
            )
            .having(
              (choice) => choice.runtimeQualification,
              'runtime qualification',
              AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
            )
            .having((choice) => choice.blocksBuild, 'blocks build', isTrue),
      ),
    );

    expect(adapter.npcChoices.first.runtimeUniqueName, 'OM_GRD_Asghan_263');
    expect(adapter.npcChoices.last.runtimeUniqueName, 'OM_STT_Viper_302');
    expect(
      () => adapter.npcChoices.add(adapter.npcChoices.first),
      throwsUnsupportedError,
    );
  });

  test('projects immutable R3 Quest catalog rows', () {
    expect(adapter.questParents, hasLength(1));
    expect(
      adapter.questParents.single.runtimeClass,
      'UQuest_SwampCamp_SCCHAPTER2',
    );
    expect(
      adapter.questParents.single.authoringSelector,
      _selectorAlias('g1r:quest-parent:swampcamp_scchapter2', 'quest_parent'),
    );
    expect(adapter.questParents.single.sourceSeal.byteLength, 100);
    expect(
      adapter.questParents.single.sourceSeal.sha256,
      List.filled(64, 'f').join(),
    );
    expect(adapter.questGivers, hasLength(2));
    expect(
      adapter.questGivers.map((giver) => giver.runtimeUniqueName),
      <String>['OM_GRD_Asghan_263', 'OM_STT_Viper_302'],
    );
    expect(
      () => adapter.questParents.add(adapter.questParents.single),
      throwsUnsupportedError,
    );
    expect(
      () => adapter.questGivers.add(adapter.questGivers.first),
      throwsUnsupportedError,
    );
  });
}

Future<AuthoringStoryCatalogSelections> _selections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': _catalogResponse(),
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogJson);

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(64, byte).join(),
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

Map<String, Object?> _npc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper ? _viperId : _asghanId;
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

Map<String, Object?> _catalogResponse() => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(_catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': <String, Object?>{
      'edition': 'g1r-steam',
      'executable': _seal('1', 171698176),
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
