part of '../core/mod_ffi.dart';

const _maxRevision3DataAssetPackageCandidates = 250000;
const _maxRevision3DataAssetPhysicalChunks = 1000000;
const _maxRevision3DataAssetMountInventoryEntries = 8192;

final _revision3DataAssetPackageIdPattern = RegExp(r'^[0-9a-f]{16}$');

enum AuthoringRevision3DataAssetPackageIndexStatus {
  completeIndex,
  partialIndex,
}

enum AuthoringRevision3DataAssetPackagePartialReason {
  noncanonicalExportBundleChunkId,
  missingDirectoryIndexPath,
  noncanonicalGameDirectoryIndexPath,
  packageIdMismatch,
}

enum AuthoringRevision3DataAssetPackageIndexScope {
  installedDataAssetPackageCandidatesOnly,
}

enum AuthoringRevision3DataAssetPackageContentStatus { metadataCandidatesOnly }

enum AuthoringRevision3DataAssetExportBundlePayloadStatus { notRead }

enum AuthoringRevision3DataAssetPackageMutationStatus { notSupported }

enum AuthoringRevision3DataAssetPackageBuildStatus { notEvaluated }

enum AuthoringRevision3DataAssetPackageRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3DataAssetPackagePublicationStatus { notSupported }

enum AuthoringRevision3DataAssetPackageAuthorityStatus { notGranted }

final class AuthoringRevision3DataAssetPackageCandidate {
  const AuthoringRevision3DataAssetPackageCandidate._({
    required this.ordinal,
    required this.targetPath,
    required this.packageIdHex,
  });

  /// Stable position in the exact, sorted native snapshot. Search filtering
  /// must preserve this value; a visible-row index is never package authority.
  final int ordinal;
  final String targetPath;
  final String packageIdHex;

  factory AuthoringRevision3DataAssetPackageCandidate._fromJson(
    Object? value,
    int index,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 DataAsset package candidate $index',
    );
    _dataAssetPackageIndexFields(json, const <String>[
      'target_path',
      'package_id_hex',
    ], 'revision-3 DataAsset package candidate $index');
    final targetPath = _dataAssetPackageIndexString(
      json['target_path'],
      'revision-3 DataAsset package candidate $index target path',
      maxBytes: 512,
    );
    try {
      _authoringRevision3DataAssetTargetPath(targetPath, 'target_path');
    } on ArgumentError {
      throw FormatException(
        'revision-3 DataAsset package candidate $index has a noncanonical target path',
      );
    }
    final packageIdHex = _dataAssetPackageIndexString(
      json['package_id_hex'],
      'revision-3 DataAsset package candidate $index package ID',
      maxBytes: 16,
    );
    if (!_revision3DataAssetPackageIdPattern.hasMatch(packageIdHex)) {
      throw FormatException(
        'revision-3 DataAsset package candidate $index has a noncanonical package ID',
      );
    }
    return AuthoringRevision3DataAssetPackageCandidate._(
      ordinal: index,
      targetPath: targetPath,
      packageIdHex: packageIdHex,
    );
  }
}

final class AuthoringRevision3DataAssetPackagePartialReasonCount {
  const AuthoringRevision3DataAssetPackagePartialReasonCount._({
    required this.reason,
    required this.count,
  });

  final AuthoringRevision3DataAssetPackagePartialReason reason;
  final int count;

  factory AuthoringRevision3DataAssetPackagePartialReasonCount._fromJson(
    Object? value,
    int index,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 DataAsset package partial reason $index',
    );
    _dataAssetPackageIndexFields(json, const <String>[
      'reason',
      'count',
    ], 'revision-3 DataAsset package partial reason $index');
    return AuthoringRevision3DataAssetPackagePartialReasonCount._(
      reason: switch (json['reason']) {
        'noncanonical_export_bundle_chunk_id' =>
          AuthoringRevision3DataAssetPackagePartialReason
              .noncanonicalExportBundleChunkId,
        'missing_directory_index_path' =>
          AuthoringRevision3DataAssetPackagePartialReason
              .missingDirectoryIndexPath,
        'noncanonical_game_directory_index_path' =>
          AuthoringRevision3DataAssetPackagePartialReason
              .noncanonicalGameDirectoryIndexPath,
        'package_id_mismatch' =>
          AuthoringRevision3DataAssetPackagePartialReason.packageIdMismatch,
        _ => throw FormatException(
          'revision-3 DataAsset package partial reason $index is unsupported',
        ),
      },
      count: _dataAssetPackageIndexInt(
        json['count'],
        'revision-3 DataAsset package partial reason $index count',
        min: 1,
        max: _maxRevision3DataAssetPackageCandidates,
      ),
    );
  }
}

