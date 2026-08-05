import 'dart:ui' show SemanticsAction, SemanticsRole, Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';

void main() {
  test('requires every destination once in canonical order', () {
    expect(Revision3ProjectWorkspaceSection.values, const [
      Revision3ProjectWorkspaceSection.home,
      Revision3ProjectWorkspaceSection.content,
      Revision3ProjectWorkspaceSection.story,
      Revision3ProjectWorkspaceSection.textVoice,
      Revision3ProjectWorkspaceSection.testRelease,
    ]);
    final canonical = _destinations();

    expect(
      () => Revision3ProjectWorkspace(
        projectIdentity: 'project-fixture',
        destinations: canonical.sublist(0, canonical.length - 1),
      ),
      throwsArgumentError,
    );

    final wrongOrder = [...canonical];
    final first = wrongOrder.removeAt(0);
    wrongOrder.insert(1, first);
    expect(
      () => Revision3ProjectWorkspace(
        projectIdentity: 'project-fixture',
        destinations: wrongOrder,
      ),
      throwsArgumentError,
    );

    final workspace = Revision3ProjectWorkspace(
      projectIdentity: 'project-fixture',
      destinations: canonical,
    );
    expect(
      workspace.destinations.map((destination) => destination.section),
      Revision3ProjectWorkspaceSection.values,
    );
    expect(
      () => workspace.destinations.add(canonical.first),
      throwsUnsupportedError,
    );
  });

  testWidgets(
    'desktop shell exposes every canonical destination as a direct top tab',
    (tester) async {
      _setSurface(tester, const Size(1400, 900));
      final semantics = tester.ensureSemantics();
      await _pumpWorkspace(tester);

      expect(
        find.byKey(const Key('revision3-project-workspace-tabbar')),
        findsOneWidget,
      );
      expect(find.byType(TabBar), findsOneWidget);
      expect(find.byType(NavigationRail), findsNothing);
      expect(find.byType(PopupMenuButton), findsNothing);
      expect(
        _selectedTabIndex(tester),
        Revision3ProjectWorkspaceSection.home.index,
      );
      for (final section in Revision3ProjectWorkspaceSection.values) {
        expect(find.byKey(_tabKey(section)), findsOneWidget);
      }
      expect(find.text('home page / secondary:none'), findsOneWidget);
      _expectTabSemantics(
        tester,
        Revision3ProjectWorkspaceSection.home,
        selected: true,
      );
      _expectTabSemantics(
        tester,
        Revision3ProjectWorkspaceSection.story,
        selected: false,
      );
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets(
    '640x420 shell keeps every destination directly reachable and exact '
    'programmatic navigation reveals the offscreen selected tab',
    (tester) async {
      _setSurface(tester, const Size(640, 420));
      final semantics = tester.ensureSemantics();
      await _pumpWorkspace(
        tester,
        destinations: _destinations(
          pageBuilder: (section) =>
              (context, location) => Center(
                child: section == Revision3ProjectWorkspaceSection.home
                    ? FilledButton(
                        key: const Key('navigate-to-last-tab'),
                        onPressed: () => Revision3ProjectWorkspace.navigate(
                          context,
                          const Revision3ProjectWorkspaceLocation(
                            Revision3ProjectWorkspaceSection.testRelease,
                            secondary: 'release',
                          ),
                        ),
                        child: const Text('Open test and release fixture'),
                      )
                    : Text(
                        '${section.name} page / '
                        'secondary:${location.secondary ?? 'none'}',
                      ),
              ),
        ),
      );

      expect(
        find.byKey(const Key('revision3-project-workspace-tabbar')),
        findsOneWidget,
      );
      for (final section in Revision3ProjectWorkspaceSection.values) {
        expect(find.byKey(_tabKey(section)), findsOneWidget);
      }

      final lastTab = find.byKey(
        _tabKey(Revision3ProjectWorkspaceSection.testRelease),
      );
      expect(lastTab.hitTestable(), findsNothing);

      await tester.tap(find.byKey(const Key('navigate-to-last-tab')));
      await tester.pumpAndSettle();

      expect(find.text('testRelease page / secondary:release'), findsOneWidget);
      expect(
        _selectedTabIndex(tester),
        Revision3ProjectWorkspaceSection.testRelease.index,
      );
      expect(lastTab.hitTestable(), findsOneWidget);
      _expectTabSemantics(
        tester,
        Revision3ProjectWorkspaceSection.testRelease,
        selected: true,
      );
      _expectTabSemantics(
        tester,
        Revision3ProjectWorkspaceSection.home,
        selected: false,
      );
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets('360x640 at 200 percent exposes every area through one semantic '
      'keyboard and pointer selector', (tester) async {
    _setSurface(tester, const Size(360, 640));
    final semantics = tester.ensureSemantics();
    await _pumpWorkspace(tester, textScaler: const TextScaler.linear(2));

    expect(
      find.byKey(const Key('revision3-project-workspace-tabbar')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-project-workspace-section-selector')),
      findsOneWidget,
    );
    for (final section in Revision3ProjectWorkspaceSection.values) {
      expect(find.byKey(_tabKey(section)), findsNothing);
    }
    expect(tester.takeException(), isNull);

    final selectorSemantics = find.byKey(
      const Key('revision3-project-workspace-section-selector-semantics'),
    );
    var node = tester.getSemantics(selectorSemantics);
    expect(node.label, _label(Revision3ProjectWorkspaceSection.home));
    expect(node.getSemanticsData().flagsCollection.isButton, isTrue);
    expect(node.getSemanticsData().hasAction(SemanticsAction.tap), isTrue);

    expect(await _focusCompactSelector(tester), isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    for (final section in Revision3ProjectWorkspaceSection.values) {
      expect(find.byKey(_sectionOptionKey(section)), findsOneWidget);
    }

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(await _focusCompactSelector(tester), isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    for (final section in Revision3ProjectWorkspaceSection.values) {
      expect(find.byKey(_sectionOptionKey(section)), findsOneWidget);
    }

    const target = Revision3ProjectWorkspaceSection.textVoice;
    await tester.tap(find.byKey(_sectionOptionKey(target)));
    await tester.pumpAndSettle();

    expect(find.text('textVoice page / secondary:none'), findsOneWidget);
    expect(_selectedCompactSection(tester), target);
    node = tester.getSemantics(selectorSemantics);
    expect(node.label, _label(target));
    expect(node.getSemanticsData().flagsCollection.isButton, isTrue);
    expect(tester.takeException(), isNull);
    semantics.dispose();
  });

  testWidgets(
    'persistent chrome follows primary and secondary navigation above pages',
    (tester) async {
      _setSurface(tester, const Size(1000, 800));
      await _pumpWorkspace(
        tester,
        chromeBuilder: (context, location) => Material(
          child: Text(
            'chrome:${location.section.name}:'
            '${location.secondary ?? 'none'}',
          ),
        ),
        destinations: _destinations(
          pageBuilder: (section) =>
              (context, location) => Center(
                child: FilledButton(
                  key: ValueKey('set-secondary-${section.name}'),
                  onPressed: () => Revision3ProjectWorkspace.navigate(
                    context,
                    Revision3ProjectWorkspaceLocation(
                      section,
                      secondary: 'details',
                    ),
                  ),
                  child: Text('${section.name} page'),
                ),
              ),
        ),
      );

      expect(
        find.byKey(const Key('revision3-project-workspace-chrome')),
        findsOneWidget,
      );
      expect(find.text('chrome:home:none'), findsOneWidget);
      final chromeTop = tester.getTopLeft(
        find.byKey(const Key('revision3-project-workspace-chrome')),
      );
      final pageTop = tester.getTopLeft(find.text('home page'));
      expect(chromeTop.dy, lessThan(pageTop.dy));

      await _tapTab(tester, Revision3ProjectWorkspaceSection.story);
      expect(find.text('chrome:story:none'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('set-secondary-story')));
      await tester.pumpAndSettle();
      expect(find.text('chrome:story:details'), findsOneWidget);

      await _tapTab(tester, Revision3ProjectWorkspaceSection.home);
      expect(find.text('chrome:home:none'), findsOneWidget);
      await _tapTab(tester, Revision3ProjectWorkspaceSection.story);
      expect(find.text('chrome:story:details'), findsOneWidget);
    },
  );

  testWidgets('persistent chrome follows compact area selection', (
    tester,
  ) async {
    _setSurface(tester, const Size(360, 480));
    await _pumpWorkspace(
      tester,
      chromeBuilder: (context, location) => SizedBox(
        width: double.infinity,
        child: Text('compact chrome:${location.section.name}'),
      ),
    );

    expect(find.text('compact chrome:home'), findsOneWidget);
    await _selectCompactSection(
      tester,
      Revision3ProjectWorkspaceSection.testRelease,
    );

    expect(find.text('compact chrome:testRelease'), findsOneWidget);
    expect(
      _selectedCompactSection(tester),
      Revision3ProjectWorkspaceSection.testRelease,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('oversized persistent chrome scrolls and preserves page space', (
    tester,
  ) async {
    _setSurface(tester, const Size(360, 240));
    await _pumpWorkspace(
      tester,
      chromeBuilder: (context, location) => const SizedBox(
        height: 240,
        child: Align(
          alignment: Alignment.bottomCenter,
          child: Text('chrome bottom action'),
        ),
      ),
    );

    final chromeScroll = find.byKey(Revision3ProjectWorkspace.chromeScrollKey);
    expect(chromeScroll, findsOneWidget);
    expect(
      tester.getSize(chromeScroll).height,
      lessThan(
        tester
            .getSize(find.byKey(const Key('revision3-project-workspace')))
            .height,
      ),
    );
    expect(find.text('home page / secondary:none'), findsOneWidget);
    expect(tester.takeException(), isNull);

    await tester.drag(chromeScroll, const Offset(0, -220));
    await tester.pumpAndSettle();
    expect(find.text('chrome bottom action').hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('project switch resets an oversized chrome scroll position', (
    tester,
  ) async {
    _setSurface(tester, const Size(360, 300));
    await tester.pumpWidget(
      const MaterialApp(home: _ChromeScrollIdentityHarness()),
    );

    final chromeScroll = find.byKey(Revision3ProjectWorkspace.chromeScrollKey);
    await tester.drag(chromeScroll, const Offset(0, -220));
    await tester.pumpAndSettle();
    expect(_chromeScrollOffset(tester, chromeScroll), greaterThan(0));
    expect(find.text('chrome bottom A').hitTestable(), findsOneWidget);

    await tester.tap(find.byKey(const Key('change-chrome-project')));
    await tester.pumpAndSettle();

    expect(_chromeScrollOffset(tester, chromeScroll), 0);
    expect(find.text('chrome top B').hitTestable(), findsOneWidget);
    expect(find.text('home-B'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('top tabs and page remain overflow-safe at short height', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 300));
    await _pumpWorkspace(tester);

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-project-workspace-tabbar')),
      findsOneWidget,
    );
    for (final section in Revision3ProjectWorkspaceSection.values) {
      expect(find.byKey(_tabKey(section)), findsOneWidget);
    }
    expect(find.text('home page / secondary:none'), findsOneWidget);
  });

  testWidgets(
    'route identity invalidates async handoffs even after navigating back',
    (tester) async {
      _setSurface(tester, const Size(1000, 800));
      late BuildContext retainedContext;
      await _pumpWorkspace(
        tester,
        destinations: _destinations(
          pageBuilder: (section) => (context, location) {
            if (section == Revision3ProjectWorkspaceSection.home) {
              retainedContext = context;
            }
            return Center(
              child: Text(
                '${section.name} page / '
                'secondary:${location.secondary ?? 'none'}',
              ),
            );
          },
        ),
      );

      final initialIdentity = Revision3ProjectWorkspace.navigationIdentityOf(
        retainedContext,
      );
      expect(
        Revision3ProjectWorkspace.currentLocationOf(retainedContext),
        const Revision3ProjectWorkspaceLocation(
          Revision3ProjectWorkspaceSection.home,
        ),
      );

      await _tapTab(tester, Revision3ProjectWorkspaceSection.content);
      final contentIdentity = Revision3ProjectWorkspace.navigationIdentityOf(
        retainedContext,
      );
      expect(identical(contentIdentity, initialIdentity), isFalse);
      expect(
        Revision3ProjectWorkspace.currentLocationOf(retainedContext).section,
        Revision3ProjectWorkspaceSection.content,
      );

      await _tapTab(tester, Revision3ProjectWorkspaceSection.home);
      final returnedIdentity = Revision3ProjectWorkspace.navigationIdentityOf(
        retainedContext,
      );
      expect(identical(returnedIdentity, initialIdentity), isFalse);
      expect(identical(returnedIdentity, contentIdentity), isFalse);
      expect(
        Revision3ProjectWorkspace.currentLocationOf(retainedContext),
        const Revision3ProjectWorkspaceLocation(
          Revision3ProjectWorkspaceSection.home,
        ),
      );
    },
  );

  testWidgets(
    'exact context navigation and primary switches retain secondary route',
    (tester) async {
      _setSurface(tester, const Size(1000, 800));
      await _pumpWorkspace(
        tester,
        destinations: _destinations(
          pageBuilder: (section) =>
              (context, location) => Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      '${section.name} secondary:${location.secondary ?? 'none'}',
                    ),
                    if (section == Revision3ProjectWorkspaceSection.home)
                      FilledButton(
                        key: const Key('open-dataasset-content'),
                        onPressed: () => Revision3ProjectWorkspace.navigate(
                          context,
                          const Revision3ProjectWorkspaceLocation(
                            Revision3ProjectWorkspaceSection.content,
                            secondary: 'data-assets',
                          ),
                        ),
                        child: const Text('Open DataAsset content fixture'),
                      ),
                    if (section == Revision3ProjectWorkspaceSection.content)
                      FilledButton(
                        key: const Key('reset-content-route'),
                        onPressed: () => Revision3ProjectWorkspace.navigate(
                          context,
                          const Revision3ProjectWorkspaceLocation(
                            Revision3ProjectWorkspaceSection.content,
                          ),
                        ),
                        child: const Text('Reset content fixture'),
                      ),
                  ],
                ),
              ),
        ),
      );

      await tester.tap(find.byKey(const Key('open-dataasset-content')));
      await tester.pumpAndSettle();
      expect(find.text('content secondary:data-assets'), findsOneWidget);

      await _tapTab(tester, Revision3ProjectWorkspaceSection.story);
      await _tapTab(tester, Revision3ProjectWorkspaceSection.content);
      expect(find.text('content secondary:data-assets'), findsOneWidget);

      await tester.tap(find.byKey(const Key('reset-content-route')));
      await tester.pumpAndSettle();
      expect(find.text('content secondary:none'), findsOneWidget);
    },
  );

  testWidgets('mounts pages lazily and retains every visited page', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 800));
    final events = <String>[];
    await _pumpWorkspace(
      tester,
      destinations: _destinations(
        pageBuilder: (section) =>
            (context, location) => _LifecyclePage(
              id: section.name,
              events: events,
              location: location,
            ),
      ),
    );

    expect(events, ['init:home']);
    await _tapTab(tester, Revision3ProjectWorkspaceSection.content);
    expect(events, ['init:home', 'init:content']);
    await _tapTab(tester, Revision3ProjectWorkspaceSection.story);
    expect(events, ['init:home', 'init:content', 'init:story']);
    await _tapTab(tester, Revision3ProjectWorkspaceSection.home);
    expect(events.where((event) => event.startsWith('dispose:')), isEmpty);
  });

  testWidgets(
    'same-project revision rebuild preserves selection route and child state',
    (tester) async {
      _setSurface(tester, const Size(1000, 800));
      await tester.pumpWidget(const MaterialApp(home: _RevisionHarness()));

      await tester.tap(find.byKey(const Key('open-content-secondary')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('counter-increment')));
      await tester.pump();
      expect(
        find.text('revision:0 count:1 secondary:data-assets'),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('revision-rebuild')));
      await tester.pump();

      expect(
        find.text('revision:1 count:1 secondary:data-assets'),
        findsOneWidget,
      );
      expect(
        _selectedTabIndex(tester),
        Revision3ProjectWorkspaceSection.content.index,
      );
    },
  );

  testWidgets(
    'changed project identity resets routes and drops mounted state',
    (tester) async {
      _setSurface(tester, const Size(1000, 800));
      final events = <String>[];
      await tester.pumpWidget(
        MaterialApp(home: _IdentityHarness(events: events)),
      );

      await tester.tap(find.byKey(const Key('open-content-secondary')));
      await tester.pumpAndSettle();
      expect(find.text('content-A secondary:data-assets'), findsOneWidget);
      await tester.tap(find.byKey(const Key('chrome-counter-increment')));
      await tester.pump();
      expect(find.text('chrome:content-A:data-assets count:1'), findsOneWidget);

      await tester.tap(find.byKey(const Key('change-project-identity')));
      await tester.pump();

      expect(find.text('home-B secondary:none'), findsOneWidget);
      expect(find.text('content-B secondary:data-assets'), findsNothing);
      expect(events, containsAll(['dispose:home-A', 'dispose:content-A']));
      expect(events, contains('init:home-B'));
      expect(events, isNot(contains('init:content-B')));
      expect(find.text('chrome:home-B:none count:0'), findsOneWidget);
      expect(find.text('chrome:content-A:data-assets count:1'), findsNothing);
      expect(
        _selectedTabIndex(tester),
        Revision3ProjectWorkspaceSection.home.index,
      );
    },
  );
}

void _setSurface(WidgetTester tester, Size size) {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
}

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  List<Revision3ProjectWorkspaceDestination>? destinations,
  Revision3ProjectWorkspaceChromeBuilder? chromeBuilder,
  TextScaler textScaler = TextScaler.noScaling,
}) => tester.pumpWidget(
  MaterialApp(
    home: MediaQuery(
      data: MediaQueryData(textScaler: textScaler),
      child: Scaffold(
        body: Revision3ProjectWorkspace(
          projectIdentity: 'project-fixture',
          destinations: destinations ?? _destinations(),
          chromeBuilder: chromeBuilder,
        ),
      ),
    ),
  ),
);

List<Revision3ProjectWorkspaceDestination> _destinations({
  Revision3ProjectWorkspacePageBuilder Function(
    Revision3ProjectWorkspaceSection section,
  )?
  pageBuilder,
}) => [
  for (final section in Revision3ProjectWorkspaceSection.values)
    Revision3ProjectWorkspaceDestination(
      section: section,
      label: _label(section),
      icon: _icon(section),
      selectedIcon: _selectedIcon(section),
      pageBuilder:
          pageBuilder?.call(section) ??
          (context, location) => Center(
            child: Text(
              '${section.name} page / secondary:${location.secondary ?? 'none'}',
            ),
          ),
    ),
];

Future<void> _tapTab(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section,
) async {
  final tab = find.byKey(_tabKey(section));
  await tester.ensureVisible(tab);
  await tester.pumpAndSettle();
  await tester.tap(tab);
  await tester.pumpAndSettle();
}

int _selectedTabIndex(WidgetTester tester) => tester
    .widget<TabBar>(find.byKey(const Key('revision3-project-workspace-tabbar')))
    .controller!
    .index;

Revision3ProjectWorkspaceSection _selectedCompactSection(WidgetTester tester) =>
    tester
        .widget<PopupMenuButton<Revision3ProjectWorkspaceSection>>(
          find.byKey(const Key('revision3-project-workspace-section-selector')),
        )
        .initialValue!;

Future<void> _selectCompactSection(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section,
) async {
  await tester.tap(
    find.byKey(const Key('revision3-project-workspace-section-selector')),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(_sectionOptionKey(section)));
  await tester.pumpAndSettle();
}

void _expectTabSemantics(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section, {
  required bool selected,
}) {
  final node = tester.getSemantics(find.byKey(_tabKey(section)));
  expect(node.label, startsWith(_label(section)));
  expect(node.role, SemanticsRole.tab);
  expect(node.flagsCollection.isSelected, isNot(Tristate.none));
  expect(
    node.flagsCollection.isSelected,
    selected ? Tristate.isTrue : Tristate.isFalse,
  );
}

Future<bool> _focusCompactSelector(WidgetTester tester) async {
  final selector = find.byKey(
    const Key('revision3-project-workspace-section-selector-semantics'),
  );
  bool hasPrimaryFocus() =>
      Focus.of(tester.element(selector), scopeOk: true).hasPrimaryFocus;

  for (var step = 0; step < 30 && !hasPrimaryFocus(); step++) {
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
  }
  return hasPrimaryFocus();
}

double _chromeScrollOffset(WidgetTester tester, Finder chromeScroll) => tester
    .state<ScrollableState>(
      find.descendant(of: chromeScroll, matching: find.byType(Scrollable)),
    )
    .position
    .pixels;

Key _tabKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-tab-${_sectionKey(section)}');

Key _sectionOptionKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-section-option-${_sectionKey(section)}');

String _sectionKey(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => 'home',
      Revision3ProjectWorkspaceSection.content => 'content',
      Revision3ProjectWorkspaceSection.story => 'story',
      Revision3ProjectWorkspaceSection.textVoice => 'text-voice',
      Revision3ProjectWorkspaceSection.testRelease => 'test-release',
    };

String _label(Revision3ProjectWorkspaceSection section) => switch (section) {
  Revision3ProjectWorkspaceSection.home => 'Home fixture',
  Revision3ProjectWorkspaceSection.content => 'Content Library fixture',
  Revision3ProjectWorkspaceSection.story => 'Story fixture',
  Revision3ProjectWorkspaceSection.textVoice => 'Text & Voice fixture',
  Revision3ProjectWorkspaceSection.testRelease => 'Test & Release fixture',
};

IconData _icon(Revision3ProjectWorkspaceSection section) => switch (section) {
  Revision3ProjectWorkspaceSection.home => Icons.home_outlined,
  Revision3ProjectWorkspaceSection.content => Icons.account_tree_outlined,
  Revision3ProjectWorkspaceSection.story => Icons.auto_stories_outlined,
  Revision3ProjectWorkspaceSection.textVoice => Icons.translate_outlined,
  Revision3ProjectWorkspaceSection.testRelease => Icons.fact_check_outlined,
};

IconData _selectedIcon(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => Icons.home,
      Revision3ProjectWorkspaceSection.content => Icons.account_tree,
      Revision3ProjectWorkspaceSection.story => Icons.auto_stories,
      Revision3ProjectWorkspaceSection.textVoice => Icons.translate,
      Revision3ProjectWorkspaceSection.testRelease => Icons.fact_check,
    };

class _LifecyclePage extends StatefulWidget {
  const _LifecyclePage({
    required this.id,
    required this.events,
    required this.location,
  });

  final String id;
  final List<String> events;
  final Revision3ProjectWorkspaceLocation location;

  @override
  State<_LifecyclePage> createState() => _LifecyclePageState();
}

class _LifecyclePageState extends State<_LifecyclePage> {
  @override
  void initState() {
    super.initState();
    widget.events.add('init:${widget.id}');
  }

  @override
  void dispose() {
    widget.events.add('dispose:${widget.id}');
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Center(
    child: Text(
      '${widget.id} secondary:${widget.location.secondary ?? 'none'}',
    ),
  );
}

class _RevisionHarness extends StatefulWidget {
  const _RevisionHarness();

  @override
  State<_RevisionHarness> createState() => _RevisionHarnessState();
}

class _RevisionHarnessState extends State<_RevisionHarness> {
  int revision = 0;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Column(
      children: [
        FilledButton(
          key: const Key('revision-rebuild'),
          onPressed: () => setState(() => revision++),
          child: const Text('Advance revision fixture'),
        ),
        Expanded(
          child: Revision3ProjectWorkspace(
            projectIdentity: 'stable-project',
            destinations: _destinations(
              pageBuilder: (section) => (context, location) {
                if (section == Revision3ProjectWorkspaceSection.home) {
                  return Center(
                    child: FilledButton(
                      key: const Key('open-content-secondary'),
                      onPressed: () => Revision3ProjectWorkspace.navigate(
                        context,
                        const Revision3ProjectWorkspaceLocation(
                          Revision3ProjectWorkspaceSection.content,
                          secondary: 'data-assets',
                        ),
                      ),
                      child: const Text('Open content fixture'),
                    ),
                  );
                }
                if (section == Revision3ProjectWorkspaceSection.content) {
                  return _CounterPage(
                    revision: revision,
                    secondary: location.secondary,
                  );
                }
                return Center(child: Text(section.name));
              },
            ),
          ),
        ),
      ],
    ),
  );
}

class _CounterPage extends StatefulWidget {
  const _CounterPage({required this.revision, required this.secondary});

  final int revision;
  final String? secondary;

  @override
  State<_CounterPage> createState() => _CounterPageState();
}

class _CounterPageState extends State<_CounterPage> {
  int count = 0;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          'revision:${widget.revision} count:$count '
          'secondary:${widget.secondary ?? 'none'}',
        ),
        FilledButton(
          key: const Key('counter-increment'),
          onPressed: () => setState(() => count++),
          child: const Text('Increment fixture'),
        ),
      ],
    ),
  );
}

