part of '../core/mod_ffi.dart';

const _revision3DataAssetManifestFormat = 'gore.dataasset.fixed-leaf-stage.v1';
const _revision3DataAssetManifestMediaType =
    'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1';
const _revision3DataAssetComponentMediaType =
    'application/vnd.gore.dataasset-fixed-leaf-component;version=1';

enum AuthoringRevision3DataAssetBuildStatus { blocked }

enum AuthoringRevision3DataAssetRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3DataAssetArtifactAuthority { notGranted }

enum AuthoringRevision3DataAssetNativePublicationStatus { notSupported }

/// One exact raw-content seal from a closed DataAsset stage manifest.
final class AuthoringRevision3DataAssetContentSeal {
  const AuthoringRevision3DataAssetContentSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  static AuthoringRevision3DataAssetContentSeal _parse(
    Object? raw,
    String context, {
    int maxByteLength = _maxAuthoringSignedJsonInteger,
  }) {
    final json = _dataAssetObject(raw, context);
    _authoringExactFields(json, const <String>{'byte_len', 'sha256'}, context);
    return AuthoringRevision3DataAssetContentSeal._(
      byteLength: _dataAssetInt(
        json,
        'byte_len',
        context,
        min: 1,
        max: maxByteLength,
      ),
      sha256: _dataAssetSha256(json, 'sha256', context),
    );
  }

  bool _same(AuthoringRevision3DataAssetContentSeal other) =>
      byteLength == other.byteLength && sha256 == other.sha256;

  Map<String, Object?> _storageJson() => <String, Object?>{
    'byte_len': byteLength,
    'sha256': sha256,
  };
}

/// Closed, offset-free projection of a verified fixed-leaf DataAsset stage.
///
/// It intentionally exposes no receipt bytes, receipt path, local filesystem path, raw patch
/// offset, deployment target, pack claim, or runtime authority.
final class AuthoringRevision3DataAssetStage {
  const AuthoringRevision3DataAssetStage._({
    required this.manifestAsset,
    required this.projectId,
    required this.projectTargetExecutable,
    required this.basisHead,
    required this.basisProjectRevision,
    required this.stagedProjectRevision,
    required this.targetPath,
    required this.selectorKind,
    required this.selectorPathDepth,
    required this.replacementByteLength,
    required this._intentBindingSha256,
    required this.patchedUasset,
    required this.patchedUexp,
    required this.usmap,
    required this.sidecars,
    required this.generationContainerCount,
    required this.generationChunkCount,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.publicationStatus,
  });

  final AuthoringRevision3DataAssetContentSeal manifestAsset;
  final String projectId;
  final AuthoringRevision3DataAssetContentSeal projectTargetExecutable;
  final AuthoringWorkingHead basisHead;
  final int basisProjectRevision;
  final int stagedProjectRevision;
  final String targetPath;
  final String selectorKind;
  final int selectorPathDepth;
  final int replacementByteLength;
  final String _intentBindingSha256;
  final AuthoringRevision3DataAssetContentSeal patchedUasset;
  final AuthoringRevision3DataAssetContentSeal patchedUexp;
  final AuthoringRevision3DataAssetContentSeal usmap;
  final Map<String, AuthoringRevision3DataAssetContentSeal> sidecars;
  final int generationContainerCount;
  final int generationChunkCount;
  final AuthoringRevision3DataAssetBuildStatus buildStatus;
  final AuthoringRevision3DataAssetRuntimeStatus runtimeStatus;
  final AuthoringRevision3DataAssetArtifactAuthority artifactAuthority;
  final AuthoringRevision3DataAssetNativePublicationStatus publicationStatus;

  static AuthoringRevision3DataAssetStage _parse(Object? raw, String context) {
    final stage = _dataAssetObject(raw, context);
    _authoringExactFields(stage, const <String>{
      'manifest_asset',
      'manifest',
    }, context);
    _dataAssetRequireCanonicalValueOrder(stage, context);
    final manifestAsset = AuthoringRevision3DataAssetContentSeal._parse(
      stage['manifest_asset'],
      '$context manifest asset',
      maxByteLength: _maxAuthoringRevision3DataAssetManifestBytes,
    );
    final parsed = _dataAssetManifest(
      stage['manifest'],
      context: '$context manifest',
    );
    final manifestJson = jsonEncode(parsed.storageJson);
    final manifestBytes = utf8.encode(manifestJson);
    if (manifestBytes.isEmpty ||
        manifestBytes.length > _maxAuthoringRevision3DataAssetManifestBytes ||
        manifestAsset.byteLength != manifestBytes.length ||
        manifestAsset.sha256 !=
            crypto.sha256.convert(manifestBytes).toString()) {
      throw FormatException(
        'authoring $context manifest disagrees with its exact content seal',
      );
    }
    return AuthoringRevision3DataAssetStage._(
      manifestAsset: manifestAsset,
      projectId: parsed.projectId,
      projectTargetExecutable: parsed.projectTargetExecutable,
      basisHead: parsed.basisHead,
      basisProjectRevision: parsed.basisProjectRevision,
      stagedProjectRevision: parsed.stagedProjectRevision,
      targetPath: parsed.targetPath,
      selectorKind: parsed.selectorKind,
      selectorPathDepth: parsed.selectorPathDepth,
      replacementByteLength: parsed.replacementByteLength,
      intentBindingSha256: parsed.intentBindingSha256,
      patchedUasset: parsed.patchedUasset,
      patchedUexp: parsed.patchedUexp,
      usmap: parsed.usmap,
      sidecars:
          Map<String, AuthoringRevision3DataAssetContentSeal>.unmodifiable(
            parsed.sidecars,
          ),
      generationContainerCount: parsed.generationContainerCount,
      generationChunkCount: parsed.generationChunkCount,
      buildStatus: parsed.buildStatus,
      runtimeStatus: parsed.runtimeStatus,
      artifactAuthority: parsed.artifactAuthority,
      publicationStatus: parsed.publicationStatus,
    );
  }
}

/// Strict result of native prepare-only DataAsset staging.
final class AuthoringRevision3DataAssetStagePreparation {
  const AuthoringRevision3DataAssetStagePreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.stage,
    required this.deduplicatedBlobs,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final AuthoringRevision3DataAssetStage stage;
  final int deduplicatedBlobs;
  final AuthoringRevision3DataAssetBuildStatus buildStatus;
  final AuthoringRevision3DataAssetRuntimeStatus runtimeStatus;
  final AuthoringRevision3DataAssetArtifactAuthority artifactAuthority;
  final AuthoringRevision3DataAssetNativePublicationStatus publicationStatus;

