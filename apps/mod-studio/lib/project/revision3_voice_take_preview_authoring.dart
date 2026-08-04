import 'dart:async';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceTakePreviewTechnicalMaterializer =
    Future<Revision3VoiceTakePreviewCapability> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakePreviewTechnicalPlan plan,
    });

typedef Revision3VoiceTakePreviewCapabilityRegistrar =
    Future<AuthoringRevision3VoiceTakePreviewRegistration> Function();
typedef Revision3VoiceTakePreviewCapabilityMaterializer =
    Future<AuthoringRevision3VoiceTakePreviewMaterialization> Function(
      String cleanupToken,
      String previewRoot,
    );
typedef Revision3VoiceTakePreviewCapabilityReleaser =
    Future<void> Function(String cleanupToken);

/// Exact hidden graph and asset identity for one read-only VoiceTake preview.
final class Revision3VoiceTakePreviewTechnicalPlan {
  const Revision3VoiceTakePreviewTechnicalPlan._({
    required this.lineId,
    required this.expectedLineRevision,
    required this.localizationId,
    required this.expectedLocalizationRevision,
    required this.locId,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.locale,
    required this.takeId,
    required this.expectedTakeRevision,
    required this.assetSha256,
    required this.assetByteLength,
    required this.assetLogicalName,
    required this.status,
    required this.codec,
    required this.channels,
    required this.sampleRate,
  });

  factory Revision3VoiceTakePreviewTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
    required String takeId,
  }) {
    final line = catalog.line(lineId);
    final normalizedLocale = locale.trim();
    final slotId = line?.slotIdForLocale(normalizedLocale);
    final slot = line?.slotSummaryForLocale(normalizedLocale);
    final take = slot?.candidate(takeId);
    final preview = take?.previewFacts;
    if (line == null ||
        normalizedLocale != locale ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        slotId == null ||
        slot == null ||
        take == null) {
      throw const Revision3VoiceTakePreviewStaleCheckpointException();
    }
    if (preview == null ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(
          line.localizationIdentity,
        )) {
      throw const FormatException(
        'This Voice take has no intact managed Ogg preview asset.',
      );
    }
    return Revision3VoiceTakePreviewTechnicalPlan._(
      lineId: line.lineId,
      expectedLineRevision: line.lineRevision,
      localizationId: line.localizationId,
      expectedLocalizationRevision: line.localizationRevision,
      locId: line.localizationIdentity,
      slotId: slotId,
      expectedSlotRevision: slot.slotRevision,
      locale: locale,
      takeId: take.id,
      expectedTakeRevision: take.revision,
      assetSha256: preview.assetSha256,
      assetByteLength: preview.assetByteLength,
      assetLogicalName: preview.assetLogicalName,
      status: take.status,
      codec: preview.codec,
      channels: preview.channels,
      sampleRate: preview.sampleRate,
    );
  }

  final String lineId;
  final int expectedLineRevision;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String locId;
  final String slotId;
  final int expectedSlotRevision;
  final String locale;
  final String takeId;
  final int expectedTakeRevision;
  final String assetSha256;
  final int assetByteLength;
  final String assetLogicalName;
  final Revision3ContentVoiceTakeStatus status;
  final Revision3ContentVoiceOggCodec codec;
  final int channels;
  final int sampleRate;
}

final class Revision3VoiceTakePreviewStaleCheckpointException
    implements Exception {
  const Revision3VoiceTakePreviewStaleCheckpointException({
    this.cleanupObligation,
  });

  /// Retained only when stale graph drift and temporary-capability cleanup
  /// failed together. Callers must finish it before offering a fresh preview.
  final Revision3VoiceTakePreviewCleanupObligation? cleanupObligation;
}

final class Revision3VoiceTakePreviewRequiresReopenException
    implements Exception {
  const Revision3VoiceTakePreviewRequiresReopenException({
    this.cause,
    this.cleanupObligation,
  });

  final Object? cause;
  final Revision3VoiceTakePreviewCleanupObligation? cleanupObligation;
}

/// Capability-local cleanup failure. It never implies project Store damage.
final class Revision3VoiceTakePreviewCleanupException implements Exception {
  const Revision3VoiceTakePreviewCleanupException(this.cause);

  final Object cause;

  @override
  String toString() => 'Voice preview cleanup failed.';
}

/// Pathless local verification failure for a native-owned preview artifact.
final class Revision3VoiceTakePreviewVerificationException
    implements Exception {
  const Revision3VoiceTakePreviewVerificationException();

  @override
  String toString() => 'Voice preview verification failed.';
}

/// Bounded ownership of a temporary preview root whose first cleanup failed.
///
/// Callers must retain this obligation until [retryCleanup] succeeds. The
/// diagnostic root is for local troubleshooting only; never render or persist
/// it as authored project data.
abstract interface class Revision3VoiceTakePreviewCleanupObligation {
  bool get isCleaned;

  Future<void> retryCleanup();
}

