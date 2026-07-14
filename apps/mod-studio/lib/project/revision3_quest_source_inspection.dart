part of '../core/mod_ffi.dart';

const _maxRevision3QuestSourceInspectionPlanBytes = 4 * 1024 * 1024;
const _maxRevision3QuestSourceInspectionSourceBytes = 1024 * 1024;
const _maxRevision3QuestSourceInspectionPriorQuests = 14285;
const _maxRevision3QuestSourceInspectionCollisionArtifactBytes =
    24 * 1024 * 1024;
const _maxRevision3QuestSourceInspectionModuleNamespaceBytes = 255;

enum AuthoringRevision3QuestInspectionScope { sourceInspectionOnly }

enum AuthoringRevision3QuestInspectionBuildStatus { blocked }

enum AuthoringRevision3QuestInspectionRuntimeQualification {
  runtimeUnqualified,
}

enum AuthoringRevision3QuestInspectionPublicationStatus { notSupported }

enum AuthoringRevision3QuestInspectionEntityKind { questDraft, scriptModule }

enum AuthoringRevision3QuestInspectionAuthoringStatus { offlineDraft }

/// One exact project-qualified reference retained by an inspection-only plan.
final class AuthoringRevision3QuestInspectionTypedRef {
  const AuthoringRevision3QuestInspectionTypedRef._({
    required this.projectId,
    required this.id,
    required this.expectedKind,
  });

  final String projectId;
  final String id;
  final AuthoringRevision3QuestInspectionEntityKind expectedKind;

  factory AuthoringRevision3QuestInspectionTypedRef._fromJson(
    Object? value, {
    required AuthoringRevision3QuestInspectionEntityKind expectedKind,
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const <String>{
      'project_id',
      'id',
      'expected_kind',
    }, context);
    _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
      'project_id',
      'id',
      'expected_kind',
    ], context);
    final projectId = _authoringRevision3QuestSourceInspectionEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      '$context.project_id',
    );
    final id = _authoringRevision3QuestSourceInspectionEntityId(
      _authoringRequiredString(json, 'id', maxBytes: 32),
      '$context.id',
    );
    final expectedWire = switch (expectedKind) {
      AuthoringRevision3QuestInspectionEntityKind.questDraft => 'quest_draft',
      AuthoringRevision3QuestInspectionEntityKind.scriptModule =>
        'script_module',
    };
    if (json['expected_kind'] != expectedWire) {
      throw FormatException(
        'authoring revision-3 Quest inspection $context has the wrong entity kind',
      );
    }
    return AuthoringRevision3QuestInspectionTypedRef._(
      projectId: projectId,
      id: id,
      expectedKind: expectedKind,
    );
  }

  bool _sameIdentity(AuthoringRevision3QuestInspectionTypedRef other) =>
      projectId == other.projectId &&
      id == other.id &&
      expectedKind == other.expectedKind;
}

/// Exact deterministic ScriptModule exposed for source inspection only.
final class AuthoringRevision3QuestInspectionGeneratedModule {
  const AuthoringRevision3QuestInspectionGeneratedModule._({
    required this.generatorId,
    required this.generatorVersion,
    required this.owner,
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
    required this.authoringStatus,
    required this.runtimeStatus,
  });

  final String generatorId;
  final int generatorVersion;
  final AuthoringRevision3QuestInspectionTypedRef owner;
  final String moduleNamespace;
  final String moduleRelativePath;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;
  final AuthoringRevision3QuestInspectionAuthoringStatus authoringStatus;
  final AuthoringRevision3QuestInspectionRuntimeQualification runtimeStatus;

