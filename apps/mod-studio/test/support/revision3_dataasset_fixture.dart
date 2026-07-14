import 'dart:collection';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';

const revision3DataAssetTargetPath = '/Game/Data/ManagedFixture';

final class Revision3DataAssetFixture {
  Revision3DataAssetFixture._({
    required this.basisHead,
    required this.basisProjectJson,
    required this.stage,
    required this.manifestAsset,
    required this.stagedProjectJson,
    required this.stagedHead,
    required this.removedProjectJson,
    required this.removedHead,
  });

  factory Revision3DataAssetFixture.fromBasis({
    required AuthoringWorkingHead basisHead,
    required String basisProjectJson,
    String targetPath = revision3DataAssetTargetPath,
    void Function(Map<String, Object?> manifest)? mutateManifest,
  }) {
    final basis = jsonDecode(basisProjectJson) as Map<String, Object?>;
    final projectId = basis['project_id']! as String;
    final basisRevision = basis['revision']! as int;
    final target = (basis['target']! as Map).cast<String, Object?>();
    final generation = _generation(targetPath);
    final selector = _selector();
    final patchedUasset = _seal(10, '1');
    final patchedUexp = _seal(20, '2');
    final usmap = _seal(30, '3');
    final storageManifest = <String, Object?>{
      'format': 'gore.dataasset.fixed-leaf-stage.v1',
      'project_id': projectId,
      'project_target': target,
      'basis_head': jsonDecode(basisHead.canonicalJson),
      'basis_project_revision': basisRevision,
      'staged_project_revision': basisRevision + 1,
      'target_path': targetPath,
      'generation': generation,
      'selector': selector,
      'replacement_hex': '02000000',
      'patched_uasset': patchedUasset,
      'patched_uexp': patchedUexp,
      'usmap': usmap,
      'sidecars': <String, Object?>{},
      'build_status': 'blocked',
      'runtime_status': 'runtime_unqualified',
      'artifact_authority': 'not_granted',
      'publication_status': 'not_supported',
    };
    mutateManifest?.call(storageManifest);
    final manifestBytes = utf8.encode(jsonEncode(storageManifest));
    final manifestAsset = <String, Object?>{
      'byte_len': manifestBytes.length,
      'sha256': crypto.sha256.convert(manifestBytes).toString(),
    };
    final responseManifest =
        _sortedJson(storageManifest) as Map<String, Object?>;
    final stage =
        _sortedJson(<String, Object?>{
              'manifest_asset': manifestAsset,
              'manifest': responseManifest,
            })!
            as Map<String, Object?>;

    final staged = _cloneObject(basis);
    staged['revision'] = basisRevision + 1;
    final assetStore = (staged['asset_store']! as Map).cast<String, Object?>();
    final assets = SplayTreeMap<String, Object?>.from(
      (assetStore['assets']! as Map).cast<String, Object?>(),
    );
    final manifestSidecars = (storageManifest['sidecars']! as Map)
        .cast<String, Object?>();
    for (final seal in <Map<String, Object?>>[
      (storageManifest['patched_uasset']! as Map).cast<String, Object?>(),
      (storageManifest['patched_uexp']! as Map).cast<String, Object?>(),
      (storageManifest['usmap']! as Map).cast<String, Object?>(),
      for (final value in manifestSidecars.values)
        (value! as Map).cast<String, Object?>(),
    ]) {
      assets[seal['sha256']! as String] = <String, Object?>{
        'byte_len': seal['byte_len'],
        'media_type':
            'application/vnd.gore.dataasset-fixed-leaf-component;version=1',
      };
    }
    assets[manifestAsset['sha256']! as String] = <String, Object?>{
      'byte_len': manifestAsset['byte_len'],
      'media_type':
          'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1',
    };
    assetStore['assets'] = assets;
    staged['asset_store'] = assetStore;
    final stagedProjectJson = jsonEncode(staged);
    final stagedHead = _headForProject(stagedProjectJson);

    final removed = _cloneObject(staged);
    removed['revision'] = basisRevision + 2;
    final removedStore = (removed['asset_store']! as Map)
        .cast<String, Object?>();
    final removedAssets = SplayTreeMap<String, Object?>.from(
      (removedStore['assets']! as Map).cast<String, Object?>(),
    )..remove(manifestAsset['sha256']);
    removedStore['assets'] = removedAssets;
    removed['asset_store'] = removedStore;
    final removedProjectJson = jsonEncode(removed);
    final removedHead = _headForProject(removedProjectJson);

    return Revision3DataAssetFixture._(
      basisHead: basisHead,
      basisProjectJson: basisProjectJson,
      stage: stage,
      manifestAsset: manifestAsset,
      stagedProjectJson: stagedProjectJson,
      stagedHead: stagedHead,
      removedProjectJson: removedProjectJson,
      removedHead: removedHead,
    );
  }