/// Preserves both a primary materialization failure and the still-owned
/// temporary capability when its first bounded cleanup also failed.
final class Revision3VoiceTakePreviewMaterializationCleanupException
    extends Revision3VoiceTakePreviewCleanupException
    implements Revision3VoiceTakePreviewCleanupObligation {
  Revision3VoiceTakePreviewMaterializationCleanupException._({
    required this.materializationCause,
    required this.materializationStackTrace,
    required this.diagnosticPreviewRoot,
    required Object cleanupCause,
    required Future<void> Function() cleanupOperation,
  }) : _retryCleanup = cleanupOperation,
       super(cleanupCause);

  final Object materializationCause;
  final StackTrace materializationStackTrace;
  final String diagnosticPreviewRoot;
  final Future<void> Function() _retryCleanup;

  Future<void>? _retryFuture;
  bool _cleaned = false;

  @override
  bool get isCleaned => _cleaned;

  @override
  Future<void> retryCleanup() {
    if (_cleaned) return Future<void>.value();
    final inFlight = _retryFuture;
    if (inFlight != null) return inFlight;

    late final Future<void> attempt;
    attempt = _retryCleanup().then(
      (_) => _cleaned = true,
      onError: (Object error, StackTrace stackTrace) {
        if (identical(_retryFuture, attempt)) _retryFuture = null;
        final wrapped = error is Revision3VoiceTakePreviewCleanupException
            ? error
            : Revision3VoiceTakePreviewCleanupException(error);
        Error.throwWithStackTrace(wrapped, stackTrace);
      },
    );
    _retryFuture = attempt;
    return attempt;
  }

  @override
  String toString() =>
      'Voice preview materialization failed and its temporary capability '
      'still requires bounded cleanup.';
}

