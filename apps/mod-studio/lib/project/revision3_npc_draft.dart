part of '../core/mod_ffi.dart';

const _authoringRevision3NpcGeneratorId =
    'gore-authoring.logical-npc-clone-draft';
const _authoringRevision3NpcGeneratorVersion = 1;
const _maxAuthoringRevision3NpcBasisRevision = 0x7ffffffffffffffe;
const _maxAuthoringRevision3NpcAppliedRevision = 0x7fffffffffffffff;
const _maxAuthoringRevision3NpcDisplayNameBytes = 256;
const _maxAuthoringRevision3NpcCatalogIdBytes = 256;
const _maxAuthoringRevision3NpcSourceBytes = 1024 * 1024;
const _authoringRevision3NpcFingerprintDomain =
    'gore-authoring.revision3.npc-draft.input-fingerprint\u0000';
const _authoringRevision3NpcSelectorDomain =
    'gore-story-catalog.authoring-selector-v1\u0000';
const _authoringRevision3NpcCatalogLayer = 'base-game.g1r.scripts';
const _authoringRevision3NpcExecutableByteLengthV1 = 171698176;
const _authoringRevision3NpcExecutableSha256V1 =
    'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5';
const _authoringRevision3NpcExecutableByteLengthV2 = 171704320;
const _authoringRevision3NpcExecutableSha256V2 =
    'b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d';
const _authoringRevision3NpcExecutableByteLengthV3 = 171787776;
const _authoringRevision3NpcExecutableSha256V3 =
    'ab2c8d9e286a437bc5343748faf40959a77e9dc7c542ff9361f1ffaeca5c811c';
const _authoringRevision3NpcExecutableByteLengthV4 = 171792384;
const _authoringRevision3NpcExecutableSha256V4 =
    '824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0';

typedef _AuthoringRevision3NpcParentEvidence = ({
  String role,
  int sourceByteLength,
  String sourceSha256,
  String runtimeClass,
});

typedef _AuthoringRevision3NpcSelectionEvidence = ({
  _AuthoringRevision3NpcParentEvidence characterDefinition,
  _AuthoringRevision3NpcParentEvidence aiAgentConfig,
  _AuthoringRevision3NpcParentEvidence spawnDefinition,
});

/// Exact closed projection of the native pinned Story-catalog rows shared by
/// every currently registered generation. A new native catalog row must be
/// reviewed and added here before its persisted parent evidence can cross the
/// strict response boundary.
const Map<String, _AuthoringRevision3NpcSelectionEvidence>
_authoringRevision3NpcSelectionEvidence = {
  'g1r:npc:om_grd_asghan_263': (
    characterDefinition: (
      role: 'character_definition',
      sourceByteLength: 460,
      sourceSha256:
          '2312e01be5dd91d043b03acbd487f310d47b99107d765ce31ad87aa77eb5723e',
      runtimeClass: 'UCharacterDefinition_Human_OM_GRD_Asghan_263',
    ),
    aiAgentConfig: (
      role: 'ai_agent_config',
      sourceByteLength: 932,
      sourceSha256:
          'b728be66667b1b220438c40c11d0881eab01f6a7cc9094ea935b90a1da36eae8',
      runtimeClass: 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
    ),
    spawnDefinition: (
      role: 'spawn_definition',
      sourceByteLength: 96033,
      sourceSha256:
          'e49a3a5f8ac2a589f40878f6f248ab8743adefeab07081754f681cb85c36b86b',
      runtimeClass: 'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
    ),
  ),
  'g1r:npc:om_stt_viper_302': (
    characterDefinition: (
      role: 'character_definition',
      sourceByteLength: 455,
      sourceSha256:
          '1a4c6caad0511154f4622722f38ec5f85cc2e12f500224f90f4e0208614e7c73',
      runtimeClass: 'UCharacterDefinition_Human_OM_STT_Viper_302',
    ),
    aiAgentConfig: (
      role: 'ai_agent_config',
      sourceByteLength: 932,
      sourceSha256:
          'dde3f35f70f23a1ae77f0768d7a947fc2fbd9deaac4b3c12a5bad4f35725220b',
      runtimeClass: 'UAIAgentConfig_Human_OM_STT_Viper_302',
    ),
    spawnDefinition: (
      role: 'spawn_definition',
      sourceByteLength: 96033,
      sourceSha256:
          'e49a3a5f8ac2a589f40878f6f248ab8743adefeab07081754f681cb85c36b86b',
      runtimeClass: 'USpawnAIAgentDefinition_OM_STT_Viper_302',
    ),
  ),
};

