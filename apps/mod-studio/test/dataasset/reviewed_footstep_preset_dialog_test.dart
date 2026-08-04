import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dataasset/domain/dataasset_inspection.dart';
import 'package:gore_mod/dataasset/domain/reviewed_dataasset_schema.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';
import 'package:gore_mod/dataasset/ui/installed_dataasset_semantic_edit_dialog.dart';
import 'package:gore_mod/dataasset/ui/reviewed_footstep_preset_dialog.dart';

const _wolfPath =
    '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';

void main() {
  testWidgets('shows reviewed Wolf evidence and honest capability boundaries', (
    tester,
  ) async {
    await _pumpHost(tester, publish: _unexpectedPublish);
    await _openDialog(tester);

    expect(find.text('Wolf footsteps'), findsOneWidget);
    expect(find.text('Footprint texture size'), findsOneWidget);
    expect(
      find.text('raw asset units — gameplay meaning not yet qualified'),
      findsOneWidget,
    );
    expect(find.text('Reviewed structure'), findsOneWidget);
    expect(find.text('Offline build available after saving'), findsOneWidget);
    expect(find.text('Gameplay/runtime unverified'), findsOneWidget);
    expect(find.text('Deployment unverified'), findsOneWidget);
    expect(find.text('Build unavailable'), findsNothing);
    expect(find.text('Deployment unavailable'), findsNothing);
    expect(find.textContaining('Z 0.0 · W 1.0'), findsOneWidget);
    expect(_fieldText(tester, 'reviewed-footstep-x'), '10.0');
    expect(_fieldText(tester, 'reviewed-footstep-y'), '10.0');
    expect(
      find.text('Choose a size different from the current X and Y values.'),
      findsOneWidget,
    );
    expect(_stageButton(tester).onPressed, isNull);
  });

  testWidgets('preset and direct input publish only the semantic request', (
    tester,
  ) async {
    ReviewedDataAssetEditRequest? publishedRequest;
    InstalledDataAssetSemanticEditResult? result;
    await _pumpHost(
      tester,
      publish: (request) async {
        publishedRequest = request;
        return const DataAssetSemanticStagePublication(
          targetPath: _wolfPath,
          revision: 8,
        );
      },
      onResult: (value) => result = value,
    );
    await _openDialog(tester);

    final preset = find.byKey(const Key('reviewed-footstep-preset-150'));
    await tester.ensureVisible(preset);
    await tester.tap(preset);
    await tester.pump();
    expect(_fieldText(tester, 'reviewed-footstep-x'), '15');
    expect(_fieldText(tester, 'reviewed-footstep-y'), '15');

    final yField = find.byKey(const Key('reviewed-footstep-y'));
    await tester.ensureVisible(yField);
    await tester.enterText(yField, '17.5');
    await tester.pump();
    expect(
      find.text('Values are valid. Preview the change before staging.'),
      findsOneWidget,
    );
    expect(_stageButton(tester).onPressed, isNull);

    await tester.tap(find.byKey(const Key('reviewed-footstep-preview')));
    await tester.pump();
    expect(
      find.byKey(const Key('reviewed-footstep-before-after')),
      findsOneWidget,
    );
    expect(find.textContaining('X 10.0 → 15'), findsOneWidget);
    expect(find.textContaining('Y 10.0 → 17.5'), findsOneWidget);
    expect(_stageButton(tester).onPressed, isNotNull);

    await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
    await tester.pumpAndSettle();
    expect(publishedRequest?.x, '15');
    expect(publishedRequest?.y, '17.5');
    expect(result?.publication?.revision, 8);

    final wire = publishedRequest!.toJson();
    expect(wire.keys, <String>{
      'format',
      'schema_id',
      'schema_revision',
      'field_id',
      'value',
    });
    final encoded = jsonEncode(wire);
    for (final forbidden in <String>[
      'target_path',
      'selector',
      'replacement',
      'expected_hex',
      'package_seal',
      'usmap',
      'project',
      'root',
    ]) {
      expect(encoded, isNot(contains(forbidden)), reason: forbidden);
    }
  });

  testWidgets('invalid and no-change states update the live status', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await _pumpHost(tester, publish: _unexpectedPublish);
    await _openDialog(tester);
    final liveStatus = find.byKey(const Key('reviewed-footstep-live-status'));
    await tester.ensureVisible(liveStatus);
    await tester.pump();

    expect(
      tester.getSemantics(liveStatus),
      matchesSemantics(
        label: 'Choose a size different from the current X and Y values.',
        isLiveRegion: true,
      ),
    );

    final xField = find.byKey(const Key('reviewed-footstep-x'));
    await tester.ensureVisible(xField);
    await tester.enterText(xField, '0');
    await tester.pump();
    await tester.ensureVisible(liveStatus);
    await tester.pump();
    expect(
      tester.getSemantics(liveStatus),
      matchesSemantics(
        label: 'Enter positive finite numbers for both X size and Y size.',
        isLiveRegion: true,
      ),
    );
    expect(_stageButton(tester).onPressed, isNull);

    await tester.enterText(xField, '11');
    await tester.pump();
    await tester.tap(find.byKey(const Key('reviewed-footstep-preview')));
    await tester.pump();
    expect(
      find.byKey(const Key('reviewed-footstep-before-after')),
      findsOneWidget,
    );
    expect(_stageButton(tester).onPressed, isNotNull);

    await tester.enterText(xField, '12');
    await tester.pump();
    expect(
      find.byKey(const Key('reviewed-footstep-before-after')),
      findsNothing,
    );
    expect(_stageButton(tester).onPressed, isNull);

    final unchanged = find.byKey(const Key('reviewed-footstep-preset-100'));
    await tester.ensureVisible(unchanged);
    await tester.tap(unchanged);
    await tester.pump();
    expect(
      find.text('Choose a size different from the current X and Y values.'),
      findsOneWidget,
    );
    semantics.dispose();
  });

  testWidgets('returns known unavailable and closes unknown failures safely', (
    tester,
  ) async {
    InstalledDataAssetSemanticEditResult? result;
    await _pumpHost(
      tester,
      publish: (_) async =>
          throw const DataAssetSemanticStageUnavailableException.staleCheckpoint(),
      onResult: (value) => result = value,
    );
    await _openDialog(tester);
    await _previewDifferentValue(tester);
    await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
    await tester.pumpAndSettle();
    expect(
      result?.unavailable?.reason,
      DataAssetSemanticStageUnavailableReason.staleCheckpoint,
    );

    result = null;
    await _pumpHost(
      tester,
      publish: (_) async => throw StateError('uncertain transport outcome'),
      onResult: (value) => result = value,
    );
    await _openDialog(tester);
    await _previewDifferentValue(tester);
    await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
    await tester.pumpAndSettle();
    expect(
      result?.unavailable?.reason,
      DataAssetSemanticStageUnavailableReason.unknownOutcome,
    );
  });

  testWidgets('active publication exposes progress and cannot be dismissed', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final completion = Completer<DataAssetSemanticStagePublication>();
    InstalledDataAssetSemanticEditResult? result;
    var calls = 0;
    await _pumpHost(
      tester,
      publish: (_) {
        calls++;
        return completion.future;
      },
      onResult: (value) => result = value,
    );
    await _openDialog(tester);
    await _previewDifferentValue(tester);
    await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
    await tester.pump();

    expect(calls, 1);
    expect(find.byKey(const Key('reviewed-footstep-progress')), findsOneWidget);
    expect(
      tester.getSemantics(
        find.byKey(const Key('reviewed-footstep-busy-status')),
      ),
      matchesSemantics(
        label: 'Rechecking the installed asset and staging the reviewed edit',
        isLiveRegion: true,
      ),
    );

    await tester.tapAt(const Offset(4, 4));
    await tester.pump();
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('reviewed-footstep-preset-dialog')),
      findsOneWidget,
    );
    expect(calls, 1);

    completion.complete(
      const DataAssetSemanticStagePublication(
        targetPath: _wolfPath,
        revision: 9,
      ),
    );
    await tester.pumpAndSettle();
    expect(result?.publication?.revision, 9);
    expect(
      find.byKey(const Key('reviewed-footstep-preset-dialog')),
      findsNothing,
    );
    semantics.dispose();
  });

  testWidgets('content remains scrollable on a compact surface', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(420, 500);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);

    await _pumpHost(tester, publish: _unexpectedPublish);
    await _openDialog(tester);
    expect(find.byType(SingleChildScrollView), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _pumpHost(
  WidgetTester tester, {
  required ReviewedDataAssetStagePublisher publish,
  ValueChanged<InstalledDataAssetSemanticEditResult?>? onResult,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Builder(
        builder: (context) => TextButton(
          key: const Key('open-reviewed-footstep-dialog'),
          onPressed: () async {
            final result =
                await showDialog<InstalledDataAssetSemanticEditResult>(
                  context: context,
                  builder: (context) => ReviewedFootstepPresetDialog(
                    evidence: _wolfEvidence(),
                    publish: publish,
                  ),
                );
            onResult?.call(result);
          },
          child: const Text('Open'),
        ),
      ),
    ),
  ),
);

