import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_dataasset_stage_panel.dart';

import '../support/revision3_dataasset_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

const _projectRoot = r'C:\projects\managed-r3';

void main() {
  late Revision3DataAssetFixture fixture;
  late AuthoringRevision3DataAssetStage stage;

  setUp(() {
    fixture = revision3DataAssetNativeGoldenFixture();
    stage = AuthoringRevision3DataAssetStageListResult.fromJson(
      fixture.listResponse(),
      expectedHead: fixture.stagedHead,
    ).stages.single;
  });

  testWidgets('empty state states the honest project-only boundary', (
    tester,
  ) async {
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () async => const [],
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      ),
    );

    expect(
      find.byKey(const Key('revision3-dataasset-stage-empty')),
      findsOneWidget,
    );
    expect(find.textContaining('new mod-file folder'), findsOneWidget);
    expect(find.text('Build / Deploy'), findsNothing);
    expect(find.text('Test in game'), findsNothing);
  });

  testWidgets('empty state remains usable with little vertical space', (
    tester,
  ) async {
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () async => const [],
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      ),
      surfaceSize: const Size(900, 420),
    );

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-dataasset-stage-empty-scroll')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-dataasset-stage-empty')),
      findsOneWidget,
    );
  });

  testWidgets(
    'compact high-text layout keeps header and empty actions scrollable',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(700, 460));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: const TextScaler.linear(2)),
            child: child!,
          ),
          home: Scaffold(
            body: Revision3DataAssetStagePanel(
              projectRoot: _projectRoot,
              projectId: stage.projectId,
              projectRevision: 5,
              projectHead: fixture.stagedHead,
              load: () async => const [],
              publish: _unexpectedPublish,
              remove: _unexpectedRemove,
              browseInstalledPackages: () async => null,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-header-scroll')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<Scrollbar>(
              find.byKey(
                const Key('revision3-dataasset-stage-header-scrollbar'),
              ),
            )
            .thumbVisibility,
        isTrue,
      );
      expect(
        find.byKey(const Key('revision3-dataasset-stage-search')).hitTestable(),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dataasset-stage-empty-scroll')),
        findsOneWidget,
      );
      final emptyAction = find.byKey(
        const Key('revision3-dataasset-empty-browse-installed'),
      );
      await tester.ensureVisible(emptyAction);
      await tester.pump();
      expect(tester.widget<FilledButton>(emptyAction).onPressed, isNotNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'reviewed quick start foregrounds installed presets and collapses expert tools',
    (tester) async {
      var browseCalls = 0;
      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () async => const [],
          publish: _unexpectedPublish,
          remove: _unexpectedRemove,
          browseInstalledPackages: () async {
            browseCalls++;
            return null;
          },
        ),
      );

      expect(
        find.byKey(const Key('revision3-dataasset-reviewed-quick-start')),
        findsOneWidget,
      );
      for (final target in footstepPresetReviewedSchema.targets) {
        expect(find.text(target.friendlyName), findsOneWidget);
      }
      final browse = find.byKey(
        const Key('revision3-dataasset-browse-installed'),
      );
      expect(tester.widget<FilledButton>(browse).onPressed, isNotNull);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-add')),
        findsNothing,
      );
      expect(find.textContaining('PatchReceipt'), findsNothing);
      expect(find.textContaining('ExtractReceipt'), findsNothing);
      expect(find.textContaining('safe X/Y'), findsNothing);
      expect(
        find.textContaining('reviewed, bounded X/Y texture-size project edit'),
        findsOneWidget,
      );

      await tester.tap(browse);
      await tester.pumpAndSettle();
      expect(browseCalls, 1);

      await _expandExpertTools(tester);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-add')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dataasset-stage-search')).hitTestable(),
        findsOneWidget,
      );
    },
  );

  testWidgets('empty-state primary action owns one installed-browser request', (
    tester,
  ) async {
    final result = Completer<DataAssetSemanticStagePublication?>();
    var browseCalls = 0;
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () async => const [],
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
        browseInstalledPackages: () {
          browseCalls++;
          return result.future;
        },
      ),
    );

    final emptyBrowse = find.byKey(
      const Key('revision3-dataasset-empty-browse-installed'),
    );
    await tester.tap(emptyBrowse);
    await tester.pump();

    expect(browseCalls, 1);
    expect(tester.widget<FilledButton>(emptyBrowse).onPressed, isNull);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-dataasset-browse-installed')),
          )
          .onPressed,
      isNull,
    );
    await tester.tap(emptyBrowse);
    expect(browseCalls, 1);

    result.complete(null);
    await tester.pumpAndSettle();
    expect(tester.widget<FilledButton>(emptyBrowse).onPressed, isNotNull);
  });

  testWidgets(
    'installed result from an opening head is ignored after same-revision head drift',
    (tester) async {
      final result = Completer<DataAssetSemanticStagePublication?>();
      var projectHead = fixture.stagedHead;
      late StateSetter rebuild;
      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('same-revision-divergent-head-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: 5,
                  projectHead: projectHead,
                  load: () async => const [],
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  browseInstalledPackages: () => result.future,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-dataasset-browse-installed')),
      );
      await tester.pump();
      rebuild(() => projectHead = fixture.removedHead);
      await tester.pump();
      await tester.pump();

      result.complete(
        DataAssetSemanticStagePublication(
          targetPath: stage.targetPath,
          revision: 6,
        ),
      );
      await tester.pumpAndSettle();

      final search = tester.widget<TextField>(
        find.byKey(const Key('revision3-dataasset-stage-search')),
      );
      expect(search.controller?.text, isEmpty);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-reveal-notice')),
        findsNothing,
      );
      expect(find.textContaining('saved and opened below'), findsNothing);
    },
  );

  testWidgets(
    'installed publication reloads the advanced checkpoint and expands its exact stage',
    (tester) async {
      final semantics = tester.ensureSemantics();
      var projectRevision = 4;
      var projectHead = fixture.basisHead;
      var stages = const <AuthoringRevision3DataAssetStage>[];
      late StateSetter rebuild;
      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('published-focus-stage-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: projectRevision,
                  projectHead: projectHead,
                  load: () async => stages,
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  buildReviewedStage:
                      ({
                        required targetPath,
                        required packName,
                        required output,
                      }) async => throw StateError('build must not run'),
                  pickBuildParentDirectory: () async => null,
                  browseInstalledPackages: () async {
                    rebuild(() {
                      projectRevision = 5;
                      projectHead = fixture.stagedHead;
                      stages = [stage];
                    });
                    return DataAssetSemanticStagePublication(
                      targetPath: stage.targetPath,
                      revision: 5,
                    );
                  },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-dataasset-browse-installed')),
      );
      await tester.pumpAndSettle();

      final search = tester.widget<TextField>(
        find.byKey(const Key('revision3-dataasset-stage-search')),
      );
      expect(search.controller?.text, stage.targetPath);
      final stageTile = tester.widget<ExpansionTile>(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      expect(stageTile.controller?.isExpanded, isTrue);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-reveal-notice')),
        findsOneWidget,
      );
      expect(find.textContaining('saved and opened below'), findsOneWidget);
      expect(
        tester.getSemantics(
          find.byKey(const Key('revision3-dataasset-stage-reveal-notice')),
        ),
        matchesSemantics(
          label:
              'TestAsset is saved and opened below. Review it, then use Build files if support for this exact edit is confirmed.',
          isLiveRegion: true,
        ),
      );
      expect(find.text('Replacement width'), findsOneWidget);
      expect(find.text('Gameplay unverified'), findsOneWidget);
      final build = find.byKey(
        ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
      );
      expect(tester.widget<FilledButton>(build).onPressed, isNotNull);
      await tester.ensureVisible(build);
      await tester.pumpAndSettle();
      expect(build.hitTestable(), findsOneWidget);
      expect(find.text('Deploy'), findsNothing);
      expect(find.text('Test in game'), findsNothing);

      await tester.tap(
        find.byKey(const Key('revision3-dataasset-stage-reveal-dismiss')),
      );
      stageTile.controller!.collapse();
      await tester.pumpAndSettle();
      final refresh = find.byKey(
        const Key('revision3-dataasset-stage-refresh'),
      );
      await tester.ensureVisible(refresh);
      await tester.pump();
      await tester.tap(refresh);
      await tester.pumpAndSettle();

      expect(stageTile.controller?.isExpanded, isFalse);
      expect(
        find.byKey(const Key('revision3-dataasset-stage-reveal-notice')),
        findsNothing,
      );
      semantics.dispose();
    },
  );

  testWidgets(
    'publication never focuses an older same-target stage from another revision',
    (tester) async {
      var projectRevision = 5;
      var projectHead = fixture.stagedHead;
      late StateSetter rebuild;
      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('mismatched-focus-stage-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: projectRevision,
                  projectHead: projectHead,
                  load: () async => [stage],
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  browseInstalledPackages: () async {
                    rebuild(() {
                      projectRevision = 6;
                      projectHead = fixture.removedHead;
                    });
                    return DataAssetSemanticStagePublication(
                      targetPath: stage.targetPath,
                      revision: 6,
                    );
                  },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-dataasset-browse-installed')),
      );
      await tester.pumpAndSettle();

      final search = tester.widget<TextField>(
        find.byKey(const Key('revision3-dataasset-stage-search')),
      );
      expect(search.controller?.text, isEmpty);
      final stageTile = tester.widget<ExpansionTile>(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      expect(stageTile.controller?.isExpanded, isFalse);
      expect(
        find.textContaining('not present at its published project revision'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dataasset-stage-reveal-notice')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'stale semantic wizard closes and stays latched until an exact reload',
    (tester) async {
      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () async => const [],
          publish: _unexpectedPublish,
          remove: _unexpectedRemove,
          publishSemanticEdit: (_) async =>
              throw const DataAssetSemanticStageUnavailableException.staleCheckpoint(),
          semanticUassetPicker: () async => r'C:\proof\TestAsset.uasset',
          semanticUsmapPicker: () async => r'C:\proof\Mappings.usmap',
          semanticExtractReceiptPicker: () async =>
              r'C:\proof\extract-receipt.v2.json',
          semanticExtractReceiptInspector: _matchingReceiptInspector,
          semanticInspector:
              ({required uassetPath, required usmapPath, exportIndex}) async =>
                  DataAssetInspection.fromJson(
                    validDataAssetInspectionResponse(),
                  ),
        ),
      );

      await _expandExpertTools(
        tester,
        targetKey: const Key('revision3-dataasset-semantic-create'),
      );
      final create = find.byKey(
        const Key('revision3-dataasset-semantic-create'),
      );
      await tester.tap(create);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-pick-usmap')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-inspect')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('dataasset-semantic-pick-receipt')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('dataasset-semantic-confirm-target')),
      );
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('dataasset-semantic-value')),
        '2',
      );
      final editorScroll = tester.state<ScrollableState>(
        find
            .descendant(
              of: find.byKey(const Key('dataasset-semantic-editor')),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      editorScroll.position.jumpTo(editorScroll.position.maxScrollExtent);
      await tester.pump();
      await tester.ensureVisible(
        find.byKey(const Key('dataasset-semantic-preview')),
      );
      await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
      await tester.pump();
      editorScroll.position.jumpTo(editorScroll.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('dataasset-semantic-stage')), findsOneWidget);
      await tester.tap(find.byKey(const Key('dataasset-semantic-stage')));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('dataasset-semantic-wizard')), findsNothing);
      expect(find.textContaining('project changed'), findsOneWidget);
      expect(tester.widget<OutlinedButton>(create).onPressed, isNull);
      final refresh = find.byKey(
        const Key('revision3-dataasset-stage-refresh'),
      );
      await tester.ensureVisible(refresh);
      await tester.pump();
      await tester.tap(refresh);
      await tester.pumpAndSettle();
      expect(tester.widget<OutlinedButton>(create).onPressed, isNotNull);
    },
  );

  testWidgets(
    'late stale result from an old head cannot relatch the newer checkpoint',
    (tester) async {
      final publication = Completer<DataAssetSemanticStagePublication>();
      var head = fixture.stagedHead;
      late StateSetter rebuild;
      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('same-semantic-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: 5,
                  projectHead: head,
                  load: () async => const [],
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  publishSemanticEdit: (_) => publication.future,
                  semanticUassetPicker: () async =>
                      r'C:\proof\TestAsset.uasset',
                  semanticUsmapPicker: () async => r'C:\proof\Mappings.usmap',
                  semanticExtractReceiptPicker: () async =>
                      r'C:\proof\extract-receipt.v2.json',
                  semanticExtractReceiptInspector: _matchingReceiptInspector,
                  semanticInspector:
                      ({
                        required uassetPath,
                        required usmapPath,
                        exportIndex,
                      }) async => DataAssetInspection.fromJson(
                        validDataAssetInspectionResponse(),
                      ),
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await _startSemanticPublication(tester);
      expect(
        find.byKey(const Key('dataasset-semantic-progress')),
        findsOneWidget,
      );
      rebuild(() => head = fixture.removedHead);
      await tester.pump();
      await tester.pump();

      publication.completeError(
        const DataAssetSemanticStageUnavailableException.staleCheckpoint(),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('dataasset-semantic-wizard')), findsNothing);
      expect(find.textContaining('project changed'), findsNothing);
      final create = find.byKey(
        const Key('revision3-dataasset-semantic-create'),
      );
      expect(tester.widget<OutlinedButton>(create).onPressed, isNotNull);
    },
  );

  testWidgets('renders friendly stage facts without receipt or raw offsets', (
    tester,
  ) async {
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () async => [stage],
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
        buildUnavailableReason:
            'Choose the Gothic 1 Remake installation in Settings before building files.',
      ),
    );

    expect(find.text('TestAsset'), findsOneWidget);
    expect(find.textContaining('/Game/TestAsset'), findsOneWidget);
    expect(find.textContaining('Boolean value'), findsOneWidget);
    expect(find.textContaining('receipt.json'), findsNothing);
    expect(find.textContaining('raw offset'), findsNothing);

    await tester.tap(
      find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
    );
    await tester.pumpAndSettle();
    expect(find.text('Build ready'), findsNothing);
    expect(
      find.text(
        'Choose the Gothic 1 Remake installation in Settings before building files.',
      ),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(
              ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
            ),
          )
          .onPressed,
      isNull,
    );
    expect(find.text('Gameplay unverified'), findsOneWidget);
    expect(find.text('1 byte'), findsOneWidget);
  });

  testWidgets(
    'places the build action beside remove without claiming readiness',
    (tester) async {
      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () async => [stage],
          publish: _unexpectedPublish,
          remove: _unexpectedRemove,
          buildReviewedStage:
              ({
                required targetPath,
                required packName,
                required output,
              }) async => throw StateError('build should not run yet'),
          pickBuildParentDirectory: () async => null,
        ),
      );

      await tester.tap(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      await tester.pumpAndSettle();

      final buildFinder = find.byKey(
        ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
      );
      final removeFinder = find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      );
      expect(find.text('Build ready'), findsNothing);
      expect(find.textContaining('Support is checked'), findsOneWidget);
      expect(tester.widget<FilledButton>(buildFinder).onPressed, isNotNull);
      final buildRect = tester.getRect(buildFinder);
      final removeRect = tester.getRect(removeFinder);
      expect((buildRect.center.dy - removeRect.center.dy).abs(), lessThan(1));
      expect(buildRect.right, lessThanOrEqualTo(removeRect.left));

      await tester.ensureVisible(buildFinder);
      await tester.pumpAndSettle();
      await tester.tap(buildFinder);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dataasset-build-dialog')),
        findsOneWidget,
      );
      expect(find.text('/Game/TestAsset'), findsOneWidget);
      expect(find.text('Build files'), findsOneWidget);
    },
  );

  testWidgets(
    'checkpoint change while build dialog is open never reaches the builder',
    (tester) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_dataasset_stage_stale_build_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));
      var projectRevision = 5;
      var projectHead = fixture.stagedHead;
      var buildCalls = 0;
      late StateSetter rebuild;

      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('same-build-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: projectRevision,
                  projectHead: projectHead,
                  load: () async => [stage],
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  buildReviewedStage:
                      ({
                        required targetPath,
                        required packName,
                        required output,
                      }) async {
                        buildCalls++;
                        throw StateError('stale build callback must not run');
                      },
                  pickBuildParentDirectory: () async => parent.path,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      await tester.pumpAndSettle();
      final build = find.byKey(
        ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
      );
      await tester.ensureVisible(build);
      await tester.pumpAndSettle();
      await tester.tap(build);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-dataasset-build-dialog')),
        findsOneWidget,
      );

      rebuild(() {
        projectRevision = 6;
        projectHead = fixture.removedHead;
      });
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-dataasset-build-pack-name')),
        'StalePack',
      );
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-build-submit')),
      );
      await tester.pumpAndSettle();

      expect(buildCalls, 0);
      expect(
        find.textContaining('project changed while this window was open'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-dataasset-build-submit')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'requires-reopen while build dialog is open never reaches the builder',
    (tester) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_dataasset_stage_locked_build_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));
      var requiresReopen = false;
      var buildCalls = 0;
      late StateSetter rebuild;

      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('same-locked-build-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: 5,
                  projectHead: fixture.stagedHead,
                  requiresReopen: requiresReopen,
                  load: () async => [stage],
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  buildReviewedStage:
                      ({
                        required targetPath,
                        required packName,
                        required output,
                      }) async {
                        buildCalls++;
                        throw StateError('locked build callback must not run');
                      },
                  pickBuildParentDirectory: () async => parent.path,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      await tester.pumpAndSettle();
      final build = find.byKey(
        ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
      );
      await tester.ensureVisible(build);
      await tester.pumpAndSettle();
      await tester.tap(build);
      await tester.pumpAndSettle();

      rebuild(() => requiresReopen = true);
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('revision3-dataasset-build-pack-name')),
        'LockedPack',
      );
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-build-choose-parent')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-build-submit')),
      );
      await tester.pumpAndSettle();

      expect(buildCalls, 0);
      expect(find.textContaining('build may already exist'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-dataasset-build-submit')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'external requires-reopen locks every stage action immediately and unlocks by exact reload',
    (tester) async {
      var requiresReopen = false;
      var loadCalls = 0;
      late StateSetter rebuild;
      await tester.binding.setSurfaceSize(const Size(1200, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return Revision3DataAssetStagePanel(
                  key: const ValueKey('externally-locked-stage-panel'),
                  projectRoot: _projectRoot,
                  projectId: stage.projectId,
                  projectRevision: 5,
                  projectHead: fixture.stagedHead,
                  requiresReopen: requiresReopen,
                  load: () async {
                    loadCalls++;
                    return [stage];
                  },
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  publishSemanticEdit: (_) async =>
                      throw StateError('locked semantic action must not run'),
                  semanticExtractReceiptInspector: _matchingReceiptInspector,
                  browseInstalledPackages: () async =>
                      throw StateError('locked browse action must not run'),
                  buildReviewedStage:
                      ({
                        required targetPath,
                        required packName,
                        required output,
                      }) async =>
                          throw StateError('locked build action must not run'),
                  pickBuildParentDirectory: () async => null,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      await tester.pumpAndSettle();
      await _expandExpertTools(tester);
      expect(loadCalls, 1);

      rebuild(() => requiresReopen = true);
      await tester.pump();

      expect(
        find.textContaining('Reopen the managed project before continuing'),
        findsOneWidget,
      );
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const Key('revision3-dataasset-stage-refresh')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-dataasset-stage-add')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-dataasset-semantic-create')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-dataasset-browse-installed')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                ValueKey('revision3-dataasset-stage-build-${stage.targetPath}'),
              ),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(
                ValueKey(
                  'revision3-dataasset-stage-remove-${stage.targetPath}',
                ),
              ),
            )
            .onPressed,
        isNull,
      );
      expect(loadCalls, 1);

      rebuild(() => requiresReopen = false);
      await tester.pumpAndSettle();
      expect(loadCalls, 2);
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const Key('revision3-dataasset-stage-refresh')),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets('internal integrity lock survives an external lock cycle', (
    tester,
  ) async {
    var requiresReopen = false;
    var loadCalls = 0;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Revision3DataAssetStagePanel(
                key: const ValueKey('internally-locked-stage-panel'),
                projectRoot: _projectRoot,
                projectId: stage.projectId,
                projectRevision: 5,
                projectHead: fixture.stagedHead,
                requiresReopen: requiresReopen,
                load: () async {
                  loadCalls++;
                  throw const Revision3DataAssetRequiresReopenException();
                },
                publish: _unexpectedPublish,
                remove: _unexpectedRemove,
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(loadCalls, 1);

    rebuild(() => requiresReopen = true);
    await tester.pump();
    rebuild(() => requiresReopen = false);
    await tester.pumpAndSettle();

    expect(loadCalls, 1);
    expect(
      tester
          .widget<IconButton>(
            find.byKey(const Key('revision3-dataasset-stage-refresh')),
          )
          .onPressed,
      isNull,
    );
    expect(
      find.textContaining('Reopen the managed project before continuing'),
      findsOneWidget,
    );
  });

  testWidgets('picker cancellation never publishes and busy picker is single', (
    tester,
  ) async {
    final picker = Completer<String?>();
    var pickerCalls = 0;
    var publishCalls = 0;
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 4,
        projectHead: fixture.basisHead,
        load: () async => const [],
        pickPatchReceipt: () {
          pickerCalls++;
          return picker.future;
        },
        publish: ({required patchReceiptPath}) async {
          publishCalls++;
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
        remove: _unexpectedRemove,
      ),
    );

    await _expandExpertTools(tester);
    final add = find.byKey(const Key('revision3-dataasset-stage-add'));
    await tester.tap(add);
    await tester.pump();
    expect(pickerCalls, 1);
    expect(tester.widget<FilledButton>(add).onPressed, isNull);

    picker.complete(null);
    await tester.pumpAndSettle();
    expect(publishCalls, 0);
    expect(tester.widget<FilledButton>(add).onPressed, isNotNull);
  });

  testWidgets('mutations require one successful exact registry read', (
    tester,
  ) async {
    final initialLoad = Completer<List<AuthoringRevision3DataAssetStage>>();
    var pickerCalls = 0;
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () => initialLoad.future,
        pickPatchReceipt: () async {
          pickerCalls++;
          return null;
        },
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      ),
      settle: false,
    );
    await tester.pump();

    await _expandExpertTools(tester);
    final add = find.byKey(const Key('revision3-dataasset-stage-add'));
    expect(tester.widget<FilledButton>(add).onPressed, isNull);
    await tester.tap(add);
    expect(pickerCalls, 0);

    initialLoad.complete([stage]);
    await tester.pumpAndSettle();
    expect(tester.widget<FilledButton>(add).onPressed, isNotNull);
  });

  testWidgets('failed refresh keeps mutations locked on the retained list', (
    tester,
  ) async {
    final refresh = Completer<List<AuthoringRevision3DataAssetStage>>();
    var loads = 0;
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () {
          loads++;
          return loads == 1 ? Future.value([stage]) : refresh.future;
        },
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      ),
    );

    await _expandExpertTools(tester);
    final add = find.byKey(const Key('revision3-dataasset-stage-add'));
    expect(tester.widget<FilledButton>(add).onPressed, isNotNull);
    final refreshAction = find.byKey(
      const Key('revision3-dataasset-stage-refresh'),
    );
    await tester.ensureVisible(refreshAction);
    await tester.pump();
    await tester.tap(refreshAction);
    await tester.pump();
    expect(tester.widget<FilledButton>(add).onPressed, isNull);

    refresh.completeError(StateError('refresh failed'));
    await tester.pumpAndSettle();
    expect(find.text('TestAsset'), findsOneWidget);
    expect(tester.widget<FilledButton>(add).onPressed, isNull);
    expect(
      find.byKey(const Key('revision3-dataasset-stage-action-error')),
      findsOneWidget,
    );
  });

  testWidgets(
    'controller opens only the exact stage at the exact wide checkpoint',
    (tester) async {
      final controller = Revision3DataAssetStagePanelController();
      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () async => [stage],
          publish: _unexpectedPublish,
          remove: _unexpectedRemove,
          controller: controller,
        ),
      );
      addTearDown(controller.dispose);

      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath,
          projectId: 'another-project',
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isFalse,
      );
      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath,
          projectId: stage.projectId,
          projectRevision: 4,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isFalse,
      );
      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.removedHead.canonicalJson,
        ),
        isFalse,
      );
      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath.toLowerCase(),
          projectId: stage.projectId,
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isFalse,
        reason: 'stage identity is exact, not case-folded or fuzzy',
      );
      expect(
        await controller.openStageByIdAtCheckpoint(
          '/Game/Missing',
          projectId: stage.projectId,
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isFalse,
        reason: 'opening the generic registry cannot claim stage success',
      );

      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isTrue,
      );
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<TextField>(
              find.byKey(const Key('revision3-dataasset-stage-search')),
            )
            .controller
            ?.text,
        stage.targetPath,
      );
      expect(
        tester
            .widget<ExpansionTile>(
              find.byKey(
                ValueKey('revision3-dataasset-stage-${stage.targetPath}'),
              ),
            )
            .controller
            ?.isExpanded,
        isTrue,
      );
      expect(
        find.byKey(
          ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'controller buffers compact pre-mount navigation until exact load',
    (tester) async {
      final controller = Revision3DataAssetStagePanelController();
      addTearDown(controller.dispose);
      final load = Completer<List<AuthoringRevision3DataAssetStage>>();
      bool? resolved;
      final opening = controller.openStageByIdAtCheckpoint(
        stage.targetPath,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
      );
      unawaited(opening.then((value) => resolved = value));

      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () => load.future,
          publish: _unexpectedPublish,
          remove: _unexpectedRemove,
          controller: controller,
        ),
        settle: false,
        surfaceSize: const Size(560, 760),
      );
      await tester.pump();
      expect(resolved, isNull, reason: 'lazy mounting does not claim success');

      load.complete([stage]);
      await tester.pumpAndSettle();

      expect(await opening, isTrue);
      expect(
        tester
            .widget<ExpansionTile>(
              find.byKey(
                ValueKey('revision3-dataasset-stage-${stage.targetPath}'),
              ),
            )
            .controller
            ?.isExpanded,
        isTrue,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('pending exact navigation cancels on same-revision head drift', (
    tester,
  ) async {
    final controller = Revision3DataAssetStagePanelController();
    addTearDown(controller.dispose);
    final oldHeadReload = Completer<List<AuthoringRevision3DataAssetStage>>();
    final newHeadReload = Completer<List<AuthoringRevision3DataAssetStage>>();
    var head = fixture.stagedHead;
    var loadCalls = 0;
    late StateSetter rebuild;
    await tester.binding.setSurfaceSize(const Size(1200, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: Revision3DataAssetStagePanel(
                key: const ValueKey('same-dataasset-panel'),
                projectRoot: _projectRoot,
                projectId: stage.projectId,
                projectRevision: 5,
                projectHead: head,
                load: () {
                  loadCalls++;
                  return switch (loadCalls) {
                    1 => Future.value([stage]),
                    2 => oldHeadReload.future,
                    _ => newHeadReload.future,
                  };
                },
                publish: _unexpectedPublish,
                remove: _unexpectedRemove,
                controller: controller,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-dataasset-stage-refresh')),
    );
    await tester.pump();
    final staleOpening = controller.openStageByIdAtCheckpoint(
      stage.targetPath,
      projectId: stage.projectId,
      projectRevision: 5,
      projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
    );
    await tester.pump();

    rebuild(() => head = fixture.removedHead);
    await tester.pump();
    expect(await staleOpening, isFalse);

    final currentOpening = controller.openStageByIdAtCheckpoint(
      stage.targetPath,
      projectId: stage.projectId,
      projectRevision: 5,
      projectHeadCanonicalJson: fixture.removedHead.canonicalJson,
    );
    oldHeadReload.complete([stage]);
    await tester.pump();
    newHeadReload.complete([stage]);
    await tester.pumpAndSettle();

    expect(await currentOpening, isTrue);
    expect(
      tester
          .widget<ExpansionTile>(
            find.byKey(
              ValueKey('revision3-dataasset-stage-${stage.targetPath}'),
            ),
          )
          .controller
          ?.isExpanded,
      isTrue,
    );
  });

  testWidgets(
    'project switch detach and controller disposal reject pending navigation',
    (tester) async {
      final controller = Revision3DataAssetStagePanelController();
      final projectALoad = Completer<List<AuthoringRevision3DataAssetStage>>();
      final projectBLoad = Completer<List<AuthoringRevision3DataAssetStage>>();
      var projectId = stage.projectId;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: Revision3DataAssetStagePanel(
                  key: const ValueKey('switching-dataasset-panel'),
                  projectRoot: _projectRoot,
                  projectId: projectId,
                  projectRevision: 5,
                  projectHead: fixture.stagedHead,
                  load: () => projectId == stage.projectId
                      ? projectALoad.future
                      : projectBLoad.future,
                  publish: _unexpectedPublish,
                  remove: _unexpectedRemove,
                  controller: controller,
                ),
              );
            },
          ),
        ),
      );
      await tester.pump();

      final projectAOpening = controller.openStageByIdAtCheckpoint(
        stage.targetPath,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
      );
      rebuild(() => projectId = 'project-b');
      await tester.pump();
      expect(await projectAOpening, isFalse);

      final projectBOpening = controller.openStageByIdAtCheckpoint(
        stage.targetPath,
        projectId: 'project-b',
        projectRevision: 5,
        projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
      );
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      await tester.pump();
      expect(await projectBOpening, isFalse);

      controller.dispose();
      expect(
        await controller.openStageByIdAtCheckpoint(
          stage.targetPath,
          projectId: 'project-b',
          projectRevision: 5,
          projectHeadCanonicalJson: fixture.stagedHead.canonicalJson,
        ),
        isFalse,
      );
    },
  );

  testWidgets('verified receipt import passes only the selected path', (
    tester,
  ) async {
    String? receivedPath;
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 4,
        projectHead: fixture.basisHead,
        load: () async => const [],
        pickPatchReceipt: () async => r'C:\proof\edit.gore-asset-patch.json',
        publish: ({required patchReceiptPath}) async {
          receivedPath = patchReceiptPath;
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
        remove: _unexpectedRemove,
      ),
    );

    await _expandExpertTools(tester);
    await tester.tap(find.byKey(const Key('revision3-dataasset-stage-add')));
    await tester.pumpAndSettle();

    expect(receivedPath, r'C:\proof\edit.gore-asset-patch.json');
    expect(find.textContaining('saved in project revision 5'), findsOneWidget);
    expect(find.textContaining(receivedPath!), findsNothing);
  });

  testWidgets(
    'remove confirms exact listed target and describes its boundary',
    (tester) async {
      String? removedPath;
      await _pumpPanel(
        tester,
        Revision3DataAssetStagePanel(
          projectRoot: _projectRoot,
          projectId: stage.projectId,
          projectRevision: 5,
          projectHead: fixture.stagedHead,
          load: () async => [stage],
          publish: _unexpectedPublish,
          remove: ({required targetPath}) async {
            removedPath = targetPath;
            return Revision3DataAssetStageRemovalPublication(
              projectId: stage.projectId,
              projectRevision: 6,
              removed: stage,
            );
          },
        ),
      );
      await tester.tap(
        find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
      );
      await tester.pumpAndSettle();
      final remove = find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      );
      await tester.ensureVisible(remove);
      await tester.pumpAndSettle();
      await tester.tap(remove);
      await tester.pumpAndSettle();

      expect(
        find.textContaining('game installation will not be changed'),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-remove-confirm')),
      );
      await tester.pumpAndSettle();

      expect(removedPath, stage.targetPath);
      expect(find.textContaining('No game files were changed'), findsOneWidget);
    },
  );

  testWidgets('stale removal confirmation cannot mutate a newer checkpoint', (
    tester,
  ) async {
    var removeCalls = 0;
    Revision3DataAssetStagePanel panel({
      required int revision,
      required Revision3DataAssetStageLoader load,
    }) => Revision3DataAssetStagePanel(
      key: const ValueKey('same-panel'),
      projectRoot: _projectRoot,
      projectId: stage.projectId,
      projectRevision: revision,
      projectHead: revision == 5 ? fixture.stagedHead : fixture.removedHead,
      load: load,
      publish: _unexpectedPublish,
      remove: ({required targetPath}) async {
        removeCalls++;
        return Revision3DataAssetStageRemovalPublication(
          projectId: stage.projectId,
          projectRevision: revision + 1,
          removed: stage,
        );
      },
    );

    await _pumpPanel(tester, panel(revision: 5, load: () async => [stage]));
    await tester.tap(
      find.byKey(ValueKey('revision3-dataasset-stage-${stage.targetPath}')),
    );
    await tester.pumpAndSettle();
    final remove = find.byKey(
      ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
    );
    await tester.ensureVisible(remove);
    await tester.pumpAndSettle();
    await tester.tap(remove);
    await tester.pumpAndSettle();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox.expand(
            child: panel(revision: 6, load: () async => const []),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-dataasset-remove-confirm')),
    );
    await tester.pumpAndSettle();

    expect(removeCalls, 0);
    expect(
      find.byKey(const Key('revision3-dataasset-stage-empty')),
      findsOneWidget,
    );
  });

  testWidgets(
    'old list cannot overwrite a same-tuple divergent root and head',
    (tester) async {
      final oldLoad = Completer<List<AuthoringRevision3DataAssetStage>>();
      Revision3DataAssetStagePanel panel(
        String root,
        AuthoringWorkingHead head,
        Revision3DataAssetStageLoader load,
      ) => Revision3DataAssetStagePanel(
        key: const ValueKey('same-panel'),
        projectRoot: root,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: head,
        load: load,
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      );

      await _pumpPanel(
        tester,
        panel(_projectRoot, fixture.stagedHead, () => oldLoad.future),
        settle: false,
      );
      await tester.pump();
      await _pumpPanel(
        tester,
        panel(
          r'C:\projects\divergent-clone',
          fixture.removedHead,
          () async => const [],
        ),
      );
      oldLoad.complete([stage]);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-dataasset-stage-empty')),
        findsOneWidget,
      );
      expect(find.text('TestAsset'), findsNothing);
    },
  );

  testWidgets('requires-reopen load failure locks all mutations', (
    tester,
  ) async {
    await _pumpPanel(
      tester,
      Revision3DataAssetStagePanel(
        projectRoot: _projectRoot,
        projectId: stage.projectId,
        projectRevision: 5,
        projectHead: fixture.stagedHead,
        load: () async =>
            throw const Revision3DataAssetRequiresReopenException(),
        publish: _unexpectedPublish,
        remove: _unexpectedRemove,
      ),
    );

    await _expandExpertTools(tester);
    expect(find.textContaining('Reopen the managed project'), findsWidgets);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-dataasset-stage-add')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<IconButton>(
            find.byKey(const Key('revision3-dataasset-stage-refresh')),
          )
          .onPressed,
      isNull,
    );
  });
}

