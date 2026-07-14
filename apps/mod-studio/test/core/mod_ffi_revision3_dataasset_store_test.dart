import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_dataasset_fixture.dart';

String _basisProjectJson() => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': 'dada0303030303030303030303030303',
  'revision': 7,
  'meta': <String, Object?>{
    'name': 'DataAsset Dart fixture',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, 'c').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Revision3DataAssetFixture _fixture({
  String targetPath = revision3DataAssetTargetPath,
  void Function(Map<String, Object?> manifest)? mutateManifest,
}) {
  final project = _basisProjectJson();
  return Revision3DataAssetFixture.fromBasis(
    basisHead: revision3DataAssetHeadForProject(project),
    basisProjectJson: project,
    targetPath: targetPath,
    mutateManifest: mutateManifest,
  );
}

Map<String, Object?> _generation(Map<String, Object?> manifest) =>
    (manifest['generation']! as Map).cast<String, Object?>();

Map<String, Object?> _selector(Map<String, Object?> manifest) =>
    (manifest['selector']! as Map).cast<String, Object?>();

Matcher _formatMessage(String fragment) => isA<FormatException>().having(
  (error) => error.message,
  'message',
  contains(fragment),
);

void _expectPrepareRejected(
  Revision3DataAssetFixture fixture,
  Matcher matcher,
) {
  expect(
    () => AuthoringRevision3DataAssetStagePreparation.fromJson(
      fixture.prepareResponse(),
      expectedHead: fixture.basisHead,
    ),
    throwsA(matcher),
  );
}

