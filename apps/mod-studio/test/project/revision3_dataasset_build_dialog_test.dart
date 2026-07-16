import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_dataasset_build_dialog.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_dataasset_fixture.dart';

const _targetPath = '/Game/TestAsset';
const _projectId = '07070707070707070707070707070707';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  late Revision3DataAssetFixture fixture;

  setUp(() => fixture = revision3DataAssetNativeGoldenFixture());

  test('pack names match the native portable component boundary', () {
    expect(validateRevision3DataAssetPackName('A'), isNull);
    expect(validateRevision3DataAssetPackName('A_b-9'), isNull);
    expect(validateRevision3DataAssetPackName('A' * 96), isNull);

    for (final invalid in <String>[
      '',
      '_starts_wrong',
      '-starts-wrong',
      'has space',
      'has.dot',
      'Größe',
      'A' * 97,
      'CON',
      'prn',
      'AUX',
      'NUL',
      'COM1',
      'lpt9',
    ]) {
      expect(
        validateRevision3DataAssetPackName(invalid),
        isNotNull,
        reason: invalid,
      );
    }
  });

  testWidgets('shows the saved asset and published build destination', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_dialog_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    String? requestedName;
    String? requestedOutput;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async {
        requestedName = packName;
        requestedOutput = output;
        return _result(
          fixture,
          packName: packName,
          output: output,
          outcome: AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
        );
      },
    );

    expect(find.text('TestAsset'), findsOneWidget);
    expect(find.text(_targetPath), findsOneWidget);
    expect(
      find.text(
        'Creates a new set of mod files for this saved edit. Your project and game installation are not changed.',
      ),
      findsOneWidget,
    );
    expect(
      tester.widget<SelectableText>(
        find.byKey(const Key('revision3-dataasset-build-target-path')),
      ),
      isA<SelectableText>(),
    );

    await _chooseParentAndName(tester, 'AsghanData', parent.path);
    final output = p.join(parent.path, 'AsghanData');
    expect(find.text(output), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    expect(requestedName, 'AsghanData');
    expect(requestedOutput, output);
    expect(find.text('Build complete'), findsOneWidget);
    expect(find.text('Files created in $output'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-dataasset-build-submit')),
      findsNothing,
    );
  });

  testWidgets('reports published cleanup warning as a completed build', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_cleanup_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async => _result(
        fixture,
        packName: packName,
        output: output,
        outcome: AuthoringRevision3ReviewedDataAssetBuildOutcome
            .publishedWithCleanupWarning,
      ),
    );
    await _chooseParentAndName(tester, 'CleanupPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    expect(find.text('Build complete'), findsOneWidget);
    expect(
      find.textContaining('temporary files could not be cleaned up'),
      findsOneWidget,
    );
  });

  testWidgets('publication uncertainty is terminal and never offers retry', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_uncertain_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    var calls = 0;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async {
        calls += 1;
        return _result(
          fixture,
          packName: packName,
          output: output,
          outcome: AuthoringRevision3ReviewedDataAssetBuildOutcome
              .publicationUncertain,
        );
      },
    );
    await _chooseParentAndName(tester, 'UncertainPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    final output = p.join(parent.path, 'UncertainPack');
    expect(calls, 1);
    expect(
      find.text(
        'The build may already exist at $output. Check that folder before trying again.',
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-dataasset-build-uncertain')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-dataasset-build-submit')),
      findsNothing,
    );
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const Key('revision3-dataasset-build-choose-parent')),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets(
    'malformed result after entering the build boundary is terminal uncertainty',
    (tester) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_dataasset_build_malformed_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));

      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        build: ({required packName, required output}) async =>
            throw const ModFfiException(
              command: 'authoring_store_build_revision3_reviewed_dataasset_v1',
              code: ModFfiException.malformedNativeResponseCode,
              message: 'malformed native response',
            ),
      );
      await _chooseParentAndName(tester, 'MalformedPack', parent.path);
      await tester.tap(
        find.byKey(const Key('revision3-dataasset-build-submit')),
      );
      await tester.pumpAndSettle();

      final output = p.join(parent.path, 'MalformedPack');
      expect(
        find.text(
          'The build may already exist at $output. Check that folder before trying again.',
        ),
        findsOneWidget,
      );
      expect(_submitButton(tester).onPressed, isNull);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-dataasset-build-choose-parent')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets('unexpected post-call failure never claims that output is absent', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_unknown_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async =>
          throw StateError('post-call binding failed'),
    );
    await _chooseParentAndName(tester, 'UnknownPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    final output = p.join(parent.path, 'UnknownPack');
    expect(
      find.text(
        'The build may already exist at $output. Check that folder before trying again.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('Nothing was added'), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets('unsupported saved edit closes the unchanged-input retry path', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_unsupported_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async =>
          throw const ModFfiException(
            command: 'authoring_store_build_revision3_reviewed_dataasset_v1',
            code: 'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED',
            message: 'not in reviewed profile',
          ),
    );
    await _chooseParentAndName(tester, 'UnsupportedPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    expect(find.textContaining('not ready to build'), findsOneWidget);
    expect(find.textContaining('may already exist'), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets('rejects an existing output before invoking the exact build', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_existing_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    Directory(p.join(parent.path, 'ExistingPack')).createSync();
    var calls = 0;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) async {
        calls += 1;
        return _result(
          fixture,
          packName: packName,
          output: output,
          outcome: AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
        );
      },
    );
    await _chooseParentAndName(tester, 'ExistingPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pumpAndSettle();

    expect(calls, 0);
    expect(find.textContaining('folder already exists'), findsOneWidget);
  });

  testWidgets('pending build disables dismissal and duplicate submission', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_dataasset_build_pending_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    final completion =
        Completer<AuthoringRevision3ReviewedDataAssetBuildResult>();
    var calls = 0;

    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      build: ({required packName, required output}) {
        calls += 1;
        return completion.future;
      },
    );
    await _chooseParentAndName(tester, 'BusyPack', parent.path);
    await tester.tap(find.byKey(const Key('revision3-dataasset-build-submit')));
    await tester.pump();

    expect(calls, 1);
    expect(find.text('Building...'), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNull);
    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-dataasset-build-close')),
          )
          .onPressed,
      isNull,
    );
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-dataasset-build-dialog')),
      findsOneWidget,
    );

    completion.complete(
      _result(
        fixture,
        packName: 'BusyPack',
        output: p.join(parent.path, 'BusyPack'),
        outcome: AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Build complete'), findsOneWidget);
    expect(calls, 1);
  });
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3DataAssetBuildParentDirectoryPicker pickParent,
  required Revision3ReviewedDataAssetExactBuild build,
}) async {
  await tester.binding.setSurfaceSize(const Size(1100, 900));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-dataasset-build'),
            onPressed: () =>
                showDialog<AuthoringRevision3ReviewedDataAssetBuildResult>(
                  context: context,
                  barrierDismissible: false,
                  builder: (_) => Revision3DataAssetBuildDialog(
                    targetPath: _targetPath,
                    build: build,
                    pickExistingParentDirectory: pickParent,
                  ),
                ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dataasset-build')));
  await tester.pumpAndSettle();
}

Future<void> _chooseParentAndName(
  WidgetTester tester,
  String packName,
  String expectedParent,
) async {
  await tester.enterText(
    find.byKey(const Key('revision3-dataasset-build-pack-name')),
    packName,
  );
  await tester.tap(
    find.byKey(const Key('revision3-dataasset-build-choose-parent')),
  );
  await tester.pumpAndSettle();
  expect(find.text(expectedParent), findsOneWidget);
}

FilledButton _submitButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-dataasset-build-submit')),
);

