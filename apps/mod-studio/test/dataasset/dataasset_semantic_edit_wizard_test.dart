import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dataasset/domain/dataasset_inspection.dart';
import 'package:gore_mod/dataasset/domain/dataasset_semantic_edit.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_wizard.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';

import 'dataasset_test_fixtures.dart';

void main() {
  testWidgets(
    'moves from strict inspection to semantic editor and can go back',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1300, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DataAssetSemanticEditWizardDialog(
              extractReceiptInspector: (_) async =>
                  DataAssetExtractReceiptSummary.fromJson(
                    validDataAssetExtractReceiptSummaryResponse(),
                  ),
              uassetPicker: () async => 'selected.uasset',
              usmapPicker: () async => 'selected.usmap',
              inspector:
                  ({
                    required uassetPath,
                    required usmapPath,
                    exportIndex,
                  }) async => DataAssetInspection.fromJson(
                    validDataAssetInspectionResponse(),
                  ),
              publish: _unexpectedPublish,
            ),
          ),
        ),
      );

      expect(find.textContaining('1 of 2'), findsOneWidget);
      await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-pick-usmap')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('dataasset-inspect')));
      await tester.pumpAndSettle();

      expect(find.textContaining('2 of 2'), findsOneWidget);
      expect(
        find.byKey(const Key('dataasset-semantic-editor')),
        findsOneWidget,
      );
      expect(find.text('Health'), findsWidgets);

      await tester.tap(find.byKey(const Key('dataasset-semantic-wizard-back')));
      await tester.pumpAndSettle();
      expect(find.textContaining('1 of 2'), findsOneWidget);
      expect(find.byKey(const Key('dataasset-inspect')), findsOneWidget);
    },
  );
}

Future<DataAssetSemanticStagePublication> _unexpectedPublish(
  DataAssetSemanticEditIntent _,
) => throw StateError('publisher must not run');
