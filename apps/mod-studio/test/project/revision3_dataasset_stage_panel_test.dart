import 'dart:async';

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
    expect(find.textContaining('not yet included in builds'), findsOneWidget);
    expect(find.text('Build / Deploy'), findsNothing);
    expect(find.text('Test in game'), findsNothing);
  });

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
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-stage-refresh')),
      );
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
    expect(find.text('Build unavailable'), findsOneWidget);
    expect(find.text('Gameplay unverified'), findsOneWidget);
    expect(find.text('1 byte'), findsOneWidget);
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

    final add = find.byKey(const Key('revision3-dataasset-stage-add'));
    expect(tester.widget<FilledButton>(add).onPressed, isNotNull);
    await tester.tap(
      find.byKey(const Key('revision3-dataasset-stage-refresh')),
    );
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
      await tester.tap(
        find.byKey(
          ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
        ),
      );
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
    await tester.tap(
      find.byKey(
        ValueKey('revision3-dataasset-stage-remove-${stage.targetPath}'),
      ),
    );
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
}) async {
  await tester.binding.setSurfaceSize(const Size(1200, 900));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(body: SizedBox.expand(child: panel)),
    ),
  );
  if (settle) await tester.pumpAndSettle();
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