final class AuthoringRevision3DataAssetPackageIndex {
  const AuthoringRevision3DataAssetPackageIndex._({
    required this.status,
    required this.physicalChunkCount,
    required this.winningExportBundleCount,
    required this.directoryIndexedExportBundleCount,
    required this.outOfScopeExportBundleCount,
    required this.candidates,
    required this.partialReasons,
  });

  final AuthoringRevision3DataAssetPackageIndexStatus status;
  final int physicalChunkCount;
  final int winningExportBundleCount;
  final int directoryIndexedExportBundleCount;
  final int outOfScopeExportBundleCount;
  final List<AuthoringRevision3DataAssetPackageCandidate> candidates;
  final List<AuthoringRevision3DataAssetPackagePartialReasonCount>
  partialReasons;

  factory AuthoringRevision3DataAssetPackageIndex._fromCanonicalJson(
    String value,
  ) {
    final json = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 DataAsset package index',
    );
    _dataAssetPackageIndexFields(json, const <String>[
      'status',
      'physical_chunk_count',
      'winning_export_bundle_count',
      'directory_indexed_export_bundle_count',
      'out_of_scope_export_bundle_count',
      'candidates',
      'partial_reasons',
    ], 'revision-3 DataAsset package index');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 DataAsset package index',
    );
    if (jsonEncode(json) != value) {
      throw const FormatException(
        'revision-3 DataAsset package index is not canonical',
      );
    }

    final rawCandidates = json['candidates'];
    if (rawCandidates is! List<Object?> ||
        rawCandidates.length > _maxRevision3DataAssetPackageCandidates) {
      throw const FormatException(
        'revision-3 DataAsset package candidate list is not bounded',
      );
    }
    final candidates = <AuthoringRevision3DataAssetPackageCandidate>[];
    String? previousTarget;
    for (var index = 0; index < rawCandidates.length; index++) {
      final candidate = AuthoringRevision3DataAssetPackageCandidate._fromJson(
        rawCandidates[index],
        index,
      );
      if (previousTarget != null &&
          previousTarget.compareTo(candidate.targetPath) >= 0) {
        throw const FormatException(
          'revision-3 DataAsset package candidates are not sorted and unique',
        );
      }
      previousTarget = candidate.targetPath;
      candidates.add(candidate);
    }

    final rawReasons = json['partial_reasons'];
    if (rawReasons is! List<Object?> || rawReasons.length > 4) {
      throw const FormatException(
        'revision-3 DataAsset package partial reasons are not bounded',
      );
    }
    final reasons = <AuthoringRevision3DataAssetPackagePartialReasonCount>[];
    var previousReason = -1;
    for (var index = 0; index < rawReasons.length; index++) {
      final reason =
          AuthoringRevision3DataAssetPackagePartialReasonCount._fromJson(
            rawReasons[index],
            index,
          );
      final order = reason.reason.index;
      if (order <= previousReason) {
        throw const FormatException(
          'revision-3 DataAsset package partial reasons are not sorted and unique',
        );
      }
      previousReason = order;
      reasons.add(reason);
    }

    final status = switch (json['status']) {
      'complete_index' =>
        AuthoringRevision3DataAssetPackageIndexStatus.completeIndex,
      'partial_index' =>
        AuthoringRevision3DataAssetPackageIndexStatus.partialIndex,
      _ => throw const FormatException(
        'revision-3 DataAsset package index has an unsupported status',
      ),
    };
    if ((status ==
            AuthoringRevision3DataAssetPackageIndexStatus.completeIndex) !=
        reasons.isEmpty) {
      throw const FormatException(
        'revision-3 DataAsset package status disagrees with its partial reasons',
      );
    }

    final physical = _dataAssetPackageIndexInt(
      json['physical_chunk_count'],
      'revision-3 DataAsset physical chunk count',
      max: _maxRevision3DataAssetPhysicalChunks,
    );
    final winning = _dataAssetPackageIndexInt(
      json['winning_export_bundle_count'],
      'revision-3 DataAsset winning ExportBundle count',
      max: _maxRevision3DataAssetPackageCandidates,
    );
    final directoryIndexed = _dataAssetPackageIndexInt(
      json['directory_indexed_export_bundle_count'],
      'revision-3 DataAsset directory-indexed ExportBundle count',
      max: _maxRevision3DataAssetPackageCandidates,
    );
    final outOfScope = _dataAssetPackageIndexInt(
      json['out_of_scope_export_bundle_count'],
      'revision-3 DataAsset out-of-scope ExportBundle count',
      max: _maxRevision3DataAssetPackageCandidates,
    );
    final reasonCounts = <AuthoringRevision3DataAssetPackagePartialReason, int>{
      for (final reason in reasons) reason.reason: reason.count,
    };
    final noncanonicalChunk =
        reasonCounts[AuthoringRevision3DataAssetPackagePartialReason
            .noncanonicalExportBundleChunkId] ??
        0;
    final missingPath =
        reasonCounts[AuthoringRevision3DataAssetPackagePartialReason
            .missingDirectoryIndexPath] ??
        0;
    final noncanonicalPath =
        reasonCounts[AuthoringRevision3DataAssetPackagePartialReason
            .noncanonicalGameDirectoryIndexPath] ??
        0;
    final packageIdMismatch =
        reasonCounts[AuthoringRevision3DataAssetPackagePartialReason
            .packageIdMismatch] ??
        0;
    if (physical < winning ||
        winning != noncanonicalChunk + missingPath + directoryIndexed ||
        directoryIndexed !=
            outOfScope +
                noncanonicalPath +
                packageIdMismatch +
                candidates.length) {
      throw const FormatException(
        'revision-3 DataAsset package counters are inconsistent',
      );
    }

    return AuthoringRevision3DataAssetPackageIndex._(
      status: status,
      physicalChunkCount: physical,
      winningExportBundleCount: winning,
      directoryIndexedExportBundleCount: directoryIndexed,
      outOfScopeExportBundleCount: outOfScope,
      candidates: List.unmodifiable(candidates),
      partialReasons: List.unmodifiable(reasons),
    );
  }
}

