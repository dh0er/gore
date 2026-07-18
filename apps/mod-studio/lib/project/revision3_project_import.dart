import 'dart:convert';

import '../core/mod_ffi.dart';

/// The only managed-project snapshot format accepted by import inspection.
const String revision3ProjectImportFormatV2 =
    'managed_revision3_exact_snapshot_v2';

/// V1 is intentionally named so callers can distinguish a canonical manifest
/// that declares review-copy authority. Its closure is not validated as V2 and
/// it is never accepted as an importable artifact.
const String revision3ProjectImportUnsupportedFormatV1 =
    'managed_revision3_exact_snapshot_v1';

const String revision3ProjectImportArtifactKindV2 =
    'portable_snapshot_restorable_copy';
const String revision3ProjectImportRestoreStatusV2 = 'supported';
const String revision3ProjectImportManifestName = 'gore-export.json';

const String _revision3ProjectImportOutcome = 'inspected_restorable_copy';
const String _revision3ProjectImportInspectionStatus = 'verified_exact';
const String _revision3ProjectImportNotPerformed = 'not_performed';
const String _revision3ProjectImportRuntimeStatus = 'runtime_unqualified';
const String _revision3ProjectImportPublicationStatus = 'not_supported';
const String _revision3ProjectImportInspectCommand =
    'authoring_store_inspect_revision3_exact_snapshot_v2';
const String _revision3ProjectImportUnsupportedReviewCopyCode =
    'AUTHORING_REVISION3_IMPORT_UNSUPPORTED_REVIEW_COPY';
const String _revision3ProjectImportPlatformUnsupportedCode =
    'AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED';
const String _revision3ProjectImportSourceInvalidCode =
    'AUTHORING_REVISION3_IMPORT_SOURCE_INVALID';

/// Public caps shared with the future native inspection boundary.
const int revision3ProjectImportMaxSourceUtf8Bytes = 32 * 1024;
const int revision3ProjectImportMaxHeadJsonUtf8Bytes = 64 * 1024;
const int revision3ProjectImportMaxManifestBytes = 128 * 1024 * 1024;
const int revision3ProjectImportMaxSnapshotObjects = 100000;
const int revision3ProjectImportMaxEntityObjects = 100000;
const int revision3ProjectImportMaxAssetObjects = 100000;
const int revision3ProjectImportMaxClosureObjects = 300000;
const int revision3ProjectImportMaxArchiveEntries = 300003;
const int revision3ProjectImportMaxUncompressedBytes = 70 * 1024 * 1024 * 1024;
const int revision3ProjectImportMaxArchiveBytes = 70 * 1024 * 1024 * 1024;

const int _revision3ProjectImportMaxSignedWireInteger = 0x7fffffffffffffff;
const int _revision3ProjectImportMaxSourceLabelUtf8Bytes = 160;
const String _zeroRevision3ProjectImportId = '00000000000000000000000000000000';
final RegExp _revision3ProjectImportSha256Pattern = RegExp(r'^[0-9a-f]{64}$');
final RegExp _revision3ProjectImportIdPattern = RegExp(r'^[0-9a-f]{32}$');

/// A recognized canonical V1 manifest declares review-copy authority, but its
/// closure is not validated as restorable and must never fall through to V2.
final class Revision3ProjectImportUnsupportedFormatException
    implements Exception {
  const Revision3ProjectImportUnsupportedFormatException();

  String get format => revision3ProjectImportUnsupportedFormatV1;

  @override
  String toString() =>
      'The selected snapshot declares the V1 review-copy format and is not restorable.';
}

/// One bounded content seal returned by native archive inspection.
final class Revision3ProjectImportArchiveSeal {
  const Revision3ProjectImportArchiveSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  factory Revision3ProjectImportArchiveSeal._fromJson(Object? value) {
    final json = _revision3ProjectImportObject(value, 'archive seal');
    _revision3ProjectImportExactFields(json, const <String>{
      'byte_len',
      'sha256',
    }, 'archive seal');
    return Revision3ProjectImportArchiveSeal._(
      byteLength: _revision3ProjectImportInt(
        json['byte_len'],
        'archive byte length',
        min: 1,
        max: revision3ProjectImportMaxArchiveBytes,
      ),
      sha256: _revision3ProjectImportSha256(json['sha256'], 'archive SHA-256'),
    );
  }
}

