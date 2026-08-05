import 'package:flutter/foundation.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

/// Stable diagnostic kinds. Presentation copy is deliberately supplied by
/// the caller instead of being embedded in this exact-current domain report.
enum Revision3ProjectProblemCode {
  foreignEntityReference,
  missingEntityReference,
  entityKindMismatch,
  missingAssetReference,
  assetByteLengthMismatch,
  assetMediaTypeMismatch,
  gameNotConfigured,
  dataAssetRegistryUnavailable,
  dataAssetStageOfflineOnly,
}

enum Revision3ProjectProblemCategory { references, setup, dataAssets }

enum Revision3ProjectProblemSeverity { information, warning, blocking }

enum Revision3ProjectProblemEvidence {
  exactContentIndex,
  exactDataAssetRegistry,
  configurationState,
  sourceUnavailable,
  capabilityBoundary,
}

enum Revision3ProjectProblemImpact {
  authoring,
  sourceInspection,
  compilerCheck,
  build,
  runtime,
}

/// Independently assessed scopes. There is intentionally no project-wide
/// readiness value because these capabilities have different proof states.
enum Revision3ProjectProblemScope {
  referenceIntegrity,
  dataAssetRegistry,
  gameConfiguration,
  compilerEvidence,
  managedBuild,
  runtime,
}

/// A scope can be clear without implying that another scope is usable.
enum Revision3ProjectProblemReadiness {
  clear,
  issues,
  unavailable,
  notEvaluated,
  blocked,
  unqualified,
}

enum Revision3ProjectProblemTargetKind {
  project,
  entity,
  asset,
  dataAssetStage,
  settings,
}

@immutable
final class Revision3ProjectProblemTarget {
  const Revision3ProjectProblemTarget._({
    required this.kind,
    required this.identity,
    this.entityKind,
  });

  const Revision3ProjectProblemTarget.project(String projectId)
    : this._(
        kind: Revision3ProjectProblemTargetKind.project,
        identity: projectId,
      );

  const Revision3ProjectProblemTarget.entity(
    String entityId,
    Revision3ContentEntityKind entityKind,
  ) : this._(
        kind: Revision3ProjectProblemTargetKind.entity,
        identity: entityId,
        entityKind: entityKind,
      );

  const Revision3ProjectProblemTarget.asset(String sha256)
    : this._(kind: Revision3ProjectProblemTargetKind.asset, identity: sha256);

  const Revision3ProjectProblemTarget.dataAssetStage(String targetPath)
    : this._(
        kind: Revision3ProjectProblemTargetKind.dataAssetStage,
        identity: targetPath,
      );

  const Revision3ProjectProblemTarget.settings()
    : this._(
        kind: Revision3ProjectProblemTargetKind.settings,
        identity: 'game-installation',
      );

  final Revision3ProjectProblemTargetKind kind;
  final String identity;
  final Revision3ContentEntityKind? entityKind;
}

sealed class Revision3ProjectProblemDetails {
  const Revision3ProjectProblemDetails();
}

@immutable
final class Revision3EntityReferenceProblemDetails
    extends Revision3ProjectProblemDetails {
  const Revision3EntityReferenceProblemDetails({
    required this.sourceEntityId,
    required this.sourceDisplayName,
    required this.sourceKind,
    required this.role,
    required this.qualifier,
    required this.targetProjectId,
    required this.targetEntityId,
    required this.expectedKind,
    required this.resolution,
  });

  final String sourceEntityId;
  final String sourceDisplayName;
  final Revision3ContentEntityKind sourceKind;
  final String role;
  final String? qualifier;
  final String targetProjectId;
  final String targetEntityId;
  final Revision3ContentEntityKind expectedKind;
  final Revision3ContentReferenceResolution resolution;
}

@immutable
final class Revision3AssetReferenceProblemDetails
    extends Revision3ProjectProblemDetails {
  const Revision3AssetReferenceProblemDetails({
    required this.sourceEntityId,
    required this.sourceDisplayName,
    required this.sourceKind,
    required this.role,
    required this.sha256,
    required this.logicalName,
    required this.expectedByteLength,
    required this.expectedMediaType,
    required this.resolution,
  });

  final String sourceEntityId;
  final String sourceDisplayName;
  final Revision3ContentEntityKind sourceKind;
  final String role;
  final String sha256;
  final String? logicalName;
  final int expectedByteLength;
  final String expectedMediaType;
  final Revision3ContentAssetReferenceResolution resolution;
}

