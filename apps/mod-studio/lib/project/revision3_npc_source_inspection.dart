part of '../core/mod_ffi.dart';

const _maxRevision3NpcInspectionPlanBytes = 4 * 1024 * 1024;

enum AuthoringRevision3NpcInspectionScope { sourceReadinessInspectionOnly }

enum AuthoringRevision3NpcInspectionSourceStatus {
  persistedAndRegeneratedExact,
}

enum AuthoringRevision3NpcInspectionCompilerStatus { notRun }

enum AuthoringRevision3NpcInspectionBuildStatus { blocked }

enum AuthoringRevision3NpcInspectionRuntimeQualification { runtimeUnqualified }

enum AuthoringRevision3NpcInspectionSpawnStatus { notSupported }

enum AuthoringRevision3NpcInspectionPublicationStatus { notSupported }

enum AuthoringRevision3NpcInspectionDiagnosticCode {
  compilerNotRun,
  productionLoweringUnavailable,
  runtimeResidenceUnqualified,
  spawnUnavailable,
}

enum AuthoringRevision3NpcInspectionDiagnosticSeverity { warning, error }

final class AuthoringRevision3NpcInspectionRef {
  const AuthoringRevision3NpcInspectionRef._({
    required this.projectId,
    required this.id,
    required this.expectedKind,
  });

  final String projectId;
  final String id;
  final String expectedKind;

  factory AuthoringRevision3NpcInspectionRef._fromJson(
    Object? value, {
    required String expectedKind,
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _npcInspectionFields(json, const <String>[
      'project_id',
      'id',
      'expected_kind',
    ], context);
    final projectId = _npcInspectionId(json['project_id'], '$context project');
    final id = _npcInspectionId(json['id'], '$context entity');
    if (json['expected_kind'] != expectedKind) {
      throw FormatException('$context has the wrong entity kind');
    }
    return AuthoringRevision3NpcInspectionRef._(
      projectId: projectId,
      id: id,
      expectedKind: expectedKind,
    );
  }

  bool sameIdentity(AuthoringRevision3NpcInspectionRef other) =>
      projectId == other.projectId &&
      id == other.id &&
      expectedKind == other.expectedKind;
}

final class AuthoringRevision3NpcInspectionGeneration {
  const AuthoringRevision3NpcInspectionGeneration._(this.executable);

  final AuthoringDraftContentSeal executable;

  factory AuthoringRevision3NpcInspectionGeneration._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _npcInspectionFields(json, const <String>['executable'], context);
    return AuthoringRevision3NpcInspectionGeneration._(
      _npcInspectionSeal(json['executable'], '$context executable'),
    );
  }

  bool sameAs(AuthoringRevision3NpcInspectionGeneration other) =>
      _npcInspectionSameSeal(executable, other.executable);
}

final class AuthoringRevision3NpcInspectionParent {
  const AuthoringRevision3NpcInspectionParent._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
    required this.canonicalSelector,
    required this.runtimeClass,
  });

  final AuthoringRevision3NpcInspectionGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;
  final String canonicalSelector;
  final String runtimeClass;

  factory AuthoringRevision3NpcInspectionParent._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _npcInspectionFields(json, const <String>[
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_class',
    ], context);
    final runtimeClass = _npcInspectionString(
      json['runtime_class'],
      '$context runtime class',
      maxBytes: 1024,
    );
    _authoringDraftValidateIdentifier(runtimeClass, 'runtime_class');
    final catalogLayer = _npcInspectionString(
      json['catalog_layer'],
      '$context catalog layer',
      maxBytes: 128,
    );
    final canonicalSelector = _npcInspectionString(
      json['canonical_selector'],
      '$context selector',
      maxBytes: 96,
    );
    if (!_npcInspectionCanonicalCatalogLayer(catalogLayer) ||
        !_npcInspectionCanonicalSelector(canonicalSelector)) {
      throw FormatException('$context provenance is not canonical');
    }
    return AuthoringRevision3NpcInspectionParent._(
      generation: AuthoringRevision3NpcInspectionGeneration._fromJson(
        json['generation'],
        '$context generation',
      ),
      sourceSeal: _npcInspectionSeal(json['source_seal'], '$context source'),
      catalogLayer: catalogLayer,
      canonicalSelector: canonicalSelector,
      runtimeClass: runtimeClass,
    );
  }
}

