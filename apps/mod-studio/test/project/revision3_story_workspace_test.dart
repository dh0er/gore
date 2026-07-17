import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_story_entity_workbench.dart';
import 'package:gore_mod/project/revision3_story_workspace.dart';

const _projectA = '11111111111111111111111111111111';
const _projectB = '99999999999999999999999999999999';
const _npcId = '22222222222222222222222222222222';
const _questId = '33333333333333333333333333333333';
const _moduleId = '44444444444444444444444444444444';
const _blockerId = '55555555555555555555555555555555';
const _transcriptLocalizationId = '66666666666666666666666666666666';
const _transcriptLineId = '77777777777777777777777777777777';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _artifactSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

final _copy = Revision3StoryWorkspaceCopy(
  title: 'Story',
  loadingLabel: 'Opening exact Story drafts',
  authorityNotice:
      'Project drafts only. Build and runtime readiness are not evaluated.',
  searchHint: 'Search NPCs and Quests',
  clearSearchLabel: 'Clear search',
  allFilterLabel: 'All',
  npcFilterLabel: 'NPCs',
  questFilterLabel: 'Quests',
  createNpcLabel: 'Create NPC',
  createQuestLabel: 'Create Quest',
  creatingNpcLabel: 'Creating NPC',
  creatingQuestLabel: 'Creating Quest',
  noStoryDrafts: 'No Story drafts yet',
  noMatchingStoryDrafts: 'No matching Story drafts',
  selectDraftLabel: 'Select an NPC or Quest',
  retryLabel: 'Retry',
  loadErrorTitle: 'Story could not be opened',
  checkpointMismatchError: 'Story index does not match this checkpoint.',
  checkpointSummary: (count, revision) => '$count drafts / revision $revision',
  loadErrorDetails: (error) => '$error',
  createErrorDetails: (error) => 'CREATE FAILED: $error',
  detailsSheetLabel: (name) => '$name details',
  removeDraftPairUnavailable: 'Draft pair unavailable.',
  removeDraftBusy: 'Another Story action is busy.',
  removeDraftBlocked: (count) => '$count removal blockers.',
  removeDraftDialogTitle: 'Remove draft from project?',
  removeDraftDialogSummary: (draft, script) => 'Remove $draft and $script.',
  removeDraftNoUndo: 'This cannot be undone in version 1.',
  removeDraftBoundary: 'Game files and save games stay unchanged.',
  removeDraftCancel: 'Cancel',
  removeDraftConfirm: 'Remove draft',
  removeDraftBlockedTitle: 'Draft is still referenced',
  removeDraftBlockedDescription: 'Open every referencing source.',
  removeDraftBlockerLabel: (source, role) => '$source · $role',
  removeDraftOpenBlocker: 'Open source',
  removeDraftBlockedClose: 'Close',
  removeDraftSucceeded: (draft) => 'REMOVED: $draft',
  removeDraftErrorDetails: (error) => 'REMOVE FAILED: $error',
  workbench: const Revision3StoryEntityWorkbenchCopy.english(),
);

final _longGermanCopy = Revision3StoryWorkspaceCopy(
  title: 'Geschichten, Charaktere und umfangreiche Quest-Entwürfe',
  loadingLabel: 'Die exakten aktuellen Story-Entwürfe werden geöffnet',
  authorityNotice:
      'Hier werden ausschließlich Projektentwürfe bearbeitet. Die Build-Bereitschaft und das Verhalten zur Laufzeit wurden noch nicht geprüft.',
  searchHint:
      'NPCs und Quests nach Namen, technischer Kennung oder Beschreibung durchsuchen',
  clearSearchLabel: 'Suche vollständig zurücksetzen',
  allFilterLabel: 'Alle Entwürfe',
  npcFilterLabel: 'Nichtspielercharaktere',
  questFilterLabel: 'Quest-Entwürfe',
  createNpcLabel: 'Neuen Nichtspielercharakter erstellen',
  createQuestLabel: 'Neuen Quest-Entwurf erstellen',
  creatingNpcLabel: 'Nichtspielercharakter wird erstellt',
  creatingQuestLabel: 'Quest-Entwurf wird erstellt',
  noStoryDrafts: 'Noch keine Story-Entwürfe vorhanden',
  noMatchingStoryDrafts: 'Keine passenden Story-Entwürfe gefunden',
  selectDraftLabel: 'NPC oder Quest-Entwurf auswählen',
  retryLabel: 'Erneut versuchen',
  loadErrorTitle: 'Story-Arbeitsbereich konnte nicht geöffnet werden',
  checkpointMismatchError:
      'Der Story-Index gehört nicht zum exakten aktuellen Projektstand.',
  checkpointSummary: (count, revision) =>
      '$count Story-Entwürfe in Projektrevision $revision',
  loadErrorDetails: (error) => 'Ladefehler: $error',
  createErrorDetails: (error) => 'Erstellungsfehler: $error',
  detailsSheetLabel: (name) => 'Details für $name',
  removeDraftPairUnavailable: 'Entwurfspaar nicht verfügbar.',
  removeDraftBusy: 'Eine Story-Aktion läuft.',
  removeDraftBlocked: (count) => '$count blockierende Referenzen.',
  removeDraftDialogTitle: 'Entwurf entfernen?',
  removeDraftDialogSummary: (draft, script) => '$draft und $script entfernen.',
  removeDraftNoUndo: 'In Version 1 nicht rückgängig zu machen.',
  removeDraftBoundary: 'Spiel und Spielstände bleiben unverändert.',
  removeDraftCancel: 'Abbrechen',
  removeDraftConfirm: 'Entwurf entfernen',
  removeDraftBlockedTitle: 'Entwurf wird noch referenziert',
  removeDraftBlockedDescription: 'Alle Quellen öffnen.',
  removeDraftBlockerLabel: (source, role) => '$source · $role',
  removeDraftOpenBlocker: 'Quelle öffnen',
  removeDraftBlockedClose: 'Schließen',
  removeDraftSucceeded: (draft) => '$draft entfernt',
  removeDraftErrorDetails: (error) => 'Entfernen fehlgeschlagen: $error',
  workbench: const Revision3StoryEntityWorkbenchCopy.english(),
);