  final AuthoringWorkingHead basisHead;
  final String basisProjectJson;
  final Map<String, Object?> stage;
  final Map<String, Object?> manifestAsset;
  final String stagedProjectJson;
  final AuthoringWorkingHead stagedHead;
  final String removedProjectJson;
  final AuthoringWorkingHead removedHead;

  Map<String, Object?> prepareResponse() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basisHead.canonicalJson,
    'head_json': stagedHead.canonicalJson,
    'project_json': stagedProjectJson,
    'revision': _projectRevision(stagedProjectJson),
    'stage': _cloneObject(stage),
    'deduplicated_blobs': 0,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'artifact_authority': 'not_granted',
    'publication_status': 'not_supported',
  };

  Map<String, Object?> listResponse() => <String, Object?>{
    'ok': true,
    'outcome': 'listed_exact_head',
    'basis_head_json': stagedHead.canonicalJson,
    'revision': _projectRevision(stagedProjectJson),
    'stages': <Object?>[_cloneObject(stage)],
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'artifact_authority': 'not_granted',
    'publication_status': 'not_supported',
  };

  Map<String, Object?> removalResponse() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_remove_unpublished',
    'basis_head_json': stagedHead.canonicalJson,
    'head_json': removedHead.canonicalJson,
    'project_json': removedProjectJson,
    'revision': _projectRevision(removedProjectJson),
    'removed': _cloneObject(stage),
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'artifact_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

