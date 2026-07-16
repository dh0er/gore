import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';

void main() {
  test('requires every destination once in canonical order', () {
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
    'wide shell exposes all canonical destinations in an extended rail',
    (tester) async {
      _setSurface(tester, const Size(1400, 900));
      await _pumpWorkspace(tester);

      expect(
        find.byKey(const Key('revision3-project-workspace-desktop-navigation')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-project-workspace-narrow-navigation')),
        findsNothing,
      );
      expect(
        tester.widget<NavigationRail>(find.byType(NavigationRail)).extended,
        isTrue,
      );
      expect(
        tester
            .widget<NavigationRail>(find.byType(NavigationRail))
            .selectedIndex,
        Revision3ProjectWorkspaceSection.home.index,
      );
      for (final section in Revision3ProjectWorkspaceSection.values) {
        expect(find.byKey(_navigationKey(section)), findsOneWidget);
      }
      expect(find.text('home page / secondary:none'), findsOneWidget);
    },
  );

  testWidgets('medium shell uses an icon rail with labels as tooltips', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 900));
    await _pumpWorkspace(tester);

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.extended, isFalse);
    for (final destination in _destinations()) {
      expect(find.byKey(_navigationKey(destination.section)), findsOneWidget);
      expect(find.byTooltip(destination.label), findsOneWidget);
    }

    await _tapDesktopSection(tester, Revision3ProjectWorkspaceSection.story);
    expect(find.text('story page / secondary:none'), findsOneWidget);
    expect(
      tester.widget<NavigationRail>(find.byType(NavigationRail)).selectedIndex,
      Revision3ProjectWorkspaceSection.story.index,
    );
  });

  testWidgets('narrow shell menu exposes and selects all nine destinations', (
    tester,
  ) async {
    _setSurface(tester, const Size(600, 700));
    await _pumpWorkspace(tester);

    expect(find.byType(NavigationRail), findsNothing);
    expect(
      find.byKey(const Key('revision3-project-workspace-narrow-navigation')),
      findsOneWidget,
    );
    expect(find.text('Home fixture'), findsOneWidget);
    expect(find.text('home page / secondary:none'), findsOneWidget);

    await tester.tap(
      find.byKey(const Key('revision3-project-workspace-narrow-menu')),
    );
    await tester.pumpAndSettle();

    for (final section in Revision3ProjectWorkspaceSection.values) {
      expect(find.byKey(_navigationKey(section)), findsOneWidget);
    }

    await tester.tap(
      find.byKey(
        _navigationKey(Revision3ProjectWorkspaceSection.localizationVoice),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text('localizationVoice page / secondary:none'),
      findsOneWidget,
    );
    expect(find.text('Localization & Voice fixture'), findsOneWidget);
  });

  testWidgets('desktop rail remains scroll-safe at short height', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 300));
    await _pumpWorkspace(tester);

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(
        const Key('revision3-project-workspace-desktop-navigation-scroll'),
      ),
      findsOneWidget,
    );
    expect(
      find.byKey(
        _navigationKey(Revision3ProjectWorkspaceSection.settingsExpert),
      ),
      findsOneWidget,
    );
  });

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

      await _tapDesktopSection(tester, Revision3ProjectWorkspaceSection.story);
      await _tapDesktopSection(
        tester,
        Revision3ProjectWorkspaceSection.content,
      );
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
    await _tapDesktopSection(tester, Revision3ProjectWorkspaceSection.content);
    expect(events, ['init:home', 'init:content']);
    await _tapDesktopSection(tester, Revision3ProjectWorkspaceSection.story);
    expect(events, ['init:home', 'init:content', 'init:story']);
    await _tapDesktopSection(tester, Revision3ProjectWorkspaceSection.home);
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
        tester
            .widget<NavigationRail>(find.byType(NavigationRail))
            .selectedIndex,
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

      await tester.tap(find.byKey(const Key('change-project-identity')));
      await tester.pump();

      expect(find.text('home-B secondary:none'), findsOneWidget);
      expect(find.text('content-B secondary:data-assets'), findsNothing);
      expect(events, containsAll(['dispose:home-A', 'dispose:content-A']));
      expect(events, contains('init:home-B'));
      expect(events, isNot(contains('init:content-B')));
      expect(
        tester
            .widget<NavigationRail>(find.byType(NavigationRail))
            .selectedIndex,
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
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3ProjectWorkspace(
        projectIdentity: 'project-fixture',
        destinations: destinations ?? _destinations(),
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

Future<void> _tapDesktopSection(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section,
) async {
  await tester.tap(find.byKey(_navigationKey(section)));
  await tester.pumpAndSettle();
}

Key _navigationKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-nav-${_sectionKey(section)}');

String _sectionKey(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => 'home',
      Revision3ProjectWorkspaceSection.content => 'content',
      Revision3ProjectWorkspaceSection.story => 'story',
      Revision3ProjectWorkspaceSection.world => 'world',
      Revision3ProjectWorkspaceSection.localizationVoice =>
        'localization-voice',
      Revision3ProjectWorkspaceSection.validateTest => 'validate-test',
      Revision3ProjectWorkspaceSection.buildRelease => 'build-release',
      Revision3ProjectWorkspaceSection.history => 'history',
      Revision3ProjectWorkspaceSection.settingsExpert => 'settings-expert',
    };

String _label(Revision3ProjectWorkspaceSection section) => switch (section) {
  Revision3ProjectWorkspaceSection.home => 'Home fixture',
  Revision3ProjectWorkspaceSection.content => 'Content Library fixture',
  Revision3ProjectWorkspaceSection.story => 'Story fixture',
  Revision3ProjectWorkspaceSection.world => 'World fixture',
  Revision3ProjectWorkspaceSection.localizationVoice =>
    'Localization & Voice fixture',
  Revision3ProjectWorkspaceSection.validateTest => 'Validate & Test fixture',
  Revision3ProjectWorkspaceSection.buildRelease => 'Build & Release fixture',
  Revision3ProjectWorkspaceSection.history => 'History fixture',
  Revision3ProjectWorkspaceSection.settingsExpert =>
    'Settings / Expert fixture',
};

IconData _icon(Revision3ProjectWorkspaceSection section) => switch (section) {
  Revision3ProjectWorkspaceSection.home => Icons.home_outlined,
  Revision3ProjectWorkspaceSection.content => Icons.account_tree_outlined,
  Revision3ProjectWorkspaceSection.story => Icons.auto_stories_outlined,
  Revision3ProjectWorkspaceSection.world => Icons.public_outlined,
  Revision3ProjectWorkspaceSection.localizationVoice =>
    Icons.translate_outlined,
  Revision3ProjectWorkspaceSection.validateTest => Icons.fact_check_outlined,
  Revision3ProjectWorkspaceSection.buildRelease => Icons.inventory_2_outlined,
  Revision3ProjectWorkspaceSection.history => Icons.history_outlined,
  Revision3ProjectWorkspaceSection.settingsExpert => Icons.settings_outlined,
};

IconData _selectedIcon(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => Icons.home,
      Revision3ProjectWorkspaceSection.content => Icons.account_tree,
      Revision3ProjectWorkspaceSection.story => Icons.auto_stories,
      Revision3ProjectWorkspaceSection.world => Icons.public,
      Revision3ProjectWorkspaceSection.localizationVoice => Icons.translate,
      Revision3ProjectWorkspaceSection.validateTest => Icons.fact_check,
      Revision3ProjectWorkspaceSection.buildRelease => Icons.inventory_2,
      Revision3ProjectWorkspaceSection.history => Icons.history,
      Revision3ProjectWorkspaceSection.settingsExpert => Icons.settings,
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
