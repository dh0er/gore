import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_voice_preview_fixture.dart';

void main() {
  test(
    'catalog exposes only friendly preview affordance and exact hidden plan',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoicePreviewContentIndex(),
      );
      final line = catalog.line(revision3VoicePreviewLineId)!;
      final take = line
          .slotSummaryForLocale('de')!
          .candidate(revision3VoicePreviewTakeId)!;

      expect(take.canPreview, isTrue);
      expect(take.displayLabel, contains('Asghan take'));
      for (final hidden in <String>{
        revision3VoicePreviewTakeId,
        revision3VoicePreviewAssetSha256,
        revision3VoicePreviewLogicalName,
      }) {
        expect(take.displayLabel, isNot(contains(hidden)));
      }

      final plan = Revision3VoiceTakePreviewTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: line.lineId,
        locale: 'de',
        takeId: take.id,
      );
      expect(plan.expectedLineRevision, 2);
      expect(plan.localizationId, revision3VoicePreviewLocalizationId);
      expect(plan.expectedLocalizationRevision, 0);
      expect(plan.slotId, revision3VoicePreviewSlotId);
      expect(plan.expectedSlotRevision, 1);
      expect(plan.expectedTakeRevision, 0);
      expect(plan.assetSha256, revision3VoicePreviewAssetSha256);
      expect(plan.assetByteLength, revision3VoicePreviewAssetByteLength);
      expect(plan.assetLogicalName, revision3VoicePreviewLogicalName);
      expect(plan.codec, Revision3ContentVoiceOggCodec.vorbis);
      expect(plan.channels, 1);
      expect(plan.sampleRate, 48000);
    },
  );

  test(
    'fresh-index service rejects a stale checkpoint before materializing',
    () async {
      final checkpoint = Revision3VoiceCatalog.fromContentIndex(
        revision3VoicePreviewContentIndex(),
      );
      var materialized = false;
      final service = Revision3VoiceTakePreviewAuthoringService(
        loadContentIndex: () async =>
            revision3VoicePreviewContentIndex(revision: 8),
        materializeTechnicalPlan:
            ({
              required String expectedProjectId,
              required int expectedProjectRevision,
              required Revision3VoiceTakePreviewTechnicalPlan plan,
            }) async {
              materialized = true;
              throw StateError('must not run');
            },
      );

      await expectLater(
        service.materialize(
          checkpoint: checkpoint,
          lineId: revision3VoicePreviewLineId,
          locale: 'de',
          takeId: revision3VoicePreviewTakeId,
        ),
        throwsA(isA<Revision3VoiceTakePreviewStaleCheckpointException>()),
      );
      expect(materialized, isFalse);
    },
  );

  test(
    'capability verifies one exact file and concurrent close is idempotent',
    () async {
      final capability = await _materializeValidCapability();
      final root = Directory(p.dirname(capability.path));
      expect(capability.isClosed, isFalse);
      expect(
        await File(capability.path).readAsBytes(),
        revision3VoicePreviewBytes,
      );

      final first = capability.close();
      final second = capability.close();
      expect(identical(first, second), isTrue);
      await Future.wait(<Future<void>>[first, second]);

      expect(capability.isClosed, isTrue);
      expect(await root.exists(), isFalse);
      await capability.close();
    },
  );

  test(
    'failed close stays open and can be retried after a local lock shape clears',
    () async {
      final capability = await _materializeValidCapability();
      final root = Directory(p.dirname(capability.path));
      await File(capability.path).delete();
      await Directory(capability.path).create();

      await expectLater(
        capability.close(),
        throwsA(isA<Revision3VoiceTakePreviewCleanupException>()),
      );
      expect(capability.isClosed, isFalse);
      expect(await root.exists(), isTrue);

      await Directory(capability.path).delete();
      await capability.close();
      expect(capability.isClosed, isTrue);
      expect(await root.exists(), isFalse);
    },
  );

  test(
    'operation failure performs exact cleanup and preserves the primary error',
    () async {
      late String previewRoot;
      final native = _FakeNativePreviewCapability();
      await expectLater(
        Revision3VoiceTakePreviewCapability.materialize(
          register: () async {
            final registration = await native.register();
            previewRoot = registration.previewRoot;
            return registration;
          },
          materialize: (token, root) async {
            await native.writePreview(root);
            throw StateError('materialization failed');
          },
          release: native.release,
        ),
        throwsA(
          isA<StateError>().having(
            (error) => error.message,
            'message',
            'materialization failed',
          ),
        ),
      );
      expect(await Directory(previewRoot).exists(), isFalse);
    },
  );

  test(
    'double failure retains bounded cleanup ownership until retry succeeds',
    () async {
      late String previewRoot;
      late Revision3VoiceTakePreviewMaterializationCleanupException retained;
      final native = _FakeNativePreviewCapability();
      try {
        await Revision3VoiceTakePreviewCapability.materialize(
          register: () async {
            final registration = await native.register();
            previewRoot = registration.previewRoot;
            return registration;
          },
          materialize: (token, root) async {
            await File(
              p.join(root, 'unexpected'),
            ).writeAsString('owned blocker');
            throw StateError('primary failure');
          },
          release: native.release,
        );
        fail('materialization must fail');
      } on Revision3VoiceTakePreviewMaterializationCleanupException catch (
        error
      ) {
        retained = error;
      }

      expect(retained.materializationCause, isA<StateError>());
      expect(retained.cause, isA<FileSystemException>());
      expect(retained.diagnosticPreviewRoot, previewRoot);
      expect(retained.toString(), isNot(contains(previewRoot)));
      expect(retained.toString(), isNot(contains('preview.ogg')));
      expect(
        retained.toString(),
        isNot(contains(revision3VoicePreviewCleanupToken)),
      );
      expect(retained.isCleaned, isFalse);
      expect(await Directory(previewRoot).exists(), isTrue);

      late Revision3VoiceTakePreviewCleanupException retryError;
      try {
        await retained.retryCleanup();
        fail('blocked cleanup retry must fail');
      } on Revision3VoiceTakePreviewCleanupException catch (error) {
        retryError = error;
      }
      expect(retryError.toString(), isNot(contains(previewRoot)));
      expect(retryError.toString(), isNot(contains('preview.ogg')));
      expect(
        retryError.toString(),
        isNot(contains(revision3VoicePreviewCleanupToken)),
      );
      expect(retained.isCleaned, isFalse);
      await File(p.join(previewRoot, 'unexpected')).delete();

      final first = retained.retryCleanup();
      final second = retained.retryCleanup();
      expect(identical(first, second), isTrue);
      await Future.wait(<Future<void>>[first, second]);
      expect(retained.isCleaned, isTrue);
      expect(await Directory(previewRoot).exists(), isFalse);
    },
  );
}