/// Frozen from `gore-authoring::dataasset_stage::ProjectionFixture` through the same
/// `serde_json::to_value` projection used by gore-ffi. In particular, the embedded WorkingHead
/// is alphabetically ordered (`snapshot`, then `store_format`) while its stored manifest bytes
/// use Rust struct order (`store_format`, then `snapshot`). The fixed seals below make either
/// ordering drift fail instead of allowing this Dart fixture to become self-consistent.
Revision3DataAssetFixture revision3DataAssetNativeGoldenFixture() {
  const basisHeadJson =
      '{"store_format":1,"snapshot":{"byte_len":365,'
      '"sha256":"bc93e54f38b3596de20ff24052f4dfdf5579cd2f7cef5bf1da749d7651caef8d"}}';
  const stagedHeadJson =
      '{"store_format":1,"snapshot":{"byte_len":1167,'
      '"sha256":"75bbe53bb6142b82ebbb0d684d1cba4f83f7bb3a1a62b2f8a03da85d941b84d5"}}';
  const removedHeadJson =
      '{"store_format":1,"snapshot":{"byte_len":1004,'
      '"sha256":"414509d96a194eb3f24363b1c1a10ec1c8206d6352bf9467aa85029110d65fdf"}}';
  final basisHead = AuthoringWorkingHead.fromCanonicalJson(basisHeadJson);
  final target = <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 17,
      'sha256': List<String>.filled(32, '09').join(),
    },
  };
  final basisProjectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': '07070707070707070707070707070707',
    'revision': 4,
    'meta': <String, Object?>{
      'name': 'DataAsset stage test',
      'version': '0.1.0',
      'author': 'tests',
    },
    'target': target,
    'authoring_locales': <Object?>[],
    'entities': <String, Object?>{},
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
  final main = _file('G1R-Windows.utoc', 64, '1');
  final global = _file('global.utoc', 32, '2');
  const usmapSha =
      '83ca62173c9e64889c0a59dd74140812fd0ce2289b801b04ce6b27dbb036a750';
  final generation = <String, Object?>{
    'format': 'gore.asset.generation.v1',
    'asset': '/Game/TestAsset',
    'usmap': <String, Object?>{
      'file_name': 'Mappings.usmap',
      'length': 19,
      'sha256': usmapSha,
    },
    'main_utoc': main,
    'global_utoc': global,
    'global_ucas': _file('global.ucas', 96, '3'),
    'container_set': <Object?>[main, global],
    'target_chunks': <Object?>[
      _nativeGoldenChunk('58becc37c6ec7b2000000001', 'ContainerHeader', main),
      _nativeGoldenChunk('58becc37c6ec7b2000000002', 'ExportBundleData', main),
      _nativeGoldenChunk('58becc37c6ec7b2000000003', 'BulkData', main),
    ],
  };
  final selector = <String, Object?>{
    'format': 1,
    'profile': 'g1r_ue5_4',
    'package_seal': <String, Object?>{
      'uasset_sha256': List<String>.filled(32, '41').join(),
      'uexp_sha256': List<String>.filled(32, '42').join(),
    },
    'usmap_sha256': usmapSha,
    'export_index': 0,
    'object_name': 'TestAsset',
    'class_path': '/Script/Test.Fixture',
    'component': 'uexp',
    'export_sha256': List<String>.filled(32, '55').join(),
    'role': 'property_value',
    'kind': 'bool',
    'path': <Object?>[
      <String, Object?>{
        'step': 'property',
        'schema_index': 0,
        'property_name': 'Enabled',
        'array_index': 0,
        'array_dimension': 1,
        'declaring_schema_name': 'Fixture',
        'declaring_module_path': '/Script/Test',
        'property_type': <String, Object?>{'type': 'bool'},
      },
    ],
    'expected_hex': '01',
  };
  final patchedUasset = <String, Object?>{
    'byte_len': 22,
    'sha256':
        '78a20a75a5998cc07081b8a07f4f5c27e2164078a32a466b9e026bd573a1ddc5',
  };
  final patchedUexp = <String, Object?>{
    'byte_len': 20,
    'sha256':
        'fc4d28b20551ddad85fd66cad2e83fe46da06b2aaafa004c76ccc0f4766f2e9a',
  };
  final usmap = <String, Object?>{'byte_len': 19, 'sha256': usmapSha};
  final bulk = <String, Object?>{
    'byte_len': 20,
    'sha256':
        'aec5134787226a11221625fb8888860ea5652e8b0b9951b8eeb419b0898fd668',
  };
  final storageManifest = <String, Object?>{
    'format': 'gore.dataasset.fixed-leaf-stage.v1',
    'project_id': '07070707070707070707070707070707',
    'project_target': target,
    'basis_head': jsonDecode(basisHeadJson),
    'basis_project_revision': 4,
    'staged_project_revision': 5,
    'target_path': '/Game/TestAsset',
    'generation': generation,
    'selector': selector,
    'replacement_hex': '00',
    'patched_uasset': patchedUasset,
    'patched_uexp': patchedUexp,
    'usmap': usmap,
    'sidecars': <String, Object?>{'BulkData': bulk},
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'artifact_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
  final manifestBytes = utf8.encode(jsonEncode(storageManifest));
  const manifestSha =
      'ca09912c0742a160ad78c099f16d92022f39a24cbfefdd19ec8a3acc2c4d8c50';
  if (manifestBytes.length != 3759 ||
      crypto.sha256.convert(manifestBytes).toString() != manifestSha) {
    throw StateError('frozen native DataAsset manifest fixture drifted');
  }
  final manifestAsset = <String, Object?>{
    'byte_len': 3759,
    'sha256': manifestSha,
  };
  final stage =
      _sortedJson(<String, Object?>{
            'manifest_asset': manifestAsset,
            'manifest': storageManifest,
          })!
          as Map<String, Object?>;

  final stagedAssets = SplayTreeMap<String, Object?>();
  for (final seal in <Map<String, Object?>>[
    patchedUasset,
    patchedUexp,
    usmap,
    bulk,
  ]) {
    stagedAssets[seal['sha256']! as String] = <String, Object?>{
      'byte_len': seal['byte_len'],
      'media_type':
          'application/vnd.gore.dataasset-fixed-leaf-component;version=1',
    };
  }
  stagedAssets[manifestSha] = <String, Object?>{
    'byte_len': 3759,
    'media_type':
        'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1',
  };
  final stagedProjectJson = _nativeGoldenProjectJson(5, target, stagedAssets);
  final stagedHead = AuthoringWorkingHead.fromCanonicalJson(stagedHeadJson);
  _requireFrozenProject(
    stagedProjectJson,
    expectedLength: 1150,
    expectedSha256:
        '8d606bbb84c66342290d6e844ad1f5a5438aeb7b2b6d1849c1651e9d0c647e1d',
    context: 'staged',
  );

  final removedAssets = SplayTreeMap<String, Object?>.from(stagedAssets)
    ..remove(manifestSha);
  final removedProjectJson = _nativeGoldenProjectJson(6, target, removedAssets);
  final removedHead = AuthoringWorkingHead.fromCanonicalJson(removedHeadJson);
  _requireFrozenProject(
    removedProjectJson,
    expectedLength: 987,
    expectedSha256:
        '779ab7f5c58577825d4eb5a91e29401003d777fd365213b92c53791d60c67ec4',
    context: 'removed',
  );
  return Revision3DataAssetFixture._(
    basisHead: basisHead,
    basisProjectJson: basisProjectJson,
    stage: stage,
    manifestAsset: manifestAsset,
    stagedProjectJson: stagedProjectJson,
    stagedHead: stagedHead,
    removedProjectJson: removedProjectJson,
    removedHead: removedHead,
  );
}

