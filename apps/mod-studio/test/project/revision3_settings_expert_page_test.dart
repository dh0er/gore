import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';
import 'package:gore_mod/project/revision3_settings_expert_page.dart';

const _settingsLabel = 'Project settings fixture';
const _dataAssetLabLabel = 'DataAsset Lab fixture';

void main() {
  testWidgets('starts directly with navigation and the settings surface', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    await _pumpPage(tester);

    expect(
      find.byKey(const Key('revision3-settings-expert-page')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-settings-expert-page-navigation')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-settings-expert-page-settings')),
      findsOneWidget,
    );
    expect(
      find.byKey(
        const Key('revision3-settings-expert-page-data-asset-lab'),
        skipOffstage: false,
      ),
      findsNothing,
    );
    expect(find.text(_settingsLabel), findsOneWidget);
    expect(find.text(_dataAssetLabLabel), findsOneWidget);
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
    expect(find.byKey(const Key('data-asset-lab-fixture-child')), findsNothing);
    expect(_selectedView(tester), Revision3SettingsExpertView.settings);
    expect(find.byType(Dialog), findsNothing);
  });

  testWidgets('exact Lab deep link mounts only the Lab page', (tester) async {
    _setSurface(tester, const Size(900, 600));
    await _pumpPage(
      tester,
      secondary: Revision3SettingsExpertView.dataAssetLab.secondaryRoute,
    );

    expect(_selectedView(tester), Revision3SettingsExpertView.dataAssetLab);
    expect(
      find.byKey(const Key('revision3-settings-expert-page-data-asset-lab')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('data-asset-lab-fixture-child')),
      findsOneWidget,
    );
    expect(
      find.byKey(
        const Key('revision3-settings-expert-page-settings'),
        skipOffstage: false,
      ),
      findsNothing,
    );
    expect(
      find.byKey(const Key('settings-fixture-child'), skipOffstage: false),
      findsNothing,
    );
  });

  testWidgets('null and unknown secondary routes fail closed to Settings', (
    tester,
  ) async {
    _setSurface(tester, const Size(900, 600));
    await _pumpPage(tester, secondary: 'future-unknown-route');

    expect(_selectedView(tester), Revision3SettingsExpertView.settings);
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
    expect(
      find.byKey(
        const Key('data-asset-lab-fixture-child'),
        skipOffstage: false,
      ),
      findsNothing,
    );

    await _pumpPage(tester);
    expect(_selectedView(tester), Revision3SettingsExpertView.settings);
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
  });

  testWidgets(
    'segmented navigation owns the parent route and retains both lazy pages',
    (tester) async {
      _setSurface(tester, const Size(1000, 700));
      await tester.pumpWidget(const _WorkspaceHarness());

      await tester.tap(find.byKey(const Key('open-settings-expert')));
      await tester.pumpAndSettle();
      expect(find.text('SETTINGS STATE:0'), findsOneWidget);
      expect(find.text('LAB STATE:0', skipOffstage: false), findsNothing);

      await tester.tap(find.byKey(const Key('settings-state-increment')));
      await tester.pump();
      expect(find.text('SETTINGS STATE:1'), findsOneWidget);

      await tester.tap(
        find.byKey(
          const Key('revision3-settings-expert-page-nav-data-asset-lab'),
        ),
      );
      await tester.pumpAndSettle();
      expect(_selectedView(tester), Revision3SettingsExpertView.dataAssetLab);
      expect(find.text('LAB STATE:0'), findsOneWidget);
      expect(
        find.text('SETTINGS STATE:1', skipOffstage: false),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('lab-state-increment')));
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-settings-expert-page-nav-settings')),
      );
      await tester.pumpAndSettle();

      expect(_selectedView(tester), Revision3SettingsExpertView.settings);
      expect(find.text('SETTINGS STATE:1'), findsOneWidget);
      expect(find.text('LAB STATE:1', skipOffstage: false), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('compact 200 percent text scrolls navigation without overflow', (
    tester,
  ) async {
    _setSurface(tester, const Size(320, 480));
    await _pumpPage(
      tester,
      textScaler: const TextScaler.linear(2),
      settingsLabel:
          'A deliberately long project settings navigation label fixture',
      dataAssetLabLabel:
          'A deliberately long DataAsset Lab navigation label fixture',
    );

    expect(tester.takeException(), isNull);
    final navigationScroll = find.byKey(
      const Key('revision3-settings-expert-page-navigation-scroll'),
    );
    expect(navigationScroll, findsOneWidget);
    expect(_scrollOffsetLimit(tester, navigationScroll), greaterThan(0));
    expect(
      tester
          .getSize(
            find.byKey(const Key('revision3-settings-expert-page-settings')),
          )
          .height,
      greaterThan(0),
    );
  });

  testWidgets('very short layout gives the active tool the remaining space', (
    tester,
  ) async {
    _setSurface(tester, const Size(800, 180));
    await _pumpPage(tester);

    expect(tester.takeException(), isNull);
    final navigationHeight = tester
        .getSize(
          find.byKey(
            const Key('revision3-settings-expert-page-navigation-scroll'),
          ),
        )
        .height;
    final settingsHeight = tester
        .getSize(
          find.byKey(const Key('revision3-settings-expert-page-settings')),
        )
        .height;
    expect(navigationHeight, lessThan(settingsHeight));
    expect(settingsHeight, greaterThan(100));
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
  });
}

