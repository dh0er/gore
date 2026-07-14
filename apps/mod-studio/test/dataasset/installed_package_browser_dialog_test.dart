import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/installed_package_browser_dialog.dart';
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
      expect(find.textContaining('read-only evidence'), findsOneWidget);
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
}

class _BrowserHost extends StatelessWidget {
  const _BrowserHost({required this.load, this.inspect, this.textScale});

  final Revision3InstalledPackageIndexLoader load;
  final Revision3InstalledDataAssetInspector? inspect;
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
}) {
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _seal(4096, 'a' * 64),
    }),
  );
  final candidates = <Object?>[
    <String, Object?>{
      'target_path': '/Game/Characters/DA_Asghan',
      'package_id_hex': '0123456789abcdef',
    },
    if (!partial)
      <String, Object?>{
        'target_path': '/Game/Characters/DA_Viper',
        'package_id_hex': 'fedcba9876543210',
      },
  ];
  final packageIndexJson = jsonEncode(<String, Object?>{
    'status': partial ? 'partial_index' : 'complete_index',
    'physical_chunk_count': partial ? 3 : 2,
    'winning_export_bundle_count': partial ? 2 : 2,
    'directory_indexed_export_bundle_count': partial ? 1 : 2,
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
}) => AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
  <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': candidate.ordinal,
    'head_json': expectedSnapshot.head.canonicalJson,
    'inspection': validDataAssetInspectionResponse(),
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
  },
  expectedSnapshot: expectedSnapshot,
  requestedOrdinal: candidate.ordinal,
);
