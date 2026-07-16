part of '../core/mod_ffi.dart';

const _authoringRevision3ReviewedDataAssetBuildReceiptFormat =
    'gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1';
const _authoringRevision3ReviewedDataAssetBuildReceiptName =
    'gore-authoring-dataasset-build.json';
const _maxAuthoringRevision3ReviewedDataAssetPackNameBytes = 96;
const _maxAuthoringRevision3ReviewedDataAssetTripletFileBytes =
    2 * 1024 * 1024 * 1024;
const _maxAuthoringRevision3ReviewedDataAssetReceiptBytes = 8 * 1024 * 1024;
const _maxAuthoringRevision3ReviewedDataAssetBuildRequestBytes =
    _maxAuthoringProjectJsonBytes * 2 +
    _maxAuthoringHeadJsonBytes * 2 +
    _maxAuthoringStorePathBytes * 6 +
    512 * 2 +
    _maxAuthoringRevision3ReviewedDataAssetPackNameBytes * 2 +
    4096;

enum AuthoringRevision3ReviewedDataAssetBuildOutcome {
  published,
  publishedWithCleanupWarning,
  publicationUncertain,
}

final class AuthoringRevision3ReviewedDataAssetBuildFileSeal {
  const AuthoringRevision3ReviewedDataAssetBuildFileSeal._({
    required this.relativeName,
    required this.byteLength,
    required this.sha256,
  });

  final String relativeName;
  final int byteLength;
  final String sha256;

  factory AuthoringRevision3ReviewedDataAssetBuildFileSeal._fromJson(
    Object? value, {
    required String expectedRelativeName,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 reviewed DataAsset build file seal',
    );
    _authoringExactFields(json, const <String>{
      'relative_name',
      'byte_len',
      'sha256',
    }, 'revision-3 reviewed DataAsset build file seal');
    final relativeName = _authoringRequiredString(
      json,
      'relative_name',
      maxBytes: _maxAuthoringRevision3ReviewedDataAssetPackNameBytes + 5,
    );
    final byteLength = _authoringRequiredInt(
      json,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3ReviewedDataAssetTripletFileBytes,
    );
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (relativeName != expectedRelativeName ||
        relativeName.contains('/') ||
        relativeName.contains(r'\') ||
        !_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build file seal is not exact and path-free',
      );
    }
    return AuthoringRevision3ReviewedDataAssetBuildFileSeal._(
      relativeName: relativeName,
      byteLength: byteLength,
      sha256: sha256,
    );
  }
}

final class AuthoringRevision3ReviewedDataAssetBuildReceiptSeal {
  const AuthoringRevision3ReviewedDataAssetBuildReceiptSeal._({
    required this.format,
    required this.relativeName,
    required this.byteLength,
    required this.sha256,
  });

  final String format;
  final String relativeName;
  final int byteLength;
  final String sha256;

  factory AuthoringRevision3ReviewedDataAssetBuildReceiptSeal._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 reviewed DataAsset build receipt seal',
    );
    _authoringExactFields(json, const <String>{
      'format',
      'relative_name',
      'byte_len',
      'sha256',
    }, 'revision-3 reviewed DataAsset build receipt seal');
    final format = _authoringRequiredString(json, 'format', maxBytes: 128);
    final relativeName = _authoringRequiredString(
      json,
      'relative_name',
      maxBytes: 64,
    );
    final byteLength = _authoringRequiredInt(
      json,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3ReviewedDataAssetReceiptBytes,
    );
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (format != _authoringRevision3ReviewedDataAssetBuildReceiptFormat ||
        relativeName != _authoringRevision3ReviewedDataAssetBuildReceiptName ||
        relativeName.contains('/') ||
        relativeName.contains(r'\') ||
        !_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build receipt seal is not canonical',
      );
    }
    return AuthoringRevision3ReviewedDataAssetBuildReceiptSeal._(
      format: format,
      relativeName: relativeName,
      byteLength: byteLength,
      sha256: sha256,
    );
  }
}

