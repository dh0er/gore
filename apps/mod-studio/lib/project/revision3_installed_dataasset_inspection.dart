part of '../core/mod_ffi.dart';

enum AuthoringRevision3InstalledDataAssetInspectionScope {
  selectedInstalledDataAssetFixedLeafInspectionOnly,
}

enum AuthoringRevision3InstalledDataAssetMutationStatus { notSupported }

enum AuthoringRevision3InstalledDataAssetBuildStatus { notEvaluated }

enum AuthoringRevision3InstalledDataAssetRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3InstalledDataAssetPublicationStatus { notSupported }

enum AuthoringRevision3InstalledDataAssetAuthorityStatus { notGranted }

/// Read-only fixed-leaf inspection of one server-selected candidate from an
/// exact installed package snapshot.
///
/// The original snapshot ordinal is the only selection crossing the native
/// boundary. The returned target path is display evidence checked against that
/// snapshot; it is never accepted as caller-supplied extraction authority.
final class AuthoringRevision3InstalledDataAssetInspectionResult {
  const AuthoringRevision3InstalledDataAssetInspectionResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.candidateOrdinal,
    required this.targetPath,
    required this.packageIdHex,
    required this.packageIndexSeal,
    required this.sourceSnapshotSeal,
    required this.usmapContentSeal,
    required this.usmapInventorySeal,
    required this.inspection,
    required this.scope,
    required this.mutationStatus,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
    required this.authorityStatus,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final int candidateOrdinal;
  final String targetPath;
  final String packageIdHex;
  final AuthoringDraftContentSeal packageIndexSeal;
  final AuthoringDraftContentSeal sourceSnapshotSeal;
  final AuthoringDraftContentSeal usmapContentSeal;
  final AuthoringDraftContentSeal usmapInventorySeal;
  final DataAssetInspection inspection;
  final AuthoringRevision3InstalledDataAssetInspectionScope scope;
  final AuthoringRevision3InstalledDataAssetMutationStatus mutationStatus;
  final AuthoringRevision3InstalledDataAssetBuildStatus buildStatus;
  final AuthoringRevision3InstalledDataAssetRuntimeStatus runtimeStatus;
  final AuthoringRevision3InstalledDataAssetPublicationStatus publicationStatus;
  final AuthoringRevision3InstalledDataAssetAuthorityStatus authorityStatus;

