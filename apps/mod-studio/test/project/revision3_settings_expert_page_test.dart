import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_settings_expert_page.dart';

const _title = 'Settings destination fixture';
const _description = 'Configure the fixture without opening another surface.';
const _statusLabel = 'Expert fixture status';
const _statusDescription =
    'The fixture reports its boundary without granting extra behavior.';

void main() {
  testWidgets('renders injected copy and the supplied settings surface', (
    tester,
  ) async {
    _setSurface(tester, const Size(1000, 700));
    await _pumpPage(tester);

    expect(
      find.byKey(const Key('revision3-settings-expert-page')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-settings-expert-page-header')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-settings-expert-page-status')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-settings-expert-page-settings')),
      findsOneWidget,
    );
    expect(find.text(_title), findsOneWidget);
    expect(find.text(_description), findsOneWidget);
    expect(find.text(_statusLabel), findsOneWidget);
    expect(find.text(_statusDescription), findsOneWidget);
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
    expect(find.byIcon(Icons.settings_outlined), findsOneWidget);
    expect(find.byIcon(Icons.science_outlined), findsOneWidget);
    expect(find.byType(Dialog), findsNothing);
  });

  testWidgets(
    'marks the title as a heading and status as a semantic container',
    (tester) async {
      _setSurface(tester, const Size(900, 600));
      await _pumpPage(tester);

      final titleSemantics = tester.widget<Semantics>(
        find.byKey(const Key('revision3-settings-expert-page-title')),
      );
      final statusSemantics = tester.widget<Semantics>(
        find.byKey(const Key('revision3-settings-expert-page-status')),
      );

      expect(titleSemantics.container, isTrue);
      expect(titleSemantics.properties.header, isTrue);
      expect(statusSemantics.container, isTrue);
      expect(statusSemantics.explicitChildNodes, isTrue);
    },
  );

  testWidgets('narrow layout keeps long injected copy and settings usable', (
    tester,
  ) async {
    _setSurface(tester, const Size(320, 480));
    await _pumpPage(
      tester,
      title: 'A deliberately long injected settings destination title fixture',
      description:
          'A deliberately long injected description that wraps across several lines in a narrow window while the owned settings surface remains independent below it.',
      statusLabel: 'A deliberately long injected expert status label fixture',
      statusDescription:
          'A deliberately long injected expert status explanation that remains scrollable and never claims unavailable behavior.',
    );

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-settings-expert-page-settings')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('settings-fixture-child')), findsOneWidget);
    expect(
      tester
          .getSize(
            find.byKey(const Key('revision3-settings-expert-page-settings')),
          )
          .height,
      greaterThan(0),
    );
  });

  testWidgets('short layout constrains a scrollable header above settings', (
    tester,
  ) async {
    _setSurface(tester, const Size(800, 180));
    await _pumpPage(
      tester,
      description:
          'Injected description fixture with enough text to require the bounded header to scroll at a very short window height.',
      statusDescription:
          'Injected status fixture with enough text to remain honest while the settings child keeps most of the available height.',
    );

    expect(tester.takeException(), isNull);
    final headerHeight = tester
        .getSize(find.byKey(const Key('revision3-settings-expert-page-header')))
        .height;
    final settingsHeight = tester
        .getSize(
          find.byKey(const Key('revision3-settings-expert-page-settings')),
        )
        .height;
    expect(headerHeight, lessThanOrEqualTo(81));
    expect(settingsHeight, greaterThan(headerHeight));
    expect(
      find.byKey(const Key('revision3-settings-expert-page-header-scroll')),
      findsOneWidget,
    );
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
  String title = _title,
  String description = _description,
  String statusLabel = _statusLabel,
  String statusDescription = _statusDescription,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Revision3SettingsExpertPage(
        title: title,
        description: description,
        expertStatusLabel: statusLabel,
        expertStatusDescription: statusDescription,
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
      ),
    ),
  ),
);