@immutable
final class Revision3DataAssetStageProblemDetails
    extends Revision3ProjectProblemDetails {
  const Revision3DataAssetStageProblemDetails({
    required this.targetPath,
    required this.selectorKind,
    required this.replacementByteLength,
    required this.manifestSha256,
  });

  final String targetPath;
  final String selectorKind;
  final int replacementByteLength;
  final String manifestSha256;
}

@immutable
final class Revision3CapabilityBoundaryProblemDetails
    extends Revision3ProjectProblemDetails {
  const Revision3CapabilityBoundaryProblemDetails({required this.scope});

  final Revision3ProjectProblemScope scope;
}

@immutable
final class Revision3ProjectProblem {
  Revision3ProjectProblem({
    required this.id,
    required this.code,
    required this.category,
    required this.severity,
    required this.evidence,
    required List<Revision3ProjectProblemImpact> impacts,
    required this.primaryTarget,
    required List<Revision3ProjectProblemTarget> relatedTargets,
    required this.details,
    required List<String> searchTerms,
  }) : assert(id != ''),
       impacts = List<Revision3ProjectProblemImpact>.unmodifiable(impacts),
       relatedTargets = List<Revision3ProjectProblemTarget>.unmodifiable(
         relatedTargets,
       ),
       searchTerms = List<String>.unmodifiable(searchTerms);

  final String id;
  final Revision3ProjectProblemCode code;
  final Revision3ProjectProblemCategory category;
  final Revision3ProjectProblemSeverity severity;
  final Revision3ProjectProblemEvidence evidence;
  final List<Revision3ProjectProblemImpact> impacts;
  final Revision3ProjectProblemTarget primaryTarget;
  final List<Revision3ProjectProblemTarget> relatedTargets;
  final Revision3ProjectProblemDetails details;
  final List<String> searchTerms;
}

@immutable
final class Revision3ProjectProblemAssessment {
  Revision3ProjectProblemAssessment({
    required this.scope,
    required this.readiness,
    required this.evidence,
    required List<Revision3ProjectProblemImpact> impacts,
    required this.problemCount,
    required this.primaryTarget,
  }) : assert(problemCount >= 0),
       impacts = List<Revision3ProjectProblemImpact>.unmodifiable(impacts);

  final Revision3ProjectProblemScope scope;
  final Revision3ProjectProblemReadiness readiness;
  final Revision3ProjectProblemEvidence evidence;
  final List<Revision3ProjectProblemImpact> impacts;
  final int problemCount;
  final Revision3ProjectProblemTarget? primaryTarget;
}

@immutable
final class Revision3ProjectProblemReport {
  Revision3ProjectProblemReport._({
    required this.projectId,
    required this.projectRevision,
    required this.gameConfigured,
    required this.dataAssetRegistryAvailable,
    required this.dataAssetStageCount,
    required List<Revision3ProjectProblem> problems,
    required List<Revision3ProjectProblemAssessment> assessments,
  }) : problems = List<Revision3ProjectProblem>.unmodifiable(problems),
       assessments = List<Revision3ProjectProblemAssessment>.unmodifiable(
         assessments,
       ),
       _assessmentsByScope = Map.unmodifiable({
         for (final assessment in assessments) assessment.scope: assessment,
       });

  final String projectId;
  final int projectRevision;
  final bool gameConfigured;
  final bool dataAssetRegistryAvailable;
  final int dataAssetStageCount;
  final List<Revision3ProjectProblem> problems;
  final List<Revision3ProjectProblemAssessment> assessments;
  final Map<Revision3ProjectProblemScope, Revision3ProjectProblemAssessment>
  _assessmentsByScope;

  int countForCategory(Revision3ProjectProblemCategory category) =>
      problems.where((problem) => problem.category == category).length;

  Revision3ProjectProblemReadiness readinessFor(
    Revision3ProjectProblemScope scope,
  ) => _assessmentsByScope[scope]!.readiness;
}

