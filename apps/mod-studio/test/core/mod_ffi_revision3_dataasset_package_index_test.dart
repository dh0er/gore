import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../dataasset/dataasset_test_fixtures.dart';

const _root = r'C:\Projects\DataAssetBrowser.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '31313131313131313131313131313131';

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