void main() {
  testWidgets(
    'loads only Story drafts with search, filters, and honest create actions',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final pending = Completer<Revision3ContentIndex>();

      await _pumpWorkspace(tester, load: () => pending.future);
      expect(
        find.byKey(const Key('revision3-story-workspace-loading')),
        findsOneWidget,
      );
      expect(find.text('Opening exact Story drafts'), findsOneWidget);

      pending.complete(_fixture());
      await tester.pumpAndSettle();

      expect(find.text('2 drafts / revision 7'), findsOneWidget);
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_moduleId')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-story-workspace-wide')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
        findsOneWidget,
      );

      expect(find.text('NPC creation is not configured.'), findsOneWidget);
      expect(find.text('Quest creation is not configured.'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-story-workspace-create-npc')),
            )
            .onPressed,
        isNull,
      );

      await tester.enterText(
        find.byKey(const Key('revision3-story-workspace-search')),
        'homer',
      );
      await tester.pump();
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsNothing,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-clear-search')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-story-workspace-filter-npc')),
      );
      await tester.pump();
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'NPC Profile edits the friendly name directly and keeps checks separate',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var editCalls = 0;
      var inspectCalls = 0;
      Revision3ContentIndex? editedIndex;
      Revision3ContentEntity? editedNpc;
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(),
        editNpcProfile: (index, npc) async {
          editCalls++;
          editedIndex = index;
          editedNpc = npc;
        },
        inspectNpcSource: (_, _) async => inspectCalls++,
      );
      await tester.pumpAndSettle();

      final edit = find.byKey(
        Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
      );
      expect(edit, findsOneWidget);
      expect(find.text('Edit name & archetype'), findsOneWidget);
      expect(find.text('Character name'), findsOneWidget);
      expect(find.text('Gate Guard'), findsWidgets);
      expect(
        find.byKey(Key('revision3-story-workbench-action-inspect-npc-$_npcId')),
        findsNothing,
      );
      expect(find.text('GORE_GATE_GUARD'), findsNothing);
      expect(find.text('PROJECT.NPCS.GATEGUARD'), findsNothing);

      await tester.tap(edit);
      await tester.pumpAndSettle();
      expect(editCalls, 1);
      expect(editedIndex?.projectId, _projectA);
      expect(editedIndex?.projectRevision, 7);
      expect(editedNpc?.id, _npcId);

      await tester.tap(
        find.byKey(Key('revision3-story-workbench-technical-$_npcId')),
      );
      await tester.pumpAndSettle();
      expect(find.text('GORE_GATE_GUARD'), findsOneWidget);
      expect(find.text('PROJECT.NPCS.GATEGUARD'), findsOneWidget);

      final checksTab = find.byKey(
        Key('revision3-story-workbench-tab-problemsChecks-$_npcId'),
      );
      await tester.ensureVisible(checksTab);
      await tester.tap(checksTab);
      await tester.pumpAndSettle();
      final inspect = find.byKey(
        Key('revision3-story-workbench-action-inspect-npc_draft-$_npcId'),
      );
      expect(inspect, findsOneWidget);
      await tester.tap(inspect);
      await tester.pumpAndSettle();
      expect(inspectCalls, 1);
    },
  );

  testWidgets('NPC Profile exposes a concrete disabled edit reason', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      editNpcProfileDisabledReason: 'Configure the game before editing.',
    );
    await tester.pumpAndSettle();

    final edit = find.byKey(
      Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
    );
    expect(edit, findsOneWidget);
    expect(find.text('Configure the game before editing.'), findsOneWidget);
    expect(
      tester
          .widget<ListTile>(
            find.descendant(of: edit, matching: find.byType(ListTile)),
          )
          .enabled,
      isFalse,
    );
  });

  testWidgets(
    'compact NPC Profile closes its sheet through the same edit path',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 600));
      var editCalls = 0;
      String? editedId;
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(),
        editNpcProfile: (_, npc) async {
          editCalls++;
          editedId = npc.id;
        },
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );

      final edit = find.byKey(
        Key('revision3-story-workbench-action-edit-npc-profile-$_npcId'),
      );
      await tester.ensureVisible(edit);
      await tester.tap(edit);
      await tester.pumpAndSettle();

      expect(editCalls, 1);
      expect(editedId, _npcId);
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
    },
  );

  testWidgets('enabled create callbacks remain direct and visible', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var npcCalls = 0;
    var questCalls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () async => npcCalls++,
      createQuestDraft: () async => questCalls++,
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-npc')),
    );
    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-quest')),
    );
    await tester.pumpAndSettle();

    expect(npcCalls, 1);
    expect(questCalls, 1);
    expect(find.text('NPC creation is not configured.'), findsNothing);
    expect(find.text('Quest creation is not configured.'), findsNothing);
  });

  testWidgets('NPC and Quest creation are mutually exclusive while pending', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final pendingNpc = Completer<void>();
    var npcCalls = 0;
    var questCalls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () {
        npcCalls++;
        return pendingNpc.future;
      },
      createQuestDraft: () async => questCalls++,
    );
    await tester.pumpAndSettle();

    final npc = find.byKey(const Key('revision3-story-workspace-create-npc'));
    final quest = find.byKey(
      const Key('revision3-story-workspace-create-quest'),
    );
    await tester.tap(npc);
    await tester.pump();

    expect(npcCalls, 1);
    expect(tester.widget<FilledButton>(npc).onPressed, isNull);
    expect(tester.widget<OutlinedButton>(quest).onPressed, isNull);
    await tester.tap(quest);
    await tester.pump();
    expect(questCalls, 0, reason: 'a second authoring dialog must not stack');

    pendingNpc.complete();
    await tester.pumpAndSettle();
    expect(tester.widget<FilledButton>(npc).onPressed, isNotNull);
    expect(tester.widget<OutlinedButton>(quest).onPressed, isNotNull);
    await tester.tap(quest);
    await tester.pumpAndSettle();
    expect(questCalls, 1);
  });

  testWidgets('create failures use their dedicated localized formatter', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      createNpcDraft: () async => throw StateError('authoring unavailable'),
      createQuestDraft: () async {},
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-create-npc')),
    );
    await tester.pump();

    expect(
      find.textContaining('CREATE FAILED: Bad state: authoring unavailable'),
      findsOneWidget,
    );
    expect(find.textContaining('Story could not be opened'), findsNothing);
  });

  testWidgets(
    'retains exact selection and Workbench tab across a same-project revision',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var revision = 7;
      var index = _fixture();
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: _workspace(
                  revision: revision,
                  head: 'head-$revision',
                  load: () async => index,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();
      final logicTab = find.byKey(
        Key('revision3-story-workbench-tab-logic-$_questId'),
      );
      await tester.ensureVisible(logicTab);
      await tester.tap(logicTab);
      await tester.pump();
      expect(tester.widget<ChoiceChip>(logicTab).selected, isTrue);

      rebuild(() {
        revision = 8;
        index = _fixture(revision: 8);
      });
      await tester.pumpAndSettle();

      expect(
        find.byKey(
          ValueKey('revision3-story-workspace-workbench-$_projectA-$_questId'),
        ),
        findsOneWidget,
      );
      expect(tester.widget<ChoiceChip>(logicTab).selected, isTrue);

      rebuild(() {
        revision = 9;
        index = _fixture(revision: 9, includeQuest: false);
      });
      await tester.pumpAndSettle();
      expect(logicTab, findsNothing);
      expect(
        find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
        findsOneWidget,
      );

      rebuild(() {
        revision = 10;
        index = _fixture(revision: 10);
      });
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();
      final restoredLogic = find.byKey(
        Key('revision3-story-workbench-tab-logic-$_questId'),
      );
      expect(tester.widget<ChoiceChip>(restoredLogic).selected, isFalse);
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                Key('revision3-story-workbench-tab-overview-$_questId'),
              ),
            )
            .selected,
        isTrue,
      );
    },
  );

  testWidgets(
    'Quest Dialog & Voice hosts the exact transcript UI and retains its line selection',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var revision = 7;
      var index = _fixture();
      late StateSetter rebuild;
      final selectedByBuild = <String?>[];

      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: _workspace(
                  revision: revision,
                  head: 'head-$revision',
                  load: () async => index,
                  questTranscriptBuilder:
                      ({
                        required index,
                        required quest,
                        required selectedLineId,
                        required onSelectedLineChanged,
                      }) {
                        selectedByBuild.add(selectedLineId);
                        return Column(
                          key: const Key('test-quest-transcript'),
                          children: [
                            const Text('Friendly Quest transcript'),
                            FilledButton(
                              key: const Key('test-select-transcript-line'),
                              onPressed: () => onSelectedLineChanged('line-a'),
                              child: const Text('Select first line'),
                            ),
                          ],
                        );
                      },
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();
      final dialogTab = find.byKey(
        Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
      );
      await tester.ensureVisible(dialogTab);
      await tester.tap(dialogTab);
      await tester.pump();

      expect(find.byKey(const Key('test-quest-transcript')), findsOneWidget);
      expect(find.text('Friendly Quest transcript'), findsOneWidget);
      expect(find.text('Not modeled yet'), findsNothing);
      await tester.tap(find.byKey(const Key('test-select-transcript-line')));
      await tester.pump();
      expect(selectedByBuild.last, 'line-a');

      rebuild(() {
        revision = 8;
        index = _fixture(revision: 8);
      });
      await tester.pumpAndSettle();

      expect(tester.widget<ChoiceChip>(dialogTab).selected, isTrue);
      expect(find.byKey(const Key('test-quest-transcript')), findsOneWidget);
      expect(selectedByBuild.last, 'line-a');
    },
  );

  testWidgets(
    'Quest journey is the default overview and opens an exact transcript row',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(includeTranscriptLine: true),
        questJourneyBuilder:
            ({required index, required quest, required onOpenDialogLine}) =>
                FilledButton(
                  key: const Key('test-journey-open-line'),
                  onPressed: () => onOpenDialogLine(_transcriptLineId),
                  child: const Text('Open journey dialog'),
                ),
        questTranscriptBuilder:
            ({
              required index,
              required quest,
              required selectedLineId,
              required onSelectedLineChanged,
            }) => Text('Transcript selection: ${selectedLineId ?? 'none'}'),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pump();

      expect(find.byKey(const Key('test-journey-open-line')), findsOneWidget);
      expect(find.text('Technical ID'), findsNothing);

      await tester.tap(find.byKey(const Key('test-journey-open-line')));
      await tester.pump();

      final dialogTab = find.byKey(
        Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
      );
      expect(tester.widget<ChoiceChip>(dialogTab).selected, isTrue);
      expect(
        find.text('Transcript selection: $_transcriptLineId'),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'compact Quest journey owns scrolling and opens the exact transcript row',
    (tester) async {
      await _setSurfaceSize(tester, const Size(560, 760));
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(includeTranscriptLine: true),
        questJourneyBuilder:
            ({required index, required quest, required onOpenDialogLine}) =>
                SingleChildScrollView(
                  key: const Key('test-compact-journey-scroll'),
                  child: Column(
                    children: [
                      FilledButton(
                        key: const Key('test-compact-journey-open-line'),
                        onPressed: () => onOpenDialogLine(_transcriptLineId),
                        child: const Text('Open compact journey dialog'),
                      ),
                      const SizedBox(height: 900),
                      const Text(
                        'General dialog reached',
                        key: Key('test-compact-journey-general-dialog'),
                      ),
                    ],
                  ),
                ),
        questTranscriptBuilder:
            ({
              required index,
              required quest,
              required selectedLineId,
              required onSelectedLineChanged,
            }) => Text('Transcript selection: ${selectedLineId ?? 'none'}'),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pumpAndSettle();
      final sheet = find.byKey(
        const Key('revision3-story-workspace-details-sheet'),
      );
      expect(sheet, findsOneWidget);

      final verticalScrollables = find
          .descendant(of: sheet, matching: find.byType(Scrollable))
          .evaluate()
          .where((element) {
            final scrollable = element.widget as Scrollable;
            return axisDirectionToAxis(scrollable.axisDirection) ==
                Axis.vertical;
          });
      expect(verticalScrollables.length, 1);

      final journeyScroll = find
          .descendant(
            of: find.byKey(const Key('test-compact-journey-scroll')),
            matching: find.byType(Scrollable),
          )
          .first;
      final general = find.byKey(
        const Key('test-compact-journey-general-dialog'),
      );
      await tester.scrollUntilVisible(general, 240, scrollable: journeyScroll);
      expect(general.hitTestable(), findsOneWidget);

      final openLine = find.byKey(const Key('test-compact-journey-open-line'));
      await tester.ensureVisible(openLine);
      await tester.pump();
      await tester.tap(openLine);
      await tester.pump();

      final dialogTab = find.byKey(
        Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
      );
      expect(tester.widget<ChoiceChip>(dialogTab).selected, isTrue);
      expect(
        find.text('Transcript selection: $_transcriptLineId'),
        findsOneWidget,
      );
    },
  );

  testWidgets('NPC Dialog & Voice remains unavailable with a Quest builder', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      questTranscriptBuilder:
          ({
            required index,
            required quest,
            required selectedLineId,
            required onSelectedLineChanged,
          }) => const Text('Friendly Quest transcript'),
    );
    await tester.pumpAndSettle();

    final dialogTab = find.byKey(
      Key('revision3-story-workbench-tab-dialogVoice-$_npcId'),
    );
    await tester.ensureVisible(dialogTab);
    await tester.tap(dialogTab);
    await tester.pump();

    expect(find.text('Friendly Quest transcript'), findsNothing);
    expect(
      find.text(
        'Dialog, localization, and voice relationships are not modeled for NPC drafts yet.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('project switch resets search, filter, selection, and tabs', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var projectId = _projectA;
    var root = 'root-a';
    var index = _fixture();
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                root: root,
                projectId: projectId,
                load: () async => index,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-filter-quest')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-story-workspace-search')),
      'homer',
    );
    await tester.tap(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
    );
    await tester.pump();
    final references = find.byKey(
      Key('revision3-story-workbench-tab-references-$_questId'),
    );
    await tester.ensureVisible(references);
    await tester.tap(references);
    await tester.pump();

    rebuild(() {
      projectId = _projectB;
      root = 'root-b';
      index = _fixture(
        projectId: _projectB,
        includeQuest: false,
        projectName: 'Other project',
      );
    });
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-story-workspace-search')),
          )
          .controller!
          .text,
      isEmpty,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(const Key('revision3-story-workspace-filter-all')),
          )
          .selected,
      isTrue,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
      findsOneWidget,
    );
  });

  testWidgets('360px and short 640x420 use list-to-details sheets', (
    tester,
  ) async {
    for (final size in const <Size>[Size(360, 760), Size(640, 420)]) {
      await tester.binding.setSurfaceSize(size);
      await _pumpWorkspace(tester, load: () async => _fixture());
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-wide')),
        findsNothing,
      );
      final quest = find.byKey(
        Key('revision3-story-workspace-entity-$_questId'),
      );
      await tester.scrollUntilVisible(
        quest,
        80,
        scrollable: find.descendant(
          of: find.byKey(const Key('revision3-story-workspace-list')),
          matching: find.byType(Scrollable),
        ),
      );
      await tester.pump();
      expect(quest.hitTestable(), findsOneWidget);
      await tester.tap(quest);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );
      expect(
        find.byKey(Key('revision3-story-workbench-tab-overview-$_questId')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull, reason: 'viewport $size');

      await tester.tapAt(const Offset(4, 4));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    }
    await tester.binding.setSurfaceSize(null);
  });

  testWidgets(
    'long German chrome stays usable and leaves a real list at tight height',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 420));
      const disabledReason =
          'Vor dem Erstellen muss eine vollständige Gothic-Spielinstallation ausgewählt und sicher geprüft werden.';
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(),
        copy: _longGermanCopy,
        createNpcDraftDisabledReason: disabledReason,
        createQuestDraftDisabledReason: disabledReason,
      );
      await tester.pumpAndSettle();

      final chrome = find.byKey(
        const Key('revision3-story-workspace-tight-chrome-scroll'),
      );
      final list = find.byKey(const Key('revision3-story-workspace-list'));
      expect(chrome, findsOneWidget);
      expect(list, findsOneWidget);
      expect(tester.getSize(list).height, greaterThanOrEqualTo(128));
      expect(
        find.byKey(const Key('revision3-story-workspace-authority-notice')),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-story-workspace-create-npc-disabled-reason'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(
          const Key('revision3-story-workspace-create-quest-disabled-reason'),
        ),
        findsNothing,
        reason: 'one identical visible setup reason is sufficient',
      );
      expect(
        find
            .byKey(const Key('revision3-story-workspace-create-npc'))
            .hitTestable(),
        findsOneWidget,
      );

      final search = find.byKey(const Key('revision3-story-workspace-search'));
      await tester.scrollUntilVisible(
        search,
        60,
        scrollable: find
            .descendant(of: chrome, matching: find.byType(Scrollable))
            .first,
      );
      await tester.pump();
      expect(search.hitTestable(), findsOneWidget);
      expect(
        find
            .byKey(Key('revision3-story-workspace-entity-$_npcId'))
            .hitTestable(),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'checkpoint and project changes close compact sheets and stale actions fail closed',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 600));
      var root = 'root-a';
      var projectId = _projectA;
      var revision = 7;
      var head = 'head-7';
      var index = _fixture();
      var editOverviewCalls = 0;
      var editStoryCalls = 0;
      var editLogicCalls = 0;
      var inspectQuestCalls = 0;
      var editNpcProfileCalls = 0;
      var inspectNpcCalls = 0;
      var externalEntityCalls = 0;
      var externalAssetCalls = 0;
      late StateSetter rebuild;

      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: _workspace(
                  root: root,
                  projectId: projectId,
                  revision: revision,
                  head: head,
                  load: () async => index,
                  editQuestOutline: (_, _) async => editOverviewCalls++,
                  editQuestContext: (_, _) async => editStoryCalls++,
                  editQuestTransitions: (_, _) async => editLogicCalls++,
                  inspectQuestSource: (_, _) async => inspectQuestCalls++,
                  editNpcProfile: (_, _) async => editNpcProfileCalls++,
                  inspectNpcSource: (_, _) async => inspectNpcCalls++,
                  onOpenExternalEntity: (_) => externalEntityCalls++,
                  onOpenExternalAsset: (_) => externalAssetCalls++,
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pumpAndSettle();
      final questWorkbench = tester.widget<Revision3StoryEntityWorkbench>(
        find.byType(Revision3StoryEntityWorkbench),
      );
      final staleQuestActions = questWorkbench.actions;
      expect(staleQuestActions.editOverview, isNotNull);
      expect(staleQuestActions.editStory, isNotNull);
      expect(staleQuestActions.editLogic, isNotNull);
      expect(staleQuestActions.inspectQuest, isNotNull);

      rebuild(() {
        revision = 8;
        head = 'head-8';
        index = _fixture(revision: 8);
      });
      await tester.pump();
      await staleQuestActions.editOverview!();
      await staleQuestActions.editStory!();
      await staleQuestActions.editLogic!();
      await staleQuestActions.inspectQuest!();
      staleQuestActions.openEntity(_moduleId);
      staleQuestActions.openAsset(_artifactSha);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(editOverviewCalls, 0);
      expect(editStoryCalls, 0);
      expect(editLogicCalls, 0);
      expect(inspectQuestCalls, 0);
      expect(externalEntityCalls, 0);
      expect(externalAssetCalls, 0);

      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_npcId')),
      );
      await tester.pumpAndSettle();
      final npcWorkbench = tester.widget<Revision3StoryEntityWorkbench>(
        find.byType(Revision3StoryEntityWorkbench),
      );
      final staleNpcEdit = npcWorkbench.actions.editNpcProfile!;
      final staleNpcInspect = npcWorkbench.actions.inspectNpc!;

      rebuild(() {
        root = 'root-b';
        projectId = _projectB;
        index = _fixture(projectId: _projectB, revision: 8);
      });
      await tester.pump();
      await staleNpcEdit();
      await staleNpcInspect();
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(editNpcProfileCalls, 0);
      expect(inspectNpcCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'disposing the project workspace removes its open details sheet',
    (tester) async {
      await _setSurfaceSize(tester, const Size(640, 600));
      var showWorkspace = true;
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Scaffold(
                body: showWorkspace
                    ? _workspace(load: () async => _fixture())
                    : const SizedBox(key: Key('project-workspace-closed')),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsOneWidget,
      );

      rebuild(() => showWorkspace = false);
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('project-workspace-closed')), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-story-workspace-details-sheet')),
        findsNothing,
      );
      expect(find.byType(Revision3StoryEntityWorkbench), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('rejects stale async results after an exact revision change', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final oldLoad = Completer<Revision3ContentIndex>();
    final freshLoad = Completer<Revision3ContentIndex>();
    var revision = 7;
    Revision3StoryWorkspaceLoader loader = () => oldLoad.future;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                revision: revision,
                head: 'head-$revision',
                load: loader,
              ),
            );
          },
        ),
      ),
    );
    await tester.pump();

    rebuild(() {
      revision = 8;
      loader = () => freshLoad.future;
    });
    await tester.pump();
    oldLoad.complete(_fixture(projectName: 'Stale result'));
    await tester.pump();
    expect(find.text('Stale result'), findsNothing);
    expect(
      find.byKey(const Key('revision3-story-workspace-loading')),
      findsOneWidget,
    );

    freshLoad.complete(_fixture(revision: 8, projectName: 'Fresh result'));
    await tester.pumpAndSettle();
    expect(find.text('2 drafts / revision 8'), findsOneWidget);
    expect(find.text('Stale result'), findsNothing);
  });

  testWidgets('shows an exact load error and retries', (tester) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    var calls = 0;
    await _pumpWorkspace(
      tester,
      load: () {
        calls++;
        if (calls == 1) return Future.error(StateError('offline'));
        return Future.value(_fixture());
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-story-workspace-error')),
      findsOneWidget,
    );
    expect(find.textContaining('offline'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-story-workspace-retry')));
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text('2 drafts / revision 7'), findsOneWidget);
  });

  testWidgets('rejects a loader result from another checkpoint', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 700));
    await _pumpWorkspace(tester, revision: 8, load: () async => _fixture());
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-story-workspace-error')),
      findsOneWidget,
    );
    expect(
      find.text('Story index does not match this checkpoint.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-story-workspace-list')),
      findsNothing,
    );
  });

  testWidgets('root and exact head changes independently force reloads', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    var root = 'root-a';
    var head = 'head-a';
    var calls = 0;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                root: root,
                head: head,
                load: () async {
                  calls++;
                  return _fixture();
                },
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(calls, 1);
    rebuild(() {});
    await tester.pumpAndSettle();
    expect(calls, 1, reason: 'loader closure identity is not a checkpoint');
    rebuild(() => head = 'head-b');
    await tester.pumpAndSettle();
    expect(calls, 2);
    rebuild(() => root = 'root-b');
    await tester.pumpAndSettle();
    expect(calls, 3);
  });

  testWidgets('controller selects a just-created exact next-revision entity', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3StoryWorkspaceController();
    addTearDown(controller.dispose);
    var revision = 7;
    var index = _fixture(includeQuest: false);
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                revision: revision,
                head: 'head-$revision',
                load: () async => index,
                controller: controller,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const Key('revision3-story-workspace-filter-npc')),
    );
    await tester.enterText(
      find.byKey(const Key('revision3-story-workspace-search')),
      'guard',
    );
    final selected = controller.selectEntityAtRevision(
      entityId: _questId,
      projectRevision: 8,
      projectHeadCanonicalJson: 'head-8',
      section: Revision3StoryWorkbenchSection.logic,
    );
    bool? resolved;
    selected.then((value) => resolved = value);
    await tester.pump();
    expect(resolved, isNull);

    rebuild(() {
      revision = 8;
      index = _fixture(revision: 8);
    });
    await tester.pumpAndSettle();

    expect(await selected, isTrue);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-story-workspace-search')),
          )
          .controller!
          .text,
      isEmpty,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(const Key('revision3-story-workspace-filter-all')),
          )
          .selected,
      isTrue,
    );
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(Key('revision3-story-workbench-tab-logic-$_questId')),
          )
          .selected,
      isTrue,
    );
    expect(
      await controller.selectEntityAtRevision(
        entityId: _moduleId,
        projectRevision: 8,
        projectHeadCanonicalJson: 'head-8',
      ),
      isFalse,
    );
    expect(
      await controller.selectEntityAtRevision(
        entityId: _questId,
        projectRevision: 8,
        projectHeadCanonicalJson: 'different-head-at-revision-8',
      ),
      isFalse,
    );
    final unresolved = controller.selectEntityAtRevision(
      entityId: _npcId,
      projectRevision: 9,
      projectHeadCanonicalJson: 'head-9',
    );
    controller.dispose();
    expect(await unresolved, isFalse);
  });

  testWidgets('controller deep-links only an exact Quest transcript row', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    final controller = Revision3StoryWorkspaceController();
    addTearDown(controller.dispose);
    await _pumpWorkspace(
      tester,
      controller: controller,
      load: () async => _fixture(includeTranscriptLine: true),
      questTranscriptBuilder:
          ({
            required index,
            required quest,
            required selectedLineId,
            required onSelectedLineChanged,
          }) => Text('Deep-linked line: ${selectedLineId ?? 'none'}'),
    );
    await tester.pumpAndSettle();

    expect(
      await controller.selectEntityAtRevision(
        entityId: _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-7',
        section: Revision3StoryWorkbenchSection.dialogVoice,
        selectedLineId: _transcriptLineId,
      ),
      isTrue,
    );
    await tester.pump();

    expect(find.text('Deep-linked line: $_transcriptLineId'), findsOneWidget);
    expect(
      tester
          .widget<ChoiceChip>(
            find.byKey(
              Key('revision3-story-workbench-tab-dialogVoice-$_questId'),
            ),
          )
          .selected,
      isTrue,
    );
    expect(
      await controller.selectEntityAtRevision(
        entityId: _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-7',
        section: Revision3StoryWorkbenchSection.dialogVoice,
        selectedLineId: 'missing-line',
      ),
      isFalse,
    );
    expect(
      await controller.selectEntityAtRevision(
        entityId: _questId,
        projectRevision: 7,
        projectHeadCanonicalJson: 'head-7',
        section: Revision3StoryWorkbenchSection.overview,
        selectedLineId: _transcriptLineId,
      ),
      isFalse,
    );
  });

  test('removal preflight proves the pair and filters incoming ownership', () {
    final exactIndex = _fixture();
    final exact = Revision3StoryDraftRemovalPreflight.fromIndex(
      index: exactIndex,
      draft: exactIndex.entityById(_questId)!,
    );
    expect(exact.hasExactPair, isTrue);
    expect(exact.scriptModule?.id, _moduleId);
    expect(exact.canRemove, isTrue);

    final blockedIndex = _fixture(
      includeBlocker: true,
      blockerExpectedKind: 'npc_draft',
      blockerResolution: 'kind_mismatch',
    );
    final blocked = Revision3StoryDraftRemovalPreflight.fromIndex(
      index: blockedIndex,
      draft: blockedIndex.entityById(_questId)!,
    );
    expect(blocked.hasExactPair, isTrue);
    expect(blocked.canRemove, isFalse);
    expect(blocked.blockers, hasLength(1));
    expect(blocked.blockers.single.source.id, _blockerId);
    expect(blocked.blockers.single.reference.role, 'script_owner');

    final foreignIndex = _fixture(
      includeBlocker: true,
      blockerProjectId: _projectB,
      blockerResolution: 'foreign_project',
    );
    final foreign = Revision3StoryDraftRemovalPreflight.fromIndex(
      index: foreignIndex,
      draft: foreignIndex.entityById(_questId)!,
    );
    expect(foreign.canRemove, isTrue);
    expect(foreign.blockers, isEmpty);

    final transcriptIndex = _fixture(includeTranscriptLine: true);
    final transcript = Revision3StoryDraftRemovalPreflight.fromIndex(
      index: transcriptIndex,
      draft: transcriptIndex.entityById(_questId)!,
    );
    expect(transcript.canRemove, isTrue);
    expect(transcript.blockers, isEmpty);
    expect(
      transcriptIndex.entityById(_transcriptLineId)?.kind,
      Revision3ContentEntityKind.dialogLine,
      reason: 'removal only drops the Quest edge, never its target line',
    );
  });

  testWidgets(
    'wide removal confirms both entities, cancel is inert, and pending action cannot double-call',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final pending = Completer<void>();
      var calls = 0;
      await _pumpWorkspace(
        tester,
        load: () async => _fixture(),
        removeDraft: ({required index, required draft, required scriptModule}) {
          calls++;
          expect(index.projectRevision, 7);
          expect(draft.id, _questId);
          expect(scriptModule.id, _moduleId);
          return pending.future;
        },
      );
      await tester.pumpAndSettle();

      await _openQuestRemovalMenu(tester);
      await tester.tap(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-story-remove-dialog')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-story-remove-draft-name')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('revision3-story-remove-draft-name')),
          matching: find.text('Find Homer'),
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-story-remove-script-name')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('revision3-story-remove-script-name')),
          matching: find.text('Find Homer source'),
        ),
        findsOneWidget,
      );
      expect(find.text('This cannot be undone in version 1.'), findsOneWidget);
      expect(
        find.text('Game files and save games stay unchanged.'),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('revision3-story-remove-cancel')));
      await tester.pumpAndSettle();
      expect(calls, 0);

      await _openQuestRemovalMenu(tester);
      await tester.tap(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-story-remove-confirm')));
      await tester.pump();
      expect(calls, 1);

      await tester.tap(
        find.byKey(Key('revision3-story-workbench-more-$_questId')),
      );
      await tester.pumpAndSettle();
      final disabled = tester.widget<PopupMenuItem<Object?>>(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
      );
      expect(disabled.enabled, isFalse);
      expect(find.text('Another Story action is busy.'), findsOneWidget);
      await tester.tap(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
        warnIfMissed: false,
      );
      await tester.tap(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
        warnIfMissed: false,
      );
      expect(calls, 1);
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();

      pending.complete();
      await tester.pumpAndSettle();
      expect(calls, 1);
      expect(find.text('REMOVED: Find Homer'), findsOneWidget);
    },
  );

  testWidgets('successful removal reloads the new checkpoint and falls back', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    var revision = 7;
    var head = 'head-7';
    var currentIndex = _fixture();
    var calls = 0;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return Scaffold(
              body: _workspace(
                revision: revision,
                head: head,
                load: () async => currentIndex,
                removeDraft:
                    ({
                      required index,
                      required draft,
                      required scriptModule,
                    }) async {
                      calls++;
                      expect(draft.id, _questId);
                      expect(scriptModule.id, _moduleId);
                      rebuild(() {
                        revision = 8;
                        head = 'head-8';
                        currentIndex = _fixture(
                          revision: 8,
                          includeQuest: false,
                        );
                      });
                    },
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    await _openQuestRemovalMenu(tester);
    await tester.tap(
      find.byKey(Key('revision3-story-workbench-remove-$_questId')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-story-remove-confirm')));
    await tester.pumpAndSettle();

    expect(calls, 1);
    expect(find.text('1 drafts / revision 8'), findsOneWidget);
    expect(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
      findsNothing,
    );
    expect(
      find.byKey(Key('revision3-story-workbench-tab-profile-$_npcId')),
      findsOneWidget,
    );
  });

  testWidgets('compact details sheet exposes the same direct removal action', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 720));
    var calls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      removeDraft:
          ({required index, required draft, required scriptModule}) async {
            calls++;
          },
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-story-workspace-details-sheet')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(Key('revision3-story-workbench-more-$_questId')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(Key('revision3-story-workbench-remove-$_questId')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-story-remove-dialog')),
      findsOneWidget,
    );
    expect(find.text('Find Homer source'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-story-remove-cancel')));
    await tester.pumpAndSettle();
    expect(calls, 0);
    expect(
      find.byKey(const Key('revision3-story-workspace-details-sheet')),
      findsOneWidget,
    );
  });

  testWidgets('incoming blocker is listed and opens its external source', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    String? openedEntity;
    var removeCalls = 0;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(includeBlocker: true),
      removeDraft:
          ({required index, required draft, required scriptModule}) async {
            removeCalls++;
          },
      onOpenExternalEntity: (entityId) => openedEntity = entityId,
    );
    await tester.pumpAndSettle();
    await _openQuestRemovalMenu(tester);

    final remove = tester.widget<PopupMenuItem<Object?>>(
      find.byKey(Key('revision3-story-workbench-remove-$_questId')),
    );
    expect(remove.enabled, isFalse);
    expect(find.text('1 removal blockers.'), findsOneWidget);
    await tester.tap(
      find.byKey(
        Key('revision3-story-workbench-review-remove-blockers-$_questId'),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-story-remove-blockers-dialog')),
      findsOneWidget,
    );
    expect(
      find.text('Referencing helper source · script_owner'),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(
        Key('revision3-story-remove-blocker-$_blockerId-script_owner-0'),
      ),
    );
    await tester.pumpAndSettle();
    expect(openedEntity, _blockerId);
    expect(removeCalls, 0);
  });

  testWidgets('reopen lock disables removal with its concrete reason', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      removeDraftDisabledReason: 'Reopen this managed project first.',
    );
    await tester.pumpAndSettle();
    await _openQuestRemovalMenu(tester);
    final remove = tester.widget<PopupMenuItem<Object?>>(
      find.byKey(Key('revision3-story-workbench-remove-$_questId')),
    );
    expect(remove.enabled, isFalse);
    expect(find.text('Reopen this managed project first.'), findsOneWidget);
  });

  testWidgets('draft without an exact generated script pair is disabled', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      removeDraft:
          ({required index, required draft, required scriptModule}) async {},
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(Key('revision3-story-workbench-more-$_npcId')));
    await tester.pumpAndSettle();
    final remove = tester.widget<PopupMenuItem<Object?>>(
      find.byKey(Key('revision3-story-workbench-remove-$_npcId')),
    );
    expect(remove.enabled, isFalse);
    expect(find.text('Draft pair unavailable.'), findsOneWidget);
  });

  testWidgets(
    'new blocker after confirmation refreshes and never retries automatically',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      var currentIndex = _fixture();
      var calls = 0;
      await _pumpWorkspace(
        tester,
        load: () async => currentIndex,
        removeDraft:
            ({required index, required draft, required scriptModule}) async {
              calls++;
              currentIndex = _fixture(includeBlocker: true);
              throw StateError('new incoming reference');
            },
      );
      await tester.pumpAndSettle();
      await _openQuestRemovalMenu(tester);
      await tester.tap(
        find.byKey(Key('revision3-story-workbench-remove-$_questId')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-story-remove-confirm')));
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(
        find.byKey(const Key('revision3-story-remove-blockers-dialog')),
        findsOneWidget,
      );
      expect(
        find.text('Referencing helper source · script_owner'),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-story-remove-blockers-close')),
      );
      await tester.pumpAndSettle();
      expect(calls, 1);
    },
  );

  testWidgets('routes non-Story entities and assets to external owners', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1200, 800));
    String? externalEntity;
    String? externalAsset;
    await _pumpWorkspace(
      tester,
      load: () async => _fixture(),
      onOpenExternalEntity: (value) => externalEntity = value,
      onOpenExternalAsset: (value) => externalAsset = value,
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(Key('revision3-story-workspace-entity-$_questId')),
    );
    await tester.pump();
    final references = find.byKey(
      Key('revision3-story-workbench-tab-references-$_questId'),
    );
    await tester.ensureVisible(references);
    await tester.tap(references);
    await tester.pump();

    final moduleReference = find.byKey(
      Key('revision3-story-workbench-outgoing-$_questId-draft_script_module-0'),
    );
    await tester.ensureVisible(moduleReference);
    await tester.tap(moduleReference);
    expect(externalEntity, _moduleId);

    final assetReference = find.byKey(
      Key(
        'revision3-story-workbench-outgoing-asset-$_questId-quest_collision_artifact-0',
      ),
    );
    await tester.ensureVisible(assetReference);
    await tester.tap(assetReference);
    expect(externalAsset, _artifactSha);
  });
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _openQuestRemovalMenu(WidgetTester tester) async {
  await tester.tap(
    find.byKey(Key('revision3-story-workspace-entity-$_questId')),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(Key('revision3-story-workbench-more-$_questId')));
  await tester.pumpAndSettle();
}

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  required Revision3StoryWorkspaceLoader load,
  int revision = 7,
  Revision3StoryWorkspaceController? controller,
  Revision3StoryWorkspaceCreateAction? createNpcDraft,
  Revision3StoryWorkspaceCreateAction? createQuestDraft,
  ValueChanged<String>? onOpenExternalEntity,
  ValueChanged<String>? onOpenExternalAsset,
  Revision3StoryWorkspaceEntityAction? editQuestOutline,
  Revision3StoryWorkspaceEntityAction? editNpcProfile,
  Revision3StoryWorkspaceEntityAction? editQuestContext,
  Revision3StoryWorkspaceEntityAction? editQuestTransitions,
  Revision3StoryWorkspaceEntityAction? inspectQuestSource,
  Revision3StoryWorkspaceEntityAction? inspectNpcSource,
  Revision3StoryWorkspaceRemoveDraftAction? removeDraft,
  String? removeDraftDisabledReason,
  String? editNpcProfileDisabledReason,
  Revision3StoryWorkspaceCopy? copy,
  String? createNpcDraftDisabledReason,
  String? createQuestDraftDisabledReason,
  Revision3StoryQuestJourneyBuilder? questJourneyBuilder,
  Revision3StoryQuestTranscriptBuilder? questTranscriptBuilder,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: _workspace(
        revision: revision,
        head: 'head-$revision',
        load: load,
        controller: controller,
        createNpcDraft: createNpcDraft,
        createQuestDraft: createQuestDraft,
        onOpenExternalEntity: onOpenExternalEntity,
        onOpenExternalAsset: onOpenExternalAsset,
        editQuestOutline: editQuestOutline,
        editNpcProfile: editNpcProfile,
        editQuestContext: editQuestContext,
        editQuestTransitions: editQuestTransitions,
        inspectQuestSource: inspectQuestSource,
        inspectNpcSource: inspectNpcSource,
        removeDraft: removeDraft,
        removeDraftDisabledReason: removeDraftDisabledReason,
        editNpcProfileDisabledReason: editNpcProfileDisabledReason,
        copy: copy,
        createNpcDraftDisabledReason: createNpcDraftDisabledReason,
        createQuestDraftDisabledReason: createQuestDraftDisabledReason,
        questJourneyBuilder: questJourneyBuilder,
        questTranscriptBuilder: questTranscriptBuilder,
      ),
    ),
  ),
);