  factory AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required int requestedOrdinal,
  }) {
    if (requestedOrdinal < 0 ||
        requestedOrdinal >= expectedSnapshot.index.candidates.length) {
      throw const FormatException(
        'installed DataAsset inspection ordinal is outside its exact snapshot',
      );
    }
    final expectedCandidate =
        expectedSnapshot.index.candidates[requestedOrdinal];
    if (expectedCandidate.ordinal != requestedOrdinal) {
      throw const FormatException(
        'installed DataAsset package snapshot lost its original ordinal',
      );
    }
    _dataAssetPackageIndexFields(json, const <String>[
      'authority_status',
      'build_status',
      'candidate_ordinal',
      'head_json',
      'inspection',
      'mutation_status',
      'ok',
      'outcome',
      'package_id_hex',
      'package_index_seal',
      'project_id',
      'project_revision',
      'publication_status',
      'runtime_status',
      'scope',
      'source_snapshot_seal',
      'target_path',
      'usmap_content_seal',
      'usmap_inventory_seal',
    ], 'revision-3 installed DataAsset inspection response');
    if (json['ok'] != true || json['outcome'] != 'inspection_only') {
      throw const FormatException(
        'revision-3 installed DataAsset response is not read-only inspection evidence',
      );
    }

    final head = AuthoringWorkingHead.fromCanonicalJson(
      _dataAssetPackageIndexString(
        json['head_json'],
        'revision-3 installed DataAsset inspection head',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedSnapshot.head.canonicalJson) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection changed its exact head',
      );
    }
    final ordinal = _dataAssetPackageIndexInt(
      json['candidate_ordinal'],
      'revision-3 installed DataAsset candidate ordinal',
      max: _maxRevision3DataAssetPackageCandidates - 1,
    );
    final targetPath = _dataAssetPackageIndexString(
      json['target_path'],
      'revision-3 installed DataAsset target path',
      maxBytes: 512,
    );
    final packageIdHex = _dataAssetPackageIndexString(
      json['package_id_hex'],
      'revision-3 installed DataAsset package ID',
      maxBytes: 16,
    );
    if (ordinal != requestedOrdinal ||
        targetPath != expectedCandidate.targetPath ||
        packageIdHex != expectedCandidate.packageIdHex) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection selected a different package candidate',
      );
    }

    final packageIndexSeal = _dataAssetPackageIndexSeal(
      json['package_index_seal'],
      'revision-3 installed DataAsset package index',
    );
    final sourceSnapshotSeal = _dataAssetPackageIndexSeal(
      json['source_snapshot_seal'],
      'revision-3 installed DataAsset source snapshot',
    );
    final usmapContentSeal = _dataAssetPackageIndexSeal(
      json['usmap_content_seal'],
      'revision-3 installed DataAsset USMAP content',
    );
    final usmapInventorySeal = _dataAssetPackageIndexSeal(
      json['usmap_inventory_seal'],
      'revision-3 installed DataAsset USMAP inventory',
    );
    if (!_installedDataAssetSameSeal(
          packageIndexSeal,
          expectedSnapshot.packageIndexSeal,
        ) ||
        !_installedDataAssetSameSeal(
          sourceSnapshotSeal,
          expectedSnapshot.sourceSnapshotSeal,
        )) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection changed its sealed package snapshot',
      );
    }
    final projectId = _authoringEntityId(
      _dataAssetPackageIndexString(
        json['project_id'],
        'revision-3 installed DataAsset project ID',
        maxBytes: 32,
      ),
      'project_id',
    );
    final projectRevision = _dataAssetPackageIndexInt(
      json['project_revision'],
      'revision-3 installed DataAsset project revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    if (projectId != expectedSnapshot.projectId ||
        projectRevision != expectedSnapshot.projectRevision) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection changed its exact project identity',
      );
    }
    final inspectionJson = json['inspection'];
    if (inspectionJson is! Map<String, Object?>) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection payload is not an object',
      );
    }
    final inspection = DataAssetInspection.fromJson(inspectionJson);
    if (inspection.selection.exportIndex != null) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection omitted package exports',
      );
    }
    if (usmapContentSeal.byteLength != inspection.input.usmapLength ||
        usmapContentSeal.sha256 != inspection.binding.usmapSha256) {
      throw const FormatException(
        'revision-3 installed DataAsset inspection changed its sealed USMAP content',
      );
    }

    return AuthoringRevision3InstalledDataAssetInspectionResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      candidateOrdinal: ordinal,
      targetPath: targetPath,
      packageIdHex: packageIdHex,
      packageIndexSeal: packageIndexSeal,
      sourceSnapshotSeal: sourceSnapshotSeal,
      usmapContentSeal: usmapContentSeal,
      usmapInventorySeal: usmapInventorySeal,
      inspection: inspection,
      scope: switch (json['scope']) {
        'selected_installed_dataasset_fixed_leaf_inspection_only' =>
          AuthoringRevision3InstalledDataAssetInspectionScope
              .selectedInstalledDataAssetFixedLeafInspectionOnly,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection has unsupported scope',
        ),
      },
      mutationStatus: switch (json['mutation_status']) {
        'not_supported' =>
          AuthoringRevision3InstalledDataAssetMutationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection grants mutation authority',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' =>
          AuthoringRevision3InstalledDataAssetBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3InstalledDataAssetRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection grants runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3InstalledDataAssetPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection grants publication authority',
        ),
      },
      authorityStatus: switch (json['authority_status']) {
        'not_granted' =>
          AuthoringRevision3InstalledDataAssetAuthorityStatus.notGranted,
        _ => throw const FormatException(
          'revision-3 installed DataAsset inspection grants unsupported authority',
        ),
      },
    );
  }
}

