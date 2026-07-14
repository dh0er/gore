import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../dataasset/dataasset_test_fixtures.dart';
import '../support/revision3_dataasset_fixture.dart';

const _root = r'C:\Projects\DataAssetBrowser.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '31313131313131313131313131313131';

String _basisProjectJson() => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': 7,
  'meta': <String, Object?>{
    'name': 'Installed DataAsset fixture',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{'executable': _digitSeal(171698176, 'd')},
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _seal(int byteLength, String sha256) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
};

Map<String, Object?> _digitSeal(int byteLength, String digit) =>
    _seal(byteLength, digit * 64);

String _headJson() => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': _digitSeal(4096, 'a'),
});

String _completeIndexJson() => jsonEncode(<String, Object?>{
  'status': 'complete_index',
  'physical_chunk_count': 3,
  'winning_export_bundle_count': 2,
  'directory_indexed_export_bundle_count': 2,
  'out_of_scope_export_bundle_count': 0,
  'candidates': <Object?>[
    <String, Object?>{
      'target_path': '/Game/Characters/DA_Asghan',
      'package_id_hex': '0123456789abcdef',
    },
    <String, Object?>{
      'target_path': '/Game/Characters/DA_Viper',
      'package_id_hex': 'fedcba9876543210',
    },
  ],
  'partial_reasons': <Object?>[],
});

String _partialIndexJson() => jsonEncode(<String, Object?>{
  'status': 'partial_index',
  'physical_chunk_count': 5,
  'winning_export_bundle_count': 4,
  'directory_indexed_export_bundle_count': 2,
  'out_of_scope_export_bundle_count': 0,
  'candidates': <Object?>[],
  'partial_reasons': <Object?>[
    <String, Object?>{
      'reason': 'noncanonical_export_bundle_chunk_id',
      'count': 1,
    },
    <String, Object?>{'reason': 'missing_directory_index_path', 'count': 1},
    <String, Object?>{
      'reason': 'noncanonical_game_directory_index_path',
      'count': 1,
    },
    <String, Object?>{'reason': 'package_id_mismatch', 'count': 1},
  ],
});

Map<String, Object?> _response({String? indexJson}) {
  final canonicalIndex = indexJson ?? _completeIndexJson();
  final bytes = utf8.encode(canonicalIndex);
  final index = jsonDecode(canonicalIndex) as Map<String, Object?>;
  final candidates = index['candidates']! as List<Object?>;
  return <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_count': candidates.length,
    'content_status': 'metadata_candidates_only',
    'export_bundle_payload_status': 'not_read',
    'head_json': _headJson(),
    'mount_inventory_entry_count': 2,
    'mount_inventory_seal': _digitSeal(80, 'b'),
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'audit_only',
    'package_index_json': canonicalIndex,
    'package_index_seal': _seal(
      bytes.length,
      crypto.sha256.convert(bytes).toString(),
    ),
    'package_index_status': index['status'],
    'project_id': _projectId,
    'project_revision': 7,
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'installed_dataasset_package_candidates_only',
    'source_snapshot_seal': _digitSeal(120, 'c'),
    'target_executable_seal': _digitSeal(171698176, 'd'),
  };
}

Map<String, Object?> _installedInspectionResponse({
  int ordinal = 1,
  Map<String, Object?>? inspection,
}) {
  final snapshot = _response();
  final index =
      jsonDecode(snapshot['package_index_json']! as String)
          as Map<String, Object?>;
  final candidate =
      (index['candidates']! as List<Object?>)[ordinal]! as Map<String, Object?>;
  return <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': ordinal,
    'head_json': snapshot['head_json'],
    'inspection': inspection ?? validDataAssetInspectionResponse(),
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'inspection_only',
    'package_id_hex': candidate['package_id_hex'],
    'package_index_seal': snapshot['package_index_seal'],
    'project_id': snapshot['project_id'],
    'project_revision': snapshot['project_revision'],
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'selected_installed_dataasset_fixed_leaf_inspection_only',
    'source_snapshot_seal': snapshot['source_snapshot_seal'],
    'target_path': candidate['target_path'],
    'usmap_content_seal': <String, Object?>{
      'byte_len': 256,
      'sha256': 'c' * 64,
    },
    'usmap_inventory_seal': <String, Object?>{
      'byte_len': 96,
      'sha256': 'e' * 64,
    },
  };
}