/// The exact archive member containing the closed V2 manifest.
final class Revision3ProjectImportManifestSeal {
  const Revision3ProjectImportManifestSeal._({
    required this.relativeName,
    required this.byteLength,
    required this.sha256,
  });

  final String relativeName;
  final int byteLength;
  final String sha256;

  factory Revision3ProjectImportManifestSeal._fromJson(Object? value) {
    final json = _revision3ProjectImportObject(value, 'manifest seal');
    _revision3ProjectImportExactFields(json, const <String>{
      'relative_name',
      'byte_len',
      'sha256',
    }, 'manifest seal');
    final relativeName = _revision3ProjectImportString(
      json['relative_name'],
      'manifest relative name',
      maxBytes: 64,
    );
    if (relativeName != revision3ProjectImportManifestName) {
      throw const FormatException(
        'revision-3 project import manifest name is not canonical',
      );
    }
    return Revision3ProjectImportManifestSeal._(
      relativeName: relativeName,
      byteLength: _revision3ProjectImportInt(
        json['byte_len'],
        'manifest byte length',
        min: 1,
        max: revision3ProjectImportMaxManifestBytes,
      ),
      sha256: _revision3ProjectImportSha256(json['sha256'], 'manifest SHA-256'),
    );
  }
}

/// Exact Store closure proven to be present in the selected archive.
final class Revision3ProjectImportClosure {
  const Revision3ProjectImportClosure._({
    required this.snapshotObjects,
    required this.entityObjects,
    required this.assetObjects,
    required this.archiveEntries,
    required this.uncompressedBytes,
  });

  final int snapshotObjects;
  final int entityObjects;
  final int assetObjects;
  final int archiveEntries;
  final int uncompressedBytes;

  int get storeObjects => snapshotObjects + entityObjects + assetObjects;

  factory Revision3ProjectImportClosure._fromJson(Object? value) {
    final json = _revision3ProjectImportObject(value, 'closure');
    _revision3ProjectImportExactFields(json, const <String>{
      'snapshot_objects',
      'entity_objects',
      'asset_objects',
      'archive_entries',
      'uncompressed_bytes',
    }, 'closure');
    final snapshotObjects = _revision3ProjectImportInt(
      json['snapshot_objects'],
      'snapshot object count',
      min: 1,
      max: revision3ProjectImportMaxSnapshotObjects,
    );
    final entityObjects = _revision3ProjectImportInt(
      json['entity_objects'],
      'entity object count',
      min: 0,
      max: revision3ProjectImportMaxEntityObjects,
    );
    final assetObjects = _revision3ProjectImportInt(
      json['asset_objects'],
      'asset object count',
      min: 0,
      max: revision3ProjectImportMaxAssetObjects,
    );
    final archiveEntries = _revision3ProjectImportInt(
      json['archive_entries'],
      'archive entry count',
      min: 4,
      max: revision3ProjectImportMaxArchiveEntries,
    );
    final uncompressedBytes = _revision3ProjectImportInt(
      json['uncompressed_bytes'],
      'uncompressed byte count',
      min: 1,
      max: revision3ProjectImportMaxUncompressedBytes,
    );
    final storeObjects = snapshotObjects + entityObjects + assetObjects;
    if (storeObjects > revision3ProjectImportMaxClosureObjects ||
        archiveEntries != storeObjects + 3) {
      throw const FormatException(
        'revision-3 project import closure counts are inconsistent',
      );
    }
    return Revision3ProjectImportClosure._(
      snapshotObjects: snapshotObjects,
      entityObjects: entityObjects,
      assetObjects: assetObjects,
      archiveEntries: archiveEntries,
      uncompressedBytes: uncompressedBytes,
    );
  }
}

/// Strict read-only result for a future native V2 archive inspection.
///
/// [source] preserves the caller's exact spelling solely for request/result
/// binding. Normal UI should render [sourceLabel], which removes parent paths
/// and bounds display length. This DTO proves no destination, candidate,
/// project mutation, restore execution, or publication.
final class Revision3ProjectImportInspection {
  const Revision3ProjectImportInspection._({
    required this.source,
    required this.sourceLabel,
    required this.archive,
    required this.manifest,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    required this.closure,
  });

  final String source;
  final String sourceLabel;
  final Revision3ProjectImportArchiveSeal archive;
  final Revision3ProjectImportManifestSeal manifest;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;
  final Revision3ProjectImportClosure closure;