final class AuthoringRevision3NpcInspectionInput {
  const AuthoringRevision3NpcInspectionInput._({
    required this.canonicalJson,
    required this.target,
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCharacterDefinition,
    required this.parentAiAgentConfig,
    required this.parentSpawnDefinition,
  });

  final String canonicalJson;
  final AuthoringRevision3NpcInspectionGeneration target;
  final String moduleNamespace;
  final String uniqueName;
  final AuthoringRevision3NpcInspectionParent parentCharacterDefinition;
  final AuthoringRevision3NpcInspectionParent parentAiAgentConfig;
  final AuthoringRevision3NpcInspectionParent parentSpawnDefinition;

  factory AuthoringRevision3NpcInspectionInput._fromJson(Object? value) {
    final json = _authoringRequiredObject(value, 'NPC inspection input');
    _npcInspectionFields(json, const <String>[
      'target',
      'module_namespace',
      'unique_name',
      'parent_character_definition',
      'parent_ai_agent_config',
      'parent_spawn_definition',
    ], 'NPC inspection input');
    final moduleNamespace = _npcInspectionString(
      json['module_namespace'],
      'NPC inspection module namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final uniqueName = _npcInspectionString(
      json['unique_name'],
      'NPC inspection unique name',
      maxBytes: 64,
    );
    _authoringDraftValidateIdentifier(uniqueName, 'unique_name', maxBytes: 64);
    final input = AuthoringRevision3NpcInspectionInput._(
      canonicalJson: jsonEncode(json),
      target: AuthoringRevision3NpcInspectionGeneration._fromJson(
        json['target'],
        'NPC inspection target',
      ),
      moduleNamespace: moduleNamespace,
      uniqueName: uniqueName,
      parentCharacterDefinition:
          AuthoringRevision3NpcInspectionParent._fromJson(
            json['parent_character_definition'],
            'NPC inspection CharacterDefinition parent',
          ),
      parentAiAgentConfig: AuthoringRevision3NpcInspectionParent._fromJson(
        json['parent_ai_agent_config'],
        'NPC inspection AIAgentConfig parent',
      ),
      parentSpawnDefinition: AuthoringRevision3NpcInspectionParent._fromJson(
        json['parent_spawn_definition'],
        'NPC inspection SpawnDefinition parent',
      ),
    );
    for (final parent in <AuthoringRevision3NpcInspectionParent>[
      input.parentCharacterDefinition,
      input.parentAiAgentConfig,
      input.parentSpawnDefinition,
    ]) {
      if (!parent.generation.sameAs(input.target)) {
        throw const FormatException(
          'NPC inspection parent generation does not match the target',
        );
      }
    }
    return input;
  }

  String? get knownParentLabel {
    if (target.executable.byteLength !=
            _authoringRevision3NpcExecutableByteLengthV1 ||
        target.executable.sha256 != _authoringRevision3NpcExecutableSha256V1) {
      return null;
    }
    for (final entry in _authoringRevision3NpcSelectionEvidenceV1.entries) {
      final evidence = entry.value;
      if (_npcInspectionKnownParent(
            parentCharacterDefinition,
            entry.key,
            evidence.characterDefinition,
          ) &&
          _npcInspectionKnownParent(
            parentAiAgentConfig,
            entry.key,
            evidence.aiAgentConfig,
          ) &&
          _npcInspectionKnownParent(
            parentSpawnDefinition,
            entry.key,
            evidence.spawnDefinition,
          )) {
        return switch (entry.key) {
          'g1r:npc:om_grd_asghan_263' => 'Asghan',
          'g1r:npc:om_stt_viper_302' => 'Viper',
          _ => null,
        };
      }
    }
    return null;
  }
}

