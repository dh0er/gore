import 'dart:convert';

import '../core/mod_ffi.dart';

/// The only managed-project snapshot format accepted by import inspection.
const String revision3ProjectImportFormatV2 =
    'managed_revision3_exact_snapshot_v2';

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
const String _revision3ProjectImportDestinationCommand =
    'authoring_store_import_revision3_exact_snapshot_v2';
const String _revision3ProjectImportPlatformUnsupportedCode =
    'AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED';
const String _revision3ProjectImportSourceInvalidCode =
    'AUTHORING_REVISION3_IMPORT_SOURCE_INVALID';
const String _revision3ProjectImportDestinationInvalidCode =
    'AUTHORING_REVISION3_IMPORT_DESTINATION_INVALID';
const String _revision3ProjectImportSourceChangedCode =
    'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED';
const String _revision3ProjectImportMaterializedStatus = 'materialized';
const String _revision3ProjectImportPublishedStatus = 'published';
const String _revision3ProjectImportPublishedWithCleanupWarningStatus =
    'published_with_cleanup_warning';
const String _revision3ProjectImportPublicationUncertainStatus =
    'publication_uncertain';
const String _revision3ProjectImportCleanupWarningCode =
    'AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING';
const String _revision3ProjectImportCleanupWarningMessage =
    'the verified project was materialized, but private staging cleanup was incomplete';
const String _revision3ProjectImportPublicationUncertainCode =
    'AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN';
const String _revision3ProjectImportPublicationUncertainMessage =
    'project publication may have completed; do not retry automatically';

/// Public caps shared with the future native inspection boundary.
const int revision3ProjectImportMaxSourceUtf8Bytes = 32 * 1024;
const int revision3ProjectImportMaxDestinationUtf8Bytes = 32 * 1024;
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
final RegExp _revision3ProjectImportWindowsDriveRootPattern = RegExp(
  r'^[A-Za-z]:[\\/]',
);
final RegExp _revision3ProjectImportWindowsSeparatorPattern = RegExp(r'[\\/]');
final RegExp _revision3ProjectImportWindowsInvalidSegmentPattern = RegExp(
  r'[<>"|?*:]',
);
final RegExp _revision3ProjectImportWindowsReservedSegmentPattern = RegExp(
  r'^(?:CON|PRN|AUX|NUL|COM[1-9¹²³]|LPT[1-9¹²³])(?:\.|$)',
  caseSensitive: false,
);

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

/// Returns a bounded destination basename without exposing its parent path.
String revision3ProjectImportDestinationLabel(String destination) {
  final exactDestination = _revision3ProjectImportDestination(destination);
  return _revision3ProjectImportBoundedBasename(
    exactDestination,
    'destination label',
  );
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
  const Revision3ProjectImportInspectionPlan._(
    this.inspection,
    this._lifecycle,
  );

  final Revision3ProjectImportInspection inspection;
  final Revision3ProjectImportLifecycle _lifecycle;

  String get sourceLabel => inspection.sourceLabel;
}

/// The complete authority handed to the future native materialization command.
///
/// It intentionally contains only the exact inspected source spelling, the
/// exact selected destination spelling, and the archive seal that was inspected.
/// No game root, session handle, build target, or adoption callback can be
/// smuggled through this request.
final class Revision3ProjectImportDestinationRequest {
  const Revision3ProjectImportDestinationRequest._({
    required this.source,
    required this.destination,
    required this.expectedArchive,
  });

  final String source;
  final String destination;
  final Revision3ProjectImportArchiveSeal expectedArchive;

  Map<String, Object?> toJson() => <String, Object?>{
    'source': source,
    'destination': destination,
    'expected_archive': <String, Object?>{
      'byte_len': expectedArchive.byteLength,
      'sha256': expectedArchive.sha256,
    },
  };
}

/// A destination choice bound to one prior exact inspection.
final class Revision3ProjectImportDestinationPlan {
  const Revision3ProjectImportDestinationPlan._({
    required this.inspection,
    required this.destination,
  });

  final Revision3ProjectImportInspection inspection;
  final String destination;

  String get source => inspection.source;
  String get sourceLabel => inspection.sourceLabel;
  String get destinationLabel =>
      revision3ProjectImportDestinationLabel(destination);
  Revision3ProjectImportArchiveSeal get expectedArchive => inspection.archive;