final class AuthoringRevision3DataAssetPackageIndexResult {
  const AuthoringRevision3DataAssetPackageIndexResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.packageIndexJson,
    required this.index,
    required this.candidateCount,
    required this.targetExecutableSeal,
    required this.mountInventoryEntryCount,
    required this.mountInventorySeal,
    required this.packageIndexSeal,
    required this.sourceSnapshotSeal,
    required this.scope,
    required this.contentStatus,
    required this.exportBundlePayloadStatus,
    required this.mutationStatus,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
    required this.authorityStatus,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String packageIndexJson;
  final AuthoringRevision3DataAssetPackageIndex index;
  final int candidateCount;
  final AuthoringDraftContentSeal targetExecutableSeal;
  final int mountInventoryEntryCount;
  final AuthoringDraftContentSeal mountInventorySeal;
  final AuthoringDraftContentSeal packageIndexSeal;
  final AuthoringDraftContentSeal sourceSnapshotSeal;
  final AuthoringRevision3DataAssetPackageIndexScope scope;
  final AuthoringRevision3DataAssetPackageContentStatus contentStatus;
  final AuthoringRevision3DataAssetExportBundlePayloadStatus
  exportBundlePayloadStatus;
  final AuthoringRevision3DataAssetPackageMutationStatus mutationStatus;
  final AuthoringRevision3DataAssetPackageBuildStatus buildStatus;
  final AuthoringRevision3DataAssetPackageRuntimeStatus runtimeStatus;
  final AuthoringRevision3DataAssetPackagePublicationStatus publicationStatus;
  final AuthoringRevision3DataAssetPackageAuthorityStatus authorityStatus;

