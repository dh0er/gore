import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

void main() {
  test(
    'history wrapper preserves exact basis and returns a closed lineage',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_list_revision3_history_v1': _historyResponse(),
        },
      );
      final ffi = ModFfi(core);
      final result = await ffi.authoringStoreListRevision3HistoryV1(
        root: r'C:\Mods\History.goreproj',
        expectedHead: _head(7),
      );

      expect(result.basisHead.canonicalJson, _head(7).canonicalJson);
      expect(result.projectRevision, 7);
      expect(result.entries.map((entry) => entry.projectRevision), [7, 6, 5]);
      expect(result.entries.first.current, isTrue);
      expect(result.entries.skip(1).every((entry) => !entry.current), isTrue);
      expect(result.historyTruncated, isFalse);
      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': _head(7).canonicalJson,
        'root': r'C:\Mods\History.goreproj',
      });
    },
  );

  test('history response rejects widened authority and broken lineage', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response['history_authority'] = 'cas_scan',
      (response) => response['project_mutation'] = 'performed',
      (response) => response['publication_status'] = 'supported',
      (response) => response['unknown'] = true,
      (response) => (response['entries'] as List<Object?>).removeAt(0),
      (response) {
        final entries = response['entries'] as List<Object?>;
        (entries[1] as Map<String, Object?>)['project_revision'] = 4;
      },
      (response) {
        final entries = response['entries'] as List<Object?>;
        (entries[1] as Map<String, Object?>)['current'] = true;
      },
      (response) {
        final entries = response['entries'] as List<Object?>;
        (entries[1] as Map<String, Object?>)['head_json'] = _head(
          7,
        ).canonicalJson;
      },
    ];
    for (final mutate in mutations) {
      final response = _historyResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3ProjectHistoryResult.fromJson(
          response,
          expectedHead: _head(7),
        ),
        throwsFormatException,
      );
    }
  });

  test(
    'restore wrapper accepts only an unpublished current+1 candidate',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_history_restore_v1':
              _restoreResponse(),
        },
      );
      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3HistoryRestoreV1(
            root: r'C:\Mods\History.goreproj',
            expectedHead: _head(7),
            targetHead: _head(5),
          );

      expect(prepared.basisHead.canonicalJson, _head(7).canonicalJson);
      expect(prepared.directParentHead.canonicalJson, _head(7).canonicalJson);
      expect(prepared.restoredFromHead.canonicalJson, _head(5).canonicalJson);
      expect(prepared.head.canonicalJson, _head(8).canonicalJson);
      expect(prepared.previousProjectRevision, 7);
      expect(prepared.revision, 8);
      expect(prepared.restoredFromRevision, 5);
      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': _head(7).canonicalJson,
        'root': r'C:\Mods\History.goreproj',
        'target_head_json': _head(5).canonicalJson,
      });
    },
  );

  test(
    'restore response rejects wrong parent, target, revision, and authority',
    () {
      final mutations = <void Function(Map<String, Object?>)>[
        (response) =>
            response['direct_parent_head_json'] = _head(6).canonicalJson,
        (response) =>
            response['restored_from_head_json'] = _head(4).canonicalJson,
        (response) => response['revision'] = 9,
        (response) => response['restored_from_revision'] = 7,
        (response) => response['project_mutation'] = 'published',
        (response) => response['publication_status'] = 'published',
        (response) => response['game_mutation'] = 'performed',
        (response) => response['unknown'] = true,
      ];
      for (final mutate in mutations) {
        final response = _restoreResponse();
        mutate(response);
        expect(
          () => AuthoringRevision3ProjectHistoryRestorePreparation.fromJson(
            response,
            expectedHead: _head(7),
            targetHead: _head(5),
          ),
          throwsFormatException,
        );
      }
    },
  );
}

const _projectId = '11111111111111111111111111111111';

Map<String, Object?> _historyResponse() => <String, Object?>{
  'ok': true,
  'outcome': 'listed_exact_current',
  'basis_head_json': _head(7).canonicalJson,
  'project_id': _projectId,
  'project_revision': 7,
  'entries': <Object?>[
    for (var revision = 7; revision >= 5; revision--)
      <String, Object?>{
        'head_json': _head(revision).canonicalJson,
        'project_id': _projectId,
        'project_revision': revision,
        'current': revision == 7,
      },
  ],
  'history_truncated': false,
  'history_authority': 'authenticated_bounded_history',
  'project_mutation': 'not_performed',
  'game_mutation': 'not_performed',
  'save_mutation': 'not_performed',
  'build_status': 'not_performed',
  'deployment_status': 'not_performed',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_applicable',
};

Map<String, Object?> _restoreResponse() => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_restore_unpublished',
  'basis_head_json': _head(7).canonicalJson,
  'direct_parent_head_json': _head(7).canonicalJson,
  'restored_from_head_json': _head(5).canonicalJson,
  'head_json': _head(8).canonicalJson,
  'project_json': _projectJson(8),
  'project_id': _projectId,
  'previous_project_revision': 7,
  'revision': 8,
  'restored_from_revision': 5,
  'history_authority': 'authenticated_bounded_history',
  'project_mutation': 'prepared_not_published',
  'game_mutation': 'not_performed',
  'save_mutation': 'not_performed',
  'build_status': 'not_performed',
  'deployment_status': 'not_performed',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_supported',
};

AuthoringWorkingHead _head(int revision) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': revision + 1,
          'sha256': revision.toRadixString(16).padLeft(64, '0'),
        },
      }),
    );

String _projectJson(int revision) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': revision,
  'meta': <String, Object?>{
    'name': 'History fixture',
    'version': '1.0',
    'author': 'Tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, 'a').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});