  factory AuthoringRevision3QuestInspectionGeneratedModule._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Quest inspection generated module',
    );
    _authoringExactFields(json, const <String>{
      'generator_id',
      'generator_version',
      'owner',
      'module_namespace',
      'module_relative_path',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    }, 'revision-3 Quest inspection generated module');
    _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
      'generator_id',
      'generator_version',
      'owner',
      'module_namespace',
      'module_relative_path',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    ], 'revision-3 Quest inspection generated module');
    final generatorId = _authoringRequiredString(
      json,
      'generator_id',
      maxBytes: 256,
    );
    if (generatorId != _authoringRevision3QuestGeneratorId) {
      throw const FormatException(
        'authoring revision-3 Quest inspection generator is unsupported',
      );
    }
    final generatorVersion = _authoringRequiredInt(
      json,
      'generator_version',
      min: _authoringRevision3QuestGeneratorVersion,
      max: _authoringRevision3SemanticQuestGeneratorVersion,
    );
    if (generatorVersion != _authoringRevision3QuestGeneratorVersion &&
        generatorVersion !=
            _authoringRevision3MultiObjectiveQuestGeneratorVersion &&
        generatorVersion != _authoringRevision3SemanticQuestGeneratorVersion) {
      throw const FormatException(
        'authoring revision-3 Quest inspection generator version is unsupported',
      );
    }
    final owner = AuthoringRevision3QuestInspectionTypedRef._fromJson(
      json['owner'],
      expectedKind: AuthoringRevision3QuestInspectionEntityKind.questDraft,
      context: 'revision-3 Quest inspection generated owner',
    );
    final moduleNamespace = _authoringRequiredString(
      json,
      'module_namespace',
      maxBytes: _maxRevision3QuestSourceInspectionModuleNamespaceBytes,
    );
    final moduleRelativePath = _authoringRequiredString(
      json,
      'module_relative_path',
      maxBytes: _maxRevision3QuestSourceInspectionModuleNamespaceBytes + 3,
    );
    if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
      throw const FormatException(
        'authoring revision-3 Quest inspection module path does not match its namespace',
      );
    }
    final source = _authoringRequiredString(
      json,
      'source',
      maxBytes: _maxRevision3QuestSourceInspectionSourceBytes,
    );
    final sourceSha256 = _authoringRevision3QuestSourceInspectionSha256(
      json,
      'source_sha256',
    );
    if (crypto.sha256.convert(utf8.encode(source)).toString() != sourceSha256) {
      throw const FormatException(
        'authoring revision-3 Quest inspection generated source seal is invalid',
      );
    }
    final inputFingerprint = _authoringRevision3QuestSourceInspectionSha256(
      json,
      'input_fingerprint',
    );
    final status = _authoringRequiredObject(
      json['status'],
      'revision-3 Quest inspection generated status',
    );
    _authoringExactFields(status, const <String>{
      'authoring',
      'runtime',
    }, 'revision-3 Quest inspection generated status');
    _authoringRevision3QuestSourceInspectionFieldOrder(status, const <String>[
      'authoring',
      'runtime',
    ], 'revision-3 Quest inspection generated status');
    if (status['authoring'] != 'offline_draft' ||
        status['runtime'] != 'runtime_unqualified') {
      throw const FormatException(
        'authoring revision-3 Quest inspection generated module overstates authority',
      );
    }
    return AuthoringRevision3QuestInspectionGeneratedModule._(
      generatorId: generatorId,
      generatorVersion: generatorVersion,
      owner: owner,
      moduleNamespace: moduleNamespace,
      moduleRelativePath: moduleRelativePath,
      source: source,
      sourceSha256: sourceSha256,
      inputFingerprint: inputFingerprint,
      authoringStatus:
          AuthoringRevision3QuestInspectionAuthoringStatus.offlineDraft,
      runtimeStatus: AuthoringRevision3QuestInspectionRuntimeQualification
          .runtimeUnqualified,
    );
  }
}

/// Source-bearing Quest/ScriptModule pair in one inspection plan.
final class AuthoringRevision3QuestInspectionModule {
  const AuthoringRevision3QuestInspectionModule._({
    required this.quest,
    required this.scriptModule,
    required this.draftInput,
    required this.persistedSource,
    required this.generated,
  });

  final AuthoringRevision3QuestInspectionTypedRef quest;
  final AuthoringRevision3QuestInspectionTypedRef scriptModule;
  final AuthoringDraftContentSeal draftInput;
  final AuthoringDraftContentSeal persistedSource;
  final AuthoringRevision3QuestInspectionGeneratedModule generated;