Revision3StoryWorkspace _workspace({
  String root = 'root-a',
  String projectId = _projectA,
  int revision = 7,
  String head = 'head-7',
  required Revision3StoryWorkspaceLoader load,
  Revision3StoryWorkspaceController? controller,
  Revision3StoryWorkspaceCreateAction? createNpcDraft,
  Revision3StoryWorkspaceCreateAction? createQuestDraft,
  ValueChanged<String>? onOpenExternalEntity,
  ValueChanged<String>? onOpenExternalAsset,
  Revision3StoryWorkspaceEntityAction? editQuestOutline,
  Revision3StoryWorkspaceEntityAction? editNpcProfile,
  Revision3StoryWorkspaceEntityAction? editQuestContext,
  Revision3StoryWorkspaceEntityAction? editQuestTransitions,
  Revision3StoryWorkspaceEntityAction? inspectQuestSource,
  Revision3StoryWorkspaceEntityAction? inspectNpcSource,
  Revision3StoryWorkspaceRemoveDraftAction? removeDraft,
  String? removeDraftDisabledReason,
  String? editNpcProfileDisabledReason,
  Revision3StoryWorkspaceCopy? copy,
  String? createNpcDraftDisabledReason,
  String? createQuestDraftDisabledReason,
  Revision3StoryQuestJourneyBuilder? questJourneyBuilder,
  Revision3StoryQuestTranscriptBuilder? questTranscriptBuilder,
}) => Revision3StoryWorkspace(
  projectRoot: root,
  projectId: projectId,
  projectRevision: revision,
  projectHeadCanonicalJson: head,
  load: load,
  copy: copy ?? _copy,
  controller: controller,
  createNpcDraft: createNpcDraft,
  createQuestDraft: createQuestDraft,
  createNpcDraftDisabledReason: createNpcDraft == null
      ? createNpcDraftDisabledReason ?? 'NPC creation is not configured.'
      : null,
  createQuestDraftDisabledReason: createQuestDraft == null
      ? createQuestDraftDisabledReason ?? 'Quest creation is not configured.'
      : null,
  onOpenExternalEntity: onOpenExternalEntity ?? (_) {},
  onOpenExternalAsset: onOpenExternalAsset ?? (_) {},
  editQuestOutline: editQuestOutline,
  editNpcProfile: editNpcProfile,
  editQuestContext: editQuestContext,
  editQuestTransitions: editQuestTransitions,
  inspectQuestSource: inspectQuestSource,
  inspectNpcSource: inspectNpcSource,
  removeDraft: removeDraft,
  removeDraftDisabledReason: removeDraftDisabledReason,
  editNpcProfileDisabledReason: editNpcProfileDisabledReason,
  questJourneyBuilder: questJourneyBuilder,
  questTranscriptBuilder: questTranscriptBuilder,
);