abstract interface class _InstalledDataAssetProofExpectation {
  AuthoringRevision3DataAssetPackageIndexResult get snapshot;
  AuthoringRevision3DataAssetPackageCandidate get candidate;
  AuthoringRevision3InstalledDataAssetInspectionResult get inspection;
  String get installedProofBindingSha256;
}

/// A typed fixed-leaf value change bound to one exact installed package
/// snapshot and its exact read-only inspection evidence.
///
/// Native selection remains ordinal-only. Target paths, package IDs, raw
/// package bytes, offsets, receipts, and output paths never cross this wire as
/// caller authority.
final class DataAssetInstalledSemanticEditIntent
    implements _InstalledDataAssetProofExpectation {
  DataAssetInstalledSemanticEditIntent._({
    required this.snapshot,
    required this.candidate,
    required this.inspection,
    required this.change,
  });

  @override
  final AuthoringRevision3DataAssetPackageIndexResult snapshot;
  @override
  final AuthoringRevision3DataAssetPackageCandidate candidate;
  @override
  final AuthoringRevision3InstalledDataAssetInspectionResult inspection;
  final DataAssetSemanticValueChange change;

  FixedLeafSelector get selector => change.selector;
  DataAssetSemanticReplacement get replacement => change.replacement;
  String get expectedTargetPath => inspection.targetPath;

  factory DataAssetInstalledSemanticEditIntent.fromInspection({
    required AuthoringRevision3DataAssetPackageIndexResult snapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
    required AuthoringRevision3InstalledDataAssetInspectionResult inspection,
    required DataAssetSemanticValueChange change,
  }) {
    final ownsSelector = inspection.inspection.exports.any(
      (export) => export.leaves.any(
        (leaf) => leaf.editable && identical(leaf.selector, change.selector),
      ),
    );
    if (!_installedDataAssetIntentIdentityMatches(
          snapshot: snapshot,
          candidate: candidate,
          inspection: inspection,
        ) ||
        !ownsSelector) {
      throw ArgumentError(
        'installed DataAsset edit must use one exact snapshot, candidate, inspection, and editable selector',
        'inspection',
      );
    }
    return DataAssetInstalledSemanticEditIntent._(
      snapshot: snapshot,
      candidate: candidate,
      inspection: inspection,
      change: change,
    );
  }

  Map<String, Object?> toNativeFields() => <String, Object?>{
    'candidate_ordinal': candidate.ordinal,
    'expected_inspection_binding': _inspectionBindingJson,
    'expected_package_index_seal': _installedDataAssetSealJson(
      snapshot.packageIndexSeal,
    ),
    'expected_source_snapshot_seal': _installedDataAssetSealJson(
      snapshot.sourceSnapshotSeal,
    ),
    'expected_usmap_content_seal': _installedDataAssetSealJson(
      inspection.usmapContentSeal,
    ),
    'expected_usmap_inventory_seal': _installedDataAssetSealJson(
      inspection.usmapInventorySeal,
    ),
    'replacement': replacement.toJson(),
    'selector': selector.toJson(),
  };

  Map<String, Object?> get _inspectionBindingJson => <String, Object?>{
    'uasset': <String, Object?>{
      'byte_len': inspection.inspection.input.uassetLength,
      'sha256': inspection.inspection.binding.packageSeal.uassetSha256,
    },
    'uexp': <String, Object?>{
      'byte_len': inspection.inspection.input.uexpLength,
      'sha256': inspection.inspection.binding.packageSeal.uexpSha256,
    },
    'usmap': <String, Object?>{
      'byte_len': inspection.inspection.input.usmapLength,
      'sha256': inspection.inspection.binding.usmapSha256,
    },
  };

  String get intentBindingSha256 => replacement.intentBindingSha256For(
    expectedTargetPath: expectedTargetPath,
    selector: selector,
  );

  @override
  String get installedProofBindingSha256 => computeInstalledProofBindingSha256(
    candidateOrdinal: candidate.ordinal,
    packageIndex: (
      byteLength: snapshot.packageIndexSeal.byteLength,
      sha256: snapshot.packageIndexSeal.sha256,
    ),
    sourceSnapshot: (
      byteLength: snapshot.sourceSnapshotSeal.byteLength,
      sha256: snapshot.sourceSnapshotSeal.sha256,
    ),
    usmapContent: (
      byteLength: inspection.usmapContentSeal.byteLength,
      sha256: inspection.usmapContentSeal.sha256,
    ),
    usmapInventory: (
      byteLength: inspection.usmapInventorySeal.byteLength,
      sha256: inspection.usmapInventorySeal.sha256,
    ),
    uasset: (
      byteLength: inspection.inspection.input.uassetLength,
      sha256: inspection.inspection.binding.packageSeal.uassetSha256,
    ),
    uexp: (
      byteLength: inspection.inspection.input.uexpLength,
      sha256: inspection.inspection.binding.packageSeal.uexpSha256,
    ),
    usmap: (
      byteLength: inspection.inspection.input.usmapLength,
      sha256: inspection.inspection.binding.usmapSha256,
    ),
  );

  /// Frozen byte contract shared with native tests. Named facts make the
  /// security-significant order explicit while raw package bytes stay out of
  /// Dart and the FFI request.
  static String computeInstalledProofBindingSha256({
    required int candidateOrdinal,
    required ({int byteLength, String sha256}) packageIndex,
    required ({int byteLength, String sha256}) sourceSnapshot,
    required ({int byteLength, String sha256}) usmapContent,
    required ({int byteLength, String sha256}) usmapInventory,
    required ({int byteLength, String sha256}) uasset,
    required ({int byteLength, String sha256}) uexp,
    required ({int byteLength, String sha256}) usmap,
  }) {
    if (candidateOrdinal < 0 ||
        candidateOrdinal > _maxAuthoringSignedJsonInteger) {
      throw const FormatException(
        'installed DataAsset proof ordinal is outside its signed wire bounds',
      );
    }
    final bytes = BytesBuilder(copy: false)
      ..add(
        utf8.encode(
          'gore.authoring.r3-installed-dataasset-proof-binding.v1\u0000',
        ),
      );
    final ordinal = ByteData(8)..setUint64(0, candidateOrdinal, Endian.little);
    bytes.add(ordinal.buffer.asUint8List());
    for (final seal in <({int byteLength, String sha256})>[
      packageIndex,
      sourceSnapshot,
      usmapContent,
      usmapInventory,
      uasset,
      uexp,
      usmap,
    ]) {
      if (seal.byteLength <= 0 ||
          seal.byteLength > _maxAuthoringSignedJsonInteger) {
        throw const FormatException(
          'installed DataAsset proof seal length is outside its signed wire bounds',
        );
      }
      final length = ByteData(8)..setUint64(0, seal.byteLength, Endian.little);
      bytes
        ..add(length.buffer.asUint8List())
        ..add(_installedDataAssetSha256Bytes(seal.sha256));
    }
    return crypto.sha256.convert(bytes.takeBytes()).toString();
  }
}