  factory AuthoringRevision3DataAssetStagePreparation.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    String? expectedIntentBindingSha256,
  }) {
    _dataAssetResponsePreflight(json, <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'stage',
      'deduplicated_blobs',
      'build_status',
      'runtime_status',
      'artifact_authority',
      'publication_status',
      if (expectedIntentBindingSha256 != null) 'intent_binding_sha256',
    }, 'revision-3 DataAsset preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 DataAsset response is not an unpublished preparation',
      );
    }
    final basisHead = _dataAssetHead(
      json,
      'basis_head_json',
      'revision-3 DataAsset preparation response',
    );
    _dataAssetRequireExpectedBasis(basisHead, expectedHead);
    final head = _dataAssetHead(
      json,
      'head_json',
      'revision-3 DataAsset preparation response',
    );
    _dataAssetRequireAdvancedHead(head, basisHead);
    final candidate = _dataAssetCandidateProject(
      json,
      context: 'revision-3 DataAsset preparation response',
    );
    final stage = AuthoringRevision3DataAssetStage._parse(
      json['stage'],
      'revision-3 DataAsset prepared stage',
    );
    if (expectedIntentBindingSha256 != null) {
      final actualBinding = _dataAssetString(
        json,
        'intent_binding_sha256',
        'revision-3 DataAsset preparation response',
        maxBytes: 64,
      );
      if (!_authoringSha256Pattern.hasMatch(actualBinding) ||
          actualBinding != expectedIntentBindingSha256 ||
          stage._intentBindingSha256 != expectedIntentBindingSha256) {
        throw const FormatException(
          'authoring revision-3 DataAsset response changed the exact typed edit binding',
        );
      }
    }
    if (stage.projectId != candidate.projectId ||
        !stage.projectTargetExecutable._same(candidate.targetExecutable) ||
        stage.basisHead.canonicalJson != basisHead.canonicalJson ||
        stage.basisProjectRevision + 1 != candidate.revision ||
        stage.stagedProjectRevision != candidate.revision) {
      throw const FormatException(
        'authoring revision-3 DataAsset stage is not bound to its exact candidate and basis',
      );
    }
    _dataAssetRequireCandidateStageAssets(
      candidate.project,
      stage,
      present: true,
    );
    final deduplicatedBlobs = _dataAssetInt(
      json,
      'deduplicated_blobs',
      'revision-3 DataAsset preparation response',
      max: 4 + stage.sidecars.length,
    );
    final statuses = _dataAssetStatuses(
      json,
      'revision-3 DataAsset preparation response',
    );
    return AuthoringRevision3DataAssetStagePreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: candidate.projectJson,
      projectId: candidate.projectId,
      revision: candidate.revision,
      stage: stage,
      deduplicatedBlobs: deduplicatedBlobs,
      buildStatus: statuses.build,
      runtimeStatus: statuses.runtime,
      artifactAuthority: statuses.artifact,
      publicationStatus: statuses.publication,
    );
  }
}

/// Strict, read-only listing at one exact published revision-3 head.
final class AuthoringRevision3DataAssetStageListResult {
  const AuthoringRevision3DataAssetStageListResult._({
    required this.basisHead,
    required this.revision,
    required this.stages,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final int revision;
  final List<AuthoringRevision3DataAssetStage> stages;
  final AuthoringRevision3DataAssetBuildStatus buildStatus;
  final AuthoringRevision3DataAssetRuntimeStatus runtimeStatus;
  final AuthoringRevision3DataAssetArtifactAuthority artifactAuthority;
  final AuthoringRevision3DataAssetNativePublicationStatus publicationStatus;

  factory AuthoringRevision3DataAssetStageListResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
  }) {
    _dataAssetResponsePreflight(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'revision',
      'stages',
      'build_status',
      'runtime_status',
      'artifact_authority',
      'publication_status',
    }, 'revision-3 DataAsset list response');
    if (json['ok'] != true || json['outcome'] != 'listed_exact_head') {
      throw const FormatException(
        'authoring revision-3 DataAsset list response is not exact-head read-only data',
      );
    }
    final basisHead = _dataAssetHead(
      json,
      'basis_head_json',
      'revision-3 DataAsset list response',
    );
    _dataAssetRequireExpectedBasis(basisHead, expectedHead);
    final revision = _dataAssetInt(
      json,
      'revision',
      'revision-3 DataAsset list response',
    );
    final rawStages = _dataAssetList(
      json['stages'],
      'revision-3 DataAsset list stages',
      maxLength: _maxAuthoringRevision3DataAssetStages,
    );
    final stages = <AuthoringRevision3DataAssetStage>[];
    final targets = <String>{};
    String? previousTargetPath;
    String? projectId;
    AuthoringRevision3DataAssetContentSeal? projectTarget;
    for (var index = 0; index < rawStages.length; index++) {
      final stage = AuthoringRevision3DataAssetStage._parse(
        rawStages[index],
        'revision-3 DataAsset listed stage $index',
      );
      if (stage.stagedProjectRevision > revision ||
          !targets.add(stage.targetPath.toLowerCase()) ||
          (previousTargetPath != null &&
              previousTargetPath.compareTo(stage.targetPath) >= 0) ||
          (projectId != null && projectId != stage.projectId) ||
          (projectTarget != null &&
              !projectTarget._same(stage.projectTargetExecutable))) {
        throw const FormatException(
          'authoring revision-3 DataAsset list is not one unique ordered project registry',
        );
      }
      previousTargetPath = stage.targetPath;
      projectId ??= stage.projectId;
      projectTarget ??= stage.projectTargetExecutable;
      stages.add(stage);
    }
    final statuses = _dataAssetStatuses(
      json,
      'revision-3 DataAsset list response',
    );
    return AuthoringRevision3DataAssetStageListResult._(
      basisHead: basisHead,
      revision: revision,
      stages: List<AuthoringRevision3DataAssetStage>.unmodifiable(stages),
      buildStatus: statuses.build,
      runtimeStatus: statuses.runtime,
      artifactAuthority: statuses.artifact,
      publicationStatus: statuses.publication,
    );
  }
}

/// Strict result of native prepare-only removal from the managed stage registry.
final class AuthoringRevision3DataAssetStageRemovalPreparation {
  const AuthoringRevision3DataAssetStageRemovalPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.removed,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final AuthoringRevision3DataAssetStage removed;
  final AuthoringRevision3DataAssetBuildStatus buildStatus;
  final AuthoringRevision3DataAssetRuntimeStatus runtimeStatus;
  final AuthoringRevision3DataAssetArtifactAuthority artifactAuthority;
  final AuthoringRevision3DataAssetNativePublicationStatus publicationStatus;

  factory AuthoringRevision3DataAssetStageRemovalPreparation.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String requestedTargetPath,
  }) {
    _dataAssetResponsePreflight(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'removed',
      'build_status',
      'runtime_status',
      'artifact_authority',
      'publication_status',
    }, 'revision-3 DataAsset removal response');
    if (json['ok'] != true ||
        json['outcome'] != 'prepared_remove_unpublished') {
      throw const FormatException(
        'authoring revision-3 DataAsset response is not an unpublished removal',
      );
    }
    final basisHead = _dataAssetHead(
      json,
      'basis_head_json',
      'revision-3 DataAsset removal response',
    );
    _dataAssetRequireExpectedBasis(basisHead, expectedHead);
    final head = _dataAssetHead(
      json,
      'head_json',
      'revision-3 DataAsset removal response',
    );
    _dataAssetRequireAdvancedHead(head, basisHead);
    final candidate = _dataAssetCandidateProject(
      json,
      context: 'revision-3 DataAsset removal response',
    );
    final removed = AuthoringRevision3DataAssetStage._parse(
      json['removed'],
      'revision-3 DataAsset removed stage',
    );
    if (removed.projectId != candidate.projectId ||
        !removed.projectTargetExecutable._same(candidate.targetExecutable) ||
        removed.targetPath.toLowerCase() != requestedTargetPath.toLowerCase() ||
        removed.stagedProjectRevision >= candidate.revision) {
      throw const FormatException(
        'authoring revision-3 DataAsset removal is not bound to its requested project stage',
      );
    }
    _dataAssetRequireCandidateStageAssets(
      candidate.project,
      removed,
      present: false,
    );
    final statuses = _dataAssetStatuses(
      json,
      'revision-3 DataAsset removal response',
    );
    return AuthoringRevision3DataAssetStageRemovalPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: candidate.projectJson,
      projectId: candidate.projectId,
      revision: candidate.revision,
      removed: removed,
      buildStatus: statuses.build,
      runtimeStatus: statuses.runtime,
      artifactAuthority: statuses.artifact,
      publicationStatus: statuses.publication,
    );
  }
}

typedef _DataAssetStatuses = ({
  AuthoringRevision3DataAssetBuildStatus build,
  AuthoringRevision3DataAssetRuntimeStatus runtime,
  AuthoringRevision3DataAssetArtifactAuthority artifact,
  AuthoringRevision3DataAssetNativePublicationStatus publication,
});

typedef _DataAssetCandidateProject = ({
  String projectJson,
  Map<String, Object?> project,
  String projectId,
  int revision,
  AuthoringRevision3DataAssetContentSeal targetExecutable,
});

