import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/installed_package_browser_dialog.dart';
import 'package:gore_mod/dataasset/ui/installed_dataasset_semantic_edit_dialog.dart';
import 'package:gore_mod/dataasset/ui/dataasset_semantic_edit_panel.dart';
import 'package:gore_mod/project/current_project_controller.dart';

import 'dataasset_test_fixtures.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'loads only when opened and keeps candidate browsing search-first',
    (tester) async {
      var calls = 0;
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async {
            calls += 1;
            expect(gameRoot, _gameRoot);
            return _packageIndexResult();
          },
        ),
      );

      expect(calls, 0);
      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(
        find.text('2 installed package candidates indexed'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('installed-package-browser-search-prompt')),
        findsOneWidget,
      );
      expect(find.text('DA_Asghan'), findsNothing);

      await tester.enterText(
        find.byKey(const Key('installed-package-browser-search')),
        'asghan',
      );
      await tester.pump(const Duration(milliseconds: 179));
      expect(find.text('DA_Asghan'), findsNothing);
      await tester.pump(const Duration(milliseconds: 1));

      expect(find.text('DA_Asghan'), findsOneWidget);
      expect(find.text('/Game/Characters/DA_Asghan'), findsOneWidget);
      expect(find.text('DA_Viper'), findsNothing);
      expect(find.text('1 match'), findsOneWidget);
    },
  );

  testWidgets('accepts an exact initial query from the unified browser', (
    tester,
  ) async {
    await tester.pumpWidget(
      _BrowserHost(
        initialQuery: '/Game/Characters/DA_Viper',
        load: ({required gameRoot}) async => _packageIndexResult(),
      ),
    );

    await tester.tap(find.byKey(const Key('open-browser')));
    await tester.pumpAndSettle();

    final search = tester.widget<TextField>(
      find.byKey(const Key('installed-package-browser-search')),
    );
    expect(search.controller?.text, '/Game/Characters/DA_Viper');
    expect(find.text('DA_Viper'), findsOneWidget);
    expect(find.text('DA_Asghan'), findsNothing);
    expect(find.text('1 match'), findsOneWidget);
  });

  testWidgets(
    'exact initial target bypasses substring siblings and opens its inspector',
    (tester) async {
      const targetPath = '/Game/Characters/DA_Viper';
      final snapshot = _packageIndexResult(
        targetPaths: const <String>[
          targetPath,
          '/Game/Characters/DA_ViperVariant',
        ],
      );
      AuthoringRevision3DataAssetPackageCandidate? inspected;
      var inspections = 0;
      await tester.pumpWidget(
        _BrowserHost(
          initialQuery: targetPath,
          initialTargetPath: targetPath,
          load: ({required gameRoot}) async => snapshot,
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async {
                inspections += 1;
                inspected = candidate;
                return _installedInspectionResult(
                  expectedSnapshot: expectedSnapshot,
                  candidate: candidate,
                );
              },
        ),
      );

      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();

      expect(inspections, 1);
      expect(inspected?.targetPath, targetPath);
      expect(inspected?.ordinal, 0);
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('installed-dataasset-inspection-close')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('installed-package-browser-refresh')),
      );
      await tester.pumpAndSettle();
      expect(inspections, 1, reason: 'the exact target intent is one-shot');
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'missing exact initial target never selects a substring or case sibling',
    (tester) async {
      const targetPath = '/Game/Characters/DA_Viper';
      var inspections = 0;
      await tester.pumpWidget(
        _BrowserHost(
          initialQuery: targetPath,
          initialTargetPath: targetPath,
          load: ({required gameRoot}) async => _packageIndexResult(
            targetPaths: const <String>[
              '/Game/Characters/DA_ViperVariant',
              '/Game/Characters/da_viper',
            ],
          ),
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async {
                inspections += 1;
                return _installedInspectionResult(
                  expectedSnapshot: expectedSnapshot,
                  candidate: candidate,
                );
              },
        ),
      );

      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();

      expect(inspections, 0);
      expect(
        find.byKey(const Key('installed-package-initial-target-error')),
        findsOneWidget,
      );
      expect(find.textContaining('fresh installed snapshot'), findsOneWidget);
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('shows partial evidence and validates the manual path fallback', (
    tester,
  ) async {
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => _packageIndexResult(partial: true),
      ),
    );
    await tester.tap(find.byKey(const Key('open-browser')));
    await tester.pumpAndSettle();

    expect(find.text('1 candidate indexed — partial result'), findsOneWidget);
    expect(
      find.textContaining('Search results are useful for discovery'),
      findsOneWidget,
    );

    await tester.drag(find.byType(CustomScrollView), const Offset(0, -180));
    await tester.pumpAndSettle();
    final manualTile = find.byKey(
      const Key('installed-package-browser-manual'),
    );
    await tester.tap(manualTile);
    await tester.pumpAndSettle();
    final input = find.byKey(
      const Key('installed-package-browser-manual-input'),
    );
    final copy = find.byKey(const Key('installed-package-browser-manual-copy'));

    await tester.enterText(input, '/Game/CON');
    await tester.pump();
    expect(tester.widget<IconButton>(copy).onPressed, isNull);
    expect(find.textContaining('Use a canonical /Game path'), findsOneWidget);

    await tester.enterText(input, '/Game/Characters/DA_CustomGuard');
    await tester.pump();
    expect(tester.widget<IconButton>(copy).onPressed, isNotNull);
  });

  testWidgets('refresh obtains a new exact snapshot', (tester) async {
    var calls = 0;
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async {
          calls += 1;
          return _packageIndexResult(partial: calls == 1);
        },
      ),
    );
    await tester.tap(find.byKey(const Key('open-browser')));
    await tester.pumpAndSettle();
    expect(calls, 1);
    expect(find.textContaining('partial result'), findsOneWidget);

    await tester.tap(
      find.byKey(const Key('installed-package-browser-refresh')),
    );
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('2 installed package candidates indexed'), findsOneWidget);
  });

  testWidgets(
    'compact scaled browser scrolls its header to inspection actions',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(800, 600);
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        _BrowserHost(
          textScale: 2,
          load: ({required gameRoot}) async => _packageIndexResult(),
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async => _installedInspectionResult(
                expectedSnapshot: expectedSnapshot,
                candidate: candidate,
              ),
        ),
      );

      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.enterText(
        find.byKey(const Key('installed-package-browser-search')),
        'asghan',
      );
      await tester.pump(const Duration(milliseconds: 180));

      final inspect = find.byKey(const Key('installed-package-inspect-0'));
      await tester.scrollUntilVisible(
        inspect,
        160,
        scrollable: find
            .descendant(
              of: find.byKey(const Key('installed-package-browser-result')),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      expect(inspect, findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(inspect);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('closes instead of retrying a stale managed checkpoint', (
    tester,
  ) async {
    var calls = 0;
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async {
          calls += 1;
          throw const Revision3DataAssetPackageIndexStaleCheckpointException();
        },
      ),
    );
    await tester.tap(find.byKey(const Key('open-browser')));
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(
      find.textContaining('project changed while this browser was open'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('installed-package-browser-retry')),
      findsNothing,
    );
    await tester.tap(
      find.byKey(const Key('installed-package-browser-close-stale')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsNothing,
    );
    expect(calls, 1);
  });

  for (final scenario in <({Object error, String label, String message})>[
    (
      error:
          const Revision3InstalledDataAssetInspectionStaleCheckpointException(),
      label: 'stale snapshot',
      message: 'project or installed package snapshot changed',
    ),
    (
      error:
          const Revision3InstalledDataAssetInspectionRequiresReopenException(),
      label: 'requires reopen',
      message: 'managed project must be reopened',
    ),
  ]) {
    testWidgets(
      'inspection ${scenario.label} closes the browser and removes old actions',
      (tester) async {
        final snapshot = _packageIndexResult();
        await tester.pumpWidget(
          _BrowserHost(
            load: ({required gameRoot}) async => snapshot,
            inspect:
                ({
                  required gameRoot,
                  required expectedSnapshot,
                  required candidate,
                }) async => throw scenario.error,
          ),
        );
        await tester.tap(find.byKey(const Key('open-browser')));
        await tester.pumpAndSettle();
        await tester.enterText(
          find.byKey(const Key('installed-package-browser-search')),
          'asghan',
        );
        await tester.pump(const Duration(milliseconds: 180));
        await tester.tap(find.byKey(const Key('installed-package-inspect-0')));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 300));

        expect(
          find.byKey(const Key('installed-package-browser-dialog')),
          findsNothing,
        );
        expect(
          find.byKey(const Key('installed-package-browser-refresh')),
          findsNothing,
        );
        expect(
          find.byKey(const Key('installed-package-inspect-0')),
          findsNothing,
        );
        expect(find.textContaining(scenario.message), findsOneWidget);
      },
    );
  }

  testWidgets(
    'filtered inspection keeps the original ordinal, candidate, and snapshot',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final snapshot = _packageIndexResult();
      AuthoringRevision3DataAssetPackageIndexResult? receivedSnapshot;
      AuthoringRevision3DataAssetPackageCandidate? receivedCandidate;
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async => snapshot,
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async {
                receivedSnapshot = expectedSnapshot;
                receivedCandidate = candidate;
                return _installedInspectionResult(
                  expectedSnapshot: expectedSnapshot,
                  candidate: candidate,
                );
              },
        ),
      );
      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('installed-package-browser-search')),
        'viper',
      );
      await tester.pump(const Duration(milliseconds: 180));

      expect(find.text('DA_Viper'), findsOneWidget);
      await tester.tap(find.byKey(const Key('installed-package-inspect-1')));
      await tester.pumpAndSettle();

      expect(identical(receivedSnapshot, snapshot), isTrue);
      expect(receivedCandidate?.ordinal, 1);
      expect(
        identical(receivedCandidate, snapshot.index.candidates[1]),
        isTrue,
      );
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('exact evidence'), findsOneWidget);
    },
  );

  testWidgets(
    'refresh suppresses a late inspection result from the old snapshot',
    (tester) async {
      final first = _packageIndexResult();
      final second = _packageIndexResult();
      final completion =
          Completer<AuthoringRevision3InstalledDataAssetInspectionResult>();
      var loads = 0;
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async {
            loads += 1;
            return loads == 1 ? first : second;
          },
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) {
                expect(identical(expectedSnapshot, first), isTrue);
                return completion.future;
              },
        ),
      );
      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('installed-package-browser-search')),
        'asghan',
      );
      await tester.pump(const Duration(milliseconds: 180));
      await tester.tap(find.byKey(const Key('installed-package-inspect-0')));
      await tester.pump();

      await tester.tap(
        find.byKey(const Key('installed-package-browser-refresh')),
      );
      await tester.pumpAndSettle();
      expect(loads, 2);

      completion.complete(
        _installedInspectionResult(
          expectedSnapshot: first,
          candidate: first.index.candidates.first,
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('installed-package-inspection-error')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'reviewed preset is curated before search and keeps its expert leaf',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1400, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final snapshot = _packageIndexResult(reviewedWolf: true);
      final candidate = snapshot.index.candidates.single;
      final exactInspection = _installedInspectionResult(
        expectedSnapshot: snapshot,
        candidate: candidate,
        inspection: _reviewedWolfInspectionResponse(),
      );
      expect(
        ReviewedFootstepPresetInspection.tryMatch(
          packagePath: exactInspection.targetPath,
          inspection: exactInspection.inspection,
        ),
        isNotNull,
      );
      ReviewedInstalledDataAssetEditIntent? publishedIntent;
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async => snapshot,
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async => exactInspection,
          publish: (intent) async => DataAssetSemanticStagePublication(
            targetPath: intent.expectedTargetPath,
            revision: 8,
          ),
          publishReviewed: (intent) async {
            publishedIntent = intent;
            return DataAssetSemanticStagePublication(
              targetPath: intent.expectedTargetPath,
              revision: 9,
            );
          },
        ),
      );

      await tester.tap(find.byKey(const Key('open-browser')));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('installed-package-browser-reviewed-presets')),
        findsOneWidget,
      );
      expect(find.text('Wolf footsteps'), findsOneWidget);
      expect(
        find.byKey(const Key('installed-package-browser-search-prompt')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('installed-package-reviewed-open-0')),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('reviewed-footstep-preset-entry')),
        findsOneWidget,
      );
      expect(find.textContaining('expert path'), findsOneWidget);

      await tester.tap(find.byKey(const Key('dataasset-export-tile-0')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('dataasset-edit-leaf-0-1')), findsOneWidget);

      final guidedEdit = find.byKey(const Key('reviewed-footstep-preset-edit'));
      await tester.ensureVisible(guidedEdit);
      await tester.pumpAndSettle();
      await tester.tap(guidedEdit);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('reviewed-footstep-preset-dialog')),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('reviewed-footstep-x')),
        '11',
      );
      await tester.enterText(
        find.byKey(const Key('reviewed-footstep-y')),
        '12',
      );
      await tester.tap(find.byKey(const Key('reviewed-footstep-preview')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('reviewed-footstep-stage')));
      await tester.pumpAndSettle();

      final intent = publishedIntent!;
      expect(identical(intent.snapshot, snapshot), isTrue);
      expect(intent.request.toJson(), <String, Object>{
        'format': 1,
        'schema_id': 'g1r.tracking.footstep-preset',
        'schema_revision': 1,
        'field_id': 'feet_texture_size',
        'value': <String, String>{'x': '11', 'y': '12'},
      });
      expect(intent.request.toJson(), isNot(contains('target_path')));
      expect(intent.request.toJson(), isNot(contains('selector')));
      expect(intent.request.toJson(), isNot(contains('replacement_hex')));
      expect(intent.toNativeFields().keys, <String>[
        'candidate_ordinal',
        'expected_package_index_seal',
        'expected_source_snapshot_seal',
        'reviewed_edit',
      ]);
      expect(
        find.byKey(const Key('installed-package-browser-dialog')),
        findsNothing,
      );
      expect(find.textContaining('project revision 9'), findsOneWidget);
    },
  );

  testWidgets('stages only a typed leaf from the exact installed inspection', (
    tester,
  ) async {
    final snapshot = _packageIndexResult();
    final candidate = snapshot.index.candidates.first;
    late final AuthoringRevision3InstalledDataAssetInspectionResult evidence;
    DataAssetInstalledSemanticEditIntent? publishedIntent;
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => snapshot,
        inspect:
            ({
              required gameRoot,
              required expectedSnapshot,
              required candidate,
            }) async {
              evidence = _installedInspectionResult(
                expectedSnapshot: expectedSnapshot,
                candidate: candidate,
              );
              return evidence;
            },
        publish: (intent) async {
          publishedIntent = intent;
          return DataAssetSemanticStagePublication(
            targetPath: intent.expectedTargetPath,
            revision: 8,
          );
        },
      ),
    );
    await _openFirstInstalledEdit(tester);
    expect(
      find.byKey(const Key('installed-dataasset-semantic-edit-dialog')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('installed-dataasset-semantic-stage-action')),
          )
          .onPressed,
      isNull,
    );
    await tester.enterText(
      find.byKey(const Key('dataasset-semantic-value')),
      '2',
    );
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-preview-action')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('installed-dataasset-semantic-preview')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-stage-action')),
    );
    await tester.pumpAndSettle();

    final intent = publishedIntent!;
    expect(identical(intent.snapshot, snapshot), isTrue);
    expect(identical(intent.candidate, candidate), isTrue);
    expect(identical(intent.inspection, evidence), isTrue);
    expect(intent.toNativeFields(), isNot(contains('target_path')));
    expect(intent.toNativeFields(), isNot(contains('package_id_hex')));
    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsNothing,
    );
    expect(find.textContaining('project revision 8'), findsOneWidget);
  });

  testWidgets('announces invalid installed edits as a live status', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final snapshot = _packageIndexResult();
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => snapshot,
        inspect:
            ({
              required gameRoot,
              required expectedSnapshot,
              required candidate,
            }) async => _installedInspectionResult(
              expectedSnapshot: expectedSnapshot,
              candidate: candidate,
            ),
        publish: (_) async => throw StateError('unexpected publication'),
      ),
    );
    await _openFirstInstalledEdit(tester);
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-preview-action')),
    );
    await tester.pump();

    expect(
      tester.getSemantics(
        find.byKey(const Key('installed-dataasset-semantic-error-status')),
      ),
      matchesSemantics(
        label: 'Choose a new value; the current value would not change.',
        isLiveRegion: true,
      ),
    );
    semantics.dispose();
  });

  testWidgets(
    'closes all installed evidence dialogs when publication becomes stale',
    (tester) async {
      final snapshot = _packageIndexResult();
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async => snapshot,
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async => _installedInspectionResult(
                expectedSnapshot: expectedSnapshot,
                candidate: candidate,
              ),
          publish: (_) async {
            throw const DataAssetSemanticStageUnavailableException.staleCheckpoint();
          },
        ),
      );
      await _openFirstInstalledEdit(tester);
      await tester.enterText(
        find.byKey(const Key('dataasset-semantic-value')),
        '2',
      );
      await tester.tap(
        find.byKey(const Key('installed-dataasset-semantic-preview-action')),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('installed-dataasset-semantic-stage-action')),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('installed-dataasset-semantic-edit-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('installed-package-browser-dialog')),
        findsNothing,
      );
      expect(find.textContaining('project changed'), findsOneWidget);
    },
  );

  testWidgets(
    'closes all evidence when the publication outcome cannot be confirmed',
    (tester) async {
      final snapshot = _packageIndexResult();
      await tester.pumpWidget(
        _BrowserHost(
          load: ({required gameRoot}) async => snapshot,
          inspect:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) async => _installedInspectionResult(
                expectedSnapshot: expectedSnapshot,
                candidate: candidate,
              ),
          publish: (_) async => throw StateError(
            'simulated failure after an indeterminate publication boundary',
          ),
        ),
      );
      await _openFirstInstalledEdit(tester);
      await _previewInstalledEdit(tester);
      await tester.tap(
        find.byKey(const Key('installed-dataasset-semantic-stage-action')),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('installed-dataasset-semantic-edit-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('installed-dataasset-inspection-dialog')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('installed-package-browser-dialog')),
        findsNothing,
      );
      expect(
        find.textContaining('outcome could not be confirmed'),
        findsOneWidget,
      );
    },
  );

  testWidgets('source drift closes evidence but keeps the project usable', (
    tester,
  ) async {
    final snapshot = _packageIndexResult();
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => snapshot,
        inspect:
            ({
              required gameRoot,
              required expectedSnapshot,
              required candidate,
            }) async => _installedInspectionResult(
              expectedSnapshot: expectedSnapshot,
              candidate: candidate,
            ),
        publish: (_) async {
          throw const DataAssetSemanticStageUnavailableException.sourceEvidenceStale();
        },
      ),
    );
    await _openFirstInstalledEdit(tester);
    await _previewInstalledEdit(tester);
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-stage-action')),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsNothing,
    );
    expect(
      find.textContaining('managed project remains usable'),
      findsOneWidget,
    );
    expect(find.textContaining('outcome could not be confirmed'), findsNothing);
  });

  testWidgets('an already staged target gives an actionable safe rejection', (
    tester,
  ) async {
    final snapshot = _packageIndexResult();
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => snapshot,
        inspect:
            ({
              required gameRoot,
              required expectedSnapshot,
              required candidate,
            }) async => _installedInspectionResult(
              expectedSnapshot: expectedSnapshot,
              candidate: candidate,
            ),
        publish: (_) async {
          throw const DataAssetSemanticStageUnavailableException.targetAlreadyStaged();
        },
      ),
    );
    await _openFirstInstalledEdit(tester);
    await _previewInstalledEdit(tester);
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-stage-action')),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsNothing,
    );
    expect(find.textContaining('already has a staged edit'), findsOneWidget);
    expect(find.textContaining('Inspect it again'), findsNothing);
    expect(find.textContaining('outcome could not be confirmed'), findsNothing);
  });

  testWidgets('cannot dismiss an installed edit while publication is active', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final snapshot = _packageIndexResult();
    final completion = Completer<DataAssetSemanticStagePublication>();
    var publications = 0;
    await tester.pumpWidget(
      _BrowserHost(
        load: ({required gameRoot}) async => snapshot,
        inspect:
            ({
              required gameRoot,
              required expectedSnapshot,
              required candidate,
            }) async => _installedInspectionResult(
              expectedSnapshot: expectedSnapshot,
              candidate: candidate,
            ),
        publish: (intent) {
          publications++;
          return completion.future;
        },
      ),
    );
    await _openFirstInstalledEdit(tester);
    await _previewInstalledEdit(tester);
    await tester.tap(
      find.byKey(const Key('installed-dataasset-semantic-stage-action')),
    );
    await tester.pump();
    expect(publications, 1);
    expect(
      find.byKey(const Key('installed-dataasset-semantic-progress')),
      findsOneWidget,
    );
    expect(
      tester.getSemantics(
        find.byKey(const Key('installed-dataasset-semantic-busy-status')),
      ),
      matchesSemantics(
        label:
            'Re-reading the exact package and preparing the managed candidate',
        isLiveRegion: true,
      ),
    );
    semantics.dispose();

    await tester.tapAt(const Offset(4, 4));
    await tester.pump();
    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('installed-dataasset-semantic-edit-dialog')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsOneWidget,
    );

    completion.complete(
      const DataAssetSemanticStagePublication(
        targetPath: '/Game/Characters/DA_Asghan',
        revision: 8,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('installed-package-browser-dialog')),
      findsNothing,
    );
    expect(find.textContaining('project revision 8'), findsOneWidget);
  });
}

