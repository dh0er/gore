import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_command_bar.dart';

void main() {
  testWidgets('wide bar keeps project orientation and exposes all commands', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1100, 700));
    var searches = 0;
    var creates = 0;
    var problems = 0;
    var settings = 0;

    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() => searches++),
      create: Revision3ProjectCommand.enabled(() => creates++),
      problems: Revision3ProjectCommand.enabled(() => problems++),
      settings: Revision3ProjectCommand.enabled(() => settings++),
    );

    expect(find.text('My Story Mod'), findsOneWidget);
    expect(find.text('Current section: Story'), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.createKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.problemsKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.settingsKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsNothing);

    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.problemsKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.settingsKey));
    await tester.pump();
    expect((searches, creates, problems, settings), (1, 1, 1, 1));
    expect(find.textContaining('Ready'), findsNothing);
    expect(find.textContaining('runtime'), findsNothing);
  });

  testWidgets('disabled commands expose the exact gate reason', (tester) async {
    await _setSurface(tester, const Size(900, 600));
    var calls = 0;
    const reason = 'Open a project before creating content.';
    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() => calls++),
      create: const Revision3ProjectCommand.disabled(reason),
      problems: const Revision3ProjectCommand.disabled(
        'Run project checks before reviewing problems.',
      ),
    );

    expect(find.byTooltip(reason), findsOneWidget);
    await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
    await tester.pump();
    expect(calls, 0);

    final semantics = tester.ensureSemantics();
    expect(
      tester.getSemantics(find.byKey(Revision3ProjectCommandBar.createKey)),
      matchesSemantics(
        label: 'Create',
        hint: reason,
        isButton: true,
        hasEnabledState: true,
        isEnabled: false,
      ),
    );
    semantics.dispose();
  });

  testWidgets('one pending callback blocks every command and then releases', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1000, 700));
    final pending = Completer<void>();
    var searches = 0;
    var creates = 0;
    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() {
        searches++;
        return pending.future;
      }),
      create: Revision3ProjectCommand.enabled(() => creates++),
      problems: const Revision3ProjectCommand.disabled('No checks yet.'),
    );

    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    expect(searches, 1);
    expect(find.text('Finishing the current project action…'), findsOneWidget);
    expect(
      find.byTooltip('Wait for the current project action to finish.'),
      findsNWidgets(3),
    );
    await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    expect((searches, creates), (1, 0));

    pending.complete();
    await tester.pumpAndSettle();
    expect(find.text('Finishing the current project action…'), findsNothing);
    await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
    await tester.pump();
    expect(creates, 1);
  });

  testWidgets('host busy state blocks commands and identifies its owner', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 600));
    var calls = 0;
    const reason = 'Wait for the open project task to finish.';
    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() => calls++),
      create: Revision3ProjectCommand.enabled(() => calls++),
      problems: Revision3ProjectCommand.enabled(() => calls++),
      busy: const Revision3ProjectCommandBarBusyState(
        label: 'Creating project content…',
        disabledReason: reason,
        command: Revision3ProjectCommandKind.create,
      ),
    );

    expect(find.text('Creating project content…'), findsOneWidget);
    expect(find.byIcon(Icons.hourglass_top), findsNWidgets(2));
    expect(find.byTooltip(reason), findsNWidgets(3));
    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    expect(calls, 0);
  });

  testWidgets(
    'compact German 200% layout keeps orientation and uses overflow',
    (tester) async {
      await _setSurface(tester, const Size(360, 480));
      var creates = 0;
      await _pumpBar(
        tester,
        copy: Revision3ProjectCommandBarCopy.german,
        textScaler: const TextScaler.linear(2),
        section: 'Dialoge und Sprachausgabe',
        search: Revision3ProjectCommand.enabled(() {}),
        create: Revision3ProjectCommand.enabled(() => creates++),
        problems: const Revision3ProjectCommand.disabled(
          'Prüfe zuerst den aktuellen Projektstand.',
        ),
      );

      expect(find.text('My Story Mod'), findsOneWidget);
      expect(
        find.text('Aktueller Bereich: Dialoge und Sprachausgabe'),
        findsOneWidget,
      );
      expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
      expect(find.byKey(Revision3ProjectCommandBar.createKey), findsNothing);
      expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsOneWidget);
      expect(tester.takeException(), isNull);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.moreKey));
      await tester.pumpAndSettle();
      expect(find.text('Erstellen'), findsOneWidget);
      expect(find.text('Probleme'), findsOneWidget);
      expect(
        find.byTooltip('Prüfe zuerst den aktuellen Projektstand.'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.compactCreateKey));
      await tester.pumpAndSettle();
      expect(creates, 1);
    },
  );

  testWidgets('compact 200% pending action keeps the bar within its host', (
    tester,
  ) async {
    await _setSurface(tester, const Size(360, 360));
    final pending = Completer<void>();
    await _pumpBar(
      tester,
      copy: Revision3ProjectCommandBarCopy.german,
      textScaler: const TextScaler.linear(2),
      section: 'Einstellungen & Expertenmodus',
      search: Revision3ProjectCommand.enabled(() => pending.future),
      create: Revision3ProjectCommand.enabled(() {}),
      problems: Revision3ProjectCommand.enabled(() {}),
    );

    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    expect(
      find.byKey(Revision3ProjectCommandBar.busyStatusKey),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);

    pending.complete();
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('orientation is semantic and Search is keyboard reachable', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 600));
    final semantics = tester.ensureSemantics();
    var searches = 0;
    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() => searches++),
      create: const Revision3ProjectCommand.disabled('Creation unavailable.'),
      problems: const Revision3ProjectCommand.disabled('Checks unavailable.'),
    );

    expect(
      tester.getSemantics(
        find.byKey(Revision3ProjectCommandBar.orientationKey),
      ),
      matchesSemantics(
        label: 'Project My Story Mod. Current section: Story.',
        isHeader: true,
      ),
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(searches, 1);
    semantics.dispose();
  });

  testWidgets(
    'pending callback may outlive the bar without a lifecycle error',
    (tester) async {
      await _setSurface(tester, const Size(900, 600));
      final pending = Completer<void>();
      await _pumpBar(
        tester,
        search: Revision3ProjectCommand.enabled(() => pending.future),
        create: const Revision3ProjectCommand.disabled('Creation unavailable.'),
        problems: const Revision3ProjectCommand.disabled('Checks unavailable.'),
      );
      await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
      await tester.pump();
      await tester.pumpWidget(const MaterialApp(home: SizedBox.shrink()));
      pending.complete();
      await tester.pump();
      expect(tester.takeException(), isNull);
    },
  );
}

Future<void> _pumpBar(
  WidgetTester tester, {
  required Revision3ProjectCommand search,
  required Revision3ProjectCommand create,
  required Revision3ProjectCommand problems,
  Revision3ProjectCommandBarBusyState? busy,
  Revision3ProjectCommand? settings,
  Revision3ProjectCommandBarCopy copy = const Revision3ProjectCommandBarCopy(),
  TextScaler textScaler = TextScaler.noScaling,
  String section = 'Story',
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: MediaQuery(
        data: MediaQueryData(textScaler: textScaler),
        child: Scaffold(
          body: Align(
            alignment: Alignment.topCenter,
            child: Revision3ProjectCommandBar(
              projectDisplayName: 'My Story Mod',
              currentSectionLabel: section,
              searchCommand: search,
              createCommand: create,
              problemsCommand: problems,
              settingsCommand: settings,
              busy: busy,
              copy: copy,
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
}

Future<void> _setSurface(WidgetTester tester, Size size) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}