/// Owns the only app-visible file copied from a private managed CAS object.
///
/// Native owns the fresh system-temporary root from birth. Cleanup is possible
/// only through its opaque token; recursive or ambient-path deletion is absent.
final class Revision3VoiceTakePreviewCapability
    implements Revision3VoiceTakePreviewCleanupObligation {
  Revision3VoiceTakePreviewCapability._({
    required this._file,
    required this._receipt,
    required this._cleanupToken,
    required this._release,
  });

  final File _file;
  final AuthoringRevision3VoiceTakePreviewMaterialization _receipt;
  final String _cleanupToken;
  final Revision3VoiceTakePreviewCapabilityReleaser _release;
  Future<void>? _closeFuture;
  bool _closed = false;

  /// Local playback capability only. Never render or persist this path.
  String get path => _file.path;

  bool get isClosed => _closed;

  @override
  bool get isCleaned => isClosed;

  AuthoringWorkingHead get basisHead => _receipt.basisHead;
  String get projectId => _receipt.projectId;
  int get projectRevision => _receipt.projectRevision;
  String get lineId => _receipt.lineId;
  int get lineRevision => _receipt.lineRevision;
  String get localizationId => _receipt.localizationId;
  int get localizationRevision => _receipt.localizationRevision;
  String get locId => _receipt.locId;
  String get slotId => _receipt.slotId;
  int get slotRevision => _receipt.slotRevision;
  String get locale => _receipt.locale;
  String get takeId => _receipt.takeId;
  int get takeRevision => _receipt.takeRevision;
  AuthoringRevision3VoiceAsset get asset => _receipt.asset;
  AuthoringRevision3VoiceTakeStatus get status => _receipt.status;
  AuthoringRevision3VoiceOggMetadata get ogg => _receipt.ogg;

  /// Create, materialize, verify, and adopt one unique preview capability.
  /// Every failure after strict registration adoption performs token cleanup.
  static Future<Revision3VoiceTakePreviewCapability> materialize({
    required Revision3VoiceTakePreviewCapabilityRegistrar register,
    required Revision3VoiceTakePreviewCapabilityMaterializer materialize,
    required Revision3VoiceTakePreviewCapabilityReleaser release,
  }) async {
    // Before a strict, adoptable response exists Dart never guesses an ambient path/token. An
    // exceptional malformed same-build success may remain isolated natively until process exit.
    final registration = await register();
    final root = Directory(registration.previewRoot);
    final file = File(registration.previewPath);

    try {
      await _requireFreshEmptyRoot(root);
      final receipt = await materialize(registration.cleanupToken, root.path);
      if (!p.equals(receipt.previewPath, file.path) ||
          receipt.previewLeaf != 'preview.ogg' ||
          receipt.cleanupToken != registration.cleanupToken ||
          !p.equals(registration.previewPath, file.path) ||
          registration.previewLeaf != 'preview.ogg') {
        throw const FormatException(
          'Voice preview receipt escaped its temporary capability.',
        );
      }
      await _requireExactMaterializedFile(root, file, receipt.asset);
      return Revision3VoiceTakePreviewCapability._(
        file: file,
        receipt: receipt,
        cleanupToken: registration.cleanupToken,
        release: release,
      );
    } catch (error, stackTrace) {
      try {
        await release(registration.cleanupToken);
      } catch (cleanupError) {
        throw Revision3VoiceTakePreviewMaterializationCleanupException._(
          materializationCause: error,
          materializationStackTrace: stackTrace,
          diagnosticPreviewRoot: root.path,
          cleanupCause: cleanupError,
          cleanupOperation: () async {
            try {
              await release(registration.cleanupToken);
            } catch (retryError) {
              if (retryError is Revision3VoiceTakePreviewCleanupException) {
                rethrow;
              }
              throw Revision3VoiceTakePreviewCleanupException(retryError);
            }
          },
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  Future<void> close() {
    if (_closed) return _closeFuture ?? Future<void>.value();
    final inFlight = _closeFuture;
    if (inFlight != null) return inFlight;

    late final Future<void> attempt;
    attempt = _closeExact().then(
      (_) => _closed = true,
      onError: (Object error, StackTrace stackTrace) {
        if (identical(_closeFuture, attempt)) _closeFuture = null;
        final wrapped = error is Revision3VoiceTakePreviewCleanupException
            ? error
            : Revision3VoiceTakePreviewCleanupException(error);
        Error.throwWithStackTrace(wrapped, stackTrace);
      },
    );
    _closeFuture = attempt;
    return attempt;
  }

  @override
  Future<void> retryCleanup() => close();

  Future<void> _closeExact() async {
    try {
      await _release(_cleanupToken);
    } catch (error) {
      throw Revision3VoiceTakePreviewCleanupException(error);
    }
  }

  static Future<void> _requireFreshEmptyRoot(Directory root) async {
    try {
      if (await FileSystemEntity.type(root.path, followLinks: false) !=
              FileSystemEntityType.directory ||
          await root.list(followLinks: false).isEmpty == false) {
        throw const Revision3VoiceTakePreviewVerificationException();
      }
    } on Revision3VoiceTakePreviewVerificationException {
      rethrow;
    } catch (_) {
      throw const Revision3VoiceTakePreviewVerificationException();
    }
  }

  static Future<void> _requireExactMaterializedFile(
    Directory root,
    File file,
    AuthoringRevision3VoiceAsset asset,
  ) async {
    try {
      final entries = await root.list(followLinks: false).toList();
      if (entries.length != 1 ||
          !p.equals(entries.single.path, file.path) ||
          await FileSystemEntity.type(file.path, followLinks: false) !=
              FileSystemEntityType.file) {
        throw const Revision3VoiceTakePreviewVerificationException();
      }
      final length = await file.length();
      final digest = await crypto.sha256.bind(file.openRead()).single;
      if (length != asset.byteLength || digest.toString() != asset.sha256) {
        throw const Revision3VoiceTakePreviewVerificationException();
      }
    } on Revision3VoiceTakePreviewVerificationException {
      rethrow;
    } catch (_) {
      throw const Revision3VoiceTakePreviewVerificationException();
    }
  }
}

/// Fresh-index service boundary for the normal-mode Voice take manager.
final class Revision3VoiceTakePreviewAuthoringService {
  const Revision3VoiceTakePreviewAuthoringService({
    required this.loadContentIndex,
    required this.materializeTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTakePreviewTechnicalMaterializer materializeTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3VoiceTakePreviewRequiresReopenException();
    }
  }

  Future<Revision3VoiceTakePreviewCapability> materialize({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
    required String takeId,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakePreviewStaleCheckpointException();
    }
    final plan = Revision3VoiceTakePreviewTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
      takeId: takeId,
    );
    final capability = await materializeTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (!_capabilityMatches(
      capability,
      projectId: fresh.projectId,
      projectRevision: fresh.projectRevision,
      plan: plan,
    )) {
      Object? cleanupCause;
      try {
        await capability.close();
      } catch (error) {
        cleanupCause = error;
      }
      throw Revision3VoiceTakePreviewRequiresReopenException(
        cause: cleanupCause,
        cleanupObligation: capability.isClosed ? null : capability,
      );
    }
    return capability;
  }
}

bool _capabilityMatches(
  Revision3VoiceTakePreviewCapability capability, {
  required String projectId,
  required int projectRevision,
  required Revision3VoiceTakePreviewTechnicalPlan plan,
}) =>
    capability.projectId == projectId &&
    capability.projectRevision == projectRevision &&
    capability.lineId == plan.lineId &&
    capability.lineRevision == plan.expectedLineRevision &&
    capability.localizationId == plan.localizationId &&
    capability.localizationRevision == plan.expectedLocalizationRevision &&
    capability.locId == plan.locId &&
    capability.slotId == plan.slotId &&
    capability.slotRevision == plan.expectedSlotRevision &&
    capability.locale == plan.locale &&
    capability.takeId == plan.takeId &&
    capability.takeRevision == plan.expectedTakeRevision &&
    capability.asset.sha256 == plan.assetSha256 &&
    capability.asset.byteLength == plan.assetByteLength &&
    capability.asset.logicalName == plan.assetLogicalName &&
    capability.status.name == plan.status.name &&
    capability.ogg.codec.name == plan.codec.name &&
    capability.ogg.channels == plan.channels &&
    capability.ogg.sampleRate == plan.sampleRate;