Map<String, Object?> _nativeGoldenChunk(
  String id,
  String type,
  Map<String, Object?> winner,
) => <String, Object?>{
  'chunk_id': id,
  'chunk_type': type,
  'winner_utoc': winner,
  'length': 1,
  'blake3': List<String>.filled(32, 'a1').join(),
  'toc_hash': List<String>.filled(20, 'b2').join(),
  'toc_hash_bytes': 20,
};

String _nativeGoldenProjectJson(
  int revision,
  Map<String, Object?> target,
  Map<String, Object?> assets,
) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': '07070707070707070707070707070707',
  'revision': revision,
  'meta': <String, Object?>{
    'name': 'DataAsset stage test',
    'version': '0.1.0',
    'author': 'tests',
  },
  'target': target,
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': assets},
});

void _requireFrozenProject(
  String projectJson, {
  required int expectedLength,
  required String expectedSha256,
  required String context,
}) {
  final bytes = utf8.encode(projectJson);
  if (bytes.length != expectedLength ||
      crypto.sha256.convert(bytes).toString() != expectedSha256) {
    throw StateError(
      'frozen native DataAsset $context project fixture drifted: '
      '${bytes.length}/${crypto.sha256.convert(bytes)}',
    );
  }
}

AuthoringWorkingHead revision3DataAssetHeadForProject(String projectJson) =>
    _headForProject(projectJson);

Map<String, Object?> revision3DataAssetClone(Map<String, Object?> value) =>
    _cloneObject(value);

