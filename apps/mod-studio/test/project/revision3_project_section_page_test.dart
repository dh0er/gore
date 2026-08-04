import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_section_page.dart';

void main() {
  testWidgets('renders only injected copy with stable section keys', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    _setSurface(tester, const Size(820, 720));

    await _pumpPage(
      tester,
      Revision3ProjectSectionPage(
        sectionId: 'welt-werkzeuge',
        icon: Icons.public_outlined,
        title: 'Welt-Werkzeuge',
        description: 'Baue Orte aus geprüften Bausteinen.',
        notice: 'Diese Ansicht verändert noch keine Spieldateien.',
        statusHeading: 'Aktueller Überblick',
        actionHeading: 'Nächste Schritte',
        statusCards: [
          Revision3ProjectSectionStatusCard(
            id: 'geladene-orte',
            icon: Icons.map_outlined,
            title: 'Geladene Orte',
            description: 'In diesem Projekt sichtbar.',
            valueText: '12 Orte',
          ),
        ],
        actionCards: [
          Revision3ProjectSectionActionCard(
            id: 'ort-anlegen',
            icon: Icons.add_location_alt_outlined,
            title: 'Ort anlegen',
            description: 'Öffnet den geführten Entwurf.',
            badge: 'Vorschau',
          ),
        ],
      ),
    );

    for (final copy in [
      'Welt-Werkzeuge',
      'Baue Orte aus geprüften Bausteinen.',
      'Diese Ansicht verändert noch keine Spieldateien.',
      'Aktueller Überblick',
      'Geladene Orte',
      'In diesem Projekt sichtbar.',
      '12 Orte',
      'Nächste Schritte',
      'Ort anlegen',
      'Öffnet den geführten Entwurf.',
      'Vorschau',
    ]) {
      expect(find.text(copy), findsOneWidget);
    }
    for (final key in [
      'revision3-project-section-welt-werkzeuge-page',
      'revision3-project-section-welt-werkzeuge-header',
      'revision3-project-section-welt-werkzeuge-notice',
      'revision3-project-section-welt-werkzeuge-status-geladene-orte',
      'revision3-project-section-welt-werkzeuge-action-ort-anlegen',
    ]) {
      expect(find.byKey(Key(key)), findsOneWidget);
    }
    expect(
      tester.getSemantics(find.text('Welt-Werkzeuge')),
      matchesSemantics(label: 'Welt-Werkzeuge', isHeader: true),
    );
    expect(tester.takeException(), isNull);
    semantics.dispose();
  });

  testWidgets('uses one, two, and three columns without overflow', (
    tester,
  ) async {
    _setSurface(tester, const Size(280, 700));
    final page = _responsiveFixturePage();

    await _pumpPage(tester, page);
    var positions = _actionPositions(tester);
    expect(positions[0].dx, moreOrLessEquals(positions[1].dx));
    expect(positions[1].dx, moreOrLessEquals(positions[2].dx));
    expect(positions[0].dy, lessThan(positions[1].dy));
    expect(positions[1].dy, lessThan(positions[2].dy));
    expect(tester.takeException(), isNull);

    tester.view.physicalSize = const Size(820, 700);
    await tester.pumpWidget(_host(page));
    await tester.pump();
    positions = _actionPositions(tester);
    expect(positions[0].dy, moreOrLessEquals(positions[1].dy));
    expect(positions[0].dx, lessThan(positions[1].dx));
    expect(positions[2].dy, greaterThan(positions[1].dy));
    expect(tester.takeException(), isNull);

    tester.view.physicalSize = const Size(1400, 700);
    await tester.pumpWidget(_host(page));
    await tester.pump();
    positions = _actionPositions(tester);
    expect(positions[0].dy, moreOrLessEquals(positions[1].dy));
    expect(positions[1].dy, moreOrLessEquals(positions[2].dy));
    expect(positions[0].dx, lessThan(positions[1].dx));
    expect(positions[1].dx, lessThan(positions[2].dx));
    expect(
      tester
          .getSize(
            find.byKey(
              const Key('revision3-project-section-layout-fixture-page'),
            ),
          )
          .width,
      lessThanOrEqualTo(1280),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('short viewport scrolls to every visible disabled action', (
    tester,
  ) async {
    _setSurface(tester, const Size(360, 220));
    await _pumpPage(tester, _responsiveFixturePage());

    final lastAction = find.byKey(
      const Key('revision3-project-section-layout-fixture-action-third'),
    );
    expect(find.byType(SingleChildScrollView), findsOneWidget);
    expect(lastAction, findsOneWidget);
    await tester.ensureVisible(lastAction);
    await tester.pump();
    expect(
      tester.getRect(lastAction).overlaps(const Rect.fromLTWH(0, 0, 360, 220)),
      isTrue,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'enabled action invokes callback and disabled action is semantic',
    (tester) async {
      final semantics = tester.ensureSemantics();
      _setSurface(tester, const Size(760, 620));
      var invocationCount = 0;
      await _pumpPage(
        tester,
        Revision3ProjectSectionPage(
          sectionId: 'action-fixture',
          icon: Icons.bolt_outlined,
          title: 'Action title fixture',
          description: 'Action description fixture',
          actionCards: [
            Revision3ProjectSectionActionCard(
              id: 'enabled',
              icon: Icons.play_arrow_outlined,
              title: 'Run fixture',
              description: 'Invokes the fixture callback',
              badge: 'Ready fixture',
              onPressed: () => invocationCount++,
            ),
            Revision3ProjectSectionActionCard(
              id: 'disabled',
              icon: Icons.lock_outline,
              title: 'Unavailable fixture',
              description: 'Remains discoverable in the fixture',
            ),
          ],
        ),
      );

      final enabled = find.byKey(
        const Key('revision3-project-section-action-fixture-action-enabled'),
      );
      final disabled = find.byKey(
        const Key('revision3-project-section-action-fixture-action-disabled'),
      );
      expect(
        tester.getSemantics(enabled),
        matchesSemantics(
          label: 'Run fixture',
          hint: 'Invokes the fixture callback',
          value: 'Ready fixture',
          isButton: true,
          hasEnabledState: true,
          isEnabled: true,
          hasTapAction: true,
        ),
      );
      expect(
        tester.getSemantics(disabled),
        matchesSemantics(
          label: 'Unavailable fixture',
          hint: 'Remains discoverable in the fixture',
          isButton: true,
          hasEnabledState: true,
        ),
      );
      expect(find.text('Unavailable fixture'), findsOneWidget);

      await tester.tap(enabled);
      await tester.pump();
      expect(invocationCount, 1);
      await tester.tap(disabled);
      await tester.pump();
      expect(invocationCount, 1);
      semantics.dispose();
    },
  );

  testWidgets('status severity selects distinct Material color roles', (
    tester,
  ) async {
    _setSurface(tester, const Size(1400, 800));
    await _pumpPage(
      tester,
      Revision3ProjectSectionPage(
        sectionId: 'severity-fixture',
        icon: Icons.palette_outlined,
        title: 'Severity title fixture',
        description: 'Severity description fixture',
        statusCards: [
          for (final severity in Revision3ProjectSectionStatusSeverity.values)
            Revision3ProjectSectionStatusCard(
              id: severity.name,
              icon: Icons.circle_outlined,
              title: '${severity.name} fixture',
              description: 'Status color fixture',
              severity: severity,
            ),
        ],
      ),
    );

    final context = tester.element(
      find.byKey(const Key('revision3-project-section-severity-fixture-page')),
    );
    final scheme = Theme.of(context).colorScheme;
    expect(_statusColor(tester, 'neutral'), scheme.surfaceContainerHighest);
    expect(_statusColor(tester, 'success'), scheme.primaryContainer);
    expect(_statusColor(tester, 'warning'), scheme.tertiaryContainer);
    expect(_statusColor(tester, 'blocked'), scheme.errorContainer);
  });

  test('constructors validate IDs and defensively copy card lists', () {
    final statuses = [_status('first')];
    final actions = [_action('open-first')];
    final page = Revision3ProjectSectionPage(
      sectionId: 'validation-fixture',
      icon: Icons.check_outlined,
      title: 'Validation title fixture',
      description: 'Validation description fixture',
      statusCards: statuses,
      actionCards: actions,
    );

    statuses.add(_status('second'));
    actions.add(_action('open-second'));
    expect(page.statusCards, hasLength(1));
    expect(page.actionCards, hasLength(1));
    expect(() => page.statusCards.clear(), throwsUnsupportedError);
    expect(() => page.actionCards.clear(), throwsUnsupportedError);

    expect(
      () => Revision3ProjectSectionPage(
        sectionId: 'duplicate-fixture',
        icon: Icons.copy_outlined,
        title: 'Duplicate title fixture',
        description: 'Duplicate description fixture',
        statusCards: [_status('same'), _status('same')],
      ),
      throwsAssertionError,
    );
    expect(
      () => Revision3ProjectSectionPage(
        sectionId: 'Not Kebab Safe',
        icon: Icons.error_outline,
        title: 'Invalid title fixture',
        description: 'Invalid description fixture',
      ),
      throwsAssertionError,
    );
    expect(() => _status(''), throwsAssertionError);
    expect(() => _action('Not-Kebab'), throwsAssertionError);
  });
}

Revision3ProjectSectionPage _responsiveFixturePage() =>
    Revision3ProjectSectionPage(
      sectionId: 'layout-fixture',
      icon: Icons.space_dashboard_outlined,
      title: 'A deliberately long injected section title fixture',
      description:
          'A deliberately long injected description that wraps safely on a '
          'narrow presentation canvas without owning application behavior.',
      notice:
          'A long injected notice remains readable at every supported width.',
      actionHeading: 'Injected action heading fixture',
      actionCards: [
        _action('first', badge: 'Long injected badge fixture'),
        _action('second'),
        _action('third'),
      ],
    );

Revision3ProjectSectionStatusCard _status(String id) =>
    Revision3ProjectSectionStatusCard(
      id: id,
      icon: Icons.info_outline,
      title: '$id status fixture',
      description: 'Status description fixture',
    );

Revision3ProjectSectionActionCard _action(String id, {String? badge}) =>
    Revision3ProjectSectionActionCard(
      id: id,
      icon: Icons.arrow_forward_outlined,
      title: '$id action fixture',
      description:
          'A longer injected action description fixture that remains visible.',
      badge: badge,
    );

List<Offset> _actionPositions(WidgetTester tester) => [
  for (final id in ['first', 'second', 'third'])
    tester.getTopLeft(
      find.byKey(Key('revision3-project-section-layout-fixture-action-$id')),
    ),
];

Color? _statusColor(WidgetTester tester, String id) => tester
    .widget<Material>(
      find.byKey(Key('revision3-project-section-severity-fixture-status-$id')),
    )
    .color;

void _setSurface(WidgetTester tester, Size size) {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
}

Future<void> _pumpPage(WidgetTester tester, Revision3ProjectSectionPage page) =>
    tester.pumpWidget(_host(page));

Widget _host(Revision3ProjectSectionPage page) => MaterialApp(
  theme: ThemeData(
    colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
  ),
  home: Scaffold(body: page),
);
