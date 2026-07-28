import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_journey.dart';
import 'package:gore_mod/project/revision3_quest_journey_panel.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _lineIds = <String>[
  '40000000000000000000000000000001',
  '40000000000000000000000000000002',
  '40000000000000000000000000000003',
];
const _localizationIds = <String>[
  '50000000000000000000000000000001',
  '50000000000000000000000000000002',
  '50000000000000000000000000000003',
];
const _lineLabels = <String>[
  'Ask Asghan about Homer',
  'DIA_GENERAL_GREETING',
  'Report the secured gate',
];
const _germanObjectiveTitles = <String>[
  'Asghan nach Homer fragen',
  'Das alte Tor untersuchen',
  'Das gesicherte Tor melden',
];
const _germanLineLabels = <String>[
  'Asghan nach Homer fragen',
  'DIA_ALLGEMEINE_BEGRUESSUNG',
  'Das gesicherte Tor melden',
];

void main() {
  testWidgets('wide view is friendly, ordered and delegates exact actions', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 900));
    final projection = await _projection();
    var nameEdits = 0;
    var connectionEdits = 0;
    var transitionEdits = 0;
    Revision3QuestTranscriptRow? opened;

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel(
        projection: projection,
        onEditNameObjectives: () => nameEdits++,
        onEditDescriptionConnections: () => connectionEdits++,
        onEditStatesTransitions: () => transitionEdits++,
        onOpenDialogLine: (row) => opened = row,
      ),
    );

    expect(find.byKey(const Key('revision3-quest-journey-wide')), findsOne);
    expect(find.text('Find Homer'), findsOne);
    expect(find.text('Quest giver: Asghan', findRichText: true), findsOne);
    expect(
      find.text('Part of: Swamp Camp SC Chapter 2', findRichText: true),
      findsOne,
    );
    expect(find.text('Draft'), findsOne);
    expect(find.text('Project logic'), findsOne);
    expect(find.textContaining('does not prove'), findsOne);
    expect(
      find.byKey(
        const Key('revision3-quest-journey-behavior-main-availability'),
      ),
      findsOne,
    );
    expect(
      find.byKey(const Key('revision3-quest-journey-behavior-main-failure')),
      findsOne,
    );

    final firstObjective = tester.getTopLeft(
      find.byKey(const Key('revision3-quest-journey-objective-title-0')),
    );
    final secondObjective = tester.getTopLeft(
      find.byKey(const Key('revision3-quest-journey-objective-title-1')),
    );
    final thirdObjective = tester.getTopLeft(
      find.byKey(const Key('revision3-quest-journey-objective-title-2')),
    );
    expect(firstObjective.dy, lessThan(secondObjective.dy));
    expect(secondObjective.dy, lessThan(thirdObjective.dy));

    final firstLinkedLine = tester.getTopLeft(
      find.byKey(const Key('revision3-quest-journey-dialog-line-0')),
    );
    final secondLinkedLine = tester.getTopLeft(
      find.byKey(const Key('revision3-quest-journey-dialog-line-1')),
    );
    expect(firstLinkedLine.dy, lessThan(secondLinkedLine.dy));
    expect(find.text('Report the secured gate'), findsNWidgets(2));
    expect(find.text('Ask Asghan about Homer'), findsNWidgets(2));
    expect(find.text('Dialog line 3'), findsOne);

    await tester.tap(
      find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-quest-journey-edit-states-transitions')),
    );
    await tester.pumpAndSettle();
    final dialogLine = find.byKey(
      const Key('revision3-quest-journey-dialog-line-1'),
    );
    await tester.ensureVisible(dialogLine);
    await tester.tap(dialogLine);
    await tester.pumpAndSettle();

    expect(nameEdits, 1);
    expect(connectionEdits, 1);
    expect(transitionEdits, 1);
    expect(opened?.lineId, _lineIds[0]);
    expect(identical(opened, projection.orderedDialogLines[1].row), isTrue);

    final renderedText = _renderedText(tester);
    expect(renderedText, isNot(contains(revision3QuestOutlineProjectId)));
    expect(renderedText, isNot(contains(revision3QuestOutlineQuestId)));
    expect(renderedText, isNot(contains(revision3QuestOutlineModuleId)));
    expect(renderedText, isNot(contains('OM_GRD_Asghan_263')));
    expect(renderedText, isNot(contains('UQuest_SwampCamp_SCChapter2')));
    expect(renderedText, isNot(contains('DIA_GENERAL_GREETING')));
    expect(renderedText, isNot(contains('PROJECT/')));
    expect(tester.takeException(), isNull);
  });

  testWidgets('narrow view stacks cleanly and keeps final dialog reachable', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    final projection = await _projection();

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel(
        projection: projection,
        onEditNameObjectives: () {},
        onEditDescriptionConnections: () {},
        onEditStatesTransitions: () {},
        onOpenDialogLine: (_) {},
      ),
    );

    expect(find.byKey(const Key('revision3-quest-journey-narrow')), findsOne);
    expect(tester.takeException(), isNull);

    final lastLine = find.byKey(
      const Key('revision3-quest-journey-dialog-line-2'),
    );
    await tester.scrollUntilVisible(
      lastLine,
      350,
      scrollable: find.byType(Scrollable).first,
    );
    expect(lastLine, findsOne);
    expect(find.text('Dialog line 3'), findsOne);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'persistent Draft setup shows two publications and one review action',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1000, 900));
      final projection = await _projection();
      var dialogOpens = 0;

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          onEditNameObjectives: () {},
          onEditDescriptionConnections: () {},
          onEditStatesTransitions: () {},
          onOpenDialogVoice: () => dialogOpens++,
        ),
      );

      expect(
        find.byKey(const Key('revision3-quest-draft-setup')),
        findsOneWidget,
      );
      for (final step in const <Revision3QuestDraftSetupStepKind>[
        Revision3QuestDraftSetupStepKind.questDetails,
        Revision3QuestDraftSetupStepKind.openingDialog,
      ]) {
        expect(
          find.byKey(Key('revision3-quest-draft-setup-step-${step.name}')),
          findsOneWidget,
        );
      }
      expect(find.text('Saved in project'), findsNWidgets(2));
      expect(find.textContaining('one project checkpoint'), findsOneWidget);
      expect(find.textContaining('separate second checkpoint'), findsOneWidget);
      final recommended = find.byKey(
        const Key('revision3-quest-draft-setup-recommended-dialog-voice'),
      );
      expect(recommended, findsOneWidget);
      expect(
        find.byKey(
          const Key('revision3-quest-draft-setup-recommended-quest-details'),
        ),
        findsNothing,
      );
      expect(find.text('Review dialog & Voice'), findsOneWidget);
      expect(
        find.textContaining('not Build-ready or proven playable'),
        findsOneWidget,
      );
      final semantics = tester.ensureSemantics();
      expect(
        find.semantics.byLabel(
          RegExp(
            r'^1\. Quest details\. Saved in project\. '
            r'Name, objectives, Quest family, giver, and explicit behavior '
            r'were saved together\.$',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.semantics.byLabel(RegExp(r'^1\. Quest details$')),
        findsNothing,
        reason: 'visible step text is excluded from the composed label',
      );
      await tester.ensureVisible(recommended);
      await tester.tap(recommended);
      await tester.pumpAndSettle();
      expect(dialogOpens, 1);
      semantics.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Draft setup remains usable at 360px and 200% text', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 900));
    final projection = await _projection(emptyTranscript: true);
    var dialogOpens = 0;

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel(
        projection: projection,
        onOpenDialogVoice: () => dialogOpens++,
      ),
      textScaler: const TextScaler.linear(2),
    );

    expect(
      find.byKey(const Key('revision3-quest-draft-setup')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-quest-draft-setup-step-openingDialog')),
      findsOneWidget,
    );
    final recommended = find.byKey(
      const Key('revision3-quest-draft-setup-recommended-dialog-voice'),
    );
    expect(recommended, findsOneWidget);
    expect(find.text('Write opening dialog'), findsOneWidget);
    await tester.scrollUntilVisible(
      recommended,
      160,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.pumpAndSettle();
    await tester.tap(recommended);
    await tester.pumpAndSettle();
    expect(dialogOpens, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'wide blocked edits stay visible, disabled and explain the owner reason',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 900));
      final projection = await _projection();
      const reason = 'Save or discard the pending project changes first.';

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          editDisabledReason: reason,
        ),
      );

      final name = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      final connections = find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      );
      final transitions = find.byKey(
        const Key('revision3-quest-journey-edit-states-transitions'),
      );
      expect(name, findsOne);
      expect(connections, findsOne);
      expect(transitions, findsOne);
      expect(tester.widget<OutlinedButton>(name).onPressed, isNull);
      expect(tester.widget<OutlinedButton>(connections).onPressed, isNull);
      expect(tester.widget<FilledButton>(transitions).onPressed, isNull);
      expect(
        find.byKey(const Key('revision3-quest-journey-edit-disabled-reason')),
        findsOne,
      );
      expect(find.text(reason), findsOne);
      expect(
        tester
            .widgetList<Tooltip>(find.byType(Tooltip))
            .where((tooltip) => tooltip.message == reason),
        hasLength(3),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'compact partial blocker keeps other edits enabled without overflow',
    (tester) async {
      await _setSurfaceSize(tester, const Size(320, 480));
      final projection = await _projection();
      const reason =
          'Richte zuerst den Gothic-Spielordner in den Einstellungen ein, bevor du Beschreibung und Verknüpfungen dieser Quest bearbeitest.';
      var nameEdits = 0;
      var transitionEdits = 0;

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          onEditNameObjectives: () => nameEdits++,
          editDescriptionConnectionsDisabledReason: reason,
          onEditStatesTransitions: () => transitionEdits++,
          copy: const Revision3QuestJourneyPanelCopy.german(),
        ),
      );

      expect(find.byKey(const Key('revision3-quest-journey-narrow')), findsOne);
      final name = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      final connections = find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      );
      final transitions = find.byKey(
        const Key('revision3-quest-journey-edit-states-transitions'),
      );
      expect(name, findsOne);
      expect(connections, findsOne);
      expect(transitions, findsOne);
      expect(tester.widget<OutlinedButton>(name).onPressed, isNotNull);
      expect(tester.widget<OutlinedButton>(connections).onPressed, isNull);
      expect(tester.widget<FilledButton>(transitions).onPressed, isNotNull);
      expect(find.text(reason), findsOne);
      expect(
        tester
            .widgetList<Tooltip>(find.byType(Tooltip))
            .where((tooltip) => tooltip.message == reason),
        hasLength(1),
      );
      await tester.tap(name);
      await tester.pumpAndSettle();
      await tester.tap(transitions);
      await tester.pumpAndSettle();
      expect(nameEdits, 1);
      expect(transitionEdits, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'duplicate per-action reasons render once but explain each action',
    (tester) async {
      await _setSurfaceSize(tester, const Size(900, 700));
      final projection = await _projection();
      const reason = 'The configured game folder is required for this editor.';

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          editDescriptionConnectionsDisabledReason: reason,
          editStatesTransitionsDisabledReason: reason,
        ),
      );

      expect(find.text(reason), findsOne);
      expect(
        tester
            .widgetList<Tooltip>(find.byType(Tooltip))
            .where((tooltip) => tooltip.message == reason),
        hasLength(2),
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(
                const Key(
                  'revision3-quest-journey-edit-description-connections',
                ),
              ),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                const Key('revision3-quest-journey-edit-states-transitions'),
              ),
            )
            .onPressed,
        isNull,
      );
      expect(
        find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'edit actions keep one single-flight lane and explain busy state',
    (tester) async {
      await _setSurfaceSize(tester, const Size(900, 820));
      final projection = await _projection();
      final pending = Completer<void>();
      var nameEdits = 0;
      var connectionEdits = 0;

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          onEditNameObjectives: () {
            nameEdits++;
            return pending.future;
          },
          onEditDescriptionConnections: () => connectionEdits++,
        ),
      );

      final name = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      final connections = find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      );
      await tester.tap(name);
      await tester.pump();

      expect(nameEdits, 1);
      expect(tester.widget<OutlinedButton>(name).onPressed, isNull);
      expect(tester.widget<OutlinedButton>(connections).onPressed, isNull);
      expect(
        tester
            .widgetList<Tooltip>(find.byType(Tooltip))
            .where(
              (tooltip) =>
                  tooltip.message ==
                  const Revision3QuestJourneyPanelCopy.english()
                      .editActionBusyReason,
            ),
        hasLength(2),
      );
      final progress = find.byKey(
        const Key('revision3-quest-journey-action-progress'),
      );
      expect(progress, findsOne);
      expect(
        find.text(
          const Revision3QuestJourneyPanelCopy.english().editActionBusyReason,
        ),
        findsOne,
      );
      expect(tester.widget<Semantics>(progress).properties.liveRegion, isTrue);
      final progressIndicator = tester.widget<CircularProgressIndicator>(
        find.descendant(
          of: progress,
          matching: find.byType(CircularProgressIndicator),
        ),
      );
      expect(
        progressIndicator.semanticsLabel,
        const Revision3QuestJourneyPanelCopy.english().editActionBusyReason,
      );

      await tester.tap(connections, warnIfMissed: false);
      await tester.pump();
      expect(connectionEdits, 0);

      pending.complete();
      await tester.pumpAndSettle();
      expect(progress, findsNothing);
      expect(tester.widget<OutlinedButton>(connections).onPressed, isNotNull);
      await tester.tap(connections);
      await tester.pumpAndSettle();
      expect(connectionEdits, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'compact German busy status keeps long progress guidance accessible',
    (tester) async {
      await _setSurfaceSize(tester, const Size(300, 440));
      final projection = await _projection(german: true);
      final pending = Completer<void>();
      const copy = Revision3QuestJourneyPanelCopy.german();

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          onEditNameObjectives: () => pending.future,
          copy: copy,
        ),
      );

      await tester.tap(
        find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
      );
      await tester.pump();

      final progress = find.byKey(
        const Key('revision3-quest-journey-action-progress'),
      );
      expect(progress, findsOne);
      expect(find.text(copy.editActionBusyReason), findsOne);
      expect(tester.widget<Semantics>(progress).properties.liveRegion, isTrue);
      expect(
        tester
            .widget<CircularProgressIndicator>(
              find.descendant(
                of: progress,
                matching: find.byType(CircularProgressIndicator),
              ),
            )
            .semanticsLabel,
        copy.editActionBusyReason,
      );
      await tester.ensureVisible(progress);
      await tester.pump();
      expect(tester.takeException(), isNull);

      pending.complete();
      await tester.pumpAndSettle();
      expect(progress, findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'German narrow view localizes actions, boundaries and objective dialogs',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 700));
      final projection = await _projection(german: true);
      var transitionEdits = 0;
      Revision3QuestTranscriptRow? opened;

      await _pumpPanel(
        tester,
        Revision3QuestJourneyPanel(
          projection: projection,
          giverDisplayName: 'Asghan',
          parentStoryDisplayName: 'Sumpflager, Kapitel 2',
          onEditNameObjectives: () {},
          onEditDescriptionConnections: () {},
          onEditStatesTransitions: () => transitionEdits++,
          onOpenDialogLine: (row) => opened = row,
          copy: const Revision3QuestJourneyPanelCopy.german(),
        ),
      );

      expect(find.byKey(const Key('revision3-quest-journey-narrow')), findsOne);
      expect(find.text('QUEST-ABLAUF'), findsOne);
      expect(find.text('Homer finden'), findsOne);
      expect(find.text('Name & Ziele bearbeiten'), findsOne);
      expect(find.text('Beschreibung & Verknüpfungen bearbeiten'), findsOne);
      expect(find.text('Zustände & Übergänge bearbeiten'), findsOneWidget);
      expect(find.text('Entwurf'), findsOne);
      expect(find.text('Projektlogik'), findsOne);
      expect(find.text('Offline-Projektansicht'), findsOne);
      expect(find.textContaining('Sie belegt nicht'), findsOne);
      expect(find.text('Hauptquest'), findsOne);
      expect(find.text('Ziele'), findsOne);
      expect(find.text('Verfügbar'), findsNWidgets(4));
      expect(find.text('Erfolg'), findsNWidgets(4));
      expect(find.text('Fehlschlag'), findsNWidgets(4));
      expect(find.text('Nicht verwendet'), findsNWidgets(5));

      await tester.tap(
        find.byKey(
          const Key('revision3-quest-journey-edit-states-transitions'),
        ),
      );
      await tester.pumpAndSettle();
      expect(transitionEdits, 1);

      final lastLine = find.byKey(
        const Key('revision3-quest-journey-dialog-line-2'),
      );
      await tester.scrollUntilVisible(
        lastLine,
        350,
        scrollable: find.byType(Scrollable).first,
      );
      expect(find.text('Dialogzeile 3'), findsOne);
      expect(find.textContaining('Text in 1 Sprache'), findsNWidgets(4));
      expect(find.textContaining('0 Sprachaufnahmen'), findsNWidgets(4));

      await tester.ensureVisible(lastLine);
      await tester.tap(lastLine);
      await tester.pumpAndSettle();
      expect(opened?.lineId, _lineIds[1]);

      final rendered = _renderedText(tester);
      for (final english in const <String>[
        'Quest journey',
        'Draft',
        'Project logic',
        'Offline project view',
        'Main Quest',
        'Objectives',
        'Original fixed behavior',
        'Linked dialog',
        'Not used',
        'Direct trigger allowed',
        'Voice take',
        'Dialog line',
        'Retry',
      ]) {
        expect(rendered, isNot(contains(english)));
      }
      expect(rendered, isNot(contains('DIA_ALLGEMEINE_BEGRUESSUNG')));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('unavailable view exposes only safe recovery guidance', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 480));
    var retries = 0;

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel.unavailable(onRetry: () => retries++),
    );

    expect(
      find.byKey(const Key('revision3-quest-journey-unavailable')),
      findsOne,
    );
    expect(find.text('Quest journey unavailable'), findsOne);
    expect(find.textContaining('exact project checkpoint'), findsOne);
    await tester.tap(find.byKey(const Key('revision3-quest-journey-retry')));
    await tester.pumpAndSettle();
    expect(retries, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets('empty transcript states do not imply missing runtime content', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(700, 820));
    final projection = await _projection(emptyTranscript: true);

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel(projection: projection),
    );

    expect(projection.orderedDialogLines, isEmpty);
    expect(
      find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-quest-journey-edit-disabled-reason')),
      findsNothing,
    );
    expect(
      find.text('No dialog is linked to this objective.'),
      findsNWidgets(3),
    );
    expect(find.textContaining('No dialog exists in the game'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('callback failures remain friendly and never expose raw errors', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(700, 820));
    final projection = await _projection();

    await _pumpPanel(
      tester,
      Revision3QuestJourneyPanel(
        projection: projection,
        onEditNameObjectives: () => throw StateError(
          r'C:\private\project.goremod 0123456789abcdef0123456789abcdef',
        ),
      ),
    );
    await tester.tap(
      find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
    );
    await tester.pumpAndSettle();

    expect(
      find.text(
        'That editor could not be opened. The project view was not changed.',
      ),
      findsOne,
    );
    final rendered = _renderedText(tester);
    expect(rendered, isNot(contains('project.goremod')));
    expect(rendered, isNot(contains('0123456789abcdef')));
    expect(tester.takeException(), isNull);
  });

  test('German copy uses natural singular and plural forms', () {
    const copy = Revision3QuestJourneyPanelCopy.german();

    expect(copy.objectiveLabel(1), 'Ziel 1');
    expect(copy.automaticRules(1), '1 automatische Regel');
    expect(copy.automaticRules(2), '2 automatische Regeln');
    expect(
      copy.directOrAutomaticRules(1),
      'Direkt oder durch 1 automatische Regel',
    );
    expect(copy.followUps(1, false), '1 Folgeaktion');
    expect(
      copy.followUps(2, true),
      '2 Folgeaktionen + schließt die übergeordnete Quest ab',
    );
    expect(copy.showDialogLines(1), '1 Dialogzeile anzeigen');
    expect(copy.showDialogLines(2), '2 Dialogzeilen anzeigen');
    expect(copy.textLanguageCount(1), 'Text in 1 Sprache');
    expect(copy.textLanguageCount(2), 'Text in 2 Sprachen');
    expect(copy.voiceTakeCount(1), '1 Sprachaufnahme');
    expect(copy.voiceTakeCount(2), '2 Sprachaufnahmen');
    expect(copy.selectedVoiceCount(1), '1 ausgewählte Aufnahme');
    expect(copy.selectedVoiceCount(2), '2 ausgewählte Aufnahmen');
    expect(copy.sharedQuestCount(1), 'Von 1 Quest verwendet');
    expect(copy.sharedQuestCount(2), 'Von 2 Quests verwendet');
    expect(copy.dialogLineLabel(3), 'Dialogzeile 3');
  });
}