typedef _DataAssetManifestProjection = ({
  String projectId,
  AuthoringRevision3DataAssetContentSeal projectTargetExecutable,
  AuthoringWorkingHead basisHead,
  int basisProjectRevision,
  int stagedProjectRevision,
  String targetPath,
  String selectorKind,
  int selectorPathDepth,
  int replacementByteLength,
  String intentBindingSha256,
  AuthoringRevision3DataAssetContentSeal patchedUasset,
  AuthoringRevision3DataAssetContentSeal patchedUexp,
  AuthoringRevision3DataAssetContentSeal usmap,
  Map<String, AuthoringRevision3DataAssetContentSeal> sidecars,
  int generationContainerCount,
  int generationChunkCount,
  AuthoringRevision3DataAssetBuildStatus buildStatus,
  AuthoringRevision3DataAssetRuntimeStatus runtimeStatus,
  AuthoringRevision3DataAssetArtifactAuthority artifactAuthority,
  AuthoringRevision3DataAssetNativePublicationStatus publicationStatus,
  Map<String, Object?> storageJson,
});

void _dataAssetResponsePreflight(
  Map<String, Object?> json,
  Set<String> fields,
  String context,
) {
  _authoringExactFields(json, fields, context);
  _authoringRequireSignedSafeUnsignedJsonNumbers(json, context);
  // The transport already caps this at 64 MiB. Re-encoding here also proves that the response
  // contains only JSON values before any nested value is promoted into a typed DTO.
  final encoded = utf8.encode(jsonEncode(json));
  if (encoded.length > _maxAuthoringRevision3DataAssetResponseBytes) {
    throw FormatException('authoring $context exceeds its response budget');
  }
}

AuthoringWorkingHead _dataAssetHead(
  Map<String, Object?> json,
  String field,
  String context,
) => AuthoringWorkingHead.fromCanonicalJson(
  _dataAssetString(json, field, context, maxBytes: _maxAuthoringHeadJsonBytes),
);

void _dataAssetRequireExpectedBasis(
  AuthoringWorkingHead actual,
  AuthoringWorkingHead expected,
) {
  if (actual.canonicalJson != expected.canonicalJson) {
    throw const FormatException(
      'authoring revision-3 DataAsset response changed its exact basis head',
    );
  }
}

void _dataAssetRequireAdvancedHead(
  AuthoringWorkingHead candidate,
  AuthoringWorkingHead basis,
) {
  if (candidate.canonicalJson == basis.canonicalJson) {
    throw const FormatException(
      'authoring revision-3 DataAsset candidate did not advance its head',
    );
  }
}

_DataAssetCandidateProject _dataAssetCandidateProject(
  Map<String, Object?> json, {
  required String context,
}) {
  final projectJson = _dataAssetString(
    json,
    'project_json',
    context,
    maxBytes: _maxAuthoringProjectJsonBytes,
  );
  final parsed = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
  final revision = _dataAssetInt(json, 'revision', context, min: 1);
  if (revision != parsed.revision) {
    throw FormatException('authoring $context revision disagrees with project');
  }
  // A WorkingHead seals the revision-3 snapshot manifest, not the reconstructed monolithic
  // project JSON. Their bytes intentionally differ because entities are stored as shards. The
  // managed session proves this relationship by fully reopening the returned head and comparing
  // both that head and the exact project JSON before publication.
  final target = _dataAssetObject(
    parsed.project['target'],
    '$context project target',
  );
  _authoringExactFields(target, const <String>{
    'executable',
  }, '$context project target');
  return (
    projectJson: projectJson,
    project: parsed.project,
    projectId: parsed.projectId,
    revision: parsed.revision,
    targetExecutable: AuthoringRevision3DataAssetContentSeal._parse(
      target['executable'],
      '$context project executable',
    ),
  );
}

_DataAssetStatuses _dataAssetStatuses(
  Map<String, Object?> json,
  String context,
) => (
  build: switch (json['build_status']) {
    'blocked' => AuthoringRevision3DataAssetBuildStatus.blocked,
    _ => throw FormatException(
      'authoring $context has an unsupported build status',
    ),
  },
  runtime: switch (json['runtime_status']) {
    'runtime_unqualified' =>
      AuthoringRevision3DataAssetRuntimeStatus.runtimeUnqualified,
    _ => throw FormatException(
      'authoring $context has an unsupported runtime status',
    ),
  },
  artifact: switch (json['artifact_authority']) {
    'not_granted' => AuthoringRevision3DataAssetArtifactAuthority.notGranted,
    _ => throw FormatException(
      'authoring $context grants unsupported artifact authority',
    ),
  },
  publication: switch (json['publication_status']) {
    'not_supported' =>
      AuthoringRevision3DataAssetNativePublicationStatus.notSupported,
    _ => throw FormatException(
      'authoring $context grants unsupported publication authority',
    ),
  },
);

void _dataAssetRequireCandidateStageAssets(
  Map<String, Object?> project,
  AuthoringRevision3DataAssetStage stage, {
  required bool present,
}) {
  final assetStore = _dataAssetObject(
    project['asset_store'],
    'revision-3 DataAsset candidate asset store',
  );
  _authoringExactFields(assetStore, const <String>{
    'assets',
  }, 'revision-3 DataAsset candidate asset store');
  final assets = _dataAssetObject(
    assetStore['assets'],
    'revision-3 DataAsset candidate assets',
  );
  final manifestMeta = assets[stage.manifestAsset.sha256];
  if (!present) {
    if (manifestMeta != null) {
      throw const FormatException(
        'authoring revision-3 DataAsset removal retained its registry manifest',
      );
    }
    return;
  }
  _dataAssetRequireAssetMeta(
    manifestMeta,
    stage.manifestAsset,
    _revision3DataAssetManifestMediaType,
    'manifest',
  );
  for (final entry in <AuthoringRevision3DataAssetContentSeal>[
    stage.patchedUasset,
    stage.patchedUexp,
    stage.usmap,
    ...stage.sidecars.values,
  ]) {
    _dataAssetRequireAssetMeta(
      assets[entry.sha256],
      entry,
      _revision3DataAssetComponentMediaType,
      'component',
    );
  }
}

void _dataAssetRequireAssetMeta(
  Object? raw,
  AuthoringRevision3DataAssetContentSeal seal,
  String mediaType,
  String context,
) {
  final meta = _dataAssetObject(
    raw,
    'revision-3 DataAsset candidate $context metadata',
  );
  _authoringExactFields(meta, const <String>{
    'byte_len',
    'media_type',
  }, 'revision-3 DataAsset candidate $context metadata');
  if (_dataAssetInt(
        meta,
        'byte_len',
        'revision-3 DataAsset candidate $context metadata',
        min: 1,
      ) !=
      seal.byteLength) {
    throw FormatException(
      'authoring revision-3 DataAsset candidate $context length disagrees',
    );
  }
  if (_dataAssetString(
        meta,
        'media_type',
        'revision-3 DataAsset candidate $context metadata',
        maxBytes: 256,
      ) !=
      mediaType) {
    throw FormatException(
      'authoring revision-3 DataAsset candidate $context media type disagrees',
    );
  }
}