AuthoringRevision3ReviewedDataAssetBuildResult _result(
  Revision3DataAssetFixture fixture, {
  required String packName,
  required String output,
  required AuthoringRevision3ReviewedDataAssetBuildOutcome outcome,
}) {
  final (wireOutcome, publicationStatus, warning) = switch (outcome) {
    AuthoringRevision3ReviewedDataAssetBuildOutcome.published => (
      'built',
      'published',
      null,
    ),
    AuthoringRevision3ReviewedDataAssetBuildOutcome
        .publishedWithCleanupWarning =>
      (
        'built_with_cleanup_warning',
        'published_with_cleanup_warning',
        <String, Object?>{
          'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
          'message':
              'the verified build was published, but private staging cleanup was incomplete',
        },
      ),
    AuthoringRevision3ReviewedDataAssetBuildOutcome.publicationUncertain => (
      'publication_uncertain',
      'publication_uncertain',
      <String, Object?>{
        'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN',
        'message': 'publication may have completed; do not retry automatically',
      },
    ),
  };
  return AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': wireOutcome,
      'basis_head_json': fixture.stagedHead.canonicalJson,
      'project_id': _projectId,
      'project_revision': 5,
      'target_path': _targetPath,
      'pack_name': packName,
      'output': output,
      'files': <Object?>[
        for (final extension in <String>['pak', 'ucas', 'utoc'])
          <String, Object?>{
            'relative_name': '$packName.$extension',
            'byte_len': 123,
            'sha256': _sha,
          },
      ],
      'receipt': <String, Object?>{
        'format':
            'gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1',
        'relative_name': 'gore-authoring-dataasset-build.json',
        'byte_len': 456,
        'sha256': _sha,
      },
      'build_authority': 'reviewed_fixed_leaf_single_package_triplet',
      'artifact_publication_status': publicationStatus,
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
      'retry_safe': false,
      'warning': warning,
    },
    expectedHead: fixture.stagedHead,
    expectedProjectJson: fixture.stagedProjectJson,
    expectedTargetPath: _targetPath,
    expectedPackName: packName,
    expectedOutput: output,
  );
}