final class AuthoringRevision3NpcInspectionEntity {
  const AuthoringRevision3NpcInspectionEntity._({
    required this.reference,
    required this.entityRevision,
    required this.displayName,
    required this.authoredRuntimeId,
    required this.input,
    required this.inputSeal,
    required this.scriptModule,
  });

  final AuthoringRevision3NpcInspectionRef reference;
  final int entityRevision;
  final String displayName;
  final String authoredRuntimeId;
  final AuthoringRevision3NpcInspectionInput input;
  final AuthoringDraftContentSeal inputSeal;
  final AuthoringRevision3NpcInspectionRef scriptModule;

  factory AuthoringRevision3NpcInspectionEntity._fromJson(Object? value) {
    final json = _authoringRequiredObject(value, 'NPC inspection entity');
    _npcInspectionFields(json, const <String>[
      'reference',
      'entity_revision',
      'display_name',
      'origin',
      'generator_id',
      'generator_version',
      'input',
      'input_seal',
      'script_module',
    ], 'NPC inspection entity');
    final input = AuthoringRevision3NpcInspectionInput._fromJson(json['input']);
    final inputSeal = _npcInspectionSeal(
      json['input_seal'],
      'NPC inspection input',
      maxByteLength: _maxAuthoringRevision3NpcSourceBytes,
    );
    final inputBytes = utf8.encode(input.canonicalJson);
    if (inputSeal.byteLength != inputBytes.length ||
        inputSeal.sha256 != crypto.sha256.convert(inputBytes).toString()) {
      throw const FormatException('NPC inspection input seal is invalid');
    }
    final origin = _authoringRequiredObject(
      json['origin'],
      'NPC inspection entity origin',
    );
    _npcInspectionFields(origin, const <String>[
      'type',
      'authored_runtime_id',
    ], 'NPC inspection entity origin');
    final authoredRuntimeId = _npcInspectionString(
      origin['authored_runtime_id'],
      'NPC inspection authored runtime ID',
      maxBytes: 64,
    );
    if (origin['type'] != 'new' ||
        authoredRuntimeId != input.uniqueName ||
        json['generator_id'] != _authoringRevision3NpcGeneratorId ||
        json['generator_version'] != _authoringRevision3NpcGeneratorVersion) {
      throw const FormatException(
        'NPC inspection entity provenance is unsupported',
      );
    }
    return AuthoringRevision3NpcInspectionEntity._(
      reference: AuthoringRevision3NpcInspectionRef._fromJson(
        json['reference'],
        expectedKind: 'npc_draft',
        context: 'NPC inspection entity reference',
      ),
      entityRevision: _npcInspectionInt(
        json['entity_revision'],
        'NPC inspection entity revision',
      ),
      displayName: _authoringRevision3NpcDisplayName(<String, Object?>{
        'display_name': json['display_name'],
      }, 'display_name'),
      authoredRuntimeId: authoredRuntimeId,
      input: input,
      inputSeal: inputSeal,
      scriptModule: AuthoringRevision3NpcInspectionRef._fromJson(
        json['script_module'],
        expectedKind: 'script_module',
        context: 'NPC inspection ScriptModule reference',
      ),
    );
  }
}

final class AuthoringRevision3NpcInspectionGeneratedModule {
  const AuthoringRevision3NpcInspectionGeneratedModule._({
    required this.owner,
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
  });

  final AuthoringRevision3NpcInspectionRef owner;
  final String moduleNamespace;
  final String moduleRelativePath;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;