_DataAssetManifestProjection _dataAssetManifest(
  Object? raw, {
  required String context,
}) {
  final json = _dataAssetObject(raw, context);
  _authoringExactFields(json, const <String>{
    'format',
    'project_id',
    'project_target',
    'basis_head',
    'basis_project_revision',
    'staged_project_revision',
    'target_path',
    'generation',
    'selector',
    'replacement_hex',
    'patched_uasset',
    'patched_uexp',
    'usmap',
    'sidecars',
    'build_status',
    'runtime_status',
    'artifact_authority',
    'publication_status',
  }, context);
  if (_dataAssetString(json, 'format', context, maxBytes: 64) !=
      _revision3DataAssetManifestFormat) {
    throw FormatException('authoring $context has an unsupported format');
  }
  final projectId = _dataAssetEntityId(json, 'project_id', context);
  final projectTarget = _dataAssetObject(
    json['project_target'],
    '$context project target',
  );
  _authoringExactFields(projectTarget, const <String>{
    'executable',
  }, '$context project target');
  final projectTargetExecutable = AuthoringRevision3DataAssetContentSeal._parse(
    projectTarget['executable'],
    '$context project executable',
  );
  final basisHeadMap = _dataAssetObject(json['basis_head'], '$context basis');
  _authoringExactFields(basisHeadMap, const <String>{
    'store_format',
    'snapshot',
  }, '$context basis');
  if (basisHeadMap['store_format'] != 1) {
    throw FormatException('authoring $context basis store format is invalid');
  }
  final basisSnapshot = _dataAssetObject(
    basisHeadMap['snapshot'],
    '$context basis snapshot',
  );
  _authoringExactFields(basisSnapshot, const <String>{
    'byte_len',
    'sha256',
  }, '$context basis snapshot');
  final basisHead = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{
        'byte_len': _dataAssetInt(
          basisSnapshot,
          'byte_len',
          '$context basis snapshot',
          min: 1,
          max: _maxAuthoringProjectJsonBytes,
        ),
        'sha256': _dataAssetSha256(
          basisSnapshot,
          'sha256',
          '$context basis snapshot',
        ),
      },
    }),
  );
  final basisProjectRevision = _dataAssetInt(
    json,
    'basis_project_revision',
    context,
    max: _maxAuthoringStoryBaseRevision,
  );
  final stagedProjectRevision = _dataAssetInt(
    json,
    'staged_project_revision',
    context,
    min: 1,
  );
  if (basisProjectRevision + 1 != stagedProjectRevision) {
    throw FormatException('authoring $context revision binding is invalid');
  }
  final targetPath = _dataAssetString(
    json,
    'target_path',
    context,
    maxBytes: 512,
  );
  _dataAssetRequireTargetPath(targetPath, context);
  final generation = _dataAssetGeneration(json['generation'], context);
  if (generation.asset != targetPath) {
    throw FormatException(
      'authoring $context generation targets another asset',
    );
  }
  final selector = _dataAssetSelector(json['selector'], context);
  if (selector.usmapSha256 != generation.usmapSha256) {
    throw FormatException(
      'authoring $context selector is not bound to the exact generation USMAP',
    );
  }
  final replacementHex = _dataAssetString(
    json,
    'replacement_hex',
    context,
    maxBytes: 64,
  );
  _dataAssetRequireHex(
    replacementHex,
    selector.width * 2,
    '$context replacement',
  );
  if (replacementHex == selector.expectedHex) {
    throw FormatException(
      'authoring $context replacement does not change the leaf',
    );
  }
  if (selector.kind == 'package_index' || selector.kind == 'fname') {
    throw FormatException(
      'authoring $context selector targets a referential fixed leaf',
    );
  }
  if (selector.kind == 'bool' &&
      (selector.expectedHex != '00' && selector.expectedHex != '01' ||
          replacementHex != '00' && replacementHex != '01')) {
    throw FormatException(
      'authoring $context Bool edit does not use canonical 0/1 bytes',
    );
  }
  final patchedUasset = AuthoringRevision3DataAssetContentSeal._parse(
    json['patched_uasset'],
    '$context patched uasset',
    maxByteLength: 512 * 1024 * 1024,
  );
  final patchedUexp = AuthoringRevision3DataAssetContentSeal._parse(
    json['patched_uexp'],
    '$context patched uexp',
    maxByteLength: 512 * 1024 * 1024,
  );
  if (patchedUasset.byteLength + patchedUexp.byteLength > 1024 * 1024 * 1024) {
    throw FormatException('authoring $context patched pair exceeds its budget');
  }
  final usmap = AuthoringRevision3DataAssetContentSeal._parse(
    json['usmap'],
    '$context USMAP',
    maxByteLength: 512 * 1024 * 1024,
  );
  if (usmap.byteLength != generation.usmapLength ||
      usmap.sha256 != generation.usmapSha256) {
    throw FormatException('authoring $context USMAP differs from generation');
  }
  final sidecarRaw = _dataAssetObject(json['sidecars'], '$context sidecars');
  if (sidecarRaw.length > 3 ||
      sidecarRaw.keys.any(
        (key) => !const <String>{
          'BulkData',
          'OptionalBulkData',
          'MemoryMappedBulkData',
        }.contains(key),
      )) {
    throw FormatException('authoring $context sidecars are not closed');
  }
  final sidecars = <String, AuthoringRevision3DataAssetContentSeal>{};
  for (final key in const <String>[
    'BulkData',
    'OptionalBulkData',
    'MemoryMappedBulkData',
  ]) {
    if (!sidecarRaw.containsKey(key)) continue;
    sidecars[key] = AuthoringRevision3DataAssetContentSeal._parse(
      sidecarRaw[key],
      '$context sidecar $key',
      maxByteLength: 512 * 1024 * 1024,
    );
  }
  final persistedSidecarRoles = <String>[
    for (final role in const <String>[
      'BulkData',
      'OptionalBulkData',
      'MemoryMappedBulkData',
    ])
      if (sidecars.containsKey(role)) role,
  ];
  if (!_dataAssetStringListsEqual(
    persistedSidecarRoles,
    generation.targetSidecarRoles,
  )) {
    throw FormatException(
      'authoring $context sidecars do not match the exact target bulk chunks',
    );
  }
  final statuses = _dataAssetStatuses(json, context);
  final storageJson = <String, Object?>{
    'format': _revision3DataAssetManifestFormat,
    'project_id': projectId,
    'project_target': <String, Object?>{
      'executable': projectTargetExecutable._storageJson(),
    },
    'basis_head': jsonDecode(basisHead.canonicalJson),
    'basis_project_revision': basisProjectRevision,
    'staged_project_revision': stagedProjectRevision,
    'target_path': targetPath,
    'generation': generation.storageJson,
    'selector': selector.storageJson,
    'replacement_hex': replacementHex,
    'patched_uasset': patchedUasset._storageJson(),
    'patched_uexp': patchedUexp._storageJson(),
    'usmap': usmap._storageJson(),
    'sidecars': <String, Object?>{
      for (final key in const <String>[
        'BulkData',
        'OptionalBulkData',
        'MemoryMappedBulkData',
      ])
        if (sidecars.containsKey(key)) key: sidecars[key]!._storageJson(),
    },
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'artifact_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
  return (
    projectId: projectId,
    projectTargetExecutable: projectTargetExecutable,
    basisHead: basisHead,
    basisProjectRevision: basisProjectRevision,
    stagedProjectRevision: stagedProjectRevision,
    targetPath: targetPath,
    selectorKind: selector.kind,
    selectorPathDepth: selector.pathDepth,
    replacementByteLength: selector.width,
    intentBindingSha256: _dataAssetIntentBindingSha256(
      targetPath,
      selector.storageJson,
      replacementHex,
    ),
    patchedUasset: patchedUasset,
    patchedUexp: patchedUexp,
    usmap: usmap,
    sidecars: sidecars,
    generationContainerCount: generation.containerCount,
    generationChunkCount: generation.chunkCount,
    buildStatus: statuses.build,
    runtimeStatus: statuses.runtime,
    artifactAuthority: statuses.artifact,
    publicationStatus: statuses.publication,
    storageJson: storageJson,
  );
}

