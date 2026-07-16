import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_dialog.dart';
import 'package:gore_mod/project/revision3_voice_take_status_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  testWidgets(
    'dialog has no implicit line choice, marks current, disables no-op and nonapproved',
    (tester) async {
      await _openDialog(tester, index: _index());

      expect(find.byKey(const Key('voice-selection-save')), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-lines')),
          matching: find.textContaining('GRD_263_ASGHAN_OPEN_INFO_06_02'),
        ),
        findsNothing,
      );
      expect(find.textContaining(r'C:\'), findsNothing);

      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();

      expect(find.textContaining('Current selection'), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      final recorded = tester.widget<RadioListTile<String>>(
        find.byKey(const Key('voice-selection-take-1')),
      );
      expect(recorded.enabled, isFalse);
      expect(find.textContaining('Approval required'), findsOneWidget);
    },
  );

  testWidgets('context handoff preselects the exact visible line and locale', (
    tester,
  ) async {
    await _openDialog(
      tester,
      index: _index(),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
    );

    expect(
      tester
          .widget<DropdownButtonFormField<String>>(
            find.byKey(const Key('voice-selection-locale')),
          )
          .initialValue,
      'de',
    );
    expect(find.textContaining('Current selection'), findsOneWidget);
    expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
  });

  testWidgets(
    'fixed context hides global navigation and publishes only its exact slot',
    (tester) async {
      Revision3VoiceTakeSelectionTechnicalPlan? received;
      await _openDialog(
        tester,
        index: _index(),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              received = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );

      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(
        find.byKey(const Key('voice-selection-line-search')),
        findsNothing,
      );
      expect(find.byKey(const Key('voice-selection-lines')), findsNothing);
      expect(find.byKey(const Key('voice-selection-locale')), findsNothing);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);

      await tester.tap(find.byKey(const Key('voice-selection-clear')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(received?.lineId, revision3VoiceContentLineId);
      expect(received?.locale, 'de');
      expect(received?.selectedTakeId, isNull);
    },
  );

  testWidgets('invalid fixed context fails closed without any mutation', (
    tester,
  ) async {
    var selectionPublishes = 0;
    var statusPublishes = 0;
    var removalPublishes = 0;
    var slotRemovalPublishes = 0;
    await _openDialog(
      tester,
      index: _index(),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'en',
      fixedContext: true,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            selectionPublishes++;
            throw StateError('must not publish');
          },
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            statusPublishes++;
            throw StateError('must not publish');
          },
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            removalPublishes++;
            throw StateError('must not publish');
          },
      publishSlotRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            slotRemovalPublishes++;
            throw StateError('must not publish');
          },
    );

    expect(find.textContaining('no longer matches'), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-line-search')), findsNothing);
    expect(find.byKey(const Key('voice-selection-lines')), findsNothing);
    expect(find.byKey(const Key('voice-selection-locale')), findsNothing);
    expect(find.byKey(const Key('voice-selection-clear')), findsNothing);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(
      selectionPublishes +
          statusPublishes +
          removalPublishes +
          slotRemovalPublishes,
      0,
    );
  });

  testWidgets(
    'older catalog completion cannot replace a newer fixed context load',
    (tester) async {
      final olderLoad = Completer<Revision3ContentIndex>();

      Widget app(Future<Revision3ContentIndex> Function() load) {
        return MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Revision3VoiceTakeSelectionDialog(
            key: const Key('selection-dialog-under-test'),
            service: Revision3VoiceTakeSelectionAuthoringService(
              loadContentIndex: load,
              publishTechnicalPlan: _unexpectedPublish,
            ),
            statusService: Revision3VoiceTakeStatusAuthoringService(
              loadContentIndex: load,
              publishTechnicalPlan: _unexpectedStatusPublish,
            ),
            removalService: Revision3VoiceTakeRemovalAuthoringService(
              loadContentIndex: load,
              publishTechnicalPlan: _unexpectedRemovalPublish,
            ),
            slotRemovalService: Revision3DialogVoiceSlotRemovalAuthoringService(
              loadContentIndex: load,
              publishTechnicalPlan: _unexpectedSlotRemovalPublish,
            ),
            initialLineId: revision3VoiceContentLineId,
            initialLocale: 'de',
            fixedContext: true,
          ),
        );
      }

      await tester.pumpWidget(app(() => olderLoad.future));
      await tester.pump();

      await tester.pumpWidget(app(() async => _index()));
      await tester.pump();
      await tester.pump();

      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(find.textContaining('no longer matches'), findsNothing);

      olderLoad.complete(
        revision3VoiceContentIndexFixture(existingDeSlot: false),
      );
      await tester.pump();
      await tester.pump();

      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(find.textContaining('no longer matches'), findsNothing);
    },
  );

  testWidgets(
    'failed fixed-context rebind clears old authority and offers retry',
    (tester) async {
      var selectionPublishes = 0;
      var statusPublishes = 0;
      var removalPublishes = 0;
      var slotRemovalPublishes = 0;
      var failedLoads = 0;

      Future<Revision3VoiceTakeSelectionPublication> publishSelection({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3VoiceTakeSelectionTechnicalPlan plan,
      }) async {
        selectionPublishes++;
        throw StateError('selection publish must stay fail-closed');
      }

      Future<Revision3VoiceTakeStatusPublication> publishStatus({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3VoiceTakeStatusTechnicalPlan plan,
      }) async {
        statusPublishes++;
        throw StateError('status publish must stay fail-closed');
      }

      Future<Revision3VoiceTakeRemovalPublication> publishRemoval({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3VoiceTakeRemovalTechnicalPlan plan,
      }) async {
        removalPublishes++;
        throw StateError('removal publish must stay fail-closed');
      }

      Future<Revision3DialogVoiceSlotRemovalPublication> publishSlotRemoval({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
      }) async {
        slotRemovalPublishes++;
        throw StateError('slot removal publish must stay fail-closed');
      }

      Widget app(Future<Revision3ContentIndex> Function() load) => MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Revision3VoiceTakeSelectionDialog(
          key: const Key('selection-dialog-under-test'),
          service: Revision3VoiceTakeSelectionAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: publishSelection,
          ),
          statusService: Revision3VoiceTakeStatusAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: publishStatus,
          ),
          removalService: Revision3VoiceTakeRemovalAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: publishRemoval,
          ),
          slotRemovalService: Revision3DialogVoiceSlotRemovalAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: publishSlotRemoval,
          ),
          initialLineId: revision3VoiceContentLineId,
          initialLocale: 'de',
          fixedContext: true,
        ),
      );

      await tester.pumpWidget(app(() async => _index()));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-selection-clear')));
      await tester.pump();
      final staleSave = _button(tester, 'voice-selection-save').onPressed;
      expect(staleSave, isNotNull);

      await tester.pumpWidget(
        app(() async {
          failedLoads++;
          throw StateError('replacement catalog unavailable');
        }),
      );
      await tester.pumpAndSettle();

      expect(failedLoads, 1);
      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsNothing,
      );
      expect(find.byKey(const Key('voice-selection-clear')), findsNothing);
      expect(find.byKey(const Key('voice-take-remove-0')), findsNothing);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      final retry = _button(tester, 'voice-selection-retry');
      expect(retry.onPressed, isNotNull);

      staleSave!();
      await tester.pump();
      expect(
        selectionPublishes +
            statusPublishes +
            removalPublishes +
            slotRemovalPublishes,
        0,
      );

      await tester.tap(find.byKey(const Key('voice-selection-retry')));
      await tester.pumpAndSettle();
      expect(failedLoads, 2);
      expect(find.byKey(const Key('voice-selection-retry')), findsOneWidget);
    },
  );

  testWidgets(
    'pending recovery cannot replace a successfully rebound fixed context',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 1000));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final initial = _index();
      final recovered = _index(
        revision: 8,
        secondApproved: true,
        secondTakeRevision: 1,
      );
      final reboundJson = revision3VoiceContentIndexJsonFixture(revision: 9);
      final reboundEntities = (reboundJson['entities']! as List<Object?>)
          .cast<Map<String, Object?>>();
      reboundEntities.singleWhere(
        (entity) => entity['id'] == revision3VoiceContentLineId,
      )['display_name'] = 'Rebound current Voice context';
      final rebound = Revision3ContentIndex.fromJsonObject(reboundJson);
      final oldRecovery = Completer<Revision3ContentIndex>();
      var oldReads = 0;
      var statusPublishes = 0;

      Future<Revision3ContentIndex> oldLoad() async {
        oldReads++;
        if (oldReads == 3) throw StateError('temporary reload failure');
        if (oldReads == 4) return oldRecovery.future;
        return initial;
      }

      Widget app({
        required Future<Revision3ContentIndex> Function() load,
        required String lineId,
        required Revision3VoiceTakeStatusTechnicalPublisher publishStatus,
      }) => MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Revision3VoiceTakeSelectionDialog(
          key: const Key('selection-dialog-under-test'),
          service: Revision3VoiceTakeSelectionAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: _unexpectedPublish,
          ),
          statusService: Revision3VoiceTakeStatusAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: publishStatus,
          ),
          removalService: Revision3VoiceTakeRemovalAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: _unexpectedRemovalPublish,
          ),
          slotRemovalService: Revision3DialogVoiceSlotRemovalAuthoringService(
            loadContentIndex: load,
            publishTechnicalPlan: _unexpectedSlotRemovalPublish,
          ),
          initialLineId: lineId,
          initialLocale: 'de',
          fixedContext: true,
        ),
      );

      await tester.pumpWidget(
        app(
          load: oldLoad,
          lineId: revision3VoiceContentLineId,
          publishStatus:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) async {
                statusPublishes++;
                return _statusPublication(
                  projectId: expectedProjectId,
                  revision: 8,
                  plan: plan,
                );
              },
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-status-change-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
      await tester.pumpAndSettle();
      expect(statusPublishes, 1);
      expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);

      await tester.tap(find.byKey(const Key('voice-status-reload')));
      await tester.pump();
      expect(oldReads, 4);

      await tester.pumpWidget(
        app(
          load: () async => rebound,
          lineId: revision3VoiceContentLineId,
          publishStatus: _unexpectedStatusPublish,
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.textContaining('Rebound current Voice context'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.textContaining('no longer matches'), findsNothing);

      oldRecovery.complete(recovered);
      await tester.pumpAndSettle();

      expect(
        find.textContaining('Rebound current Voice context'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.textContaining('no longer matches'), findsNothing);
      expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    },
  );

  testWidgets(
    'clear warns, stays explicit, and publishes a project-only clear',
    (tester) async {
      Revision3VoiceTakeSelectionTechnicalPlan? received;
      await _openDialog(
        tester,
        index: _index(),
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              received = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-selection-clear')));
      await tester.pump();

      expect(
        find.byKey(const Key('voice-selection-clear-warning')),
        findsOneWidget,
      );
      expect(find.textContaining('takes stay in this project'), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);

      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(received, isNotNull);
      expect(received!.selectedTakeId, isNull);
      expect(received!.expectedSelectedTakeId, isNotNull);
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('only an Approved alternate can be selected and saved', (
    tester,
  ) async {
    final index = _index(secondApproved: true);
    Revision3VoiceTakeSelectionTechnicalPlan? received;
    await _openDialog(
      tester,
      index: index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            received = plan;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    final alternate = tester.widget<RadioListTile<String>>(
      find.byKey(const Key('voice-selection-take-1')),
    );
    expect(alternate.enabled, isTrue);
    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(received?.selectedTakeId, isNotNull);
    expect(received?.selectedTakeId, isNot(received?.expectedSelectedTakeId));
  });

  testWidgets('busy state locks controls and exposes no premature result', (
    tester,
  ) async {
    final completer = Completer<Revision3VoiceTakeSelectionPublication>();
    Revision3VoiceTakeSelectionTechnicalPlan? received;
    await _openDialog(
      tester,
      index: _index(),
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            received = plan;
            return completer.future;
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-clear')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('voice-selection-line-search')),
          )
          .enabled,
      isFalse,
    );

    completer.complete(
      _publication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: received!,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsNothing,
    );
  });

  testWidgets('fresh-index drift is shown as stale and keeps dialog open', (
    tester,
  ) async {
    var reads = 0;
    final initial = _index(secondApproved: true);
    final stale = revision3VoiceContentIndexFixture(
      revision: initial.projectRevision + 1,
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    );
    await _openDialog(
      tester,
      index: initial,
      load: () async => reads++ == 0 ? initial : stale,
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('voice-selection-error')), findsOneWidget);
    expect(find.textContaining('project changed'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsOneWidget,
    );
  });

  testWidgets('reopen load failure is friendly and terminal', (tester) async {
    await _openDialog(
      tester,
      index: _index(),
      load: () async => throw const Revision3ContentRequiresReopenException(),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Reopen the managed project'), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-retry')), findsNothing);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
  });

  testWidgets('shows workflow status and locks the selected Approved take', (
    tester,
  ) async {
    await _openDialog(tester, index: _index());
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();

    expect(find.textContaining('author workflow label'), findsOneWidget);
    expect(find.textContaining('in-game readiness'), findsOneWidget);
    expect(find.textContaining('Approved • Current selection'), findsOneWidget);
    expect(find.textContaining('Recorded'), findsOneWidget);
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
            find.byKey(const Key('voice-status-change-0')),
          )
          .enabled,
      isFalse,
    );
    expect(find.textContaining('Clear the selection'), findsOneWidget);

    final alternate = find.byKey(const Key('voice-status-change-1'));
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(alternate)
          .enabled,
      isTrue,
    );
    await tester.tap(alternate);
    await tester.pumpAndSettle();
    for (final status in AuthoringRevision3VoiceTakeStatus.values) {
      final item = find.byKey(Key('voice-status-option-1-${status.name}'));
      expect(item, findsOneWidget);
      expect(
        tester
            .widget<PopupMenuItem<AuthoringRevision3VoiceTakeStatus>>(item)
            .enabled,
        status == AuthoringRevision3VoiceTakeStatus.recorded ? isFalse : isTrue,
      );
    }
  });

  testWidgets('historical selected Recorded take can only become Approved', (
    tester,
  ) async {
    await _openDialog(tester, index: _index(selectedStatus: 'recorded'));
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();

    expect(
      find.textContaining(
        'Current selection must be Approved; change to Approved or clear it',
      ),
      findsOneWidget,
    );
    final control = find.byKey(const Key('voice-status-change-0'));
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(control)
          .enabled,
      isTrue,
    );
    await tester.tap(control);
    await tester.pumpAndSettle();
    for (final status in AuthoringRevision3VoiceTakeStatus.values) {
      final item = find.byKey(Key('voice-status-option-0-${status.name}'));
      expect(
        tester
            .widget<PopupMenuItem<AuthoringRevision3VoiceTakeStatus>>(item)
            .enabled,
        status == AuthoringRevision3VoiceTakeStatus.approved ? isTrue : isFalse,
      );
    }
  });

  testWidgets('Approved status reloads exactly and is immediately selectable', (
    tester,
  ) async {
    var current = _index();
    var statusCalls = 0;
    Revision3VoiceTakeStatusTechnicalPlan? statusPlan;
    Revision3VoiceTakeSelectionTechnicalPlan? selectionPlan;
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            statusCalls++;
            statusPlan = plan;
            expect(expectedProjectRevision, 7);
            current = _index(
              revision: 8,
              secondApproved: true,
              secondTakeRevision: 1,
            );
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            selectionPlan = plan;
            expect(expectedProjectRevision, 8);
            return _publication(
              projectId: expectedProjectId,
              revision: 9,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();

    expect(statusCalls, 1);
    expect(
      statusPlan?.desiredStatus,
      AuthoringRevision3VoiceTakeStatus.approved,
    );
    expect(find.textContaining('can now be selected'), findsOneWidget);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-take-1')),
          )
          .enabled,
      isTrue,
    );

    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    expect(
      find.byKey(const Key('voice-status-selection-pending')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
            find.byKey(const Key('voice-status-change-1')),
          )
          .enabled,
      isFalse,
    );
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(selectionPlan?.selectedTakeId, statusPlan?.takeId);
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsNothing,
    );
  });

  testWidgets('status publish busy state blocks every second action', (
    tester,
  ) async {
    var current = _index();
    var publishCalls = 0;
    Revision3VoiceTakeStatusTechnicalPlan? received;
    final completer = Completer<Revision3VoiceTakeStatusPublication>();
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            publishCalls++;
            received = plan;
            return completer.future;
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pump();

    expect(publishCalls, 1);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('voice-selection-line-search')),
          )
          .enabled,
      isFalse,
    );
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-clear')),
          )
          .enabled,
      isFalse,
    );

    current = _index(revision: 8, secondApproved: true, secondTakeRevision: 1);
    completer.complete(
      _statusPublication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: received!,
      ),
    );
    await tester.pumpAndSettle();
    expect(publishCalls, 1);
  });

  testWidgets('reload recovery never repeats an already saved status', (
    tester,
  ) async {
    final initial = _index();
    final refreshed = _index(
      revision: 8,
      secondApproved: true,
      secondTakeRevision: 1,
    );
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async {
        reads++;
        if (reads == 3) throw StateError('temporary reload failure');
        return reads >= 4 ? refreshed : initial;
      },
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.textContaining('status was saved'), findsOneWidget);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);

    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.textContaining('Saved status confirmed'), findsOneWidget);
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-take-1')),
          )
          .enabled,
      isTrue,
    );
  });

  testWidgets('reload requiring reopen leaves only Close enabled', (
    tester,
  ) async {
    final initial = _index();
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async {
        reads++;
        if (reads == 3) throw StateError('temporary reload failure');
        if (reads >= 4) {
          throw const Revision3ContentRequiresReopenException();
        }
        return initial;
      },
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.textContaining('reopen the managed project'), findsOneWidget);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNotNull);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
  });

  testWidgets(
    'remove is localized, names its exact scope, and cancel never publishes',
    (tester) async {
      var publishCalls = 0;
      await _openDialog(
        tester,
        index: _index(),
        locale: const Locale('de'),
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              throw StateError('must not publish after cancel');
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();

      expect(find.text('Aus dieser Zeile entfernen…'), findsNWidgets(2));
      await tester.tap(find.byKey(const Key('voice-take-remove-1')));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('voice-take-remove-confirm-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('Mine entrance question'), findsWidgets);
      expect(find.textContaining('(de)'), findsOneWidget);
      expect(
        find.textContaining('keinen Projektspeicher frei'),
        findsOneWidget,
      );
      expect(
        find.textContaining('Spielinstallation und Spielstände'),
        findsOneWidget,
      );
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);

      await tester.tap(find.byKey(const Key('voice-take-remove-cancel')));
      await tester.pumpAndSettle();
      expect(publishCalls, 0);
      expect(
        find.byKey(const Key('voice-take-remove-confirm-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'empty Voice setup removal is explicit and cancel never publishes',
    (tester) async {
      var publishes = 0;
      await _openDialog(
        tester,
        index: _index(candidateCount: 0, selected: false, generatedSlot: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        publishSlotRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              throw StateError('cancel must not publish');
            },
      );

      final remove = find.byKey(const Key('voice-slot-remove-empty'));
      await tester.ensureVisible(remove);
      await tester.tap(remove);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('voice-slot-remove-confirm-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('dialog text stays'), findsOneWidget);
      expect(find.textContaining('game file'), findsOneWidget);
      expect(
        find.textContaining('created again automatically'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('voice-slot-remove-target-warning')),
        findsNothing,
      );
      await tester.tap(find.byKey(const Key('voice-slot-remove-cancel')));
      await tester.pumpAndSettle();
      expect(publishes, 0);
      expect(
        find.byKey(const Key('voice-slot-remove-confirm-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('empty imported Voice setup exposes no destructive action', (
    tester,
  ) async {
    await _openDialog(
      tester,
      index: _index(candidateCount: 0, selected: false),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
    );

    expect(
      find.byKey(const Key('voice-selection-no-candidates')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('voice-slot-remove-empty')), findsNothing);
  });

  testWidgets(
    'empty Voice setup removal warns about target evidence and confirms refresh',
    (tester) async {
      var loads = 0;
      var publishes = 0;
      final initial = revision3VoiceContentIndexFixture(
        existingSlotTargetResolution: 'resolved',
        existingSlotGenerated: true,
      );
      final refreshed = _indexWithoutSlot(revision: 8, lineRevision: 3);
      await _openDialog(
        tester,
        index: initial,
        load: () async => ++loads < 3 ? initial : refreshed,
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        publishSlotRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              return _slotRemovalPublication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );

      final remove = find.byKey(const Key('voice-slot-remove-empty'));
      await tester.ensureVisible(remove);
      await tester.tap(remove);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('voice-slot-remove-target-warning')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('voice-slot-remove-confirm')));
      await tester.pumpAndSettle();

      expect(publishes, 1);
      expect(loads, 3);
      expect(find.textContaining('Empty Voice setup removed'), findsOneWidget);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'uncertain empty setup removal is neutral, terminal, and never retryable',
    (tester) async {
      var publishes = 0;
      await _openDialog(
        tester,
        index: _index(candidateCount: 0, selected: false, generatedSlot: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        publishSlotRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
            },
      );

      await tester.tap(find.byKey(const Key('voice-slot-remove-empty')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-slot-remove-confirm')));
      await tester.pumpAndSettle();

      expect(publishes, 1);
      expect(find.textContaining('may have been saved'), findsOneWidget);
      expect(find.textContaining('Do not repeat'), findsOneWidget);
      expect(find.byKey(const Key('voice-status-reload')), findsNothing);
      expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('voice-slot-remove-empty')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'saved last-slot removal reload clears the now invisible line selection',
    (tester) async {
      final initial = _index(
        candidateCount: 0,
        selected: false,
        generatedSlot: true,
      );
      final refreshed = _indexWithoutSlot(revision: 8, lineRevision: 3);
      var loads = 0;
      var publishes = 0;
      await _openDialog(
        tester,
        index: initial,
        load: () async {
          loads++;
          if (loads == 3) throw StateError('temporary reload failure');
          return loads >= 4 ? refreshed : initial;
        },
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        publishSlotRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              return _slotRemovalPublication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );

      await tester.tap(find.byKey(const Key('voice-slot-remove-empty')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-slot-remove-confirm')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);

      await tester.tap(find.byKey(const Key('voice-status-reload')));
      await tester.pumpAndSettle();

      expect(publishes, 1);
      expect(find.textContaining('confirmed'), findsOneWidget);
      expect(find.byKey(const Key('voice-selection-locale')), findsNothing);
      expect(find.textContaining('No matching dialog line'), findsOneWidget);
    },
  );

  testWidgets(
    'selected take warns, removes and clears atomically without replacement',
    (tester) async {
      var current = _index(candidateCount: 1);
      final takeId = _candidateIds(current).single;
      var publishCalls = 0;
      await _openDialog(
        tester,
        index: current,
        load: () async => current,
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              current = _removedIndex(
                candidateCount: 1,
                selected: true,
                removedTakeId: takeId,
                retainTakeEntity: false,
              );
              return _removalPublication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
                selectionCleared: true,
                takeEntityRemoved: true,
                remainingCandidateCount: 0,
              );
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-take-remove-0')));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('voice-take-remove-selected-warning')),
        findsOneWidget,
      );
      expect(find.textContaining('No replacement is chosen'), findsOneWidget);
      expect(
        find.textContaining('Voice build remains blocked'),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
      await tester.pumpAndSettle();

      expect(publishCalls, 1);
      expect(
        find.textContaining('selection was cleared atomically'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('voice-selection-no-candidates')),
        findsOneWidget,
      );
      expect(find.byKey(const Key('voice-selection-locale')), findsOneWidget);
      expect(
        tester
            .widget<DropdownButtonFormField<String>>(
              find.byKey(const Key('voice-selection-locale')),
            )
            .initialValue,
        'de',
      );
    },
  );

  testWidgets(
    'nonselected removal reloads the same line and preserves its selection',
    (tester) async {
      var current = _index(secondApproved: true);
      final candidates = _candidateIds(current);
      final removedTakeId = candidates[1];
      await _openDialog(
        tester,
        index: current,
        load: () async => current,
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              current = _removedIndex(
                candidateCount: 2,
                selected: true,
                removedTakeId: removedTakeId,
                retainTakeEntity: false,
              );
              return _removalPublication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
                selectionCleared: false,
                takeEntityRemoved: true,
                remainingCandidateCount: 1,
              );
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-take-remove-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('voice-selection-take-0')), findsOneWidget);
      expect(find.byKey(const Key('voice-selection-take-1')), findsNothing);
      expect(find.textContaining('Current selection'), findsOneWidget);
      expect(find.textContaining('current project graph'), findsOneWidget);
      expect(find.byKey(const Key('voice-selection-locale')), findsOneWidget);
    },
  );

  testWidgets('shared outcome says only the line link was removed', (
    tester,
  ) async {
    var current = _index(
      candidateCount: 1,
      selected: false,
      sharedFirstTake: true,
    );
    final takeId = _candidateIds(current).single;
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            expect(plan.expectedTakeEntityRemoved, isFalse);
            current = _removedIndex(
              candidateCount: 1,
              selected: false,
              removedTakeId: takeId,
              retainTakeEntity: true,
              sharedTakeUse: true,
            );
            return _removalPublication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
              selectionCleared: false,
              takeEntityRemoved: false,
              remainingCandidateCount: 0,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-take-remove-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
    await tester.pumpAndSettle();

    expect(find.textContaining('other project uses'), findsOneWidget);
    expect(find.textContaining('audio data remains retained'), findsOneWidget);
    expect(find.textContaining('free'), findsNothing);
  });

  testWidgets(
    'removal busy state locks every action and cannot double publish',
    (tester) async {
      var current = _index(candidateCount: 1, selected: false);
      final takeId = _candidateIds(current).single;
      final completer = Completer<Revision3VoiceTakeRemovalPublication>();
      Revision3VoiceTakeRemovalTechnicalPlan? received;
      var publishCalls = 0;
      await _openDialog(
        tester,
        index: current,
        load: () async => current,
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) {
              publishCalls++;
              received = plan;
              return completer.future;
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-take-remove-0')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
      await tester.pump();

      expect(publishCalls, 1);
      expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
      expect(
        tester
            .widget<TextButton>(find.byKey(const Key('voice-take-remove-0')))
            .onPressed,
        isNull,
      );
      expect(find.byType(CircularProgressIndicator), findsWidgets);

      current = _removedIndex(
        candidateCount: 1,
        selected: false,
        removedTakeId: takeId,
        retainTakeEntity: false,
      );
      completer.complete(
        _removalPublication(
          projectId: revision3VoiceContentProjectId,
          revision: 8,
          plan: received!,
          selectionCleared: false,
          takeEntityRemoved: true,
          remainingCandidateCount: 0,
        ),
      );
      await tester.pumpAndSettle();
      expect(publishCalls, 1);
    },
  );

  testWidgets('reload recovery never repeats an already saved removal', (
    tester,
  ) async {
    final initial = _index(candidateCount: 1, selected: false);
    final takeId = _candidateIds(initial).single;
    final refreshed = _removedIndex(
      candidateCount: 1,
      selected: false,
      removedTakeId: takeId,
      retainTakeEntity: false,
    );
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async {
        reads++;
        if (reads == 3) throw StateError('temporary reload failure');
        return reads >= 4 ? refreshed : initial;
      },
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _removalPublication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
              selectionCleared: false,
              takeEntityRemoved: true,
              remainingCandidateCount: 0,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-take-remove-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.textContaining('removal was saved'), findsOneWidget);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(
      tester
          .widget<TextButton>(find.byKey(const Key('voice-take-remove-0')))
          .onPressed,
      isNull,
    );

    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.textContaining('saved removal was confirmed'), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-line-0')), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-locale')), findsOneWidget);
    expect(
      tester
          .widget<DropdownButtonFormField<String>>(
            find.byKey(const Key('voice-selection-locale')),
          )
          .initialValue,
      'de',
    );
    expect(
      find.byKey(const Key('voice-selection-no-candidates')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('voice-take-remove-0')), findsNothing);
  });

  testWidgets('fixed-context recovery closes when its exact slot disappeared', (
    tester,
  ) async {
    final initial = _index(candidateCount: 1, selected: false);
    final withoutSlot = _indexWithoutSlot(revision: 8, lineRevision: 3);
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async => reads++ == 0 ? initial : withoutSlot,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            throw StateError('stale context must reject before publish');
          },
    );

    await tester.tap(find.byKey(const Key('voice-take-remove-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
    await tester.pumpAndSettle();

    expect(publishCalls, 0);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);

    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 0);
    expect(find.textContaining('no longer matches'), findsOneWidget);
    expect(find.textContaining('Latest Voice takes reloaded'), findsNothing);
    expect(find.textContaining('saved removal was confirmed'), findsNothing);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(
      find.byKey(const Key('voice-selection-fixed-context')),
      findsNothing,
    );
    expect(find.byKey(const Key('voice-selection-line-search')), findsNothing);
    expect(find.byKey(const Key('voice-selection-locale')), findsNothing);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
  });

  testWidgets('requires-reopen removal is terminal and is never retried', (
    tester,
  ) async {
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: _index(candidateCount: 1, selected: false),
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            throw const Revision3VoiceTakeRemovalRequiresReopenException();
          },
    );
    final line = find.byKey(const Key('voice-selection-line-0'));
    await tester.ensureVisible(line);
    await tester.pump();
    await tester.tap(line);
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-take-remove-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.textContaining('Do not retry'), findsOneWidget);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
    expect(
      tester
          .widget<TextButton>(find.byKey(const Key('voice-take-remove-0')))
          .onPressed,
      isNull,
    );
  });

  testWidgets('stale removal offers reload and never invokes native publish', (
    tester,
  ) async {
    final initial = _index(candidateCount: 1, selected: false);
    final stale = _index(
      revision: initial.projectRevision + 1,
      candidateCount: 1,
      selected: false,
    );
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async => reads++ == 0 ? initial : stale,
      publishRemoval:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            throw StateError('stale service must reject before publish');
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-take-remove-0')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
    await tester.pumpAndSettle();

    expect(publishCalls, 0);
    expect(find.textContaining('review the action again'), findsOneWidget);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
  });

  testWidgets('remove action and confirmation fit a compact dialog', (
    tester,
  ) async {
    await _openDialog(
      tester,
      index: _index(candidateCount: 1, selected: false),
      surfaceSize: const Size(390, 680),
    );
    final line = find.byKey(const Key('voice-selection-line-0'));
    await tester.ensureVisible(line);
    await tester.pump();
    await tester.tap(line);
    await tester.pump();
    final remove = find.byKey(const Key('voice-take-remove-0'));
    await tester.ensureVisible(remove);
    await tester.tap(remove);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('voice-take-remove-confirm-dialog')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('voice-take-remove-confirm')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3ContentIndex index,
  Future<Revision3ContentIndex> Function()? load,
  Revision3VoiceTakeSelectionTechnicalPublisher? publish,
  Revision3VoiceTakeStatusTechnicalPublisher? publishStatus,
  Revision3VoiceTakeRemovalTechnicalPublisher? publishRemoval,
  Revision3DialogVoiceSlotRemovalTechnicalPublisher? publishSlotRemoval,
  Locale locale = const Locale('en'),
  Size surfaceSize = const Size(1200, 1000),
  String? initialLineId,
  String? initialLocale,
  bool fixedContext = false,
}) async {
  await tester.binding.setSurfaceSize(surfaceSize);
  addTearDown(() => tester.binding.setSurfaceSize(null));
  final service = Revision3VoiceTakeSelectionAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publish ?? _unexpectedPublish,
  );
  final statusService = Revision3VoiceTakeStatusAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publishStatus ?? _unexpectedStatusPublish,
  );
  final removalService = Revision3VoiceTakeRemovalAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publishRemoval ?? _unexpectedRemovalPublish,
  );
  final slotRemovalService = Revision3DialogVoiceSlotRemovalAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publishSlotRemoval ?? _unexpectedSlotRemovalPublish,
  );
  await tester.pumpWidget(
    MaterialApp(
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Builder(
        builder: (context) => FilledButton(
          key: const Key('open-dialog'),
          onPressed: () => showDialog<Revision3VoiceTakeSelectionPublication>(
            context: context,
            builder: (_) => Revision3VoiceTakeSelectionDialog(
              service: service,
              statusService: statusService,
              removalService: removalService,
              slotRemovalService: slotRemovalService,
              initialLineId: initialLineId,
              initialLocale: initialLocale,
              fixedContext: fixedContext,
            ),
          ),
          child: const Text('Open'),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dialog')));
  await tester.pumpAndSettle();
}

Revision3ContentIndex _index({
  int revision = 7,
  bool secondApproved = false,
  int secondTakeRevision = 0,
  String selectedStatus = 'approved',
  int candidateCount = 2,
  bool selected = true,
  bool sharedFirstTake = false,
  bool generatedSlot = false,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotGenerated: generatedSlot,
    existingSlotCandidateCount: candidateCount,
    existingSlotHasSelectedTake: selected,
  );
  final takes = (json['entities']! as List)
      .cast<Map<String, Object?>>()
      .where((entity) => entity['kind'] == 'voice_take')
      .toList(growable: false);
  if (takes.isNotEmpty) {
    final selectedSummary = (takes[0]['summary']! as Map)
        .cast<String, Object?>();
    final selectedData = (selectedSummary['data']! as Map)
        .cast<String, Object?>();
    selectedData['status'] = selectedStatus;
    selectedSummary['data'] = selectedData;
    takes[0]['summary'] = selectedSummary;
  }
  if (secondApproved && takes.length > 1) {
    final summary = (takes[1]['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['status'] = 'approved';
    summary['data'] = data;
    takes[1]['summary'] = summary;
  }
  if (takes.length > 1) takes[1]['revision'] = secondTakeRevision;
  if (sharedFirstTake) {
    _addSharedVoiceCandidateUse(json, takeId: takes.first['id']! as String);
  }
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _indexWithoutSlot({
  required int revision,
  required int lineRevision,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: false,
  );
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLineId,
  )['revision'] = lineRevision;
  return Revision3ContentIndex.fromJsonObject(json);
}

List<String> _candidateIds(Revision3ContentIndex index) {
  final catalog = Revision3VoiceCatalog.fromContentIndex(index);
  final line = catalog.line(revision3VoiceContentLineId)!;
  return line
      .slotSummaryForLocale('de')!
      .candidates
      .map((take) => take.id)
      .toList(growable: false);
}

Revision3ContentIndex _removedIndex({
  required int candidateCount,
  required bool selected,
  required String removedTakeId,
  required bool retainTakeEntity,
  bool sharedTakeUse = false,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: 8,
    existingSlotCandidateCount: candidateCount,
    existingSlotHasSelectedTake: selected,
  );
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  final slot = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentSlotId,
  );
  slot['revision'] = 2;
  final references = (slot['references']! as List).cast<Map<String, Object?>>();
  references.removeWhere((reference) {
    final target = (reference['target']! as Map).cast<String, Object?>();
    return target['entity_id'] == removedTakeId;
  });
  final remainingCandidateCount = references
      .where((reference) => reference['role'] == 'voice_candidate')
      .length;
  final hasSelected = references.any(
    (reference) => reference['role'] == 'voice_selected',
  );
  final summary = (slot['summary']! as Map).cast<String, Object?>();
  final summaryData = (summary['data']! as Map).cast<String, Object?>();
  summaryData['candidate_count'] = remainingCandidateCount;
  summaryData['has_selected_take'] = hasSelected;
  summary['data'] = summaryData;
  slot['summary'] = summary;
  if (!retainTakeEntity) {
    entities.removeWhere((entity) => entity['id'] == removedTakeId);
  }
  if (sharedTakeUse) {
    _addSharedVoiceCandidateUse(json, takeId: removedTakeId);
  }
  final counts = (json['entity_counts']! as Map).cast<String, Object?>();
  final takeCount = entities
      .where((entity) => entity['kind'] == 'voice_take')
      .length;
  if (takeCount == 0) {
    counts.remove('voice_take');
  } else {
    counts['voice_take'] = takeCount;
  }
  json['entity_counts'] = counts;
  json['entities'] = entities;
  return Revision3ContentIndex.fromJsonObject(json);
}

const _sharedVoiceLocalizationId = '77777777777777777777777777777777';
const _sharedVoiceLineId = '88888888888888888888888888888888';
const _sharedVoiceSlotId = '99999999999999999999999999999999';

void _addSharedVoiceCandidateUse(
  Map<String, Object?> index, {
  required String takeId,
}) {
  final entities = (index['entities']! as List).cast<Map<String, Object?>>();
  entities.addAll(<Map<String, Object?>>[
    <String, Object?>{
      'id': _sharedVoiceLocalizationId,
      'kind': 'localization_entry',
      'display_name': 'SHARED_VOICE_LINE',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'shared-voice-localization',
      },
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': 'SHARED_VOICE_LINE',
          'locales': <Object?>[],
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _sharedVoiceLineId,
      'kind': 'dialog_line',
      'display_name': 'Second shared take owner',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'shared-voice-line',
      },
      'summary': <String, Object?>{
        'kind': 'dialog_line',
        'data': <String, Object?>{
          'speaker_hint': 'Viper',
          'voice_slot_locales': <Object?>['de'],
        },
      },
      'references': <Object?>[
        _sharedVoiceReference(
          role: 'dialog_localization',
          entityId: _sharedVoiceLocalizationId,
          expectedKind: 'localization_entry',
        ),
        _sharedVoiceReference(
          role: 'dialog_voice_slot',
          qualifier: 'de',
          entityId: _sharedVoiceSlotId,
          expectedKind: 'voice_slot',
        ),
      ],
      'asset_references': <Object?>[],
    },
    <String, Object?>{
      'id': _sharedVoiceSlotId,
      'kind': 'voice_slot',
      'display_name': 'Second German Voice slot',
      'revision': 0,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'shared-voice-slot',
      },
      'summary': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': 'de',
          'target_resolution': 'unresolved',
          'candidate_count': 1,
          'has_selected_take': false,
        },
      },
      'references': <Object?>[
        _sharedVoiceReference(
          role: 'voice_candidate',
          entityId: takeId,
          expectedKind: 'voice_take',
        ),
      ],
      'asset_references': <Object?>[],
    },
  ]);
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  final counts = (index['entity_counts']! as Map).cast<String, Object?>();
  counts['localization_entry'] = (counts['localization_entry']! as int) + 1;
  counts['dialog_line'] = (counts['dialog_line']! as int) + 1;
  counts['voice_slot'] = (counts['voice_slot']! as int) + 1;
}