Future<void> _openFirstInstalledEdit(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('open-browser')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const Key('installed-package-browser-search')),
    'asghan',
  );
  await tester.pump(const Duration(milliseconds: 180));
  await tester.tap(find.byKey(const Key('installed-package-inspect-0')));
  await tester.pumpAndSettle();
  await tester.drag(
    find.byKey(const Key('dataasset-inspection-report')),
    const Offset(0, -360),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('dataasset-export-tile-0')));
  await tester.pumpAndSettle();

  final editLeaf = find.byKey(const Key('dataasset-edit-leaf-0-0'));
  await tester.ensureVisible(editLeaf);
  await tester.pumpAndSettle();
  await tester.tap(editLeaf);
  await tester.pumpAndSettle();
  expect(
    find.byKey(const Key('installed-dataasset-semantic-edit-dialog')),
    findsOneWidget,
  );
}

Future<void> _previewInstalledEdit(WidgetTester tester) async {
  await tester.enterText(
    find.byKey(const Key('dataasset-semantic-value')),
    '2',
  );
  await tester.tap(
    find.byKey(const Key('installed-dataasset-semantic-preview-action')),
  );
  await tester.pump();
  expect(
    find.byKey(const Key('installed-dataasset-semantic-preview')),
    findsOneWidget,
  );
}

