import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

String _validHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":321,'
    '"sha256":"${List.filled(64, 'a').join()}"}}';

String _validRevision3ProjectJson() =>
    '{"format":2,"schema_revision":3,'
    '"project_id":"00000000000000000000000000000003","revision":7,'
    '"meta":{"name":"Revision 3 Store","version":"1.0.0","author":"tests"},'
    '"target":{"executable":{"byte_len":1,'
    '"sha256":"${List.filled(64, '5').join()}"}},'
    '"authoring_locales":[],"entities":{},"asset_store":{"assets":{}}}';

Map<String, Object?> _validOpenedResponse() => <String, Object?>{
  'ok': true,
  'head_json': _validHeadJson(),
  'project_json': _validRevision3ProjectJson(),
};

Map<String, Object?> _validPreparedResponse() => <String, Object?>{
  'ok': true,
  'head_json': _validHeadJson(),
};

void main() {
  test('revision-3 Store commands are mandatory Studio capabilities', () {
    expect(
      requiredStudioCoreCommands.where(
        (command) => command.contains('revision3'),
      ),
      <String>[
        'authoring_store_open_revision3',
        'authoring_store_open_revision3_head_bytes',
        'authoring_store_prepare_revision3_checkpoint',
      ],
    );
  });

  test(
    'revision-3 Store wrappers preserve exact nested strings and payloads',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _validOpenedResponse(),
          'authoring_store_open_revision3_head_bytes': _validOpenedResponse(),
          'authoring_store_prepare_revision3_checkpoint':
              _validPreparedResponse(),
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(_validHeadJson());
      const root = 'C:\\Mods\\Revision "Three".goreproj';
      const rawDuplicateProject =
          '{"schema_revision":3,"revision":0,"revision":1}';

      final opened = await ffi.authoringStoreOpenRevision3(
        root: root,
        verification: AuthoringAssetVerification.structural,
      );
      final reopened = await ffi.authoringStoreOpenRevision3HeadBytes(
        root: root,
        head: head,
        verification: AuthoringAssetVerification.full,
      );
      final preparedAbsent = await ffi.authoringStorePrepareRevision3Checkpoint(
        root: root,
        expectedHead: null,
        projectJson: rawDuplicateProject,
      );
      final preparedCas = await ffi.authoringStorePrepareRevision3Checkpoint(
        root: root,
        expectedHead: head,
        projectJson: rawDuplicateProject,
      );

      expect(opened.projectJson, _validRevision3ProjectJson());
      expect(opened.projectId, '00000000000000000000000000000003');
      expect(opened.projectRevision, 7);
      expect(opened.head.canonicalJson, _validHeadJson());
      expect(reopened.projectJson, _validRevision3ProjectJson());
      expect(preparedAbsent.head.canonicalJson, _validHeadJson());
      expect(preparedCas.head.canonicalJson, _validHeadJson());
      expect(core.calls, hasLength(4));
      expect(core.calls[0].command, 'authoring_store_open_revision3');
      expect(core.calls[0].payload, <String, Object?>{
        'root': root,
        'verification': 'structural',
      });
      expect(
        core.calls[1].command,
        'authoring_store_open_revision3_head_bytes',
      );
      expect(core.calls[1].payload, <String, Object?>{
        'root': root,
        'head_json': _validHeadJson(),
        'verification': 'full',
      });
      expect(
        core.calls[2].command,
        'authoring_store_prepare_revision3_checkpoint',
      );
      expect(core.calls[2].payload, <String, Object?>{
        'root': root,
        'expected_head_json': null,
        'project_json': rawDuplicateProject,
      });
      expect(core.calls[3].payload, <String, Object?>{
        'root': root,
        'expected_head_json': _validHeadJson(),
        'project_json': rawDuplicateProject,
      });
    },
  );

  test('revision-3 open DTO rejects loose fields, types, and claims', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('ok'),
      (response) => response.remove('head_json'),
      (response) => response.remove('project_json'),
      (response) => response['ok'] = false,
      (response) => response['ok'] = 'true',
      (response) => response['head_json'] = 1,
      (response) => response['project_json'] = true,
      (response) => response['unknown'] = true,
      (response) => response['diagnostics'] = <Object?>[],
      (response) => response['blocks_build'] = false,
      (response) => response['readiness'] = 'ready',
      (response) => response['publication_status'] = 'supported',
    ];
    for (final mutate in mutations) {
      final response = _validOpenedResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3StoreOpenedResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'revision-3 open DTO accounts exact UTF-8 bounds and canonical bytes',
    () {
      final badNestedStrings = <void Function(Map<String, Object?>)>[
        (response) => response['head_json'] = String.fromCharCodes(
          Uint8List(64 * 1024 + 1),
        ),
        (response) => response['project_json'] = String.fromCharCodes(
          Uint8List(16 * 1024 * 1024 + 1),
        ),
        (response) => response['head_json'] = String.fromCharCode(0xd800),
        (response) => response['project_json'] = String.fromCharCode(0xd800),
        (response) =>
            response['project_json'] = ' ${_validRevision3ProjectJson()}',
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst('"schema_revision":3', '"schema_revision":2'),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst('"revision":7', '"revision":7,"revision":7'),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"name":"Revision 3 Store"',
              '"name":"Revision 3 Store","name":"shadow"',
            ),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"project_id":"00000000000000000000000000000003"',
              '"project_id":"00000000000000000000000000000000"',
            ),
        (response) => response['project_json'] = _validRevision3ProjectJson()
            .replaceFirst(
              '"format":2,"schema_revision":3',
              '"schema_revision":3,"format":2',
            ),
      ];
      for (final mutate in badNestedStrings) {
        final response = _validOpenedResponse();
        mutate(response);
        expect(
          () => AuthoringRevision3StoreOpenedResult.fromJson(response),
          throwsFormatException,
        );
      }
    },
  );

  test('revision-3 prepare DTO is exact and exposes no authority claims', () {
    expect(
      AuthoringRevision3CheckpointPreparation.fromJson(
        _validPreparedResponse(),
      ).head.canonicalJson,
      _validHeadJson(),
    );

    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response.remove('ok'),
      (response) => response.remove('head_json'),
      (response) => response['ok'] = false,
      (response) => response['ok'] = 1,
      (response) => response['head_json'] = false,
      (response) => response['head_json'] = String.fromCharCodes(
        Uint8List(64 * 1024 + 1),
      ),
      (response) => response['head_json'] = String.fromCharCode(0xd800),
      (response) => response['unknown'] = true,
      (response) => response['diagnostics'] = <Object?>[],
      (response) => response['blocks_build'] = false,
      (response) => response['readiness'] = 'ready',
      (response) => response['publication_status'] = 'supported',
      (response) => response['project_json'] = _validRevision3ProjectJson(),
    ];
    for (final mutate in mutations) {
      final response = _validPreparedResponse();
      mutate(response);
      expect(
        () => AuthoringRevision3CheckpointPreparation.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test(
    'revision-3 Store requests fail locally on unsafe or oversized strings',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _validOpenedResponse(),
          'authoring_store_prepare_revision3_checkpoint':
              _validPreparedResponse(),
        },
      );
      final ffi = ModFfi(core);

      for (final root in <String>[
        '',
        String.fromCharCodes(Uint8List(32 * 1024 + 1).map((_) => 0x78)),
        'root\u0000tail',
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ffi.authoringStoreOpenRevision3(
            root: root,
            verification: AuthoringAssetVerification.full,
          ),
          throwsArgumentError,
        );
      }

      for (final projectJson in <String>[
        '',
        String.fromCharCodes(Uint8List(16 * 1024 * 1024 + 1)),
        String.fromCharCode(0xd800),
        // Raw size is valid, but conservative JSON escaping exceeds the 64 MiB transport cap.
        String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
      ]) {
        await expectLater(
          ffi.authoringStorePrepareRevision3Checkpoint(
            root: 'root',
            expectedHead: null,
            projectJson: projectJson,
          ),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    },
  );
}
