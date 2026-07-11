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
  op: ScriptOp.edit,
  moduleName: 'Bar',
  relPath: 'Bar.as',
  asPath: '',
);

Future<void> _pumpTab(
  WidgetTester tester, {
  ScriptMod? staged,
  bool onlyStaged = false,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        scriptModulesProvider.overrideWith((ref) async => _fakeModules()),
        if (staged != null)
          scriptModsProvider.overrideWith(
            (ref) => ScriptModsNotifier()..setMod(staged),
          ),
      ],
      child: MaterialApp(
        home: Scaffold(body: ScriptTab(onlyStaged: onlyStaged)),
      ),
    ),
  );
  // Resolve the modules future (loading spinner → data).
  await tester.pump();
  await tester.pump();
}

void main() {
  testWidgets('existing-module edit can explicitly opt in to new symbols', (
    tester,
  ) async {
    const staged = ScriptMod(
      op: ScriptOp.edit,
      moduleName: 'Bar',
      relPath: 'Bar.as',
      asPath: 'Bar.as',
      miniPath: 'old.cache',
      compiledHash: 'old-hash',
    );
    await _pumpTab(tester, staged: staged);

    await tester.tap(find.text('Bar.as').first);
    await tester.pump();
    final toggle = tester.widget<SwitchListTile>(find.byType(SwitchListTile));
    expect(toggle.value, isFalse);

    await tester.tap(find.text('Allow new symbols'));
    await tester.pump();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ScriptTab)),
      listen: false,
    );
    final updated = container.read(scriptModsProvider).entries.single;
    expect(updated.allowNewSymbols, isTrue);
    expect(updated.miniPath, isEmpty);
    expect(updated.compiledHash, isEmpty);
    expect(find.text('Symbol policy changed — compile again.'), findsOneWidget);
  });

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
      of: find.text('Bar.as'),
      matching: find.byType(ListTile),
    );
    expect(
      find.descendant(of: barTile, matching: find.byIcon(Icons.check)),
      findsOneWidget,
    );

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
        matching: find.byWidgetPredicate((w) => w is FilledButton),
      ),
      findsOneWidget,
    );
    // Left pane: the flat hit's subtitle shows the path.
    expect(
      find.descendant(
        of: find.byType(ListTile),
        matching: find.text('Misc/Other.as'),
      ),
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
        overrides: [scriptModulesProvider.overrideWith((ref) async => modules)],
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
      listen: false,
    );
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
      final tile = find.ancestor(
        of: find.text(leaf),
        matching: find.byType(ListTile),
      );
      expect(
        find.descendant(of: tile, matching: find.byIcon(Icons.check)),
        findsOneWidget,
        reason: 'leaf $leaf should be check-marked',
      );
    }
    // The selection (the staged mod's REAL relPath) highlights its first
    // owning leaf in the tree.
    expect(
      tester
          .widget<ListTile>(
            find.ancestor(
              of: find.text('Foo.as'),
              matching: find.byType(ListTile),
            ),
          )
          .selected,
      isTrue,
    );

    // Detail semantics: BOTH leaves share the staged real relPath, so tapping
    // EITHER shows the staged detail — the disambiguated leaf must not claim
    // "vanilla" (its Edit would silently overwrite the staged mod with a fresh
    // vanilla emit). With no vanilla card, no second Edit overwrite is possible.
    for (final leaf in ['Foo (2).as', 'Foo.as']) {
      // Tap the TREE leaf specifically — the staged detail's Path row echoes
      // 'Foo.as' as plain text too, which would make a bare text tap ambiguous.
      await tester.tap(
        find.descendant(of: find.byType(ListTile), matching: find.text(leaf)),
      );
      await tester.pump();
      expect(
        find.text('Edit existing module'),
        findsOneWidget,
        reason: 'leaf $leaf should show the staged detail',
      );
      expect(
        find.text('Vanilla module — not staged'),
        findsNothing,
        reason: 'leaf $leaf must not claim vanilla',
      );
      expect(
        find.text('Edit'),
        findsNothing,
        reason: 'leaf $leaf must not offer a vanilla Edit',
      );
    }
    // The staged mod survived untouched (an overwrite would re-stage 'Foo').
    expect(
      container.read(scriptModsProvider).items.values.single.moduleName,
      'Bar',
    );

    // The flat search list uses the same real-relPath marker semantics: both
    // hits (matched via their tree paths) carry the staged check.
    await tester.enterText(find.byType(TextField), 'foo');
    await tester.pump();
    expect(
      find.descendant(
        of: find.byType(ListTile),
        matching: find.byIcon(Icons.check),
      ),
      findsNWidgets(2),
    );
  });

  testWidgets('staged panel lists the mod with compile status, tapping selects '
      'it, delete unstages and the detail falls back to vanilla', (
    tester,
  ) async {
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
      find.textContaining('not compiled', findRichText: true),
      findsWidgets,
    );

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

  testWidgets('staged panel scrolls long lists instead of overflowing the tab', (
    tester,
  ) async {
    final notifier = ScriptModsNotifier();
    for (var i = 0; i < 30; i++) {
      notifier.setMod(
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Mod$i',
          relPath: 'Mods/Mod$i.as',
          asPath: '',
        ),
      );
    }
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scriptModulesProvider.overrideWith((ref) async => const []),
          scriptModsProvider.overrideWith((ref) => notifier),
        ],
        child: const MaterialApp(home: Scaffold(body: ScriptTab())),
      ),
    );
    await tester.pump();
    await tester.pump();

    // Expanding must not overflow the page column (the framework turns a
    // RenderFlex overflow into a test failure automatically).
    await tester.tap(find.text('Staged script mods (30)'));
    await tester.pumpAndSettle();

    // The entries area is height-capped and lazily built: the first row is
    // there, the last is beyond the cap and not built yet.
    expect(find.text('Mod0'), findsOneWidget);
    expect(find.text('Mod29'), findsNothing);

    // The list scrolls INSIDE the panel to reach the last row. (Don't anchor
    // the scrollable finder on a row widget — rows unbuild as they scroll out,
    // which would empty the finder mid-scroll. With the browser showing the
    // no-modules hint, the panel's ListView is the only Scrollable here.)
    await tester.scrollUntilVisible(
      find.text('Mod29'),
      80,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text('Mod29'), findsOneWidget);
  });

  testWidgets('onlyStaged: browser shows only staged leaves, search filters '
      'within them, un-staging empties live and re-staging restores', (
    tester,
  ) async {
    await _pumpTab(tester, staged: _stagedBar(), onlyStaged: true);

    // Only the staged module's leaf renders — vanilla folders/leaves are out,
    // and the count caption covers the staged slice only.
    expect(find.text('Bar.as'), findsOneWidget);
    expect(find.text('Gameplay'), findsNothing);
    expect(find.text('Misc'), findsNothing);
    expect(find.text('1 modules'), findsOneWidget);
    // The staged bottom panel is unchanged.
    expect(find.text('Staged script mods (1)'), findsOneWidget);

    // Search runs over the FILTERED entries: 'ba' matches staged Bar but must
    // not surface the unstaged Baz; the totals are staged-slice totals.
    await tester.enterText(find.byType(TextField), 'ba');
    await tester.pump();
    expect(find.text('Bar'), findsOneWidget);
    expect(find.text('Baz'), findsNothing);
    expect(find.text('1 match / 1 total'), findsOneWidget);
    await tester.tap(find.byIcon(Icons.clear));
    await tester.pump();

    // Un-stage through the container (as any outside action would): the
    // browser empties LIVE to the hint while the staged panel follows.
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ScriptTab)),
      listen: false,
    );
    container.read(scriptModsProvider.notifier).remove('Bar.as');
    await tester.pump();
    expect(find.text('Bar.as'), findsNothing);
    expect(
      find.textContaining('No staged edits of vanilla modules'),
      findsOneWidget,
    );
    expect(find.text('Staged script mods (0)'), findsOneWidget);

    // Re-staging brings the leaf back (fresh filtered-list identity → the
    // tree rebuilds).
    container.read(scriptModsProvider.notifier).setMod(_stagedBar());
    await tester.pump();
    expect(find.text('Bar.as'), findsOneWidget);
    expect(find.text('1 modules'), findsOneWidget);
  });

  testWidgets('onlyStaged: a staged add (no vanilla leaf) stays out of the '
      'browser but in the panel; a folder-nested staged edit shows only its '
      'own leaf', (tester) async {
    final notifier = ScriptModsNotifier()
      ..setMod(
        const ScriptMod(
          op: ScriptOp.edit,
          moduleName: 'Foo',
          relPath: 'Gameplay/Foo.as',
          asPath: '',
        ),
      )
      ..setMod(
        const ScriptMod(
          op: ScriptOp.add,
          moduleName: 'New',
          relPath: 'Mods/New.as',
          asPath: '',
        ),
      );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scriptModulesProvider.overrideWith((ref) async => _fakeModules()),
          scriptModsProvider.overrideWith((ref) => notifier),
        ],
        child: const MaterialApp(
          home: Scaffold(body: ScriptTab(onlyStaged: true)),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    // Only the staged EDIT's folder renders; the add has no vanilla leaf and
    // must not invent one. The caption counts vanilla leaves only.
    expect(find.text('Gameplay'), findsOneWidget);
    expect(find.text('Mods'), findsNothing);
    expect(find.text('Bar.as'), findsNothing);
    expect(find.text('1 modules'), findsOneWidget);

    // Expanding the folder shows only the staged leaf (check-marked), not the
    // unstaged sibling.
    await tester.tap(find.text('Gameplay'));
    await tester.pump();
    expect(find.text('Foo.as'), findsOneWidget);
    expect(find.text('Baz.as'), findsNothing);
    final fooTile = find.ancestor(
      of: find.text('Foo.as'),
      matching: find.byType(ListTile),
    );
    expect(
      find.descendant(of: fooTile, matching: find.byIcon(Icons.check)),
      findsOneWidget,
    );

    // The add remains reachable via the staged panel.
    expect(find.text('Staged script mods (2)'), findsOneWidget);
    await tester.tap(find.text('Staged script mods (2)'));
    await tester.pumpAndSettle();
    expect(find.text('New'), findsOneWidget);
  });

  testWidgets('onlyStaged embed keeps the shared selection but shows the '
      'placeholder for non-staged selections; un-staging falls back to the '
      'placeholder, never the vanilla editor', (tester) async {
    // One container across both mounts — like the real app, where the main
    // Scripts tab (kept alive) and the ChangesTab embed share the app-scoped
    // selection provider.
    final container = ProviderContainer(
      overrides: [
        scriptModulesProvider.overrideWith((ref) async => _fakeModules()),
      ],
    );
    addTearDown(container.dispose);

    Future<void> pumpTab(Widget tab) async {
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(home: Scaffold(body: tab)),
        ),
      );
      await tester.pump();
      await tester.pump();
    }

    // Seed the selection through the MAIN tab (the provider is private to the
    // library, so seed it the way the app does — by selecting a module).
    await pumpTab(const ScriptTab());
    await tester.tap(find.text('Bar.as'));
    await tester.pump();
    expect(find.text('Vanilla module — not staged'), findsOneWidget);

    // Opening Changes>Scripts mounts a FRESH staged-only embed on every visit
    // (plain content swap, no keep-alive). The selection is not staged, so
    // the browser shows the empty hint and the detail shows the PLACEHOLDER —
    // a vanilla editor for an entry the filtered browser doesn't list would
    // contradict the view.
    await pumpTab(const ScriptTab(onlyStaged: true));
    expect(
      find.textContaining('No staged edits of vanilla modules'),
      findsOneWidget,
    );
    expect(find.text('Select or add a script mod'), findsOneWidget);
    expect(find.text('Vanilla module — not staged'), findsNothing);

    // The embed's initState must NOT have nulled the shared selection:
    // staging the selected module makes its staged detail (and highlighted
    // leaf) appear WITHOUT any new tap.
    container.read(scriptModsProvider.notifier).setMod(_stagedBar());
    await tester.pump();
    expect(find.text('Edit existing module'), findsOneWidget);
    expect(
      tester
          .widget<ListTile>(
            find.ancestor(
              of: find.text('Bar.as'),
              matching: find.byType(ListTile),
            ),
          )
          .selected,
      isTrue,
    );

    // Un-staging the last mod while it is selected: back to the placeholder —
    // never the vanilla editor the main tab would show.
    container.read(scriptModsProvider.notifier).remove('Bar.as');
    await tester.pump();
    expect(find.text('Select or add a script mod'), findsOneWidget);
    expect(find.text('Vanilla module — not staged'), findsNothing);
    expect(find.text('Edit existing module'), findsNothing);
  });

  testWidgets('a fresh main-tab mount keeps a shared selection made in the '
      'Changes embed first (no mount-time reset)', (tester) async {
    // One container across both mounts. The user visits Changes>Scripts and
    // selects a staged module BEFORE the main Scripts tab has ever built —
    // the first main-tab mount must NOT clobber the shared selection.
    final container = ProviderContainer(
      overrides: [
        scriptModulesProvider.overrideWith((ref) async => _fakeModules()),
        scriptModsProvider.overrideWith(
          (ref) => ScriptModsNotifier()..setMod(_stagedBar()),
        ),
      ],
    );
    addTearDown(container.dispose);

    Future<void> pumpTab(Widget tab) async {
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(home: Scaffold(body: tab)),
        ),
      );
      await tester.pump();
      await tester.pump();
    }

    // Select the staged module in the embed.
    await pumpTab(const ScriptTab(key: ValueKey('embed'), onlyStaged: true));
    await tester.tap(find.text('Bar.as'));
    await tester.pump();
    expect(find.text('Edit existing module'), findsOneWidget);

    // First-ever MAIN mount (distinct key → fresh State, initState runs; in
    // the real app the embed and the main tab are separate mounts anyway).
    // The selection survives: staged detail + highlighted leaf, no
    // placeholder.
    await pumpTab(const ScriptTab(key: ValueKey('main')));
    expect(find.text('Gameplay'), findsOneWidget); // full browser = main tab
    expect(find.text('Edit existing module'), findsOneWidget);
    expect(find.text('Select or add a script mod'), findsNothing);
    expect(
      tester
          .widget<ListTile>(
            find.ancestor(
              of: find.text('Bar.as'),
              matching: find.byType(ListTile),
            ),
          )
          .selected,
      isTrue,
    );
  });

  testWidgets('staged-panel selection resolves by REAL relPath even when it '
      'equals another module\'s generated collision leaf', (tester) async {
    // Pathological vanilla list: two modules collide on real 'Foo.as' (the
    // second gets the generated leaf 'Foo (2).as'), and a third module's REAL
    // path is literally 'Foo (2).as' — displaced to leaf 'Foo (2) (2).as'.
    final modules = [
      ScriptModuleInfo(name: 'Foo', file: 'Foo.as'),
      ScriptModuleInfo(name: 'Bar', file: 'Foo.as'),
      ScriptModuleInfo(name: 'Baz', file: 'Foo (2).as'),
    ];
    final notifier = ScriptModsNotifier()
      ..setMod(
        const ScriptMod(
          op: ScriptOp.edit,
          moduleName: 'Baz',
          relPath: 'Foo (2).as',
          asPath: '',
        ),
      );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scriptModulesProvider.overrideWith((ref) async => modules),
          scriptModsProvider.overrideWith((ref) => notifier),
        ],
        child: const MaterialApp(home: Scaffold(body: ScriptTab())),
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(find.text('Foo.as'), findsOneWidget);
    expect(find.text('Foo (2).as'), findsOneWidget);
    expect(find.text('Foo (2) (2).as'), findsOneWidget);

    // Select the staged mod via the staged panel — the selection becomes the
    // mod's REAL relPath 'Foo (2).as', which TEXT-equals Bar's generated leaf.
    await tester.tap(find.text('Staged script mods (1)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Baz'));
    await tester.pumpAndSettle();

    // A staged key resolves by relPath FIRST: the highlight lands on Baz's
    // ACTUAL leaf, not on Bar's same-text generated leaf, and the detail is
    // the staged mod (not Bar's vanilla card).
    expect(find.text('Edit existing module'), findsOneWidget);
    ListTile tileOf(String label) => tester.widget<ListTile>(
      find.ancestor(of: find.text(label), matching: find.byType(ListTile)),
    );
    expect(tileOf('Foo (2) (2).as').selected, isTrue);
    expect(tileOf('Foo (2).as').selected, isFalse);
    // The staged check marker agrees (real-relPath keyed): Baz's leaf only.
    final bazTile = find.ancestor(
      of: find.text('Foo (2) (2).as'),
      matching: find.byType(ListTile),
    );
    expect(
      find.descendant(of: bazTile, matching: find.byIcon(Icons.check)),
      findsOneWidget,
    );
  });

  testWidgets('game-path change: after remount + module reload a selection '
      'the new install cannot resolve shows the placeholder', (tester) async {
    // One container across both mounts — like the real app: the selection
    // provider lives in the root ProviderScope, GamePathScope swaps the
    // subtree key on a game-exe-path change (fresh State), and
    // scriptModulesProvider reloads the NEW install's list. There is no
    // mount-time selection reset (it would clobber valid cross-view
    // selections) — a stale selection is neutralized by the render guards.
    var modules = _fakeModules();
    final container = ProviderContainer(
      overrides: [scriptModulesProvider.overrideWith((ref) async => modules)],
    );
    addTearDown(container.dispose);

    Future<void> pumpTab(Key key) async {
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(body: ScriptTab(key: key)),
          ),
        ),
      );
      await tester.pump();
      await tester.pump();
    }

    await pumpTab(const ValueKey('install-1'));
    await tester.tap(find.text('Bar.as'));
    await tester.pump();
    expect(find.text('Vanilla module — not staged'), findsOneWidget);

    // Install 2 has no 'Bar.as' (nor any other install-1 path): the stale
    // selection resolves to neither a staged key nor a current module, so
    // the detail falls back to the action-less placeholder — the render
    // guards cover what the old mount-time reset was for.
    modules = [ScriptModuleInfo(name: 'Other', file: 'Misc/Other.as')];
    container.refresh(scriptModulesProvider);
    await pumpTab(const ValueKey('install-2'));
    expect(find.text('Vanilla module — not staged'), findsNothing);
    expect(find.text('Select or add a script mod'), findsOneWidget);
    // No stale-module action is reachable: no vanilla Edit button anywhere.
    expect(find.text('Edit'), findsNothing);
  });

  // Fix 3: loadProject treats the script relPath as untrusted (defense-in-depth, matching the
  // asPath guard + gore-as compile-side check) and drops mods whose relPath is empty/absolute/'..'.
  test('loadProject drops script mods with an unsafe relPath', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_relpath_test_');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final asFile = File(p.join(tmp.path, 'New.as'))
      ..writeAsStringSync('void Foo(){}');
    final project = ModProject(
      name: 'M',
      scripts: [
        // Safe sibling — must survive the load.
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Good',
          relPath: 'AI/Good.as',
          asPath: asFile.path,
        ),
        // Escapes the staged tree — must be dropped.
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Esc',
          relPath: '../evil.as',
          asPath: asFile.path,
        ),
        // Absolute — must be dropped.
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Abs',
          relPath: '/etc/evil.as',
          asPath: asFile.path,
        ),
        // Empty — must be dropped.
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Empty',
          relPath: '',
          asPath: asFile.path,
        ),
      ],
    );
    final out = p.join(tmp.path, 'm.goremod');
    await saveProject(project, out);
    final loaded = await loadProject(out);
    expect(loaded.scripts.map((s) => s.moduleName).toList(), ['Good']);
    expect(loaded.scripts.single.relPath, 'AI/Good.as');
  });
}