Future<void> _openDialog(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('open-reviewed-footstep-dialog')));
  await tester.pumpAndSettle();
  expect(
    find.byKey(const Key('reviewed-footstep-preset-dialog')),
    findsOneWidget,
  );
}

Future<void> _previewDifferentValue(WidgetTester tester) async {
  final xField = find.byKey(const Key('reviewed-footstep-x'));
  await tester.ensureVisible(xField);
  await tester.enterText(xField, '11');
  await tester.pump();
  await tester.tap(find.byKey(const Key('reviewed-footstep-preview')));
  await tester.pump();
  expect(_stageButton(tester).onPressed, isNotNull);
}

String _fieldText(WidgetTester tester, String key) =>
    tester.widget<TextField>(find.byKey(Key(key))).controller!.text;

FilledButton _stageButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('reviewed-footstep-stage')),
);

Future<DataAssetSemanticStagePublication> _unexpectedPublish(
  ReviewedDataAssetEditRequest request,
) => throw StateError('unexpected publication: ${request.canonicalJson}');

ReviewedFootstepPresetInspection _wolfEvidence() {
  final evidence = ReviewedFootstepPresetInspection.tryMatch(
    packagePath: _wolfPath,
    inspection: DataAssetInspection.fromJson(_wolfInspectionJson()),
  );
  return evidence!;
}

