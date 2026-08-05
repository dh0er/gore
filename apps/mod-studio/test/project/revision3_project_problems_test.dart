import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_problems.dart';

import '../support/revision3_project_problems_fixture.dart';

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