/// A reviewed semantic edit bound to one exact installed inspection.
///
/// The narrow request carries no target, package identity, inspector binding,
/// selector, or encoded replacement bytes. Native code resolves those facts
/// again from the retained ordinal and sealed snapshot.
final class ReviewedInstalledDataAssetEditIntent
    implements _InstalledDataAssetProofExpectation {
  ReviewedInstalledDataAssetEditIntent._({
    required this.snapshot,
    required this.candidate,
    required this.inspection,
    required this.request,
    required this.evidence,
  });

  @override
  final AuthoringRevision3DataAssetPackageIndexResult snapshot;
  @override
  final AuthoringRevision3DataAssetPackageCandidate candidate;
  @override
  final AuthoringRevision3InstalledDataAssetInspectionResult inspection;
  final ReviewedDataAssetEditRequest request;
  final ReviewedFootstepPresetInspection evidence;

  String get expectedTargetPath => inspection.targetPath;

  /// Reconstruct the ordinary fixed-leaf stage binding locally without
  /// granting selector or replacement-byte authority to the request wire.
  /// This makes a natively self-consistent but semantically different stage
  /// fail closed at the Dart boundary.
  String get expectedStageIntentBindingSha256 {
    final change = DataAssetSemanticValueEditor.fromLeaf(evidence.leaf)
        .changeComponents(
          values: <String>[
            request.x,
            request.y,
            evidence.currentZ,
            evidence.currentW,
          ],
        );
    return change.replacement.intentBindingSha256For(
      expectedTargetPath: expectedTargetPath,
      selector: evidence.leaf.selector,
    );
  }

  /// Recompute the native reviewed-schema binding over the complete exact
  /// selector and replacement. This is independent of the ordinary stage
  /// binding and prevents an arbitrary digest echo from becoming evidence.
  String get expectedReviewedIntentBindingSha256 {
    final replacement = ByteData(32)
      ..setFloat64(0, double.parse(request.x), Endian.little)
      ..setFloat64(8, double.parse(request.y), Endian.little)
      ..setFloat64(16, double.parse(evidence.currentZ), Endian.little)
      ..setFloat64(24, double.parse(evidence.currentW), Endian.little);
    final format = ByteData(4)
      ..setUint32(0, reviewedDataAssetEditRequestFormat, Endian.little);
    final revision = ByteData(4)
      ..setUint32(0, footstepPresetSchemaRevision, Endian.little);
    final bytes = BytesBuilder(copy: false)
      ..add(
        utf8.encode(
          'gore-asset.reviewed-dataasset.footstep-preset.feet-texture-size.v1\u0000',
        ),
      );
    for (final value in <List<int>>[
      format.buffer.asUint8List(),
      utf8.encode(footstepPresetSchemaId),
      revision.buffer.asUint8List(),
      utf8.encode(feetTextureSizeFieldId),
      utf8.encode(_reviewedFootstepTargetId(evidence.target)),
      utf8.encode(expectedTargetPath),
      utf8.encode(jsonEncode(evidence.leaf.selector.toJson())),
      replacement.buffer.asUint8List(),
    ]) {
      final length = ByteData(8)..setUint64(0, value.length, Endian.little);
      bytes
        ..add(length.buffer.asUint8List())
        ..add(value);
    }
    return crypto.sha256.convert(bytes.takeBytes()).toString();
  }

  factory ReviewedInstalledDataAssetEditIntent.fromInspection({
    required AuthoringRevision3DataAssetPackageIndexResult snapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
    required AuthoringRevision3InstalledDataAssetInspectionResult inspection,
    required ReviewedDataAssetEditRequest request,
  }) {
    final evidence = ReviewedFootstepPresetInspection.tryMatch(
      packagePath: inspection.targetPath,
      inspection: inspection.inspection,
    );
    if (!_installedDataAssetIntentIdentityMatches(
          snapshot: snapshot,
          candidate: candidate,
          inspection: inspection,
        ) ||
        evidence == null) {
      throw ArgumentError(
        'reviewed installed DataAsset edit must use one exact snapshot, candidate, inspection, and reviewed leaf',
        'inspection',
      );
    }
    return ReviewedInstalledDataAssetEditIntent._(
      snapshot: snapshot,
      candidate: candidate,
      inspection: inspection,
      request: request,
      evidence: evidence,
    );
  }

  Map<String, Object?> toNativeFields() => <String, Object?>{
    'candidate_ordinal': candidate.ordinal,
    'expected_package_index_seal': _installedDataAssetSealJson(
      snapshot.packageIndexSeal,
    ),
    'expected_source_snapshot_seal': _installedDataAssetSealJson(
      snapshot.sourceSnapshotSeal,
    ),
    'reviewed_edit': request.toJson(),
  };

  @override
  String get installedProofBindingSha256 =>
      DataAssetInstalledSemanticEditIntent.computeInstalledProofBindingSha256(
        candidateOrdinal: candidate.ordinal,
        packageIndex: (
          byteLength: snapshot.packageIndexSeal.byteLength,
          sha256: snapshot.packageIndexSeal.sha256,
        ),
        sourceSnapshot: (
          byteLength: snapshot.sourceSnapshotSeal.byteLength,
          sha256: snapshot.sourceSnapshotSeal.sha256,
        ),
        usmapContent: (
          byteLength: inspection.usmapContentSeal.byteLength,
          sha256: inspection.usmapContentSeal.sha256,
        ),
        usmapInventory: (
          byteLength: inspection.usmapInventorySeal.byteLength,
          sha256: inspection.usmapInventorySeal.sha256,
        ),
        uasset: (
          byteLength: inspection.inspection.input.uassetLength,
          sha256: inspection.inspection.binding.packageSeal.uassetSha256,
        ),
        uexp: (
          byteLength: inspection.inspection.input.uexpLength,
          sha256: inspection.inspection.binding.packageSeal.uexpSha256,
        ),
        usmap: (
          byteLength: inspection.inspection.input.usmapLength,
          sha256: inspection.inspection.binding.usmapSha256,
        ),
      );
}