  factory AuthoringRevision3NpcInspectionGeneratedModule._fromJson(
    Object? value,
    AuthoringRevision3NpcInspectionInput input,
  ) {
    final json = _authoringRequiredObject(
      value,
      'NPC inspection generated module',
    );
    _npcInspectionFields(json, const <String>[
      'generator_id',
      'generator_version',
      'owner',
      'module_namespace',
      'module_relative_path',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    ], 'NPC inspection generated module');
    if (json['generator_id'] != _authoringRevision3NpcGeneratorId ||
        json['generator_version'] != _authoringRevision3NpcGeneratorVersion) {
      throw const FormatException(
        'NPC inspection generated module uses an unsupported generator',
      );
    }
    final namespace = _npcInspectionString(
      json['module_namespace'],
      'NPC inspection generated namespace',
      maxBytes: 255,
    );
    final path = _npcInspectionString(
      json['module_relative_path'],
      'NPC inspection generated path',
      maxBytes: 258,
    );
    if (namespace != input.moduleNamespace ||
        path != '${namespace.replaceAll('.', '/')}.as') {
      throw const FormatException('NPC inspection generated path is invalid');
    }
    final source = _npcInspectionString(
      json['source'],
      'NPC inspection generated source',
      maxBytes: _maxAuthoringRevision3NpcSourceBytes,
    );
    final expectedSource = _authoringRevision3NpcGeneratedSource(
      uniqueName: input.uniqueName,
      parentCharacter: input.parentCharacterDefinition.runtimeClass,
      parentAgent: input.parentAiAgentConfig.runtimeClass,
      parentSpawn: input.parentSpawnDefinition.runtimeClass,
    );
    final sourceSha256 = _npcInspectionSha(json['source_sha256'], 'source');
    final fingerprint = _npcInspectionSha(
      json['input_fingerprint'],
      'input fingerprint',
    );
    final status = _authoringRequiredObject(
      json['status'],
      'NPC inspection generated status',
    );
    _npcInspectionFields(status, const <String>[
      'authoring',
      'runtime',
    ], 'NPC inspection generated status');
    if (source != expectedSource ||
        sourceSha256 != crypto.sha256.convert(utf8.encode(source)).toString() ||
        fingerprint !=
            _authoringRevision3NpcInputFingerprint(
              jsonDecode(input.canonicalJson) as Map<String, Object?>,
            ) ||
        status['authoring'] != 'offline_draft' ||
        status['runtime'] != 'runtime_unqualified') {
      throw const FormatException(
        'NPC inspection regenerated module is not exact',
      );
    }
    return AuthoringRevision3NpcInspectionGeneratedModule._(
      owner: AuthoringRevision3NpcInspectionRef._fromJson(
        json['owner'],
        expectedKind: 'npc_draft',
        context: 'NPC inspection generated owner',
      ),
      moduleNamespace: namespace,
      moduleRelativePath: path,
      source: source,
      sourceSha256: sourceSha256,
      inputFingerprint: fingerprint,
    );
  }
}

final class AuthoringRevision3NpcInspectionModule {
  const AuthoringRevision3NpcInspectionModule._({
    required this.reference,
    required this.entityRevision,
    required this.displayName,
    required this.persistedSource,
    required this.generated,
  });

  final AuthoringRevision3NpcInspectionRef reference;
  final int entityRevision;
  final String displayName;
  final AuthoringDraftContentSeal persistedSource;
  final AuthoringRevision3NpcInspectionGeneratedModule generated;