  factory AuthoringRevision3QuestInspectionModule._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Quest inspection module',
    );
    _authoringExactFields(json, const <String>{
      'quest',
      'script_module',
      'draft_input',
      'persisted_source',
      'generated',
    }, 'revision-3 Quest inspection module');
    _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
      'quest',
      'script_module',
      'draft_input',
      'persisted_source',
      'generated',
    ], 'revision-3 Quest inspection module');
    final quest = AuthoringRevision3QuestInspectionTypedRef._fromJson(
      json['quest'],
      expectedKind: AuthoringRevision3QuestInspectionEntityKind.questDraft,
      context: 'revision-3 Quest inspection Quest ref',
    );
    final scriptModule = AuthoringRevision3QuestInspectionTypedRef._fromJson(
      json['script_module'],
      expectedKind: AuthoringRevision3QuestInspectionEntityKind.scriptModule,
      context: 'revision-3 Quest inspection ScriptModule ref',
    );
    final draftInput = _authoringRevision3QuestSourceInspectionSeal(
      json['draft_input'],
      'revision-3 Quest inspection draft input',
      maxByteLength: _maxRevision3QuestSourceInspectionSourceBytes,
    );
    final persistedSource = _authoringRevision3QuestSourceInspectionSeal(
      json['persisted_source'],
      'revision-3 Quest inspection persisted source',
      maxByteLength: _maxRevision3QuestSourceInspectionSourceBytes,
    );
    final generated =
        AuthoringRevision3QuestInspectionGeneratedModule._fromJson(
          json['generated'],
        );
    final sourceBytes = utf8.encode(generated.source).length;
    if (quest.id == scriptModule.id ||
        quest.projectId != scriptModule.projectId ||
        !generated.owner._sameIdentity(quest) ||
        persistedSource.byteLength != sourceBytes ||
        persistedSource.sha256 != generated.sourceSha256) {
      throw const FormatException(
        'authoring revision-3 Quest inspection module identity or source binding is invalid',
      );
    }
    return AuthoringRevision3QuestInspectionModule._(
      quest: quest,
      scriptModule: scriptModule,
      draftInput: draftInput,
      persistedSource: persistedSource,
      generated: generated,
    );
  }
}

/// Exact Store, game-generation, and collision-evidence provenance.
final class AuthoringRevision3QuestInspectionProvenanceV3 {
  const AuthoringRevision3QuestInspectionProvenanceV3._({
    required this.projectId,
    required this.projectRevision,
    required this.targetExecutable,
    required this.canonicalProject,
    required this.collisionBasisHead,
    required this.collisionBasisProject,
    required this.collisionNonquestProject,
    required this.collisionPriorQuestCount,
    required this.collisionPriorQuestEvidence,
    required this.collisionArtifact,
    required this.collisionSource,
  });

  final String projectId;
  final int projectRevision;
  final AuthoringDraftContentSeal targetExecutable;
  final AuthoringDraftContentSeal canonicalProject;
  final AuthoringWorkingHead collisionBasisHead;
  final AuthoringDraftContentSeal collisionBasisProject;
  final AuthoringDraftContentSeal collisionNonquestProject;
  final int collisionPriorQuestCount;
  final AuthoringDraftContentSeal collisionPriorQuestEvidence;
  final AuthoringDraftContentSeal collisionArtifact;
  final AuthoringDraftContentSeal collisionSource;