void _setSurface(WidgetTester tester, Size size) {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
}

Future<void> _pumpPage(
  WidgetTester tester, {
  String? secondary,
  String settingsLabel = _settingsLabel,
  String dataAssetLabLabel = _dataAssetLabLabel,
  TextScaler textScaler = TextScaler.noScaling,
}) => tester.pumpWidget(
  MaterialApp(
    home: MediaQuery(
      data: MediaQueryData(textScaler: textScaler),
      child: Scaffold(
        body: Revision3SettingsExpertPage(
          location: Revision3ProjectWorkspaceLocation(
            Revision3ProjectWorkspaceSection.settingsExpert,
            secondary: secondary,
          ),
          settingsLabel: settingsLabel,
          dataAssetLabLabel: dataAssetLabLabel,
          settings: ListView(
            key: const Key('settings-fixture-child'),
            children: const [
              ListTile(
                key: Key('settings-fixture-control'),
                leading: Icon(Icons.videogame_asset_outlined),
                title: Text('Injected settings child fixture'),
              ),
            ],
          ),
          dataAssetLab: ListView(
            key: const Key('data-asset-lab-fixture-child'),
            children: const [
              ListTile(title: Text('Injected DataAsset Lab child fixture')),
            ],
          ),
        ),
      ),
    ),
  ),
);

Revision3SettingsExpertView _selectedView(WidgetTester tester) => tester
    .widget<SegmentedButton<Revision3SettingsExpertView>>(
      find.byKey(const Key('revision3-settings-expert-page-navigation')),
    )
    .selected
    .single;

double _scrollOffsetLimit(WidgetTester tester, Finder scroll) => tester
    .state<ScrollableState>(
      find.descendant(of: scroll, matching: find.byType(Scrollable)),
    )
    .position
    .maxScrollExtent;

class _WorkspaceHarness extends StatelessWidget {
  const _WorkspaceHarness();

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3ProjectWorkspace(
        projectIdentity: 'settings-expert-project',
        destinations: [
          for (final section in Revision3ProjectWorkspaceSection.values)
            Revision3ProjectWorkspaceDestination(
              section: section,
              label: section.name,
              icon: Icons.circle_outlined,
              selectedIcon: Icons.circle,
              pageBuilder: (context, location) => switch (section) {
                Revision3ProjectWorkspaceSection.home => Center(
                  child: FilledButton(
                    key: const Key('open-settings-expert'),
                    onPressed: () => Revision3ProjectWorkspace.navigate(
                      context,
                      const Revision3ProjectWorkspaceLocation(
                        Revision3ProjectWorkspaceSection.settingsExpert,
                      ),
                    ),
                    child: const Text('OPEN SETTINGS EXPERT'),
                  ),
                ),
                Revision3ProjectWorkspaceSection.settingsExpert =>
                  Revision3SettingsExpertPage(
                    location: location,
                    settingsLabel: 'SETTINGS VIEW',
                    dataAssetLabLabel: 'DATAASSET LAB VIEW',
                    settings: const _CounterSurface(
                      label: 'SETTINGS',
                      buttonKey: Key('settings-state-increment'),
                    ),
                    dataAssetLab: const _CounterSurface(
                      label: 'LAB',
                      buttonKey: Key('lab-state-increment'),
                    ),
                  ),
                _ => const SizedBox.shrink(),
              },
            ),
        ],
      ),
    ),
  );
}

class _CounterSurface extends StatefulWidget {
  const _CounterSurface({required this.label, required this.buttonKey});

  final String label;
  final Key buttonKey;

  @override
  State<_CounterSurface> createState() => _CounterSurfaceState();
}

class _CounterSurfaceState extends State<_CounterSurface> {
  int _count = 0;

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('${widget.label} STATE:$_count'),
        FilledButton(
          key: widget.buttonKey,
          onPressed: () => setState(() => _count++),
          child: const Text('Increment fixture'),
        ),
      ],
    ),
  );
}