void main() {
  test(
    'frozen native stage/list/removal golden preserves split JSON ordering',
    () {
      final fixture = revision3DataAssetNativeGoldenFixture();
      final responseStage = (fixture.prepareResponse()['stage']! as Map)
          .cast<String, Object?>();
      final responseManifest = (responseStage['manifest']! as Map)
          .cast<String, Object?>();
      final embeddedHead = (responseManifest['basis_head']! as Map)
          .cast<String, Object?>();
      expect(embeddedHead.keys, <String>['snapshot', 'store_format']);
      expect(
        fixture.basisHead.canonicalJson.startsWith('{"store_format":1'),
        isTrue,
      );

      final prepared = AuthoringRevision3DataAssetStagePreparation.fromJson(
        fixture.prepareResponse(),
        expectedHead: fixture.basisHead,
      );
      final listed = AuthoringRevision3DataAssetStageListResult.fromJson(
        fixture.listResponse(),
        expectedHead: fixture.stagedHead,
      );
      final removed =
          AuthoringRevision3DataAssetStageRemovalPreparation.fromJson(
            fixture.removalResponse(),
            expectedHead: fixture.stagedHead,
            requestedTargetPath: '/Game/testasset',
          );

      expect(prepared.stage.manifestAsset.byteLength, 3759);
      expect(
        prepared.stage.manifestAsset.sha256,
        'ca09912c0742a160ad78c099f16d92022f39a24cbfefdd19ec8a3acc2c4d8c50',
      );
      expect(prepared.head.snapshotSha256, fixture.stagedHead.snapshotSha256);
      expect(listed.stages.single.targetPath, '/Game/TestAsset');
      expect(listed.stages.single.sidecars.keys, <String>['BulkData']);
      expect(removed.head.snapshotSha256, fixture.removedHead.snapshotSha256);
    },
  );

  test(
    'DataAsset list follows native target-path order, not CAS hash order',
    () {
      final candidates = <Revision3DataAssetFixture>[
        for (final name in <String>[
          'Alpha',
          'Bravo',
          'Charlie',
          'Delta',
          'Echo',
          'Foxtrot',
          'Golf',
          'Hotel',
        ])
          _fixture(targetPath: '/Game/Data/$name'),
      ];
      Revision3DataAssetFixture? first;
      Revision3DataAssetFixture? second;
      for (var left = 0; left < candidates.length && first == null; left++) {
        for (var right = left + 1; right < candidates.length; right++) {
          if ((candidates[left].manifestAsset['sha256']! as String).compareTo(
                candidates[right].manifestAsset['sha256']! as String,
              ) >
              0) {
            first = candidates[left];
            second = candidates[right];
            break;
          }
        }
      }
      expect(first, isNotNull, reason: 'fixture needs a path/hash inversion');

      final ordered = first!.listResponse();
      ordered['stages'] = <Object?>[first.stage, second!.stage];
      final parsed = AuthoringRevision3DataAssetStageListResult.fromJson(
        ordered,
        expectedHead: first.stagedHead,
      );
      expect(parsed.stages.map((stage) => stage.targetPath), <String>[
        first.stage['manifest'] is Map
            ? ((first.stage['manifest']! as Map)['target_path']! as String)
            : fail('fixture manifest is missing'),
        ((second.stage['manifest']! as Map)['target_path']! as String),
      ]);

      final reversed = jsonDecode(jsonEncode(ordered)) as Map<String, Object?>;
      (reversed['stages']! as List<Object?>).setAll(0, <Object?>[
        second.stage,
        first.stage,
      ]);
      expect(
        () => AuthoringRevision3DataAssetStageListResult.fromJson(
          reversed,
          expectedHead: first!.stagedHead,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'DataAsset manifest accepts exact target bulk roles and ignores dependency bulk chunks',
    () {
      final fixture = _fixture(
        mutateManifest: (manifest) {
          final generation = _generation(manifest);
          final chunks = generation['target_chunks']! as List<Object?>;
          final export = (chunks[1]! as Map).cast<String, Object?>();
          final prefix = (export['chunk_id']! as String).substring(0, 16);
          chunks.add(<String, Object?>{
            ...export,
            'chunk_id': '${prefix}00000003',
            'chunk_type': 'BulkData',
          });
          chunks.add(<String, Object?>{
            ...export,
            'chunk_id': '111111111111111100000004',
            'chunk_type': 'OptionalBulkData',
          });
          manifest['sidecars'] = <String, Object?>{
            'BulkData': <String, Object?>{
              'byte_len': 5,
              'sha256': List<String>.filled(64, 'd').join(),
            },
          };
        },
      );

      final parsed = AuthoringRevision3DataAssetStagePreparation.fromJson(
        fixture.prepareResponse(),
        expectedHead: fixture.basisHead,
      );

      expect(parsed.stage.generationChunkCount, 4);
      expect(parsed.stage.sidecars.keys, <String>['BulkData']);
    },
  );

  test('DataAsset target identity matches the native long-path package ID', () {
    const targetPath =
        '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';
    final fixture = _fixture(targetPath: targetPath);

    final parsed = AuthoringRevision3DataAssetStagePreparation.fromJson(
      fixture.prepareResponse(),
      expectedHead: fixture.basisHead,
    );

    expect(parsed.stage.targetPath, targetPath);
  });

  test('DataAsset manifest binds selector to the generation USMAP', () {
    final fixture = _fixture(
      mutateManifest: (manifest) {
        _selector(manifest)['usmap_sha256'] = List<String>.filled(
          64,
          'f',
        ).join();
      },
    );

    _expectPrepareRejected(fixture, _formatMessage('exact generation USMAP'));
  });

  test('DataAsset manifest rejects referential fixed-leaf kinds', () {
    for (final kind in const <String>['package_index', 'fname']) {
      final fixture = _fixture(
        mutateManifest: (manifest) {
          final selector = _selector(manifest);
          selector['kind'] = kind;
          if (kind == 'fname') {
            selector['expected_hex'] = '0100000000000000';
            manifest['replacement_hex'] = '0200000000000000';
          }
        },
      );

      _expectPrepareRejected(fixture, _formatMessage('referential fixed leaf'));
    }
  });

  test('DataAsset manifest limits Bool bytes to canonical zero or one', () {
    final invalidExpected = _fixture(
      mutateManifest: (manifest) {
        final selector = _selector(manifest);
        selector['kind'] = 'bool';
        selector['expected_hex'] = '02';
        manifest['replacement_hex'] = '00';
      },
    );
    final invalidReplacement = _fixture(
      mutateManifest: (manifest) {
        final selector = _selector(manifest);
        selector['kind'] = 'bool';
        selector['expected_hex'] = '00';
        manifest['replacement_hex'] = '02';
      },
    );

    _expectPrepareRejected(invalidExpected, _formatMessage('canonical 0/1'));
    _expectPrepareRejected(invalidReplacement, _formatMessage('canonical 0/1'));
  });

  test('DataAsset manifest limits the semantic selector path to 128 steps', () {
    final fixture = _fixture(
      mutateManifest: (manifest) {
        final selector = _selector(manifest);
        final step = (selector['path']! as List<Object?>).single;
        selector['path'] = <Object?>[
          for (var index = 0; index < 129; index++)
            jsonDecode(jsonEncode(step)),
        ];
      },
    );

    _expectPrepareRejected(fixture, _formatMessage('not a bounded list'));
  });

  test('DataAsset manifest requires a target-owned ExportBundleData chunk', () {
    final fixture = _fixture(
      mutateManifest: (manifest) {
        final chunks = _generation(manifest)['target_chunks']! as List<Object?>;
        final export = (chunks[1]! as Map).cast<String, Object?>();
        export['chunk_id'] = '111111111111111100000002';
      },
    );

    _expectPrepareRejected(
      fixture,
      _formatMessage('lacks required target chunks'),
    );
  });

  test('DataAsset manifest binds sidecars to exact target bulk roles', () {
    final missingSidecar = _fixture(
      mutateManifest: (manifest) {
        final chunks = _generation(manifest)['target_chunks']! as List<Object?>;
        final export = (chunks[1]! as Map).cast<String, Object?>();
        final prefix = (export['chunk_id']! as String).substring(0, 16);
        chunks.add(<String, Object?>{
          ...export,
          'chunk_id': '${prefix}00000003',
          'chunk_type': 'BulkData',
        });
      },
    );
    final unexpectedSidecar = _fixture(
      mutateManifest: (manifest) {
        manifest['sidecars'] = <String, Object?>{
          'BulkData': <String, Object?>{
            'byte_len': 5,
            'sha256': List<String>.filled(64, 'd').join(),
          },
        };
      },
    );

    _expectPrepareRejected(
      missingSidecar,
      _formatMessage('exact target bulk chunks'),
    );
    _expectPrepareRejected(
      unexpectedSidecar,
      _formatMessage('exact target bulk chunks'),
    );
  });

  test('DataAsset manifest rejects duplicate target bulk roles', () {
    final fixture = _fixture(
      mutateManifest: (manifest) {
        final chunks = _generation(manifest)['target_chunks']! as List<Object?>;
        final export = (chunks[1]! as Map).cast<String, Object?>();
        final prefix = (export['chunk_id']! as String).substring(0, 16);
        for (final suffix in const <String>['00000003', '00000004']) {
          chunks.add(<String, Object?>{
            ...export,
            'chunk_id': '$prefix$suffix',
            'chunk_type': 'BulkData',
          });
        }
      },
    );

    _expectPrepareRejected(fixture, _formatMessage('duplicates a target bulk'));
  });

  test(
    'DataAsset R3 wrappers preserve exact request capabilities and closed results',
    () async {
      final fixture = _fixture();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_dataasset_stage_v1': fixture
              .prepareResponse(),
          'authoring_store_list_revision3_dataasset_stages_v1': fixture
              .listResponse(),
          'authoring_store_prepare_remove_revision3_dataasset_stage_v1': fixture
              .removalResponse(),
        },
      );
      final ffi = ModFfi(core);
      const root = r'C:\Mods\Managed DataAsset.goreproj';
      const receipt = r'C:\Receipts\patch-receipt.v2.json';

      final prepared = await ffi.authoringStorePrepareRevision3DataAssetStageV1(
        root: root,
        expectedHead: fixture.basisHead,
        patchReceiptPath: receipt,
      );
      final listed = await ffi.authoringStoreListRevision3DataAssetStagesV1(
        root: root,
        expectedHead: fixture.stagedHead,
      );
      final removed = await ffi
          .authoringStorePrepareRemoveRevision3DataAssetStageV1(
            root: root,
            expectedHead: fixture.stagedHead,
            targetPath: revision3DataAssetTargetPath,
          );

      expect(prepared.basisHead.canonicalJson, fixture.basisHead.canonicalJson);
      expect(prepared.head.canonicalJson, fixture.stagedHead.canonicalJson);
      expect(prepared.projectId, 'dada0303030303030303030303030303');
      expect(prepared.revision, 8);
      expect(prepared.stage.targetPath, revision3DataAssetTargetPath);
      expect(prepared.stage.selectorKind, 'int32');
      expect(prepared.stage.selectorPathDepth, 1);
      expect(prepared.stage.replacementByteLength, 4);
      expect(prepared.deduplicatedBlobs, 0);
      expect(
        prepared.buildStatus,
        AuthoringRevision3DataAssetBuildStatus.blocked,
      );
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3DataAssetRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.artifactAuthority,
        AuthoringRevision3DataAssetArtifactAuthority.notGranted,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3DataAssetNativePublicationStatus.notSupported,
      );
      expect(
        listed.stages.single.manifestAsset.sha256,
        prepared.stage.manifestAsset.sha256,
      );
      expect(removed.removed.targetPath, revision3DataAssetTargetPath);
      expect(removed.revision, 9);

      expect(core.calls[0].payload, <String, Object?>{
        'expected_head_json': fixture.basisHead.canonicalJson,
        'patch_receipt_path': receipt,
        'root': root,
      });
      expect(core.calls[1].payload, <String, Object?>{
        'expected_head_json': fixture.stagedHead.canonicalJson,
        'root': root,
      });
      expect(core.calls[2].payload, <String, Object?>{
        'expected_head_json': fixture.stagedHead.canonicalJson,
        'root': root,
        'target_path': revision3DataAssetTargetPath,
      });
    },
  );

  test(
    'DataAsset DTOs reject expanded claims, loose schemas, and wrong exact heads',
    () {
      final fixture = _fixture();
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response.remove('outcome'),
        (response) => response['ok'] = false,
        (response) => response['outcome'] = 'published',
        (response) => response['build_status'] = 'ready',
        (response) => response['runtime_status'] = 'qualified',
        (response) => response['artifact_authority'] = 'granted',
        (response) => response['publication_status'] = 'published',
        (response) => response['pack_status'] = 'ready',
        (response) => response['receipt_path'] = r'C:\secret\receipt.json',
        (response) =>
            response['basis_head_json'] = fixture.stagedHead.canonicalJson,
      ];
      for (final mutate in mutations) {
        final response = fixture.prepareResponse();
        mutate(response);
        expect(
          () => AuthoringRevision3DataAssetStagePreparation.fromJson(
            response,
            expectedHead: fixture.basisHead,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'DataAsset DTO checks every signed number and canonical manifest order',
    () {
      final fixture = _fixture();
      final unsafe = fixture.prepareResponse();
      final stage = (unsafe['stage']! as Map).cast<String, Object?>();
      final manifest = (stage['manifest']! as Map).cast<String, Object?>();
      final generation = (manifest['generation']! as Map)
          .cast<String, Object?>();
      final chunks = generation['target_chunks']! as List<Object?>;
      final chunk = (chunks.first! as Map).cast<String, Object?>();
      chunk['length'] = jsonDecode('9223372036854775808');
      expect(
        () => AuthoringRevision3DataAssetStagePreparation.fromJson(
          unsafe,
          expectedHead: fixture.basisHead,
        ),
        throwsFormatException,
      );

      final decimal = fixture.listResponse();
      decimal['revision'] = 8.0;
      expect(
        () => AuthoringRevision3DataAssetStageListResult.fromJson(
          decimal,
          expectedHead: fixture.stagedHead,
        ),
        throwsFormatException,
      );

      final reordered = fixture.prepareResponse();
      final reorderedStage = (reordered['stage']! as Map)
          .cast<String, Object?>();
      final orderedManifest = (reorderedStage['manifest']! as Map)
          .cast<String, Object?>();
      reorderedStage['manifest'] = <String, Object?>{
        for (final entry in orderedManifest.entries.toList().reversed)
          entry.key: entry.value,
      };
      expect(
        () => AuthoringRevision3DataAssetStagePreparation.fromJson(
          reordered,
          expectedHead: fixture.basisHead,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'DataAsset preparation closes manifest, candidate, and AssetStore bindings',
    () {
      final fixture = _fixture();
      final badSeal = fixture.prepareResponse();
      final stage = (badSeal['stage']! as Map).cast<String, Object?>();
      final seal = (stage['manifest_asset']! as Map).cast<String, Object?>();
      seal['sha256'] = List<String>.filled(64, 'f').join();
      expect(
        () => AuthoringRevision3DataAssetStagePreparation.fromJson(
          badSeal,
          expectedHead: fixture.basisHead,
        ),
        throwsFormatException,
      );

      final badMeta = fixture.prepareResponse();
      final project =
          jsonDecode(badMeta['project_json']! as String)
              as Map<String, Object?>;
      final assetStore = (project['asset_store']! as Map)
          .cast<String, Object?>();
      final assets = (assetStore['assets']! as Map).cast<String, Object?>();
      final manifestMeta = (assets[fixture.manifestAsset['sha256']]! as Map)
          .cast<String, Object?>();
      manifestMeta['media_type'] = 'application/octet-stream';
      badMeta['project_json'] = jsonEncode(project);
      expect(
        () => AuthoringRevision3DataAssetStagePreparation.fromJson(
          badMeta,
          expectedHead: fixture.basisHead,
        ),
        throwsFormatException,
      );

      final retained = fixture.removalResponse();
      final removedProject =
          jsonDecode(retained['project_json']! as String)
              as Map<String, Object?>;
      final removedStore = (removedProject['asset_store']! as Map)
          .cast<String, Object?>();
      final removedAssets = (removedStore['assets']! as Map)
          .cast<String, Object?>();
      removedAssets[fixture.manifestAsset['sha256']!
          as String] = <String, Object?>{
        'byte_len': fixture.manifestAsset['byte_len'],
        'media_type':
            'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1',
      };
      retained['project_json'] = jsonEncode(removedProject);
      expect(
        () => AuthoringRevision3DataAssetStageRemovalPreparation.fromJson(
          retained,
          expectedHead: fixture.stagedHead,
          requestedTargetPath: revision3DataAssetTargetPath,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'DataAsset request preflight rejects unsafe paths before native calls',
    () async {
      final fixture = _fixture();
      final core = FakeGoreCoreFfiService(responses: const {});
      final ffi = ModFfi(core);
      await expectLater(
        ffi.authoringStorePrepareRevision3DataAssetStageV1(
          root: 'bad\u0000root',
          expectedHead: fixture.basisHead,
          patchReceiptPath: r'C:\receipt.json',
        ),
        throwsArgumentError,
      );
      await expectLater(
        ffi.authoringStorePrepareRemoveRevision3DataAssetStageV1(
          root: r'C:\project',
          expectedHead: fixture.stagedHead,
          targetPath: '/Engine/Foreign',
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    },
  );
}
