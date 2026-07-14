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

    return AuthoringRevision3InstalledDataAssetInspectionResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      candidateOrdinal: ordinal,
      targetPath: targetPath,
      packageIdHex: packageIdHex,
      packageIndexSeal: packageIndexSeal,
      sourceSnapshotSeal: sourceSnapshotSeal,
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

bool _installedDataAssetSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

Map<String, Object?> _installedDataAssetSealJson(
  AuthoringDraftContentSeal seal,
) => <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};
