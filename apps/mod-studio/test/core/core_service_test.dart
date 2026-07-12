import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';

String _coreInfoResponse({
  int abi = goreCoreAbi,
  String version = '0.1.0-test',
  List<String> commands = requiredStudioCoreCommands,
}) => jsonEncode({
  'ok': true,
  'abi': abi,
  'version': version,
  'commands': commands,
});

Pointer<Uint8> _publishResponse(
  Pointer<GoreCoreResponseV2> out,
  List<int> bytes, {
  int? claimedLength,
}) {
  final allocation = malloc<Uint8>(bytes.length);
  allocation.asTypedList(bytes.length).setAll(0, bytes);
  out.ref.data = allocation;
  out.ref.len = claimedLength ?? bytes.length;
  out.ref.handle = allocation.cast<Void>();
  return allocation;
}

void main() {
  group('canonical native response decoding', () {
    test('retains one compact exact object', () {
      expect(decodeCanonicalGoreCoreResponse('{"ok":true,"value":"x"}'), {
        'ok': true,
        'value': 'x',
      });
    });

    test('rejects duplicates and normalized JSON spellings', () {
      for (final response in [
        '{"ok":true,"ok":false}',
        ' {"ok":true}',
        '{"ok" : true}',
        '{"ok":true,"value":"\\u0078"}',
        '[]',
        '{',
      ]) {
        expect(
          () => decodeCanonicalGoreCoreResponse(response),
          throwsFormatException,
        );
      }
    });
  });

  group('GoreCoreInfo', () {
    test('strictly parses a canonical bounded response', () {
      final info = GoreCoreInfo.parseResponse(_coreInfoResponse());

      expect(info.abi, goreCoreAbi);
      expect(info.version, '0.1.0-test');
      expect(info.commands, requiredStudioCoreCommands);
      expect(info.isStudioCompatible, isTrue);
      expect(info.missingRequiredCommands, isEmpty);
      expect(() => info.commands.add('validate'), throwsUnsupportedError);
    });

    test(
      'candidate decision skips legacy, wrong ABI, and incomplete cores',
      () {
        final legacy = jsonEncode({
          'ok': false,
          'error': {
            'code': 'UNKNOWN_COMMAND',
            'message': 'unknown command: core_info',
          },
        });
        final wrongAbi = _coreInfoResponse(abi: goreCoreAbi + 1);
        final missingAuthoring = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'authoring_project_check')
              .toList(),
        );
        final missingVoice = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'voice_archive_match_line')
              .toList(),
        );
        final missingWorkingStore = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where(
                (command) => command != 'authoring_store_prepare_checkpoint',
              )
              .toList(),
        );
        final missingDocumentStore = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'authoring_store_open_document')
              .toList(),
        );
        final missingStoryTransaction = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where(
                (command) =>
                    command != 'authoring_project_story_draft_insert_v1',
              )
              .toList(),
        );
        final missingStoryCatalog = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'authoring_story_catalog_v1_read')
              .toList(),
        );
        final missingStoryCatalogBuild = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'authoring_story_catalog_v1_build')
              .toList(),
        );
        final missingNpcDraft = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where(
                (command) =>
                    command != 'authoring_logical_npc_clone_draft_v1_generate',
              )
              .toList(),
        );
        final missingQuestDraft = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where(
                (command) =>
                    command != 'authoring_draft_quest_skeleton_v1_generate',
              )
              .toList(),
        );
        final current = _coreInfoResponse(version: '0.2.0-current');

        final decisions = [
          legacy,
          wrongAbi,
          missingAuthoring,
          missingVoice,
          missingWorkingStore,
          missingDocumentStore,
          missingStoryTransaction,
          missingStoryCatalog,
          missingStoryCatalogBuild,
          missingNpcDraft,
          missingQuestDraft,
          current,
        ].map(GoreCoreInfo.tryParseCompatibleResponse).toList();
        expect(decisions.take(11), everyElement(isNull));
        expect(decisions.last?.version, '0.2.0-current');
      },
    );

    test('bounded transport probe is mandatory even with valid core_info', () {
      final response = _coreInfoResponse();

      expect(
        GoreCoreInfo.tryParseCompatibleTransportV2Response(1, response),
        isNull,
      );
      expect(
        GoreCoreInfo.tryParseCompatibleTransportV2Response(3, response),
        isNull,
      );
      expect(
        GoreCoreInfo.tryParseCompatibleTransportV2Response(
          goreCoreTransportAbiV2,
          response,
        ),
        isNotNull,
      );
    });

    test('reports every missing Studio capability deterministically', () {
      final commands = requiredStudioCoreCommands
          .where(
            (command) =>
                command != 'authoring_project_check' &&
                command != 'authoring_project_story_draft_insert_v1' &&
                command != 'authoring_story_catalog_v1_build' &&
                command != 'authoring_story_catalog_v1_read' &&
                command != 'authoring_store_open_document' &&
                command != 'voice_archive_match_line' &&
                command != 'authoring_logical_npc_clone_draft_v1_generate' &&
                command != 'authoring_draft_quest_skeleton_v1_generate',
          )
          .toList();
      final info = GoreCoreInfo.parseResponse(
        _coreInfoResponse(commands: commands),
      );

      expect(info.isStudioCompatible, isFalse);
      expect(info.missingRequiredCommands, [
        'authoring_draft_quest_skeleton_v1_generate',
        'authoring_logical_npc_clone_draft_v1_generate',
        'authoring_project_check',
        'authoring_project_story_draft_insert_v1',
        'authoring_store_open_document',
        'authoring_story_catalog_v1_build',
        'authoring_story_catalog_v1_read',
        'voice_archive_match_line',
      ]);
    });

    test('rejects non-exact, non-canonical, and oversized responses', () {
      final unsortedCommands = requiredStudioCoreCommands.reversed.toList();
      final duplicateCommands = [...requiredStudioCoreCommands]
        ..insert(1, requiredStudioCoreCommands.first);
      final cases = <String>[
        '[]',
        '{',
        jsonEncode({
          'ok': true,
          'abi': goreCoreAbi,
          'version': '0.1.0',
          'commands': requiredStudioCoreCommands,
          'extra': true,
        }),
        jsonEncode({
          'ok': true,
          'abi': 1.0,
          'version': '0.1.0',
          'commands': requiredStudioCoreCommands,
        }),
        _coreInfoResponse(version: 'bad version'),
        _coreInfoResponse(commands: unsortedCommands),
        _coreInfoResponse(commands: duplicateCommands),
        _coreInfoResponse(commands: const ['Not_Canonical']),
        ' ' * (64 * 1024 + 1),
      ];

      for (final response in cases) {
        expect(
          () => GoreCoreInfo.parseResponse(response),
          throwsFormatException,
          reason: response.length < 100 ? response : 'bounded response',
        );
        expect(GoreCoreInfo.tryParseCompatibleResponse(response), isNull);
      }
    });
  });

  group('length-aware core transport', () {
    test('passes exact non-NUL UTF-8 bytes and frees the handle once', () {
      const request = '{"text":"Grüße 😀"}';
      List<int>? capturedRequest;
      var frees = 0;
      final responseBytes = utf8.encode('{"ok":true}');

      final response = executeGoreCoreV2WithBindings(
        (requestPointer, requestLength, out) {
          capturedRequest = List<int>.of(
            requestPointer.asTypedList(requestLength),
          );
          _publishResponse(out, responseBytes);
          return 0;
        },
        (handle) {
          frees++;
          malloc.free(handle);
        },
        request,
      );

      expect(capturedRequest, utf8.encode(request));
      expect(response, '{"ok":true}');
      expect(frees, 1);
    });

    test('checks bounded UTF-8 length before allocating request bytes', () {
      var calls = 0;
      int execute(
        Pointer<Uint8> request,
        int requestLength,
        Pointer<GoreCoreResponseV2> out,
      ) {
        calls++;
        _publishResponse(out, utf8.encode('{"ok":true}'));
        return 0;
      }

      void free(Pointer<Void> handle) => malloc.free(handle);

      final oversizedCases = <({String value, int limit})>[
        (value: 'a' * (1024 * 1024), limit: 1024 * 1024 - 1),
        (value: 'é', limit: 1),
        (value: '😀', limit: 3),
        (value: String.fromCharCode(0xd800), limit: 2),
        (value: String.fromCharCode(0xdc00), limit: 2),
        (value: String.fromCharCodes([0xd800, 0x61]), limit: 3),
      ];
      for (final testCase in oversizedCases) {
        final response = executeGoreCoreV2WithBindings(
          execute,
          free,
          testCase.value,
          requestLimitBytes: testCase.limit,
        );
        expect(
          (jsonDecode(response) as Map<String, dynamic>)['error']['code'],
          'FFI_REQUEST_LIMIT',
        );
      }
      expect(calls, 0);

      final exactBoundaryCases = <({String value, int limit})>[
        (value: 'é', limit: 2),
        (value: '😀', limit: 4),
        (value: String.fromCharCode(0xd800), limit: 3),
        (value: String.fromCharCode(0xdc00), limit: 3),
        (value: String.fromCharCodes([0xd800, 0x61]), limit: 4),
      ];
      for (final testCase in exactBoundaryCases) {
        final response = executeGoreCoreV2WithBindings(
          execute,
          free,
          testCase.value,
          requestLimitBytes: testCase.limit,
        );
        expect(jsonDecode(response), {'ok': true});
      }
      expect(calls, exactBoundaryCases.length);
    });

    test('rejects response length before reading and still frees once', () {
      var frees = 0;
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) {
            _publishResponse(out, [0], claimedLength: 2);
            return 0;
          },
          (handle) {
            frees++;
            malloc.free(handle);
          },
          '{}',
          responseLimitBytes: 1,
        ),
        throwsFormatException,
      );
      expect(frees, 1);
    });

    test('rejects malformed UTF-8 and frees the handle once', () {
      var frees = 0;
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) {
            _publishResponse(out, [0xff]);
            return 0;
          },
          (handle) {
            frees++;
            malloc.free(handle);
          },
          '{}',
        ),
        throwsFormatException,
      );
      expect(frees, 1);
    });

    test('recovers and frees a handle when injected execute throws', () {
      var frees = 0;
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) {
            _publishResponse(out, utf8.encode('{}'));
            throw StateError('injected execute failure');
          },
          (handle) {
            frees++;
            malloc.free(handle);
          },
          '{}',
        ),
        throwsStateError,
      );
      expect(frees, 1);
    });

    test('uses exact response length instead of truncating at NUL', () {
      var frees = 0;
      final response = executeGoreCoreV2WithBindings(
        (request, requestLength, out) {
          _publishResponse(out, [0x7b, 0x7d, 0, 0x78]);
          return 0;
        },
        (handle) {
          frees++;
          malloc.free(handle);
        },
        '{}',
      );

      expect(response.codeUnits, [0x7b, 0x7d, 0, 0x78]);
      expect(() => jsonDecode(response), throwsFormatException);
      expect(frees, 1);
    });

    test('maps known statuses and rejects unknown or inconsistent output', () {
      final invalidArguments = executeGoreCoreV2WithBindings(
        (request, requestLength, out) => 1,
        (_) => fail('empty failed response must not be freed'),
        '{}',
      );
      final panic = executeGoreCoreV2WithBindings(
        (request, requestLength, out) => 2,
        (_) => fail('empty failed response must not be freed'),
        '{}',
      );
      expect(
        (jsonDecode(invalidArguments) as Map<String, dynamic>)['error']['code'],
        'CORE_TRANSPORT_INVALID_ARGUMENT',
      );
      expect(
        (jsonDecode(panic) as Map<String, dynamic>)['error']['code'],
        'CORE_TRANSPORT_PANIC',
      );
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) => 99,
          (_) => fail('empty failed response must not be freed'),
          '{}',
        ),
        throwsFormatException,
      );

      var frees = 0;
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) {
            _publishResponse(out, utf8.encode('{}'));
            return 1;
          },
          (handle) {
            frees++;
            malloc.free(handle);
          },
          '{}',
        ),
        throwsFormatException,
      );
      expect(frees, 1);
    });

    test('status zero requires a complete non-empty response descriptor', () {
      expect(
        () => executeGoreCoreV2WithBindings(
          (request, requestLength, out) => 0,
          (_) => fail('incomplete response has no owned handle'),
          '{}',
        ),
        throwsFormatException,
      );
    });
  });

  test(
    'FakeGoreCoreFfiService records calls and returns canned response',
    () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'validate_override': {'ok': true, 'data': {}},
        },
      );
      final result = await fake.execute(
        'validate_override',
        payload: {'class': 'ItFo_Apple', 'field': 'm_Value', 'value': 500},
      );
      expect(result['ok'], isTrue);
      expect(fake.calls, hasLength(1));
      expect(fake.calls.first.command, 'validate_override');
      expect(fake.calls.first.payload['class'], 'ItFo_Apple');
    },
  );

  test('MissingGoreCoreFfiService returns CORE_UNAVAILABLE', () async {
    final svc = MissingGoreCoreFfiService();
    final result = await svc.execute('validate_override');
    expect(result['ok'], isFalse);
    final err = result['error'] as Map<String, Object?>;
    expect(err['code'], 'CORE_UNAVAILABLE');
    expect(svc.description, contains('bounded-transport'));
    expect(err['message'], contains('bounded-transport'));
  });
}