Future<void> _expectMalformed(Map<String, Object?> response) async {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_store_read_revision3_dataasset_package_index_v1': response,
    },
  );
  await expectLater(
    ModFfi(core).authoringStoreReadRevision3DataAssetPackageIndexV1(
      root: _root,
      gameRoot: _gameRoot,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
    ),
    throwsA(
      isA<ModFfiException>().having(
        (error) => error.code,
        'code',
        ModFfiException.malformedNativeResponseCode,
      ),
    ),
  );
}

void main() {
  test('installed proof binding matches the frozen native golden', () {
    expect(
      DataAssetInstalledSemanticEditIntent.computeInstalledProofBindingSha256(
        candidateOrdinal: 7,
        packageIndex: (byteLength: 1, sha256: '11' * 32),
        sourceSnapshot: (byteLength: 2, sha256: '22' * 32),
        usmapContent: (byteLength: 3, sha256: '33' * 32),
        usmapInventory: (byteLength: 4, sha256: '44' * 32),
        uasset: (byteLength: 5, sha256: '55' * 32),
        uexp: (byteLength: 6, sha256: '66' * 32),
        usmap: (byteLength: 7, sha256: '77' * 32),
      ),
      '827161c17b537a2b63095c51ff204cb398d653d3144bc012d276b4957cea5aed',
    );
  });

  test('Studio handshake requires the sorted installed DataAsset commands', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_read_revision3_dataasset_package_index_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_inspect_revision3_installed_dataasset_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'sends only the exact read request and parses the closed audit',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_dataasset_package_index_v1':
              _response(),
        },
      );
      final result = await ModFfi(core)
          .authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
          );

      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': _headJson(),
        'game_root': _gameRoot,
        'root': _root,
      });
      expect(core.calls.single.payload, isNot(contains('project_json')));
      expect(result.projectId, _projectId);
      expect(result.projectRevision, 7);
      expect(result.candidateCount, 2);
      expect(
        result.index.status,
        AuthoringRevision3DataAssetPackageIndexStatus.completeIndex,
      );
      expect(
        result.index.candidates.map((candidate) => candidate.targetPath),
        <String>['/Game/Characters/DA_Asghan', '/Game/Characters/DA_Viper'],
      );
      expect(
        result.index.candidates.map((candidate) => candidate.ordinal),
        <int>[0, 1],
      );
      expect(
        result.exportBundlePayloadStatus,
        AuthoringRevision3DataAssetExportBundlePayloadStatus.notRead,
      );
      expect(
        result.authorityStatus,
        AuthoringRevision3DataAssetPackageAuthorityStatus.notGranted,
      );
    },
  );

  test('accepts a counter-consistent, reason-sorted partial audit', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_read_revision3_dataasset_package_index_v1': _response(
          indexJson: _partialIndexJson(),
        ),
      },
    );
    final result = await ModFfi(core)
        .authoringStoreReadRevision3DataAssetPackageIndexV1(
          root: _root,
          gameRoot: _gameRoot,
          expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
        );
    expect(
      result.index.status,
      AuthoringRevision3DataAssetPackageIndexStatus.partialIndex,
    );
    expect(result.index.partialReasons, hasLength(4));
    expect(result.candidateCount, 0);
  });

  test('rejects duplicate and noncanonical nested index JSON', () async {
    final canonical = _completeIndexJson();
    final duplicate = canonical.replaceFirst(
      '"status":"complete_index"',
      '"status":"complete_index","status":"complete_index"',
    );
    await _expectMalformed(_response(indexJson: duplicate));

    final reordered = (jsonDecode(canonical) as Map).cast<String, Object?>();
    final candidate =
        ((reordered['candidates']! as List<Object?>).first! as Map)
            .cast<String, Object?>();
    final target = candidate.remove('target_path');
    candidate['target_path'] = target;
    await _expectMalformed(_response(indexJson: jsonEncode(reordered)));
  });

  test('rejects authority, identity, order, seal, and counter drift', () async {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response['authority_status'] = 'granted',
      (response) => response['project_id'] = '0' * 32,
      (response) => response['project_revision'] = 0x8000000000000000,
      (response) => response['head_json'] = jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': _digitSeal(4096, 'e'),
      }),
      (response) => response['package_index_seal'] = _digitSeal(1, 'f'),
      (response) {
        final copy = Map<String, Object?>.from(response);
        response
          ..clear()
          ..['build_status'] = copy['build_status']
          ..addAll(copy);
      },
      (response) {
        final index =
            (jsonDecode(response['package_index_json']! as String) as Map)
                .cast<String, Object?>();
        index['physical_chunk_count'] = 0;
        final encoded = jsonEncode(index);
        final bytes = utf8.encode(encoded);
        response['package_index_json'] = encoded;
        response['package_index_seal'] = _seal(
          bytes.length,
          crypto.sha256.convert(bytes).toString(),
        );
      },
      (response) {
        final index =
            (jsonDecode(response['package_index_json']! as String) as Map)
                .cast<String, Object?>();
        index['physical_chunk_count'] = 0x8000000000000000;
        final encoded = jsonEncode(index);
        final bytes = utf8.encode(encoded);
        response['package_index_json'] = encoded;
        response['package_index_seal'] = _seal(
          bytes.length,
          crypto.sha256.convert(bytes).toString(),
        );
      },
      (response) {
        final index =
            (jsonDecode(response['package_index_json']! as String) as Map)
                .cast<String, Object?>();
        final candidates = index['candidates']! as List<Object?>;
        candidates[1] = Map<String, Object?>.from(
          candidates.first! as Map<String, Object?>,
        );
        final encoded = jsonEncode(index);
        final bytes = utf8.encode(encoded);
        response['package_index_json'] = encoded;
        response['package_index_seal'] = _seal(
          bytes.length,
          crypto.sha256.convert(bytes).toString(),
        );
      },
    ];
    for (final mutate in mutations) {
      final response = _response();
      mutate(response);
      await _expectMalformed(response);
    }
  });

  test('rejects unsafe request paths before native dispatch', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_read_revision3_dataasset_package_index_v1':
            _response(),
      },
    );
    final ffi = ModFfi(core);
    final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());
    for (final operation
        in <Future<AuthoringRevision3DataAssetPackageIndexResult> Function()>[
          () => ffi.authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: '$_root\u0000forged',
            gameRoot: _gameRoot,
            expectedHead: head,
          ),
          () => ffi.authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: _root,
            gameRoot: '$_gameRoot\u0000forged',
            expectedHead: head,
          ),
        ]) {
      await expectLater(operation(), throwsA(isA<ArgumentError>()));
    }
    expect(core.calls, isEmpty);
  });

  test(
    'installed inspection sends only sealed ordinal authority and parses nested evidence',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_dataasset_package_index_v1':
              _response(),
          'authoring_store_inspect_revision3_installed_dataasset_v1':
              _installedInspectionResponse(),
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());
      final snapshot = await ffi
          .authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
          );
      final candidate = snapshot.index.candidates[1];

      final result = await ffi
          .authoringStoreInspectRevision3InstalledDataAssetV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
            expectedSnapshot: snapshot,
            candidate: candidate,
          );

      expect(core.calls.last.payload, <String, Object?>{
        'candidate_ordinal': 1,
        'expected_head_json': _headJson(),
        'expected_package_index_seal': _response()['package_index_seal'],
        'expected_source_snapshot_seal': _response()['source_snapshot_seal'],
        'game_root': _gameRoot,
        'root': _root,
      });
      expect(core.calls.last.payload, isNot(contains('target_path')));
      expect(core.calls.last.payload, isNot(contains('package_id_hex')));
      expect(result.candidateOrdinal, 1);
      expect(result.targetPath, '/Game/Characters/DA_Viper');
      expect(result.inspection.summary.editableLeaves, 1);
      expect(
        result.authorityStatus,
        AuthoringRevision3InstalledDataAssetAuthorityStatus.notGranted,
      );
    },
  );

  test(
    'installed edit sends only sealed proof authority and parses its unpublished stage',
    () async {
      final customIndex =
          jsonDecode(_completeIndexJson()) as Map<String, Object?>;
      final customCandidates = customIndex['candidates']! as List<Object?>;
      customCandidates[1] = <String, Object?>{
        'target_path': revision3DataAssetTargetPath,
        'package_id_hex': 'e54f79b8fc97323c',
      };
      final snapshotResponse = _response(indexJson: jsonEncode(customIndex));
      final inspectionJson = validDataAssetInspectionResponse();
      (inspectionJson['binding']! as Map<String, Object?>)['usmap_sha256'] =
          '3' * 64;
      dataAssetSelector(inspectionJson)['usmap_sha256'] = '3' * 64;
      final inspectionResponse =
          _installedInspectionResponse(inspection: inspectionJson)
            ..['target_path'] = revision3DataAssetTargetPath
            ..['package_id_hex'] = 'e54f79b8fc97323c'
            ..['package_index_seal'] = snapshotResponse['package_index_seal']
            ..['usmap_content_seal'] = _seal(256, '3' * 64);
      final evidenceCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_dataasset_package_index_v1':
              snapshotResponse,
          'authoring_store_inspect_revision3_installed_dataasset_v1':
              inspectionResponse,
        },
      );
      final evidenceFfi = ModFfi(evidenceCore);
      final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());
      final snapshot = await evidenceFfi
          .authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
          );
      final candidate = snapshot.index.candidates[1];
      final inspection = await evidenceFfi
          .authoringStoreInspectRevision3InstalledDataAssetV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
            expectedSnapshot: snapshot,
            candidate: candidate,
          );
      final change = DataAssetSemanticValueEditor.fromLeaf(
        inspection.inspection.exports.single.leaves.single,
      ).changeScalar(value: '2');
      final intent = DataAssetInstalledSemanticEditIntent.fromInspection(
        snapshot: snapshot,
        candidate: candidate,
        inspection: inspection,
        change: change,
      );
      final fixture = Revision3DataAssetFixture.fromBasis(
        basisHead: head,
        basisProjectJson: _basisProjectJson(),
        targetPath: candidate.targetPath,
        selector: intent.selector.toJson(),
        replacementHex: '02000000',
      );
      final nativeFields = intent.toNativeFields();
      final response = fixture.prepareResponse()
        ..['intent_binding_sha256'] = intent.intentBindingSha256
        ..['installed_proof_binding_sha256'] =
            intent.installedProofBindingSha256
        ..['installed_source'] = <String, Object?>{
          'candidate_ordinal': candidate.ordinal,
          'format': 'gore.authoring.revision3-installed-dataasset-source.v1',
          'inspection_binding': nativeFields['expected_inspection_binding'],
          'package_index_seal': nativeFields['expected_package_index_seal'],
          'source_snapshot_seal': nativeFields['expected_source_snapshot_seal'],
          'usmap_content_seal': nativeFields['expected_usmap_content_seal'],
          'usmap_inventory_seal': nativeFields['expected_usmap_inventory_seal'],
        };
      final editCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_installed_dataasset_edit_v1':
              response,
        },
      );

      final prepared = await ModFfi(editCore)
          .authoringStorePrepareRevision3InstalledDataAssetEditV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
            intent: intent,
          );

      expect(prepared.stage.targetPath, candidate.targetPath);
      expect(prepared.installedSource?.candidateOrdinal, candidate.ordinal);
      expect(editCore.calls.single.payload, <String, Object?>{
        'expected_head_json': head.canonicalJson,
        ...nativeFields,
        'game_root': _gameRoot,
        'root': _root,
      });
      expect(editCore.calls.single.payload, isNot(contains('target_path')));
      expect(editCore.calls.single.payload, isNot(contains('package_id_hex')));
      expect(editCore.calls.single.payload, isNot(contains('receipt')));
      expect(editCore.calls.single.payload, isNot(contains('output')));

      for (final mutate in <void Function(Map<String, Object?>)>[
        (value) => value['installed_proof_binding_sha256'] = '0' * 64,
        (value) =>
            (value['installed_source']!
                    as Map<String, Object?>)['candidate_ordinal'] =
                0,
        (value) =>
            ((value['installed_source']!
                        as Map<String, Object?>)['package_index_seal']!
                    as Map<String, Object?>)['sha256'] =
                '0' * 64,
        (value) {
          final source = value['installed_source']! as Map<String, Object?>;
          final binding = source['inspection_binding']! as Map<String, Object?>;
          final uexp = binding['uexp']! as Map<String, Object?>;
          uexp['sha256'] = '0' * 64;
        },
        (value) =>
            (value['installed_source']!
                    as Map<String, Object?>)['target_path'] =
                revision3DataAssetTargetPath,
      ]) {
        final malformed = (jsonDecode(jsonEncode(response)) as Map)
            .cast<String, Object?>();
        mutate(malformed);
        final malformedCore = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{
            'authoring_store_prepare_revision3_installed_dataasset_edit_v1':
                malformed,
          },
        );
        await expectLater(
          ModFfi(
            malformedCore,
          ).authoringStorePrepareRevision3InstalledDataAssetEditV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: head,
            intent: intent,
          ),
          throwsA(
            isA<ModFfiException>().having(
              (error) => error.code,
              'code',
              ModFfiException.malformedNativeResponseCode,
            ),
          ),
        );
      }
    },
  );

  test('installed inspection rejects candidate and response drift', () async {
    Future<AuthoringRevision3DataAssetPackageIndexResult> snapshotFor(
      FakeGoreCoreFfiService core,
    ) => ModFfi(core).authoringStoreReadRevision3DataAssetPackageIndexV1(
      root: _root,
      gameRoot: _gameRoot,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
    );

    final firstCore = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_read_revision3_dataasset_package_index_v1':
            _response(),
      },
    );
    final secondCore = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_read_revision3_dataasset_package_index_v1':
            _response(),
      },
    );
    final first = await snapshotFor(firstCore);
    final second = await snapshotFor(secondCore);
    await expectLater(
      ModFfi(secondCore).authoringStoreInspectRevision3InstalledDataAssetV1(
        root: _root,
        gameRoot: _gameRoot,
        expectedHead: second.head,
        expectedSnapshot: second,
        candidate: first.index.candidates.first,
      ),
      throwsArgumentError,
    );
    expect(secondCore.calls, hasLength(1));

    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response['candidate_ordinal'] = 0,
      (response) => response['target_path'] = '/Game/Forged',
      (response) => response['package_id_hex'] = '0' * 16,
      (response) => response['package_index_seal'] = _digitSeal(1, 'e'),
      (response) => response['source_snapshot_seal'] = _digitSeal(1, 'f'),
      (response) => response['authority_status'] = 'granted',
      (response) =>
          (response['inspection']! as Map<String, Object?>)['ok'] = false,
      (response) => response['inspection'] = validDataAssetInspectionResponse(
        exportIndex: 1,
        packageExports: 2,
      ),
    ];
    for (final mutate in mutations) {
      final response = _installedInspectionResponse();
      mutate(response);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_dataasset_package_index_v1':
              _response(),
          'authoring_store_inspect_revision3_installed_dataasset_v1': response,
        },
      );
      final ffi = ModFfi(core);
      final snapshot = await ffi
          .authoringStoreReadRevision3DataAssetPackageIndexV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
          );
      await expectLater(
        ffi.authoringStoreInspectRevision3InstalledDataAssetV1(
          root: _root,
          gameRoot: _gameRoot,
          expectedHead: snapshot.head,
          expectedSnapshot: snapshot,
          candidate: snapshot.index.candidates[1],
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );
    }
  });
}