Map<String, Object?> _generation(String targetPath) {
  final main = _file('pakchunk0-Windows.utoc', 40, '4');
  final global = _file('global.utoc', 50, '5');
  final targetChunkPrefix = switch (targetPath) {
    '/Game/Data/ManagedFixture' => 'e54f79b8fc97323c',
    '/Game/Data/Alpha' => '6fc77ed64484fc94',
    '/Game/Data/Bravo' => '512d90171282bc5d',
    '/Game/Data/Charlie' => 'df162575fb03dc35',
    '/Game/Data/Delta' => '0f603f7d4858c8af',
    '/Game/Data/Echo' => 'e0de4d814e1ef6cb',
    '/Game/Data/Foxtrot' => '57a58392a9b3ec77',
    '/Game/Data/Golf' => '32e3c9df4e801491',
    '/Game/Data/Hotel' => '90591333a90d59c5',
    '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps' =>
      '01e173a19ea374c9',
    _ => throw ArgumentError.value(
      targetPath,
      'targetPath',
      'fixture needs a native CityHash64 package-id prefix',
    ),
  };
  return <String, Object?>{
    'format': 'gore.asset.generation.v1',
    'asset': targetPath,
    'usmap': _file('Mappings.usmap', 30, '3'),
    'main_utoc': main,
    'global_utoc': global,
    'global_ucas': _file('global.ucas', 60, '6'),
    'container_set': <Object?>[main, global],
    'target_chunks': <Object?>[
      _chunk('${targetChunkPrefix}00000001', 'ContainerHeader', main, 1),
      _chunk('${targetChunkPrefix}00000002', 'ExportBundleData', main, 2),
    ],
  };
}

Map<String, Object?> _selector() => <String, Object?>{
  'format': 1,
  'profile': 'g1r_ue5_4',
  'package_seal': <String, Object?>{
    'uasset_sha256': List<String>.filled(64, '7').join(),
    'uexp_sha256': List<String>.filled(64, '8').join(),
  },
  'usmap_sha256': List<String>.filled(64, '3').join(),
  'export_index': 0,
  'object_name': 'ManagedFixture',
  'class_path': '/Script/G1.ManagedFixture',
  'component': 'uexp',
  'export_sha256': List<String>.filled(64, '9').join(),
  'role': 'property_value',
  'kind': 'int32',
  'path': <Object?>[
    <String, Object?>{
      'step': 'property',
      'schema_index': 0,
      'property_name': 'Value',
      'array_index': 0,
      'array_dimension': 1,
      'declaring_schema_name': 'ManagedFixture',
      'declaring_module_path': '/Script/G1',
      'property_type': <String, Object?>{'type': 'int'},
    },
  ],
  'expected_hex': '01000000',
};

Map<String, Object?> _file(String name, int length, String digit) =>
    <String, Object?>{
      'file_name': name,
      'length': length,
      'sha256': List<String>.filled(64, digit).join(),
    };

Map<String, Object?> _chunk(
  String id,
  String type,
  Map<String, Object?> winner,
  int length,
) => <String, Object?>{
  'chunk_id': id,
  'chunk_type': type,
  'winner_utoc': winner,
  'length': length,
  'blake3': List<String>.filled(64, 'a').join(),
  'toc_hash': List<String>.filled(40, 'b').join(),
  'toc_hash_bytes': 20,
};

Map<String, Object?> _seal(int length, String digit) => <String, Object?>{
  'byte_len': length,
  'sha256': List<String>.filled(64, digit).join(),
};

AuthoringWorkingHead _headForProject(String projectJson) {
  final bytes = utf8.encode(projectJson);
  return AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{
        'byte_len': bytes.length,
        'sha256': crypto.sha256.convert(bytes).toString(),
      },
    }),
  );
}

int _projectRevision(String projectJson) =>
    (jsonDecode(projectJson) as Map<String, Object?>)['revision']! as int;

Map<String, Object?> _cloneObject(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Object? _sortedJson(Object? value) {
  if (value is List) return value.map<Object?>(_sortedJson).toList();
  if (value is Map) {
    final sorted = SplayTreeMap<String, Object?>();
    for (final entry in value.entries) {
      sorted[entry.key as String] = _sortedJson(entry.value);
    }
    return sorted;
  }
  return value;
}
