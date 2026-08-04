import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/installed_package_browser_dialog.dart';
import 'package:gore_mod/project/revision3_installed_content_browser.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';

void main() {
  testWidgets('missing source shows setup action without loading', (
    tester,
  ) async {
    var loadCalls = 0;
    var settingsCalls = 0;
    Future<AuthoringRevision3DataAssetPackageIndexResult> loader({
      required String gameRoot,
    }) async {
      loadCalls++;
      return _packageIndexResult(paths: const ['/Game/NPC/DA_Never']);
    }

    await tester.pumpWidget(
      _host(
        gameRoot: null,
        loader: loader,
        openSettings: () => settingsCalls++,
      ),
    );

    expect(loadCalls, 0);
    expect(
      find.byKey(const Key('revision3-installed-content-browser-setup')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-installed-content-browser-setup-action')),
    );
    expect(settingsCalls, 1);
    expect(loadCalls, 0);

    await tester.pumpWidget(
      _host(
        sourceIdentity: null,
        loader: loader,
        openSettings: () => settingsCalls++,
      ),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-installed-content-browser-setup')),
      findsOneWidget,
    );
    expect(loadCalls, 0);
  });

  testWidgets('partial snapshot searches metadata and opens exact path', (
    tester,
  ) async {
    String? openedPath;
    var loadCalls = 0;
    await tester.pumpWidget(
      _host(
        loader: ({required gameRoot}) async {
          loadCalls++;
          expect(gameRoot, _gameRoot);
          return _packageIndexResult(
            paths: const ['/Game/Characters/DA_Asghan'],
            partial: true,
          );
        },
        openInspector: (path) => openedPath = path,
      ),
    );
    await tester.pumpAndSettle();

    expect(loadCalls, 1);
    expect(find.text('Partial exact snapshot: 1'), findsOneWidget);
    expect(
      find.byKey(
        const Key('revision3-installed-content-browser-search-prompt'),
      ),
      findsOneWidget,
    );
    expect(find.text('DA_Asghan'), findsNothing);

    await tester.enterText(
      find.byKey(const Key('revision3-installed-content-browser-search')),
      'ASGHAN',
    );
    await tester.pump();

    expect(find.text('DA_Asghan'), findsOneWidget);
    expect(find.text('/Game/Characters/DA_Asghan'), findsOneWidget);
    expect(find.text('DataAsset'), findsOneWidget);
    expect(find.text('Installed'), findsOneWidget);
    expect(find.text('Metadata only'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-installed-content-browser-result-limit')),
      findsNothing,
    );

    await tester.tap(
      find.byKey(const Key('revision3-installed-content-browser-open-0')),
    );
    expect(openedPath, '/Game/Characters/DA_Asghan');
  });

  testWidgets('result-limit notice appears only when matches are truncated', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        loader: ({required gameRoot}) async => _packageIndexResult(
          paths: List<String>.generate(
            101,
            (index) => '/Game/Generated/DA_Match_$index',
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('revision3-installed-content-browser-search')),
      'match',
    );
    await tester.pump();

    expect(
      find.byKey(const Key('revision3-installed-content-browser-result-limit')),
      findsOneWidget,
    );
  });

  testWidgets('source change suppresses a late result from the old source', (
    tester,
  ) async {
    final first = Completer<AuthoringRevision3DataAssetPackageIndexResult>();
    final second = Completer<AuthoringRevision3DataAssetPackageIndexResult>();
    var calls = 0;
    final key = GlobalKey<_ChangingSourceHostState>();
    await tester.pumpWidget(
      _ChangingSourceHost(
        key: key,
        loader: ({required gameRoot}) {
          calls++;
          return calls == 1 ? first.future : second.future;
        },
      ),
    );

    expect(calls, 1);
    key.currentState!.changeSource();
    await tester.pump();
    expect(calls, 2);

    second.complete(
      _packageIndexResult(paths: const ['/Game/NPC/DA_NewSource']),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-installed-content-browser-search')),
      'newsource',
    );
    await tester.pump();
    expect(find.text('DA_NewSource'), findsOneWidget);

    first.complete(
      _packageIndexResult(paths: const ['/Game/NPC/DA_OldSource']),
    );
    await tester.pumpAndSettle();
    expect(find.text('DA_NewSource'), findsOneWidget);
    expect(find.text('DA_OldSource'), findsNothing);
  });

  testWidgets('equivalent parent rebuild does not reload the exact source', (
    tester,
  ) async {
    var calls = 0;
    final key = GlobalKey<_RebuildingLoaderHostState>();
    await tester.pumpWidget(
      _RebuildingLoaderHost(
        key: key,
        loader: ({required gameRoot}) async {
          calls++;
          return _packageIndexResult(
            paths: const ['/Game/NPC/DA_StableSource'],
          );
        },
      ),
    );
    await tester.pumpAndSettle();
    expect(calls, 1);

    key.currentState!.rebuildWithEquivalentLoaderClosure();
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(find.text('Complete exact snapshot: 1'), findsOneWidget);
  });

  testWidgets('load failure is honest and retry obtains a fresh snapshot', (
    tester,
  ) async {
    var calls = 0;
    await tester.pumpWidget(
      _host(
        loader: ({required gameRoot}) async {
          calls++;
          if (calls == 1) throw StateError('fixture failure');
          return _packageIndexResult(paths: const ['/Game/NPC/DA_Retry']);
        },
      ),
    );
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(
      find.byKey(const Key('revision3-installed-content-browser-error')),
      findsOneWidget,
    );
    expect(
      find.text(
        'Nothing was changed. Check the selected installation and retry.',
      ),
      findsOneWidget,
    );

    await tester.tap(
      find.byKey(const Key('revision3-installed-content-browser-retry')),
    );
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('Complete exact snapshot: 1'), findsOneWidget);
  });

  testWidgets('result remains scrollable without overflow at 280 by 300', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(280, 300);
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(
        loader: ({required gameRoot}) async =>
            _packageIndexResult(paths: const ['/Game/Characters/DA_Compact']),
        openInspector: (_) {},
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    final search = find.byKey(
      const Key('revision3-installed-content-browser-search'),
    );
    await tester.scrollUntilVisible(
      search,
      120,
      scrollable: find
          .descendant(
            of: find.byKey(
              const Key('revision3-installed-content-browser-result'),
            ),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    await tester.enterText(search, 'compact');
    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-installed-content-browser-result')),
      findsOneWidget,
    );
  });
}

Widget _host({
  String? gameRoot = _gameRoot,
  Object? sourceIdentity = 'source-a',
  required Revision3InstalledPackageIndexLoader loader,
  VoidCallback? openSettings,
  ValueChanged<String>? openInspector,
}) => MaterialApp(
  home: Scaffold(
    body: Revision3InstalledContentBrowser(
      gameRoot: gameRoot,
      sourceIdentity: sourceIdentity,
      loader: loader,
      copy: _copy,
      openSettings: openSettings,
      openInspector: openInspector,
    ),
  ),
);

final class _ChangingSourceHost extends StatefulWidget {
  const _ChangingSourceHost({required this.loader, super.key});

  final Revision3InstalledPackageIndexLoader loader;

  @override
  State<_ChangingSourceHost> createState() => _ChangingSourceHostState();
}

final class _ChangingSourceHostState extends State<_ChangingSourceHost> {
  String _root = r'C:\Games\Source A';
  String _identity = 'source-a';

  void changeSource() => setState(() {
    _root = r'C:\Games\Source B';
    _identity = 'source-b';
  });

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3InstalledContentBrowser(
        gameRoot: _root,
        sourceIdentity: _identity,
        loader: widget.loader,
        copy: _copy,
      ),
    ),
  );
}