  String get format => revision3ProjectImportFormatV2;
  String get artifactKind => revision3ProjectImportArtifactKindV2;
  String get restoreStatus => revision3ProjectImportRestoreStatusV2;
  bool get retrySafe => true;

  factory Revision3ProjectImportInspection.fromJson(
    Object? value, {
    required String expectedSource,
  }) {
    final exactSource = _revision3ProjectImportSource(expectedSource);
    final json = _revision3ProjectImportObject(value, 'inspection response');
    _revision3ProjectImportExactFields(json, const <String>{
      'ok',
      'outcome',
      'source',
      'format',
      'artifact_kind',
      'restore_status',
      'archive',
      'manifest',
      'project_id',
      'project_revision',
      'head_json',
      'closure',
      'inspection_status',
      'import_status',
      'project_mutation',
      'game_mutation',
      'save_mutation',
      'build_status',
      'deployment_status',
      'runtime_status',
      'publication_status',
      'retry_safe',
    }, 'inspection response');

    if (json['ok'] != true ||
        json['outcome'] != _revision3ProjectImportOutcome ||
        json['format'] != revision3ProjectImportFormatV2 ||
        json['artifact_kind'] != revision3ProjectImportArtifactKindV2 ||
        json['restore_status'] != revision3ProjectImportRestoreStatusV2 ||
        json['inspection_status'] != _revision3ProjectImportInspectionStatus ||
        json['import_status'] != _revision3ProjectImportNotPerformed ||
        json['project_mutation'] != _revision3ProjectImportNotPerformed ||
        json['game_mutation'] != _revision3ProjectImportNotPerformed ||
        json['save_mutation'] != _revision3ProjectImportNotPerformed ||
        json['build_status'] != _revision3ProjectImportNotPerformed ||
        json['deployment_status'] != _revision3ProjectImportNotPerformed ||
        json['runtime_status'] != _revision3ProjectImportRuntimeStatus ||
        json['publication_status'] !=
            _revision3ProjectImportPublicationStatus ||
        json['retry_safe'] != true) {
      throw const FormatException(
        'revision-3 project import inspection status pairing is invalid',
      );
    }

    final responseSource = _revision3ProjectImportSource(
      _revision3ProjectImportString(
        json['source'],
        'inspection source',
        maxBytes: revision3ProjectImportMaxSourceUtf8Bytes,
      ),
    );
    if (responseSource != exactSource) {
      throw const FormatException(
        'revision-3 project import response disagrees with its exact source spelling',
      );
    }

    final archive = Revision3ProjectImportArchiveSeal._fromJson(
      json['archive'],
    );
    final manifest = Revision3ProjectImportManifestSeal._fromJson(
      json['manifest'],
    );
    final projectId = _revision3ProjectImportId(json['project_id']);
    final projectRevision = _revision3ProjectImportInt(
      json['project_revision'],
      'project revision',
      min: 0,
      max: _revision3ProjectImportMaxSignedWireInteger,
    );
    final headJson = _revision3ProjectImportString(
      json['head_json'],
      'project head',
      maxBytes: revision3ProjectImportMaxHeadJsonUtf8Bytes,
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(headJson);
    final closure = Revision3ProjectImportClosure._fromJson(json['closure']);
    if (archive.byteLength <= closure.uncompressedBytes ||
        manifest.byteLength > closure.uncompressedBytes ||
        head.snapshotByteLength >
            closure.uncompressedBytes - manifest.byteLength) {
      throw const FormatException(
        'revision-3 project import seals exceed the inspected closure',
      );
    }

    return Revision3ProjectImportInspection._(
      source: responseSource,
      sourceLabel: revision3ProjectImportSourceLabel(responseSource),
      archive: archive,
      manifest: manifest,
      projectId: projectId,
      projectRevision: projectRevision,
      head: head,
      closure: closure,
    );
  }
}

/// Returns a bounded basename suitable for normal UI. It never includes the
/// selected file's parent path. Invalid source spellings throw without echoing
/// the supplied value.
String revision3ProjectImportSourceLabel(String source) {
  final exactSource = _revision3ProjectImportSource(source);
  final slash = exactSource.lastIndexOf('/');
  final backslash = exactSource.lastIndexOf(r'\');
  final separator = slash > backslash ? slash : backslash;
  final basename = exactSource.substring(separator + 1);
  if (_revision3ProjectImportUtf8Length(basename, 'source label') <=
      _revision3ProjectImportMaxSourceLabelUtf8Bytes) {
    return basename;
  }
  const suffix = '\u2026';
  final output = StringBuffer();
  var bytes = 0;
  for (final rune in basename.runes) {
    final character = String.fromCharCode(rune);
    final width = utf8.encode(character).length;
    if (bytes + width + utf8.encode(suffix).length >
        _revision3ProjectImportMaxSourceLabelUtf8Bytes) {
      break;
    }
    output.write(character);
    bytes += width;
  }
  return '${output.toString()}$suffix';
}

/// Opaque host lifecycle captured around file selection and native inspection.
/// [owner] uses identity so a new dialog/session cannot accidentally reuse an
/// equal value; [generation] invalidates state changes within the same owner.
final class Revision3ProjectImportLifecycle {
  const Revision3ProjectImportLifecycle({
    required this.owner,
    required this.generation,
  });

  final Object owner;
  final int generation;

  bool sameAs(Revision3ProjectImportLifecycle other) =>
      identical(owner, other.owner) && generation == other.generation;
}

/// An inspect-only plan for a future dialog. It intentionally carries no
/// destination, restore callback, candidate, or publication authority.
final class Revision3ProjectImportInspectionPlan {
  const Revision3ProjectImportInspectionPlan._(this.inspection);

  final Revision3ProjectImportInspection inspection;

  String get sourceLabel => inspection.sourceLabel;
}

enum Revision3ProjectImportPlanningOutcome {
  inspected,
  cancelled,
  unsupportedFormat,
  invalidSource,
  inspectionFailed,
  stale,
  superseded,
  busy,
  unavailable,
}

/// Sanitized planning result. Native or picker exceptions are deliberately not
/// retained, preventing paths or archive internals from reaching normal UI.
final class Revision3ProjectImportPlanningResult {
  const Revision3ProjectImportPlanningResult._(this.outcome, {this.plan});

  const Revision3ProjectImportPlanningResult.inspected(
    Revision3ProjectImportInspectionPlan plan,
  ) : this._(Revision3ProjectImportPlanningOutcome.inspected, plan: plan);

  const Revision3ProjectImportPlanningResult.cancelled()
    : this._(Revision3ProjectImportPlanningOutcome.cancelled);

  const Revision3ProjectImportPlanningResult.unsupportedFormat()
    : this._(Revision3ProjectImportPlanningOutcome.unsupportedFormat);

  const Revision3ProjectImportPlanningResult.invalidSource()
    : this._(Revision3ProjectImportPlanningOutcome.invalidSource);

  const Revision3ProjectImportPlanningResult.inspectionFailed()
    : this._(Revision3ProjectImportPlanningOutcome.inspectionFailed);

  const Revision3ProjectImportPlanningResult.stale()
    : this._(Revision3ProjectImportPlanningOutcome.stale);

  const Revision3ProjectImportPlanningResult.superseded()
    : this._(Revision3ProjectImportPlanningOutcome.superseded);

  const Revision3ProjectImportPlanningResult.busy()
    : this._(Revision3ProjectImportPlanningOutcome.busy);

  const Revision3ProjectImportPlanningResult.unavailable()
    : this._(Revision3ProjectImportPlanningOutcome.unavailable);

  final Revision3ProjectImportPlanningOutcome outcome;
  final Revision3ProjectImportInspectionPlan? plan;
}

typedef Revision3ProjectImportLifecycleReader =
    Revision3ProjectImportLifecycle? Function();
typedef Revision3ProjectImportSourcePicker = Future<String?> Function();
typedef Revision3ProjectImportNativeInspector =
    Future<Object?> Function(String source);

/// Single-flight inspect-only orchestration for a future project-import dialog.
///
/// Every await is followed by epoch and lifecycle checks. Cancellation,
/// disposal, or host drift can suppress a late native result; no callback in
/// this class can create a working directory or publish a Store head.
final class Revision3ProjectImportInspectionCoordinator {
  factory Revision3ProjectImportInspectionCoordinator({
    required Revision3ProjectImportLifecycleReader readLifecycle,
    required Revision3ProjectImportSourcePicker pickSource,
    required Revision3ProjectImportNativeInspector inspect,
  }) => Revision3ProjectImportInspectionCoordinator._(
    readLifecycle,
    pickSource,
    inspect,
  );

  Revision3ProjectImportInspectionCoordinator._(
    this._readLifecycle,
    this._pickSource,
    this._inspect,
  );

  final Revision3ProjectImportLifecycleReader _readLifecycle;
  final Revision3ProjectImportSourcePicker _pickSource;
  final Revision3ProjectImportNativeInspector _inspect;

  bool _busy = false;
  bool _disposed = false;
  int _epoch = 0;

  bool get isBusy => _busy;
  bool get isDisposed => _disposed;

  /// Invalidates a pending selection/inspection without starting another one.
  /// The underlying Future is allowed to finish, but its result is suppressed.
  bool cancelPending() {
    if (_disposed || !_busy) return false;
    _epoch++;
    return true;
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _epoch++;
  }

  Future<Revision3ProjectImportPlanningResult> plan() async {
    if (_disposed) {
      return const Revision3ProjectImportPlanningResult.unavailable();
    }
    if (_busy) return const Revision3ProjectImportPlanningResult.busy();

    _busy = true;
    final epoch = ++_epoch;
    try {
      final lifecycle = _safeReadLifecycle();
      if (!_validRevision3ProjectImportLifecycle(lifecycle)) {
        return const Revision3ProjectImportPlanningResult.unavailable();
      }

      final String? selectedSource;
      try {
        selectedSource = await _pickSource();
      } catch (_) {
        return _failureAfterAwait(epoch, lifecycle!);
      }
      final gateAfterSelection = _gateAfterAwait(epoch, lifecycle!);
      if (gateAfterSelection != null) return gateAfterSelection;
      if (selectedSource == null) {
        return const Revision3ProjectImportPlanningResult.cancelled();
      }

      final String source;
      try {
        source = _revision3ProjectImportSource(selectedSource);
      } on FormatException {
        return const Revision3ProjectImportPlanningResult.invalidSource();
      }

      final Object? response;
      try {
        response = await _inspect(source);
      } on Revision3ProjectImportUnsupportedFormatException {
        final gate = _gateAfterAwait(epoch, lifecycle);
        return gate ??
            const Revision3ProjectImportPlanningResult.unsupportedFormat();
      } on ModFfiException catch (error) {
        final gate = _gateAfterAwait(epoch, lifecycle);
        if (gate != null) return gate;
        if (error.command == _revision3ProjectImportInspectCommand &&
            error.code == _revision3ProjectImportUnsupportedReviewCopyCode) {
          return const Revision3ProjectImportPlanningResult.unsupportedFormat();
        }
        if (error.command == _revision3ProjectImportInspectCommand &&
            error.code == _revision3ProjectImportPlatformUnsupportedCode) {
          return const Revision3ProjectImportPlanningResult.unavailable();
        }
        if (error.command == _revision3ProjectImportInspectCommand &&
            error.code == _revision3ProjectImportSourceInvalidCode) {
          return const Revision3ProjectImportPlanningResult.invalidSource();
        }
        return const Revision3ProjectImportPlanningResult.inspectionFailed();
      } catch (_) {
        return _failureAfterAwait(epoch, lifecycle);
      }
      final gateAfterInspection = _gateAfterAwait(epoch, lifecycle);
      if (gateAfterInspection != null) return gateAfterInspection;

      final Revision3ProjectImportInspection inspection;
      try {
        inspection = Revision3ProjectImportInspection.fromJson(
          response,
          expectedSource: source,
        );
      } on Revision3ProjectImportUnsupportedFormatException {
        return const Revision3ProjectImportPlanningResult.unsupportedFormat();
      } on FormatException {
        return const Revision3ProjectImportPlanningResult.inspectionFailed();
      }
      final finalGate = _gateAfterAwait(epoch, lifecycle);
      if (finalGate != null) return finalGate;
      return Revision3ProjectImportPlanningResult.inspected(
        Revision3ProjectImportInspectionPlan._(inspection),
      );
    } finally {
      _busy = false;
    }
  }

  Revision3ProjectImportPlanningResult _failureAfterAwait(
    int epoch,
    Revision3ProjectImportLifecycle lifecycle,
  ) =>
      _gateAfterAwait(epoch, lifecycle) ??
      const Revision3ProjectImportPlanningResult.inspectionFailed();

  Revision3ProjectImportPlanningResult? _gateAfterAwait(
    int epoch,
    Revision3ProjectImportLifecycle lifecycle,
  ) {
    if (_disposed || epoch != _epoch) {
      return const Revision3ProjectImportPlanningResult.superseded();
    }
    final current = _safeReadLifecycle();
    if (current == null || !current.sameAs(lifecycle)) {
      return const Revision3ProjectImportPlanningResult.stale();
    }
    return null;
  }

  Revision3ProjectImportLifecycle? _safeReadLifecycle() {
    try {
      return _readLifecycle();
    } catch (_) {
      return null;
    }
  }
}

bool _validRevision3ProjectImportLifecycle(
  Revision3ProjectImportLifecycle? lifecycle,
) =>
    lifecycle != null &&
    lifecycle.generation >= 0 &&
    lifecycle.generation <= _revision3ProjectImportMaxSignedWireInteger;

Map<String, Object?> _revision3ProjectImportObject(
  Object? value,
  String context,
) {
  if (value is! Map) {
    throw FormatException(
      'revision-3 project import $context is not an object',
    );
  }
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    final key = entry.key;
    if (key is! String) {
      throw FormatException(
        'revision-3 project import $context has a non-string field',
      );
    }
    result[key] = entry.value;
  }
  return result;
}

void _revision3ProjectImportExactFields(
  Map<String, Object?> json,
  Set<String> expected,
  String context,
) {
  if (json.length != expected.length || !json.keys.every(expected.contains)) {
    throw FormatException(
      'revision-3 project import $context has an unknown or missing field',
    );
  }
}

String _revision3ProjectImportString(
  Object? value,
  String context, {
  required int maxBytes,
}) {
  if (value is! String || value.isEmpty) {
    throw FormatException(
      'revision-3 project import $context is not a non-empty string',
    );
  }
  if (_revision3ProjectImportUtf8Length(value, context) > maxBytes) {
    throw FormatException(
      'revision-3 project import $context exceeds its byte cap',
    );
  }
  return value;
}

int _revision3ProjectImportInt(
  Object? value,
  String context, {
  required int min,
  required int max,
}) {
  if (value is! int || value < min || value > max) {
    throw FormatException(
      'revision-3 project import $context is outside its closed integer range',
    );
  }
  return value;
}

String _revision3ProjectImportSha256(Object? value, String context) {
  final sha256 = _revision3ProjectImportString(value, context, maxBytes: 64);
  if (!_revision3ProjectImportSha256Pattern.hasMatch(sha256)) {
    throw FormatException(
      'revision-3 project import $context is not canonical lowercase hex',
    );
  }
  return sha256;
}

String _revision3ProjectImportId(Object? value) {
  final id = _revision3ProjectImportString(value, 'project ID', maxBytes: 32);
  if (!_revision3ProjectImportIdPattern.hasMatch(id) ||
      id == _zeroRevision3ProjectImportId) {
    throw const FormatException(
      'revision-3 project import project ID is not one canonical non-zero ID',
    );
  }
  return id;
}

String _revision3ProjectImportSource(String value) {
  _revision3ProjectImportString(
    value,
    'source spelling',
    maxBytes: revision3ProjectImportMaxSourceUtf8Bytes,
  );
  for (final rune in value.runes) {
    if (rune < 0x20 || (rune >= 0x7f && rune <= 0x9f)) {
      throw const FormatException(
        'revision-3 project import source spelling contains a control character',
      );
    }
  }
  final slash = value.lastIndexOf('/');
  final backslash = value.lastIndexOf(r'\');
  final separator = slash > backslash ? slash : backslash;
  final basename = value.substring(separator + 1);
  if (basename.isEmpty || basename == '.' || basename == '..') {
    throw const FormatException(
      'revision-3 project import source spelling does not name a file',
    );
  }
  return value;
}

int _revision3ProjectImportUtf8Length(String value, String context) {
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    if (unit <= 0x7f) {
      length += 1;
    } else if (unit <= 0x7ff) {
      length += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException(
          'revision-3 project import $context contains malformed UTF-16',
        );
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException(
          'revision-3 project import $context contains malformed UTF-16',
        );
      }
      index++;
      length += 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw FormatException(
        'revision-3 project import $context contains malformed UTF-16',
      );
    } else {
      length += 3;
    }
  }
  return length;
}
