import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_scoped_content_browser.dart';

void main() {
  testWidgets('defaults to This mod and mounts other scopes lazily', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness());

    expect(find.text('THIS MOD BODY'), findsOneWidget);
    expect(find.text('BASE GAME BODY', skipOffstage: false), findsNothing);
    expect(find.text('INSTALLED BODY', skipOffstage: false), findsNothing);
    expect(find.text('ALL SOURCES BODY', skipOffstage: false), findsNothing);

    await tester.tap(find.text('Base game'));
    await tester.pump();

    expect(find.text('THIS MOD BODY'), findsNothing);
    expect(find.text('BASE GAME BODY'), findsOneWidget);
    expect(find.text('THIS MOD BODY', skipOffstage: false), findsOneWidget);
    expect(find.text('INSTALLED BODY', skipOffstage: false), findsNothing);

    await tester.tap(find.text('Installed'));
    await tester.pump();

    expect(find.text('INSTALLED BODY'), findsOneWidget);
    expect(
      find.byKey(
        const Key('revision3-scoped-content-browser-page-installed'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );

    await tester.tap(find.text('Search all'));
    await tester.pump();

    expect(find.text('ALL SOURCES BODY'), findsOneWidget);
    expect(
      find.byKey(
        const Key('revision3-scoped-content-browser-page-all-sources'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );
  });

  testWidgets('retains state in every mounted scope while switching', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness());

    await tester.tap(find.text('THIS MOD INCREMENT'));
    await tester.pump();
    expect(find.text('THIS MOD COUNT 1'), findsOneWidget);

    await tester.tap(find.text('Base game'));
    await tester.pump();
    await tester.tap(find.text('BASE GAME INCREMENT'));
    await tester.pump();
    expect(find.text('BASE GAME COUNT 1'), findsOneWidget);

    await tester.tap(find.text('This mod'));
    await tester.pump();
    expect(find.text('THIS MOD COUNT 1'), findsOneWidget);

    await tester.tap(find.text('Base game'));
    await tester.pump();
    expect(find.text('BASE GAME COUNT 1'), findsOneWidget);
  });

  testWidgets('same identity retains selection and new identity resets pages', (
    tester,
  ) async {
    final key = GlobalKey<_HarnessState>();
    await tester.pumpWidget(_Harness(key: key));

    await tester.tap(find.text('Base game'));
    await tester.pump();
    await tester.tap(find.text('BASE GAME INCREMENT'));
    await tester.pump();
    expect(find.text('BASE GAME COUNT 1'), findsOneWidget);

    key.currentState!.rebuildSameProject();
    await tester.pump();
    expect(find.text('BASE GAME COUNT 1'), findsOneWidget);

    key.currentState!.openDifferentProject();
    await tester.pump();
    expect(find.text('THIS MOD BODY'), findsOneWidget);
    expect(find.text('BASE GAME BODY', skipOffstage: false), findsNothing);

    await tester.tap(find.text('Base game'));
    await tester.pump();
    expect(find.text('BASE GAME COUNT 0'), findsOneWidget);
  });

  testWidgets('scope row scrolls without overflow at 280 by 300', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(280, 300);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const _Harness());

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(
        const Key('revision3-scoped-content-browser-navigation-scroll'),
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-scoped-content-browser-pages')),
      findsOneWidget,
    );
  });

  testWidgets('descendant can return to an exact presentation scope', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Revision3ScopedContentBrowser(
            projectIdentity: 'project',
            thisModLabel: 'This mod',
            baseGameLabel: 'Base game',
            installedLabel: 'Installed',
            allSourcesLabel: 'Search all',
            thisMod: const Text('THIS MOD TARGET'),
            baseGame: const Text('BASE'),
            installed: const Text('INSTALLED'),
            allSources: Builder(
              builder: (context) => TextButton(
                onPressed: () => Revision3ScopedContentBrowser.navigate(
                  context,
                  Revision3ScopedContentScope.thisMod,
                ),
                child: const Text('OPEN THIS MOD'),
              ),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('Search all'));
    await tester.pump();
    await tester.tap(find.text('OPEN THIS MOD'));
    await tester.pump();

    expect(find.text('THIS MOD TARGET'), findsOneWidget);
  });
}

class _Harness extends StatefulWidget {
  const _Harness({super.key});

  @override
  State<_Harness> createState() => _HarnessState();
}

class _HarnessState extends State<_Harness> {
  String _projectIdentity = 'project-a';
  int _revision = 0;

  void rebuildSameProject() => setState(() => _revision++);

  void openDifferentProject() => setState(() {
    _projectIdentity = 'project-b';
    _revision = 0;
  });

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3ScopedContentBrowser(
        projectIdentity: _projectIdentity,
        thisModLabel: 'This mod',
        baseGameLabel: 'Base game',
        installedLabel: 'Installed',
        allSourcesLabel: 'Search all',
        thisMod: _ScopeProbe(name: 'THIS MOD', revision: _revision),
        baseGame: _ScopeProbe(name: 'BASE GAME', revision: _revision),
        installed: _ScopeProbe(name: 'INSTALLED', revision: _revision),
        allSources: _ScopeProbe(name: 'ALL SOURCES', revision: _revision),
      ),
    ),
  );
}

class _ScopeProbe extends StatefulWidget {
  const _ScopeProbe({required this.name, required this.revision});

  final String name;
  final int revision;

  @override
  State<_ScopeProbe> createState() => _ScopeProbeState();
}

class _ScopeProbeState extends State<_ScopeProbe> {
  int _count = 0;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('${widget.name} BODY'),
        Text('${widget.name} COUNT $_count'),
        TextButton(
          onPressed: () => setState(() => _count++),
          child: Text('${widget.name} INCREMENT'),
        ),
      ],
    ),
  );
}