class _IdentityHarness extends StatefulWidget {
  const _IdentityHarness({required this.events});

  final List<String> events;

  @override
  State<_IdentityHarness> createState() => _IdentityHarnessState();
}

class _IdentityHarnessState extends State<_IdentityHarness> {
  String project = 'A';

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Column(
      children: [
        FilledButton(
          key: const Key('change-project-identity'),
          onPressed: () => setState(() => project = 'B'),
          child: const Text('Change project fixture'),
        ),
        Expanded(
          child: Revision3ProjectWorkspace(
            projectIdentity: project,
            chromeBuilder: (context, location) =>
                _IdentityChrome(project: project, location: location),
            destinations: _destinations(
              pageBuilder: (section) => (context, location) {
                final id = '${section.name}-$project';
                if (section == Revision3ProjectWorkspaceSection.home) {
                  return _IdentityHomePage(
                    id: id,
                    events: widget.events,
                    location: location,
                  );
                }
                return _LifecyclePage(
                  id: id,
                  events: widget.events,
                  location: location,
                );
              },
            ),
          ),
        ),
      ],
    ),
  );
}

class _IdentityChrome extends StatefulWidget {
  const _IdentityChrome({required this.project, required this.location});

  final String project;
  final Revision3ProjectWorkspaceLocation location;