final _authoringRevision3NpcCatalogIdPattern = RegExp(
  r'^[a-z0-9._-]+(?::[a-z0-9._-]+){2,}$',
);

/// Bounded NPC intent. The catalog ID remains only a selector: resolved parent classes and their
/// provenance are rebuilt by native code and never accepted from this transport.
final class AuthoringRevision3NpcDraftIntentV1 {
  const AuthoringRevision3NpcDraftIntentV1._({
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCatalogId,
  });

  factory AuthoringRevision3NpcDraftIntentV1({
    required String moduleNamespace,
    required String uniqueName,
    required String parentCatalogId,
  }) => AuthoringRevision3NpcDraftIntentV1.fromJson(<String, Object?>{
    'module_namespace': moduleNamespace,
    'unique_name': uniqueName,
    'parent_catalog_id': parentCatalogId,
  });

  final String moduleNamespace;
  final String uniqueName;
  final String parentCatalogId;

  factory AuthoringRevision3NpcDraftIntentV1.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'module_namespace',
      'unique_name',
      'parent_catalog_id',
    }, 'revision-3 NPC intent');
    _authoringRevision3NpcRequireFieldOrder(json, const <String>[
      'module_namespace',
      'unique_name',
      'parent_catalog_id',
    ], 'intent');
    final moduleNamespace = _authoringRevision3NpcString(
      json,
      'module_namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final uniqueName = _authoringRevision3NpcString(
      json,
      'unique_name',
      maxBytes: 64,
    );
    _authoringDraftValidateIdentifier(uniqueName, 'unique_name', maxBytes: 64);
    final parentCatalogId = _authoringRevision3NpcString(
      json,
      'parent_catalog_id',
      maxBytes: _maxAuthoringRevision3NpcCatalogIdBytes,
    );
    if (!_authoringRevision3NpcCatalogIdPattern.hasMatch(parentCatalogId)) {
      throw const FormatException(
        'authoring revision-3 NPC parent catalog ID is not canonical',
      );
    }
    return AuthoringRevision3NpcDraftIntentV1._(
      moduleNamespace: moduleNamespace,
      uniqueName: uniqueName,
      parentCatalogId: parentCatalogId,
    );
  }

  Map<String, Object?> toJson() => <String, Object?>{
    'module_namespace': moduleNamespace,
    'unique_name': uniqueName,
    'parent_catalog_id': parentCatalogId,
  };
}

/// Exact canonical request bound to one published revision-3 project/head.
///
/// [forProject] derives the target, project ID, and revision from the same canonical project bytes
/// later sent to native code. Callers cannot accidentally combine an independent target value.
final class AuthoringRevision3NpcDraftRequestV1 {
  const AuthoringRevision3NpcDraftRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.npcId,
    required this.scriptModuleId,
    required this.displayName,
    required this.intent,
  });

  factory AuthoringRevision3NpcDraftRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String npcId,
    required String scriptModuleId,
    required String displayName,
    required AuthoringRevision3NpcDraftIntentV1 intent,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3NpcDraftRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'npc_id': npcId,
        'script_module_id': scriptModuleId,
        'display_name': displayName,
        'intent': intent.toJson(),
      }),
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String npcId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringRevision3NpcDraftIntentV1 intent;

  factory AuthoringRevision3NpcDraftRequestV1.fromCanonicalJson(String value) {
    try {
      _authoringRevision3RequestString(
        value,
        'npcRequestJson',
        _maxAuthoringRevision3NpcRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 NPC request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 NPC request',
    );
    _authoringExactFields(request, const {
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'npc_id',
      'script_module_id',
      'display_name',
      'intent',
    }, 'revision-3 NPC request');
    _authoringRevision3NpcRequireFieldOrder(request, const <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'npc_id',
      'script_module_id',
      'display_name',
      'intent',
    ], 'request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 NPC request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 NPC expected head',
        ),
      ),
    );
    final expectedTarget = _authoringRevision3NpcGeneration(
      request['expected_target'],
      'request target',
    );
    final npcId = _authoringRevision3NpcEntityId(request, 'npc_id');
    final scriptModuleId = _authoringRevision3NpcEntityId(
      request,
      'script_module_id',
    );
    if (npcId == scriptModuleId) {
      throw const FormatException(
        'authoring revision-3 NPC request entity IDs must be distinct',
      );
    }
    return AuthoringRevision3NpcDraftRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: _authoringRevision3NpcEntityId(
        request,
        'expected_project_id',
      ),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3NpcBasisRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(expectedTarget.json),
      npcId: npcId,
      scriptModuleId: scriptModuleId,
      displayName: _authoringRevision3NpcDisplayName(request, 'display_name'),
      intent: AuthoringRevision3NpcDraftIntentV1.fromJson(
        _authoringRequiredObject(
          request['intent'],
          'revision-3 NPC request intent',
        ),
      ),
    );
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'authoring revision-3 NPC request does not bind the exact current project',
      );
    }
  }
}