  factory AuthoringRevision3QuestInspectionProvenanceV3._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Quest inspection provenance',
    );
    _authoringExactFields(json, const <String>{
      'project_id',
      'project_revision',
      'target_executable',
      'canonical_project',
      'collision_basis_head',
      'collision_basis_project',
      'collision_nonquest_project',
      'collision_prior_quest_count',
      'collision_prior_quest_evidence',
      'collision_artifact',
      'collision_source',
    }, 'revision-3 Quest inspection provenance');
    _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
      'project_id',
      'project_revision',
      'target_executable',
      'canonical_project',
      'collision_basis_head',
      'collision_basis_project',
      'collision_nonquest_project',
      'collision_prior_quest_count',
      'collision_prior_quest_evidence',
      'collision_artifact',
      'collision_source',
    ], 'revision-3 Quest inspection provenance');
    final projectId = _authoringRevision3QuestSourceInspectionEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'provenance.project_id',
    );
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final targetExecutable = _authoringRevision3QuestSourceInspectionSeal(
      json['target_executable'],
      'revision-3 Quest inspection target executable',
    );
    final canonicalProject = _authoringRevision3QuestSourceInspectionSeal(
      json['canonical_project'],
      'revision-3 Quest inspection canonical project',
      maxByteLength: _maxAuthoringProjectJsonBytes,
    );
    final collisionBasisHeadJson = _authoringRequiredObject(
      json['collision_basis_head'],
      'revision-3 Quest inspection collision basis head',
    );
    _authoringExactFields(collisionBasisHeadJson, const <String>{
      'store_format',
      'snapshot',
    }, 'revision-3 Quest inspection collision basis head');
    _authoringRevision3QuestSourceInspectionFieldOrder(
      collisionBasisHeadJson,
      const <String>['store_format', 'snapshot'],
      'revision-3 Quest inspection collision basis head',
    );
    final collisionBasisSnapshotJson = _authoringRequiredObject(
      collisionBasisHeadJson['snapshot'],
      'revision-3 Quest inspection collision basis snapshot',
    );
    _authoringExactFields(
      collisionBasisSnapshotJson,
      const <String>{'byte_len', 'sha256'},
      'revision-3 Quest inspection collision basis snapshot',
    );
    _authoringRevision3QuestSourceInspectionFieldOrder(
      collisionBasisSnapshotJson,
      const <String>['byte_len', 'sha256'],
      'revision-3 Quest inspection collision basis snapshot',
    );
    final collisionBasisHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(collisionBasisHeadJson),
    );
    final collisionBasisProject = _authoringRevision3QuestSourceInspectionSeal(
      json['collision_basis_project'],
      'revision-3 Quest inspection collision basis project',
      maxByteLength: _maxAuthoringProjectJsonBytes,
    );
    final collisionNonquestProject =
        _authoringRevision3QuestSourceInspectionSeal(
          json['collision_nonquest_project'],
          'revision-3 Quest inspection collision non-Quest project',
          maxByteLength: _maxAuthoringProjectJsonBytes,
        );
    final collisionPriorQuestCount = _authoringRequiredInt(
      json,
      'collision_prior_quest_count',
      max: _maxRevision3QuestSourceInspectionPriorQuests,
    );
    final collisionPriorQuestEvidence =
        _authoringRevision3QuestSourceInspectionSeal(
          json['collision_prior_quest_evidence'],
          'revision-3 Quest inspection collision prior-Quest evidence',
          maxByteLength: _maxAuthoringProjectJsonBytes,
        );
    final collisionArtifact = _authoringRevision3QuestSourceInspectionSeal(
      json['collision_artifact'],
      'revision-3 Quest inspection collision artifact',
      maxByteLength: _maxRevision3QuestSourceInspectionCollisionArtifactBytes,
    );
    final collisionSource = _authoringRevision3QuestSourceInspectionSeal(
      json['collision_source'],
      'revision-3 Quest inspection collision source',
      maxByteLength: _maxRevision3QuestSourceInspectionCollisionArtifactBytes,
    );
    if (collisionSource.byteLength != collisionArtifact.byteLength) {
      throw const FormatException(
        'authoring revision-3 Quest inspection collision seals disagree',
      );
    }
    return AuthoringRevision3QuestInspectionProvenanceV3._(
      projectId: projectId,
      projectRevision: projectRevision,
      targetExecutable: targetExecutable,
      canonicalProject: canonicalProject,
      collisionBasisHead: collisionBasisHead,
      collisionBasisProject: collisionBasisProject,
      collisionNonquestProject: collisionNonquestProject,
      collisionPriorQuestCount: collisionPriorQuestCount,
      collisionPriorQuestEvidence: collisionPriorQuestEvidence,
      collisionArtifact: collisionArtifact,
      collisionSource: collisionSource,
    );
  }
}

/// Canonical schema-3 plan. Its closed status fields cannot represent build,
/// runtime, deployment, mutation, or publication readiness.
final class AuthoringRevision3QuestSourceInspectionPlanV3 {
  const AuthoringRevision3QuestSourceInspectionPlanV3._({
    required this.canonicalJson,
    required this.scope,
    required this.buildStatus,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.provenance,
    required this.module,
  });

  final String canonicalJson;
  final AuthoringRevision3QuestInspectionScope scope;
  final AuthoringRevision3QuestInspectionBuildStatus buildStatus;
  final AuthoringRevision3QuestInspectionRuntimeQualification
  runtimeQualification;
  final AuthoringRevision3QuestInspectionPublicationStatus publicationStatus;
  final AuthoringRevision3QuestInspectionProvenanceV3 provenance;
  final AuthoringRevision3QuestInspectionModule module;

  String get generatedSource => module.generated.source;
  String get moduleNamespace => module.generated.moduleNamespace;
  String get moduleRelativePath => module.generated.moduleRelativePath;

