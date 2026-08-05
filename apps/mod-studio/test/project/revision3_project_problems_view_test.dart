import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_project_problems.dart';
import 'package:gore_mod/project/revision3_project_problems_view.dart';

import '../support/revision3_project_problems_fixture.dart';

const _copy = Revision3ProjectProblemsCopy(
  title: 'Project Problems',
  description: 'Exact project evidence only.',
  scopeNotice: 'Clear references do not prove build or runtime readiness.',
  refreshTooltip: 'Refresh exact evidence',
  loadingSemanticsLabel: 'Loading exact Problems',
  loadErrorSemanticsLabel: 'Problems unavailable',
  loadErrorTitle: 'Problems could not be loaded',
  loadErrorDescription: 'Retry the exact current project.',
  retryLabel: 'Retry Problems',
  partialTitle: 'Partial evidence.',
  dataAssetsUnavailableDescription: 'DataAsset registry unavailable.',
  overviewHeading: 'Capability scopes',
  scopeTitle: _scopeTitle,
  scopeDescription: _scopeDescription,
  readinessName: _readinessName,
  evidenceName: _evidenceName,
  problemTitle: _problemTitle,
  problemDescription: _problemDescription,
  categoryName: _categoryName,
  severityName: _severityName,
  searchLabel: 'Search Problems',
  clearSearchTooltip: 'Clear Problems search',
  filterAllLabel: 'All Problems',
  listHeading: 'Current Problems',
  emptyTitle: 'No scoped Problems found',
  emptyDescription: 'Exact references and available registries are clear.',
  emptyBoundaryDescription: 'Build remains blocked and runtime unqualified.',
  filteredEmptyTitle: 'No matching Problems',
  filteredEmptyDescription: 'Change the search or category.',
  selectProblemTitle: 'Select a Problem',
  selectProblemDescription: 'Review exact evidence and safe navigation.',
  detailHeading: 'Problem detail',
  closeDetailTooltip: 'Close Problem detail',
  categoryLabel: 'Category',
  severityLabel: 'Severity',
  sourceLabel: 'Evidence',
  openEntityLabel: 'Open source entity',
  openAssetLabel: 'Open source asset',
  openDataAssetStageLabel: 'Open DataAsset edits',
  openSettingsLabel: 'Open settings',
  verifyCurrentProjectLabel: 'Verify current project',
  actionFailedMessage: 'Navigation failed safely.',
  actionInProgressSemanticsLabel: 'Navigation in progress',
);

