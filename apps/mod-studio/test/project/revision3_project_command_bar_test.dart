import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_project_command_bar.dart';

void main() {
  testWidgets('wide bar exposes Undo directly and serializes it', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1100, 700));
    final pending = Completer<void>();
    var undos = 0;
    var searches = 0;
    await _pumpBar(
      tester,
      undo: Revision3ProjectCommand.enabled(() {
        undos++;
        return pending.future;
      }),
      search: Revision3ProjectCommand.enabled(() => searches++),
      create: const Revision3ProjectCommand.disabled('Creation unavailable.'),
      problems: const Revision3ProjectCommand.disabled('Checks unavailable.'),
    );

    expect(find.byKey(Revision3ProjectCommandBar.undoKey), findsOneWidget);
    expect(find.byIcon(Icons.undo), findsOneWidget);
    await tester.tap(find.byKey(Revision3ProjectCommandBar.undoKey));
    await tester.pump();
    expect(undos, 1);
    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.tap(find.byKey(Revision3ProjectCommandBar.undoKey));
    await tester.pump();
    expect((undos, searches), (1, 0));

    pending.complete();
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    expect(searches, 1);
  });

  testWidgets('wide bar keeps project orientation and exposes all commands', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1100, 700));
    var searches = 0;
    var creates = 0;
    var problems = 0;
    var history = 0;
    var settings = 0;

    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() => searches++),
      create: Revision3ProjectCommand.enabled(() => creates++),
      problems: Revision3ProjectCommand.enabled(() => problems++),
      history: Revision3ProjectCommand.enabled(() => history++),
      settings: Revision3ProjectCommand.enabled(() => settings++),
    );

    expect(find.text('My Story Mod'), findsOneWidget);
    expect(find.text('Current section: Story'), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.createKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.problemsKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.historyKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.settingsKey), findsOneWidget);
    expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsNothing);

    await tester.tap(find.byKey(Revision3ProjectCommandBar.searchKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.createKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.problemsKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.historyKey));
    await tester.pump();
    await tester.tap(find.byKey(Revision3ProjectCommandBar.settingsKey));
    await tester.pump();
    expect((searches, creates, problems, history, settings), (1, 1, 1, 1, 1));
    expect(find.textContaining('Ready'), findsNothing);
    expect(find.textContaining('runtime'), findsNothing);
  });

  testWidgets('disabled secondary actions expose their exact gate reasons', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1100, 700));
    const historyReason = 'History is unavailable while checks are running.';
    const settingsReason = 'Settings are locked by the current project task.';
    await _pumpBar(
      tester,
      search: Revision3ProjectCommand.enabled(() {}),
      create: Revision3ProjectCommand.enabled(() {}),
      problems: Revision3ProjectCommand.enabled(() {}),
      history: const Revision3ProjectCommand.disabled(historyReason),
      settings: const Revision3ProjectCommand.disabled(settingsReason),
    );

    expect(find.byTooltip(historyReason), findsOneWidget);
    expect(find.byTooltip(settingsReason), findsOneWidget);

    final semantics = tester.ensureSemantics();
    expect(
      tester.getSemantics(find.byKey(Revision3ProjectCommandBar.historyKey)),
      matchesSemantics(
        label: 'History',
        hint: historyReason,
        isButton: true,
        hasEnabledState: true,
        isEnabled: false,
      ),
    );
    expect(
      tester.getSemantics(find.byKey(Revision3ProjectCommandBar.settingsKey)),
      matchesSemantics(
        label: 'Settings',
        hint: settingsReason,
        isButton: true,
        hasEnabledState: true,
        isEnabled: false,
      ),
    );
    semantics.dispose();
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
      var history = 0;
      var settings = 0;
      await _pumpBar(
        tester,
        copy: _germanCopy,
        textScaler: const TextScaler.linear(2),
        section: 'Dialoge und Sprachausgabe',
        undo: Revision3ProjectCommand.enabled(() {}),
        search: Revision3ProjectCommand.enabled(() {}),
        create: Revision3ProjectCommand.enabled(() => creates++),
        problems: const Revision3ProjectCommand.disabled(
          'Prüfe zuerst den aktuellen Projektstand.',
        ),
        history: Revision3ProjectCommand.enabled(() => history++),
        settings: Revision3ProjectCommand.enabled(() => settings++),
      );

      expect(find.text('My Story Mod'), findsOneWidget);
      expect(
        find.text('Aktueller Bereich: Dialoge und Sprachausgabe'),
        findsOneWidget,
      );
      expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
      expect(find.byKey(Revision3ProjectCommandBar.undoKey), findsNothing);
      expect(find.byKey(Revision3ProjectCommandBar.createKey), findsNothing);
      expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsOneWidget);
      expect(tester.takeException(), isNull);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.moreKey));
      await tester.pumpAndSettle();
      expect(find.text('R\u00fcckg\u00e4ngig'), findsOneWidget);
      expect(find.text('Erstellen'), findsOneWidget);
      expect(find.text('Probleme'), findsOneWidget);
      expect(find.text('Verlauf'), findsOneWidget);
      expect(find.text('Einstellungen'), findsOneWidget);
      expect(
        find.byTooltip('Prüfe zuerst den aktuellen Projektstand.'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.compactCreateKey));
      await tester.pumpAndSettle();
      expect(creates, 1);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.moreKey));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Revision3ProjectCommandBar.compactHistoryKey),
      );
      await tester.pumpAndSettle();
      expect(history, 1);

      await tester.tap(find.byKey(Revision3ProjectCommandBar.moreKey));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Revision3ProjectCommandBar.compactSettingsKey),
      );
      await tester.pumpAndSettle();
      expect(settings, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('compact 200% pending action keeps the bar within its host', (
    tester,
  ) async {
    await _setSurface(tester, const Size(360, 360));
    final pending = Completer<void>();
    await _pumpBar(
      tester,
      copy: _germanCopy,
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

  testWidgets(
    'long copy at 720px and 159% switches to compact without overflow',
    (tester) async {
      await _setSurface(tester, const Size(720, 600));
      await _pumpBar(
        tester,
        copy: _longCopy,
        textScaler: const TextScaler.linear(1.59),
        section: 'Localized authoring and recording workflows',
        undo: Revision3ProjectCommand.enabled(() {}),
        search: Revision3ProjectCommand.enabled(() {}),
        create: Revision3ProjectCommand.enabled(() {}),
        problems: Revision3ProjectCommand.enabled(() {}),
        history: Revision3ProjectCommand.enabled(() {}),
        settings: Revision3ProjectCommand.enabled(() {}),
      );

      expect(find.byKey(Revision3ProjectCommandBar.searchKey), findsOneWidget);
      expect(find.byKey(Revision3ProjectCommandBar.undoKey), findsNothing);
      expect(find.byKey(Revision3ProjectCommandBar.createKey), findsNothing);
      expect(find.byKey(Revision3ProjectCommandBar.moreKey), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}

const _germanCopy = Revision3ProjectCommandBarCopy(
  currentSectionTemplate: 'Aktueller Bereich: {section}',
  orientationSemanticsTemplate:
      'Projekt {project}. Aktueller Bereich: {section}.',
  undoLabel: 'R\u00fcckg\u00e4ngig',
  searchLabel: 'Suchen',
  createLabel: 'Erstellen',
  problemsLabel: 'Probleme',
  historyLabel: 'Verlauf',
  settingsLabel: 'Einstellungen',
  moreActionsTooltip: 'Weitere Projektaktionen',
  busyLabel: 'Die aktuelle Projektaktion wird abgeschlossen\u2026',
  busyDisabledReason:
      'Warte, bis die aktuelle Projektaktion abgeschlossen ist.',
);

const _englishCopy = Revision3ProjectCommandBarCopy(
  currentSectionTemplate: 'Current section: {section}',
  orientationSemanticsTemplate:
      'Project {project}. Current section: {section}.',
  undoLabel: 'Undo',
  searchLabel: 'Search',
  createLabel: 'Create',
  problemsLabel: 'Problems',
  historyLabel: 'History',
  settingsLabel: 'Settings',
  moreActionsTooltip: 'More project actions',
  busyLabel: 'Finishing the current project action\u2026',
  busyDisabledReason: 'Wait for the current project action to finish.',
);

const _longCopy = Revision3ProjectCommandBarCopy(
  currentSectionTemplate: 'Current localized workspace area: {section}',
  orientationSemanticsTemplate:
      'Authoring project {project}. Current localized workspace area: '
      '{section}.',
  undoLabel: 'Reverse the latest project change',
  searchLabel: 'Search throughout the entire project',
  createLabel: 'Create new project content',
  problemsLabel: 'Review every unresolved project problem',
  historyLabel: 'Project change history',
  settingsLabel: 'Project configuration settings',
  moreActionsTooltip: 'Open additional project authoring actions',
  busyLabel: 'Finishing the current project authoring action\u2026',
  busyDisabledReason:
      'Wait for the current project authoring action to finish.',
);

Future<void> _pumpBar(
  WidgetTester tester, {
  Revision3ProjectCommand? undo,
  required Revision3ProjectCommand search,
  required Revision3ProjectCommand create,
  required Revision3ProjectCommand problems,
  Revision3ProjectCommandBarBusyState? busy,
  Revision3ProjectCommand? history,
  Revision3ProjectCommand? settings,
  Revision3ProjectCommandBarCopy copy = _englishCopy,
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
              undoCommand: undo,
              searchCommand: search,
              createCommand: create,
              problemsCommand: problems,
              historyCommand: history,
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