/// Derives project Problems only from validated exact-current projections and
/// explicit capability boundaries. It intentionally does not infer Voice
/// readiness, compiler success, buildability, or game behavior.
abstract final class Revision3ProjectProblemBuilder {
  static Revision3ProjectProblemReport build(
    Revision3ContentIndex contentIndex, {
    List<AuthoringRevision3DataAssetStage>? dataAssetStages,
    required bool gameConfigured,
  }) {
    _validateStages(contentIndex, dataAssetStages);
    final problems = <Revision3ProjectProblem>[];

    for (final entity in contentIndex.entities) {
      for (final reference in entity.references) {
        if (reference.resolution ==
            Revision3ContentReferenceResolution.resolved) {
          continue;
        }
        problems.add(_entityReferenceProblem(entity, reference));
      }
      for (final reference in entity.assetReferences) {
        if (reference.resolution ==
            Revision3ContentAssetReferenceResolution.resolved) {
          continue;
        }
        problems.add(_assetReferenceProblem(entity, reference));
      }
    }

    if (!gameConfigured) {
      problems.add(
        Revision3ProjectProblem(
          id: 'setup:game-installation',
          code: Revision3ProjectProblemCode.gameNotConfigured,
          category: Revision3ProjectProblemCategory.setup,
          severity: Revision3ProjectProblemSeverity.warning,
          evidence: Revision3ProjectProblemEvidence.configurationState,
          impacts: const [
            Revision3ProjectProblemImpact.sourceInspection,
            Revision3ProjectProblemImpact.compilerCheck,
            Revision3ProjectProblemImpact.runtime,
          ],
          primaryTarget: const Revision3ProjectProblemTarget.settings(),
          relatedTargets: const [],
          details: const Revision3CapabilityBoundaryProblemDetails(
            scope: Revision3ProjectProblemScope.gameConfiguration,
          ),
          searchTerms: const ['game', 'installation', 'settings'],
        ),
      );
    }

    if (dataAssetStages == null) {
      problems.add(
        Revision3ProjectProblem(
          id: 'dataasset:registry-unavailable',
          code: Revision3ProjectProblemCode.dataAssetRegistryUnavailable,
          category: Revision3ProjectProblemCategory.dataAssets,
          severity: Revision3ProjectProblemSeverity.warning,
          evidence: Revision3ProjectProblemEvidence.sourceUnavailable,
          impacts: const [Revision3ProjectProblemImpact.authoring],
          primaryTarget: Revision3ProjectProblemTarget.project(
            contentIndex.projectId,
          ),
          relatedTargets: const [],
          details: const Revision3CapabilityBoundaryProblemDetails(
            scope: Revision3ProjectProblemScope.dataAssetRegistry,
          ),
          searchTerms: const ['dataasset', 'registry', 'unavailable'],
        ),
      );
    } else {
      for (final stage in dataAssetStages) {
        problems.add(_stageBoundaryProblem(stage));
      }
    }

    problems.sort(_compareProblems);
    final referenceCount = problems
        .where(
          (problem) =>
              problem.category == Revision3ProjectProblemCategory.references,
        )
        .length;
    final setupCount = gameConfigured ? 0 : 1;
    final stageCount = dataAssetStages?.length ?? 0;

    final assessments = <Revision3ProjectProblemAssessment>[
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.referenceIntegrity,
        readiness: referenceCount == 0
            ? Revision3ProjectProblemReadiness.clear
            : Revision3ProjectProblemReadiness.issues,
        evidence: Revision3ProjectProblemEvidence.exactContentIndex,
        impacts: const [
          Revision3ProjectProblemImpact.authoring,
          Revision3ProjectProblemImpact.build,
        ],
        problemCount: referenceCount,
        primaryTarget: null,
      ),
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.dataAssetRegistry,
        readiness: dataAssetStages == null
            ? Revision3ProjectProblemReadiness.unavailable
            : Revision3ProjectProblemReadiness.clear,
        evidence: dataAssetStages == null
            ? Revision3ProjectProblemEvidence.sourceUnavailable
            : Revision3ProjectProblemEvidence.exactDataAssetRegistry,
        impacts: const [Revision3ProjectProblemImpact.authoring],
        problemCount: dataAssetStages == null ? 1 : 0,
        primaryTarget: null,
      ),
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.gameConfiguration,
        readiness: gameConfigured
            ? Revision3ProjectProblemReadiness.clear
            : Revision3ProjectProblemReadiness.issues,
        evidence: Revision3ProjectProblemEvidence.configurationState,
        impacts: const [
          Revision3ProjectProblemImpact.sourceInspection,
          Revision3ProjectProblemImpact.compilerCheck,
          Revision3ProjectProblemImpact.runtime,
        ],
        problemCount: setupCount,
        primaryTarget: gameConfigured
            ? null
            : const Revision3ProjectProblemTarget.settings(),
      ),
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.compilerEvidence,
        readiness: Revision3ProjectProblemReadiness.notEvaluated,
        evidence: Revision3ProjectProblemEvidence.capabilityBoundary,
        impacts: const [Revision3ProjectProblemImpact.compilerCheck],
        problemCount: 0,
        primaryTarget: null,
      ),
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.managedBuild,
        readiness: Revision3ProjectProblemReadiness.blocked,
        evidence: Revision3ProjectProblemEvidence.capabilityBoundary,
        impacts: const [Revision3ProjectProblemImpact.build],
        problemCount: 0,
        primaryTarget: null,
      ),
      Revision3ProjectProblemAssessment(
        scope: Revision3ProjectProblemScope.runtime,
        readiness: Revision3ProjectProblemReadiness.unqualified,
        evidence: Revision3ProjectProblemEvidence.capabilityBoundary,
        impacts: const [Revision3ProjectProblemImpact.runtime],
        problemCount: 0,
        primaryTarget: null,
      ),
    ];

    return Revision3ProjectProblemReport._(
      projectId: contentIndex.projectId,
      projectRevision: contentIndex.projectRevision,
      gameConfigured: gameConfigured,
      dataAssetRegistryAvailable: dataAssetStages != null,
      dataAssetStageCount: stageCount,
      problems: problems,
      assessments: assessments,
    );
  }
}