Future<Revision3VoiceTakePreviewCapability> _materializeValidCapability() {
  final request = revision3VoicePreviewRequest();
  final native = _FakeNativePreviewCapability();
  return Revision3VoiceTakePreviewCapability.materialize(
    register: native.register,
    materialize: (token, root) async {
      await native.writePreview(root);
      return AuthoringRevision3VoiceTakePreviewMaterialization.fromJson(
        revision3VoicePreviewResponse(
          previewRoot: root,
          cleanupToken: token,
          request: request,
        ),
        previewRoot: root,
        cleanupToken: token,
        request: request,
      );
    },
    release: native.release,
  );
}

final class _FakeNativePreviewCapability {
  String? _root;

  Future<AuthoringRevision3VoiceTakePreviewRegistration> register() async {
    final root = (await createRevision3VoicePreviewTestRoot()).path;
    _root = root;
    return AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
      revision3VoicePreviewRegistrationResponse(previewRoot: root),
    );
  }

  Future<void> writePreview(String root) => File(
    p.join(root, 'preview.ogg'),
  ).writeAsBytes(revision3VoicePreviewBytes, flush: true);

  Future<void> release(String token) async {
    if (token != revision3VoicePreviewCleanupToken || _root == null) {
      throw StateError('unknown fake cleanup token');
    }
    final root = Directory(_root!);
    final entries = await root.list(followLinks: false).toList();
    if (entries.length > 1 ||
        (entries.isNotEmpty &&
            (p.basename(entries.single.path) != 'preview.ogg' ||
                await FileSystemEntity.type(
                      entries.single.path,
                      followLinks: false,
                    ) !=
                    FileSystemEntityType.file))) {
      throw const FileSystemException('fake retained cleanup failure');
    }
    if (entries.isNotEmpty) await entries.single.delete();
    await root.delete();
    _root = null;
  }
}