final class AuthoringRevision3ReviewedDataAssetBuildWarning {
  const AuthoringRevision3ReviewedDataAssetBuildWarning._({
    required this.code,
    required this.message,
  });

  final String code;
  final String message;

  factory AuthoringRevision3ReviewedDataAssetBuildWarning._fromJson(
    Object? value, {
    required String expectedCode,
    required String expectedMessage,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 reviewed DataAsset build warning',
    );
    _authoringExactFields(json, const <String>{
      'code',
      'message',
    }, 'revision-3 reviewed DataAsset build warning');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
    final message = _authoringRequiredString(json, 'message', maxBytes: 256);
    if (code != expectedCode || message != expectedMessage) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build warning is not canonical',
      );
    }
    return AuthoringRevision3ReviewedDataAssetBuildWarning._(
      code: code,
      message: message,
    );
  }
}

final class AuthoringRevision3ReviewedDataAssetBuildResult {
  AuthoringRevision3ReviewedDataAssetBuildResult._({
    required this.outcome,
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.targetPath,
    required this.packName,
    required this.output,
    required List<AuthoringRevision3ReviewedDataAssetBuildFileSeal> files,
    required this.receipt,
    required this.warning,
  }) : files = List.unmodifiable(files);

  final AuthoringRevision3ReviewedDataAssetBuildOutcome outcome;
  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final String targetPath;
  final String packName;
  final String output;
  final List<AuthoringRevision3ReviewedDataAssetBuildFileSeal> files;
  final AuthoringRevision3ReviewedDataAssetBuildReceiptSeal receipt;
  final AuthoringRevision3ReviewedDataAssetBuildWarning? warning;

  bool get publicationIsUncertain =>
      outcome ==
      AuthoringRevision3ReviewedDataAssetBuildOutcome.publicationUncertain;

  bool get hasCleanupWarning =>
      outcome ==
      AuthoringRevision3ReviewedDataAssetBuildOutcome
          .publishedWithCleanupWarning;