Revision3ProjectProblem _entityReferenceProblem(
  Revision3ContentEntity source,
  Revision3ContentReference reference,
) {
  final code = switch (reference.resolution) {
    Revision3ContentReferenceResolution.foreignProject =>
      Revision3ProjectProblemCode.foreignEntityReference,
    Revision3ContentReferenceResolution.missingEntity =>
      Revision3ProjectProblemCode.missingEntityReference,
    Revision3ContentReferenceResolution.kindMismatch =>
      Revision3ProjectProblemCode.entityKindMismatch,
    Revision3ContentReferenceResolution.resolved => throw StateError(
      'resolved entity reference cannot become a problem',
    ),
  };
  return Revision3ProjectProblem(
    id:
        'entity-ref:${source.id}:${reference.role}:${reference.qualifier ?? '-'}:'
        '${reference.target.projectId}:${reference.target.entityId}',
    code: code,
    category: Revision3ProjectProblemCategory.references,
    severity: Revision3ProjectProblemSeverity.blocking,
    evidence: Revision3ProjectProblemEvidence.exactContentIndex,
    impacts: const [
      Revision3ProjectProblemImpact.authoring,
      Revision3ProjectProblemImpact.build,
    ],
    primaryTarget: Revision3ProjectProblemTarget.entity(source.id, source.kind),
    relatedTargets: const [],
    details: Revision3EntityReferenceProblemDetails(
      sourceEntityId: source.id,
      sourceDisplayName: source.displayName,
      sourceKind: source.kind,
      role: reference.role,
      qualifier: reference.qualifier,
      targetProjectId: reference.target.projectId,
      targetEntityId: reference.target.entityId,
      expectedKind: reference.target.expectedKind,
      resolution: reference.resolution,
    ),
    searchTerms: [
      source.displayName,
      source.id,
      source.kind.wireName,
      reference.role,
      ?reference.qualifier,
      reference.target.projectId,
      reference.target.entityId,
      reference.target.expectedKind.wireName,
      reference.resolution.wireName,
    ],
  );
}

