part of '../core/mod_ffi.dart';

const _revision3ProjectBuildSchemaV1 = 1;
const _revision3ProjectBuildMaxBlockers = 64;
const _revision3ProjectBuildMaxEntities = 100000;
const _revision3ProjectBuildMaxAssets = 100000;
const _revision3ProjectBuildMaxDataAssetStages = 1024;
const _revision3ProjectBuildInputSealFormat =
    'gore.authoring.revision3-project-build-input.v1';
const _revision3ProjectBuildPlanSealFormat =
    'gore.authoring.revision3-project-build-plan.v1';

enum AuthoringRevision3ProjectBuildOutcome { empty, blocked, coverageComplete }

enum AuthoringRevision3ProjectBuildDomain {
  localization,
  dialog,
  voice,
  npc,
  quest,
  scripts,
  items,
  dataAssets,
}

enum AuthoringRevision3ProjectBuildDomainStatus { notPresent, ready, blocked }

enum AuthoringRevision3ProjectBuildBlockerCategory {
  authorProject,
  toolkitSupport,
}

enum AuthoringRevision3ProjectBuildBlockReason {
  localizationLoweringUnavailable,
  dialogLoweringUnavailable,
  voiceProjectNameUnsupported,
  voiceLineLabelUnsupported,
  voiceSlotLimitExceeded,
  voiceTargetUnresolved,
  voiceTargetAmbiguous,
  voiceAddUnqualified,
  voiceSelectedTakeMissing,
  voiceSelectedTakeNotApproved,
  voiceSelectedTakeCodecUnqualified,
  voicePayloadBudgetExceeded,
  npcLoweringUnavailable,
  questLoweringUnavailable,
  scriptLoweringUnavailable,
  itemPatchLoweringUnavailable,
  dataAssetTargetUnsupported,
  dataAssetSelectorMismatch,
  dataAssetReplacementMalformed,
  dataAssetReplacementNonFinite,
  dataAssetReplacementNonPositive,
  dataAssetPreservedComponentChanged,
  dataAssetReviewedPreparationFailed,
  dataAssetDerivedReplacementMismatch,
}

enum AuthoringRevision3ProjectBuildScope { projectBuildReadinessOnly }

enum AuthoringRevision3ProjectBuildAuthority { notGranted }

enum AuthoringRevision3ProjectBuildArtifactStatus { notCreated }

enum AuthoringRevision3ProjectBuildDeploymentStatus { notPerformed }

enum AuthoringRevision3ProjectBuildRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3ProjectBuildPublicationStatus { notSupported }

final class AuthoringRevision3ProjectBuildSeal {
  const AuthoringRevision3ProjectBuildSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  static AuthoringRevision3ProjectBuildSeal _parse(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const <String>{'byte_len', 'sha256'}, context);
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw FormatException('authoring $context SHA-256 is not canonical');
    }
    return AuthoringRevision3ProjectBuildSeal._(
      byteLength: _authoringRequiredInt(
        json,
        'byte_len',
        min: 1,
        max: _maxAuthoringSignedJsonInteger,
      ),
      sha256: sha256,
    );
  }

  bool _same(AuthoringRevision3ProjectBuildSeal other) =>
      byteLength == other.byteLength && sha256 == other.sha256;

  Map<String, Object?> _wireJson() => <String, Object?>{
    'byte_len': byteLength,
    'sha256': sha256,
  };
}

final class AuthoringRevision3ProjectBuildDomainSummary {
  const AuthoringRevision3ProjectBuildDomainSummary._({
    required this.domain,
    required this.status,
    required this.contentCount,
    required this.readyCount,
    required this.blockedCount,
  });

  final AuthoringRevision3ProjectBuildDomain domain;
  final AuthoringRevision3ProjectBuildDomainStatus status;
  final int contentCount;
  final int readyCount;
  final int blockedCount;

  static AuthoringRevision3ProjectBuildDomainSummary _parse(
    Object? value,
    int index,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project build domain $index',
    );
    _authoringExactFields(json, const <String>{
      'domain',
      'status',
      'content_count',
      'ready_count',
      'blocked_count',
    }, 'revision-3 project build domain $index');
    final domain = _revision3ProjectBuildDomain(
      _authoringRequiredString(json, 'domain', maxBytes: 32),
    );
    if (domain.index != index) {
      throw const FormatException(
        'revision-3 project build domains are not in the fixed canonical order',
      );
    }
    final status = _revision3ProjectBuildDomainStatus(
      _authoringRequiredString(json, 'status', maxBytes: 32),
    );
    final contentCount = _authoringRequiredInt(
      json,
      'content_count',
      max: _maxAuthoringSignedJsonInteger,
    );
    final readyCount = _authoringRequiredInt(
      json,
      'ready_count',
      max: _maxAuthoringSignedJsonInteger,
    );
    final blockedCount = _authoringRequiredInt(
      json,
      'blocked_count',
      max: _maxAuthoringSignedJsonInteger,
    );
    if (readyCount > contentCount ||
        blockedCount > contentCount ||
        readyCount + blockedCount != contentCount) {
      throw const FormatException(
        'revision-3 project build domain counts do not form one partition',
      );
    }
    final expectedStatus = contentCount == 0
        ? AuthoringRevision3ProjectBuildDomainStatus.notPresent
        : blockedCount == 0
        ? AuthoringRevision3ProjectBuildDomainStatus.ready
        : AuthoringRevision3ProjectBuildDomainStatus.blocked;
    if (status != expectedStatus) {
      throw const FormatException(
        'revision-3 project build domain status disagrees with its counts',
      );
    }
    return AuthoringRevision3ProjectBuildDomainSummary._(
      domain: domain,
      status: status,
      contentCount: contentCount,
      readyCount: readyCount,
      blockedCount: blockedCount,
    );
  }

  Map<String, Object?> _wireJson() => <String, Object?>{
    'domain': domain._wireName,
    'status': status._wireName,
    'content_count': contentCount,
    'ready_count': readyCount,
    'blocked_count': blockedCount,
  };
}