  @override
  State<_IdentityChrome> createState() => _IdentityChromeState();
}

class _IdentityChromeState extends State<_IdentityChrome> {
  int count = 0;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      Text(
        'chrome:${widget.location.section.name}-${widget.project}:'
        '${widget.location.secondary ?? 'none'} count:$count',
      ),
      IconButton(
        key: const Key('chrome-counter-increment'),
        onPressed: () => setState(() => count++),
        icon: const Icon(Icons.add),
      ),
    ],
  );
}

class _IdentityHomePage extends StatefulWidget {
  const _IdentityHomePage({
    required this.id,
    required this.events,
    required this.location,
  });

  final String id;
  final List<String> events;
  final Revision3ProjectWorkspaceLocation location;

  @override
  State<_IdentityHomePage> createState() => _IdentityHomePageState();
}

class _IdentityHomePageState extends State<_IdentityHomePage> {
  @override
  void initState() {
    super.initState();
    widget.events.add('init:${widget.id}');
  }

  @override
  void dispose() {
    widget.events.add('dispose:${widget.id}');
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('${widget.id} secondary:${widget.location.secondary ?? 'none'}'),
        FilledButton(
          key: const Key('open-content-secondary'),
          onPressed: () => Revision3ProjectWorkspace.navigate(
            context,
            const Revision3ProjectWorkspaceLocation(
              Revision3ProjectWorkspaceSection.content,
              secondary: 'data-assets',
            ),
          ),
          child: const Text('Open content fixture'),
        ),
      ],
    ),
  );
}

class _ChromeScrollIdentityHarness extends StatefulWidget {
  const _ChromeScrollIdentityHarness();

  @override
  State<_ChromeScrollIdentityHarness> createState() =>
      _ChromeScrollIdentityHarnessState();
}

class _ChromeScrollIdentityHarnessState
    extends State<_ChromeScrollIdentityHarness> {
  String project = 'A';

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Column(
      children: [
        FilledButton(
          key: const Key('change-chrome-project'),
          onPressed: () => setState(() => project = 'B'),
          child: const Text('Change chrome project'),
        ),
        Expanded(
          child: Revision3ProjectWorkspace(
            projectIdentity: project,
            chromeBuilder: (context, location) => SizedBox(
              height: 240,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text('chrome top $project'),
                  Text('chrome bottom $project'),
                ],
              ),
            ),
            destinations: _destinations(
              pageBuilder: (section) =>
                  (context, location) =>
                      Center(child: Text('${section.name}-$project')),
            ),
          ),
        ),
      ],
    ),
  );
}