  factory AuthoringRevision3QuestSourceInspectionPlanV3.fromCanonicalJson(
    String value,
  ) {
    try {
      _authoringRevision3RequestString(
        value,
        'planJson',
        _maxRevision3QuestSourceInspectionPlanBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest inspection plan is not bounded UTF-8',
      );
    }
    final json = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest source inspection plan',
    );
    _authoringExactFields(json, const <String>{
      'format',
      'schema_revision',
      'scope',
      'build_status',
      'runtime_qualification',
      'publication_status',
      'provenance',
      'module',
    }, 'revision-3 Quest source inspection plan');
    _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
      'format',
      'schema_revision',
      'scope',
      'build_status',
      'runtime_qualification',
      'publication_status',
      'provenance',
      'module',
    ], 'revision-3 Quest source inspection plan');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 Quest source inspection plan',
    );
    if (jsonEncode(json) != value) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection plan is not canonical',
      );
    }
    if (json['format'] != 'revision3_quest_source_inspection_plan' ||
        json['schema_revision'] != 3) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection plan version is unsupported',
      );
    }
    final scope = _authoringRevision3QuestInspectionScope(json['scope']);
    final buildStatus = _authoringRevision3QuestInspectionBuildStatus(
      json['build_status'],
    );
    final runtimeQualification =
        _authoringRevision3QuestInspectionRuntimeQualification(
          json['runtime_qualification'],
        );
    final publicationStatus =
        _authoringRevision3QuestInspectionPublicationStatus(
          json['publication_status'],
        );
    final provenance = AuthoringRevision3QuestInspectionProvenanceV3._fromJson(
      json['provenance'],
    );
    final module = AuthoringRevision3QuestInspectionModule._fromJson(
      json['module'],
    );
    if (module.quest.projectId != provenance.projectId ||
        module.scriptModule.projectId != provenance.projectId) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection plan project identities disagree',
      );
    }
    return AuthoringRevision3QuestSourceInspectionPlanV3._(
      canonicalJson: value,
      scope: scope,
      buildStatus: buildStatus,
      runtimeQualification: runtimeQualification,
      publicationStatus: publicationStatus,
      provenance: provenance,
      module: module,
    );
  }
}

/// Fully parsed read-only result for one exact-current Quest source check.
final class AuthoringRevision3QuestSourceInspectionResult {
  const AuthoringRevision3QuestSourceInspectionResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.projectSeal,
    required this.questId,
    required this.planJson,
    required this.planSeal,
    required this.scope,
    required this.buildStatus,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.plan,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final AuthoringDraftContentSeal projectSeal;
  final String questId;
  final String planJson;
  final AuthoringDraftContentSeal planSeal;
  final AuthoringRevision3QuestInspectionScope scope;
  final AuthoringRevision3QuestInspectionBuildStatus buildStatus;
  final AuthoringRevision3QuestInspectionRuntimeQualification
  runtimeQualification;
  final AuthoringRevision3QuestInspectionPublicationStatus publicationStatus;
  final AuthoringRevision3QuestSourceInspectionPlanV3 plan;

  String get generatedSource => plan.generatedSource;
  String get moduleNamespace => plan.moduleNamespace;
  String get moduleRelativePath => plan.moduleRelativePath;

  factory AuthoringRevision3QuestSourceInspectionResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String requestedQuestId,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'head_json',
      'project_id',
      'project_revision',
      'project_seal',
      'quest_id',
      'plan_json',
      'plan_seal',
      'scope',
      'build_status',
      'runtime_qualification',
      'publication_status',
    }, 'revision-3 Quest source inspection response');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 Quest source inspection response',
    );
    if (json['ok'] != true || json['outcome'] != 'inspection_only') {
      throw const FormatException(
        'authoring revision-3 Quest source inspection outcome is not inspection-only',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection head is not exact-current',
      );
    }
    final projectId = _authoringRevision3QuestSourceInspectionEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final projectSeal = _authoringRevision3QuestSourceInspectionSeal(
      json['project_seal'],
      'revision-3 Quest source inspection project seal',
      maxByteLength: _maxAuthoringProjectJsonBytes,
    );
    final questId = _authoringRevision3QuestSourceInspectionEntityId(
      _authoringRequiredString(json, 'quest_id', maxBytes: 32),
      'quest_id',
    );
    if (questId != requestedQuestId) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection returned another Quest',
      );
    }
    final planJson = _authoringRevision3ResponseString(
      json,
      'plan_json',
      maxBytes: _maxRevision3QuestSourceInspectionPlanBytes,
    );
    final plan =
        AuthoringRevision3QuestSourceInspectionPlanV3.fromCanonicalJson(
          planJson,
        );
    final planSeal = _authoringRevision3QuestSourceInspectionSeal(
      json['plan_seal'],
      'revision-3 Quest source inspection plan seal',
      maxByteLength: _maxRevision3QuestSourceInspectionPlanBytes,
    );
    final planBytes = utf8.encode(planJson);
    if (planSeal.byteLength != planBytes.length ||
        planSeal.sha256 != crypto.sha256.convert(planBytes).toString()) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection plan seal is invalid',
      );
    }
    final scope = _authoringRevision3QuestInspectionScope(json['scope']);
    final buildStatus = _authoringRevision3QuestInspectionBuildStatus(
      json['build_status'],
    );
    final runtimeQualification =
        _authoringRevision3QuestInspectionRuntimeQualification(
          json['runtime_qualification'],
        );
    final publicationStatus =
        _authoringRevision3QuestInspectionPublicationStatus(
          json['publication_status'],
        );
    if (projectId != plan.provenance.projectId ||
        projectRevision != plan.provenance.projectRevision ||
        !_authoringRevision3QuestSourceInspectionSameSeal(
          projectSeal,
          plan.provenance.canonicalProject,
        ) ||
        questId != plan.module.quest.id ||
        scope != plan.scope ||
        buildStatus != plan.buildStatus ||
        runtimeQualification != plan.runtimeQualification ||
        publicationStatus != plan.publicationStatus) {
      throw const FormatException(
        'authoring revision-3 Quest source inspection response bindings disagree',
      );
    }
    return AuthoringRevision3QuestSourceInspectionResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      projectSeal: projectSeal,
      questId: questId,
      planJson: planJson,
      planSeal: planSeal,
      scope: scope,
      buildStatus: buildStatus,
      runtimeQualification: runtimeQualification,
      publicationStatus: publicationStatus,
      plan: plan,
    );
  }
}

