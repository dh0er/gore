import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/scripts/domain/script_modules_provider.dart';
import 'package:gore_mod/scripts/ui/script_tab.dart';
import 'package:path/path.dart' as p;

/// Fake vanilla module list: two leaves under one folder, one entry with an
/// empty `file` (exercises the `<name>.as` root-leaf fallback), and one whose
/// name does NOT appear in its path (exercises name-only search matching).
List<ScriptModuleInfo> _fakeModules() => [
      ScriptModuleInfo(name: 'Foo', file: 'Gameplay/Foo.as'),
      ScriptModuleInfo(name: 'Baz', file: 'Gameplay/Baz.as'),
      ScriptModuleInfo(name: 'Bar', file: ''), // fallback → 'Bar.as' at the root
      ScriptModuleInfo(name: 'Quux', file: 'Misc/Other.as'),
    ];

ScriptMod _stagedBar() => const ScriptMod(
    op: ScriptOp.edit, moduleName: 'Bar', relPath: 'Bar.as', asPath: '');

Future<void> _pumpTab(WidgetTester tester, {ScriptMod? staged}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        scriptModulesProvider.overrideWith((ref) async => _fakeModules()),
        if (staged != null)
          scriptModsProvider
              .overrideWith((ref) => ScriptModsNotifier()..setMod(staged)),
      ],
      child: const MaterialApp(home: Scaffold(body: ScriptTab())),
    ),
  );
  // Resolve the modules future (loading spinner → data).
  await tester.pump();
  await tester.pump();
}

void main() {
  testWidgets('tree shows folders + root leaf, staged leaf is check-marked, '
      'expanding a folder reveals its scripts', (tester) async {
    await _pumpTab(tester, staged: _stagedBar());

    // Folders from the fake paths render collapsed at the top level.
    expect(find.text('Gameplay'), findsOneWidget);
    expect(find.text('Misc'), findsOneWidget);
    // Leaves inside a collapsed folder are not built yet.
    expect(find.text('Foo.as'), findsNothing);
    // The empty-file module falls back to '<name>.as' at the tree root.
    expect(find.text('Bar.as'), findsOneWidget);
    // Count caption reflects the full module list.
    expect(find.text('4 modules'), findsOneWidget);

    // The staged module's tree leaf carries the check marker.
    final barTile = find.ancestor(
        of: find.text('Bar.as'), matching: find.byType(ListTile));
    expect(find.descendant(of: barTile, matching: find.byIcon(Icons.check)),
        findsOneWidget);

    // Expanding 'Gameplay' reveals its two scripts.
    await tester.tap(find.text('Gameplay'));
    await tester.pump();
    expect(find.text('Foo.as'), findsOneWidget);
    expect(find.text('Baz.as'), findsOneWidget);
  });

  testWidgets('search shows a flat hit list (name-only match included) and '
      'selecting a vanilla module shows the Edit action', (tester) async {
    await _pumpTab(tester);

    // 'quux' matches the module NAME only (its path is Misc/Other.as).
    await tester.enterText(find.byType(TextField), 'quux');
    await tester.pump();
    expect(find.text('Quux'), findsOneWidget);
    expect(find.text('Misc/Other.as'), findsOneWidget);
    expect(find.text('1 match / 4 total'), findsOneWidget);
    // The tree is only offstage during a search — still mounted, not visible.
    expect(find.text('Gameplay'), findsNothing);
    expect(find.text('Gameplay', skipOffstage: false), findsOneWidget);

    // Selecting the hit shows the vanilla-module detail with an Edit button.
    await tester.tap(find.text('Quux'));
    await tester.pump();
    expect(find.text('Vanilla module — not staged'), findsOneWidget);
    expect(
      find.ancestor(
          of: find.text('Edit'),
          matching: find.byWidgetPredicate((w) => w is FilledButton)),
      findsOneWidget,
    );
    // The detail pane echoes the game-relative path (also in the hit subtitle).
    expect(find.text('Misc/Other.as'), findsNWidgets(2));
  });

  testWidgets('staged panel lists the mod with compile status, tapping selects '
      'it, delete unstages and the detail falls back to vanilla',
      (tester) async {
    await _pumpTab(tester, staged: _stagedBar());

    // Header shows the staged count plus the Add entry point.
    expect(find.text('Staged script mods (1)'), findsOneWidget);
    expect(find.text('Add new .as'), findsOneWidget);

    // Expand the panel: the staged mod row appears (op icon + name + status).
    await tester.tap(find.text('Staged script mods (1)'));
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.edit_note_outlined), findsOneWidget);
    expect(find.text('Bar'), findsOneWidget);
    // An uncompiled mod surfaces a "not compiled" affordance.
    expect(
        find.textContaining('not compiled', findRichText: true), findsWidgets);

    // Tapping the row selects the mod → staged detail pane.
    await tester.tap(find.text('Bar'));
    await tester.pumpAndSettle();
    expect(find.text('Edit existing module'), findsOneWidget);

    // Deleting unstages it; the selection now resolves to the vanilla module.
    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    expect(find.text('Staged script mods (0)'), findsOneWidget);
    expect(find.text('Edit existing module'), findsNothing);
    expect(find.text('Vanilla module — not staged'), findsOneWidget);
  });

  // Fix 3: loadProject treats the script relPath as untrusted (defense-in-depth, matching the
  // asPath guard + gore-as compile-side check) and drops mods whose relPath is empty/absolute/'..'.
  test('loadProject drops script mods with an unsafe relPath', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_relpath_test_');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final asFile = File(p.join(tmp.path, 'New.as'))..writeAsStringSync('void Foo(){}');
    final project = ModProject(
      name: 'M',
      scripts: [
        // Safe sibling — must survive the load.
        ScriptMod(op: ScriptOp.add, moduleName: 'Good', relPath: 'AI/Good.as', asPath: asFile.path),
        // Escapes the staged tree — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Esc', relPath: '../evil.as', asPath: asFile.path),
        // Absolute — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Abs', relPath: '/etc/evil.as', asPath: asFile.path),
        // Empty — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Empty', relPath: '', asPath: asFile.path),
      ],
    );
    final out = p.join(tmp.path, 'm.goremod');
    await saveProject(project, out);
    final loaded = await loadProject(out);
    expect(loaded.scripts.map((s) => s.moduleName).toList(), ['Good']);
    expect(loaded.scripts.single.relPath, 'AI/Good.as');
  });
}