Future<void> _pumpPanel(
  WidgetTester tester,
  Revision3DataAssetStagePanel panel, {
  bool settle = true,
  Size surfaceSize = const Size(1200, 900),
}) async {
  await tester.binding.setSurfaceSize(surfaceSize);
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(body: SizedBox.expand(child: panel)),
    ),
  );
  if (settle) await tester.pumpAndSettle();
}

Future<void> _expandExpertTools(
  WidgetTester tester, {
  Key targetKey = const Key('revision3-dataasset-stage-add'),
}) async {
  final finder = find.byKey(const Key('revision3-dataasset-expert-tools'));
  final tile = tester.widget<ExpansionTile>(finder);
  if (tile.controller?.isExpanded ?? false) return;
  await tester.ensureVisible(finder);
  await tester.pump(const Duration(milliseconds: 200));
  await tester.tap(finder);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 300));
  final headerScroll = tester.state<ScrollableState>(
    find
        .descendant(
          of: find.byKey(const Key('revision3-dataasset-stage-header-scroll')),
          matching: find.byType(Scrollable),
        )
        .first,
  );
  headerScroll.position.jumpTo(headerScroll.position.maxScrollExtent);
  await tester.pump();
  final action = find.byKey(targetKey);
  if (action.evaluate().isNotEmpty) {
    await Scrollable.ensureVisible(
      tester.element(action),
      alignment: 0.5,
      duration: Duration.zero,
    );
    await tester.pump();
  }
}