Revision3ProjectProblem _assetReferenceProblem(
  Revision3ContentEntity source,
  Revision3ContentAssetReference reference,
) {
  final code = switch (reference.resolution) {
    Revision3ContentAssetReferenceResolution.missingAsset =>
      Revision3ProjectProblemCode.missingAssetReference,
    Revision3ContentAssetReferenceResolution.byteLengthMismatch =>
      Revision3ProjectProblemCode.assetByteLengthMismatch,
    Revision3ContentAssetReferenceResolution.mediaTypeMismatch =>
      Revision3ProjectProblemCode.assetMediaTypeMismatch,
    Revision3ContentAssetReferenceResolution.resolved => throw StateError(
      'resolved asset reference cannot become a problem',
    ),
  };
  final related =
      reference.resolution ==
          Revision3ContentAssetReferenceResolution.missingAsset
      ? <Revision3ProjectProblemTarget>[]
      : <Revision3ProjectProblemTarget>[
          Revision3ProjectProblemTarget.asset(reference.sha256),
        ];
  return Revision3ProjectProblem(
    id: 'asset-ref:${source.id}:${reference.role}:${reference.sha256}',
    code: code,
    category: Revision3ProjectProblemCategory.references,
    severity: Revision3ProjectProblemSeverity.blocking,
    evidence: Revision3ProjectProblemEvidence.exactContentIndex,
    impacts: const [
      Revision3ProjectProblemImpact.authoring,
      Revision3ProjectProblemImpact.build,
    ],
    primaryTarget: Revision3ProjectProblemTarget.entity(source.id, source.kind),
    relatedTargets: related,
    details: Revision3AssetReferenceProblemDetails(
      sourceEntityId: source.id,
      sourceDisplayName: source.displayName,
      sourceKind: source.kind,
      role: reference.role,
      sha256: reference.sha256,
      logicalName: reference.logicalName,
      expectedByteLength: reference.byteLength,
      expectedMediaType: reference.expectedMediaType,
      resolution: reference.resolution,
    ),
    searchTerms: [
      source.displayName,
      source.id,
      source.kind.wireName,
      reference.role,
      reference.sha256,
      ?reference.logicalName,
      reference.expectedMediaType,
      reference.resolution.wireName,
    ],
  );
}

Revision3ProjectProblem _stageBoundaryProblem(
  AuthoringRevision3DataAssetStage stage,
) => Revision3ProjectProblem(
  id: 'dataasset-stage:${stage.targetPath}',
  code: Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
  category: Revision3ProjectProblemCategory.dataAssets,
  severity: Revision3ProjectProblemSeverity.warning,
  evidence: Revision3ProjectProblemEvidence.capabilityBoundary,
  impacts: const [
    Revision3ProjectProblemImpact.build,
    Revision3ProjectProblemImpact.runtime,
  ],
  primaryTarget: Revision3ProjectProblemTarget.dataAssetStage(stage.targetPath),
  relatedTargets: [
    Revision3ProjectProblemTarget.asset(stage.manifestAsset.sha256),
  ],
  details: Revision3DataAssetStageProblemDetails(
    targetPath: stage.targetPath,
    selectorKind: stage.selectorKind,
    replacementByteLength: stage.replacementByteLength,
    manifestSha256: stage.manifestAsset.sha256,
  ),
  searchTerms: [
    stage.targetPath,
    stage.selectorKind,
    stage.manifestAsset.sha256,
    'dataasset',
  ],
);

void _validateStages(
  Revision3ContentIndex index,
  List<AuthoringRevision3DataAssetStage>? stages,
) {
  if (stages == null) return;
  final targets = <String>{};
  for (final stage in stages) {
    final manifestAsset = index.assetBySha256(stage.manifestAsset.sha256);
    if (stage.projectId != index.projectId ||
        stage.stagedProjectRevision > index.projectRevision ||
        stage.projectTargetExecutable.sha256 != index.targetExecutableSha256 ||
        stage.projectTargetExecutable.byteLength !=
            index.targetExecutableByteLength ||
        manifestAsset == null ||
        manifestAsset.byteLength != stage.manifestAsset.byteLength ||
        manifestAsset.assetClass !=
            Revision3ContentAssetClass.dataAssetStageManifest ||
        !targets.add(stage.targetPath)) {
      throw ArgumentError.value(
        stages,
        'dataAssetStages',
        'must be unique and match the exact content checkpoint',
      );
    }
  }
}

int _compareProblems(
  Revision3ProjectProblem left,
  Revision3ProjectProblem right,
) {
  final category = left.category.index.compareTo(right.category.index);
  if (category != 0) return category;
  final severity = right.severity.index.compareTo(left.severity.index);
  if (severity != 0) return severity;
  final code = left.code.index.compareTo(right.code.index);
  if (code != 0) return code;
  return left.id.compareTo(right.id);
}