class _BrowserHost extends StatelessWidget {
  const _BrowserHost({
    required this.load,
    this.inspect,
    this.publish,
    this.publishReviewed,
    this.initialQuery = '',
    this.initialTargetPath,
    this.textScale,
  });

  final Revision3InstalledPackageIndexLoader load;
  final Revision3InstalledDataAssetInspector? inspect;
  final InstalledDataAssetSemanticStagePublisher? publish;
  final ReviewedInstalledDataAssetStagePublisher? publishReviewed;
  final String initialQuery;
  final String? initialTargetPath;
  final double? textScale;

  @override
  Widget build(BuildContext context) => MaterialApp(
    builder: textScale == null
        ? null
        : (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(textScale!)),
            child: child!,
          ),
    home: Scaffold(
      body: Builder(
        builder: (context) => TextButton(
          key: const Key('open-browser'),
          onPressed: () => showDialog<void>(
            context: context,
            builder: (context) => InstalledPackageBrowserDialog(
              gameRoot: _gameRoot,
              load: load,
              inspect: inspect,
              publish: publish,
              publishReviewed: publishReviewed,
              initialQuery: initialQuery,
              initialTargetPath: initialTargetPath,
            ),
          ),
          child: const Text('Open'),
        ),
      ),
    ),
  );
}