Future<Revision3DataAssetStagePublication> _unexpectedPublish({
  required String patchReceiptPath,
}) => throw StateError('unexpected DataAsset publication');

Future<Revision3DataAssetStageRemovalPublication> _unexpectedRemove({
  required String targetPath,
}) => throw StateError('unexpected DataAsset removal');

Future<DataAssetExtractReceiptSummary> _matchingReceiptInspector(
  String _,
) async => DataAssetExtractReceiptSummary.fromJson(
  validDataAssetExtractReceiptSummaryResponse(),
);

Future<void> _startSemanticPublication(WidgetTester tester) async {
  await _expandExpertTools(
    tester,
    targetKey: const Key('revision3-dataasset-semantic-create'),
  );
  await tester.tap(
    find.byKey(const Key('revision3-dataasset-semantic-create')),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-pick-usmap')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-inspect')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-semantic-pick-receipt')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-semantic-confirm-target')));
  await tester.pump();
  await tester.enterText(
    find.byKey(const Key('dataasset-semantic-value')),
    '2',
  );
  final scroll = tester.state<ScrollableState>(
    find
        .descendant(
          of: find.byKey(const Key('dataasset-semantic-editor')),
          matching: find.byType(Scrollable),
        )
        .first,
  );
  scroll.position.jumpTo(scroll.position.maxScrollExtent);
  await tester.pump();
  await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
  await tester.pump();
  scroll.position.jumpTo(scroll.position.maxScrollExtent);
  await tester.pumpAndSettle();
  final stage = find.byKey(const Key('dataasset-semantic-stage'));
  await tester.ensureVisible(stage);
  await tester.tap(stage);
  await tester.pump();
}