Revision3ContentIndex _fixture({
  String projectId = _projectA,
  int revision = 7,
  String projectName = 'Fixture project',
  bool includeNpc = true,
  bool includeQuest = true,
  bool includeBlocker = false,
  String? blockerProjectId,
  String blockerExpectedKind = 'quest_draft',
  String blockerResolution = 'resolved',
  bool includeTranscriptLine = false,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': projectName,
  'project_version': '0.1.0',
  'project_author': 'GORE',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 123, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{
    if (includeTranscriptLine) 'localization_entry': 1,
    if (includeTranscriptLine) 'dialog_line': 1,
    if (includeNpc) 'npc_draft': 1,
    if (includeQuest) 'quest_draft': 1,
    if (includeQuest) 'script_module': includeBlocker ? 2 : 1,
  },
  'entities': <Object?>[
    if (includeNpc)
      <String, Object?>{
        'id': _npcId,
        'kind': 'npc_draft',
        'display_name': 'Gate Guard',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_GATE_GUARD',
        },
        'summary': <String, Object?>{
          'kind': 'npc_draft',
          'data': <String, Object?>{
            'unique_name': 'GORE_GATE_GUARD',
            'module_namespace': 'PROJECT.NPCS.GATEGUARD',
            'parent_character_definition': 'UCharacterDefinition_Asghan',
            'parent_ai_agent_config': 'UAIAgentConfig_Asghan',
            'parent_spawn_definition': 'USpawnAIAgentDefinition_Asghan',
          },
        },
        'references': <Object?>[],
        'asset_references': <Object?>[],
      },
    if (includeQuest)
      <String, Object?>{
        'id': _questId,
        'kind': 'quest_draft',
        'display_name': 'Find Homer',
        'revision': 1,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_FIND_HOMER',
        },
        'summary': <String, Object?>{
          'kind': 'quest_draft',
          'data': <String, Object?>{
            'technical_id': 'GORE_FIND_HOMER',
            'title': 'Find Homer',
            'objective_title': 'Ask Asghan about Homer',
            if (includeTranscriptLine) 'transcript_count': 1,
            'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
            'parent_runtime_class': 'B_Quest_FindHomer_C',
            'giver_runtime_unique_name': 'ASGHAN',
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'draft_script_module',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': projectId,
              'entity_id': _moduleId,
              'expected_kind': 'script_module',
            },
            'resolution': 'resolved',
          },
          if (includeTranscriptLine)
            <String, Object?>{
              'role': 'quest_transcript_line',
              'qualifier': null,
              'target': <String, Object?>{
                'project_id': projectId,
                'entity_id': _transcriptLineId,
                'expected_kind': 'dialog_line',
              },
              'resolution': 'resolved',
            },
        ],
        'asset_references': <Object?>[
          <String, Object?>{
            'role': 'quest_collision_artifact',
            'sha256': _artifactSha,
            'byte_len': 8192,
            'logical_name': null,
            'expected_media_type':
                'application/vnd.gore.quest-collision-capability+json;version=2',
            'resolution': 'resolved',
          },
        ],
      },
    if (includeQuest)
      <String, Object?>{
        'id': _moduleId,
        'kind': 'script_module',
        'display_name': 'Find Homer source',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.quest-draft',
          'generator_version': 2,
          'owner': <String, Object?>{
            'project_id': projectId,
            'entity_id': _questId,
            'expected_kind': 'quest_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.quest-draft',
            'generator_version': 2,
            'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
            'module_relative_path': 'Project/Quests/FindHomer.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'origin_owner',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': projectId,
              'entity_id': _questId,
              'expected_kind': 'quest_draft',
            },
            'resolution': 'resolved',
          },
          <String, Object?>{
            'role': 'script_owner',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': projectId,
              'entity_id': _questId,
              'expected_kind': 'quest_draft',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[],
      },
    if (includeQuest && includeBlocker)
      <String, Object?>{
        'id': _blockerId,
        'kind': 'script_module',
        'display_name': 'Referencing helper source',
        'revision': 3,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_HELPER_SOURCE',
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'fixture.helper',
            'generator_version': 1,
            'module_namespace': 'PROJECT.HELPER',
            'module_relative_path': 'Project/Helper.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'script_owner',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': blockerProjectId ?? projectId,
              'entity_id': _questId,
              'expected_kind': blockerExpectedKind,
            },
            'resolution': blockerResolution,
          },
        ],
        'asset_references': <Object?>[],
      },
    if (includeTranscriptLine)
      <String, Object?>{
        'id': _transcriptLocalizationId,
        'kind': 'localization_entry',
        'display_name': 'Asghan greeting',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_ASGHAN_GREETING',
        },
        'summary': <String, Object?>{
          'kind': 'localization_entry',
          'data': <String, Object?>{
            'loc_id': 'GORE_ASGHAN_GREETING',
            'locales': <Object?>['de', 'en'],
          },
        },
        'references': <Object?>[],
        'asset_references': <Object?>[],
      },
    if (includeTranscriptLine)
      <String, Object?>{
        'id': _transcriptLineId,
        'kind': 'dialog_line',
        'display_name': 'Asghan greeting line',
        'revision': 0,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_ASGHAN_GREETING_LINE',
        },
        'summary': <String, Object?>{
          'kind': 'dialog_line',
          'data': <String, Object?>{
            'speaker_hint': 'Asghan',
            'voice_slot_locales': <Object?>[],
          },
        },
        'references': <Object?>[
          <String, Object?>{
            'role': 'dialog_localization',
            'qualifier': null,
            'target': <String, Object?>{
              'project_id': projectId,
              'entity_id': _transcriptLocalizationId,
              'expected_kind': 'localization_entry',
            },
            'resolution': 'resolved',
          },
        ],
        'asset_references': <Object?>[],
      },
  ],
  'assets': <Object?>[
    if (includeQuest)
      <String, Object?>{
        'sha256': _artifactSha,
        'byte_len': 8192,
        'media_type':
            'application/vnd.gore.quest-collision-capability+json;version=2',
        'class': 'quest_collision_artifact',
      },
  ],
});
