import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _projectId = '00000000000000000000000000000003';
const _targetSha =
    '5555555555555555555555555555555555555555555555555555555555555555';

String _headJson({int revision = 7}) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': revision + 1,
    'sha256': revision.toRadixString(16).padLeft(64, '0'),
  },
});

Map<String, Object?> _index({
  String projectId = _projectId,
  int projectRevision = 7,
  Object? targetByteLength = 1,
}) => <String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': projectRevision,
  'project_name': 'Content fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': targetByteLength,
      'sha256': _targetSha,
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
};

Map<String, Object?> _response({
  String? headJson,
  String projectId = _projectId,
  int projectRevision = 7,
  String? indexJson,
}) => <String, Object?>{
  'ok': true,
  'head_json': headJson ?? _headJson(),
  'project_id': projectId,
  'project_revision': projectRevision,
  'index_json': indexJson ?? jsonEncode(_index()),
  'content_authority': 'read_only_exact_current_project',
  'build_status': 'not_evaluated',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_applicable',
};

void main() {
  test(
    'content-index wrapper is exact-head read-only and strictly typed',
    () async {
      final response = _response();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_content_index_v1': response,
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());

      final result = await ffi.authoringStoreReadRevision3ContentIndexV1(
        root: r'C:\Mods\Content.goreproj',
        expectedHead: head,
      );

      expect(result.head.canonicalJson, head.canonicalJson);
      expect(result.projectId, _projectId);
      expect(result.projectRevision, 7);
      expect(result.indexJson, jsonEncode(_index()));
      expect(result.index.projectId, _projectId);
      expect(result.index.projectRevision, 7);
      expect(
        result.contentAuthority,
        AuthoringRevision3ContentAuthority.readOnlyExactCurrentProject,
      );
      expect(
        result.buildStatus,
        AuthoringRevision3ContentBuildStatus.notEvaluated,
      );
      expect(
        result.runtimeStatus,
        AuthoringRevision3ContentRuntimeStatus.runtimeUnqualified,
      );
      expect(
        result.publicationStatus,
        AuthoringRevision3ContentPublicationStatus.notApplicable,
      );
      expect(core.calls, hasLength(1));
      expect(
        core.calls.single.command,
        'authoring_store_read_revision3_content_index_v1',
      );
      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': head.canonicalJson,
        'root': r'C:\Mods\Content.goreproj',
      });
    },
  );

  test('content-index response rejects loose claims and false bindings', () {
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(_headJson());
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('ok'),
      (response) => response.remove('index_json'),
      (response) => response['ok'] = false,
      (response) => response['unknown'] = true,
      (response) => response['head_json'] = _headJson(revision: 8),
      (response) =>
          response['project_id'] = List<String>.filled(32, '0').join(),
      (response) => response['project_revision'] = -1,
      (response) => response['project_revision'] = 0x8000000000000000,
      (response) => response['content_authority'] = 'mutable',
      (response) => response['build_status'] = 'ready',
      (response) => response['runtime_status'] = 'qualified',
      (response) => response['publication_status'] = 'published',
      (response) => response['index_json'] = jsonEncode(
        _index(projectId: '04040404040404040404040404040404'),
      ),
      (response) =>
          response['index_json'] = jsonEncode(_index(projectRevision: 8)),
      (response) => response['index_json'] = jsonEncode(
        _index(targetByteLength: 0x8000000000000000),
      ),
      (response) => response['index_json'] = jsonEncode(_index()).replaceFirst(
        '"project_revision":7',
        '"project_revision":7,"project_revision":7',
      ),
      (response) => response['index_json'] = ' ${jsonEncode(_index())}',
    ];

    for (final mutate in mutations) {
      final response = _response();
      mutate(response);
      expect(
        () => AuthoringRevision3ContentIndexResult.fromJson(
          response,
          expectedHead: expectedHead,
        ),
        throwsFormatException,
      );
    }
  });

  test(
    'content-index wrapper rejects bad paths before native dispatch',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_read_revision3_content_index_v1': _response(),
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());

      for (final root in <String>[
        '',
        'root\u0000tail',
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ffi.authoringStoreReadRevision3ContentIndexV1(
            root: root,
            expectedHead: head,
          ),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    },
  );
}