final class _RebuildingLoaderHost extends StatefulWidget {
  const _RebuildingLoaderHost({required this.loader, super.key});

  final Revision3InstalledPackageIndexLoader loader;

  @override
  State<_RebuildingLoaderHost> createState() => _RebuildingLoaderHostState();
}

final class _RebuildingLoaderHostState extends State<_RebuildingLoaderHost> {
  void rebuildWithEquivalentLoaderClosure() => setState(() {});

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3InstalledContentBrowser(
        gameRoot: _gameRoot,
        sourceIdentity: 'stable-source',
        loader: ({required gameRoot}) => widget.loader(gameRoot: gameRoot),
        copy: _copy,
      ),
    ),
  );
}

String _completeSummary(int count) => 'Complete exact snapshot: $count';
String _partialSummary(int count) => 'Partial exact snapshot: $count';

const _copy = Revision3InstalledContentBrowserCopy(
  setupTitle: 'Choose a game installation',
  setupDescription: 'Installed metadata needs an exact configured source.',
  setupActionLabel: 'Open settings',
  loadingLabel: 'Reading the exact installed metadata snapshot...',
  completeSummary: _completeSummary,
  partialSummary: _partialSummary,
  completeDescription: 'The installed metadata index is complete.',
  partialDescription: 'The exact metadata result is partial.',
  authorityNotice:
      'Search reads metadata only. It grants no edit, build, deployment, runtime, game, or save authority.',
  refreshTooltip: 'Read a fresh exact snapshot',
  searchLabel: 'Search installed content',
  searchHint: 'Asset name or /Game path',
  searchPrompt: 'Type an asset name or /Game path to search.',
  noMatchesTitle: 'No metadata matches',
  noMatchesDescription: 'Try another asset name or /Game path.',
  resultLimitDescription: 'Showing up to 100 metadata matches.',
  kindBadgeLabel: 'DataAsset',
  sourceBadgeLabel: 'Installed',
  readinessBadgeLabel: 'Metadata only',
  openInspectorLabel: 'Inspect exact',
  errorTitle: 'Installed metadata could not be read',
  errorDescription:
      'Nothing was changed. Check the selected installation and retry.',
  retryLabel: 'Try again',
);

AuthoringRevision3DataAssetPackageIndexResult _packageIndexResult({
  required List<String> paths,
  bool partial = false,
}) {
  final sortedPaths = [...paths]..sort();
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _seal(4096, 'a' * 64),
    }),
  );
  final candidates = <Object?>[
    for (var index = 0; index < sortedPaths.length; index++)
      <String, Object?>{
        'target_path': sortedPaths[index],
        'package_id_hex': index.toRadixString(16).padLeft(16, '0'),
      },
  ];
  final packageIndexJson = jsonEncode(<String, Object?>{
    'status': partial ? 'partial_index' : 'complete_index',
    'physical_chunk_count': candidates.length + (partial ? 1 : 0),
    'winning_export_bundle_count': candidates.length + (partial ? 1 : 0),
    'directory_indexed_export_bundle_count': candidates.length,
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