  factory AuthoringRevision3NpcInspectionModule._fromJson(
    Object? value,
    AuthoringRevision3NpcInspectionEntity npc,
  ) {
    final json = _authoringRequiredObject(value, 'NPC inspection module');
    _npcInspectionFields(json, const <String>[
      'reference',
      'entity_revision',
      'display_name',
      'origin',
      'persisted_source',
      'generated',
    ], 'NPC inspection module');
    final reference = AuthoringRevision3NpcInspectionRef._fromJson(
      json['reference'],
      expectedKind: 'script_module',
      context: 'NPC inspection module reference',
    );
    final origin = _authoringRequiredObject(
      json['origin'],
      'NPC inspection module origin',
    );
    _npcInspectionFields(origin, const <String>[
      'type',
      'generator_id',
      'generator_version',
      'owner',
    ], 'NPC inspection module origin');
    final originOwner = AuthoringRevision3NpcInspectionRef._fromJson(
      origin['owner'],
      expectedKind: 'npc_draft',
      context: 'NPC inspection module origin owner',
    );
    final generated = AuthoringRevision3NpcInspectionGeneratedModule._fromJson(
      json['generated'],
      npc.input,
    );
    final persistedSource = _npcInspectionSeal(
      json['persisted_source'],
      'NPC inspection persisted source',
      maxByteLength: _maxAuthoringRevision3NpcSourceBytes,
    );
    if (origin['type'] != 'generated' ||
        origin['generator_id'] != _authoringRevision3NpcGeneratorId ||
        origin['generator_version'] != _authoringRevision3NpcGeneratorVersion ||
        !originOwner.sameIdentity(npc.reference) ||
        !generated.owner.sameIdentity(npc.reference) ||
        !reference.sameIdentity(npc.scriptModule) ||
        persistedSource.byteLength != utf8.encode(generated.source).length ||
        persistedSource.sha256 != generated.sourceSha256) {
      throw const FormatException('NPC inspection module binding is invalid');
    }
    return AuthoringRevision3NpcInspectionModule._(
      reference: reference,
      entityRevision: _npcInspectionInt(
        json['entity_revision'],
        'NPC inspection module revision',
      ),
      displayName: _authoringRevision3NpcDisplayName(<String, Object?>{
        'display_name': json['display_name'],
      }, 'display_name'),
      persistedSource: persistedSource,
      generated: generated,
    );
  }
}

final class AuthoringRevision3NpcInspectionDiagnostic {
  const AuthoringRevision3NpcInspectionDiagnostic._({
    required this.code,
    required this.severity,
    required this.entity,
    required this.propertyPath,
    required this.message,
  });

  final AuthoringRevision3NpcInspectionDiagnosticCode code;
  final AuthoringRevision3NpcInspectionDiagnosticSeverity severity;
  final AuthoringRevision3NpcInspectionRef entity;
  final String propertyPath;
  final String message;

  factory AuthoringRevision3NpcInspectionDiagnostic._fromJson(
    Object? value,
    int index,
    AuthoringRevision3NpcInspectionEntity npc,
    AuthoringRevision3NpcInspectionModule module,
  ) {
    final json = _authoringRequiredObject(value, 'NPC inspection diagnostic');
    _npcInspectionFields(json, const <String>[
      'code',
      'severity',
      'entity',
      'property_path',
      'message',
      'blocks_build',
    ], 'NPC inspection diagnostic');
    const codes = <String>[
      'NPC_COMPILER_NOT_RUN',
      'NPC_PRODUCTION_LOWERING_UNAVAILABLE',
      'NPC_RUNTIME_RESIDENCE_UNQUALIFIED',
      'NPC_SPAWN_UNAVAILABLE',
    ];
    const severities = <String>['warning', 'error', 'error', 'error'];
    const paths = <String>[
      'payload.data.source',
      'payload.data.script_module',
      'payload.data.script_module',
      'payload.data.input',
    ];
    const messages = <String>[
      'The exact generated NPC source was not submitted to a compiler by this read-only inspection.',
      'Production lowering for revision-3 NPC drafts is unavailable.',
      'NPC class residence, effective behavior, distinct state, and persistence are runtime-unqualified.',
      'No qualified spawn or world-placement mechanism is available for this NPC draft.',
    ];
    if (index < 0 ||
        index >= codes.length ||
        json['code'] != codes[index] ||
        json['severity'] != severities[index] ||
        json['property_path'] != paths[index] ||
        json['message'] != messages[index] ||
        json['blocks_build'] != true) {
      throw const FormatException(
        'NPC inspection diagnostics do not match the closed readiness set',
      );
    }
    final expected = index == 0 ? module.reference : npc.reference;
    final entity = AuthoringRevision3NpcInspectionRef._fromJson(
      json['entity'],
      expectedKind: expected.expectedKind,
      context: 'NPC inspection diagnostic entity',
    );
    if (!entity.sameIdentity(expected)) {
      throw const FormatException(
        'NPC inspection diagnostic addresses another entity',
      );
    }
    return AuthoringRevision3NpcInspectionDiagnostic._(
      code: AuthoringRevision3NpcInspectionDiagnosticCode.values[index],
      severity: index == 0
          ? AuthoringRevision3NpcInspectionDiagnosticSeverity.warning
          : AuthoringRevision3NpcInspectionDiagnosticSeverity.error,
      entity: entity,
      propertyPath: paths[index],
      message: messages[index],
    );
  }
}