final class AuthoringRevision3ProjectBuildBlocker {
  const AuthoringRevision3ProjectBuildBlocker._({
    required this.category,
    required this.domain,
    required this.reason,
    required this.affectedCount,
  });

  final AuthoringRevision3ProjectBuildBlockerCategory category;
  final AuthoringRevision3ProjectBuildDomain domain;
  final AuthoringRevision3ProjectBuildBlockReason reason;
  final int affectedCount;

  static AuthoringRevision3ProjectBuildBlocker _parse(
    Object? value,
    int index,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project build blocker $index',
    );
    _authoringExactFields(json, const <String>{
      'category',
      'domain',
      'reason',
      'affected_count',
    }, 'revision-3 project build blocker $index');
    return AuthoringRevision3ProjectBuildBlocker._(
      category: _revision3ProjectBuildBlockerCategory(
        _authoringRequiredString(json, 'category', maxBytes: 32),
      ),
      domain: _revision3ProjectBuildDomain(
        _authoringRequiredString(json, 'domain', maxBytes: 32),
      ),
      reason: _revision3ProjectBuildBlockReason(
        _authoringRequiredString(json, 'reason', maxBytes: 64),
      ),
      affectedCount: _authoringRequiredInt(
        json,
        'affected_count',
        min: 1,
        max: _maxAuthoringSignedJsonInteger,
      ),
    );
  }

  Map<String, Object?> _wireJson() => <String, Object?>{
    'category': category._wireName,
    'domain': domain._wireName,
    'reason': reason._wireName,
    'affected_count': affectedCount,
  };
}