enum AuthoringRevision3NpcBuildStatus { blocked }

enum AuthoringRevision3NpcRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3NpcCatalogAuthority { notGranted }

enum AuthoringRevision3NpcCollisionAuthority { notGranted }

enum AuthoringRevision3NpcSourceInspection { freshNativeContextRequired }

enum AuthoringRevision3NpcNativePublicationStatus { notSupported }

/// Strict prepare-only result. A managed session may subsequently publish [head] only after its
/// independent full candidate reopen and exact fixed-head byte-CAS checks.
final class AuthoringRevision3NpcDraftPreparation {
  const AuthoringRevision3NpcDraftPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.npcId,
    required this.scriptModuleId,
    required this.displayName,
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCatalogId,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.catalogAuthority,
    required this.collisionAuthority,
    required this.sourceInspection,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String npcId;
  final String scriptModuleId;
  final String displayName;
  final String moduleNamespace;
  final String uniqueName;
  final String parentCatalogId;
  final AuthoringRevision3NpcBuildStatus buildStatus;
  final AuthoringRevision3NpcRuntimeStatus runtimeStatus;
  final AuthoringRevision3NpcCatalogAuthority catalogAuthority;
  final AuthoringRevision3NpcCollisionAuthority collisionAuthority;
  final AuthoringRevision3NpcSourceInspection sourceInspection;
  final AuthoringRevision3NpcNativePublicationStatus publicationStatus;

  factory AuthoringRevision3NpcDraftPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(base);
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'npc_id',
      'script_module_id',
      'build_status',
      'runtime_status',
      'catalog_authority',
      'collision_authority',
      'source_inspection',
      'publication_status',
    }, 'revision-3 NPC preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 NPC preparation response is not prepared',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3NpcString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 NPC response basis head disagrees with its request',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3NpcString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 NPC candidate did not advance its head',
      );
    }
    final projectJson = _authoringRevision3NpcString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    if (candidate.projectId != base.projectId ||
        revision != candidate.revision ||
        revision != base.revision + 1) {
      throw const FormatException(
        'authoring revision-3 NPC candidate identity or revision disagrees with its basis',
      );
    }
    final npcId = _authoringRevision3NpcEntityId(json, 'npc_id');
    final scriptModuleId = _authoringRevision3NpcEntityId(
      json,
      'script_module_id',
    );
    if (npcId != request.npcId ||
        scriptModuleId != request.scriptModuleId ||
        npcId == scriptModuleId) {
      throw const FormatException(
        'authoring revision-3 NPC response entity IDs disagree with its request',
      );
    }
    _authoringRevision3NpcRequireExactDelta(
      base.project,
      candidate.project,
      npcId: npcId,
      scriptModuleId: scriptModuleId,
    );
    final pair = _authoringRevision3NpcRequireCandidatePair(
      candidate.project,
      projectId: candidate.projectId,
      npcId: npcId,
      scriptModuleId: scriptModuleId,
      parentCatalogId: request.intent.parentCatalogId,
    );
    if (pair.displayName != request.displayName ||
        pair.moduleNamespace != request.intent.moduleNamespace ||
        pair.uniqueName != request.intent.uniqueName) {
      throw const FormatException(
        'authoring revision-3 NPC candidate disagrees with its exact request intent',
      );
    }
    return AuthoringRevision3NpcDraftPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: candidate.projectId,
      revision: revision,
      npcId: npcId,
      scriptModuleId: scriptModuleId,
      displayName: pair.displayName,
      moduleNamespace: pair.moduleNamespace,
      uniqueName: pair.uniqueName,
      parentCatalogId: request.intent.parentCatalogId,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3NpcBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 NPC response has an unsupported build status',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3NpcRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 NPC response grants unsupported runtime authority',
        ),
      },
      catalogAuthority: switch (json['catalog_authority']) {
        'not_granted' => AuthoringRevision3NpcCatalogAuthority.notGranted,
        _ => throw const FormatException(
          'authoring revision-3 NPC response grants unsupported catalog authority',
        ),
      },
      collisionAuthority: switch (json['collision_authority']) {
        'not_granted' => AuthoringRevision3NpcCollisionAuthority.notGranted,
        _ => throw const FormatException(
          'authoring revision-3 NPC response grants unsupported collision authority',
        ),
      },
      sourceInspection: switch (json['source_inspection']) {
        'fresh_native_context_required' =>
          AuthoringRevision3NpcSourceInspection.freshNativeContextRequired,
        _ => throw const FormatException(
          'authoring revision-3 NPC response grants unsupported source inspection',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3NpcNativePublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 NPC response grants unsupported native publication authority',
        ),
      },
    );
  }
}