String _dataAssetIntentBindingSha256(
  String targetPath,
  Map<String, Object?> selector,
  String replacementHex,
) {
  final bytes = BytesBuilder(copy: false)
    ..add(
      utf8.encode('gore.authoring.r3-dataasset-edit.intent-binding.v1\u0000'),
    );
  for (final value in <List<int>>[
    utf8.encode(targetPath),
    utf8.encode(jsonEncode(selector)),
    _dataAssetDecodeCanonicalHex(replacementHex),
  ]) {
    final length = ByteData(8)..setUint64(0, value.length, Endian.little);
    bytes
      ..add(length.buffer.asUint8List())
      ..add(value);
  }
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Uint8List _dataAssetDecodeCanonicalHex(String value) {
  final result = Uint8List(value.length ~/ 2);
  for (var index = 0; index < result.length; index++) {
    result[index] = int.parse(
      value.substring(index * 2, index * 2 + 2),
      radix: 16,
    );
  }
  return result;
}

typedef _DataAssetGeneration = ({
  String asset,
  int usmapLength,
  String usmapSha256,
  List<String> targetSidecarRoles,
  int containerCount,
  int chunkCount,
  Map<String, Object?> storageJson,
});

_DataAssetGeneration _dataAssetGeneration(Object? raw, String context) {
  final json = _dataAssetObject(raw, '$context generation');
  _authoringExactFields(json, const <String>{
    'format',
    'asset',
    'usmap',
    'main_utoc',
    'global_utoc',
    'global_ucas',
    'container_set',
    'target_chunks',
  }, '$context generation');
  if (_dataAssetString(json, 'format', '$context generation', maxBytes: 64) !=
      'gore.asset.generation.v1') {
    throw FormatException('authoring $context generation format is invalid');
  }
  final asset = _dataAssetString(
    json,
    'asset',
    '$context generation',
    maxBytes: 512,
  );
  _dataAssetRequireTargetPath(asset, '$context generation');
  final usmap = _dataAssetFileAnchor(
    json['usmap'],
    '$context generation USMAP',
  );
  final mainUtoc = _dataAssetFileAnchor(
    json['main_utoc'],
    '$context generation main UTOC',
  );
  final globalUtoc = _dataAssetFileAnchor(
    json['global_utoc'],
    '$context generation global UTOC',
  );
  final globalUcas = _dataAssetFileAnchor(
    json['global_ucas'],
    '$context generation global UCAS',
  );
  final rawContainers = _dataAssetList(
    json['container_set'],
    '$context generation container set',
    minLength: 1,
    maxLength: 256,
  );
  final containers = <Map<String, Object?>>[];
  final containerIdentities = <String>{};
  for (var index = 0; index < rawContainers.length; index++) {
    final anchor = _dataAssetFileAnchor(
      rawContainers[index],
      '$context generation container $index',
    );
    final identity = jsonEncode(anchor);
    if (!containerIdentities.add(identity)) {
      throw FormatException(
        'authoring $context generation has duplicate containers',
      );
    }
    containers.add(anchor);
  }
  if (!containerIdentities.contains(jsonEncode(mainUtoc)) ||
      !containerIdentities.contains(jsonEncode(globalUtoc))) {
    throw FormatException(
      'authoring $context generation omits a selected container',
    );
  }
  final rawChunks = _dataAssetList(
    json['target_chunks'],
    '$context generation chunks',
    minLength: 1,
    maxLength: 4096,
  );
  final chunks = <Map<String, Object?>>[];
  final chunkIds = <String>{};
  var hasHeader = false;
  var hasExport = false;
  final targetSidecarRoles = <String>{};
  for (var index = 0; index < rawChunks.length; index++) {
    final chunk = _dataAssetChunkAnchor(
      rawChunks[index],
      '$context generation chunk $index',
      containerIdentities,
    );
    final chunkId = chunk['chunk_id']! as String;
    if (!chunkIds.add(chunkId)) {
      throw FormatException(
        'authoring $context generation has duplicate chunk IDs',
      );
    }
    hasHeader |= chunk['chunk_type'] == 'ContainerHeader';
    final chunkType = chunk['chunk_type']! as String;
    final belongsToTarget = _dataAssetChunkIdMatchesTarget(chunkId, asset);
    hasExport |= chunkType == 'ExportBundleData' && belongsToTarget;
    if (belongsToTarget &&
        const <String>{
          'BulkData',
          'OptionalBulkData',
          'MemoryMappedBulkData',
        }.contains(chunkType) &&
        !targetSidecarRoles.add(chunkType)) {
      throw FormatException(
        'authoring $context generation duplicates a target bulk chunk role',
      );
    }
    chunks.add(chunk);
  }
  if (!hasHeader || !hasExport) {
    throw FormatException(
      'authoring $context generation lacks required target chunks',
    );
  }
  return (
    asset: asset,
    usmapLength: usmap['length']! as int,
    usmapSha256: usmap['sha256']! as String,
    targetSidecarRoles: <String>[
      for (final role in const <String>[
        'BulkData',
        'OptionalBulkData',
        'MemoryMappedBulkData',
      ])
        if (targetSidecarRoles.contains(role)) role,
    ],
    containerCount: containers.length,
    chunkCount: chunks.length,
    storageJson: <String, Object?>{
      'format': 'gore.asset.generation.v1',
      'asset': asset,
      'usmap': usmap,
      'main_utoc': mainUtoc,
      'global_utoc': globalUtoc,
      'global_ucas': globalUcas,
      'container_set': containers,
      'target_chunks': chunks,
    },
  );
}

Map<String, Object?> _dataAssetFileAnchor(Object? raw, String context) {
  final json = _dataAssetObject(raw, context);
  _authoringExactFields(json, const <String>{
    'file_name',
    'length',
    'sha256',
  }, context);
  final fileName = _dataAssetString(json, 'file_name', context, maxBytes: 255);
  if (fileName.contains('/') ||
      fileName.contains('\\') ||
      fileName.endsWith('.') ||
      fileName.endsWith(' ') ||
      _authoringRevision3DataAssetWindowsReservedName(fileName)) {
    throw FormatException('authoring $context file name is not canonical');
  }
  return <String, Object?>{
    'file_name': fileName,
    'length': _dataAssetInt(json, 'length', context, min: 1),
    'sha256': _dataAssetSha256(json, 'sha256', context),
  };
}

Map<String, Object?> _dataAssetChunkAnchor(
  Object? raw,
  String context,
  Set<String> containers,
) {
  final json = _dataAssetObject(raw, context);
  _authoringExactFields(json, const <String>{
    'chunk_id',
    'chunk_type',
    'winner_utoc',
    'length',
    'blake3',
    'toc_hash',
    'toc_hash_bytes',
  }, context);
  final chunkId = _dataAssetString(json, 'chunk_id', context, maxBytes: 24);
  _dataAssetRequireHex(chunkId, 24, '$context chunk ID');
  final chunkType = _dataAssetString(json, 'chunk_type', context, maxBytes: 32);
  if (!const <String>{
    'ContainerHeader',
    'ExportBundleData',
    'BulkData',
    'OptionalBulkData',
    'MemoryMappedBulkData',
  }.contains(chunkType)) {
    throw FormatException('authoring $context chunk type is unsupported');
  }
  final winner = _dataAssetFileAnchor(json['winner_utoc'], '$context winner');
  if (!containers.contains(jsonEncode(winner))) {
    throw FormatException('authoring $context winner is not a container');
  }
  final length = _dataAssetInt(json, 'length', context, max: 512 * 1024 * 1024);
  if ((chunkType == 'ContainerHeader' || chunkType == 'ExportBundleData') &&
      length == 0) {
    throw FormatException('authoring $context chunk must not be empty');
  }
  final blake3 = _dataAssetString(json, 'blake3', context, maxBytes: 64);
  _dataAssetRequireHex(blake3, 64, '$context BLAKE3');
  final tocHashBytes = _dataAssetInt(
    json,
    'toc_hash_bytes',
    context,
    min: 20,
    max: 32,
  );
  if (tocHashBytes != 20 && tocHashBytes != 32) {
    throw FormatException('authoring $context TOC hash width is unsupported');
  }
  final tocHash = _dataAssetString(json, 'toc_hash', context, maxBytes: 64);
  _dataAssetRequireHex(tocHash, tocHashBytes * 2, '$context TOC hash');
  return <String, Object?>{
    'chunk_id': chunkId,
    'chunk_type': chunkType,
    'winner_utoc': winner,
    'length': length,
    'blake3': blake3,
    'toc_hash': tocHash,
    'toc_hash_bytes': tocHashBytes,
  };
}

bool _dataAssetStringListsEqual(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

bool _dataAssetChunkIdMatchesTarget(String chunkId, String assetPath) =>
    chunkId.startsWith(_dataAssetTargetPackageChunkPrefix(assetPath));

/// Reproduces `FIoContainerId::from_name`: CityHash64 over lowercase UTF-16LE, serialized as the
/// little-endian package-id prefix of a raw 12-byte IoStore chunk id.
String _dataAssetTargetPackageChunkPrefix(String assetPath) {
  final lower = assetPath.toLowerCase();
  final bytes = Uint8List(lower.length * 2);
  for (var index = 0; index < lower.length; index++) {
    // Canonical game asset paths are ASCII-only, so every UTF-16 code unit has a zero high byte.
    bytes[index * 2] = lower.codeUnitAt(index);
  }
  final packageId = _dataAssetCityHash64(bytes);
  final prefix = StringBuffer();
  for (var index = 0; index < 8; index++) {
    final byte = ((packageId >> (index * 8)) & BigInt.from(0xff)).toInt();
    prefix.write(byte.toRadixString(16).padLeft(2, '0'));
  }
  return prefix.toString();
}

final _dataAssetU64Mask = (BigInt.one << 64) - BigInt.one;
final _dataAssetCityK1 = BigInt.parse('b492b66fbe98f273', radix: 16);
final _dataAssetCityK2 = BigInt.parse('9ae16a3b2f90404f', radix: 16);
final _dataAssetCityHash16Mul = BigInt.parse('9ddfea08eb382d69', radix: 16);

BigInt _dataAssetU64(BigInt value) => value & _dataAssetU64Mask;

BigInt _dataAssetFetch64(Uint8List bytes, int offset) {
  var value = BigInt.zero;
  for (var index = 0; index < 8; index++) {
    value |= BigInt.from(bytes[offset + index]) << (index * 8);
  }
  return value;
}

BigInt _dataAssetRotate64(BigInt value, int shift) {
  value = _dataAssetU64(value);
  if (shift == 0) return value;
  return _dataAssetU64((value >> shift) | (value << (64 - shift)));
}

BigInt _dataAssetShiftMix(BigInt value) {
  value = _dataAssetU64(value);
  return value ^ (value >> 47);
}

BigInt _dataAssetSwapU64Bytes(BigInt value) {
  value = _dataAssetU64(value);
  var swapped = BigInt.zero;
  for (var index = 0; index < 8; index++) {
    swapped |=
        ((value >> (index * 8)) & BigInt.from(0xff)) << ((7 - index) * 8);
  }
  return swapped;
}

BigInt _dataAssetHashLen16WithMul(BigInt left, BigInt right, BigInt mul) {
  var first = _dataAssetU64((left ^ right) * mul);
  first ^= first >> 47;
  var second = _dataAssetU64((right ^ first) * mul);
  second ^= second >> 47;
  return _dataAssetU64(second * mul);
}

BigInt _dataAssetHashLen16(BigInt left, BigInt right) =>
    _dataAssetHashLen16WithMul(left, right, _dataAssetCityHash16Mul);

(BigInt, BigInt) _dataAssetWeakHashLen32WithSeeds(
  Uint8List bytes,
  int offset,
  BigInt firstSeed,
  BigInt secondSeed,
) {
  var first = _dataAssetU64(firstSeed + _dataAssetFetch64(bytes, offset));
  var second = _dataAssetRotate64(
    _dataAssetU64(secondSeed + first + _dataAssetFetch64(bytes, offset + 24)),
    21,
  );
  final carriedFirst = first;
  first = _dataAssetU64(
    first +
        _dataAssetFetch64(bytes, offset + 8) +
        _dataAssetFetch64(bytes, offset + 16),
  );
  second = _dataAssetU64(second + _dataAssetRotate64(first, 44));
  return (
    _dataAssetU64(first + _dataAssetFetch64(bytes, offset + 24)),
    _dataAssetU64(second + carriedFirst),
  );
}

BigInt _dataAssetCityHash64(Uint8List bytes) {
  final length = bytes.length;
  // A validated `/Game/<segment>` path is at least seven ASCII code units / 14 UTF-16LE bytes.
  if (length <= 16) {
    final mul = _dataAssetU64(_dataAssetCityK2 + BigInt.from(length * 2));
    final first = _dataAssetU64(_dataAssetFetch64(bytes, 0) + _dataAssetCityK2);
    final second = _dataAssetFetch64(bytes, length - 8);
    final third = _dataAssetU64(_dataAssetRotate64(second, 37) * mul + first);
    final fourth = _dataAssetU64(
      (_dataAssetRotate64(first, 25) + second) * mul,
    );
    return _dataAssetHashLen16WithMul(third, fourth, mul);
  }
  if (length <= 32) {
    final mul = _dataAssetU64(_dataAssetCityK2 + BigInt.from(length * 2));
    final first = _dataAssetU64(_dataAssetFetch64(bytes, 0) * _dataAssetCityK1);
    final second = _dataAssetFetch64(bytes, 8);
    final third = _dataAssetU64(_dataAssetFetch64(bytes, length - 8) * mul);
    final fourth = _dataAssetU64(
      _dataAssetFetch64(bytes, length - 16) * _dataAssetCityK2,
    );
    return _dataAssetHashLen16WithMul(
      _dataAssetU64(
        _dataAssetRotate64(_dataAssetU64(first + second), 43) +
            _dataAssetRotate64(third, 30) +
            fourth,
      ),
      _dataAssetU64(
        first +
            _dataAssetRotate64(_dataAssetU64(second + _dataAssetCityK2), 18) +
            third,
      ),
      mul,
    );
  }
  if (length <= 64) {
    final mul = _dataAssetU64(_dataAssetCityK2 + BigInt.from(length * 2));
    final first = _dataAssetU64(_dataAssetFetch64(bytes, 0) * _dataAssetCityK2);
    final second = _dataAssetFetch64(bytes, 8);
    final third = _dataAssetFetch64(bytes, length - 24);
    final fourth = _dataAssetFetch64(bytes, length - 32);
    final fifth = _dataAssetU64(
      _dataAssetFetch64(bytes, 16) * _dataAssetCityK2,
    );
    final sixth = _dataAssetU64(_dataAssetFetch64(bytes, 24) * BigInt.from(9));
    final seventh = _dataAssetFetch64(bytes, length - 8);
    final eighth = _dataAssetU64(_dataAssetFetch64(bytes, length - 16) * mul);
    final ninth = _dataAssetU64(
      _dataAssetRotate64(_dataAssetU64(first + seventh), 43) +
          _dataAssetU64(
            (_dataAssetRotate64(second, 30) + third) * BigInt.from(9),
          ),
    );
    final tenth = _dataAssetU64(
      (_dataAssetU64(first + seventh) ^ fourth) + sixth + BigInt.one,
    );
    final eleventh = _dataAssetU64(
      _dataAssetSwapU64Bytes(_dataAssetU64((ninth + tenth) * mul)) + eighth,
    );
    final twelfth = _dataAssetU64(
      _dataAssetRotate64(_dataAssetU64(fifth + sixth), 42) + third,
    );
    final thirteenth = _dataAssetU64(
      (_dataAssetSwapU64Bytes(_dataAssetU64((tenth + eleventh) * mul)) +
              seventh) *
          mul,
    );
    final fourteenth = _dataAssetU64(fifth + sixth + third);
    final fifteenth = _dataAssetU64(
      _dataAssetSwapU64Bytes(
            _dataAssetU64((twelfth + fourteenth) * mul + thirteenth),
          ) +
          second,
    );
    final sixteenth = _dataAssetU64(
      _dataAssetShiftMix(
            _dataAssetU64((fourteenth + fifteenth) * mul + fourth + eighth),
          ) *
          mul,
    );
    return _dataAssetU64(sixteenth + twelfth);
  }

  var first = _dataAssetFetch64(bytes, length - 40);
  var second = _dataAssetU64(
    _dataAssetFetch64(bytes, length - 16) +
        _dataAssetFetch64(bytes, length - 56),
  );
  var third = _dataAssetHashLen16(
    _dataAssetU64(_dataAssetFetch64(bytes, length - 48) + BigInt.from(length)),
    _dataAssetFetch64(bytes, length - 24),
  );
  var weakFirst = _dataAssetWeakHashLen32WithSeeds(
    bytes,
    length - 64,
    BigInt.from(length),
    third,
  );
  var weakSecond = _dataAssetWeakHashLen32WithSeeds(
    bytes,
    length - 32,
    _dataAssetU64(second + _dataAssetCityK1),
    first,
  );
  first = _dataAssetU64(first * _dataAssetCityK1 + _dataAssetFetch64(bytes, 0));
  final loopBytes = ((length - 1) ~/ 64) * 64;
  for (var offset = 0; offset < loopBytes; offset += 64) {
    first = _dataAssetU64(
      _dataAssetRotate64(
            _dataAssetU64(
              first +
                  second +
                  weakFirst.$1 +
                  _dataAssetFetch64(bytes, offset + 8),
            ),
            37,
          ) *
          _dataAssetCityK1,
    );
    second = _dataAssetU64(
      _dataAssetRotate64(
            _dataAssetU64(
              second + weakFirst.$2 + _dataAssetFetch64(bytes, offset + 48),
            ),
            42,
          ) *
          _dataAssetCityK1,
    );
    first ^= weakSecond.$2;
    second = _dataAssetU64(
      second + weakFirst.$1 + _dataAssetFetch64(bytes, offset + 40),
    );
    third = _dataAssetU64(
      _dataAssetRotate64(_dataAssetU64(third + weakSecond.$1), 33) *
          _dataAssetCityK1,
    );
    weakFirst = _dataAssetWeakHashLen32WithSeeds(
      bytes,
      offset,
      _dataAssetU64(weakFirst.$2 * _dataAssetCityK1),
      _dataAssetU64(first + weakSecond.$1),
    );
    weakSecond = _dataAssetWeakHashLen32WithSeeds(
      bytes,
      offset + 32,
      _dataAssetU64(third + weakSecond.$2),
      _dataAssetU64(second + _dataAssetFetch64(bytes, offset + 16)),
    );
    final swapped = first;
    first = third;
    third = swapped;
  }
  return _dataAssetHashLen16(
    _dataAssetU64(
      _dataAssetHashLen16(weakFirst.$1, weakSecond.$1) +
          _dataAssetU64(_dataAssetShiftMix(second) * _dataAssetCityK1) +
          third,
    ),
    _dataAssetU64(_dataAssetHashLen16(weakFirst.$2, weakSecond.$2) + first),
  );
}

typedef _DataAssetSelector = ({
  String kind,
  int width,
  String expectedHex,
  String usmapSha256,
  int pathDepth,
  Map<String, Object?> storageJson,
});

_DataAssetSelector _dataAssetSelector(Object? raw, String context) {
  final json = _dataAssetObject(raw, '$context selector');
  _authoringExactFields(json, const <String>{
    'format',
    'profile',
    'package_seal',
    'usmap_sha256',
    'export_index',
    'object_name',
    'class_path',
    'component',
    'export_sha256',
    'role',
    'kind',
    'path',
    'expected_hex',
  }, '$context selector');
  if (_dataAssetInt(json, 'format', '$context selector', min: 1, max: 1) != 1 ||
      _dataAssetString(json, 'profile', '$context selector', maxBytes: 32) !=
          'g1r_ue5_4') {
    throw FormatException('authoring $context selector profile is unsupported');
  }
  final packageSeal = _dataAssetPackageSeal(
    json['package_seal'],
    '$context selector package seal',
  );
  final usmapSha = _dataAssetSha256(json, 'usmap_sha256', '$context selector');
  final exportIndex = _dataAssetInt(json, 'export_index', '$context selector');
  final objectName = _dataAssetString(
    json,
    'object_name',
    '$context selector',
    maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
  );
  final classPath = _dataAssetString(
    json,
    'class_path',
    '$context selector',
    maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
  );
  final component = _dataAssetString(
    json,
    'component',
    '$context selector',
    maxBytes: 16,
  );
  if (component != 'uasset' && component != 'uexp') {
    throw FormatException(
      'authoring $context selector component is unsupported',
    );
  }
  final exportSha = _dataAssetSha256(
    json,
    'export_sha256',
    '$context selector',
  );
  if (_dataAssetString(json, 'role', '$context selector', maxBytes: 32) !=
      'property_value') {
    throw FormatException('authoring $context selector role is not editable');
  }
  final kind = _dataAssetString(
    json,
    'kind',
    '$context selector',
    maxBytes: 32,
  );
  final width = _dataAssetFixedKindWidth(kind, '$context selector');
  final rawPath = _dataAssetList(
    json['path'],
    '$context selector path',
    minLength: 1,
    maxLength: 128,
  );
  final path = <Map<String, Object?>>[];
  for (var index = 0; index < rawPath.length; index++) {
    path.add(
      _dataAssetSelectorStep(
        rawPath[index],
        '$context selector path step $index',
        0,
      ),
    );
  }
  final expectedHex = _dataAssetString(
    json,
    'expected_hex',
    '$context selector',
    maxBytes: 64,
  );
  _dataAssetRequireHex(expectedHex, width * 2, '$context expected bytes');
  return (
    kind: kind,
    width: width,
    expectedHex: expectedHex,
    usmapSha256: usmapSha,
    pathDepth: path.length,
    storageJson: <String, Object?>{
      'format': 1,
      'profile': 'g1r_ue5_4',
      'package_seal': packageSeal,
      'usmap_sha256': usmapSha,
      'export_index': exportIndex,
      'object_name': objectName,
      'class_path': classPath,
      'component': component,
      'export_sha256': exportSha,
      'role': 'property_value',
      'kind': kind,
      'path': path,
      'expected_hex': expectedHex,
    },
  );
}

Map<String, Object?> _dataAssetPackageSeal(Object? raw, String context) {
  final json = _dataAssetObject(raw, context);
  _authoringExactFields(json, const <String>{
    'uasset_sha256',
    'uexp_sha256',
  }, context);
  return <String, Object?>{
    'uasset_sha256': _dataAssetSha256(json, 'uasset_sha256', context),
    'uexp_sha256': _dataAssetSha256(json, 'uexp_sha256', context),
  };
}

Map<String, Object?> _dataAssetSelectorStep(
  Object? raw,
  String context,
  int depth,
) {
  if (depth > 128) {
    throw FormatException('authoring $context is too deeply nested');
  }
  final json = _dataAssetObject(raw, context);
  final step = _dataAssetString(json, 'step', context, maxBytes: 32);
  switch (step) {
    case 'property':
      _authoringExactFields(json, const <String>{
        'step',
        'schema_index',
        'property_name',
        'array_index',
        'array_dimension',
        'declaring_schema_name',
        'declaring_module_path',
        'property_type',
      }, context);
      final modulePath = json['declaring_module_path'];
      if (modulePath != null &&
          (modulePath is! String ||
              modulePath.isEmpty ||
              utf8.encode(modulePath).length >
                  _maxAuthoringRevision3DataAssetManifestStringBytes)) {
        throw FormatException('authoring $context module path is invalid');
      }
      return <String, Object?>{
        'step': step,
        'schema_index': _dataAssetInt(json, 'schema_index', context),
        'property_name': _dataAssetString(
          json,
          'property_name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
        'array_index': _dataAssetInt(json, 'array_index', context),
        'array_dimension': _dataAssetInt(
          json,
          'array_dimension',
          context,
          min: 1,
        ),
        'declaring_schema_name': _dataAssetString(
          json,
          'declaring_schema_name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
        'declaring_module_path': modulePath,
        'property_type': _dataAssetWireType(
          json['property_type'],
          '$context property type',
          depth + 1,
        ),
      };
    case 'struct':
      _authoringExactFields(json, const <String>{
        'step',
        'name',
        'schema_name',
      }, context);
      return <String, Object?>{
        'step': step,
        'name': _dataAssetString(
          json,
          'name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
        'schema_name': _dataAssetString(
          json,
          'schema_name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
      };
    case 'map':
      _authoringExactFields(json, const <String>{
        'step',
        'key_type',
        'value_type',
      }, context);
      return <String, Object?>{
        'step': step,
        'key_type': _dataAssetWireType(
          json['key_type'],
          '$context key type',
          depth + 1,
        ),
        'value_type': _dataAssetWireType(
          json['value_type'],
          '$context value type',
          depth + 1,
        ),
      };
    case 'map_entry_value':
    case 'map_entry_key':
    case 'removed_map_key':
      _authoringExactFields(json, const <String>{'step', 'key'}, context);
      return <String, Object?>{
        'step': step,
        'key': _dataAssetMapKey(json['key'], '$context key'),
      };
    default:
      throw FormatException('authoring $context has an unsupported step');
  }
}

Map<String, Object?> _dataAssetMapKey(Object? raw, String context) {
  final json = _dataAssetObject(raw, context);
  _authoringExactFields(json, const <String>{
    'kind',
    'byte_length',
    'sha256',
  }, context);
  final kind = json['kind'];
  if (kind != null) {
    if (kind is! String) {
      throw FormatException('authoring $context kind is invalid');
    }
    _dataAssetFixedKindWidth(kind, context);
  }
  return <String, Object?>{
    'kind': kind,
    'byte_length': _dataAssetInt(json, 'byte_length', context, min: 1),
    'sha256': _dataAssetSha256(json, 'sha256', context),
  };
}

Map<String, Object?> _dataAssetWireType(
  Object? raw,
  String context,
  int depth,
) {
  if (depth > 128) {
    throw FormatException('authoring $context is too deeply nested');
  }
  final json = _dataAssetObject(raw, context);
  final type = _dataAssetString(json, 'type', context, maxBytes: 32);
  const leafTypes = <String>{
    'byte',
    'bool',
    'int',
    'float',
    'object',
    'name',
    'delegate',
    'double',
    'string',
    'text',
    'interface',
    'multicast_delegate',
    'weak_object',
    'lazy_object',
    'asset_object',
    'soft_object',
    'uint64',
    'uint32',
    'uint16',
    'int64',
    'int16',
    'int8',
    'field_path',
    'utf8_string',
    'ansi_string',
    'unknown',
  };
  if (leafTypes.contains(type)) {
    _authoringExactFields(json, const <String>{'type'}, context);
    return <String, Object?>{'type': type};
  }
  switch (type) {
    case 'array':
    case 'optional':
      _authoringExactFields(json, const <String>{'type', 'inner'}, context);
      return <String, Object?>{
        'type': type,
        'inner': _dataAssetWireType(json['inner'], '$context inner', depth + 1),
      };
    case 'struct':
      _authoringExactFields(json, const <String>{'type', 'name'}, context);
      return <String, Object?>{
        'type': type,
        'name': _dataAssetString(
          json,
          'name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
      };
    case 'map':
      _authoringExactFields(json, const <String>{
        'type',
        'key',
        'value',
      }, context);
      return <String, Object?>{
        'type': type,
        'key': _dataAssetWireType(json['key'], '$context key', depth + 1),
        'value': _dataAssetWireType(json['value'], '$context value', depth + 1),
      };
    case 'set':
      _authoringExactFields(json, const <String>{'type', 'key'}, context);
      return <String, Object?>{
        'type': type,
        'key': _dataAssetWireType(json['key'], '$context key', depth + 1),
      };
    case 'enum':
      _authoringExactFields(json, const <String>{
        'type',
        'inner',
        'name',
      }, context);
      return <String, Object?>{
        'type': type,
        'inner': _dataAssetWireType(json['inner'], '$context inner', depth + 1),
        'name': _dataAssetString(
          json,
          'name',
          context,
          maxBytes: _maxAuthoringRevision3DataAssetManifestStringBytes,
        ),
      };
    default:
      throw FormatException('authoring $context type is unsupported');
  }
}

int _dataAssetFixedKindWidth(String kind, String context) => switch (kind) {
  'byte' || 'bool' || 'int8' => 1,
  'uint16' || 'int16' => 2,
  'int32' || 'float32' || 'package_index' || 'uint32' => 4,
  'fname' || 'float64' || 'uint64' || 'int64' => 8,
  'linear_color_f32x4' => 16,
  'vector4_f64x4' => 32,
  _ => throw FormatException(
    'authoring $context fixed wire kind is unsupported',
  ),
};

Map<String, Object?> _dataAssetObject(Object? raw, String context) =>
    _authoringRequiredObject(raw, context);

List<Object?> _dataAssetList(
  Object? raw,
  String context, {
  int minLength = 0,
  required int maxLength,
}) {
  if (raw is! List || raw.length < minLength || raw.length > maxLength) {
    throw FormatException('authoring $context is not a bounded list');
  }
  return raw.cast<Object?>();
}

String _dataAssetString(
  Map<String, Object?> json,
  String field,
  String context, {
  required int maxBytes,
}) {
  final value = json[field];
  if (value is! String || value.isEmpty) {
    throw FormatException('authoring $context field $field is not a string');
  }
  try {
    _authoringRevision3RequestString(value, field, maxBytes);
  } on ArgumentError {
    throw FormatException(
      'authoring $context field $field is not bounded UTF-8',
    );
  }
  return value;
}

int _dataAssetInt(
  Map<String, Object?> json,
  String field,
  String context, {
  int min = 0,
  int max = _maxAuthoringSignedJsonInteger,
}) {
  final value = json[field];
  if (value is! int || value < min || value > max) {
    throw FormatException(
      'authoring $context field $field is outside the signed integer range',
    );
  }
  return value;
}

String _dataAssetSha256(
  Map<String, Object?> json,
  String field,
  String context,
) {
  final value = _dataAssetString(json, field, context, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw FormatException('authoring $context field $field is not SHA-256');
  }
  return value;
}

String _dataAssetEntityId(
  Map<String, Object?> json,
  String field,
  String context,
) {
  final value = _dataAssetString(json, field, context, maxBytes: 32);
  if (!_authoringEntityIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw FormatException(
      'authoring $context field $field is not an entity ID',
    );
  }
  return value;
}

void _dataAssetRequireHex(String value, int length, String context) {
  if (value.length != length ||
      value.codeUnits.any(
        (unit) =>
            !((unit >= 0x30 && unit <= 0x39) || (unit >= 0x61 && unit <= 0x66)),
      )) {
    throw FormatException('authoring $context is not canonical lowercase hex');
  }
}

void _dataAssetRequireTargetPath(String value, String context) {
  try {
    _authoringRevision3DataAssetTargetPath(value, 'targetPath');
  } on ArgumentError {
    throw FormatException('authoring $context target path is invalid');
  }
}

void _dataAssetRequireCanonicalValueOrder(
  Object? root,
  String context, [
  int depth = 0,
]) {
  if (depth > 128) {
    throw FormatException('authoring $context JSON is too deeply nested');
  }
  if (root is List) {
    for (final value in root) {
      _dataAssetRequireCanonicalValueOrder(value, context, depth + 1);
    }
    return;
  }
  if (root is! Map) return;
  final keys = <String>[];
  for (final entry in root.entries) {
    if (entry.key is! String) {
      throw FormatException('authoring $context has a non-string key');
    }
    keys.add(entry.key as String);
    _dataAssetRequireCanonicalValueOrder(entry.value, context, depth + 1);
  }
  final sorted = keys.toList()..sort();
  for (var index = 0; index < keys.length; index++) {
    if (keys[index] != sorted[index]) {
      throw FormatException(
        'authoring $context JSON field order is not canonical',
      );
    }
  }
}