  factory AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectJson,
    required String expectedTargetPath,
    required String expectedPackName,
    required String expectedOutput,
  }) {
    final expectedProject = _authoringRequireCanonicalRevision3ProjectJson(
      expectedProjectJson,
    );
    try {
      _authoringRevision3DataAssetTargetPath(
        expectedTargetPath,
        'expectedTargetPath',
      );
      _authoringRevision3ReviewedDataAssetPackName(
        expectedPackName,
        'expectedPackName',
      );
      _authoringRevision3Path(expectedOutput, 'expectedOutput');
    } on ArgumentError {
      throw const FormatException(
        'revision-3 reviewed DataAsset build expectation is invalid',
      );
    }
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'target_path',
      'pack_name',
      'output',
      'files',
      'receipt',
      'build_authority',
      'artifact_publication_status',
      'deployment_status',
      'runtime_status',
      'retry_safe',
      'warning',
    }, 'revision-3 reviewed DataAsset build response');
    if (json['ok'] != true) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build response is not successful',
      );
    }
    final outcome = switch (json['outcome']) {
      'built' => AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
      'built_with_cleanup_warning' =>
        AuthoringRevision3ReviewedDataAssetBuildOutcome
            .publishedWithCleanupWarning,
      'publication_uncertain' =>
        AuthoringRevision3ReviewedDataAssetBuildOutcome.publicationUncertain,
      _ => throw const FormatException(
        'revision-3 reviewed DataAsset build response has an unknown outcome',
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
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final targetPath = _authoringRequiredString(
      json,
      'target_path',
      maxBytes: 512,
    );
    final packName = _authoringRequiredString(
      json,
      'pack_name',
      maxBytes: _maxAuthoringRevision3ReviewedDataAssetPackNameBytes,
    );
    final output = _authoringRequiredString(
      json,
      'output',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        projectId != expectedProject.projectId ||
        projectRevision != expectedProject.revision ||
        targetPath != expectedTargetPath ||
        packName != expectedPackName ||
        output != expectedOutput) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build response disagrees with its exact request basis',
      );
    }

    final rawFiles = json['files'];
    if (rawFiles is! List || rawFiles.length != 3) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build response must contain exactly three file seals',
      );
    }
    final expectedNames = <String>[
      '$expectedPackName.pak',
      '$expectedPackName.ucas',
      '$expectedPackName.utoc',
    ];
    final files = <AuthoringRevision3ReviewedDataAssetBuildFileSeal>[
      for (var index = 0; index < expectedNames.length; index++)
        AuthoringRevision3ReviewedDataAssetBuildFileSeal._fromJson(
          rawFiles[index],
          expectedRelativeName: expectedNames[index],
        ),
    ];
    final receipt =
        AuthoringRevision3ReviewedDataAssetBuildReceiptSeal._fromJson(
          json['receipt'],
        );

    if (json['build_authority'] !=
            'reviewed_fixed_leaf_single_package_triplet' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        _authoringRequiredBool(json, 'retry_safe')) {
      throw const FormatException(
        'revision-3 reviewed DataAsset build response widens its closed authority',
      );
    }

    final AuthoringRevision3ReviewedDataAssetBuildWarning? warning;
    switch (outcome) {
      case AuthoringRevision3ReviewedDataAssetBuildOutcome.published:
        if (json['artifact_publication_status'] != 'published' ||
            json['warning'] != null) {
          throw const FormatException(
            'published revision-3 reviewed DataAsset build has invalid terminal metadata',
          );
        }
        warning = null;
      case AuthoringRevision3ReviewedDataAssetBuildOutcome
          .publishedWithCleanupWarning:
        if (json['artifact_publication_status'] !=
            'published_with_cleanup_warning') {
          throw const FormatException(
            'cleanup-warning revision-3 reviewed DataAsset build has invalid publication status',
          );
        }
        warning = AuthoringRevision3ReviewedDataAssetBuildWarning._fromJson(
          json['warning'],
          expectedCode: 'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
          expectedMessage:
              'the verified build was published, but private staging cleanup was incomplete',
        );
      case AuthoringRevision3ReviewedDataAssetBuildOutcome.publicationUncertain:
        if (json['artifact_publication_status'] != 'publication_uncertain') {
          throw const FormatException(
            'uncertain revision-3 reviewed DataAsset build has invalid publication status',
          );
        }
        warning = AuthoringRevision3ReviewedDataAssetBuildWarning._fromJson(
          json['warning'],
          expectedCode:
              'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN',
          expectedMessage:
              'publication may have completed; do not retry automatically',
        );
    }

    return AuthoringRevision3ReviewedDataAssetBuildResult._(
      outcome: outcome,
      basisHead: basisHead,
      projectId: projectId,
      projectRevision: projectRevision,
      targetPath: targetPath,
      packName: packName,
      output: output,
      files: files,
      receipt: receipt,
      warning: warning,
    );
  }
}

void _authoringRevision3ReviewedDataAssetPackName(String value, String field) {
  final bytes = utf8.encode(value);
  final first = bytes.isEmpty ? null : bytes.first;
  bool asciiAlphaNumeric(int byte) =>
      (byte >= 0x30 && byte <= 0x39) ||
      (byte >= 0x41 && byte <= 0x5a) ||
      (byte >= 0x61 && byte <= 0x7a);
  if (bytes.isEmpty ||
      bytes.length > _maxAuthoringRevision3ReviewedDataAssetPackNameBytes ||
      bytes.any((byte) => byte > 0x7f) ||
      first == null ||
      !asciiAlphaNumeric(first) ||
      bytes.any(
        (byte) => !asciiAlphaNumeric(byte) && byte != 0x5f && byte != 0x2d,
      ) ||
      _authoringRevision3DataAssetWindowsReservedName(value)) {
    throw ArgumentError.value(
      value,
      field,
      'must be a non-reserved 1..=96 byte ASCII component',
    );
  }
}