Future<Revision3QuestJourneyProjection> _projection({
  bool emptyTranscript = false,
  bool german = false,
}) async {
  final objectiveTitles = german
      ? _germanObjectiveTitles
      : const <String>[
          'Ask Asghan about Homer',
          'Inspect the old gate',
          'Report the secured gate',
        ];
  final title = german ? 'Homer finden' : 'Find Homer';
  final outline = Revision3QuestOutlineFixture(
    displayName: title,
    title: title,
    objectiveTitles: objectiveTitles,
  );
  final index = Revision3ContentIndex.fromJsonObject(
    _contentIndexJson(emptyTranscript: emptyTranscript, german: german),
  );
  final service = Revision3QuestTranscriptAuthoringService(
    expectedHead: outline.head,
    loadContentIndex: () async => index,
    readExactLocalization:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required localizationId,
          required expectedLocalizationRevision,
          required expectedLocId,
        }) async => throw StateError('Panel must not read localization'),
    publishReplace:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async => throw StateError('Panel must not publish'),
    publishCreate:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async => throw StateError('Panel must not publish'),
  );
  final transcript = await service.load(
    questId: revision3QuestOutlineQuestId,
    expectedQuestRevision: outline.questRevision,
  );
  final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
    currentProjectJson: outline.projectJson,
    questId: revision3QuestOutlineQuestId,
    expectedQuestRevision: outline.questRevision,
    expectedModuleId: revision3QuestOutlineModuleId,
    expectedModuleRevision: outline.moduleRevision,
  );
  return Revision3QuestJourneyProjection.compose(
    index: index,
    quest: index.entityById(revision3QuestOutlineQuestId)!,
    module: index.entityById(revision3QuestOutlineModuleId)!,
    transitionSeed: seed,
    transcript: transcript,
  );
}

