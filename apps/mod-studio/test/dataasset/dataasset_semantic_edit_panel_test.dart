import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dataasset/domain/dataasset_inspection.dart';
import 'package:gore_mod/dataasset/domain/dataasset_semantic_edit.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';

import 'dataasset_test_fixtures.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester,
    DataAssetSemanticEditPanel panel,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1100, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: panel)));
  }

  testWidgets('previews friendly diff and stages the exact semantic intent', (
    tester,
  ) async {
    DataAssetSemanticEditIntent? intent;
    await pumpPanel(
      tester,
      DataAssetSemanticEditPanel(
        inspection: _inspection(),
        initialExtractReceiptPath: r'C:\proof\gore-asset-extract.json',
        extractReceiptInspector: _matchingReceiptInspector,
        publish: (value) async {
          intent = value;
          return const DataAssetSemanticStagePublication(
            targetPath: '/Game/Data/DA_Test',
            revision: 4,
          );
        },
      ),
    );
    await _confirmReceipt(tester);

    expect(find.text('Health'), findsWidgets);
    expect(find.textContaining('Current: 1'), findsOneWidget);
    await tester.enterText(
      find.byKey(const Key('dataasset-semantic-value')),
      '42',
    );
    expect(
      tester
          .widget<ButtonStyleButton>(
            find.byKey(const Key('dataasset-semantic-preview')),
          )
          .onPressed,
      isNotNull,
    );
    await tester.ensureVisible(
      find.byKey(const Key('dataasset-semantic-preview')),
    );
    await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
    await tester.pumpAndSettle();

    final previewError = find.byKey(const Key('dataasset-semantic-error'));
    if (previewError.evaluate().isNotEmpty) {
      fail(tester.widget<Text>(previewError).data ?? 'unknown preview error');
    }

    expect(find.byKey(const Key('dataasset-semantic-diff')), findsOneWidget);
    expect(find.text('Before: 1'), findsOneWidget);
    expect(find.text('After: 42'), findsOneWidget);
    await _tapStage(tester);
    await tester.pumpAndSettle();

    expect(intent, isNotNull);
    expect(intent!.extractReceiptPath, r'C:\proof\gore-asset-extract.json');
    expect(intent!.selector.pathLabel, 'Health');
    expect(intent!.replacement.toJson(), <String, Object>{
      'kind': 'int32',
      'decimal': '42',
    });
    expect(find.byKey(const Key('dataasset-semantic-success')), findsOneWidget);
    expect(find.textContaining('project revision 4'), findsOneWidget);
  });

  testWidgets(
    'requires provenance, rejects no-op, and preserves picker cancellation',
    (tester) async {
      var picks = 0;
      await pumpPanel(
        tester,
        DataAssetSemanticEditPanel(
          inspection: _inspection(),
          initialExtractReceiptPath: r'C:\proof\kept.json',
          extractReceiptInspector: _matchingReceiptInspector,
          extractReceiptPicker: () async {
            picks++;
            return null;
          },
          publish: _unexpectedPublish,
        ),
      );

      await _confirmReceipt(tester);

      await tester.tap(
        find.byKey(const Key('dataasset-semantic-pick-receipt')),
      );
      await tester.pumpAndSettle();
      expect(picks, 1);
      expect(find.text(r'C:\proof\kept.json'), findsOneWidget);

      await tester.ensureVisible(
        find.byKey(const Key('dataasset-semantic-preview')),
      );
      await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
      await tester.pumpAndSettle();
      expect(find.textContaining('would not change'), findsOneWidget);
      expect(find.byKey(const Key('dataasset-semantic-stage')), findsNothing);
    },
  );

  testWidgets('in-flight result cannot overwrite a newly adopted inspection', (
    tester,
  ) async {
    final publication = Completer<DataAssetSemanticStagePublication>();
    var inspection = _inspection(objectName: 'DA_First');
    late StateSetter rebuild;
    await tester.binding.setSurfaceSize(const Size(1100, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return DataAssetSemanticEditPanel(
                inspection: inspection,
                initialExtractReceiptPath: r'C:\proof\extract.json',
                extractReceiptInspector: _matchingReceiptInspector,
                publish: (_) => publication.future,
              );
            },
          ),
        ),
      ),
    );
    await _confirmReceipt(tester);
    await tester.enterText(
      find.byKey(const Key('dataasset-semantic-value')),
      '2',
    );
    await tester.ensureVisible(
      find.byKey(const Key('dataasset-semantic-preview')),
    );
    await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
    await tester.pump();
    await _tapStage(tester);
    await tester.pump();
    expect(
      find.byKey(const Key('dataasset-semantic-progress')),
      findsOneWidget,
    );

    rebuild(() {
      inspection = _inspection(objectName: 'DA_Second');
    });
    await tester.pump();
    expect(find.textContaining('DA_Second'), findsOneWidget);

    publication.complete(
      const DataAssetSemanticStagePublication(
        targetPath: '/Game/Data/Stale',
        revision: 99,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('dataasset-semantic-success')), findsNothing);
    expect(find.textContaining('revision 99'), findsNothing);
  });

  testWidgets('publisher failures do not expose private diagnostics', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      DataAssetSemanticEditPanel(
        inspection: _inspection(),
        initialExtractReceiptPath: r'C:\proof\extract.json',
        extractReceiptInspector: _matchingReceiptInspector,
        publish: (_) async =>
            throw StateError(r'C:\private\secret\receipt.json'),
      ),
    );
    await _confirmReceipt(tester);
    await tester.enterText(
      find.byKey(const Key('dataasset-semantic-value')),
      '2',
    );
    await tester.ensureVisible(
      find.byKey(const Key('dataasset-semantic-preview')),
    );
    await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
    await tester.pump();
    await _tapStage(tester);
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('dataasset-semantic-error')), findsOneWidget);
    expect(find.textContaining('could not be staged'), findsOneWidget);
    expect(find.textContaining('private'), findsNothing);
    expect(find.textContaining('secret'), findsNothing);
  });

  testWidgets('non-retryable stale checkpoint is reported to the wizard', (
    tester,
  ) async {
    DataAssetSemanticStageUnavailableException? unavailable;
    await pumpPanel(
      tester,
      DataAssetSemanticEditPanel(
        inspection: _inspection(),
        initialExtractReceiptPath: r'C:\proof\extract.json',
        extractReceiptInspector: _matchingReceiptInspector,
        publish: (_) async =>
            throw const DataAssetSemanticStageUnavailableException.staleCheckpoint(),
        onUnavailable: (error) => unavailable = error,
      ),
    );
    await _confirmReceipt(tester);
    await tester.enterText(
      find.byKey(const Key('dataasset-semantic-value')),
      '2',
    );
    await tester.ensureVisible(
      find.byKey(const Key('dataasset-semantic-preview')),
    );
    await tester.tap(find.byKey(const Key('dataasset-semantic-preview')));
    await tester.pump();
    await _tapStage(tester);
    await tester.pumpAndSettle();

    expect(
      unavailable?.reason,
      DataAssetSemanticStageUnavailableReason.staleCheckpoint,
    );
    expect(find.byKey(const Key('dataasset-semantic-success')), findsNothing);
  });

  testWidgets('picker failure is friendly and retryable', (tester) async {
    var picks = 0;
    await pumpPanel(
      tester,
      DataAssetSemanticEditPanel(
        inspection: _inspection(),
        extractReceiptInspector: _matchingReceiptInspector,
        extractReceiptPicker: () async {
          picks++;
          if (picks == 1) throw StateError(r'C:\private\picker-failure');
          return r'C:\proof\retry.json';
        },
        publish: _unexpectedPublish,
      ),
    );

    await tester.tap(find.byKey(const Key('dataasset-semantic-pick-receipt')));
    await tester.pumpAndSettle();
    expect(find.textContaining('could not be opened'), findsOneWidget);
    expect(find.textContaining('private'), findsNothing);
    expect(
      tester
          .widget<ButtonStyleButton>(
            find.byKey(const Key('dataasset-semantic-pick-receipt')),
          )
          .onPressed,
      isNotNull,
    );

    await tester.tap(find.byKey(const Key('dataasset-semantic-pick-receipt')));
    await tester.pumpAndSettle();
    expect(picks, 2);
    expect(find.text(r'C:\proof\retry.json'), findsOneWidget);
    expect(find.text('/Game/Data/DA_Test'), findsOneWidget);
  });

  testWidgets('receipt facts must match and exact target needs confirmation', (
    tester,
  ) async {
    var useMismatch = true;
    await pumpPanel(
      tester,
      DataAssetSemanticEditPanel(
        inspection: _inspection(),
        extractReceiptInspector: (_) async {
          final wire = validDataAssetExtractReceiptSummaryResponse();
          if (useMismatch) {
            (wire['package_seal'] as Map<String, Object?>)['uasset_sha256'] =
                'f' * 64;
          }
          return DataAssetExtractReceiptSummary.fromJson(wire);
        },
        extractReceiptPicker: () async => r'C:\proof\chosen.json',
        publish: _unexpectedPublish,
      ),
    );

    await tester.tap(find.byKey(const Key('dataasset-semantic-pick-receipt')));
    await tester.pumpAndSettle();
    expect(find.textContaining('does not match'), findsOneWidget);
    expect(
      find.byKey(const Key('dataasset-semantic-confirm-target')),
      findsNothing,
    );

    useMismatch = false;
    await tester.tap(find.byKey(const Key('dataasset-semantic-pick-receipt')));
    await tester.pumpAndSettle();
    expect(find.text('/Game/Data/DA_Test'), findsOneWidget);
    expect(
      tester
          .widget<ButtonStyleButton>(
            find.byKey(const Key('dataasset-semantic-preview')),
          )
          .onPressed,
      isNull,
    );
    await _confirmReceipt(tester);
    expect(
      tester
          .widget<ButtonStyleButton>(
            find.byKey(const Key('dataasset-semantic-preview')),
          )
          .onPressed,
      isNotNull,
    );
  });
}