  Revision3ProjectImportDestinationRequest get request =>
      Revision3ProjectImportDestinationRequest._(
        source: source,
        destination: destination,
        expectedArchive: expectedArchive,
      );

  factory Revision3ProjectImportDestinationPlan.fromInspection({
    required Revision3ProjectImportInspection inspection,
    required String destination,
  }) {
    final exactDestination = _revision3ProjectImportDestination(destination);
    if (exactDestination == inspection.source) {
      throw const FormatException(
        'revision-3 project import source and destination must be distinct',
      );
    }
    return Revision3ProjectImportDestinationPlan._(
      inspection: inspection,
      destination: exactDestination,
    );
  }
}

enum Revision3ProjectImportMaterializationOutcome {
  imported,
  importedWithCleanupWarning,
  publicationUncertain,
}

/// Receipt for a project whose destination publication is known to have
/// completed. Creating or receiving this receipt does not adopt the project
/// into an application session.
final class Revision3ProjectImportedReceipt {
  const Revision3ProjectImportedReceipt._({
    required this.source,
    required this.destination,
    required this.archive,
    required this.manifest,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    required this.closure,
  });

  final String source;
  final String destination;
  final Revision3ProjectImportArchiveSeal archive;
  final Revision3ProjectImportManifestSeal manifest;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;
  final Revision3ProjectImportClosure closure;
}

/// Strict terminal response from the future native materialization command.
///
/// Publication uncertainty deliberately carries no identity fields and yields
/// no [receipt]. Callers must never infer a safely adoptable project from that
/// terminal state and must never retry it automatically.
final class Revision3ProjectImportDestinationResult {
  const Revision3ProjectImportDestinationResult._({
    required this.outcome,
    required this.receipt,
  });

  final Revision3ProjectImportMaterializationOutcome outcome;
  final Revision3ProjectImportedReceipt? receipt;

  bool get retrySafe => false;
  bool get hasCleanupWarning =>
      outcome ==
      Revision3ProjectImportMaterializationOutcome.importedWithCleanupWarning;
  bool get publicationIsUncertain =>
      outcome ==
      Revision3ProjectImportMaterializationOutcome.publicationUncertain;