AuthoringRevision3DataAssetPackageIndexResult _packageIndexResult({
  bool partial = false,
  bool reviewedWolf = false,
  List<String>? targetPaths,
}) {
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _seal(4096, 'a' * 64),
    }),
  );
  final candidates = <Object?>[
    if (reviewedWolf)
      <String, Object?>{
        'target_path':
            '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
        'package_id_hex': '01e173a19ea374c9',
      },
    if (targetPaths != null)
      for (final (index, targetPath) in targetPaths.indexed)
        <String, Object?>{
          'target_path': targetPath,
          'package_id_hex': (index + 1).toRadixString(16).padLeft(16, '0'),
        }
    else if (!reviewedWolf) ...[
      <String, Object?>{
        'target_path': '/Game/Characters/DA_Asghan',
        'package_id_hex': '0123456789abcdef',
      },
      if (!partial)
        <String, Object?>{
          'target_path': '/Game/Characters/DA_Viper',
          'package_id_hex': 'fedcba9876543210',
        },
    ],
  ];
  final packageIndexJson = jsonEncode(<String, Object?>{
    'status': partial ? 'partial_index' : 'complete_index',
    'physical_chunk_count': partial ? 3 : candidates.length,
    'winning_export_bundle_count': partial ? 2 : candidates.length,
    'directory_indexed_export_bundle_count': partial ? 1 : candidates.length,
    'out_of_scope_export_bundle_count': 0,
    'candidates': candidates,
    'partial_reasons': partial
        ? <Object?>[
            <String, Object?>{
              'reason': 'missing_directory_index_path',
              'count': 1,
            },
          ]
        : <Object?>[],
  });
  final packageIndexBytes = utf8.encode(packageIndexJson);
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': candidates.length,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': _seal(80, 'b' * 64),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': packageIndexJson,
      'package_index_seal': _seal(
        packageIndexBytes.length,
        crypto.sha256.convert(packageIndexBytes).toString(),
      ),
      'package_index_status': partial ? 'partial_index' : 'complete_index',
      'project_id': '31313131313131313131313131313131',
      'project_revision': 7,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': _seal(120, 'c' * 64),
      'target_executable_seal': _seal(171698176, 'd' * 64),
    },
    expectedHead: head,
  );
}