String _authoringRevision3NpcString(
  Map<String, Object?> json,
  String field, {
  required int maxBytes,
}) {
  final value = json[field];
  if (value is! String) {
    throw FormatException(
      'authoring revision-3 NPC field $field is not a string',
    );
  }
  try {
    // Validate the UTF-16 code units before any jsonEncode/UTF-8 operation can
    // silently replace an unpaired surrogate in a security-sensitive field.
    _authoringDraftRequestString(value, field, maxBytes);
  } on ArgumentError {
    throw FormatException(
      'authoring revision-3 NPC field $field is not bounded UTF-8',
    );
  }
  return value;
}

String _authoringRevision3NpcDisplayName(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRevision3NpcString(
    json,
    field,
    maxBytes: _maxAuthoringRevision3NpcDisplayNameBytes,
  );
  if (value.trim().isEmpty || value.runes.any(_authoringRevision3NpcControl)) {
    throw const FormatException(
      'authoring revision-3 NPC display name is invalid',
    );
  }
  return value;
}

bool _authoringRevision3NpcControl(int rune) =>
    rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

String _authoringRevision3NpcEntityId(Map<String, Object?> json, String field) {
  final id = _authoringEntityId(
    _authoringRequiredString(json, field, maxBytes: 32),
    field,
  );
  if (id == '00000000000000000000000000000000') {
    throw FormatException(
      'authoring revision-3 NPC field $field must not be zero',
    );
  }
  return id;
}

void _authoringRevision3NpcRequireFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final actual = json.keys.toList(growable: false);
  if (actual.length != expected.length) {
    throw FormatException(
      'authoring revision-3 NPC $context has an invalid field count',
    );
  }
  for (var index = 0; index < expected.length; index++) {
    if (actual[index] != expected[index]) {
      throw FormatException(
        'authoring revision-3 NPC $context has non-canonical field order',
      );
    }
  }
}

({Map<String, Object?> json, int byteLength, String sha256})
_authoringRevision3NpcContentSeal(Object? value, String context) {
  final seal = _authoringRequiredObject(value, 'revision-3 NPC $context seal');
  _authoringExactFields(seal, const {
    'byte_len',
    'sha256',
  }, 'revision-3 NPC $context seal');
  _authoringRevision3NpcRequireFieldOrder(seal, const <String>[
    'byte_len',
    'sha256',
  ], '$context seal');
  final sha256 = _authoringRequiredString(seal, 'sha256', maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha256)) {
    throw FormatException(
      'authoring revision-3 NPC $context seal SHA-256 is invalid',
    );
  }
  return (
    json: seal,
    byteLength: _authoringRequiredInt(
      seal,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    ),
    sha256: sha256,
  );
}

({Map<String, Object?> json, int byteLength, String sha256})
_authoringRevision3NpcGeneration(Object? value, String context) {
  final generation = _authoringRequiredObject(
    value,
    'revision-3 NPC $context generation',
  );
  _authoringExactFields(generation, const {
    'executable',
  }, 'revision-3 NPC $context generation');
  _authoringRevision3NpcRequireFieldOrder(generation, const <String>[
    'executable',
  ], '$context generation');
  final executable = _authoringRevision3NpcContentSeal(
    generation['executable'],
    '$context executable',
  );
  return (
    json: generation,
    byteLength: executable.byteLength,
    sha256: executable.sha256,
  );
}

