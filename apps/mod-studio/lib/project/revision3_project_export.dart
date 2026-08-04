part of '../core/mod_ffi.dart';

const _authoringRevision3ExactSnapshotExportFormatV2 =
    'managed_revision3_exact_snapshot_v2';
const _authoringRevision3ExactSnapshotExportArtifactKindV2 =
    'portable_snapshot_restorable_copy';
const _authoringRevision3ExactSnapshotManifestName = 'gore-export.json';
const _maxAuthoringRevision3ExactSnapshotManifestBytes = 128 * 1024 * 1024;
const _maxAuthoringRevision3ExactSnapshotClosureObjects = 100000;
const _maxAuthoringRevision3ExactSnapshotArchiveEntries = 300003;
const _maxAuthoringRevision3ExactSnapshotArchiveBytesV2 =
    70 * 1024 * 1024 * 1024;

enum AuthoringRevision3ExactSnapshotExportOutcome {
  exported,
  exportedWithCleanupWarning,
  publicationUncertain,
}

enum AuthoringRevision3ExactSnapshotExportPublicationStatus {
  published,
  publishedWithCleanupWarning,
  publicationUncertain,
}

final class AuthoringRevision3ExactSnapshotArchiveSeal {
  const AuthoringRevision3ExactSnapshotArchiveSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  factory AuthoringRevision3ExactSnapshotArchiveSeal._fromJson(
    Object? value, {
    int maxByteLength = _maxAuthoringSignedJsonInteger,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 exact snapshot archive seal',
    );
    _authoringExactFields(json, const <String>{
      'byte_len',
      'sha256',
    }, 'revision-3 exact snapshot archive seal');
    final byteLength = _authoringRequiredInt(
      json,
      'byte_len',
      min: 1,
      max: maxByteLength,
    );
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException(
        'revision-3 exact snapshot archive seal is not canonical',
      );
    }
    return AuthoringRevision3ExactSnapshotArchiveSeal._(
      byteLength: byteLength,
      sha256: sha256,
    );
  }
}

final class AuthoringRevision3ExactSnapshotManifestSeal {
  const AuthoringRevision3ExactSnapshotManifestSeal._({
    required this.relativeName,
    required this.byteLength,
    required this.sha256,
  });

  final String relativeName;
  final int byteLength;
  final String sha256;

  factory AuthoringRevision3ExactSnapshotManifestSeal._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 exact snapshot manifest seal',
    );
    _authoringExactFields(json, const <String>{
      'relative_name',
      'byte_len',
      'sha256',
    }, 'revision-3 exact snapshot manifest seal');
    final relativeName = _authoringRequiredString(
      json,
      'relative_name',
      maxBytes: 64,
    );
    final byteLength = _authoringRequiredInt(
      json,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3ExactSnapshotManifestBytes,
    );
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (relativeName != _authoringRevision3ExactSnapshotManifestName ||
        !_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException(
        'revision-3 exact snapshot manifest seal is not canonical',
      );
    }
    return AuthoringRevision3ExactSnapshotManifestSeal._(
      relativeName: relativeName,
      byteLength: byteLength,
      sha256: sha256,
    );
  }
}