bool _installedDataAssetIntentIdentityMatches({
  required AuthoringRevision3DataAssetPackageIndexResult snapshot,
  required AuthoringRevision3DataAssetPackageCandidate candidate,
  required AuthoringRevision3InstalledDataAssetInspectionResult inspection,
}) {
  final ordinal = candidate.ordinal;
  if (ordinal < 0 || ordinal >= snapshot.index.candidates.length) return false;
  return identical(candidate, snapshot.index.candidates[ordinal]) &&
      inspection.head.canonicalJson == snapshot.head.canonicalJson &&
      inspection.projectId == snapshot.projectId &&
      inspection.projectRevision == snapshot.projectRevision &&
      inspection.candidateOrdinal == ordinal &&
      inspection.targetPath == candidate.targetPath &&
      inspection.packageIdHex == candidate.packageIdHex &&
      _installedDataAssetSameSeal(
        inspection.packageIndexSeal,
        snapshot.packageIndexSeal,
      ) &&
      _installedDataAssetSameSeal(
        inspection.sourceSnapshotSeal,
        snapshot.sourceSnapshotSeal,
      );
}

Uint8List _installedDataAssetSha256Bytes(String value) {
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw const FormatException('installed DataAsset proof SHA-256 is invalid');
  }
  final result = Uint8List(32);
  for (var index = 0; index < result.length; index++) {
    result[index] = int.parse(
      value.substring(index * 2, index * 2 + 2),
      radix: 16,
    );
  }
  return result;
}