final class AuthoringRevision3NpcSourceInspectionPlanV1 {
  const AuthoringRevision3NpcSourceInspectionPlanV1._({
    required this.canonicalJson,
    required this.projectId,
    required this.projectRevision,
    required this.target,
    required this.canonicalProject,
    required this.npc,
    required this.module,
    required this.diagnostics,
  });

  final String canonicalJson;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3NpcInspectionGeneration target;
  final AuthoringDraftContentSeal canonicalProject;
  final AuthoringRevision3NpcInspectionEntity npc;
  final AuthoringRevision3NpcInspectionModule module;
  final List<AuthoringRevision3NpcInspectionDiagnostic> diagnostics;

  String get generatedSource => module.generated.source;
  String? get knownParentLabel => npc.input.knownParentLabel;

  factory AuthoringRevision3NpcSourceInspectionPlanV1.fromCanonicalJson(
    String value,
  ) {
    final json = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 NPC source inspection plan',
    );
    if (utf8.encode(value).length > _maxRevision3NpcInspectionPlanBytes ||
        jsonEncode(json) != value) {
      throw const FormatException(
        'NPC inspection plan is not bounded canonical JSON',
      );
    }
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 NPC source inspection plan',
    );
    _npcInspectionFields(json, const <String>[
      'format',
      'schema_revision',
      'scope',
      'source_status',
      'compiler_status',
      'build_status',
      'runtime_qualification',
      'spawn_status',
      'publication_status',
      'provenance',
      'npc',
      'module',
      'diagnostics',
    ], 'NPC inspection plan');
    if (json['format'] != 'revision3_npc_source_inspection_plan' ||
        json['schema_revision'] != 1 ||
        json['scope'] != 'source_readiness_inspection_only' ||
        json['source_status'] != 'persisted_and_regenerated_exact' ||
        json['compiler_status'] != 'not_run' ||
        json['build_status'] != 'blocked' ||
        json['runtime_qualification'] != 'runtime_unqualified' ||
        json['spawn_status'] != 'not_supported' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException('NPC inspection plan widens authority');
    }
    final provenance = _authoringRequiredObject(
      json['provenance'],
      'NPC inspection provenance',
    );
    _npcInspectionFields(provenance, const <String>[
      'project_id',
      'project_revision',
      'target',
      'canonical_project',
    ], 'NPC inspection provenance');
    final projectId = _npcInspectionId(
      provenance['project_id'],
      'NPC inspection project',
    );
    final projectRevision = _npcInspectionInt(
      provenance['project_revision'],
      'NPC inspection project revision',
    );
    final target = AuthoringRevision3NpcInspectionGeneration._fromJson(
      provenance['target'],
      'NPC inspection provenance target',
    );
    final npc = AuthoringRevision3NpcInspectionEntity._fromJson(json['npc']);
    final module = AuthoringRevision3NpcInspectionModule._fromJson(
      json['module'],
      npc,
    );
    final rawDiagnostics = json['diagnostics'];
    if (rawDiagnostics is! List || rawDiagnostics.length != 4) {
      throw const FormatException(
        'NPC inspection plan must contain four readiness diagnostics',
      );
    }
    final diagnostics = <AuthoringRevision3NpcInspectionDiagnostic>[
      for (var index = 0; index < rawDiagnostics.length; index++)
        AuthoringRevision3NpcInspectionDiagnostic._fromJson(
          rawDiagnostics[index],
          index,
          npc,
          module,
        ),
    ];
    if (npc.reference.projectId != projectId ||
        module.reference.projectId != projectId ||
        module.reference.id == npc.reference.id ||
        !npc.input.target.sameAs(target)) {
      throw const FormatException(
        'NPC inspection plan project or target binding disagrees',
      );
    }
    return AuthoringRevision3NpcSourceInspectionPlanV1._(
      canonicalJson: value,
      projectId: projectId,
      projectRevision: projectRevision,
      target: target,
      canonicalProject: _npcInspectionSeal(
        provenance['canonical_project'],
        'NPC inspection canonical project',
        maxByteLength: _maxAuthoringProjectJsonBytes,
      ),
      npc: npc,
      module: module,
      diagnostics: List.unmodifiable(diagnostics),
    );
  }
}