DataAssetInspection _inspection({String objectName = 'DA_Test'}) =>
    DataAssetInspection.fromJson(
      validDataAssetInspectionResponse(objectName: objectName),
    );

Future<DataAssetSemanticStagePublication> _unexpectedPublish(
  DataAssetSemanticEditIntent _,
) => throw StateError('publisher must not run');

Future<DataAssetExtractReceiptSummary> _matchingReceiptInspector(
  String _,
) async => DataAssetExtractReceiptSummary.fromJson(
  validDataAssetExtractReceiptSummaryResponse(),
);

Future<void> _confirmReceipt(WidgetTester tester) async {
  await tester.pumpAndSettle();
  expect(find.text('/Game/Data/DA_Test'), findsOneWidget);
  await tester.tap(find.byKey(const Key('dataasset-semantic-confirm-target')));
  await tester.pump();
  expect(
    tester
        .widget<CheckboxListTile>(
          find.byKey(const Key('dataasset-semantic-confirm-target')),
        )
        .value,
    isTrue,
  );
}

Future<void> _tapStage(WidgetTester tester) async {
  final stage = find.byKey(const Key('dataasset-semantic-stage'));
  for (var attempt = 0; attempt < 5 && stage.evaluate().isEmpty; attempt++) {
    await tester.drag(
      find.byKey(const Key('dataasset-semantic-editor')),
      const Offset(0, -300),
    );
    await tester.pump();
  }
  expect(stage, findsOneWidget);
  await tester.ensureVisible(stage);
  await tester.pump();
  await tester.tap(stage);
}