  factory Revision3ProjectImportDestinationResult.fromJson(
    Object? value, {
    required Revision3ProjectImportDestinationPlan expectedPlan,
  }) {
    final json = _revision3ProjectImportObject(value, 'destination response');
    final outcome = switch (json['outcome']) {
      'imported' => Revision3ProjectImportMaterializationOutcome.imported,
      'imported_with_cleanup_warning' =>
        Revision3ProjectImportMaterializationOutcome.importedWithCleanupWarning,
      'publication_uncertain' =>
        Revision3ProjectImportMaterializationOutcome.publicationUncertain,
      _ => throw const FormatException(
        'revision-3 project import destination outcome is unknown',
      ),
    };
    final expectedFields =
        outcome ==
            Revision3ProjectImportMaterializationOutcome.publicationUncertain
        ? const <String>{
            'ok',
            'outcome',
            'source',
            'destination',
            'format',
            'artifact_kind',
            'restore_status',
            'inspection_status',
            'import_status',
            'project_mutation',
            'session_adoption',
            'game_mutation',
            'save_mutation',
            'build_status',
            'deployment_status',
            'runtime_status',
            'publication_status',
            'retry_safe',
            'warning',
          }
        : const <String>{
            'ok',
            'outcome',
            'source',
            'destination',
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
            'session_adoption',
            'game_mutation',
            'save_mutation',
            'build_status',
            'deployment_status',
            'runtime_status',
            'publication_status',
            'retry_safe',
            'warning',
          };
    _revision3ProjectImportExactFields(
      json,
      expectedFields,
      'destination response',
    );
    if (json['ok'] != true ||
        json['format'] != revision3ProjectImportFormatV2 ||
        json['artifact_kind'] != revision3ProjectImportArtifactKindV2 ||
        json['restore_status'] != revision3ProjectImportRestoreStatusV2 ||
        json['inspection_status'] != _revision3ProjectImportInspectionStatus ||
        json['import_status'] != _revision3ProjectImportMaterializedStatus ||
        json['project_mutation'] != _revision3ProjectImportMaterializedStatus ||
        json['session_adoption'] != _revision3ProjectImportNotPerformed ||
        json['game_mutation'] != _revision3ProjectImportNotPerformed ||
        json['save_mutation'] != _revision3ProjectImportNotPerformed ||
        json['build_status'] != _revision3ProjectImportNotPerformed ||
        json['deployment_status'] != _revision3ProjectImportNotPerformed ||
        json['runtime_status'] != _revision3ProjectImportRuntimeStatus ||
        json['retry_safe'] != false) {
      throw const FormatException(
        'revision-3 project import destination status pairing is invalid',
      );
    }

    final source = _revision3ProjectImportSource(
      _revision3ProjectImportString(
        json['source'],
        'destination response source',
        maxBytes: revision3ProjectImportMaxSourceUtf8Bytes,
      ),
    );
    final destination = _revision3ProjectImportDestination(
      _revision3ProjectImportString(
        json['destination'],
        'destination response destination',
        maxBytes: revision3ProjectImportMaxDestinationUtf8Bytes,
      ),
    );
    if (source != expectedPlan.source ||
        destination != expectedPlan.destination) {
      throw const FormatException(
        'revision-3 project import destination response disagrees with its exact request spellings',
      );
    }

    if (outcome ==
        Revision3ProjectImportMaterializationOutcome.publicationUncertain) {
      if (json['publication_status'] !=
          _revision3ProjectImportPublicationUncertainStatus) {
        throw const FormatException(
          'revision-3 project import uncertain terminal metadata is invalid',
        );
      }
      _revision3ProjectImportWarning(
        json['warning'],
        expectedCode: _revision3ProjectImportPublicationUncertainCode,
        expectedMessage: _revision3ProjectImportPublicationUncertainMessage,
      );
      return const Revision3ProjectImportDestinationResult._(
        outcome:
            Revision3ProjectImportMaterializationOutcome.publicationUncertain,
        receipt: null,
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
      'destination project revision',
      min: 0,
      max: _revision3ProjectImportMaxSignedWireInteger,
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _revision3ProjectImportString(
        json['head_json'],
        'destination project head',
        maxBytes: revision3ProjectImportMaxHeadJsonUtf8Bytes,
      ),
    );
    final closure = Revision3ProjectImportClosure._fromJson(json['closure']);
    final inspected = expectedPlan.inspection;
    if (!_sameRevision3ProjectImportArchive(archive, inspected.archive) ||
        !_sameRevision3ProjectImportManifest(manifest, inspected.manifest) ||
        projectId != inspected.projectId ||
        projectRevision != inspected.projectRevision ||
        head.canonicalJson != inspected.head.canonicalJson ||
        !_sameRevision3ProjectImportClosure(closure, inspected.closure)) {
      throw const FormatException(
        'revision-3 project import destination response disagrees with its inspected project identity',
      );
    }

    switch (outcome) {
      case Revision3ProjectImportMaterializationOutcome.imported:
        if (json['publication_status'] !=
                _revision3ProjectImportPublishedStatus ||
            json['warning'] != null) {
          throw const FormatException(
            'revision-3 project import published terminal metadata is invalid',
          );
        }
      case Revision3ProjectImportMaterializationOutcome
          .importedWithCleanupWarning:
        if (json['publication_status'] !=
            _revision3ProjectImportPublishedWithCleanupWarningStatus) {
          throw const FormatException(
            'revision-3 project import cleanup-warning metadata is invalid',
          );
        }
        _revision3ProjectImportWarning(
          json['warning'],
          expectedCode: _revision3ProjectImportCleanupWarningCode,
          expectedMessage: _revision3ProjectImportCleanupWarningMessage,
        );
      case Revision3ProjectImportMaterializationOutcome.publicationUncertain:
        throw StateError('unreachable publication-uncertain receipt branch');
    }

    final receipt = Revision3ProjectImportedReceipt._(
      source: source,
      destination: destination,
      archive: archive,
      manifest: manifest,
      projectId: projectId,
      projectRevision: projectRevision,
      head: head,
      closure: closure,
    );
    return Revision3ProjectImportDestinationResult._(
      outcome: outcome,
      receipt: receipt,
    );
  }
}

enum Revision3ProjectImportPlanningOutcome {
  inspected,
  cancelled,
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
      } on ModFfiException catch (error) {
        final gate = _gateAfterAwait(epoch, lifecycle);
        if (gate != null) return gate;
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
      } on FormatException {
        return const Revision3ProjectImportPlanningResult.inspectionFailed();
      }
      final finalGate = _gateAfterAwait(epoch, lifecycle);
      if (finalGate != null) return finalGate;
      return Revision3ProjectImportPlanningResult.inspected(
        Revision3ProjectImportInspectionPlan._(inspection, lifecycle),
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

enum Revision3ProjectImportDestinationExecutionOutcome {
  imported,
  importedWithCleanupWarning,
  publicationUncertain,
  cancelled,
  invalidDestination,
  inspectionExpired,
  importFailed,
  stale,
  superseded,
  busy,
  unavailable,
}

/// Sanitized destination-import result. Every terminal is deliberately
/// non-retryable; a new user gesture and, where needed, a new inspection are
/// required. Raw native exceptions and paths are never retained.
final class Revision3ProjectImportDestinationExecutionResult {
  const Revision3ProjectImportDestinationExecutionResult._(
    this.outcome, {
    this.receipt,
  });

  const Revision3ProjectImportDestinationExecutionResult.imported(
    Revision3ProjectImportedReceipt receipt,
  ) : this._(
        Revision3ProjectImportDestinationExecutionOutcome.imported,
        receipt: receipt,
      );

  const Revision3ProjectImportDestinationExecutionResult.importedWithCleanupWarning(
    Revision3ProjectImportedReceipt receipt,
  ) : this._(
        Revision3ProjectImportDestinationExecutionOutcome
            .importedWithCleanupWarning,
        receipt: receipt,
      );

  const Revision3ProjectImportDestinationExecutionResult.publicationUncertain()
    : this._(
        Revision3ProjectImportDestinationExecutionOutcome.publicationUncertain,
      );

  const Revision3ProjectImportDestinationExecutionResult.cancelled()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.cancelled);

  const Revision3ProjectImportDestinationExecutionResult.invalidDestination()
    : this._(
        Revision3ProjectImportDestinationExecutionOutcome.invalidDestination,
      );

  const Revision3ProjectImportDestinationExecutionResult.inspectionExpired()
    : this._(
        Revision3ProjectImportDestinationExecutionOutcome.inspectionExpired,
      );

  const Revision3ProjectImportDestinationExecutionResult.importFailed()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.importFailed);