final class AuthoringRevision3NpcSourceInspectionResult {
  const AuthoringRevision3NpcSourceInspectionResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.projectSeal,
    required this.npcId,
    required this.planJson,
    required this.planSeal,
    required this.plan,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final AuthoringDraftContentSeal projectSeal;
  final String npcId;
  final String planJson;
  final AuthoringDraftContentSeal planSeal;
  final AuthoringRevision3NpcSourceInspectionPlanV1 plan;

  factory AuthoringRevision3NpcSourceInspectionResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String requestedNpcId,
  }) {
    _npcInspectionFields(json, const <String>[
      'build_status',
      'compiler_status',
      'head_json',
      'npc_id',
      'ok',
      'outcome',
      'plan_json',
      'plan_seal',
      'project_id',
      'project_revision',
      'project_seal',
      'publication_status',
      'runtime_qualification',
      'scope',
      'source_status',
      'spawn_status',
    ], 'NPC inspection response');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 NPC source inspection response',
    );
    if (json['ok'] != true ||
        json['outcome'] != 'inspection_only' ||
        json['scope'] != 'source_readiness_inspection_only' ||
        json['source_status'] != 'persisted_and_regenerated_exact' ||
        json['compiler_status'] != 'not_run' ||
        json['build_status'] != 'blocked' ||
        json['runtime_qualification'] != 'runtime_unqualified' ||
        json['spawn_status'] != 'not_supported' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException('NPC inspection response widens authority');
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _npcInspectionString(
        json['head_json'],
        'NPC inspection head',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _npcInspectionId(
      json['project_id'],
      'NPC inspection response project',
    );
    final projectRevision = _npcInspectionInt(
      json['project_revision'],
      'NPC inspection response revision',
    );
    final npcId = _npcInspectionId(
      json['npc_id'],
      'NPC inspection response NPC',
    );
    final planJson = _npcInspectionString(
      json['plan_json'],
      'NPC inspection plan JSON',
      maxBytes: _maxRevision3NpcInspectionPlanBytes,
    );
    final plan = AuthoringRevision3NpcSourceInspectionPlanV1.fromCanonicalJson(
      planJson,
    );
    final planSeal = _npcInspectionSeal(
      json['plan_seal'],
      'NPC inspection plan',
      maxByteLength: _maxRevision3NpcInspectionPlanBytes,
    );
    final planBytes = utf8.encode(planJson);
    final projectSeal = _npcInspectionSeal(
      json['project_seal'],
      'NPC inspection project',
      maxByteLength: _maxAuthoringProjectJsonBytes,
    );
    if (head.canonicalJson != expectedHead.canonicalJson ||
        npcId != requestedNpcId ||
        projectId != plan.projectId ||
        projectRevision != plan.projectRevision ||
        !_npcInspectionSameSeal(projectSeal, plan.canonicalProject) ||
        npcId != plan.npc.reference.id ||
        planSeal.byteLength != planBytes.length ||
        planSeal.sha256 != crypto.sha256.convert(planBytes).toString()) {
      throw const FormatException(
        'NPC inspection response exact-current bindings disagree',
      );
    }
    return AuthoringRevision3NpcSourceInspectionResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      projectSeal: projectSeal,
      npcId: npcId,
      planJson: planJson,
      planSeal: planSeal,
      plan: plan,
    );
  }
}