Map<String, Object?> _wolfInspectionJson() => <String, Object?>{
  'ok': true,
  'format': 'gore.dataasset.fixed-inspect.v1',
  'status': 'walked',
  'summary': <String, Object?>{
    'package_exports': 1,
    'reported_exports': 1,
    'walked_exports': 1,
    'editable_leaves': 1,
  },
  'selector_format': <String, Object?>{'format': 1, 'profile': 'g1r_ue5_4'},
  'binding': <String, Object?>{
    'package_seal': <String, Object?>{
      'uasset_sha256': 'a' * 64,
      'uexp_sha256': 'b' * 64,
    },
    'usmap_sha256': 'c' * 64,
  },
  'input': <String, Object?>{
    'uasset_length': 1290,
    'uexp_length': 90,
    'usmap_length': 2516955,
  },
  'selection': <String, Object?>{'export_index': null},
  'exports': <Object?>[
    <String, Object?>{
      'index': 0,
      'object_name': 'DA_WolfFootsteps',
      'class_path': '/Script/G1R.FootstepTag',
      'component': 'uexp',
      'length': 86,
      'status': 'walked',
      'failure': null,
      'schema': '/Script/G1R.FootstepTag',
      'property_bytes': 82,
      'native_suffix_bytes': 4,
      'leaves': <Object?>[
        <String, Object?>{
          'index': 0,
          'editable': true,
          'selector': <String, Object?>{
            'format': 1,
            'profile': 'g1r_ue5_4',
            'package_seal': <String, Object?>{
              'uasset_sha256': 'a' * 64,
              'uexp_sha256': 'b' * 64,
            },
            'usmap_sha256': 'c' * 64,
            'export_index': 0,
            'object_name': 'DA_WolfFootsteps',
            'class_path': '/Script/G1R.FootstepTag',
            'component': 'uexp',
            'export_sha256': 'd' * 64,
            'role': 'property_value',
            'kind': 'vector4_f64x4',
            'path': <Object?>[
              <String, Object?>{
                'step': 'property',
                'schema_index': 0,
                'property_name': 'BoneData',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': 'FootstepTag',
                'declaring_module_path': '/Script/G1R',
                'property_type': <String, Object?>{
                  'type': 'struct',
                  'name': 'BoneFeetData',
                },
              },
              <String, Object?>{
                'step': 'struct',
                'name': 'BoneFeetData',
                'schema_name': '/Script/G1R.BoneFeetData',
              },
              <String, Object?>{
                'step': 'property',
                'schema_index': 0,
                'property_name': 'FeetTextureSize',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': 'BoneFeetData',
                'declaring_module_path': '/Script/G1R',
                'property_type': <String, Object?>{
                  'type': 'struct',
                  'name': 'Vector4',
                },
              },
            ],
            'expected_hex':
                '000000000000244000000000000024400000000000000000000000000000f03f',
          },
        },
      ],
    },
  ],
};