  factory AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
  }) {
    _dataAssetPackageIndexFields(json, const <String>[
      'authority_status',
      'build_status',
      'candidate_count',
      'content_status',
      'export_bundle_payload_status',
      'head_json',
      'mount_inventory_entry_count',
      'mount_inventory_seal',
      'mutation_status',
      'ok',
      'outcome',
      'package_index_json',
      'package_index_seal',
      'package_index_status',
      'project_id',
      'project_revision',
      'publication_status',
      'runtime_status',
      'scope',
      'source_snapshot_seal',
      'target_executable_seal',
    ], 'revision-3 DataAsset package-index response');
    if (json['ok'] != true || json['outcome'] != 'audit_only') {
      throw const FormatException(
        'revision-3 DataAsset package-index response is not an audit',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _dataAssetPackageIndexString(
        json['head_json'],
        'revision-3 DataAsset package-index head',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException(
        'revision-3 DataAsset package-index response changed its exact head',
      );
    }
    final projectId = _authoringEntityId(
      _dataAssetPackageIndexString(
        json['project_id'],
        'revision-3 DataAsset package-index project ID',
        maxBytes: 32,
      ),
      'project_id',
    );
    if (projectId == '00000000000000000000000000000000') {
      throw const FormatException(
        'revision-3 DataAsset package-index project ID must not be zero',
      );
    }
    final projectRevision = _dataAssetPackageIndexInt(
      json['project_revision'],
      'revision-3 DataAsset package-index project revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final packageIndexJson = _dataAssetPackageIndexString(
      json['package_index_json'],
      'revision-3 DataAsset package index JSON',
      maxBytes: _maxAuthoringRevision3DataAssetPackageIndexJsonBytes,
    );
    final index = AuthoringRevision3DataAssetPackageIndex._fromCanonicalJson(
      packageIndexJson,
    );
    final candidateCount = _dataAssetPackageIndexInt(
      json['candidate_count'],
      'revision-3 DataAsset package candidate count',
      max: _maxRevision3DataAssetPackageCandidates,
    );
    if (candidateCount != index.candidates.length ||
        json['package_index_status'] !=
            switch (index.status) {
              AuthoringRevision3DataAssetPackageIndexStatus.completeIndex =>
                'complete_index',
              AuthoringRevision3DataAssetPackageIndexStatus.partialIndex =>
                'partial_index',
            }) {
      throw const FormatException(
        'revision-3 DataAsset package-index summary disagrees with its index',
      );
    }

    final targetExecutableSeal = _dataAssetPackageIndexSeal(
      json['target_executable_seal'],
      'revision-3 DataAsset target executable',
    );
    final mountInventorySeal = _dataAssetPackageIndexSeal(
      json['mount_inventory_seal'],
      'revision-3 DataAsset mount inventory',
    );
    final packageIndexSeal = _dataAssetPackageIndexSeal(
      json['package_index_seal'],
      'revision-3 DataAsset package index',
    );
    final sourceSnapshotSeal = _dataAssetPackageIndexSeal(
      json['source_snapshot_seal'],
      'revision-3 DataAsset source snapshot',
    );
    final indexBytes = utf8.encode(packageIndexJson);
    if (packageIndexSeal.byteLength != indexBytes.length ||
        packageIndexSeal.sha256 !=
            crypto.sha256.convert(indexBytes).toString()) {
      throw const FormatException(
        'revision-3 DataAsset package index seal is invalid',
      );
    }

    return AuthoringRevision3DataAssetPackageIndexResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      packageIndexJson: packageIndexJson,
      index: index,
      candidateCount: candidateCount,
      targetExecutableSeal: targetExecutableSeal,
      mountInventoryEntryCount: _dataAssetPackageIndexInt(
        json['mount_inventory_entry_count'],
        'revision-3 DataAsset mount inventory entry count',
        min: 1,
        max: _maxRevision3DataAssetMountInventoryEntries,
      ),
      mountInventorySeal: mountInventorySeal,
      packageIndexSeal: packageIndexSeal,
      sourceSnapshotSeal: sourceSnapshotSeal,
      scope: switch (json['scope']) {
        'installed_dataasset_package_candidates_only' =>
          AuthoringRevision3DataAssetPackageIndexScope
              .installedDataAssetPackageCandidatesOnly,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response has an unsupported scope',
        ),
      },
      contentStatus: switch (json['content_status']) {
        'metadata_candidates_only' =>
          AuthoringRevision3DataAssetPackageContentStatus
              .metadataCandidatesOnly,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants unsupported content authority',
        ),
      },
      exportBundlePayloadStatus: switch (json['export_bundle_payload_status']) {
        'not_read' =>
          AuthoringRevision3DataAssetExportBundlePayloadStatus.notRead,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response claims export payload access',
        ),
      },
      mutationStatus: switch (json['mutation_status']) {
        'not_supported' =>
          AuthoringRevision3DataAssetPackageMutationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants mutation authority',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' =>
          AuthoringRevision3DataAssetPackageBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DataAssetPackageRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DataAssetPackagePublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants publication authority',
        ),
      },
      authorityStatus: switch (json['authority_status']) {
        'not_granted' =>
          AuthoringRevision3DataAssetPackageAuthorityStatus.notGranted,
        _ => throw const FormatException(
          'revision-3 DataAsset package-index response grants unsupported authority',
        ),
      },
    );
  }

  /// Compare the native installed executable seal with the exact target in one
  /// already-canonical revision-3 project document.
  bool matchesCanonicalProjectTarget(String projectJson) {
    final project = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    final target = _authoringRequiredObject(
      project.project['target'],
      'revision-3 DataAsset package-index project target',
    );
    _dataAssetPackageIndexFields(target, const <String>[
      'executable',
    ], 'revision-3 DataAsset package-index project target');
    final executable = _dataAssetPackageIndexSeal(
      target['executable'],
      'revision-3 DataAsset package-index project executable',
    );
    return executable.byteLength == targetExecutableSeal.byteLength &&
        executable.sha256 == targetExecutableSeal.sha256;
  }
}