  const Revision3ProjectImportDestinationExecutionResult.stale()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.stale);

  const Revision3ProjectImportDestinationExecutionResult.superseded()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.superseded);

  const Revision3ProjectImportDestinationExecutionResult.busy()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.busy);

  const Revision3ProjectImportDestinationExecutionResult.unavailable()
    : this._(Revision3ProjectImportDestinationExecutionOutcome.unavailable);

  final Revision3ProjectImportDestinationExecutionOutcome outcome;
  final Revision3ProjectImportedReceipt? receipt;

  bool get retrySafe => false;
}

typedef Revision3ProjectImportDestinationPicker = Future<String?> Function();
typedef Revision3ProjectImportNativeDestinationImporter =
    Future<Object?> Function(Revision3ProjectImportDestinationRequest request);

/// Single-flight destination materialization for one inspected V2 archive.
///
/// The coordinator has no session-adoption callback and no game path. It
/// checks lifecycle and epoch state after every await, suppresses all late
/// receipts, and invokes native at most once per user operation.
final class Revision3ProjectImportDestinationCoordinator {
  factory Revision3ProjectImportDestinationCoordinator({
    required Revision3ProjectImportLifecycleReader readLifecycle,
    required Revision3ProjectImportDestinationPicker pickDestination,
    required Revision3ProjectImportNativeDestinationImporter importProject,
  }) => Revision3ProjectImportDestinationCoordinator._(
    readLifecycle,
    pickDestination,
    importProject,
  );

  Revision3ProjectImportDestinationCoordinator._(
    this._readLifecycle,
    this._pickDestination,
    this._importProject,
  );

