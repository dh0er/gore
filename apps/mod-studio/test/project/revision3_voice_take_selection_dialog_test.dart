import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_playback.dart';
import 'package:gore_mod/project/revision3_voice_take_media_qa_service.dart';
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

  testWidgets(
    'inline preview uses injected playback and preserves a pending selection',
    (tester) async {
      final events = <String>[];
      final player = _DialogFakePreviewPlayer(events);
      final playback = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _DialogFakeLease('C:\\hidden\\preview.ogg', events);
      var materializations = 0;
      await _openDialog(
        tester,
        index: _index(secondApproved: true, previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        previewPlayback: playback,
        previewMaterialize:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async {
              materializations++;
              return lease.value;
            },
      );

      await tester.tap(find.byKey(const Key('voice-selection-take-1')));
      await tester.pump();
      expect(
        find.byKey(const Key('voice-status-selection-pending')),
        findsOneWidget,
      );

      final start = find.byKey(const Key('voice-preview-start-0'));
      await tester.ensureVisible(start);
      await tester.tap(start);
      await tester.pump();
      await tester.pump();

      expect(materializations, 1);
      expect(events, contains('player:open:C:\\hidden\\preview.ogg'));
      expect(find.byKey(const Key('voice-preview-active-0')), findsOneWidget);
      expect(
        find.byKey(const Key('voice-status-selection-pending')),
        findsOneWidget,
      );
      expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);
      expect(find.textContaining('C:\\hidden'), findsNothing);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);

      await tester.tap(find.byKey(const Key('voice-preview-toggle-0')));
      await tester.pump();
      expect(events, contains('player:pause'));
      expect(
        playback.snapshot.phase,
        Revision3VoiceTakePreviewPlaybackPhase.paused,
      );

      final slider = tester.widget<Slider>(
        find.byKey(const Key('voice-preview-progress-0')),
      );
      slider.onChanged!(5000);
      await tester.pump();
      expect(events, contains('player:seek:5000'));

      unawaited(playback.dispose());
    },
  );

  testWidgets('preview controls remain usable at a compact dialog width', (
    tester,
  ) async {
    final events = <String>[];
    final player = _DialogFakePreviewPlayer(events);
    final playback = Revision3VoiceTakePreviewPlaybackController(
      player: player,
    );
    final lease = _DialogFakeLease('compact.ogg', events);
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      surfaceSize: const Size(620, 720),
      previewPlayback: playback,
      previewMaterialize:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async => lease.value,
    );

    final start = find.byKey(const Key('voice-preview-start-0'));
    await tester.ensureVisible(start);
    await tester.tap(start);
    await tester.pump();
    await tester.pump();

    expect(find.byKey(const Key('voice-preview-progress-0')), findsOneWidget);
    expect(tester.takeException(), isNull);
    unawaited(playback.dispose());
  });

  testWidgets(
    'media QA is on demand, per take, and keeps its exact safe result separate from preview',
    (tester) async {
      final pending = Completer<Revision3VoiceTakeMediaQaDialogResult>();
      var inspections = 0;
      final index = _index(previewable: true);
      final privateTakeId = _candidateIds(index).first;
      await _openDialog(
        tester,
        index: index,
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        mediaQaInspect:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) {
              inspections++;
              return pending.future;
            },
      );

      expect(inspections, 0);
      expect(find.byKey(const Key('voice-media-qa-result-0')), findsNothing);
      expect(find.byKey(const Key('voice-preview-start-0')), findsNothing);

      final check = find.byKey(const Key('voice-media-qa-start-0'));
      await tester.ensureVisible(check);
      await tester.tap(check);
      await tester.pump();

      expect(inspections, 1);
      expect(find.byKey(const Key('voice-media-qa-loading-0')), findsOneWidget);
      expect(find.byKey(const Key('voice-media-qa-loading-1')), findsNothing);

      pending.complete(
        Revision3VoiceTakeMediaQaDialogResult(
          sampleFrames: 72000,
          timebaseHz: 48000,
          assurance: Revision3VoiceTakeMediaQaDialogAssurance.fullyDecoded,
        ),
      );
      await tester.pump();
      await tester.pump();

      expect(find.textContaining('1.50 s'), findsOneWidget);
      expect(find.textContaining('fully decoded'), findsOneWidget);
      expect(find.textContaining('not audio quality'), findsOneWidget);
      expect(find.textContaining('in-game playback'), findsOneWidget);
      expect(find.textContaining(privateTakeId), findsNothing);
      expect(find.textContaining(r'C:\'), findsNothing);
    },
  );

  testWidgets(
    'Opus media QA states its limited assurance without overclaiming',
    (tester) async {
      await _openDialog(
        tester,
        index: _index(previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        mediaQaInspect:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async => Revision3VoiceTakeMediaQaDialogResult(
              sampleFrames: 123456,
              timebaseHz: 48000,
              assurance: Revision3VoiceTakeMediaQaDialogAssurance
                  .timingAndStructureOnly,
            ),
      );

      final check = find.byKey(const Key('voice-media-qa-start-0'));
      await tester.ensureVisible(check);
      await tester.tap(check);
      await tester.pumpAndSettle();

      expect(
        find.textContaining(
          'timing/structure checked; audio not fully decoded',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('not audio quality'), findsOneWidget);
    },
  );

  testWidgets('German media QA copy stays author-friendly', (tester) async {
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      copy: Revision3VoiceTakeSelectionDialogCopy.german,
      mediaQaInspect:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async => Revision3VoiceTakeMediaQaDialogResult(
            sampleFrames: 48000,
            timebaseHz: 48000,
            assurance: Revision3VoiceTakeMediaQaDialogAssurance.fullyDecoded,
          ),
    );

    final check = find.byKey(const Key('voice-media-qa-start-0'));
    expect(find.widgetWithText(TextButton, 'Audiodatei prüfen'), findsWidgets);
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();

    expect(find.textContaining('Audiodatei geprüft'), findsOneWidget);
    expect(find.textContaining('vollständig dekodiert'), findsOneWidget);
    expect(find.textContaining('nicht Audioqualität'), findsOneWidget);
    expect(find.textContaining('Wiedergabe im Spiel'), findsOneWidget);
  });

  testWidgets(
    'unknown media QA failure stays per take, hides details, and retries',
    (tester) async {
      var inspections = 0;
      await _openDialog(
        tester,
        index: _index(previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        mediaQaInspect:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async {
              inspections++;
              if (inspections == 1) {
                throw StateError(
                  r'C:\private\take.ogg aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                );
              }
              return Revision3VoiceTakeMediaQaDialogResult(
                sampleFrames: 48000,
                timebaseHz: 48000,
                assurance:
                    Revision3VoiceTakeMediaQaDialogAssurance.fullyDecoded,
              );
            },
      );

      final check = find.byKey(const Key('voice-media-qa-start-0'));
      await tester.ensureVisible(check);
      await tester.tap(check);
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('voice-media-qa-error-0')), findsOneWidget);
      expect(find.byKey(const Key('voice-media-qa-error-1')), findsNothing);
      expect(find.textContaining('private'), findsNothing);
      expect(find.textContaining('aaaaaaaaaaaaaaaa'), findsNothing);
      expect(find.widgetWithText(TextButton, 'Retry'), findsOneWidget);

      await tester.tap(check);
      await tester.pumpAndSettle();
      expect(inspections, 2);
      expect(find.byKey(const Key('voice-media-qa-error-0')), findsNothing);
      expect(find.byKey(const Key('voice-media-qa-result-0')), findsOneWidget);
    },
  );

  testWidgets('stale media QA offers a catalog reload and discards its state', (
    tester,
  ) async {
    var inspections = 0;
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      mediaQaInspect:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async {
            inspections++;
            throw const Revision3VoiceTakeMediaQaStaleCheckpointException();
          },
    );

    final check = find.byKey(const Key('voice-media-qa-start-0'));
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();

    expect(inspections, 1);
    expect(find.byKey(const Key('voice-media-qa-error-0')), findsOneWidget);
    expect(find.textContaining('project changed'), findsWidgets);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
    expect(tester.widget<TextButton>(check).onPressed, isNull);

    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('voice-media-qa-error-0')), findsNothing);
    expect(find.byKey(const Key('voice-media-qa-result-0')), findsNothing);
    expect(inspections, 1);
  });

  testWidgets('requires-reopen media QA is per-take and terminal', (
    tester,
  ) async {
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      mediaQaInspect:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async =>
              throw const Revision3VoiceTakeMediaQaRequiresReopenException(),
    );

    final check = find.byKey(const Key('voice-media-qa-start-0'));
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('voice-media-qa-error-0')), findsOneWidget);
    expect(find.textContaining('Reopen the managed project'), findsWidgets);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
    expect(tester.widget<TextButton>(check).onPressed, isNull);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
  });

  testWidgets('media QA result is discarded on line change and mutation', (
    tester,
  ) async {
    var current = _index(previewable: true, sharedFirstTake: true);
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      mediaQaInspect:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async => Revision3VoiceTakeMediaQaDialogResult(
            sampleFrames: 48000,
            timebaseHz: 48000,
            assurance: Revision3VoiceTakeMediaQaDialogAssurance.fullyDecoded,
          ),
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            current = _index(
              revision: 8,
              secondApproved: true,
              secondTakeRevision: 1,
              previewable: true,
              sharedFirstTake: true,
            );
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
    );

    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    final check = find.byKey(const Key('voice-media-qa-start-0'));
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('voice-media-qa-result-0')), findsOneWidget);

    final secondLine = find.byKey(const Key('voice-selection-line-1'));
    await tester.ensureVisible(secondLine);
    await tester.tap(secondLine);
    await tester.pump();
    expect(find.byKey(const Key('voice-media-qa-result-0')), findsNothing);

    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('voice-media-qa-result-0')), findsOneWidget);

    final statusChange = find.byKey(const Key('voice-status-change-1'));
    await tester.ensureVisible(statusChange);
    await tester.pump();
    await tester.tap(statusChange);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('voice-media-qa-result-0')), findsNothing);
  });

  testWidgets('media QA result remains usable at a narrow dialog width', (
    tester,
  ) async {
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      surfaceSize: const Size(390, 680),
      mediaQaInspect:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async => Revision3VoiceTakeMediaQaDialogResult(
            sampleFrames: 48000,
            timebaseHz: 48000,
            assurance: Revision3VoiceTakeMediaQaDialogAssurance.fullyDecoded,
          ),
    );

    final check = find.byKey(const Key('voice-media-qa-start-0'));
    await tester.ensureVisible(check);
    await tester.tap(check);
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.byKey(const Key('voice-media-qa-result-0')),
    );
    await tester.pump();

    expect(find.textContaining('not audio quality'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('barrier dismissal unloads playback before closing its lease', (
    tester,
  ) async {
    final events = <String>[];
    final player = _DialogFakePreviewPlayer(events);
    final playback = Revision3VoiceTakePreviewPlaybackController(
      player: player,
    );
    final lease = _DialogFakeLease('barrier.ogg', events);
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      previewPlayback: playback,
      previewMaterialize:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async => lease.value,
    );
    final start = find.byKey(const Key('voice-preview-start-0'));
    await tester.ensureVisible(start);
    await tester.tap(start);
    await tester.pump();
    await tester.pump();
    events.clear();

    await tester.tapAt(const Offset(2, 2));
    await tester.pumpAndSettle();
    await tester.pump();

    expect(
      events,
      containsAllInOrder(<String>['player:stop', 'lease:close:barrier.ogg']),
    );
    expect(lease.closed, isTrue);
    unawaited(playback.dispose());
  });

  testWidgets('stale preview offers catalog reload instead of blind retry', (
    tester,
  ) async {
    final playback = Revision3VoiceTakePreviewPlaybackController(
      player: _DialogFakePreviewPlayer(<String>[]),
    );
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      previewPlayback: playback,
      previewMaterialize:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async =>
              throw const Revision3VoiceTakePreviewStaleCheckpointException(),
    );
    final start = find.byKey(const Key('voice-preview-start-0'));
    await tester.ensureVisible(start);
    await tester.tap(start);
    await tester.pump();
    await tester.pump();

    expect(find.textContaining('project changed'), findsWidgets);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
    expect(find.byKey(const Key('voice-preview-retry-0')), findsNothing);
    unawaited(playback.dispose());
  });

  testWidgets('requires-reopen preview failure is terminal and not retryable', (
    tester,
  ) async {
    final playback = Revision3VoiceTakePreviewPlaybackController(
      player: _DialogFakePreviewPlayer(<String>[]),
    );
    await _openDialog(
      tester,
      index: _index(previewable: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      previewPlayback: playback,
      previewMaterialize:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async =>
              throw const Revision3VoiceTakePreviewRequiresReopenException(),
    );
    final start = find.byKey(const Key('voice-preview-start-0'));
    await tester.ensureVisible(start);
    await tester.tap(start);
    await tester.pump();
    await tester.pump();

    expect(find.textContaining('Reopen the managed project'), findsWidgets);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.byKey(const Key('voice-preview-retry-0')), findsNothing);
    expect(
      find.descendant(
        of: find.byKey(const Key('voice-selection-cancel')),
        matching: find.text('Close'),
      ),
      findsOneWidget,
    );
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    unawaited(playback.dispose());
  });

  testWidgets(
    'cleanup retry restores terminal requires-reopen state and locks actions',
    (tester) async {
      final cleanup = _DialogFakeCleanupObligation(failuresBeforeClean: 1);
      final playback = Revision3VoiceTakePreviewPlaybackController(
        player: _DialogFakePreviewPlayer(<String>[]),
      );
      await _openDialog(
        tester,
        index: _index(secondApproved: true, previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        previewPlayback: playback,
        previewMaterialize:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async => throw Revision3VoiceTakePreviewRequiresReopenException(
              cause: StateError('fake receipt mismatch'),
              cleanupObligation: cleanup,
            ),
      );
      await tester.tap(find.byKey(const Key('voice-selection-take-1')));
      await tester.pump();
      expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);

      final start = find.byKey(const Key('voice-preview-start-0'));
      await tester.ensureVisible(start);
      await tester.tap(start);
      await tester.pump();
      await tester.pump();

      expect(cleanup.attempts, 1);
      expect(find.textContaining('could not be closed safely'), findsOneWidget);
      expect(find.byKey(const Key('voice-preview-retry-0')), findsOneWidget);
      expect(
        tester
            .widget<TextButton>(find.byKey(const Key('voice-preview-retry-0')))
            .onPressed,
        isNull,
      );
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-cancel')),
          matching: find.text('Close'),
        ),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('voice-preview-stop-0')));
      await tester.pump();
      await tester.pump();

      expect(cleanup.attempts, 2);
      expect(cleanup.isCleaned, isTrue);
      expect(find.textContaining('Reopen the managed project'), findsWidgets);
      expect(find.byKey(const Key('voice-preview-retry-0')), findsNothing);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      expect(
        tester
            .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
              find.byKey(const Key('voice-status-change-0')),
            )
            .enabled,
        isFalse,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-cancel')),
          matching: find.text('Close'),
        ),
        findsOneWidget,
      );
      unawaited(playback.dispose());
    },
  );

  testWidgets(
    'status mutation locks the route before preview Stop and aborts on cleanup',
    (tester) async {
      final events = <String>[];
      final player = _DialogFakePreviewPlayer(events);
      final playback = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _DialogFakeLease('status-stop.ogg', events);
      var statusPublishes = 0;
      await _openDialog(
        tester,
        index: _index(previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        previewPlayback: playback,
        previewMaterialize:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async => lease.value,
        publishStatus:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              statusPublishes++;
              throw StateError('status publish must not follow failed Stop');
            },
      );
      final start = find.byKey(const Key('voice-preview-start-0'));
      await tester.ensureVisible(start);
      await tester.tap(start);
      await tester.pump();
      await tester.pump();

      final stopGate = Completer<void>();
      player
        ..stopGate = stopGate
        ..failuresBeforeStop = player.stopAttempts + 1;
      await tester.tap(find.byKey(const Key('voice-status-change-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
      await tester.pump(const Duration(milliseconds: 300));

      expect(player.stopGate, isNull);
      expect(statusPublishes, 0);
      expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
      await tester.tapAt(const Offset(2, 2));
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );

      stopGate.complete();
      await tester.pumpAndSettle();

      expect(statusPublishes, 0);
      expect(find.textContaining('could not be closed safely'), findsWidgets);
      expect(
        tester
            .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
              find.byKey(const Key('voice-status-change-1')),
            )
            .enabled,
        isFalse,
      );
      expect(_button(tester, 'voice-take-remove-1').onPressed, isNull);
      expect(_button(tester, 'voice-selection-cancel').onPressed, isNotNull);
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-cancel')),
          matching: find.text('Close'),
        ),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key('voice-preview-stop-0')));
      await tester.pumpAndSettle();

      expect(lease.closed, isTrue);
      expect(find.textContaining('could not be closed safely'), findsNothing);
      expect(
        tester
            .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
              find.byKey(const Key('voice-status-change-1')),
            )
            .enabled,
        isTrue,
      );
      expect(_button(tester, 'voice-take-remove-1').onPressed, isNotNull);
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-cancel')),
          matching: find.text('Cancel'),
        ),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('voice-selection-cancel')));
      await tester.pumpAndSettle();
      await playback.dispose();
    },
  );

  testWidgets(
    'take removal locks the route before preview Stop and never publishes on cleanup',
    (tester) async {
      final events = <String>[];
      final player = _DialogFakePreviewPlayer(events);
      final playback = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _DialogFakeLease('removal-stop.ogg', events);
      var removalPublishes = 0;
      await _openDialog(
        tester,
        index: _index(previewable: true),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        previewPlayback: playback,
        previewMaterialize:
            ({
              required checkpoint,
              required lineId,
              required locale,
              required takeId,
            }) async => lease.value,
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              removalPublishes++;
              throw StateError('removal publish must not follow failed Stop');
            },
      );
      final start = find.byKey(const Key('voice-preview-start-0'));
      await tester.ensureVisible(start);
      await tester.tap(start);
      await tester.pump();
      await tester.pump();

      final stopGate = Completer<void>();
      player
        ..stopGate = stopGate
        ..failuresBeforeStop = player.stopAttempts + 1;
      await tester.tap(find.byKey(const Key('voice-take-remove-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('voice-take-remove-confirm')));
      await tester.pump(const Duration(milliseconds: 300));

      expect(player.stopGate, isNull);
      expect(removalPublishes, 0);
      expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
      await tester.tapAt(const Offset(2, 2));
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );

      stopGate.complete();
      await tester.pumpAndSettle();

      expect(removalPublishes, 0);
      expect(find.textContaining('could not be closed safely'), findsWidgets);
      expect(_button(tester, 'voice-take-remove-1').onPressed, isNull);
      expect(_button(tester, 'voice-selection-cancel').onPressed, isNotNull);
      await tester.tap(find.byKey(const Key('voice-preview-stop-0')));
      await tester.pumpAndSettle();
      expect(lease.closed, isTrue);
      expect(_button(tester, 'voice-take-remove-1').onPressed, isNotNull);
      await tester.tap(find.byKey(const Key('voice-selection-cancel')));
      await tester.pumpAndSettle();
      await playback.dispose();
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
    'fixed take handoff focuses a nonselected take without selecting it',
    (tester) async {
      final index = _index(secondApproved: true);
      final takeIds = _candidateIds(index);
      final targetTake = Revision3VoiceCatalog.fromContentIndex(index)
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!
          .candidates[1];
      await _openDialog(
        tester,
        index: index,
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        initialTakeId: takeIds[1],
        fixedContext: true,
      );

      expect(
        tester
            .widget<RadioGroup<String>>(find.byType(RadioGroup<String>))
            .groupValue,
        takeIds.first,
      );
      expect(
        find.byKey(const ValueKey('voice-selection-take-navigation-target-1')),
        findsOneWidget,
      );
      final navigationTarget = find.byKey(
        const ValueKey('voice-selection-take-navigation-target-1'),
      );
      final targetElement = tester.element(navigationTarget);
      final navigationFocus = targetElement
          .findAncestorWidgetOfExactType<Focus>()
          ?.focusNode;
      expect(navigationFocus?.hasFocus, isTrue);
      expect(navigationFocus?.skipTraversal, isTrue);
      final semantics = tester.getSemantics(navigationTarget);
      expect(semantics.label, contains('Opened Voice take'));
      expect(semantics.label, contains(targetTake.displayLabel));
      expect(semantics.label, isNot(contains(takeIds[1])));
      expect(
        find.byKey(const Key('voice-status-selection-pending')),
        findsNothing,
      );
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      expect(find.textContaining(takeIds[1]), findsNothing);

      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();

      expect(
        tester
            .widget<RadioGroup<String>>(find.byType(RadioGroup<String>))
            .groupValue,
        takeIds[1],
      );
      expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);
    },
  );

  testWidgets('fixed take handoff focuses the already selected take', (
    tester,
  ) async {
    final index = _index();
    final selectedTakeId = _candidateIds(index).first;
    await _openDialog(
      tester,
      index: index,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      initialTakeId: selectedTakeId,
      fixedContext: true,
    );

    expect(
      tester
          .widget<RadioGroup<String>>(find.byType(RadioGroup<String>))
          .groupValue,
      selectedTakeId,
    );
    final targetElement = tester.element(
      find.byKey(const ValueKey('voice-selection-take-navigation-target-0')),
    );
    expect(
      targetElement.findAncestorWidgetOfExactType<Focus>()?.focusNode?.hasFocus,
      isTrue,
    );
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(find.textContaining(selectedTakeId), findsNothing);
  });

  testWidgets('missing fixed take fails closed without choosing a neighbor', (
    tester,
  ) async {
    const missingTakeId = 'ffffffffffffffffffffffffffffffff';
    await _openDialog(
      tester,
      index: _index(secondApproved: true),
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      initialTakeId: missingTakeId,
      fixedContext: true,
    );

    expect(find.textContaining('no longer matches'), findsOneWidget);
    expect(find.byType(RadioGroup<String>), findsNothing);
    expect(
      find.byKey(const ValueKey('voice-selection-take-navigation-target-0')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('voice-selection-take-navigation-target-1')),
      findsNothing,
    );
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(find.textContaining(missingTakeId), findsNothing);
  });

  testWidgets('reload rejects a focused take reassigned to another slot', (
    tester,
  ) async {
    final initial = _index(previewable: true);
    final focusedTakeId = _candidateIds(initial).first;
    var current = initial;
    final playback = Revision3VoiceTakePreviewPlaybackController(
      player: _DialogFakePreviewPlayer(<String>[]),
    );
    await _openDialog(
      tester,
      index: initial,
      load: () async => current,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      initialTakeId: focusedTakeId,
      fixedContext: true,
      previewPlayback: playback,
      previewMaterialize:
          ({
            required checkpoint,
            required lineId,
            required locale,
            required takeId,
          }) async =>
              throw const Revision3VoiceTakePreviewStaleCheckpointException(),
    );

    await tester.tap(find.byKey(const Key('voice-preview-start-0')));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);

    current = _removedIndex(
      candidateCount: 2,
      selected: true,
      removedTakeId: focusedTakeId,
      retainTakeEntity: true,
      sharedTakeUse: true,
    );
    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(find.textContaining('no longer matches'), findsOneWidget);
    expect(find.byType(RadioGroup<String>), findsNothing);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(find.textContaining(focusedTakeId), findsNothing);
    await playback.dispose();
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
      final current = _index(secondApproved: true);
      final takeIds = _candidateIds(current);

      Widget app(
        Future<Revision3ContentIndex> Function() load, {
        required String initialTakeId,
      }) {
        return MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Revision3VoiceTakeSelectionDialog(
            key: const Key('selection-dialog-under-test'),
            copy: Revision3VoiceTakeSelectionDialogCopy.english,
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
            initialTakeId: initialTakeId,
            fixedContext: true,
          ),
        );
      }

      await tester.pumpWidget(
        app(() => olderLoad.future, initialTakeId: takeIds.first),
      );
      await tester.pump();

      await tester.pumpWidget(
        app(() async => current, initialTakeId: takeIds[1]),
      );
      await tester.pump();
      await tester.pump();

      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(find.textContaining('no longer matches'), findsNothing);
      expect(
        find.byKey(const ValueKey('voice-selection-take-navigation-target-1')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<RadioGroup<String>>(find.byType(RadioGroup<String>))
            .groupValue,
        takeIds.first,
      );

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
      expect(
        find.byKey(const ValueKey('voice-selection-take-navigation-target-1')),
        findsOneWidget,
      );
      expect(find.textContaining(takeIds[1]), findsNothing);
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
          copy: Revision3VoiceTakeSelectionDialogCopy.english,
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
    'unknown native load failure hides raw path, identity, code, and command',
    (tester) async {
      const privateId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
      const privatePath = r'C:\private-load-path\voice-take.ogg';

      await _openDialog(
        tester,
        index: _index(),
        load: () async => throw const ModFfiException(
          command: 'private_load_command',
          code: 'PRIVATE_UNKNOWN_CODE',
          message: '$privatePath $privateId',
        ),
      );

      expect(
        find.text(
          'Voice takes could not be loaded safely. Try again or reopen the managed project.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('private-load-path'), findsNothing);
      expect(find.textContaining(privateId), findsNothing);
      expect(find.textContaining('PRIVATE_UNKNOWN_CODE'), findsNothing);
      expect(find.textContaining('private_load_command'), findsNothing);
      expect(_button(tester, 'voice-selection-retry').onPressed, isNotNull);
    },
  );

  testWidgets(
    'German unknown format publication failure hides raw path and identity',
    (tester) async {
      const privateId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
      const privatePath = r'C:\private-format-path\selection.json';
      var publishes = 0;

      await _openDialog(
        tester,
        index: _index(),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        copy: Revision3VoiceTakeSelectionDialogCopy.german,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              throw const FormatException('$privatePath $privateId');
            },
      );

      await tester.tap(find.byKey(const Key('voice-selection-clear')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(publishes, 1);
      expect(
        find.text(
          'Die Voice-Auswahl konnte nicht sicher gespeichert werden. Es wurden keine Spiel- oder Spielstanddateien geändert.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining('private-format-path'), findsNothing);
      expect(find.textContaining(privateId), findsNothing);
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
          copy: Revision3VoiceTakeSelectionDialogCopy.english,
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

  testWidgets(
    'busy publication blocks dismissal and returns only its exact result',
    (tester) async {
      final completer = Completer<Revision3VoiceTakeSelectionPublication>();
      Revision3VoiceTakeSelectionTechnicalPlan? received;
      Revision3VoiceTakeSelectionPublication? dialogResult;
      await _openDialog(
        tester,
        index: _index(),
        onResult: (result) => dialogResult = result,
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

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );
      expect(dialogResult, isNull);

      await tester.tapAt(const Offset(4, 4));
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsOneWidget,
      );
      expect(dialogResult, isNull);

      final publication = _publication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: received!,
      );
      completer.complete(publication);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsNothing,
      );
      expect(dialogResult, same(publication));
    },
  );

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
    'German copy covers fixed context, selection, removal, and terminal status',
    (tester) async {
      var removalPublishes = 0;
      var statusPublishes = 0;
      await _openDialog(
        tester,
        index: _index(),
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
        copy: Revision3VoiceTakeSelectionDialogCopy.german,
        publishStatus:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              statusPublishes++;
              throw const Revision3VoiceTakeStatusRequiresReopenException();
            },
        publishRemoval:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              removalPublishes++;
              throw StateError('must not publish after cancel');
            },
      );

      expect(find.text('Voice-Takes verwalten'), findsOneWidget);
      expect(
        find.byKey(const Key('voice-selection-fixed-context')),
        findsOneWidget,
      );
      expect(find.text('Voice-Sprache: de'), findsOneWidget);
      expect(find.text('Ausgewählter Take'), findsOneWidget);
      expect(find.textContaining('Freigegeben'), findsWidgets);
      expect(find.textContaining('Aufgenommen'), findsWidgets);
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
      expect(find.text('Aus Zeile entfernen'), findsOneWidget);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);

      await tester.tap(find.byKey(const Key('voice-take-remove-cancel')));
      await tester.pumpAndSettle();
      expect(removalPublishes, 0);
      expect(
        find.byKey(const Key('voice-take-remove-confirm-dialog')),
        findsNothing,
      );

      await tester.tap(find.byKey(const Key('voice-status-change-1')));
      await tester.pumpAndSettle();
      expect(find.text('Freigegeben'), findsWidgets);
      await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
      await tester.pumpAndSettle();

      expect(statusPublishes, 1);
      expect(
        find.textContaining('Statusergebnis konnte nicht bestätigt werden'),
        findsOneWidget,
      );
      expect(find.widgetWithText(TextButton, 'Schließen'), findsOneWidget);
      expect(find.byKey(const Key('voice-status-reload')), findsNothing);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
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
  Revision3VoiceTakePreviewPlaybackController? previewPlayback,
  Revision3VoiceTakePreviewDialogMaterializer? previewMaterialize,
  Revision3VoiceTakeMediaQaDialogInspector? mediaQaInspect,
  Locale locale = const Locale('en'),
  Size surfaceSize = const Size(1200, 1000),
  String? initialLineId,
  String? initialLocale,
  String? initialTakeId,
  bool fixedContext = false,
  ValueChanged<Revision3VoiceTakeSelectionPublication?>? onResult,
  Revision3VoiceTakeSelectionDialogCopy copy =
      Revision3VoiceTakeSelectionDialogCopy.english,
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
          onPressed: () async {
            final result =
                await showDialog<Revision3VoiceTakeSelectionPublication>(
                  context: context,
                  builder: (_) => Revision3VoiceTakeSelectionDialog(
                    service: service,
                    statusService: statusService,
                    removalService: removalService,
                    slotRemovalService: slotRemovalService,
                    previewPlayback: previewPlayback,
                    previewMaterialize: previewMaterialize,
                    mediaQaInspect: mediaQaInspect,
                    initialLineId: initialLineId,
                    initialLocale: initialLocale,
                    initialTakeId: initialTakeId,
                    fixedContext: fixedContext,
                    copy: copy,
                  ),
                );
            onResult?.call(result);
          },
          child: const Text('Open'),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dialog')));
  await tester.pumpAndSettle();
}

final class _DialogFakeLease {
  _DialogFakeLease(this.path, this.events);

  final String path;
  final List<String> events;
  bool closed = false;

  late final Revision3VoiceTakePreviewPlaybackLease value =
      Revision3VoiceTakePreviewPlaybackLease(
        path: path,
        isClosed: () => closed,
        close: () async {
          events.add('lease:close:$path');
          closed = true;
        },
      );
}

final class _DialogFakeCleanupObligation
    implements Revision3VoiceTakePreviewCleanupObligation {
  _DialogFakeCleanupObligation({required this.failuresBeforeClean});

  final int failuresBeforeClean;
  int attempts = 0;
  bool _cleaned = false;

  @override
  bool get isCleaned => _cleaned;

  @override
  Future<void> retryCleanup() async {
    attempts++;
    if (attempts <= failuresBeforeClean) {
      throw StateError('fake retained cleanup failure');
    }
    _cleaned = true;
  }
}

final class _DialogFakePreviewPlayer
    implements Revision3VoiceTakePreviewPlayer {
  _DialogFakePreviewPlayer(this.events);

  final List<String> events;
  final StreamController<Revision3VoiceTakePreviewPlayerSnapshot> _snapshots =
      StreamController<Revision3VoiceTakePreviewPlayerSnapshot>.broadcast();
  Revision3VoiceTakePreviewPlayerSnapshot _snapshot =
      const Revision3VoiceTakePreviewPlayerSnapshot.idle();
  Completer<void>? stopGate;
  int stopAttempts = 0;
  int failuresBeforeStop = 0;

  @override
  Revision3VoiceTakePreviewPlayerSnapshot get snapshot => _snapshot;

  @override
  Stream<Revision3VoiceTakePreviewPlayerSnapshot> get snapshots =>
      _snapshots.stream;

  void _emit(Revision3VoiceTakePreviewPlayerSnapshot value) {
    _snapshot = value;
    _snapshots.add(value);
  }

  @override
  Future<void> open(String path) async {
    events.add('player:open:$path');
    _emit(
      const Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
        duration: Duration(seconds: 10),
      ),
    );
  }

  @override
  Future<void> pause() async {
    events.add('player:pause');
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.paused,
        position: _snapshot.position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> play() async {
    events.add('player:play');
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
        position: _snapshot.position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> seek(Duration position) async {
    events.add('player:seek:${position.inMilliseconds}');
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: _snapshot.phase,
        position: position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> stopAndUnload() async {
    stopAttempts++;
    events.add('player:stop');
    final gate = stopGate;
    if (gate != null) {
      stopGate = null;
      await gate.future;
    }
    if (stopAttempts <= failuresBeforeStop) {
      throw StateError('fake player stop failure');
    }
    _snapshot = const Revision3VoiceTakePreviewPlayerSnapshot.idle();
  }

  @override
  Future<void> dispose() async {
    events.add('player:dispose');
    await _snapshots.close();
  }
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
  bool previewable = false,
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
  if (previewable) {
    final assets = <Object?>[];
    for (var index = 0; index < takes.length; index++) {
      final sha = ''.padLeft(64, index.isEven ? 'b' : 'c');
      final logicalName = 'asghan_take_$index.ogg';
      takes[index]['asset_references'] = <Object?>[
        <String, Object?>{
          'role': 'voice_audio',
          'sha256': sha,
          'byte_len': 42 + index,
          'logical_name': logicalName,
          'expected_media_type': 'audio/ogg',
          'resolution': 'resolved',
        },
      ];
      assets.add(<String, Object?>{
        'sha256': sha,
        'byte_len': 42 + index,
        'media_type': 'audio/ogg',
        'class': 'voice_audio',
      });
    }
    json['assets'] = assets;
  }
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
  head: _publicationHead(),
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
  head: _publicationHead(),
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
  head: _publicationHead(),
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
  head: _publicationHead(),
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

AuthoringWorkingHead _publicationHead() =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': 1,
          'sha256': List<String>.filled(64, 'a').join(),
        },
      }),
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
