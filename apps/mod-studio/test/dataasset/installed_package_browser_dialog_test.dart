import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/dataasset/ui/installed_package_browser_dialog.dart';
import 'package:gore_mod/project/current_project_controller.dart';

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
}

class _BrowserHost extends StatelessWidget {
  const _BrowserHost({required this.load});

  final Revision3InstalledPackageIndexLoader load;

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Builder(
        builder: (context) => TextButton(
          key: const Key('open-browser'),
          onPressed: () => showDialog<void>(
            context: context,
            builder: (context) =>
                InstalledPackageBrowserDialog(gameRoot: _gameRoot, load: load),
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
