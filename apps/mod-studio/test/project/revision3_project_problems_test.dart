import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_project_problems.dart';

import '../support/revision3_project_problems_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('keeps readiness scope-specific for an exact empty project', () {
    final fixture = revision3ProjectProblemsEmptyFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: true,
    );

    expect(report.projectId, fixture.projectId);
    expect(report.projectRevision, fixture.projectRevision);
    expect(report.problems, isEmpty);
    expect(report.dataAssetRegistryAvailable, isTrue);
    expect(report.dataAssetStageCount, 0);
    expect(
      report.readinessFor(Revision3ProjectProblemScope.referenceIntegrity),
      Revision3ProjectProblemReadiness.clear,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.compilerEvidence),
      Revision3ProjectProblemReadiness.notEvaluated,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.managedBuild),
      Revision3ProjectProblemReadiness.blocked,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.runtime),
      Revision3ProjectProblemReadiness.unqualified,
    );
    expect(
      report.assessments
          .singleWhere(
            (item) =>
                item.scope == Revision3ProjectProblemScope.compilerEvidence,
          )
          .primaryTarget,
      isNull,
      reason: 'structural project verification is not compiler evidence',
    );
  });

  test('projects every unresolved exact reference and no Voice heuristics', () {
    final fixture = revision3ProjectProblemsFilterFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: true,
    );

    expect(report.problems, hasLength(3));
    expect(
      report.problems.map((item) => item.code),
      orderedEquals(const [
        Revision3ProjectProblemCode.missingEntityReference,
        Revision3ProjectProblemCode.missingAssetReference,
        Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
      ]),
    );
    expect(
      report.countForCategory(Revision3ProjectProblemCategory.references),
      2,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.referenceIntegrity),
      Revision3ProjectProblemReadiness.issues,
    );

    final entityProblem = report.problems.first;
    expect(entityProblem.primaryTarget.identity, revision3ProjectProblemsNpcId);
    expect(
      (entityProblem.details as Revision3EntityReferenceProblemDetails)
          .targetEntityId,
      revision3ProjectProblemsMissingModuleId,
    );

    final assetProblem = report.problems[1];
    expect(
      (assetProblem.details as Revision3AssetReferenceProblemDetails).sha256,
      revision3ProjectProblemsMissingAudioSha256,
    );
    expect(
      assetProblem.relatedTargets,
      isEmpty,
      reason: 'a missing asset cannot be offered as an openable target',
    );

    final stageProblem = report.problems.last;
    final stageDetails =
        stageProblem.details as Revision3DataAssetStageProblemDetails;
    expect(stageDetails.targetPath, fixture.dataAssetStage!.targetPath);
    expect(stageDetails.manifestSha256, fixture.dataAssetManifestSha256);
    expect(
      stageProblem.primaryTarget.kind,
      Revision3ProjectProblemTargetKind.dataAssetStage,
    );
  });

  test('projects one exact source with canonical report parity', () {
    final contentIndex = _unsortedSourceProblemsFixture();
    final report = Revision3ProjectProblemBuilder.build(
      contentIndex,
      dataAssetStages: const [],
      gameConfigured: false,
    );
    final source = contentIndex.entityById(revision3VoiceContentSlotId)!;

    final scoped =
        Revision3ProjectProblemBuilder.buildReferenceProblemsForSourceEntity(
          contentIndex,
          sourceEntityId: source.id,
        );
    final reportSubset = report.problems
        .where(
          (problem) =>
              problem.primaryTarget.kind ==
                  Revision3ProjectProblemTargetKind.entity &&
              problem.primaryTarget.identity == source.id &&
              problem.primaryTarget.entityKind == source.kind,
        )
        .toList(growable: false);

    expect(
      scoped.map((problem) => problem.id),
      reportSubset.map((item) => item.id),
    );
    expect(
      scoped.map((problem) => problem.code),
      orderedEquals(reportSubset.map((problem) => problem.code)),
    );
    expect(scoped, hasLength(2));
    expect(
      scoped.map((problem) => problem.code),
      orderedEquals(const [
        Revision3ProjectProblemCode.foreignEntityReference,
        Revision3ProjectProblemCode.missingEntityReference,
      ]),
    );
    expect(scoped.every((problem) => problem.relatedTargets.isEmpty), isTrue);
    expect(
      scoped.every(
        (problem) =>
            problem.category == Revision3ProjectProblemCategory.references &&
            problem.primaryTarget.identity == source.id,
      ),
      isTrue,
    );
    expect(() => scoped.clear(), throwsUnsupportedError);
  });

  test('source projection excludes other sources and global problems', () {
    final fixture = revision3ProjectProblemsFilterFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: false,
    );
    final assetSource = fixture.contentIndex.entities.singleWhere(
      (entity) => entity.assetReferences.any(
        (reference) =>
            reference.sha256 == revision3ProjectProblemsMissingAudioSha256,
      ),
    );

    final scoped =
        Revision3ProjectProblemBuilder.buildReferenceProblemsForSourceEntity(
          fixture.contentIndex,
          sourceEntityId: assetSource.id,
        );

    expect(scoped, hasLength(1));
    expect(
      scoped.single.code,
      Revision3ProjectProblemCode.missingAssetReference,
    );
    expect(scoped.single.primaryTarget.identity, assetSource.id);
    expect(scoped.single.relatedTargets, isEmpty);
    expect(
      report.problems.map((problem) => problem.code),
      containsAll(const [
        Revision3ProjectProblemCode.missingEntityReference,
        Revision3ProjectProblemCode.gameNotConfigured,
        Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
      ]),
    );
    expect(
      scoped.any(
        (problem) =>
            problem.code ==
                Revision3ProjectProblemCode.dataAssetStageOfflineOnly ||
            problem.code == Revision3ProjectProblemCode.gameNotConfigured,
      ),
      isFalse,
    );
  });

  test('source projection rejects unknown source instead of false-clean', () {
    final fixture = revision3ProjectProblemsCleanFixture();

    expect(
      () =>
          Revision3ProjectProblemBuilder.buildReferenceProblemsForSourceEntity(
            fixture.contentIndex,
            sourceEntityId: revision3ProjectProblemsMissingModuleId,
          ),
      throwsArgumentError,
    );
  });

  test('clean source projection is an immutable empty list', () {
    final fixture = revision3ProjectProblemsCleanFixture();
    final source = fixture.contentIndex.entities.first;

    final scoped =
        Revision3ProjectProblemBuilder.buildReferenceProblemsForSourceEntity(
          fixture.contentIndex,
          sourceEntityId: source.id,
        );

    expect(scoped, isEmpty);
    expect(() => scoped.clear(), throwsUnsupportedError);
  });

  test('reports unavailable stage registry as partial exact evidence', () {
    final fixture = revision3ProjectProblemsCleanFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: null,
      gameConfigured: true,
    );

    expect(report.dataAssetRegistryAvailable, isFalse);
    expect(report.dataAssetStageCount, 0);
    expect(report.problems, hasLength(1));
    expect(
      report.problems.single.code,
      Revision3ProjectProblemCode.dataAssetRegistryUnavailable,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.dataAssetRegistry),
      Revision3ProjectProblemReadiness.unavailable,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.referenceIntegrity),
      Revision3ProjectProblemReadiness.clear,
    );
  });

  test('routes missing game configuration only to settings', () {
    final fixture = revision3ProjectProblemsCleanFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: false,
    );

    final problem = report.problems.single;
    expect(problem.code, Revision3ProjectProblemCode.gameNotConfigured);
    expect(
      problem.primaryTarget.kind,
      Revision3ProjectProblemTargetKind.settings,
    );
    expect(
      report.readinessFor(Revision3ProjectProblemScope.gameConfiguration),
      Revision3ProjectProblemReadiness.issues,
    );
  });

  test('accepts a retained DataAsset stage from an earlier revision', () {
    final retained = revision3ProjectProblemsRetainedDataAssetFixture(
      projectRevision: 8,
    );

    final report = Revision3ProjectProblemBuilder.build(
      retained.contentIndex,
      dataAssetStages: retained.dataAssetStages,
      gameConfigured: true,
    );

    expect(retained.dataAssetStage!.stagedProjectRevision, 7);
    expect(report.dataAssetStageCount, 1);
    expect(
      report.problems.single.code,
      Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
    );
  });

  test('rejects a retained stage whose manifest left current content', () {
    final current = revision3ProjectProblemsEmptyFixture(projectRevision: 8);
    final retained = revision3ProjectProblemsRetainedDataAssetFixture(
      projectRevision: 8,
    );

    expect(
      () => Revision3ProjectProblemBuilder.build(
        current.contentIndex,
        dataAssetStages: retained.dataAssetStages,
        gameConfigured: true,
      ),
      throwsArgumentError,
    );
  });

  test('rejects a DataAsset registry from a future revision', () {
    final current = revision3ProjectProblemsCleanFixture(projectRevision: 7);
    final future = revision3ProjectProblemsFilterFixture(projectRevision: 8);

    expect(
      () => Revision3ProjectProblemBuilder.build(
        current.contentIndex,
        dataAssetStages: future.dataAssetStages,
        gameConfigured: true,
      ),
      throwsArgumentError,
    );
  });

  test('report collections are immutable and assessments are unique', () {
    final fixture = revision3ProjectProblemsFilterFixture();
    final report = Revision3ProjectProblemBuilder.build(
      fixture.contentIndex,
      dataAssetStages: fixture.dataAssetStages,
      gameConfigured: false,
    );

    expect(() => report.problems.clear(), throwsUnsupportedError);
    expect(() => report.assessments.clear(), throwsUnsupportedError);
    expect(
      report.assessments.map((item) => item.scope).toSet(),
      hasLength(Revision3ProjectProblemScope.values.length),
    );
  });
}

Revision3ContentIndex _unsortedSourceProblemsFixture() {
  final json = revision3VoiceContentIndexJsonFixture(
    existingDeSlot: true,
    existingSlotCandidateCount: 2,
  );
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  final slot = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentSlotId,
  );
  final candidates = (slot['references']! as List)
      .cast<Map<String, Object?>>()
      .where((reference) => reference['role'] == 'voice_candidate')
      .toList(growable: false);

  final missingTarget = candidates[0]['target']! as Map<String, Object?>;
  missingTarget['entity_id'] = revision3ProjectProblemsMissingModuleId;
  candidates[0]['resolution'] = 'missing_entity';

  final foreignTarget = candidates[1]['target']! as Map<String, Object?>;
  foreignTarget['project_id'] = '99999999999999999999999999999999';
  candidates[1]['resolution'] = 'foreign_project';

  return Revision3ContentIndex.fromJsonObject(json);
}
