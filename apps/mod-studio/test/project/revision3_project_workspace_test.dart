import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';

const _overviewLabel = 'Overview fixture';
const _contentLabel = 'Content fixture';
const _dataAssetsLabel = 'DataAssets fixture';

void main() {
  testWidgets('uses a desktop rail and extends it only at wide width', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    await _pumpWorkspace(tester);

    expect(
      find.byKey(const Key('revision3-project-workspace-desktop-navigation')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-workspace-compact-navigation')),
      findsNothing,
    );
    expect(
      tester.widget<NavigationRail>(find.byType(NavigationRail)).extended,
      isFalse,
    );
    expect(find.text('Overview page fixture'), findsOneWidget);
    expect(
      find.byKey(const Key('managed-revision3-overview-tab')),
      findsOneWidget,
    );
    expect(
      tester
          .getSemantics(
            find.byKey(const Key('revision3-project-workspace-page-overview')),
          )
          .label,
      _overviewLabel,
    );

    await _tapRail(tester, Revision3ProjectWorkspaceSection.content);
    expect(
      find.byKey(const Key('managed-revision3-library-tab')),
      findsOneWidget,
    );

    tester.view.physicalSize = const Size(1400, 700);
    await tester.pump();

    expect(
      tester.widget<NavigationRail>(find.byType(NavigationRail)).extended,
      isTrue,
    );
  });

  testWidgets('uses compact bottom navigation and changes sections', (
    tester,
  ) async {
    _setSurface(tester, const Size(600, 700));
    await _pumpWorkspace(tester);

    expect(find.byType(NavigationRail), findsNothing);
    expect(
      find.byKey(const Key('revision3-project-workspace-compact-navigation')),
      findsOneWidget,
    );
    expect(find.text('Overview page fixture'), findsOneWidget);

    await tester.tap(
      find.byKey(const Key('revision3-project-workspace-nav-content')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Content page fixture'), findsOneWidget);
    expect(find.text('Overview page fixture'), findsNothing);
    expect(
      tester.widget<NavigationBar>(find.byType(NavigationBar)).selectedIndex,
      1,
    );
  });

  testWidgets('mounts pages lazily and retains every visited page', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    final events = <String>[];
    await _pumpWorkspace(
      tester,
      overview: _LifecyclePage(id: 'overview', events: events),
      content: _LifecyclePage(id: 'content', events: events),
      dataAssets: _LifecyclePage(id: 'dataAssets', events: events),
    );

    expect(events, ['init:overview']);

    await _tapRail(tester, Revision3ProjectWorkspaceSection.content);
    expect(events, ['init:overview', 'init:content']);

    await _tapRail(tester, Revision3ProjectWorkspaceSection.dataAssets);
    expect(events, ['init:overview', 'init:content', 'init:dataAssets']);

    await _tapRail(tester, Revision3ProjectWorkspaceSection.overview);
    expect(events.where((event) => event.startsWith('dispose:')), isEmpty);
  });

  testWidgets(
    'same project revision rebuild preserves selection and child state',
    (tester) async {
      _setSurface(tester, const Size(1000, 700));
      await tester.pumpWidget(const MaterialApp(home: _RevisionHarness()));

      await _tapRail(tester, Revision3ProjectWorkspaceSection.content);
      await tester.tap(find.byKey(const Key('counter-increment')));
      await tester.pump();
      expect(find.text('revision:0 count:1'), findsOneWidget);

      await tester.tap(find.byKey(const Key('revision-rebuild')));
      await tester.pump();

      expect(find.text('revision:1 count:1'), findsOneWidget);
      expect(
        tester
            .widget<NavigationRail>(find.byType(NavigationRail))
            .selectedIndex,
        Revision3ProjectWorkspaceSection.content.index,
      );

      await _tapRail(tester, Revision3ProjectWorkspaceSection.overview);
      await _tapRail(tester, Revision3ProjectWorkspaceSection.content);
      expect(find.text('revision:1 count:1'), findsOneWidget);
    },
  );

  testWidgets('changed project identity resets and drops mounted page state', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    final events = <String>[];
    await tester.pumpWidget(
      MaterialApp(home: _IdentityHarness(events: events)),
    );

    await _tapRail(tester, Revision3ProjectWorkspaceSection.content);
    expect(events, containsAll(['init:overview-A', 'init:content-A']));

    await tester.tap(find.byKey(const Key('change-project-identity')));
    await tester.pump();

    expect(find.text('overview-B'), findsOneWidget);
    expect(find.text('content-B'), findsNothing);
    expect(events, contains('dispose:overview-A'));
    expect(events, contains('dispose:content-A'));
    expect(events, contains('init:overview-B'));
    expect(events, isNot(contains('init:content-B')));
    expect(
      tester.widget<NavigationRail>(find.byType(NavigationRail)).selectedIndex,
      Revision3ProjectWorkspaceSection.overview.index,
    );
  });

  testWidgets('descendant context can navigate to a workspace section', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    await _pumpWorkspace(
      tester,
      overview: Builder(
        builder: (context) => Center(
          child: FilledButton(
            key: const Key('overview-open-content'),
            onPressed: () => Revision3ProjectWorkspace.navigate(
              context,
              Revision3ProjectWorkspaceSection.content,
            ),
            child: const Text('Open content fixture'),
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('overview-open-content')));
    await tester.pumpAndSettle();

    expect(find.text('Content page fixture'), findsOneWidget);
    expect(
      tester.widget<NavigationRail>(find.byType(NavigationRail)).selectedIndex,
      Revision3ProjectWorkspaceSection.content.index,
    );
  });
}

void _setSurface(WidgetTester tester, Size size) {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
}

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  Widget overview = const Center(child: Text('Overview page fixture')),
  Widget content = const Center(child: Text('Content page fixture')),
  Widget dataAssets = const Center(child: Text('DataAssets page fixture')),
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3ProjectWorkspace(
        projectIdentity: 'project-fixture',
        overviewLabel: _overviewLabel,
        contentLabel: _contentLabel,
        dataAssetsLabel: _dataAssetsLabel,
        overview: overview,
        content: content,
        dataAssets: dataAssets,
      ),
    ),
  ),
);