String _authoringRevision3QuestSourceInspectionEntityId(
  String value,
  String field,
) {
  final id = _authoringEntityId(value, field);
  if (id == '00000000000000000000000000000000') {
    throw FormatException(
      'authoring revision-3 Quest inspection field $field must not be zero',
    );
  }
  return id;
}

String _authoringRevision3QuestSourceInspectionSha256(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw FormatException(
      'authoring revision-3 Quest inspection field $field is not a SHA-256',
    );
  }
  return value;
}

AuthoringDraftContentSeal _authoringRevision3QuestSourceInspectionSeal(
  Object? value,
  String context, {
  int? maxByteLength,
}) {
  final json = _authoringRequiredObject(value, context);
  _authoringExactFields(json, const <String>{'byte_len', 'sha256'}, context);
  _authoringRevision3QuestSourceInspectionFieldOrder(json, const <String>[
    'byte_len',
    'sha256',
  ], context);
  final seal = AuthoringDraftContentSeal.fromJson(json);
  if (maxByteLength != null && seal.byteLength > maxByteLength) {
    throw FormatException('authoring $context exceeds its byte limit');
  }
  return seal;
}

bool _authoringRevision3QuestSourceInspectionSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

void _authoringRevision3QuestSourceInspectionFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final actual = json.keys.toList(growable: false);
  if (actual.length != expected.length) {
    throw FormatException('authoring $context has an invalid schema');
  }
  for (var index = 0; index < expected.length; index++) {
    if (actual[index] != expected[index]) {
      throw FormatException('authoring $context has non-canonical field order');
    }
  }
}

AuthoringRevision3QuestInspectionScope _authoringRevision3QuestInspectionScope(
  Object? value,
) {
  if (value != 'source_inspection_only') {
    throw const FormatException(
      'authoring revision-3 Quest inspection scope grants unsupported authority',
    );
  }
  return AuthoringRevision3QuestInspectionScope.sourceInspectionOnly;
}

AuthoringRevision3QuestInspectionBuildStatus
_authoringRevision3QuestInspectionBuildStatus(Object? value) {
  if (value != 'blocked') {
    throw const FormatException(
      'authoring revision-3 Quest inspection build status grants unsupported authority',
    );
  }
  return AuthoringRevision3QuestInspectionBuildStatus.blocked;
}

AuthoringRevision3QuestInspectionRuntimeQualification
_authoringRevision3QuestInspectionRuntimeQualification(Object? value) {
  if (value != 'runtime_unqualified') {
    throw const FormatException(
      'authoring revision-3 Quest inspection runtime status grants unsupported authority',
    );
  }
  return AuthoringRevision3QuestInspectionRuntimeQualification
      .runtimeUnqualified;
}

AuthoringRevision3QuestInspectionPublicationStatus
_authoringRevision3QuestInspectionPublicationStatus(Object? value) {
  if (value != 'not_supported') {
    throw const FormatException(
      'authoring revision-3 Quest inspection publication status grants unsupported authority',
    );
  }
  return AuthoringRevision3QuestInspectionPublicationStatus.notSupported;
}