Map<String, Object?> _sharedVoiceReference({
  required String role,
  String? qualifier,
  required String entityId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': entityId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};

Revision3VoiceTakeSelectionPublication _publication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => Revision3VoiceTakeSelectionPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision + 1,
  locale: plan.locale,
  locId: plan.locId,
  previousSelectedTakeId: plan.expectedSelectedTakeId,
  selectedTakeId: plan.selectedTakeId,
);

Revision3VoiceTakeStatusPublication _statusPublication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => Revision3VoiceTakeStatusPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision,
  locale: plan.locale,
  locId: plan.locId,
  takeId: plan.takeId,
  takeRevision: plan.expectedTakeRevision + 1,
  previousStatus: plan.expectedStatus,
  status: plan.desiredStatus,
);

Revision3VoiceTakeRemovalPublication _removalPublication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeRemovalTechnicalPlan plan,
  required bool selectionCleared,
  required bool takeEntityRemoved,
  required int remainingCandidateCount,
}) => Revision3VoiceTakeRemovalPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision + 1,
  locale: plan.locale,
  locId: plan.locId,
  takeId: plan.takeId,
  takeRevision: plan.expectedTakeRevision,
  previousSelectedTakeId: plan.expectedSelectedTakeId,
  selectionCleared: selectionCleared,
  takeEntityRemoved: takeEntityRemoved,
  remainingCandidateCount: remainingCandidateCount,
);

Revision3DialogVoiceSlotRemovalPublication _slotRemovalPublication({
  required String projectId,
  required int revision,
  required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
}) => Revision3DialogVoiceSlotRemovalPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  lineRevision: plan.expectedLineRevision + 1,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  removedSlotRevision: plan.expectedSlotRevision,
  locale: plan.locale,
  locId: plan.locId,
  removedTargetResolution: plan.targetResolution,
);

Future<Revision3VoiceTakeSelectionPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => throw StateError('publisher was not expected');

Future<Revision3VoiceTakeStatusPublication> _unexpectedStatusPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => throw StateError('status publisher was not expected');

Future<Revision3VoiceTakeRemovalPublication> _unexpectedRemovalPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeRemovalTechnicalPlan plan,
}) => throw StateError('removal publisher was not expected');

Future<Revision3DialogVoiceSlotRemovalPublication>
_unexpectedSlotRemovalPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
}) => throw StateError('slot removal publisher was not expected');

ButtonStyleButton _button(WidgetTester tester, String key) =>
    tester.widget<ButtonStyleButton>(find.byKey(Key(key)));