Map<String, Object?> _contentIndexJson({
  required bool emptyTranscript,
  required bool german,
}) {
  final title = german ? 'Homer finden' : 'Find Homer';
  final objectiveTitles = german
      ? _germanObjectiveTitles
      : const <String>[
          'Ask Asghan about Homer',
          'Inspect the old gate',
          'Report the secured gate',
        ];
  final lineLabels = german ? _germanLineLabels : _lineLabels;
  final bindings = emptyTranscript
      ? const <({int lineIndex, int slot})>[]
      : <({int lineIndex, int slot})>[
          (lineIndex: 2, slot: 1),
          (lineIndex: 0, slot: 1),
          (lineIndex: 1, slot: 3),
        ];
  final lineCount = emptyTranscript ? 0 : 3;
  return <String, Object?>{
    'schema_revision': 1,
    'project_id': revision3QuestOutlineProjectId,
    'project_revision': 7,
    'project_name': 'Quest journey panel fixture',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256': revision3QuestOutlineTargetSha,
      },
    },
    'authoring_locales': <Object?>['de'],
    'entity_counts': <String, Object?>{
      if (lineCount > 0) 'localization_entry': lineCount,
      if (lineCount > 0) 'dialog_line': lineCount,
      'quest_draft': 1,
      'script_module': 1,
    },
    'entities': <Object?>[
      _entity(
        id: revision3QuestOutlineQuestId,
        kind: 'quest_draft',
        displayName: title,
        revision: 4,
        origin: <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_FIND_HOMER',
        },
        summary: <String, Object?>{
          'technical_id': 'GORE_FIND_HOMER',
          'title': title,
          'objective_title': objectiveTitles.first,
          'additional_objective_titles': objectiveTitles.skip(1).toList(),
          'objective_slots': <Object?>[1, 2, 3],
          'transcript_count': bindings.length,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
          'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
        },
        references: <Object?>[
          _reference(
            role: 'draft_script_module',
            targetId: revision3QuestOutlineModuleId,
            expectedKind: 'script_module',
          ),
          for (final binding in bindings)
            _reference(
              role: 'quest_transcript_line',
              qualifier: binding.slot.toString(),
              targetId: _lineIds[binding.lineIndex],
              expectedKind: 'dialog_line',
            ),
        ],
        assetReferences: _questCollisionAssetReferences,
      ),
      _entity(
        id: revision3QuestOutlineModuleId,
        kind: 'script_module',
        displayName: 'Find Homer Script',
        revision: 5,
        origin: <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'owner': <String, Object?>{
            'project_id': revision3QuestOutlineProjectId,
            'entity_id': revision3QuestOutlineQuestId,
            'expected_kind': 'quest_draft',
          },
        },
        summary: <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
        references: <Object?>[
          _reference(
            role: 'origin_owner',
            targetId: revision3QuestOutlineQuestId,
            expectedKind: 'quest_draft',
          ),
          _reference(
            role: 'script_owner',
            targetId: revision3QuestOutlineQuestId,
            expectedKind: 'quest_draft',
          ),
        ],
      ),
      for (var index = 0; index < lineCount; index++)
        _entity(
          id: _lineIds[index],
          kind: 'dialog_line',
          displayName: lineLabels[index],
          revision: 1,
          summary: <String, Object?>{
            'speaker_hint': index == 1 ? 'OM_GRD_Asghan_263' : 'Asghan',
            'voice_slot_locales': <Object?>[],
          },
          references: <Object?>[
            _reference(
              role: 'dialog_localization',
              targetId: _localizationIds[index],
              expectedKind: 'localization_entry',
            ),
          ],
        ),
      for (var index = 0; index < lineCount; index++)
        _entity(
          id: _localizationIds[index],
          kind: 'localization_entry',
          displayName: 'Dialog text ${index + 1}',
          revision: 1,
          summary: <String, Object?>{
            'loc_id': 'DIA_JOURNEY_${index + 1}',
            'locales': <Object?>['de'],
          },
        ),
    ],
    'assets': <Object?>[
      <String, Object?>{
        'sha256': revision3QuestOutlineArtifactSha,
        'byte_len': 123,
        'media_type':
            'application/vnd.gore.quest-collision-capability+json;version=2',
        'class': 'quest_collision_artifact',
      },
    ],
  };
}

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summary,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
  List<Object?> assetReferences = const <Object?>[],
}) => <String, Object?>{
  'id': id,
  'kind': kind,
  'display_name': displayName,
  'revision': revision,
  'origin':
      origin ??
      <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'AUTHORED_${kind.toUpperCase()}',
      },
  'summary': <String, Object?>{'kind': kind, 'data': summary},
  'references': references,
  'asset_references': assetReferences,
};

const _questCollisionAssetReferences = <Object?>[
  <String, Object?>{
    'role': 'quest_collision_artifact',
    'sha256': revision3QuestOutlineArtifactSha,
    'byte_len': 123,
    'logical_name': null,
    'expected_media_type':
        'application/vnd.gore.quest-collision-capability+json;version=2',
    'resolution': 'resolved',
  },
];

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3QuestOutlineProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Future<void> _pumpPanel(
  WidgetTester tester,
  Revision3QuestJourneyPanel panel, {
  TextScaler textScaler = TextScaler.noScaling,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: ThemeData(useMaterial3: true),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: textScaler),
        child: child!,
      ),
      home: Scaffold(body: panel),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

String _renderedText(WidgetTester tester) => tester
    .widgetList<Text>(find.byType(Text))
    .map((widget) => widget.data ?? '')
    .join('\n');