bool _authoringRevision3NpcIsSupportedExecutable(
  int byteLength,
  String sha256,
) =>
    (byteLength == _authoringRevision3NpcExecutableByteLengthV1 &&
        sha256 == _authoringRevision3NpcExecutableSha256V1) ||
    (byteLength == _authoringRevision3NpcExecutableByteLengthV2 &&
        sha256 == _authoringRevision3NpcExecutableSha256V2) ||
    (byteLength == _authoringRevision3NpcExecutableByteLengthV3 &&
        sha256 == _authoringRevision3NpcExecutableSha256V3) ||
    (byteLength == _authoringRevision3NpcExecutableByteLengthV4 &&
        sha256 == _authoringRevision3NpcExecutableSha256V4);

bool _authoringRevision3NpcIsSupportedGeneration(
  ({Map<String, Object?> json, int byteLength, String sha256}) generation,
) => _authoringRevision3NpcIsSupportedExecutable(
  generation.byteLength,
  generation.sha256,
);

void _authoringRevision3NpcRequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required String npcId,
  required String scriptModuleId,
}) {
  for (final field in const <String>[
    'format',
    'schema_revision',
    'project_id',
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (!_authoringJsonDeepEquals(base[field], candidate[field])) {
      throw FormatException(
        'authoring revision-3 NPC candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 NPC basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 NPC candidate entities',
  );
  if (baseEntities.containsKey(npcId) ||
      baseEntities.containsKey(scriptModuleId) ||
      candidateEntities.length != baseEntities.length + 2) {
    throw const FormatException(
      'authoring revision-3 NPC candidate entity delta is not exactly two additions',
    );
  }
  for (final entry in baseEntities.entries) {
    if (!candidateEntities.containsKey(entry.key) ||
        !_authoringJsonDeepEquals(candidateEntities[entry.key], entry.value)) {
      throw const FormatException(
        'authoring revision-3 NPC candidate changed a preexisting entity',
      );
    }
  }
  for (final key in candidateEntities.keys) {
    if (!baseEntities.containsKey(key) &&
        key != npcId &&
        key != scriptModuleId) {
      throw const FormatException(
        'authoring revision-3 NPC candidate added an unexpected entity',
      );
    }
  }
}

({String displayName, String moduleNamespace, String uniqueName})
_authoringRevision3NpcRequireCandidatePair(
  Map<String, Object?> project, {
  required String projectId,
  required String npcId,
  required String scriptModuleId,
  required String parentCatalogId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 NPC candidate entities',
  );
  final target = _authoringRevision3NpcGeneration(
    project['target'],
    'project target',
  );
  if (!_authoringRevision3NpcIsSupportedGeneration(target)) {
    throw const FormatException(
      'authoring revision-3 NPC candidate uses unsupported catalog generation evidence',
    );
  }
  final npcEntity = _authoringRevision3NpcEntity(entities, npcId, 'npc_draft');
  final displayName = _authoringRevision3NpcDisplayName(
    npcEntity.entity,
    'display_name',
  );
  final npcData = npcEntity.data;
  _authoringExactFields(npcData, const {
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'revision-3 NPC candidate NPC data');
  _authoringRevision3NpcRequireFieldOrder(npcData, const <String>[
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  ], 'candidate NPC data');
  _authoringRevision3NpcRequireGenerator(npcData, 'NPC data');
  final input = _authoringRequiredObject(
    npcData['input'],
    'revision-3 NPC candidate input',
  );
  _authoringExactFields(input, const {
    'target',
    'module_namespace',
    'unique_name',
    'parent_character_definition',
    'parent_ai_agent_config',
    'parent_spawn_definition',
  }, 'revision-3 NPC candidate input');
  _authoringRevision3NpcRequireFieldOrder(input, const <String>[
    'target',
    'module_namespace',
    'unique_name',
    'parent_character_definition',
    'parent_ai_agent_config',
    'parent_spawn_definition',
  ], 'candidate input');
  final inputTarget = _authoringRevision3NpcGeneration(
    input['target'],
    'input target',
  );
  if (jsonEncode(inputTarget.json) != jsonEncode(target.json)) {
    throw const FormatException(
      'authoring revision-3 NPC candidate target disagrees with its project',
    );
  }
  final moduleNamespace = _authoringRevision3NpcString(
    input,
    'module_namespace',
    maxBytes: 255,
  );
  _authoringDraftValidateModuleNamespace(moduleNamespace);
  final uniqueName = _authoringRevision3NpcString(
    input,
    'unique_name',
    maxBytes: 64,
  );
  _authoringDraftValidateIdentifier(uniqueName, 'unique_name', maxBytes: 64);
  final selectionEvidence =
      _authoringRevision3NpcSelectionEvidence[parentCatalogId];
  if (selectionEvidence == null) {
    throw const FormatException(
      'authoring revision-3 NPC candidate has unsupported parent selection evidence',
    );
  }
  final characterParent = _authoringRevision3NpcResolvedParent(
    input['parent_character_definition'],
    target: target,
    context: 'parent CharacterDefinition',
    parentCatalogId: parentCatalogId,
    evidence: selectionEvidence.characterDefinition,
  );
  final agentParent = _authoringRevision3NpcResolvedParent(
    input['parent_ai_agent_config'],
    target: target,
    context: 'parent AIAgentConfig',
    parentCatalogId: parentCatalogId,
    evidence: selectionEvidence.aiAgentConfig,
  );
  final spawnParent = _authoringRevision3NpcResolvedParent(
    input['parent_spawn_definition'],
    target: target,
    context: 'parent SpawnDefinition',
    parentCatalogId: parentCatalogId,
    evidence: selectionEvidence.spawnDefinition,
  );
  final npcOrigin = _authoringRequiredObject(
    npcEntity.entity['origin'],
    'revision-3 NPC candidate NPC origin',
  );
  _authoringExactFields(npcOrigin, const {
    'type',
    'authored_runtime_id',
  }, 'revision-3 NPC candidate NPC origin');
  if (npcOrigin['type'] != 'new' ||
      npcOrigin['authored_runtime_id'] != uniqueName ||
      npcEntity.entity['revision'] != 0) {
    throw const FormatException(
      'authoring revision-3 NPC candidate NPC metadata is invalid',
    );
  }
  _authoringRevision3NpcTypedRef(
    npcData['script_module'],
    projectId: projectId,
    id: scriptModuleId,
    kind: 'script_module',
    context: 'NPC script module',
  );

  final moduleEntity = _authoringRevision3NpcEntity(
    entities,
    scriptModuleId,
    'script_module',
  );
  if (moduleEntity.entity['display_name'] != moduleNamespace ||
      moduleEntity.entity['revision'] != 0) {
    throw const FormatException(
      'authoring revision-3 NPC candidate ScriptModule metadata is invalid',
    );
  }
  final moduleOrigin = _authoringRequiredObject(
    moduleEntity.entity['origin'],
    'revision-3 NPC candidate ScriptModule origin',
  );
  _authoringExactFields(moduleOrigin, const {
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'revision-3 NPC candidate ScriptModule origin');
  if (moduleOrigin['type'] != 'generated') {
    throw const FormatException(
      'authoring revision-3 NPC candidate ScriptModule origin is invalid',
    );
  }
  _authoringRevision3NpcRequireGenerator(moduleOrigin, 'ScriptModule origin');
  _authoringRevision3NpcTypedRef(
    moduleOrigin['owner'],
    projectId: projectId,
    id: npcId,
    kind: 'npc_draft',
    context: 'ScriptModule origin owner',
  );
  final moduleData = moduleEntity.data;
  _authoringExactFields(moduleData, const {
    'generator_id',
    'generator_version',
    'owner',
    'module_namespace',
    'module_relative_path',
    'source',
    'source_sha256',
    'input_fingerprint',
    'status',
  }, 'revision-3 NPC candidate ScriptModule data');
  _authoringRevision3NpcRequireFieldOrder(moduleData, const <String>[
    'generator_id',
    'generator_version',
    'owner',
    'module_namespace',
    'module_relative_path',
    'source',
    'source_sha256',
    'input_fingerprint',
    'status',
  ], 'candidate ScriptModule data');
  _authoringRevision3NpcRequireGenerator(moduleData, 'ScriptModule data');
  _authoringRevision3NpcTypedRef(
    moduleData['owner'],
    projectId: projectId,
    id: npcId,
    kind: 'npc_draft',
    context: 'ScriptModule owner',
  );
  if (moduleData['module_namespace'] != moduleNamespace ||
      moduleData['module_relative_path'] !=
          '${moduleNamespace.replaceAll('.', '/')}.as') {
    throw const FormatException(
      'authoring revision-3 NPC candidate ScriptModule path is invalid',
    );
  }
  final expectedSource = _authoringRevision3NpcGeneratedSource(
    uniqueName: uniqueName,
    parentCharacter: characterParent.runtimeClass,
    parentAgent: agentParent.runtimeClass,
    parentSpawn: spawnParent.runtimeClass,
  );
  final source = _authoringRevision3NpcString(
    moduleData,
    'source',
    maxBytes: _maxAuthoringRevision3NpcSourceBytes,
  );
  if (source != expectedSource) {
    throw const FormatException(
      'authoring revision-3 NPC candidate generated source is not exact',
    );
  }
  final sourceSha256 = _authoringRequiredString(
    moduleData,
    'source_sha256',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sourceSha256) ||
      sourceSha256 != crypto.sha256.convert(utf8.encode(source)).toString()) {
    throw const FormatException(
      'authoring revision-3 NPC candidate source seal disagrees',
    );
  }
  final inputFingerprint = _authoringRequiredString(
    moduleData,
    'input_fingerprint',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(inputFingerprint) ||
      inputFingerprint != _authoringRevision3NpcInputFingerprint(input)) {
    throw const FormatException(
      'authoring revision-3 NPC candidate input fingerprint disagrees',
    );
  }
  final status = _authoringRequiredObject(
    moduleData['status'],
    'revision-3 NPC candidate ScriptModule status',
  );
  _authoringExactFields(status, const {
    'authoring',
    'runtime',
  }, 'revision-3 NPC candidate ScriptModule status');
  if (status['authoring'] != 'offline_draft' ||
      status['runtime'] != 'runtime_unqualified') {
    throw const FormatException(
      'authoring revision-3 NPC candidate ScriptModule status is unsupported',
    );
  }
  return (
    displayName: displayName,
    moduleNamespace: moduleNamespace,
    uniqueName: uniqueName,
  );
}

({Map<String, Object?> entity, Map<String, Object?> data})
_authoringRevision3NpcEntity(
  Map<String, Object?> entities,
  String id,
  String kind,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'revision-3 NPC candidate $kind entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'revision-3 NPC candidate $kind entity');
  if (entity['id'] != id) {
    throw FormatException(
      'authoring revision-3 NPC candidate $kind key and ID disagree',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 NPC candidate $kind payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'revision-3 NPC candidate $kind payload');
  if (payload['kind'] != kind) {
    throw FormatException(
      'authoring revision-3 NPC candidate $kind payload disagrees',
    );
  }
  return (
    entity: entity,
    data: _authoringRequiredObject(
      payload['data'],
      'revision-3 NPC candidate $kind data',
    ),
  );
}

void _authoringRevision3NpcRequireGenerator(
  Map<String, Object?> json,
  String context,
) {
  if (json['generator_id'] != _authoringRevision3NpcGeneratorId ||
      json['generator_version'] != _authoringRevision3NpcGeneratorVersion) {
    throw FormatException(
      'authoring revision-3 NPC candidate $context generator is unsupported',
    );
  }
}

({Map<String, Object?> json, String runtimeClass})
_authoringRevision3NpcResolvedParent(
  Object? value, {
  required ({Map<String, Object?> json, int byteLength, String sha256}) target,
  required String context,
  required String parentCatalogId,
  required _AuthoringRevision3NpcParentEvidence evidence,
}) {
  final parent = _authoringRequiredObject(
    value,
    'revision-3 NPC candidate $context',
  );
  _authoringExactFields(parent, const {
    'generation',
    'source_seal',
    'catalog_layer',
    'canonical_selector',
    'runtime_class',
  }, 'revision-3 NPC candidate $context');
  _authoringRevision3NpcRequireFieldOrder(parent, const <String>[
    'generation',
    'source_seal',
    'catalog_layer',
    'canonical_selector',
    'runtime_class',
  ], 'candidate $context');
  final generation = _authoringRevision3NpcGeneration(
    parent['generation'],
    '$context generation',
  );
  if (!_authoringRevision3NpcIsSupportedGeneration(generation) ||
      jsonEncode(generation.json) != jsonEncode(target.json)) {
    throw FormatException(
      'authoring revision-3 NPC candidate $context generation disagrees',
    );
  }
  final sourceSeal = _authoringRevision3NpcContentSeal(
    parent['source_seal'],
    '$context source',
  );
  final catalogLayer = _authoringRevision3NpcString(
    parent,
    'catalog_layer',
    maxBytes: 1024,
  );
  final canonicalSelector = _authoringRevision3NpcString(
    parent,
    'canonical_selector',
    maxBytes: 1024,
  );
  final runtimeClass = _authoringRevision3NpcString(
    parent,
    'runtime_class',
    maxBytes: 1024,
  );
  _authoringDraftValidateIdentifier(runtimeClass, 'runtime_class');
  if (sourceSeal.byteLength != evidence.sourceByteLength ||
      sourceSeal.sha256 != evidence.sourceSha256 ||
      catalogLayer != _authoringRevision3NpcCatalogLayer ||
      canonicalSelector !=
          _authoringRevision3NpcAuthoringSelector(
            parentCatalogId,
            evidence.role,
          ) ||
      runtimeClass != evidence.runtimeClass) {
    throw FormatException(
      'authoring revision-3 NPC candidate $context selection evidence is not exact',
    );
  }
  return (json: parent, runtimeClass: runtimeClass);
}

String _authoringRevision3NpcAuthoringSelector(
  String parentCatalogId,
  String role,
) {
  final bytes = BytesBuilder(copy: false)
    ..add(utf8.encode(_authoringRevision3NpcSelectorDomain));
  for (final value in <String>[parentCatalogId, role]) {
    final encoded = utf8.encode(value);
    bytes
      ..add(_authoringRevision3NpcUint64LittleEndian(encoded.length))
      ..add(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes.takeBytes())}';
}

Uint8List _authoringRevision3NpcUint64LittleEndian(int value) {
  final data = ByteData(8)..setUint64(0, value, Endian.little);
  return data.buffer.asUint8List();
}

void _authoringRevision3NpcTypedRef(
  Object? value, {
  required String projectId,
  required String id,
  required String kind,
  required String context,
}) {
  final ref = _authoringRequiredObject(
    value,
    'revision-3 NPC candidate $context reference',
  );
  _authoringExactFields(ref, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 NPC candidate $context reference');
  if (ref['project_id'] != projectId ||
      ref['id'] != id ||
      ref['expected_kind'] != kind) {
    throw FormatException(
      'authoring revision-3 NPC candidate $context reference is not exact',
    );
  }
}

String _authoringRevision3NpcGeneratedSource({
  required String uniqueName,
  required String parentCharacter,
  required String parentAgent,
  required String parentSpawn,
}) {
  final character = 'UCharacterDefinition_Human_$uniqueName';
  final agent = 'UAIAgentConfig_Human_$uniqueName';
  final spawn = 'USpawnAIAgentDefinition_$uniqueName';
  return '''class $character
    : $parentCharacter
{
    default m_UniqueName = n"$uniqueName";
}

class $agent
    : $parentAgent
{
    default m_CharacterDefinition =
        $character::StaticClass();
}

class $spawn
    : $parentSpawn
{
    default AIAgentConfigClass =
        $agent::StaticClass();
}
''';
}

String _authoringRevision3NpcInputFingerprint(Map<String, Object?> input) {
  final canonical = utf8.encode(jsonEncode(input));
  final generator = utf8.encode(_authoringRevision3NpcGeneratorId);
  final bytes = BytesBuilder(copy: false)
    ..add(utf8.encode(_authoringRevision3NpcFingerprintDomain))
    ..add(_authoringRevision3NpcUint64(generator.length))
    ..add(generator)
    ..add(_authoringRevision3NpcUint64(_authoringRevision3NpcGeneratorVersion))
    ..add(_authoringRevision3NpcUint64(canonical.length))
    ..add(canonical);
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

Uint8List _authoringRevision3NpcUint64(int value) {
  final data = ByteData(8)..setUint64(0, value, Endian.big);
  return data.buffer.asUint8List();
}