void main() {
  testWidgets('loads exact sources and retries a sanitized content error', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 650));
    final fixture = revision3ProjectProblemsEmptyFixture();
    var contentCalls = 0;
    var stageCalls = 0;

    await _pumpView(
      tester,
      fixture: fixture,
      loadContent: () async {
        contentCalls++;
        if (contentCalls == 1) throw StateError(r'C:\private\project');
        return fixture.contentIndex;
      },
      loadStages: () async {
        stageCalls++;
        return fixture.dataAssetStages;
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-problems-error')),
      findsOneWidget,
    );
    expect(find.textContaining('private'), findsNothing);

    await tester.tap(find.byKey(const Key('revision3-project-problems-retry')));
    await tester.pumpAndSettle();

    expect(contentCalls, 2);
    expect(stageCalls, 2);
    expect(
      find.byKey(const Key('revision3-project-problems-empty')),
      findsOneWidget,
    );
  });

  testWidgets(
    'keeps exact content visible when DataAsset registry is partial',
    (tester) async {
      await _setSurface(tester, const Size(900, 650));
      final fixture = revision3ProjectProblemsCleanFixture();
      await _pumpView(
        tester,
        fixture: fixture,
        loadStages: () async => throw StateError('registry offline'),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-project-problems-partial')),
        findsOneWidget,
      );
      expect(
        find.textContaining('DataAsset registry unavailable.'),
        findsOneWidget,
      );
      expect(
        find.text(
          'Problem '
          '${Revision3ProjectProblemCode.dataAssetRegistryUnavailable.name}',
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-problems-error')),
        findsNothing,
      );
    },
  );

  testWidgets('keeps a retained earlier DataAsset stage actionable', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1280, 1200));
    final retained = revision3ProjectProblemsRetainedDataAssetFixture(
      projectRevision: 8,
    );
    final stage = retained.dataAssetStage!;
    String? openedTarget;

    await _pumpView(
      tester,
      fixture: retained,
      actions: Revision3ProjectProblemsActions(
        openDataAssetStage: (value) => openedTarget = value,
      ),
    );
    await tester.pumpAndSettle();

    expect(stage.stagedProjectRevision, 7);
    expect(
      find.byKey(const Key('revision3-project-problems-partial')),
      findsNothing,
    );
    expect(
      find.text(
        'Problem ${Revision3ProjectProblemCode.dataAssetStageOfflineOnly.name}',
      ),
      findsWidgets,
    );
    final problem = _report(retained).problems.single;
    await tester.tap(
      find.byKey(Key('revision3-project-problem-${problem.id}')),
    );
    await tester.pump();
    final action = find.byKey(
      Key(
        'revision3-project-problems-action-dataAssetStage-${stage.targetPath}',
      ),
    );
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pump();
    expect(openedTarget, stage.targetPath);
  });

  testWidgets('filters by category and searches localized plus exact terms', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1280, 720));
    final fixture = revision3ProjectProblemsFilterFixture();
    final report = _report(fixture);
    await _pumpView(tester, fixture: fixture);
    await tester.pumpAndSettle();

    expect(_problemTiles(), findsNWidgets(report.problems.length));

    await tester.tap(
      find.byKey(const Key('revision3-project-problems-filter-dataAssets')),
    );
    await tester.pump();
    expect(_problemTiles(), findsOneWidget);
    expect(
      find.text(
        _problemTitle(
          report.problems.singleWhere(
            (item) =>
                item.category == Revision3ProjectProblemCategory.dataAssets,
          ),
        ),
      ),
      findsWidgets,
    );

    await tester.tap(
      find.byKey(const Key('revision3-project-problems-filter-all')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-problems-search')),
      'Gate Guard',
    );
    await tester.pump();
    expect(_problemTiles(), findsOneWidget);
    expect(
      find.text(
        _problemTitle(
          report.problems.singleWhere(
            (item) =>
                item.code == Revision3ProjectProblemCode.missingEntityReference,
          ),
        ),
      ),
      findsWidgets,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-project-problems-search')),
      'no such problem',
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-project-problems-filtered-empty')),
      findsWidgets,
    );
  });

  testWidgets('empty statement never claims build or runtime readiness', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 650));
    final fixture = revision3ProjectProblemsEmptyFixture();
    await _pumpView(tester, fixture: fixture);
    await tester.pumpAndSettle();

    expect(find.text(_copy.emptyTitle), findsOneWidget);
    expect(find.text(_copy.emptyBoundaryDescription), findsOneWidget);
    await tester.drag(
      find.byKey(const Key('revision3-project-problems-assessments')),
      const Offset(-700, 0),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(
        const Key('revision3-project-problems-assessment-managedBuild'),
      ),
      findsOneWidget,
    );
    expect(find.text('Blocked'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-project-problems-assessment-runtime')),
      findsOneWidget,
    );
    expect(find.text('Unqualified'), findsOneWidget);
    expect(find.text('Ready'), findsNothing);
  });

  testWidgets('uses wide split and short narrow scrollable detail', (
    tester,
  ) async {
    final fixture = revision3ProjectProblemsFilterFixture();
    final firstProblem = _report(fixture).problems.first;
    await _setSurface(tester, const Size(1280, 720));
    await _pumpView(tester, fixture: fixture);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-problems-split')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-problems-detail')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);

    await tester.binding.setSurfaceSize(const Size(640, 420));
    await tester.pumpWidget(_host(fixture: fixture));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-project-problems-split')),
      findsNothing,
    );
    await tester.tap(
      find.byKey(Key('revision3-project-problem-${firstProblem.id}')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-project-problems-detail-scroll')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-problems-close-detail')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'ignores stale content and stage completions after revision change',
    (tester) async {
      await _setSurface(tester, const Size(900, 650));
      final stale = revision3ProjectProblemsFilterFixture(projectRevision: 7);
      final current = revision3ProjectProblemsEmptyFixture(projectRevision: 8);
      final staleContent = Completer<Revision3ContentIndex>();
      final staleStages = Completer<List<AuthoringRevision3DataAssetStage>>();
      final currentContent = Completer<Revision3ContentIndex>();
      final currentStages = Completer<List<AuthoringRevision3DataAssetStage>>();
      var contentCalls = 0;
      var stageCalls = 0;
      var revision = 7;
      late StateSetter rebuild;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3ProjectProblemsView(
                  projectRoot: 'managed-root',
                  projectId: current.projectId,
                  projectRevision: revision,
                  projectHeadCanonicalJson: 'head-$revision',
                  loadContent: () => contentCalls++ == 0
                      ? staleContent.future
                      : currentContent.future,
                  loadDataAssetStages: () => stageCalls++ == 0
                      ? staleStages.future
                      : currentStages.future,
                  gameConfigured: true,
                  copy: _copy,
                );
              },
            ),
          ),
        ),
      );
      await tester.pump();
      rebuild(() => revision = 8);
      await tester.pump();

      currentContent.complete(current.contentIndex);
      currentStages.complete(current.dataAssetStages);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-problems-empty')),
        findsOneWidget,
      );

      staleContent.complete(stale.contentIndex);
      staleStages.complete(stale.dataAssetStages);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-problems-empty')),
        findsOneWidget,
      );
      expect(_problemTiles(), findsNothing);
      expect(contentCalls, 2);
      expect(stageCalls, 2);
    },
  );

  testWidgets('passes exact entity, DataAsset stage, and asset identifiers', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1280, 720));
    final fixture = revision3ProjectProblemsFilterFixture();
    final report = _report(fixture);
    String? entityId;
    String? assetSha;
    String? stagePath;
    await _pumpView(
      tester,
      fixture: fixture,
      actions: Revision3ProjectProblemsActions(
        openEntity: (value) => entityId = value,
        openAsset: (value) => assetSha = value,
        openDataAssetStage: (value) => stagePath = value,
      ),
    );
    await tester.pumpAndSettle();

    final entityAction = find.byKey(
      Key(
        'revision3-project-problems-action-entity-'
        '$revision3ProjectProblemsNpcId',
      ),
    );
    await tester.ensureVisible(entityAction);
    await tester.pump();
    await tester.tap(entityAction);
    await tester.pump();
    expect(entityId, revision3ProjectProblemsNpcId);

    final stageProblem = report.problems.singleWhere(
      (item) =>
          item.code == Revision3ProjectProblemCode.dataAssetStageOfflineOnly,
    );
    await tester.tap(
      find.byKey(Key('revision3-project-problem-${stageProblem.id}')),
    );
    await tester.pump();
    final stageAction = find.byKey(
      Key(
        'revision3-project-problems-action-dataAssetStage-'
        '${fixture.dataAssetStage!.targetPath}',
      ),
    );
    await tester.ensureVisible(stageAction);
    await tester.tap(stageAction);
    await tester.pump();
    expect(stagePath, fixture.dataAssetStage!.targetPath);

    final assetAction = find.byKey(
      Key(
        'revision3-project-problems-action-asset-'
        '${fixture.dataAssetManifestSha256}',
      ),
    );
    await tester.ensureVisible(assetAction);
    await tester.pump();
    await tester.tap(assetAction);
    await tester.pump();
    expect(assetSha, fixture.dataAssetManifestSha256);
  });

  testWidgets('routes settings and verify callbacks without mutation', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1280, 720));
    final fixture = revision3ProjectProblemsCleanFixture();
    var settingsCalls = 0;
    var verifyCalls = 0;
    await _pumpView(
      tester,
      fixture: fixture,
      gameConfigured: false,
      actions: Revision3ProjectProblemsActions(
        openSettings: () => settingsCalls++,
        verifyCurrentProject: () => verifyCalls++,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(
        const Key(
          'revision3-project-problems-assessment-action-gameConfiguration',
        ),
      ),
    );
    await tester.tap(
      find.byKey(
        const Key('revision3-project-problems-verify-current-project'),
      ),
    );
    await tester.pump();
    expect(settingsCalls, 1);
    expect(verifyCalls, 1);
  });

  testWidgets('mobile action closes only after successful navigation', (
    tester,
  ) async {
    await _setSurface(tester, const Size(640, 420));
    final fixture = revision3ProjectProblemsFilterFixture();
    final first = _report(fixture).problems.first;
    var fail = true;
    var calls = 0;
    await _pumpView(
      tester,
      fixture: fixture,
      actions: Revision3ProjectProblemsActions(
        openEntity: (_) {
          calls++;
          if (fail) throw StateError('navigation rejected');
        },
      ),
    );
    await tester.pumpAndSettle();

    Future<void> openDetailAndTap() async {
      await tester.tap(
        find.byKey(Key('revision3-project-problem-${first.id}')),
      );
      await tester.pumpAndSettle();
      final action = find.byKey(
        Key(
          'revision3-project-problems-action-entity-'
          '$revision3ProjectProblemsNpcId',
        ),
      );
      await tester.ensureVisible(action);
      await tester.pump();
      await tester.tap(action);
      await tester.pumpAndSettle();
    }

    await openDetailAndTap();
    expect(calls, 1);
    expect(
      find.byKey(const Key('revision3-project-problems-detail')),
      findsOneWidget,
    );
    expect(find.text(_copy.actionFailedMessage), findsOneWidget);

    fail = false;
    final retryAction = find.byKey(
      Key(
        'revision3-project-problems-action-entity-'
        '$revision3ProjectProblemsNpcId',
      ),
    );
    await tester.ensureVisible(retryAction);
    await tester.pump();
    await tester.tap(retryAction);
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(
      find.byKey(const Key('revision3-project-problems-detail')),
      findsNothing,
    );
  });

  testWidgets('exposes loading, heading, scope, and list semantics', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await _setSurface(tester, const Size(1280, 720));
    final fixture = revision3ProjectProblemsFilterFixture();
    final content = Completer<Revision3ContentIndex>();
    final stages = Completer<List<AuthoringRevision3DataAssetStage>>();
    await _pumpView(
      tester,
      fixture: fixture,
      loadContent: () => content.future,
      loadStages: () => stages.future,
    );
    await tester.pump();
    expect(find.bySemanticsLabel(_copy.loadingSemanticsLabel), findsOneWidget);

    content.complete(fixture.contentIndex);
    stages.complete(fixture.dataAssetStages);
    await tester.pumpAndSettle();
    expect(
      tester.getSemantics(find.text(_copy.title)),
      matchesSemantics(label: _copy.title, isHeader: true),
    );
    final assessment = find.byKey(
      const Key('revision3-project-problems-assessment-referenceIntegrity'),
    );
    expect(tester.getSemantics(assessment).label, 'Scope referenceIntegrity');
    expect(tester.getSemantics(assessment).value, 'Issues');
    expect(
      find.bySemanticsLabel(_problemTitle(_report(fixture).problems.first)),
      findsWidgets,
    );
    semantics.dispose();
  });

  testWidgets('same-revision head drift invalidates a delayed report action', (
    tester,
  ) async {
    await _setSurface(tester, const Size(640, 420));
    final fixture = revision3ProjectProblemsFilterFixture();
    final pendingAction = Completer<void>();
    var actionCalls = 0;
    var contentCalls = 0;
    var stageCalls = 0;
    var head = '{"checkpoint":"a"}';
    late StateSetter rebuild;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Revision3ProjectProblemsView(
                projectRoot: r'C:\mods\problems-a',
                projectId: fixture.projectId,
                projectRevision: fixture.projectRevision,
                projectHeadCanonicalJson: head,
                loadContent: () async {
                  contentCalls++;
                  return fixture.contentIndex;
                },
                loadDataAssetStages: () async {
                  stageCalls++;
                  return fixture.dataAssetStages;
                },
                gameConfigured: true,
                copy: _copy,
                actions: Revision3ProjectProblemsActions(
                  openEntity: (_) {
                    actionCalls++;
                    return pendingAction.future;
                  },
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final first = _report(fixture).problems.first;
    await tester.tap(find.byKey(Key('revision3-project-problem-${first.id}')));
    await tester.pumpAndSettle();
    final staleAction = find.byKey(
      const Key(
        'revision3-project-problems-action-entity-'
        '$revision3ProjectProblemsNpcId',
      ),
    );
    await tester.ensureVisible(staleAction);
    await tester.tap(staleAction);
    await tester.pump();
    expect(actionCalls, 1);

    rebuild(() => head = '{"checkpoint":"b"}');
    await tester.pump();
    await tester.pump();
    expect(contentCalls, 2);
    expect(stageCalls, 2);

    pendingAction.complete();
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-problems-detail')),
      findsOneWidget,
      reason: 'a stale action must not close its retained detail sheet',
    );
    expect(find.text(_copy.actionFailedMessage), findsOneWidget);
    expect(actionCalls, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'root collision invalidates an old report callback at the same id and head',
    (tester) async {
      await _setSurface(tester, const Size(640, 420));
      final fixture = revision3ProjectProblemsFilterFixture();
      var root = r'C:\mods\problems-a';
      const head = '{"checkpoint":"same"}';
      var actionCalls = 0;
      var contentCalls = 0;
      var stageCalls = 0;
      late StateSetter rebuild;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3ProjectProblemsView(
                  projectRoot: root,
                  projectId: fixture.projectId,
                  projectRevision: fixture.projectRevision,
                  projectHeadCanonicalJson: head,
                  loadContent: () async {
                    contentCalls++;
                    return fixture.contentIndex;
                  },
                  loadDataAssetStages: () async {
                    stageCalls++;
                    return fixture.dataAssetStages;
                  },
                  gameConfigured: true,
                  copy: _copy,
                  actions: Revision3ProjectProblemsActions(
                    openEntity: (_) => actionCalls++,
                  ),
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final first = _report(fixture).problems.first;
      await tester.tap(
        find.byKey(Key('revision3-project-problem-${first.id}')),
      );
      await tester.pumpAndSettle();
      final staleAction = find.byKey(
        const Key(
          'revision3-project-problems-action-entity-'
          '$revision3ProjectProblemsNpcId',
        ),
      );
      await tester.ensureVisible(staleAction);

      rebuild(() => root = r'C:\mods\problems-copy');
      await tester.pumpAndSettle();
      expect(contentCalls, 2);
      expect(stageCalls, 2);

      await tester.tap(staleAction);
      await tester.pumpAndSettle();

      expect(actionCalls, 0);
      expect(
        find.byKey(const Key('revision3-project-problems-detail')),
        findsOneWidget,
      );
      expect(find.text(_copy.actionFailedMessage), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}

Future<void> _pumpView(
  WidgetTester tester, {
  required Revision3ProjectProblemsFixture fixture,
  Revision3ProblemsViewContentLoader? loadContent,
  Revision3ProblemsViewDataAssetStageLoader? loadStages,
  bool gameConfigured = true,
  Revision3ProjectProblemsActions actions =
      const Revision3ProjectProblemsActions(),
}) => tester.pumpWidget(
  _host(
    fixture: fixture,
    loadContent: loadContent,
    loadStages: loadStages,
    gameConfigured: gameConfigured,
    actions: actions,
  ),
);

Widget _host({
  required Revision3ProjectProblemsFixture fixture,
  Revision3ProblemsViewContentLoader? loadContent,
  Revision3ProblemsViewDataAssetStageLoader? loadStages,
  bool gameConfigured = true,
  Revision3ProjectProblemsActions actions =
      const Revision3ProjectProblemsActions(),
}) => MaterialApp(
  home: Scaffold(
    body: Revision3ProjectProblemsView(
      projectRoot: 'managed-root',
      projectId: fixture.projectId,
      projectRevision: fixture.projectRevision,
      projectHeadCanonicalJson: 'head-${fixture.projectRevision}',
      loadContent: loadContent ?? () async => fixture.contentIndex,
      loadDataAssetStages: loadStages ?? () async => fixture.dataAssetStages,
      gameConfigured: gameConfigured,
      copy: _copy,
      actions: actions,
    ),
  ),
);

Finder _problemTiles() => find.descendant(
  of: find.byKey(const Key('revision3-project-problems-list')),
  matching: find.byType(ListTile),
);

Revision3ProjectProblemReport _report(
  Revision3ProjectProblemsFixture fixture, {
  bool gameConfigured = true,
}) => Revision3ProjectProblemBuilder.build(
  fixture.contentIndex,
  dataAssetStages: fixture.dataAssetStages,
  gameConfigured: gameConfigured,
);

Future<void> _setSurface(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

String _problemTitle(Revision3ProjectProblem problem) =>
    'Problem ${problem.code.name}';

String _problemDescription(Revision3ProjectProblem problem) =>
    switch (problem.details) {
      Revision3EntityReferenceProblemDetails details =>
        '${details.sourceDisplayName} references ${details.targetEntityId}',
      Revision3AssetReferenceProblemDetails details =>
        '${details.sourceDisplayName} references '
            '${details.logicalName ?? details.sha256}',
      Revision3DataAssetStageProblemDetails details =>
        'Offline-only stage ${details.targetPath}',
      Revision3CapabilityBoundaryProblemDetails details =>
        'Boundary ${details.scope.name}',
    };

String _categoryName(Revision3ProjectProblemCategory category) =>
    'Category ${category.name}';

String _severityName(Revision3ProjectProblemSeverity severity) =>
    'Severity ${severity.name}';

String _scopeTitle(Revision3ProjectProblemScope scope) => 'Scope ${scope.name}';

String _scopeDescription(Revision3ProjectProblemScope scope) =>
    'Scoped evidence for ${scope.name} only.';

String _readinessName(Revision3ProjectProblemReadiness readiness) =>
    switch (readiness) {
      Revision3ProjectProblemReadiness.clear => 'Scope clear',
      Revision3ProjectProblemReadiness.issues => 'Issues',
      Revision3ProjectProblemReadiness.unavailable => 'Unavailable',
      Revision3ProjectProblemReadiness.notEvaluated => 'Not evaluated',
      Revision3ProjectProblemReadiness.blocked => 'Blocked',
      Revision3ProjectProblemReadiness.unqualified => 'Unqualified',
    };

String _evidenceName(Revision3ProjectProblemEvidence evidence) =>
    'Evidence ${evidence.name}';
