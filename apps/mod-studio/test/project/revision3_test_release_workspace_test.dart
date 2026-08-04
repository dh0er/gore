import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_test_release_workspace.dart';

const _projectId = '11111111111111111111111111111111';
const _projectRevision = 7;
const _checkpointIdentity = '{"revision":7,"head":"current"}';

void main() {
  testWidgets(
    'shows compact checks and calm future steps while callbacks authorize nothing',
    (tester) async {
      final semantics = tester.ensureSemantics();
      var buildCalls = 0;
      var deploymentCalls = 0;
      await _pumpWorkspace(
        tester,
        workspace: _workspace(
          playableBuild: _capability(
            title: 'Spielbare Mod erstellen',
            evidence: Revision3TestReleaseEvidence(
              projectId: _projectId,
              projectRevision: _projectRevision,
              checkpointIdentity: '{"revision":7,"head":"stale-build"}',
              scope: Revision3TestReleaseEvidenceScope.playableBuild,
              summary: 'Veralteter Build-Nachweis.',
            ),
            onPressed: () => buildCalls += 1,
          ),
          deployment: _capability(
            title: 'Im Spiel installieren',
            onPressed: () => deploymentCalls += 1,
          ),
        ),
      );

      expect(find.text('Testen & Veröffentlichen'), findsOneWidget);
      for (final id in <String>[
        'project-structure',
        'scripts',
        'voice',
        'data-assets',
        'playable-build',
        'deployment',
      ]) {
        expect(
          find.byKey(Key('revision3-test-release-$id-card')),
          findsOneWidget,
        );
      }
      expect(find.text('Nicht geprüft'), findsNWidgets(4));
      expect(find.text('Nicht verfügbar'), findsNWidgets(2));
      expect(find.text('Blockiert'), findsNothing);
      expect(find.text('Verfügbar'), findsNothing);
      expect(find.textContaining('Bereit'), findsNothing);
      expect(find.textContaining('Veralteter Build-Nachweis'), findsNothing);
      expect(
        find.byKey(const Key('revision3-test-release-step-1')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-test-release-step-2')),
        findsOneWidget,
      );
      final capabilitySurface = tester.widget<Material>(
        find.byKey(const Key('revision3-test-release-capabilities')),
      );
      final scheme = Theme.of(
        tester.element(
          find.byKey(const Key('revision3-test-release-capabilities')),
        ),
      ).colorScheme;
      expect(capabilitySurface.color, scheme.surfaceContainerLow);
      expect(capabilitySurface.color, isNot(scheme.errorContainer));

      final buildAction = find.byKey(
        const Key('revision3-test-release-playable-build-action'),
      );
      final deploymentAction = find.byKey(
        const Key('revision3-test-release-deployment-action'),
      );
      expect(tester.widget<FilledButton>(buildAction).onPressed, isNull);
      expect(tester.widget<FilledButton>(deploymentAction).onPressed, isNull);
      expect(buildCalls, 0);
      expect(deploymentCalls, 0);

      final buildSemantics = tester.getSemantics(
        find.byKey(const Key('revision3-test-release-playable-build-card')),
      );
      expect(buildSemantics.label, 'Spielbare Mod erstellen');
      expect(buildSemantics.value, 'Nicht verfügbar');
      expect(
        buildSemantics.hint,
        'Dieses Ergebnis gehört zu einer anderen Projektversion. Bitte prüfe den Bereich erneut.',
      );
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets(
    'requires exact head evidence and keeps evidence without action unavailable',
    (tester) async {
      var buildCalls = 0;
      await _pumpWorkspace(
        tester,
        workspace: _workspace(
          projectStructure: _check(
            state: Revision3TestReleaseCheckState.passed,
            title: 'Projektstruktur',
            evidence: _evidence(
              'Projektstruktur für aktuellen Stand geprüft.',
              scope: Revision3TestReleaseEvidenceScope.projectStructure,
            ),
          ),
          scripts: _check(
            state: Revision3TestReleaseCheckState.passed,
            title: 'Skripte',
            evidence: Revision3TestReleaseEvidence(
              projectId: _projectId,
              projectRevision: _projectRevision,
              checkpointIdentity: '{"revision":7,"head":"stale"}',
              scope: Revision3TestReleaseEvidenceScope.scripts,
              summary: 'Veralteter Compiler-Nachweis.',
            ),
          ),
          playableBuild: _capability(
            title: 'Spielbare Mod erstellen',
            evidence: _evidence(
              'Build-Plan für aktuellen Stand geprüft.',
              scope: Revision3TestReleaseEvidenceScope.playableBuild,
            ),
            onPressed: () => buildCalls += 1,
          ),
          deployment: _capability(
            title: 'Im Spiel installieren',
            evidence: _evidence(
              'Installationsplan für aktuellen Stand geprüft.',
              scope: Revision3TestReleaseEvidenceScope.deployment,
            ),
          ),
        ),
      );

      expect(find.text('Geprüft'), findsOneWidget);
      expect(
        find.text(
          'Dieses Ergebnis gehört zu einer anderen Projektversion. Bitte prüfe den Bereich erneut.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('Veralteter Compiler-Nachweis'), findsNothing);
      expect(find.text('Verfügbar'), findsOneWidget);
      expect(find.text('Nicht verfügbar'), findsOneWidget);

      final buildAction = find.byKey(
        const Key('revision3-test-release-playable-build-action'),
      );
      final deploymentAction = find.byKey(
        const Key('revision3-test-release-deployment-action'),
      );
      expect(tester.widget<FilledButton>(buildAction).onPressed, isNotNull);
      expect(tester.widget<FilledButton>(deploymentAction).onPressed, isNull);
      await tester.ensureVisible(buildAction);
      await tester.pump();
      await tester.tap(buildAction);
      expect(buildCalls, 1);
      expect(find.textContaining('Bereit'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('evidence scope cannot cross-authorize checks or capabilities', (
    tester,
  ) async {
    var buildCalls = 0;
    var deploymentCalls = 0;
    await _pumpWorkspace(
      tester,
      workspace: _workspace(
        projectStructure: _check(
          state: Revision3TestReleaseCheckState.passed,
          title: 'Projektstruktur',
          evidence: _evidence(
            'Falscher Skript-Nachweis.',
            scope: Revision3TestReleaseEvidenceScope.scripts,
          ),
        ),
        scripts: _check(
          state: Revision3TestReleaseCheckState.passed,
          title: 'Skripte',
          evidence: _evidence(
            'Falscher Struktur-Nachweis.',
            scope: Revision3TestReleaseEvidenceScope.projectStructure,
          ),
        ),
        playableBuild: _capability(
          title: 'Spielbare Mod erstellen',
          evidence: _evidence(
            'Deployment-Nachweis darf keinen Build autorisieren.',
            scope: Revision3TestReleaseEvidenceScope.deployment,
          ),
          onPressed: () => buildCalls += 1,
        ),
        deployment: _capability(
          title: 'Im Spiel installieren',
          evidence: _evidence(
            'Build-Nachweis darf kein Deployment autorisieren.',
            scope: Revision3TestReleaseEvidenceScope.playableBuild,
          ),
          onPressed: () => deploymentCalls += 1,
        ),
      ),
    );

    expect(find.text('Nicht geprüft'), findsNWidgets(4));
    expect(find.text('Nicht verfügbar'), findsNWidgets(2));
    expect(find.text('Blockiert'), findsNothing);
    expect(
      find.text(
        'Dieses Ergebnis gehört zu einer anderen Projektversion. Bitte prüfe den Bereich erneut.',
      ),
      findsNothing,
    );
    expect(
      find.text('Kein passender Build-Nachweis vorhanden.'),
      findsOneWidget,
    );
    expect(
      find.text('Kein passender Installations-Nachweis vorhanden.'),
      findsOneWidget,
    );
    expect(find.textContaining('Falscher Skript-Nachweis'), findsNothing);
    expect(find.textContaining('Falscher Struktur-Nachweis'), findsNothing);
    expect(find.textContaining('darf keinen Build'), findsNothing);
    expect(find.textContaining('darf kein Deployment'), findsNothing);

    expect(
      tester
          .widget<FilledButton>(
            find.byKey(
              const Key('revision3-test-release-playable-build-action'),
            ),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-test-release-deployment-action')),
          )
          .onPressed,
      isNull,
    );
    expect(buildCalls, 0);
    expect(deploymentCalls, 0);

    await _pumpWorkspace(
      tester,
      workspace: _workspace(
        playableBuild: _capability(
          title: 'Spielbare Mod erstellen',
          evidence: _evidence(
            'Check-Nachweis darf keine Capability autorisieren.',
            scope: Revision3TestReleaseEvidenceScope.projectStructure,
          ),
          onPressed: () => buildCalls += 1,
        ),
      ),
    );

    expect(
      tester
          .widget<FilledButton>(
            find.byKey(
              const Key('revision3-test-release-playable-build-action'),
            ),
          )
          .onPressed,
      isNull,
    );
    expect(
      find.textContaining('Check-Nachweis darf keine Capability'),
      findsNothing,
    );
    expect(buildCalls, 0);
    expect(tester.takeException(), isNull);
  });

  testWidgets('compact high-text-scale layout scrolls through both slots', (
    tester,
  ) async {
    await _setSurface(tester, const Size(300, 260));
    await _pumpWorkspace(
      tester,
      textScaler: const TextScaler.linear(2),
      workspace: _workspace(
        focus: Revision3TestReleaseFocus.release,
        problemsBuilder: (_) => const Card(
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Text('Problems fixture'),
          ),
        ),
        voiceContinuationBuilder: (_) => const Card(
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Text('Voice continuation fixture'),
          ),
        ),
      ),
    );

    expect(find.byType(SingleChildScrollView), findsOneWidget);
    final releaseHeading = find.byKey(
      const Key('revision3-test-release-release-heading'),
    );
    expect(tester.getTopLeft(releaseHeading).dy, inInclusiveRange(0, 260));
    expect(
      find.byKey(const Key('revision3-test-release-problems-slot')),
      findsOneWidget,
    );
    final voiceSlot = find.byKey(
      const Key('revision3-test-release-voice-continuation-slot'),
    );
    expect(voiceSlot, findsOneWidget);
    await tester.ensureVisible(voiceSlot);
    await tester.pump();
    expect(find.text('Voice continuation fixture'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('wide high-text-scale layout still stacks long actions', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 500));
    await _pumpWorkspace(
      tester,
      textScaler: const TextScaler.linear(2),
      workspace: _workspace(),
    );

    final action = find.byKey(
      const Key('revision3-test-release-playable-build-action'),
    );
    await tester.ensureVisible(action);
    await tester.pump();
    expect(tester.getTopLeft(action).dx, lessThan(200));
    expect(tester.takeException(), isNull);
  });

  testWidgets('build preview is a separate slot and authorizes no capability', (
    tester,
  ) async {
    await _setSurface(tester, const Size(320, 300));
    await _pumpWorkspace(
      tester,
      textScaler: const TextScaler.linear(2),
      workspace: _workspace(
        focus: Revision3TestReleaseFocus.buildPreview,
        buildPreviewBuilder: (_) => const Card(
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Text('Exact project build preview fixture'),
          ),
        ),
      ),
    );

    final slot = find.byKey(
      const Key('revision3-test-release-build-preview-slot'),
    );
    expect(slot, findsOneWidget);
    expect(tester.getTopLeft(slot).dy, inInclusiveRange(0, 300));
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(
              const Key('revision3-test-release-playable-build-action'),
            ),
          )
          .onPressed,
      isNull,
    );
    expect(tester.takeException(), isNull);
  });

  test('evaluated checks require explicit evidence', () {
    expect(
      () => Revision3TestReleaseCheck(
        state: Revision3TestReleaseCheckState.passed,
        title: 'Project structure',
        description: 'Exact structure check.',
      ),
      throwsArgumentError,
    );
    expect(
      () => Revision3TestReleaseCheck(
        state: Revision3TestReleaseCheckState.needsAttention,
        title: 'Voice',
        description: 'Voice blockers.',
      ),
      throwsArgumentError,
    );
  });
}

Revision3TestReleaseWorkspace _workspace({
  Revision3TestReleaseCheck? projectStructure,
  Revision3TestReleaseCheck? scripts,
  Revision3TestReleaseCheck? voice,
  Revision3TestReleaseCheck? dataAssets,
  Revision3TestReleaseCapability? playableBuild,
  Revision3TestReleaseCapability? deployment,
  WidgetBuilder? buildPreviewBuilder,
  WidgetBuilder? problemsBuilder,
  WidgetBuilder? voiceContinuationBuilder,
  Revision3TestReleaseFocus focus = Revision3TestReleaseFocus.overview,
}) => Revision3TestReleaseWorkspace(
  projectId: _projectId,
  projectRevision: _projectRevision,
  checkpointIdentity: _checkpointIdentity,
  projectStructure:
      projectStructure ??
      _check(
        state: Revision3TestReleaseCheckState.notEvaluated,
        title: 'Projektstruktur',
      ),
  scripts:
      scripts ??
      _check(
        state: Revision3TestReleaseCheckState.notEvaluated,
        title: 'Skripte',
      ),
  voice:
      voice ??
      _check(
        state: Revision3TestReleaseCheckState.notEvaluated,
        title: 'Sprachausgabe',
      ),
  dataAssets:
      dataAssets ??
      _check(
        state: Revision3TestReleaseCheckState.notEvaluated,
        title: 'DataAssets',
      ),
  playableBuild: playableBuild ?? _capability(title: 'Spielbare Mod erstellen'),
  deployment: deployment ?? _capability(title: 'Im Spiel installieren'),
  copy: const Revision3TestReleaseCopy.german(),
  focus: focus,
  buildPreviewBuilder: buildPreviewBuilder,
  problemsBuilder: problemsBuilder,
  voiceContinuationBuilder: voiceContinuationBuilder,
);

Revision3TestReleaseCheck _check({
  required Revision3TestReleaseCheckState state,
  required String title,
  Revision3TestReleaseEvidence? evidence,
}) => Revision3TestReleaseCheck(
  state: state,
  title: title,
  description: 'Prüfung für $title.',
  evidence: evidence,
  actionLabel: 'Öffnen',
);

Revision3TestReleaseCapability _capability({
  required String title,
  Revision3TestReleaseEvidence? evidence,
  VoidCallback? onPressed,
}) => Revision3TestReleaseCapability(
  title: title,
  description: '$title ist für diesen Projektstand freigegeben.',
  blockedReason: title.startsWith('Spielbare')
      ? 'Kein passender Build-Nachweis vorhanden.'
      : 'Kein passender Installations-Nachweis vorhanden.',
  actionLabel: title,
  evidence: evidence,
  onPressed: onPressed,
);

Revision3TestReleaseEvidence _evidence(
  String summary, {
  required Revision3TestReleaseEvidenceScope scope,
}) => Revision3TestReleaseEvidence(
  projectId: _projectId,
  projectRevision: _projectRevision,
  checkpointIdentity: _checkpointIdentity,
  scope: scope,
  summary: summary,
);

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  required Revision3TestReleaseWorkspace workspace,
  TextScaler textScaler = TextScaler.noScaling,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: MediaQuery(
        data: MediaQueryData(textScaler: textScaler),
        child: Scaffold(body: workspace),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _setSurface(WidgetTester tester, Size size) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
}
