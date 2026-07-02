import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/path_tree.dart';

void main() {
  testWidgets('compresses single-child chains and expands folders', (t) async {
    String? tapped;
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PathTreeBrowser(
            paths: const [
              'A/B/leaf1.uasset',
              'A/B/leaf2.uasset',
              'C/leaf3.uasset',
            ],
            selectedPath: null,
            onSelect: (p) => tapped = p,
            leafIcon: Icons.image_outlined,
          ),
        ),
      ),
    );
    // Single-child folder chain A -> B renders as one compressed row "A/B".
    expect(find.text('A/B'), findsOneWidget);
    // Folders start collapsed: leaves hidden until expanded.
    expect(find.text('leaf1.uasset'), findsNothing);
    // Folder rows show their leaf-count badge.
    expect(find.text('2'), findsOneWidget); // A/B
    expect(find.text('1'), findsOneWidget); // C
    await t.tap(find.text('A/B'));
    await t.pumpAndSettle();
    expect(find.text('leaf1.uasset'), findsOneWidget);
    expect(find.text('leaf2.uasset'), findsOneWidget);
    // C is still collapsed.
    expect(find.text('leaf3.uasset'), findsNothing);
    await t.tap(find.text('leaf1.uasset'));
    expect(tapped, 'A/B/leaf1.uasset');
  });

  testWidgets('marked leaves show a trailing check and selection highlights', (
    t,
  ) async {
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PathTreeBrowser(
            paths: const ['A/leaf1.uasset', 'A/leaf2.uasset'],
            selectedPath: 'A/leaf1.uasset',
            onSelect: (_) {},
            leafIcon: Icons.description_outlined,
            markedPaths: const {'A/leaf2.uasset'},
          ),
        ),
      ),
    );
    await t.tap(find.text('A'));
    await t.pumpAndSettle();
    // Marked leaf carries the trailing check; unmarked one doesn't.
    expect(find.byIcon(Icons.check), findsOneWidget);
    final marked = t.widget<ListTile>(
      find.ancestor(
        of: find.text('leaf2.uasset'),
        matching: find.byType(ListTile),
      ),
    );
    expect(marked.trailing, isNotNull);
    final selected = t.widget<ListTile>(
      find.ancestor(
        of: find.text('leaf1.uasset'),
        matching: find.byType(ListTile),
      ),
    );
    expect(selected.selected, isTrue);
    // The custom leaf icon is used for leaves.
    expect(find.byIcon(Icons.description_outlined), findsNWidgets(2));
  });

  testWidgets('rebuilds the tree only when the paths list identity changes', (
    t,
  ) async {
    Widget host(List<String> paths) => MaterialApp(
      home: Scaffold(
        body: PathTreeBrowser(
          paths: paths,
          selectedPath: null,
          onSelect: (_) {},
          leafIcon: Icons.image_outlined,
        ),
      ),
    );
    final first = ['A/leaf1.uasset'];
    await t.pumpWidget(host(first));
    expect(find.text('A'), findsOneWidget);
    // Same identity: tree unchanged.
    await t.pumpWidget(host(first));
    expect(find.text('A'), findsOneWidget);
    // New list identity: tree rebuilt with the new contents.
    await t.pumpWidget(host(['D/leaf9.uasset']));
    expect(find.text('A'), findsNothing);
    expect(find.text('D'), findsOneWidget);
  });
}