/// Strict path-free echo of the installed evidence native code retained and
/// revalidated through preparation.
final class AuthoringRevision3InstalledDataAssetSourceProof {
  const AuthoringRevision3InstalledDataAssetSourceProof._({
    required this.candidateOrdinal,
    required this.packageIndexSeal,
    required this.sourceSnapshotSeal,
    required this.usmapContentSeal,
    required this.usmapInventorySeal,
  });

  final int candidateOrdinal;
  final AuthoringDraftContentSeal packageIndexSeal;
  final AuthoringDraftContentSeal sourceSnapshotSeal;
  final AuthoringDraftContentSeal usmapContentSeal;
  final AuthoringDraftContentSeal usmapInventorySeal;

  factory AuthoringRevision3InstalledDataAssetSourceProof._fromJson(
    Object? value, {
    required _InstalledDataAssetProofExpectation expected,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 installed DataAsset source proof',
    );
    _dataAssetPackageIndexFields(json, const <String>[
      'candidate_ordinal',
      'format',
      'inspection_binding',
      'package_index_seal',
      'source_snapshot_seal',
      'usmap_content_seal',
      'usmap_inventory_seal',
    ], 'revision-3 installed DataAsset source proof');
    if (json['format'] !=
        'gore.authoring.revision3-installed-dataasset-source.v1') {
      throw const FormatException(
        'revision-3 installed DataAsset source proof has an unsupported format',
      );
    }
    final candidateOrdinal = _dataAssetPackageIndexInt(
      json['candidate_ordinal'],
      'revision-3 installed DataAsset source proof ordinal',
      max: _maxRevision3DataAssetPackageCandidates - 1,
    );
    final packageIndexSeal = _dataAssetPackageIndexSeal(
      json['package_index_seal'],
      'revision-3 installed DataAsset source proof package index',
    );
    final sourceSnapshotSeal = _dataAssetPackageIndexSeal(
      json['source_snapshot_seal'],
      'revision-3 installed DataAsset source proof source snapshot',
    );
    final usmapContentSeal = _dataAssetPackageIndexSeal(
      json['usmap_content_seal'],
      'revision-3 installed DataAsset source proof USMAP content',
    );
    final usmapInventorySeal = _dataAssetPackageIndexSeal(
      json['usmap_inventory_seal'],
      'revision-3 installed DataAsset source proof USMAP inventory',
    );
    final actualBinding = _authoringRequiredObject(
      json['inspection_binding'],
      'revision-3 installed DataAsset source proof inspection binding',
    );
    _dataAssetPackageIndexFields(
      actualBinding,
      const <String>['uasset', 'uexp', 'usmap'],
      'revision-3 installed DataAsset source proof inspection binding',
    );
    final uasset = _dataAssetPackageIndexSeal(
      actualBinding['uasset'],
      'revision-3 installed DataAsset source proof UASSET',
    );
    final uexp = _dataAssetPackageIndexSeal(
      actualBinding['uexp'],
      'revision-3 installed DataAsset source proof UEXP',
    );
    final usmap = _dataAssetPackageIndexSeal(
      actualBinding['usmap'],
      'revision-3 installed DataAsset source proof USMAP',
    );
    final inspected = expected.inspection.inspection;
    if (candidateOrdinal != expected.candidate.ordinal ||
        !_installedDataAssetSameSeal(
          packageIndexSeal,
          expected.snapshot.packageIndexSeal,
        ) ||
        !_installedDataAssetSameSeal(
          sourceSnapshotSeal,
          expected.snapshot.sourceSnapshotSeal,
        ) ||
        !_installedDataAssetSameSeal(
          usmapContentSeal,
          expected.inspection.usmapContentSeal,
        ) ||
        !_installedDataAssetSameSeal(
          usmapInventorySeal,
          expected.inspection.usmapInventorySeal,
        ) ||
        uasset.byteLength != inspected.input.uassetLength ||
        uasset.sha256 != inspected.binding.packageSeal.uassetSha256 ||
        uexp.byteLength != inspected.input.uexpLength ||
        uexp.sha256 != inspected.binding.packageSeal.uexpSha256 ||
        usmap.byteLength != inspected.input.usmapLength ||
        usmap.sha256 != inspected.binding.usmapSha256) {
      throw const FormatException(
        'revision-3 installed DataAsset source proof changed its exact evidence',
      );
    }
    return AuthoringRevision3InstalledDataAssetSourceProof._(
      candidateOrdinal: candidateOrdinal,
      packageIndexSeal: packageIndexSeal,
      sourceSnapshotSeal: sourceSnapshotSeal,
      usmapContentSeal: usmapContentSeal,
      usmapInventorySeal: usmapInventorySeal,
    );
  }
}

bool _installedDataAssetSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

Map<String, Object?> _installedDataAssetSealJson(
  AuthoringDraftContentSeal seal,
) => <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};
