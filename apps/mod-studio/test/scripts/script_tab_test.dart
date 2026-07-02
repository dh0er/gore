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
    // Left pane: the flat hit's subtitle shows the path.
    expect(
      find.descendant(
          of: find.byType(ListTile), matching: find.text('Misc/Other.as')),
      findsOneWidget,
    );
    // Right pane: the detail's Path row echoes it (the only occurrence that is
    // NOT inside a ListTile).
    final inDetail = find
        .text('Misc/Other.as')
        .evaluate()
        .where((e) => e.findAncestorWidgetOfExactType<ListTile>() == null);
    expect(inDetail.length, 1);
  });

  testWidgets('colliding relPaths are disambiguated in the tree and Edit '
      'stages the right module under its REAL relPath', (tester) async {
    // 'Foo' has no recorded file → fallback 'Foo.as', which collides with the
    // real root-level file of 'Bar'.
    final modules = [
      ScriptModuleInfo(name: 'Foo', file: ''),
      ScriptModuleInfo(name: 'Bar', file: 'Foo.as'),
    ];
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scriptModulesProvider.overrideWith((ref) async => modules),
        ],
        child: const MaterialApp(home: Scaffold(body: ScriptTab())),
      ),
    );
    await tester.pump();
    await tester.pump();

    // Both modules appear: the FIRST keeps the pristine path, the collision
    // gets a display-only suffix — and the caption agrees with the leaf count.
    expect(find.text('Foo.as'), findsOneWidget);
    expect(find.text('Foo (2).as'), findsOneWidget);
    expect(find.text('2 modules'), findsOneWidget);

    // The pristine leaf maps to the first module...
    await tester.tap(find.text('Foo.as'));
    await tester.pump();
    expect(find.text('Vanilla module — not staged'), findsOneWidget);
    expect(find.text('Foo'), findsNWidgets(2)); // detail title + Module row

    // ...and the disambiguated leaf maps to the second.
    await tester.tap(find.text('Foo (2).as'));
    await tester.pump();
    expect(find.text('Bar'), findsNWidgets(2));

    // Edit stages THAT module, keyed by its REAL relPath (no game configured
    // in tests → no emit; staged without a source, like the old picker flow).
    await tester.tap(find.text('Edit'));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
        tester.element(find.byType(ScriptTab)),
        listen: false);
    final staged = container.read(scriptModsProvider).items;
    expect(staged.keys.single, 'Foo.as');
    expect(staged.values.single.moduleName, 'Bar');
    expect(staged.values.single.op, ScriptOp.edit);
    // The staged detail replaces the vanilla card.
    expect(find.text('Edit existing module'), findsOneWidget);

    // Marker semantics: the staged key is the REAL relPath, which BOTH leaves
    // share (their staging keys collide for such data) — so the check marker
    // shows on every leaf whose real relPath is staged, disambiguated or not.
    for (final leaf in ['Foo.as', 'Foo (2).as']) {
      final tile =
          find.ancestor(of: find.text(leaf), matching: find.byType(ListTile));
      expect(find.descendant(of: tile, matching: find.byIcon(Icons.check)),
          findsOneWidget, reason: 'leaf $leaf should be check-marked');
    }
    // The selection (the staged mod's REAL relPath) highlights its first
    // owning leaf in the tree.
    expect(
      tester
          .widget<ListTile>(find.ancestor(
              of: find.text('Foo.as'), matching: find.byType(ListTile)))
          .selected,
      isTrue,
    );

    // The flat search list uses the same real-relPath marker semantics: both
    // hits (matched via their tree paths) carry the staged check.
    await tester.enterText(find.byType(TextField), 'foo');
    await tester.pump();
    expect(
      find.descendant(
          of: find.byType(ListTile), matching: find.byIcon(Icons.check)),
      findsNWidgets(2),
    );
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