  final Revision3ProjectImportLifecycleReader _readLifecycle;
  final Revision3ProjectImportDestinationPicker _pickDestination;
  final Revision3ProjectImportNativeDestinationImporter _importProject;

  bool _busy = false;
  bool _disposed = false;
  int _epoch = 0;

  bool get isBusy => _busy;
  bool get isDisposed => _disposed;

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

  Future<Revision3ProjectImportDestinationExecutionResult> materialize(
    Revision3ProjectImportInspectionPlan inspected,
  ) async {
    if (_disposed) {
      return const Revision3ProjectImportDestinationExecutionResult.unavailable();
    }
    if (_busy) {
      return const Revision3ProjectImportDestinationExecutionResult.busy();
    }

    _busy = true;
    final epoch = ++_epoch;
    try {
      final lifecycle = _safeReadLifecycle();
      if (!_validRevision3ProjectImportLifecycle(lifecycle)) {
        return const Revision3ProjectImportDestinationExecutionResult.unavailable();
      }
      if (!lifecycle!.sameAs(inspected._lifecycle)) {
        return const Revision3ProjectImportDestinationExecutionResult.stale();
      }

      final String? selectedDestination;
      try {
        selectedDestination = await _pickDestination();
      } catch (_) {
        return _failureAfterAwait(epoch, lifecycle);
      }
      final gateAfterPicker = _gateAfterAwait(epoch, lifecycle);
      if (gateAfterPicker != null) return gateAfterPicker;
      if (selectedDestination == null) {
        return const Revision3ProjectImportDestinationExecutionResult.cancelled();
      }

      final Revision3ProjectImportDestinationPlan destinationPlan;
      try {
        destinationPlan = Revision3ProjectImportDestinationPlan.fromInspection(
          inspection: inspected.inspection,
          destination: selectedDestination,
        );
      } on FormatException {
        return const Revision3ProjectImportDestinationExecutionResult.invalidDestination();
      }

      final Object? response;
      try {
        response = await _importProject(destinationPlan.request);
      } on ModFfiException catch (error) {
        final gate = _gateAfterAwait(epoch, lifecycle);
        if (gate != null) return gate;
        if (error.command == _revision3ProjectImportDestinationCommand &&
            error.code == _revision3ProjectImportDestinationInvalidCode) {
          return const Revision3ProjectImportDestinationExecutionResult.invalidDestination();
        }
        if (error.command == _revision3ProjectImportDestinationCommand &&
            error.code == _revision3ProjectImportSourceChangedCode) {
          return const Revision3ProjectImportDestinationExecutionResult.inspectionExpired();
        }
        if (error.command == _revision3ProjectImportDestinationCommand &&
            error.code == _revision3ProjectImportPlatformUnsupportedCode) {
          return const Revision3ProjectImportDestinationExecutionResult.unavailable();
        }
        return const Revision3ProjectImportDestinationExecutionResult.importFailed();
      } catch (_) {
        return _failureAfterAwait(epoch, lifecycle);
      }
      final gateAfterImport = _gateAfterAwait(epoch, lifecycle);
      if (gateAfterImport != null) return gateAfterImport;

      final Revision3ProjectImportDestinationResult terminal;
      try {
        terminal = Revision3ProjectImportDestinationResult.fromJson(
          response,
          expectedPlan: destinationPlan,
        );
      } on FormatException {
        return const Revision3ProjectImportDestinationExecutionResult.importFailed();
      }
      final finalGate = _gateAfterAwait(epoch, lifecycle);
      if (finalGate != null) return finalGate;
      switch (terminal.outcome) {
        case Revision3ProjectImportMaterializationOutcome.imported:
          return Revision3ProjectImportDestinationExecutionResult.imported(
            terminal.receipt!,
          );
        case Revision3ProjectImportMaterializationOutcome
            .importedWithCleanupWarning:
          return Revision3ProjectImportDestinationExecutionResult.importedWithCleanupWarning(
            terminal.receipt!,
          );
        case Revision3ProjectImportMaterializationOutcome.publicationUncertain:
          return const Revision3ProjectImportDestinationExecutionResult.publicationUncertain();
      }
    } finally {
      _busy = false;
    }
  }