void _dataAssetPackageIndexFields(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  _authoringExactFields(json, expected.toSet(), context);
  final actual = json.keys.toList(growable: false);
  for (var index = 0; index < expected.length; index++) {
    if (actual[index] != expected[index]) {
      throw FormatException('$context has non-canonical field order');
    }
  }
}

String _dataAssetPackageIndexString(
  Object? value,
  String context, {
  required int maxBytes,
}) {
  if (value is! String) throw FormatException('$context is not a string');
  try {
    _authoringRevision3RequestString(value, context, maxBytes);
  } on ArgumentError {
    throw FormatException('$context is not bounded UTF-8');
  }
  return value;
}

int _dataAssetPackageIndexInt(
  Object? value,
  String context, {
  int min = 0,
  required int max,
}) {
  if (value is! int || value < min || value > max) {
    throw FormatException('$context is outside its signed wire bounds');
  }
  return value;
}

AuthoringDraftContentSeal _dataAssetPackageIndexSeal(
  Object? value,
  String context,
) {
  final json = _authoringRequiredObject(value, '$context seal');
  _dataAssetPackageIndexFields(json, const <String>[
    'byte_len',
    'sha256',
  ], '$context seal');
  final byteLength = _dataAssetPackageIndexInt(
    json['byte_len'],
    '$context seal byte length',
    min: 1,
    max: _maxAuthoringSignedJsonInteger,
  );
  final sha256 = _dataAssetPackageIndexString(
    json['sha256'],
    '$context seal SHA-256',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sha256)) {
    throw FormatException('$context seal SHA-256 is not canonical');
  }
  return AuthoringDraftContentSeal._(byteLength: byteLength, sha256: sha256);
}