final class AuthoringRevision3ExactSnapshotClosure {
  const AuthoringRevision3ExactSnapshotClosure._({
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

  factory AuthoringRevision3ExactSnapshotClosure._fromJson(
    Object? value, {
    int maxUncompressedBytes = _maxAuthoringSignedJsonInteger,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 exact snapshot closure',
    );
    _authoringExactFields(json, const <String>{
      'snapshot_objects',
      'entity_objects',
      'asset_objects',
      'archive_entries',
      'uncompressed_bytes',
    }, 'revision-3 exact snapshot closure');
    final snapshotObjects = _authoringRequiredInt(
      json,
      'snapshot_objects',
      min: 1,
      max: _maxAuthoringRevision3ExactSnapshotClosureObjects,
    );
    final entityObjects = _authoringRequiredInt(
      json,
      'entity_objects',
      max: _maxAuthoringRevision3ExactSnapshotClosureObjects,
    );
    final assetObjects = _authoringRequiredInt(
      json,
      'asset_objects',
      max: _maxAuthoringRevision3ExactSnapshotClosureObjects,
    );
    final archiveEntries = _authoringRequiredInt(
      json,
      'archive_entries',
      min: 4,
      max: _maxAuthoringRevision3ExactSnapshotArchiveEntries,
    );
    final uncompressedBytes = _authoringRequiredInt(
      json,
      'uncompressed_bytes',
      min: 1,
      max: maxUncompressedBytes,
    );
    if (archiveEntries != 3 + snapshotObjects + entityObjects + assetObjects) {
      throw const FormatException(
        'revision-3 exact snapshot closure has an inconsistent archive entry count',
      );
    }
    return AuthoringRevision3ExactSnapshotClosure._(
      snapshotObjects: snapshotObjects,
      entityObjects: entityObjects,
      assetObjects: assetObjects,
      archiveEntries: archiveEntries,
      uncompressedBytes: uncompressedBytes,
    );
  }
}

final class AuthoringRevision3ExactSnapshotExportWarning {
  const AuthoringRevision3ExactSnapshotExportWarning._({
    required this.code,
    required this.message,
  });

  final String code;
  final String message;

  factory AuthoringRevision3ExactSnapshotExportWarning._fromJson(
    Object? value, {
    required String expectedCode,
    required String expectedMessage,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 exact snapshot export warning',
    );
    _authoringExactFields(json, const <String>{
      'code',
      'message',
    }, 'revision-3 exact snapshot export warning');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
    final message = _authoringRequiredString(json, 'message', maxBytes: 256);
    if (code != expectedCode || message != expectedMessage) {
      throw const FormatException(
        'revision-3 exact snapshot export warning is not canonical',
      );
    }
    return AuthoringRevision3ExactSnapshotExportWarning._(
      code: code,
      message: message,
    );
  }
}

/// Receipt for the closed restorable V2 exact-snapshot archive.
///
/// Snapshot V2 is the only project-backup format exposed by Mod Studio.
final class AuthoringRevision3ExactSnapshotExportResultV2 {
  const AuthoringRevision3ExactSnapshotExportResultV2._({
    required this.outcome,
    required this.publicationStatus,
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.output,
    required this.archive,
    required this.manifest,
    required this.closure,
    required this.warning,
  });

  final AuthoringRevision3ExactSnapshotExportOutcome outcome;
  final AuthoringRevision3ExactSnapshotExportPublicationStatus
  publicationStatus;
  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final String output;
  final AuthoringRevision3ExactSnapshotArchiveSeal archive;
  final AuthoringRevision3ExactSnapshotManifestSeal manifest;
  final AuthoringRevision3ExactSnapshotClosure closure;
  final AuthoringRevision3ExactSnapshotExportWarning? warning;

  bool get publicationIsUncertain =>
      outcome ==
      AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain;

  bool get hasCleanupWarning =>
      outcome ==
      AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning;

  /// V2's closed authority is a restorable project copy. It is still not a
  /// build, deployment, runtime qualification, game write, or save write.
  bool get isRestorableProjectCopy => true;

  factory AuthoringRevision3ExactSnapshotExportResultV2.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String expectedOutput,
  }) {
    try {
      _authoringRevision3Path(expectedOutput, 'expectedOutput');
    } on ArgumentError {
      throw const FormatException(
        'revision-3 exact snapshot export expectation is invalid',
      );
    }
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'format',
      'artifact_kind',
      'restore_status',
      'basis_head_json',
      'project_id',
      'project_revision',
      'output',
      'archive',
      'manifest',
      'closure',
      'publication_status',
      'retry_safe',
      'warning',
      'project_mutation',
      'game_mutation',
      'save_mutation',
      'build_status',
      'deployment_status',
      'runtime_status',
    }, 'revision-3 exact snapshot V2 export response');
    if (json['ok'] != true) {
      throw const FormatException(
        'revision-3 exact snapshot V2 export response is not successful',
      );
    }
    final outcome = switch (json['outcome']) {
      'exported' => AuthoringRevision3ExactSnapshotExportOutcome.exported,
      'exported_with_cleanup_warning' =>
        AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning,
      'publication_uncertain' =>
        AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain,
      _ => throw const FormatException(
        'revision-3 exact snapshot V2 export response has an unknown outcome',
      ),
    };
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    if (projectId == '00000000000000000000000000000000') {
      throw const FormatException(
        'revision-3 exact snapshot V2 export project ID is zero',
      );
    }
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final output = _authoringRequiredString(
      json,
      'output',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        output != expectedOutput) {
      throw const FormatException(
        'revision-3 exact snapshot V2 export response disagrees with its exact request basis',
      );
    }

    final archive = AuthoringRevision3ExactSnapshotArchiveSeal._fromJson(
      json['archive'],
      maxByteLength: _maxAuthoringRevision3ExactSnapshotArchiveBytesV2,
    );
    final manifest = AuthoringRevision3ExactSnapshotManifestSeal._fromJson(
      json['manifest'],
    );
    final closure = AuthoringRevision3ExactSnapshotClosure._fromJson(
      json['closure'],
      maxUncompressedBytes: _maxAuthoringRevision3ExactSnapshotArchiveBytesV2,
    );
    if (archive.byteLength <= closure.uncompressedBytes ||
        manifest.byteLength > closure.uncompressedBytes) {
      throw const FormatException(
        'revision-3 exact snapshot V2 archive and closure byte counts are inconsistent',
      );
    }
    if (json['format'] != _authoringRevision3ExactSnapshotExportFormatV2 ||
        json['artifact_kind'] !=
            _authoringRevision3ExactSnapshotExportArtifactKindV2 ||
        json['restore_status'] != 'supported' ||
        json['project_mutation'] != 'not_performed' ||
        json['game_mutation'] != 'not_performed' ||
        json['save_mutation'] != 'not_performed' ||
        json['build_status'] != 'not_performed' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        _authoringRequiredBool(json, 'retry_safe')) {
      throw const FormatException(
        'revision-3 exact snapshot V2 export response widens or changes its closed authority',
      );
    }

    final AuthoringRevision3ExactSnapshotExportPublicationStatus
    publicationStatus;
    final AuthoringRevision3ExactSnapshotExportWarning? warning;
    switch (outcome) {
      case AuthoringRevision3ExactSnapshotExportOutcome.exported:
        if (json['publication_status'] != 'published' ||
            json['warning'] != null) {
          throw const FormatException(
            'published revision-3 exact snapshot V2 export has invalid terminal metadata',
          );
        }
        publicationStatus =
            AuthoringRevision3ExactSnapshotExportPublicationStatus.published;
        warning = null;
      case AuthoringRevision3ExactSnapshotExportOutcome
          .exportedWithCleanupWarning:
        if (json['publication_status'] != 'published_with_cleanup_warning') {
          throw const FormatException(
            'cleanup-warning revision-3 exact snapshot V2 export has invalid publication status',
          );
        }
        publicationStatus =
            AuthoringRevision3ExactSnapshotExportPublicationStatus
                .publishedWithCleanupWarning;
        warning = AuthoringRevision3ExactSnapshotExportWarning._fromJson(
          json['warning'],
          expectedCode: 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
          expectedMessage:
              'the verified snapshot was published, but private staging cleanup was incomplete',
        );
      case AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain:
        if (json['publication_status'] != 'publication_uncertain') {
          throw const FormatException(
            'uncertain revision-3 exact snapshot V2 export has invalid publication status',
          );
        }
        publicationStatus =
            AuthoringRevision3ExactSnapshotExportPublicationStatus
                .publicationUncertain;
        warning = AuthoringRevision3ExactSnapshotExportWarning._fromJson(
          json['warning'],
          expectedCode: 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
          expectedMessage:
              'publication may have completed; do not retry automatically',
        );
    }

    return AuthoringRevision3ExactSnapshotExportResultV2._(
      outcome: outcome,
      publicationStatus: publicationStatus,
      basisHead: basisHead,
      projectId: projectId,
      projectRevision: projectRevision,
      output: output,
      archive: archive,
      manifest: manifest,
      closure: closure,
      warning: warning,
    );
  }
}

extension AuthoringRevision3ExactSnapshotExportV2ModFfi on ModFfi {
  /// Export the exact current managed revision-3 checkpoint as a portable,
  /// restorable V2 project copy without mutating the project, game, or saves.
  Future<AuthoringRevision3ExactSnapshotExportResultV2>
  authoringStoreExportRevision3ExactSnapshotV2({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) async {
    const command = 'authoring_store_export_revision3_exact_snapshot_v2';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(output, 'output');
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'output': output,
      'root': root,
    });
    try {
      return AuthoringRevision3ExactSnapshotExportResultV2.fromJson(
        response,
        expectedHead: expectedHead,
        expectedOutput: output,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }
}
