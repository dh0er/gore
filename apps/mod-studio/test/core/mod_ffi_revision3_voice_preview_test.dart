import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_preview_fixture.dart';

const _root = r'C:\Projects\VoicePreview.goreproj';
const _previewRoot =
    r'C:\Temp\gore-mod-studio-voice-preview-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const _command = 'authoring_store_materialize_revision3_voice_take_preview_v1';
const _registerCommand =
    'authoring_store_register_revision3_voice_take_preview_v1';
const _releaseCommand =
    'authoring_store_release_revision3_voice_take_preview_v1';

void main() {
  test('handshake includes the sorted Voice preview command', () {
    expect(
      requiredStudioCoreCommands,
      containsAll(<String>[_registerCommand, _command, _releaseCommand]),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request wire is canonical, ordered, and exact-current', () {
    final request = revision3VoicePreviewRequest();
    final wire = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();

    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'line_id',
      'expected_line_revision',
      'localization_id',
      'expected_localization_revision',
      'expected_loc_id',
      'slot_id',
      'expected_slot_revision',
      'locale',
      'take_id',
      'expected_take_revision',
      'expected_asset',
    ]);
    expect((wire['expected_asset']! as Map).keys, <String>[
      'sha256',
      'byte_len',
      'logical_name',
    ]);
    for (final forbidden in <String>{
      'current_project_json',
      'game_root',
      'source',
      'output',
      'status',
      'codec',
      'preview_root',
    }) {
      expect(wire, isNot(contains(forbidden)));
    }

    final reordered = <String, Object?>{
      'expected_project_id': wire['expected_project_id'],
      for (final entry in wire.entries)
        if (entry.key != 'expected_project_id') entry.key: entry.value,
    };
    expect(
      () => AuthoringRevision3VoiceTakePreviewRequestV1.fromCanonicalJson(
        jsonEncode(reordered),
      ),
      throwsFormatException,
    );
  });

  test('transport sends only roots and the signed exact request', () async {
    final request = revision3VoicePreviewRequest();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _command: revision3VoicePreviewResponse(
          previewRoot: _previewRoot,
          request: request,
        ),
      },
    );

    final result = await ModFfi(core)
        .authoringStoreMaterializeRevision3VoiceTakePreviewV1(
          root: _root,
          cleanupToken: revision3VoicePreviewCleanupToken,
          previewRoot: _previewRoot,
          request: request,
        );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload.keys, <String>[
      'root',
      'cleanup_token',
      'voice_take_preview_request_json',
    ]);
    expect(
      core.calls.single.payload['voice_take_preview_request_json'],
      request.canonicalJson,
    );
    expect(result.basisHead.canonicalJson, request.expectedHead.canonicalJson);
    expect(result.projectRevision, 7);
    expect(result.takeId, revision3VoicePreviewTakeId);
    expect(result.asset.sha256, revision3VoicePreviewAssetSha256);
    expect(result.cleanupToken, revision3VoicePreviewCleanupToken);
    expect(result.previewPath, '$_previewRoot\\preview.ogg');
  });

  test(
    'strict receipt rejects authority, binding, path, and key smuggling',
    () async {
      final request = revision3VoicePreviewRequest();
      for (final mutate in <void Function(Map<String, Object?>)>[
        (response) => response['line_revision'] = 3,
        (response) => response['preview_path'] = r'C:\Temp\escaped.ogg',
        (response) => response['preview_authority'] = 'runtime_ready',
        (response) => response['project_write_status'] = 'performed',
        (response) => response['extra'] = true,
        (response) {
          final asset = (response['asset']! as Map).cast<String, Object?>();
          asset['sha256'] = ''.padLeft(64, 'c');
        },
      ]) {
        final response = revision3VoicePreviewResponse(
          previewRoot: _previewRoot,
          request: request,
        );
        mutate(response);
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{_command: response},
            ),
          ).authoringStoreMaterializeRevision3VoiceTakePreviewV1(
            root: _root,
            cleanupToken: revision3VoicePreviewCleanupToken,
            previewRoot: _previewRoot,
            request: request,
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

  test(
    'register and release wires are strict opaque lifecycle commands',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          _registerCommand: revision3VoicePreviewRegistrationResponse(
            previewRoot: _previewRoot,
          ),
          _releaseCommand: revision3VoicePreviewCleanupResponse(),
        },
      );
      final ffi = ModFfi(core);
      final registration = await ffi
          .authoringStoreRegisterRevision3VoiceTakePreviewV1(root: _root);
      expect(registration.cleanupToken, revision3VoicePreviewCleanupToken);
      expect(registration.previewRoot, _previewRoot);
      await ffi.authoringStoreReleaseRevision3VoiceTakePreviewV1(
        cleanupToken: registration.cleanupToken,
      );
      expect(core.calls[0].payload, <String, Object?>{'root': _root});
      expect(core.calls[1].payload, <String, Object?>{
        'cleanup_token': revision3VoicePreviewCleanupToken,
      });

      final invalidRegistration = revision3VoicePreviewRegistrationResponse(
        previewRoot: r'C:\Temp\gore-mod-studio-voice-preview-not-native',
      );
      expect(
        () => AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
          invalidRegistration,
        ),
        throwsFormatException,
      );

      const sensitiveRelativeRoot = r'secret\managed-store';
      try {
        await ffi.authoringStoreRegisterRevision3VoiceTakePreviewV1(
          root: sensitiveRelativeRoot,
        );
        fail('relative Store root must fail preflight');
      } on ArgumentError catch (error) {
        expect(error.toString(), isNot(contains(sensitiveRelativeRoot)));
      }
    },
  );
}