final class AuthoringRevision3ProjectBuildPlan {
  const AuthoringRevision3ProjectBuildPlan._({
    required this.schemaRevision,
    required this.projectId,
    required this.projectRevision,
    required this.outcome,
    required this.productionContentCount,
    required this.inputSeal,
    required this.planSeal,
    required this.domains,
    required this.blockers,
    required this.scope,
    required this.buildAuthority,
    required this.artifactStatus,
    required this.deploymentStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final int schemaRevision;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3ProjectBuildOutcome outcome;
  final int productionContentCount;
  final AuthoringRevision3ProjectBuildSeal inputSeal;
  final AuthoringRevision3ProjectBuildSeal planSeal;
  final List<AuthoringRevision3ProjectBuildDomainSummary> domains;
  final List<AuthoringRevision3ProjectBuildBlocker> blockers;
  final AuthoringRevision3ProjectBuildScope scope;
  final AuthoringRevision3ProjectBuildAuthority buildAuthority;
  final AuthoringRevision3ProjectBuildArtifactStatus artifactStatus;
  final AuthoringRevision3ProjectBuildDeploymentStatus deploymentStatus;
  final AuthoringRevision3ProjectBuildRuntimeStatus runtimeStatus;
  final AuthoringRevision3ProjectBuildPublicationStatus publicationStatus;

  bool get isEmpty => outcome == AuthoringRevision3ProjectBuildOutcome.empty;
  bool get isBlocked =>
      outcome == AuthoringRevision3ProjectBuildOutcome.blocked;
  bool get hasCompleteCoverage =>
      outcome == AuthoringRevision3ProjectBuildOutcome.coverageComplete;

  AuthoringRevision3ProjectBuildDomainSummary domain(
    AuthoringRevision3ProjectBuildDomain value,
  ) => domains[value.index];

  static AuthoringRevision3ProjectBuildPlan _parse(
    Object? value, {
    required String expectedProjectJson,
  }) {
    final expectation = _Revision3ProjectBuildExpectation.fromProjectJson(
      expectedProjectJson,
    );
    final json = _authoringRequiredObject(
      value,
      'revision-3 project build plan',
    );
    _authoringExactFields(json, const <String>{
      'schema_revision',
      'project_id',
      'project_revision',
      'outcome',
      'production_content_count',
      'input_seal',
      'plan_seal',
      'domains',
      'blockers',
      'scope',
      'build_authority',
      'artifact_status',
      'deployment_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 project build plan');

    final schemaRevision = _authoringRequiredInt(
      json,
      'schema_revision',
      min: _revision3ProjectBuildSchemaV1,
      max: _revision3ProjectBuildSchemaV1,
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
    if (projectId != expectation.projectId ||
        projectRevision != expectation.projectRevision) {
      throw const FormatException(
        'revision-3 project build plan is not bound to the exact project',
      );
    }
    final outcome = _revision3ProjectBuildOutcome(
      _authoringRequiredString(json, 'outcome', maxBytes: 32),
    );
    final productionContentCount = _authoringRequiredInt(
      json,
      'production_content_count',
      max: _maxAuthoringSignedJsonInteger,
    );
    final inputSeal = AuthoringRevision3ProjectBuildSeal._parse(
      json['input_seal'],
      'revision-3 project build input seal',
    );
    final planSeal = AuthoringRevision3ProjectBuildSeal._parse(
      json['plan_seal'],
      'revision-3 project build plan seal',
    );

    final rawDomains = json['domains'];
    if (rawDomains is! List ||
        rawDomains.length !=
            AuthoringRevision3ProjectBuildDomain.values.length) {
      throw const FormatException(
        'revision-3 project build plan does not contain the fixed domain set',
      );
    }
    final domains = <AuthoringRevision3ProjectBuildDomainSummary>[
      for (var index = 0; index < rawDomains.length; index++)
        AuthoringRevision3ProjectBuildDomainSummary._parse(
          rawDomains[index],
          index,
        ),
    ];

    final rawBlockers = json['blockers'];
    if (rawBlockers is! List ||
        rawBlockers.length > _revision3ProjectBuildMaxBlockers) {
      throw const FormatException(
        'revision-3 project build blocker list exceeds its fixed bound',
      );
    }
    final blockers = <AuthoringRevision3ProjectBuildBlocker>[
      for (var index = 0; index < rawBlockers.length; index++)
        AuthoringRevision3ProjectBuildBlocker._parse(rawBlockers[index], index),
    ];

    if (json['scope'] != 'project_build_readiness_only' ||
        json['build_authority'] != 'not_granted' ||
        json['artifact_status'] != 'not_created' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'revision-3 project build plan overstates its authority',
      );
    }

    _revision3ProjectBuildValidateDomains(domains, expectation);
    _revision3ProjectBuildValidateBlockers(blockers, domains, expectation);
    if (productionContentCount != expectation.productionContentCount) {
      throw const FormatException(
        'revision-3 project build production count disagrees with the exact project',
      );
    }
    final expectedOutcome = productionContentCount == 0
        ? AuthoringRevision3ProjectBuildOutcome.empty
        : blockers.isEmpty
        ? AuthoringRevision3ProjectBuildOutcome.coverageComplete
        : AuthoringRevision3ProjectBuildOutcome.blocked;
    if (outcome != expectedOutcome) {
      throw const FormatException(
        'revision-3 project build outcome disagrees with its exact coverage',
      );
    }
    if (!inputSeal._same(expectation.inputSeal)) {
      throw const FormatException(
        'revision-3 project build input seal disagrees with the exact project',
      );
    }

    final expectedPlanSeal = _revision3ProjectBuildSealJson(<String, Object?>{
      'format': _revision3ProjectBuildPlanSealFormat,
      'schema_revision': schemaRevision,
      'project_id': projectId,
      'project_revision': projectRevision,
      'outcome': outcome._wireName,
      'production_content_count': productionContentCount,
      'input_seal': inputSeal._wireJson(),
      'domains': <Object?>[for (final domain in domains) domain._wireJson()],
      'blockers': <Object?>[
        for (final blocker in blockers) blocker._wireJson(),
      ],
      'scope': 'project_build_readiness_only',
      'build_authority': 'not_granted',
      'artifact_status': 'not_created',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    });
    if (!planSeal._same(expectedPlanSeal)) {
      throw const FormatException(
        'revision-3 project build plan seal is not exact',
      );
    }

    return AuthoringRevision3ProjectBuildPlan._(
      schemaRevision: schemaRevision,
      projectId: projectId,
      projectRevision: projectRevision,
      outcome: outcome,
      productionContentCount: productionContentCount,
      inputSeal: inputSeal,
      planSeal: planSeal,
      domains: List.unmodifiable(domains),
      blockers: List.unmodifiable(blockers),
      scope: AuthoringRevision3ProjectBuildScope.projectBuildReadinessOnly,
      buildAuthority: AuthoringRevision3ProjectBuildAuthority.notGranted,
      artifactStatus: AuthoringRevision3ProjectBuildArtifactStatus.notCreated,
      deploymentStatus:
          AuthoringRevision3ProjectBuildDeploymentStatus.notPerformed,
      runtimeStatus:
          AuthoringRevision3ProjectBuildRuntimeStatus.runtimeUnqualified,
      publicationStatus:
          AuthoringRevision3ProjectBuildPublicationStatus.notSupported,
    );
  }
}

final class AuthoringRevision3ProjectBuildPlanResult {
  const AuthoringRevision3ProjectBuildPlanResult._({
    required this.basisHead,
    required this.plan,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringRevision3ProjectBuildPlan plan;

  factory AuthoringRevision3ProjectBuildPlanResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectJson,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'basis_head_json',
      'plan',
    }, 'revision-3 project build-plan response');
    if (json['ok'] != true) {
      throw const FormatException(
        'revision-3 project build-plan response is not successful',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException(
        'revision-3 project build-plan response is stale',
      );
    }
    return AuthoringRevision3ProjectBuildPlanResult._(
      basisHead: basisHead,
      plan: AuthoringRevision3ProjectBuildPlan._parse(
        json['plan'],
        expectedProjectJson: expectedProjectJson,
      ),
    );
  }
}

final class _Revision3ProjectBuildExpectation {
  _Revision3ProjectBuildExpectation._({
    required this.projectId,
    required this.projectRevision,
    required this.domainCounts,
    required this.voiceReadyCount,
    required this.voiceBlockedCount,
    required this.expectedBlockers,
    required this.inputSeal,
  });

  final String projectId;
  final int projectRevision;
  final List<int> domainCounts;
  final int voiceReadyCount;
  final int voiceBlockedCount;
  final List<_Revision3ProjectBuildExpectedBlocker> expectedBlockers;
  final AuthoringRevision3ProjectBuildSeal inputSeal;

  int get productionContentCount {
    var total = 0;
    for (final domain in const <AuthoringRevision3ProjectBuildDomain>[
      AuthoringRevision3ProjectBuildDomain.localization,
      AuthoringRevision3ProjectBuildDomain.dialog,
      AuthoringRevision3ProjectBuildDomain.voice,
      AuthoringRevision3ProjectBuildDomain.npc,
      AuthoringRevision3ProjectBuildDomain.quest,
      AuthoringRevision3ProjectBuildDomain.items,
      AuthoringRevision3ProjectBuildDomain.dataAssets,
    ]) {
      total += domainCounts[domain.index];
      if (total > _maxAuthoringSignedJsonInteger) {
        throw const FormatException(
          'revision-3 project build count exceeds the signed wire range',
        );
      }
    }
    return total;
  }

  factory _Revision3ProjectBuildExpectation.fromProjectJson(
    String projectJson,
  ) {
    if (utf8.encode(projectJson).length > _maxAuthoringProjectJsonBytes) {
      throw const FormatException(
        'revision-3 project build project exceeds its fixed bound',
      );
    }
    final current = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    final entities = _authoringRequiredObject(
      current.project['entities'],
      'revision-3 project build entities',
    );
    if (entities.length > _revision3ProjectBuildMaxEntities) {
      throw const FormatException(
        'revision-3 project build entity count exceeds its fixed bound',
      );
    }
    final counts = List<int>.filled(
      AuthoringRevision3ProjectBuildDomain.values.length,
      0,
    );
    for (final entry in entities.entries) {
      final id = _authoringEntityId(entry.key, 'project entity ID');
      final entity = _authoringRequiredObject(
        entry.value,
        'revision-3 project build entity',
      );
      _authoringExactFields(entity, const <String>{
        'id',
        'display_name',
        'origin',
        'revision',
        'payload',
      }, 'revision-3 project build entity');
      if (entity['id'] != id) {
        throw const FormatException(
          'revision-3 project build entity key and ID disagree',
        );
      }
      final originType = _revision3ProjectBuildOriginType(entity['origin']);
      final payload = _authoringRequiredObject(
        entity['payload'],
        'revision-3 project build entity payload',
      );
      _authoringExactFields(payload, const <String>{
        'kind',
        'data',
      }, 'revision-3 project build entity payload');
      final kind = _authoringRequiredString(payload, 'kind', maxBytes: 64);
      final domain = switch (kind) {
        'localization_entry'
            when originType == 'new' || originType == 'generated' =>
          AuthoringRevision3ProjectBuildDomain.localization,
        'dialog_line' when originType == 'new' || originType == 'generated' =>
          AuthoringRevision3ProjectBuildDomain.dialog,
        'voice_slot' => AuthoringRevision3ProjectBuildDomain.voice,
        'npc_draft' => AuthoringRevision3ProjectBuildDomain.npc,
        'quest_draft' => AuthoringRevision3ProjectBuildDomain.quest,
        'script_module' => AuthoringRevision3ProjectBuildDomain.scripts,
        'item_patch' => AuthoringRevision3ProjectBuildDomain.items,
        'localization_entry' || 'dialog_line' || 'voice_take' => null,
        _ => throw const FormatException(
          'revision-3 project build project has an unknown entity kind',
        ),
      };
      if (domain != null) counts[domain.index]++;
    }

    final stageSeals = _revision3ProjectBuildStageManifestSeals(
      current.project,
    );
    counts[AuthoringRevision3ProjectBuildDomain.dataAssets.index] =
        stageSeals.length;
    final projectBytes = utf8.encode(projectJson);
    final projectSeal = _revision3ProjectBuildSealBytes(projectBytes);
    final inputSeal = _revision3ProjectBuildSealJson(<String, Object?>{
      'format': _revision3ProjectBuildInputSealFormat,
      'project': projectSeal._wireJson(),
      'dataasset_stage_manifests': <Object?>[
        for (final seal in stageSeals) seal._wireJson(),
      ],
    });

    final expectedBlockers = <_Revision3ProjectBuildExpectedBlocker>[];
    void allBlocked(
      AuthoringRevision3ProjectBuildDomain domain,
      AuthoringRevision3ProjectBuildBlockReason reason,
    ) {
      final count = counts[domain.index];
      if (count == 0) return;
      expectedBlockers.add(
        _Revision3ProjectBuildExpectedBlocker(
          category:
              AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport,
          domain: domain,
          reason: reason,
          affectedCount: count,
        ),
      );
    }

    allBlocked(
      AuthoringRevision3ProjectBuildDomain.localization,
      AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable,
    );
    allBlocked(
      AuthoringRevision3ProjectBuildDomain.dialog,
      AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable,
    );

    final voiceCount = counts[AuthoringRevision3ProjectBuildDomain.voice.index];
    var voiceReadyCount = 0;
    var voiceBlockedCount = 0;
    if (voiceCount > 0) {
      final safeBundleName = _revision3ProjectBuildSafeBundleName(
        current.project,
      );
      final voice = safeBundleName
          ? _AuthoringRevision3VoiceBuildExpectation.fromCanonicalProjectJson(
              projectJson,
              allowUnsupportedLineLabel: true,
            )
          : null;
      if (voice == null) {
        voiceBlockedCount = voiceCount;
        expectedBlockers.add(
          _Revision3ProjectBuildExpectedBlocker(
            category:
                AuthoringRevision3ProjectBuildBlockerCategory.authorProject,
            domain: AuthoringRevision3ProjectBuildDomain.voice,
            reason: AuthoringRevision3ProjectBuildBlockReason
                .voiceProjectNameUnsupported,
            affectedCount: voiceCount,
          ),
        );
      } else if (voice.hasUnsupportedLineLabel) {
        voiceBlockedCount = voiceCount;
        expectedBlockers.add(
          _Revision3ProjectBuildExpectedBlocker(
            category:
                AuthoringRevision3ProjectBuildBlockerCategory.authorProject,
            domain: AuthoringRevision3ProjectBuildDomain.voice,
            reason: AuthoringRevision3ProjectBuildBlockReason
                .voiceLineLabelUnsupported,
            affectedCount: voiceCount,
          ),
        );
      } else if (voice.isReady) {
        voiceReadyCount = voiceCount;
      } else {
        voiceReadyCount = voice.readySlots;
        voiceBlockedCount = voiceCount - voiceReadyCount;
        final aggregates =
            <
              ({
                AuthoringRevision3ProjectBuildBlockerCategory category,
                AuthoringRevision3ProjectBuildBlockReason reason,
              }),
              int
            >{};
        for (final blocker in voice.blockers) {
          final reason = _revision3ProjectBuildVoiceReason(blocker.reason);
          if (reason == null) continue;
          final category =
              reason ==
                  AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified
              ? AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport
              : AuthoringRevision3ProjectBuildBlockerCategory.authorProject;
          final key = (category: category, reason: reason);
          aggregates.update(
            key,
            (count) => count + (blocker.isGlobal ? voiceCount : 1),
            ifAbsent: () => blocker.isGlobal ? voiceCount : 1,
          );
        }
        for (final entry in aggregates.entries) {
          expectedBlockers.add(
            _Revision3ProjectBuildExpectedBlocker(
              category: entry.key.category,
              domain: AuthoringRevision3ProjectBuildDomain.voice,
              reason: entry.key.reason,
              affectedCount: entry.value,
            ),
          );
        }
      }
    }

    allBlocked(
      AuthoringRevision3ProjectBuildDomain.npc,
      AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable,
    );
    allBlocked(
      AuthoringRevision3ProjectBuildDomain.quest,
      AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable,
    );
    allBlocked(
      AuthoringRevision3ProjectBuildDomain.scripts,
      AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable,
    );
    allBlocked(
      AuthoringRevision3ProjectBuildDomain.items,
      AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable,
    );
    expectedBlockers.sort(_revision3ProjectBuildExpectedBlockerCompare);

    return _Revision3ProjectBuildExpectation._(
      projectId: current.projectId,
      projectRevision: current.revision,
      domainCounts: List.unmodifiable(counts),
      voiceReadyCount: voiceReadyCount,
      voiceBlockedCount: voiceBlockedCount,
      expectedBlockers: List.unmodifiable(expectedBlockers),
      inputSeal: inputSeal,
    );
  }
}

final class _Revision3ProjectBuildExpectedBlocker {
  const _Revision3ProjectBuildExpectedBlocker({
    required this.category,
    required this.domain,
    required this.reason,
    required this.affectedCount,
  });

  final AuthoringRevision3ProjectBuildBlockerCategory category;
  final AuthoringRevision3ProjectBuildDomain domain;
  final AuthoringRevision3ProjectBuildBlockReason reason;
  final int affectedCount;

  bool matches(AuthoringRevision3ProjectBuildBlocker other) =>
      category == other.category &&
      domain == other.domain &&
      reason == other.reason &&
      affectedCount == other.affectedCount;
}

void _revision3ProjectBuildValidateDomains(
  List<AuthoringRevision3ProjectBuildDomainSummary> domains,
  _Revision3ProjectBuildExpectation expectation,
) {
  for (final summary in domains) {
    final expectedCount = expectation.domainCounts[summary.domain.index];
    var expectedReady = 0;
    var expectedBlocked = expectedCount;
    if (summary.domain == AuthoringRevision3ProjectBuildDomain.voice) {
      expectedReady = expectation.voiceReadyCount;
      expectedBlocked = expectation.voiceBlockedCount;
    } else if (summary.domain ==
        AuthoringRevision3ProjectBuildDomain.dataAssets) {
      // The reviewed cooked-asset evaluator remains native-only. Dart binds the
      // exact manifest membership/seals and validates the aggregate blocker
      // structure below; it deliberately does not claim an independent
      // ready/blocked re-evaluation without the Store-owned manifest bytes.
      continue;
    }
    if (summary.contentCount != expectedCount ||
        summary.readyCount != expectedReady ||
        summary.blockedCount != expectedBlocked) {
      throw const FormatException(
        'revision-3 project build domain disagrees with exact project content',
      );
    }
  }
  final dataAssets =
      domains[AuthoringRevision3ProjectBuildDomain.dataAssets.index];
  if (dataAssets.contentCount !=
      expectation.domainCounts[AuthoringRevision3ProjectBuildDomain
          .dataAssets
          .index]) {
    throw const FormatException(
      'revision-3 project build DataAsset count disagrees with the exact stage set',
    );
  }
}

void _revision3ProjectBuildValidateBlockers(
  List<AuthoringRevision3ProjectBuildBlocker> blockers,
  List<AuthoringRevision3ProjectBuildDomainSummary> domains,
  _Revision3ProjectBuildExpectation expectation,
) {
  for (var index = 0; index < blockers.length; index++) {
    final blocker = blockers[index];
    final domain = domains[blocker.domain.index];
    if (blocker.affectedCount > domain.contentCount ||
        domain.blockedCount == 0 ||
        !_revision3ProjectBuildReasonBelongsToDomain(
          blocker.reason,
          blocker.domain,
        )) {
      throw const FormatException(
        'revision-3 project build blocker is not valid for its domain',
      );
    }
    if (index > 0 &&
        _revision3ProjectBuildBlockerCompare(blockers[index - 1], blocker) >=
            0) {
      throw const FormatException(
        'revision-3 project build blockers are not unique canonical groups',
      );
    }
  }

  final nonDataAssets = blockers
      .where(
        (blocker) =>
            blocker.domain != AuthoringRevision3ProjectBuildDomain.dataAssets,
      )
      .toList(growable: false);
  if (nonDataAssets.length != expectation.expectedBlockers.length) {
    throw const FormatException(
      'revision-3 project build blockers disagree with exact project content',
    );
  }
  for (var index = 0; index < nonDataAssets.length; index++) {
    if (!expectation.expectedBlockers[index].matches(nonDataAssets[index])) {
      throw const FormatException(
        'revision-3 project build blocker disagrees with exact project content',
      );
    }
  }

  final dataAssets = blockers.where(
    (blocker) =>
        blocker.domain == AuthoringRevision3ProjectBuildDomain.dataAssets,
  );
  var affected = 0;
  for (final blocker in dataAssets) {
    if (!_revision3ProjectBuildDataAssetCategoryIsExact(blocker)) {
      throw const FormatException(
        'revision-3 project build DataAsset blocker category is invalid',
      );
    }
    affected += blocker.affectedCount;
  }
  if (affected !=
      domains[AuthoringRevision3ProjectBuildDomain.dataAssets.index]
          .blockedCount) {
    throw const FormatException(
      'revision-3 project build DataAsset blockers do not cover the blocked stages exactly',
    );
  }
}

List<AuthoringRevision3ProjectBuildSeal>
_revision3ProjectBuildStageManifestSeals(Map<String, Object?> project) {
  final assetStore = _authoringRequiredObject(
    project['asset_store'],
    'revision-3 project build asset Store',
  );
  _authoringExactFields(assetStore, const <String>{
    'assets',
  }, 'revision-3 project build asset Store');
  final assets = _authoringRequiredObject(
    assetStore['assets'],
    'revision-3 project build assets',
  );
  if (assets.length > _revision3ProjectBuildMaxAssets) {
    throw const FormatException(
      'revision-3 project build asset count exceeds its fixed bound',
    );
  }
  final seals = <AuthoringRevision3ProjectBuildSeal>[];
  final digests = assets.keys.toList(growable: false)..sort();
  for (final digest in digests) {
    if (!_authoringSha256Pattern.hasMatch(digest)) {
      throw const FormatException(
        'revision-3 project build asset key is not a SHA-256',
      );
    }
    final meta = _authoringRequiredObject(
      assets[digest],
      'revision-3 project build asset metadata',
    );
    _authoringExactFields(meta, const <String>{
      'byte_len',
      'media_type',
    }, 'revision-3 project build asset metadata');
    final byteLength = _authoringRequiredInt(
      meta,
      'byte_len',
      min: 1,
      max: _maxAuthoringSignedJsonInteger,
    );
    final mediaType = _authoringRequiredString(
      meta,
      'media_type',
      maxBytes: 256,
    );
    if (mediaType == _revision3DataAssetManifestMediaType) {
      if (byteLength > _maxAuthoringRevision3DataAssetManifestBytes) {
        throw const FormatException(
          'revision-3 project build DataAsset manifest exceeds its fixed bound',
        );
      }
      seals.add(
        AuthoringRevision3ProjectBuildSeal._(
          byteLength: byteLength,
          sha256: digest,
        ),
      );
    }
  }
  if (seals.length > _revision3ProjectBuildMaxDataAssetStages) {
    throw const FormatException(
      'revision-3 project build DataAsset stage count exceeds its fixed bound',
    );
  }
  return List.unmodifiable(seals);
}

String _revision3ProjectBuildOriginType(Object? value) {
  final origin = _authoringRequiredObject(
    value,
    'revision-3 project build entity origin',
  );
  final type = _authoringRequiredString(origin, 'type', maxBytes: 32);
  final fields = switch (type) {
    'new' => const <String>{'type', 'authored_runtime_id'},
    'vanilla' => const <String>{
      'type',
      'generation',
      'catalog_layer',
      'canonical_selector',
      'source_seal',
    },
    'imported' =>
      origin.containsKey('external_identity')
          ? const <String>{
              'type',
              'importer',
              'source_seal',
              'external_identity',
            }
          : const <String>{'type', 'importer', 'source_seal'},
    'generated' => const <String>{
      'type',
      'generator_id',
      'generator_version',
      'owner',
    },
    _ => throw const FormatException(
      'revision-3 project build entity has an unknown origin',
    ),
  };
  _authoringExactFields(
    origin,
    fields,
    'revision-3 project build entity origin',
  );
  return type;
}

bool _revision3ProjectBuildSafeBundleName(Map<String, Object?> project) {
  final meta = _authoringRequiredObject(
    project['meta'],
    'revision-3 project build metadata',
  );
  _authoringExactFields(meta, const <String>{
    'name',
    'version',
    'author',
  }, 'revision-3 project build metadata');
  final rawName = meta['name'];
  if (rawName is! String) {
    throw const FormatException(
      'revision-3 project build metadata name is not a string',
    );
  }
  final nameBytes = _revision3ProjectBuildStrictUtf8ByteLength(
    rawName,
    'revision-3 project build metadata name',
  );
  if (nameBytes == 0 || nameBytes > 1024) return false;
  if (rawName.startsWith('/') ||
      rawName.startsWith(r'\') ||
      rawName.contains('/') ||
      rawName.contains(r'\') ||
      rawName.contains(':') ||
      rawName.endsWith(' ') ||
      rawName.endsWith('.') ||
      rawName.runes.any(
        (rune) =>
            _authoringRevision3VoiceControl(rune) ||
            const <int>{0x3c, 0x3e, 0x22, 0x7c, 0x3f, 0x2a}.contains(rune),
      )) {
    return false;
  }
  final stem = rawName.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final upper = _revision3ProjectBuildAsciiUpper(stem);
  if (const <String>{
    'CON',
    'PRN',
    'AUX',
    'NUL',
    r'CLOCK$',
    r'CONIN$',
    r'CONOUT$',
  }.contains(upper)) {
    return false;
  }
  final reservedPort = upper.startsWith('COM') || upper.startsWith('LPT')
      ? upper.substring(3)
      : null;
  return reservedPort == null ||
      !const <String>{
        '1',
        '2',
        '3',
        '4',
        '5',
        '6',
        '7',
        '8',
        '9',
        '¹',
        '²',
        '³',
      }.contains(reservedPort);
}

int _revision3ProjectBuildStrictUtf8ByteLength(String value, String context) {
  var bytes = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    if (unit <= 0x7f) {
      bytes += 1;
    } else if (unit <= 0x7ff) {
      bytes += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException('$context contains invalid UTF-16');
      }
      final low = value.codeUnitAt(++index);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException('$context contains invalid UTF-16');
      }
      bytes += 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw FormatException('$context contains invalid UTF-16');
    } else {
      bytes += 3;
    }
  }
  return bytes;
}

String _revision3ProjectBuildAsciiUpper(String value) => String.fromCharCodes(
  value.runes.map((rune) => rune >= 0x61 && rune <= 0x7a ? rune - 0x20 : rune),
);

AuthoringRevision3ProjectBuildBlockReason? _revision3ProjectBuildVoiceReason(
  AuthoringRevision3VoiceBuildBlockReason reason,
) => switch (reason) {
  AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots => null,
  AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded,
  AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget =>
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved,
  AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget =>
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous,
  AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd =>
    AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified,
  AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing,
  AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved,
  AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeCodecUnqualified,
  AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded =>
    AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded,
};

bool _revision3ProjectBuildReasonBelongsToDomain(
  AuthoringRevision3ProjectBuildBlockReason reason,
  AuthoringRevision3ProjectBuildDomain domain,
) => switch (reason) {
  AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.localization,
  AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.dialog,
  AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported ||
  AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported ||
  AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded ||
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved ||
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous ||
  AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified ||
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing ||
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved ||
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeCodecUnqualified ||
  AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded =>
    domain == AuthoringRevision3ProjectBuildDomain.voice,
  AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.npc,
  AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.quest,
  AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.scripts,
  AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable =>
    domain == AuthoringRevision3ProjectBuildDomain.items,
  AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetPreservedComponentChanged ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetReviewedPreparationFailed ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetDerivedReplacementMismatch =>
    domain == AuthoringRevision3ProjectBuildDomain.dataAssets,
};

bool _revision3ProjectBuildDataAssetCategoryIsExact(
  AuthoringRevision3ProjectBuildBlocker blocker,
) => switch (blocker.reason) {
  AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported =>
    blocker.category ==
        AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport,
  AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite ||
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetPreservedComponentChanged ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetReviewedPreparationFailed ||
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetDerivedReplacementMismatch =>
    blocker.category ==
        AuthoringRevision3ProjectBuildBlockerCategory.authorProject,
  _ => false,
};

int _revision3ProjectBuildBlockerCompare(
  AuthoringRevision3ProjectBuildBlocker left,
  AuthoringRevision3ProjectBuildBlocker right,
) {
  final category = left.category.index.compareTo(right.category.index);
  if (category != 0) return category;
  final domain = left.domain.index.compareTo(right.domain.index);
  if (domain != 0) return domain;
  return left.reason.index.compareTo(right.reason.index);
}

int _revision3ProjectBuildExpectedBlockerCompare(
  _Revision3ProjectBuildExpectedBlocker left,
  _Revision3ProjectBuildExpectedBlocker right,
) {
  final category = left.category.index.compareTo(right.category.index);
  if (category != 0) return category;
  final domain = left.domain.index.compareTo(right.domain.index);
  if (domain != 0) return domain;
  return left.reason.index.compareTo(right.reason.index);
}

AuthoringRevision3ProjectBuildSeal _revision3ProjectBuildSealJson(
  Map<String, Object?> json,
) => _revision3ProjectBuildSealBytes(utf8.encode(jsonEncode(json)));

AuthoringRevision3ProjectBuildSeal _revision3ProjectBuildSealBytes(
  List<int> bytes,
) => AuthoringRevision3ProjectBuildSeal._(
  byteLength: bytes.length,
  sha256: crypto.sha256.convert(bytes).toString(),
);

AuthoringRevision3ProjectBuildOutcome _revision3ProjectBuildOutcome(
  String value,
) => switch (value) {
  'empty' => AuthoringRevision3ProjectBuildOutcome.empty,
  'blocked' => AuthoringRevision3ProjectBuildOutcome.blocked,
  'coverage_complete' => AuthoringRevision3ProjectBuildOutcome.coverageComplete,
  _ => throw const FormatException(
    'revision-3 project build plan has an unknown outcome',
  ),
};

AuthoringRevision3ProjectBuildDomain _revision3ProjectBuildDomain(
  String value,
) => switch (value) {
  'localization' => AuthoringRevision3ProjectBuildDomain.localization,
  'dialog' => AuthoringRevision3ProjectBuildDomain.dialog,
  'voice' => AuthoringRevision3ProjectBuildDomain.voice,
  'npc' => AuthoringRevision3ProjectBuildDomain.npc,
  'quest' => AuthoringRevision3ProjectBuildDomain.quest,
  'scripts' => AuthoringRevision3ProjectBuildDomain.scripts,
  'items' => AuthoringRevision3ProjectBuildDomain.items,
  'data_assets' => AuthoringRevision3ProjectBuildDomain.dataAssets,
  _ => throw const FormatException(
    'revision-3 project build plan has an unknown domain',
  ),
};

AuthoringRevision3ProjectBuildDomainStatus _revision3ProjectBuildDomainStatus(
  String value,
) => switch (value) {
  'not_present' => AuthoringRevision3ProjectBuildDomainStatus.notPresent,
  'ready' => AuthoringRevision3ProjectBuildDomainStatus.ready,
  'blocked' => AuthoringRevision3ProjectBuildDomainStatus.blocked,
  _ => throw const FormatException(
    'revision-3 project build plan has an unknown domain status',
  ),
};

AuthoringRevision3ProjectBuildBlockerCategory
_revision3ProjectBuildBlockerCategory(String value) => switch (value) {
  'author_project' =>
    AuthoringRevision3ProjectBuildBlockerCategory.authorProject,
  'toolkit_support' =>
    AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport,
  _ => throw const FormatException(
    'revision-3 project build plan has an unknown blocker category',
  ),
};

AuthoringRevision3ProjectBuildBlockReason _revision3ProjectBuildBlockReason(
  String value,
) => switch (value) {
  'localization_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable,
  'dialog_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable,
  'voice_project_name_unsupported' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported,
  'voice_line_label_unsupported' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported,
  'voice_slot_limit_exceeded' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded,
  'voice_target_unresolved' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved,
  'voice_target_ambiguous' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous,
  'voice_add_unqualified' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified,
  'voice_selected_take_missing' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing,
  'voice_selected_take_not_approved' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved,
  'voice_selected_take_codec_unqualified' =>
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeCodecUnqualified,
  'voice_payload_budget_exceeded' =>
    AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded,
  'npc_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable,
  'quest_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable,
  'script_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable,
  'item_patch_lowering_unavailable' =>
    AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable,
  'data_asset_target_unsupported' =>
    AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported,
  'data_asset_selector_mismatch' =>
    AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch,
  'data_asset_replacement_malformed' =>
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed,
  'data_asset_replacement_non_finite' =>
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite,
  'data_asset_replacement_non_positive' =>
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive,
  'data_asset_preserved_component_changed' =>
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetPreservedComponentChanged,
  'data_asset_reviewed_preparation_failed' =>
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetReviewedPreparationFailed,
  'data_asset_derived_replacement_mismatch' =>
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetDerivedReplacementMismatch,
  _ => throw const FormatException(
    'revision-3 project build plan has an unknown blocker reason',
  ),
};

extension on AuthoringRevision3ProjectBuildOutcome {
  String get _wireName => switch (this) {
    AuthoringRevision3ProjectBuildOutcome.empty => 'empty',
    AuthoringRevision3ProjectBuildOutcome.blocked => 'blocked',
    AuthoringRevision3ProjectBuildOutcome.coverageComplete =>
      'coverage_complete',
  };
}

extension on AuthoringRevision3ProjectBuildDomain {
  String get _wireName => switch (this) {
    AuthoringRevision3ProjectBuildDomain.localization => 'localization',
    AuthoringRevision3ProjectBuildDomain.dialog => 'dialog',
    AuthoringRevision3ProjectBuildDomain.voice => 'voice',
    AuthoringRevision3ProjectBuildDomain.npc => 'npc',
    AuthoringRevision3ProjectBuildDomain.quest => 'quest',
    AuthoringRevision3ProjectBuildDomain.scripts => 'scripts',
    AuthoringRevision3ProjectBuildDomain.items => 'items',
    AuthoringRevision3ProjectBuildDomain.dataAssets => 'data_assets',
  };
}

extension on AuthoringRevision3ProjectBuildDomainStatus {
  String get _wireName => switch (this) {
    AuthoringRevision3ProjectBuildDomainStatus.notPresent => 'not_present',
    AuthoringRevision3ProjectBuildDomainStatus.ready => 'ready',
    AuthoringRevision3ProjectBuildDomainStatus.blocked => 'blocked',
  };
}

extension on AuthoringRevision3ProjectBuildBlockerCategory {
  String get _wireName => switch (this) {
    AuthoringRevision3ProjectBuildBlockerCategory.authorProject =>
      'author_project',
    AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport =>
      'toolkit_support',
  };
}

extension on AuthoringRevision3ProjectBuildBlockReason {
  String get _wireName => switch (this) {
    AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable =>
      'localization_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable =>
      'dialog_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported =>
      'voice_project_name_unsupported',
    AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported =>
      'voice_line_label_unsupported',
    AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded =>
      'voice_slot_limit_exceeded',
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved =>
      'voice_target_unresolved',
    AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous =>
      'voice_target_ambiguous',
    AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified =>
      'voice_add_unqualified',
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing =>
      'voice_selected_take_missing',
    AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved =>
      'voice_selected_take_not_approved',
    AuthoringRevision3ProjectBuildBlockReason
        .voiceSelectedTakeCodecUnqualified =>
      'voice_selected_take_codec_unqualified',
    AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded =>
      'voice_payload_budget_exceeded',
    AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable =>
      'npc_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable =>
      'quest_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable =>
      'script_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable =>
      'item_patch_lowering_unavailable',
    AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported =>
      'data_asset_target_unsupported',
    AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch =>
      'data_asset_selector_mismatch',
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed =>
      'data_asset_replacement_malformed',
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite =>
      'data_asset_replacement_non_finite',
    AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive =>
      'data_asset_replacement_non_positive',
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetPreservedComponentChanged =>
      'data_asset_preserved_component_changed',
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetReviewedPreparationFailed =>
      'data_asset_reviewed_preparation_failed',
    AuthoringRevision3ProjectBuildBlockReason
        .dataAssetDerivedReplacementMismatch =>
      'data_asset_derived_replacement_mismatch',
  };
}