void _npcInspectionFields(
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

String _npcInspectionString(
  Object? value,
  String context, {
  required int maxBytes,
}) {
  if (value is! String) throw FormatException('$context is not a string');
  try {
    _authoringDraftRequestString(value, context, maxBytes);
  } on ArgumentError {
    throw FormatException('$context is not bounded UTF-8');
  }
  return value;
}

String _npcInspectionId(Object? value, String context) {
  final id = _authoringEntityId(
    _npcInspectionString(value, context, maxBytes: 32),
    context,
  );
  if (id == '00000000000000000000000000000000') {
    throw FormatException('$context must not be zero');
  }
  return id;
}

int _npcInspectionInt(Object? value, String context) {
  if (value is! int || value < 0 || value > _maxAuthoringSignedJsonInteger) {
    throw FormatException('$context is outside the signed wire domain');
  }
  return value;
}

String _npcInspectionSha(Object? value, String context) {
  final sha = _npcInspectionString(value, context, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha)) {
    throw FormatException('$context is not a SHA-256');
  }
  return sha;
}

AuthoringDraftContentSeal _npcInspectionSeal(
  Object? value,
  String context, {
  int? maxByteLength,
}) {
  final json = _authoringRequiredObject(value, '$context seal');
  _npcInspectionFields(json, const <String>[
    'byte_len',
    'sha256',
  ], '$context seal');
  final length = _npcInspectionInt(json['byte_len'], '$context byte length');
  if (length == 0 || (maxByteLength != null && length > maxByteLength)) {
    throw FormatException('$context seal has an invalid byte length');
  }
  _npcInspectionSha(json['sha256'], '$context SHA-256');
  return AuthoringDraftContentSeal.fromJson(json);
}

bool _npcInspectionSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _npcInspectionKnownParent(
  AuthoringRevision3NpcInspectionParent parent,
  String catalogId,
  _AuthoringRevision3NpcParentEvidence evidence,
) =>
    parent.catalogLayer == _authoringRevision3NpcCatalogLayer &&
    parent.canonicalSelector ==
        _authoringRevision3NpcAuthoringSelector(catalogId, evidence.role) &&
    parent.runtimeClass == evidence.runtimeClass &&
    parent.sourceSeal.byteLength == evidence.sourceByteLength &&
    parent.sourceSeal.sha256 == evidence.sourceSha256;

bool _npcInspectionCanonicalCatalogLayer(String value) {
  if (value.isEmpty || value.length > 128) return false;
  var previousSeparator = true;
  for (final byte in value.codeUnits) {
    final separator = byte == 0x2e || byte == 0x2d || byte == 0x5f;
    final lowercase = byte >= 0x61 && byte <= 0x7a;
    final digit = byte >= 0x30 && byte <= 0x39;
    if ((!lowercase && !digit && !separator) ||
        (separator && previousSeparator)) {
      return false;
    }
    previousSeparator = separator;
  }
  return !previousSeparator;
}

bool _npcInspectionCanonicalSelector(String value) {
  if (value.isEmpty || value.length > 96 || value.startsWith('__')) {
    return false;
  }
  final units = value.codeUnits;
  bool letter(int byte) =>
      (byte >= 0x41 && byte <= 0x5a) || (byte >= 0x61 && byte <= 0x7a);
  bool digit(int byte) => byte >= 0x30 && byte <= 0x39;
  if (!letter(units.first) && units.first != 0x5f) return false;
  return units
      .skip(1)
      .every((byte) => letter(byte) || digit(byte) || byte == 0x5f);
}
