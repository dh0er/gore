import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/dataasset_lab.dart';

import 'dataasset_test_fixtures.dart';

void main() {
  Future<void> pumpLab(WidgetTester tester, DataAssetLab lab) async {
    await tester.binding.setSurfaceSize(const Size(1400, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(home: Scaffold(body: lab)),
      ),
    );
  }

  testWidgets('picker cancellation preserves the selected snapshot', (
    tester,
  ) async {
    var calls = 0;
    await pumpLab(
      tester,
      DataAssetLab(
        uassetPicker: () async => calls++ == 0 ? 'kept.uasset' : null,
        usmapPicker: () async => null,
        inspector: _unexpectedInspector,
      ),
    );

    await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
    await tester.pumpAndSettle();
    expect(find.text('kept.uasset'), findsOneWidget);

    await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
    await tester.pumpAndSettle();
    expect(find.text('kept.uasset'), findsOneWidget);
  });

  testWidgets('passes the selected inputs and renders proven facts', (
    tester,
  ) async {
    ({String uassetPath, String usmapPath, int? exportIndex})? request;
    await pumpLab(
      tester,
      DataAssetLab(
        uassetPicker: () async => 'selected.uasset',
        usmapPicker: () async => 'selected.usmap',
        inspector:
            ({required uassetPath, required usmapPath, exportIndex}) async {
              request = (
                uassetPath: uassetPath,
                usmapPath: usmapPath,
                exportIndex: exportIndex,
              );
              return DataAssetInspection.fromJson(
                validDataAssetInspectionResponse(
                  exportIndex: 3,
                  packageExports: 5,
                  objectName: 'DA_Selected',
                ),
              );
            },
      ),
    );

    await _chooseBoth(tester);
    await tester.enterText(
      find.byKey(const Key('dataasset-export-index')),
      '3',
    );
    await tester.tap(find.byKey(const Key('dataasset-inspect')));
    await tester.pumpAndSettle();

    expect(request, (
      uassetPath: 'selected.uasset',
      usmapPath: 'selected.usmap',
      exportIndex: 3,
    ));
    expect(find.byKey(const Key('dataasset-summary')), findsOneWidget);
    expect(find.textContaining('DA_Selected'), findsOneWidget);
    expect(find.text('walked'), findsWidgets);
  });

  testWidgets(
    'reports only a completed strictly parsed inspection to a parent flow',
    (tester) async {
      DataAssetInspection? completed;
      await pumpLab(
        tester,
        DataAssetLab(
          uassetPicker: () async => 'selected.uasset',
          usmapPicker: () async => 'selected.usmap',
          inspector:
              ({required uassetPath, required usmapPath, exportIndex}) async =>
                  DataAssetInspection.fromJson(
                    validDataAssetInspectionResponse(),
                  ),
          onInspectionReady: (inspection) => completed = inspection,
        ),
      );

      expect(completed, isNull);
      await _chooseBoth(tester);
      await tester.tap(find.byKey(const Key('dataasset-inspect')));
      await tester.pumpAndSettle();
      expect(completed?.exports.single.objectName, 'DA_Test');
    },
  );

  testWidgets('latest request wins when an older inspection finishes last', (
    tester,
  ) async {
    final first = Completer<DataAssetInspection>();
    final second = Completer<DataAssetInspection>();
    var pickerCalls = 0;
    await pumpLab(
      tester,
      DataAssetLab(
        uassetPicker: () async =>
            pickerCalls++ == 0 ? 'first.uasset' : 'second.uasset',
        usmapPicker: () async => 'schema.usmap',
        inspector: ({required uassetPath, required usmapPath, exportIndex}) =>
            uassetPath == 'first.uasset' ? first.future : second.future,
      ),
    );

    await _chooseBoth(tester);
    await tester.tap(find.byKey(const Key('dataasset-inspect')));
    await tester.pump();
    expect(find.byKey(const Key('dataasset-progress')), findsOneWidget);

    await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
    await tester.pump();
    await tester.pump();
    expect(find.text('second.uasset'), findsOneWidget);
    await tester.tap(find.byKey(const Key('dataasset-inspect')));
    await tester.pump();

    second.complete(
      DataAssetInspection.fromJson(
        validDataAssetInspectionResponse(objectName: 'DA_Second'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('DA_Second'), findsOneWidget);

    first.complete(
      DataAssetInspection.fromJson(
        validDataAssetInspectionResponse(objectName: 'DA_First'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('DA_Second'), findsOneWidget);
    expect(find.textContaining('DA_First'), findsNothing);
  });

  testWidgets('shows bounded errors without inventing result state', (
    tester,
  ) async {
    await pumpLab(
      tester,
      DataAssetLab(
        uassetPicker: () async => 'broken.uasset',
        usmapPicker: () async => 'schema.usmap',
        inspector:
            ({required uassetPath, required usmapPath, exportIndex}) async =>
                throw const FormatException('malformed evidence'),
      ),
    );

    await _chooseBoth(tester);
    await tester.tap(find.byKey(const Key('dataasset-inspect')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('dataasset-error')), findsOneWidget);
    expect(find.textContaining('malformed evidence'), findsOneWidget);
    expect(find.byKey(const Key('dataasset-summary')), findsNothing);
  });

  testWidgets(
    'unsupported exports stay disabled and no mutation controls exist',
    (tester) async {
      final unsupported = _unsupportedInspection(1);
      await pumpLab(
        tester,
        DataAssetLab(
          uassetPicker: () async => 'unsupported.uasset',
          usmapPicker: () async => 'schema.usmap',
          inspector:
              ({required uassetPath, required usmapPath, exportIndex}) async =>
                  unsupported,
        ),
      );

      await _chooseBoth(tester);
      await tester.tap(find.byKey(const Key('dataasset-inspect')));
      await tester.pumpAndSettle();

      final tile = tester.widget<ExpansionTile>(
        find.byKey(const Key('dataasset-export-tile-0')),
      );
      expect(tile.enabled, isFalse);
      expect(find.textContaining('schema_unsupported'), findsOneWidget);
      for (final label in const ['Edit', 'Patch', 'Save', 'Deploy']) {
        expect(find.text(label), findsNothing);
      }
    },
  );

  testWidgets('large export results render lazily and remain searchable', (
    tester,
  ) async {
    final large = _unsupportedInspection(500, markedIndex: 333);
    await pumpLab(
      tester,
      DataAssetLab(
        uassetPicker: () async => 'large.uasset',
        usmapPicker: () async => 'schema.usmap',
        inspector:
            ({required uassetPath, required usmapPath, exportIndex}) async =>
                large,
      ),
    );

    await _chooseBoth(tester);
    await tester.tap(find.byKey(const Key('dataasset-inspect')));
    await tester.pumpAndSettle();

    final initiallyBuilt = find.byType(ExpansionTile).evaluate().length;
    expect(initiallyBuilt, greaterThan(0));
    expect(initiallyBuilt, lessThan(40));

    await tester.enterText(
      find.byKey(const Key('dataasset-search')),
      'needle_export',
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('Needle_Export'), findsOneWidget);
    expect(find.byType(ExpansionTile), findsOneWidget);
  });
}

Future<void> _chooseBoth(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('dataasset-pick-uasset')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-pick-usmap')));
  await tester.pumpAndSettle();
}

Future<DataAssetInspection> _unexpectedInspector({
  required String uassetPath,
  required String usmapPath,
  int? exportIndex,
}) => throw StateError('inspector should not run');

DataAssetInspection _unsupportedInspection(int count, {int? markedIndex}) {
  final response = validDataAssetInspectionResponse();
  response['status'] = 'unsupported';
  final summary = response['summary'] as Map<String, Object?>;
  summary['package_exports'] = count;
  summary['reported_exports'] = count;
  summary['walked_exports'] = 0;
  summary['editable_leaves'] = 0;
  final template = dataAssetExport(response);
  final exports = <Object?>[];
  for (var index = 0; index < count; index++) {
    final export = cloneDataAssetResponse(template);
    export['index'] = index;
    export['object_name'] = index == markedIndex
        ? 'Needle_Export'
        : 'Unsupported_$index';
    export['status'] = 'unsupported';
    export['failure'] = <String, Object?>{
      'stage': 'schema',
      'code': 'schema_unsupported',
    };
    export['schema'] = null;
    export['property_bytes'] = null;
    export['native_suffix_bytes'] = null;
    export['leaves'] = <Object?>[];
    exports.add(export);
  }
  response['exports'] = exports;
  return DataAssetInspection.fromJson(response);
}