  Revision3ProjectImportDestinationExecutionResult _failureAfterAwait(
    int epoch,
    Revision3ProjectImportLifecycle lifecycle,
  ) =>
      _gateAfterAwait(epoch, lifecycle) ??
      const Revision3ProjectImportDestinationExecutionResult.importFailed();

  Revision3ProjectImportDestinationExecutionResult? _gateAfterAwait(
    int epoch,
    Revision3ProjectImportLifecycle lifecycle,
  ) {
    if (_disposed || epoch != _epoch) {
      return const Revision3ProjectImportDestinationExecutionResult.superseded();
    }
    final current = _safeReadLifecycle();
    if (current == null || !current.sameAs(lifecycle)) {
      return const Revision3ProjectImportDestinationExecutionResult.stale();
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

void _revision3ProjectImportWarning(
  Object? value, {
  required String expectedCode,
  required String expectedMessage,
}) {
  final json = _revision3ProjectImportObject(value, 'terminal warning');
  _revision3ProjectImportExactFields(json, const <String>{
    'code',
    'message',
  }, 'terminal warning');
  if (_revision3ProjectImportString(
        json['code'],
        'terminal warning code',
        maxBytes: 128,
      ) !=
      expectedCode) {
    throw const FormatException(
      'revision-3 project import terminal warning code is invalid',
    );
  }
  if (_revision3ProjectImportString(
        json['message'],
        'terminal warning message',
        maxBytes: 256,
      ) !=
      expectedMessage) {
    throw const FormatException(
      'revision-3 project import terminal warning message is invalid',
    );
  }
}

bool _sameRevision3ProjectImportArchive(
  Revision3ProjectImportArchiveSeal left,
  Revision3ProjectImportArchiveSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _sameRevision3ProjectImportManifest(
  Revision3ProjectImportManifestSeal left,
  Revision3ProjectImportManifestSeal right,
) =>
    left.relativeName == right.relativeName &&
    left.byteLength == right.byteLength &&
    left.sha256 == right.sha256;

bool _sameRevision3ProjectImportClosure(
  Revision3ProjectImportClosure left,
  Revision3ProjectImportClosure right,
) =>
    left.snapshotObjects == right.snapshotObjects &&
    left.entityObjects == right.entityObjects &&
    left.assetObjects == right.assetObjects &&
    left.archiveEntries == right.archiveEntries &&
    left.uncompressedBytes == right.uncompressedBytes;

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

String _revision3ProjectImportDestination(String value) {
  _revision3ProjectImportString(
    value,
    'destination spelling',
    maxBytes: revision3ProjectImportMaxDestinationUtf8Bytes,
  );
  for (final rune in value.runes) {
    if (rune < 0x20 || (rune >= 0x7f && rune <= 0x9f)) {
      throw const FormatException(
        'revision-3 project import destination spelling contains a control character',
      );
    }
  }

  final bool driveRooted = _revision3ProjectImportWindowsDriveRootPattern
      .hasMatch(value);
  final bool uncRooted = value.startsWith(r'\\') || value.startsWith('//');
  if (!driveRooted && !uncRooted) {
    throw const FormatException(
      'revision-3 project import destination is not an absolute Windows path',
    );
  }

  final tail = value.substring(driveRooted ? 3 : 2);
  final segments = tail.split(_revision3ProjectImportWindowsSeparatorPattern);
  if (segments.isEmpty || (uncRooted && segments.length < 3)) {
    throw const FormatException(
      'revision-3 project import destination does not name a project directory',
    );
  }
  for (final segment in segments) {
    if (segment.isEmpty ||
        segment == '.' ||
        segment == '..' ||
        segment.endsWith('.') ||
        segment.endsWith(' ') ||
        _revision3ProjectImportWindowsInvalidSegmentPattern.hasMatch(segment) ||
        _revision3ProjectImportWindowsReservedSegmentPattern.hasMatch(
          segment,
        )) {
      throw const FormatException(
        'revision-3 project import destination contains a non-canonical Windows path segment',
      );
    }
  }
  return value;
}

String _revision3ProjectImportBoundedBasename(String path, String context) {
  final slash = path.lastIndexOf('/');
  final backslash = path.lastIndexOf(r'\');
  final separator = slash > backslash ? slash : backslash;
  final basename = path.substring(separator + 1);
  if (_revision3ProjectImportUtf8Length(basename, context) <=
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
