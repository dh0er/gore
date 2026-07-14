import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

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
  test('Studio handshake requires the sorted package-index command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_read_revision3_dataasset_package_index_v1'),
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
}