Future<void> _tapRail(
  WidgetTester tester,
  Revision3ProjectWorkspaceSection section,
) async {
  final suffix = switch (section) {
    Revision3ProjectWorkspaceSection.overview => 'overview',
    Revision3ProjectWorkspaceSection.content => 'content',
    Revision3ProjectWorkspaceSection.dataAssets => 'data-assets',
  };
  final unselected = find.byKey(
    Key('revision3-project-workspace-rail-$suffix-icon'),
  );
  final selected = find.byKey(
    Key('revision3-project-workspace-rail-$suffix-selected-icon'),
  );
  await tester.tap(unselected.evaluate().isNotEmpty ? unselected : selected);
  await tester.pumpAndSettle();
}

class _LifecyclePage extends StatefulWidget {
  const _LifecyclePage({required this.id, required this.events});

  final String id;
  final List<String> events;

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
  Widget build(BuildContext context) => Center(child: Text(widget.id));
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
          child: const Text('Advance fixture revision'),
        ),
        Expanded(
          child: Revision3ProjectWorkspace(
            projectIdentity: 'stable-project',
            overviewLabel: _overviewLabel,
            contentLabel: _contentLabel,
            dataAssetsLabel: _dataAssetsLabel,
            overview: const Center(child: Text('Stable overview fixture')),
            content: _CounterPage(revision: revision),
            dataAssets: const Center(child: Text('Stable DataAssets fixture')),
          ),
        ),
      ],
    ),
  );
}

class _CounterPage extends StatefulWidget {
  const _CounterPage({required this.revision});

  final int revision;

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
        Text('revision:${widget.revision} count:$count'),
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
          child: const Text('Change fixture project'),
        ),
        Expanded(
          child: Revision3ProjectWorkspace(
            projectIdentity: project,
            overviewLabel: _overviewLabel,
            contentLabel: _contentLabel,
            dataAssetsLabel: _dataAssetsLabel,
            overview: _LifecyclePage(
              id: 'overview-$project',
              events: widget.events,
            ),
            content: _LifecyclePage(
              id: 'content-$project',
              events: widget.events,
            ),
            dataAssets: _LifecyclePage(
              id: 'dataAssets-$project',
              events: widget.events,
            ),
          ),
        ),
      ],
    ),
  );
}