Map<String, Object?> _seal(int byteLength, String sha256) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
};

AuthoringRevision3InstalledDataAssetInspectionResult
_installedInspectionResult({
  required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
  required AuthoringRevision3DataAssetPackageCandidate candidate,
  Map<String, Object?>? inspection,
}) => AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
  <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': candidate.ordinal,
    'head_json': expectedSnapshot.head.canonicalJson,
    'inspection': inspection ?? validDataAssetInspectionResponse(),
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'inspection_only',
    'package_id_hex': candidate.packageIdHex,
    'package_index_seal': _seal(
      expectedSnapshot.packageIndexSeal.byteLength,
      expectedSnapshot.packageIndexSeal.sha256,
    ),
    'project_id': expectedSnapshot.projectId,
    'project_revision': expectedSnapshot.projectRevision,
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'selected_installed_dataasset_fixed_leaf_inspection_only',
    'source_snapshot_seal': _seal(
      expectedSnapshot.sourceSnapshotSeal.byteLength,
      expectedSnapshot.sourceSnapshotSeal.sha256,
    ),
    'target_path': candidate.targetPath,
    'usmap_content_seal': _seal(256, 'c' * 64),
    'usmap_inventory_seal': _seal(96, 'e' * 64),
  },
  expectedSnapshot: expectedSnapshot,
  requestedOrdinal: candidate.ordinal,
);

Map<String, Object?> _reviewedWolfInspectionResponse() => <String, Object?>{
  'ok': true,
  'format': 'gore.dataasset.fixed-inspect.v1',
  'status': 'walked',
  'summary': <String, Object?>{
    'package_exports': 1,
    'reported_exports': 1,
    'walked_exports': 1,
    'editable_leaves': 2,
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
    'usmap_length': 256,
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
          'selector': _reviewedWolfSelector(),
        },
        <String, Object?>{
          'index': 1,
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
            'kind': 'bool',
            'path': <Object?>[
              <String, Object?>{
                'step': 'property',
                'schema_index': 1,
                'property_name': 'InvertX',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': 'FootstepTag',
                'declaring_module_path': '/Script/G1R',
                'property_type': <String, Object?>{'type': 'bool'},
              },
            ],
            'expected_hex': '00',
          },
        },
      ],
    },
  ],
};

Map<String, Object?> _reviewedWolfSelector() => <String, Object?>{
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
      'property_type': <String, Object?>{'type': 'struct', 'name': 'Vector4'},
    },
  ],
  'expected_hex':
      '000000000000244000000000000024400000000000000000000000000000f03f',
};
