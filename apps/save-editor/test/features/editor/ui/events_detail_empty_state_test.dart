import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/ui/progression_panel.dart'
    show EventsDetail;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';

import '../../../support/l10n_test_app.dart';

/// The core reports a character with no `LongTermMemoryByGlobalId` entry via a
/// benign `character "..." has no memory entry` error. That is a NORMAL state
/// for an NPC the hero never interacted with, so [EventsDetail] must fold it
/// into a neutral "no events" empty state instead of a red error — while every
/// OTHER events-query failure still renders as an error. Mirrors
/// KnowledgeDetail's handling of the equivalent "has no knowledge entry" error.

/// Minimal fake core: serves scan/inspect/list_backups/check_codec so an
/// [EditorNotifier] can settle on a selected save, and answers the events
/// `query_progression` with a canned error.
class _EventsErrorCore implements GoresaveCoreService {
  _EventsErrorCore({this.eventsError, this.eventsData});

  /// The error message returned for every events query.
  final String? eventsError;
  final Map<String, Object?>? eventsData;

  @override
  String get description => 'events-error-fake';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': payload['path'],
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 100,
                'sha1': 'a',
                'status': 'ok',
                'playerSaveName': 'Save A',
              },
            ],
            'profiles': <Object?>[],
          },
        };
      case 'inspect_save':
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 100,
            'sha1': 'a',
            'private': {
              'status': 'decoded',
              'progression': {'status': 'ok'},
              'typedParse': {'status': 'ok'},
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': <Object?>[],
            'companionBackups': <Object?>[],
          },
        };
      case 'check_codec':
        return {
          'ok': true,
          'data': {
            'backend': 'ooz_kraken',
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
          },
        };
      case 'query_progression':
        if (payload['section'] == 'events') {
          if (eventsData != null) {
            return {'ok': true, 'data': eventsData};
          }
          return {
            'ok': false,
            'error': {'message': eventsError ?? 'events query failed'},
          };
        }
        return {
          'ok': true,
          'data': {'total': 0, 'offset': 0, 'limit': 50},
        };
      default:
        return {
          'ok': true,
          'data': {'total': 0, 'offset': 0, 'limit': 50},
        };
    }
  }
}

Future<EditorNotifier> _notifier(
  WidgetTester tester,
  _EventsErrorCore core,
) async {
  final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
  // The constructor's refresh()/checkCodec() and inspect() schedule real
  // Timers/microtasks. The widget-test fake-async clock does not advance bare
  // Future.delayed/Timers without pumping, so awaiting them directly here hangs
  // the setup. runAsync runs them against the real clock so they complete.
  await tester.runAsync(() async {
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
  });
  return notifier;
}

Widget _wrap(Widget child) => ProviderScope(
  // EventsDetail is a ConsumerStatefulWidget, so it needs a ProviderScope
  // ancestor even though this detail reads no providers in build.
  child: MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: testLocalizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: SizedBox(width: 800, height: 600, child: child)),
  ),
);

// The events load chains on the notifier's internal core queue — a real
// Future. The widget-test fake clock will not advance it, so interleave
// runAsync (lets the real microtasks/timers behind the queue complete) with
// pump (rebuilds the tree against new state).
Future<void> _settle(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
  }
}

Future<void> _pumpDetail(
  WidgetTester tester,
  EditorNotifier notifier, {
  required String globalId,
}) async {
  await tester.pumpWidget(
    _wrap(
      Builder(
        builder: (context) => EventsDetail(
          globalId: globalId,
          notifier: notifier,
          editable: true,
          reloadKey: notifier.state.inspection!,
          theme: Theme.of(context),
        ),
      ),
    ),
  );
  await _settle(tester);
}

void main() {
  const emptyStateText = 'No events for this character.';

  testWidgets(
    'the benign no-memory-entry error renders as a neutral empty state',
    (tester) async {
      // Exact shape of the core error (gore-save query_progression events
      // branch): `character {character:?} has no memory entry`.
      const coreError = 'character "Lizard-WP_A" has no memory entry';
      final core = _EventsErrorCore(eventsError: coreError);
      final notifier = await _notifier(tester, core);

      await _pumpDetail(tester, notifier, globalId: 'Lizard-WP_A');

      // Neutral empty state, NOT the red error: the core message is folded
      // away entirely.
      expect(find.text(emptyStateText), findsOneWidget);
      expect(find.text(coreError), findsNothing);
      // The hint is styled as muted body text, not with the error color.
      final context = tester.element(find.text(emptyStateText));
      final scheme = Theme.of(context).colorScheme;
      final text = tester.widget<Text>(find.text(emptyStateText));
      expect(text.style?.color, scheme.onSurfaceVariant);
      expect(text.style?.color, isNot(scheme.error));
    },
  );

  testWidgets('a real events-load failure still renders as an error', (
    tester,
  ) async {
    const coreError = 'memory subtree parse failed';
    final core = _EventsErrorCore(eventsError: coreError);
    final notifier = await _notifier(tester, core);

    await _pumpDetail(tester, notifier, globalId: 'Lizard-WP_A');

    // The genuine failure retains the core detail inside its localized error
    // envelope; the neutral empty state must NOT swallow it.
    final localizedError = AppLocalizationsEn().editorProgressionQueryFailed(
      coreError,
    );
    expect(find.text(localizedError), findsOneWidget);
    expect(find.text(emptyStateText), findsNothing);
    final context = tester.element(find.text(localizedError));
    final scheme = Theme.of(context).colorScheme;
    final text = tester.widget<Text>(find.text(localizedError));
    expect(text.style?.color, scheme.error);
  });

  testWidgets(
    'delete queues immediately, stacks removals, and supports row/all undo',
    (tester) async {
      const arrayPath = [
        'LongTermMemoryByGlobalId',
        '{Hero}',
        'MemorizedEvents',
      ];
      final core = _EventsErrorCore(
        eventsData: {
          'character': 'Hero',
          'total': 3,
          'offset': 0,
          'limit': 50,
          'arrayPath': arrayPath,
          'events': [
            {
              'index': 8,
              'tags': ['Memory.Quest.Started'],
              'timeSeconds': 8.0,
            },
            {
              'index': 5,
              'tags': ['Memory.Document.Read'],
              'timeSeconds': 5.0,
            },
            {
              'index': 2,
              'tags': ['Memory.Chapter.Completed'],
              'timeSeconds': 2.0,
            },
          ],
        },
      );
      final notifier = await _notifier(tester, core);
      await _pumpDetail(tester, notifier, globalId: 'Hero');

      expect(find.byTooltip('Remove event'), findsNWidgets(3));
      await tester.tap(find.byTooltip('Remove event').first);
      await tester.pump();

      // Removal is only queued: no confirmation modal interrupts the flow.
      expect(find.byType(AlertDialog), findsNothing);
      expect(
        notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index),
        [8],
      );
      // Other remove buttons remain active; duplicate is exclusive while a
      // removal batch is pending.
      expect(find.byTooltip('Remove event'), findsNWidgets(2));
      expect(
        tester
            .widgetList<IconButton>(
              find.widgetWithIcon(IconButton, Icons.delete_outline),
            )
            .every((button) => button.onPressed != null),
        isTrue,
      );
      expect(
        tester
            .widgetList<IconButton>(
              find.widgetWithIcon(IconButton, Icons.copy_outlined),
            )
            .every((button) => button.onPressed == null),
        isTrue,
      );

      await tester.tap(find.byTooltip('Remove event').first);
      await tester.pump();
      expect(find.byType(AlertDialog), findsNothing);
      expect(
        notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index),
        [8, 5],
      );

      // Undo on one row removes only that original index.
      await tester.tap(find.byTooltip('Cancel').first);
      await tester.pump();
      expect(
        notifier.pendingMemoryEventEdits('Hero').map((edit) => edit.index),
        [5],
      );

      // The banner action clears the remaining batch in one click.
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await tester.pump();
      expect(notifier.pendingMemoryEventEdits('Hero'), isEmpty);
      expect(find.byTooltip('Remove event'), findsNWidgets(3));
    },
  );

  testWidgets(
    'event cards show a meaningful localized summary and expandable raw facts',
    (tester) async {
      final core = _EventsErrorCore(
        eventsData: {
          'character': 'Hero',
          'total': 1,
          'offset': 0,
          'limit': 50,
          'arrayPath': const [
            'LongTermMemoryByGlobalId',
            '{Hero}',
            'MemorizedEvents',
          ],
          'events': const [
            {
              'index': 17,
              'tags': ['Memory.Quest.Started', 'Quest.Context.Main'],
              'timeSeconds': 90061.0,
              'durationSeconds': 90.0,
              'magnitude': 2.0,
              'instigator': 'Hero',
              'affected': 'OC_STT_Diego-WorldPointActor_Diego',
              'optionalClass1':
                  '/Script/Angelscript.Quest_OldCamp_OCCHAPTER1_TEST',
              'position': {'x': 10.5, 'y': 20, 'z': -3},
              'payload': {
                'type': '/Script/G1R.TestPayload',
                'fieldCount': 1,
                'fields': [
                  {
                    'name': 'EventName',
                    'type': 'NameProperty',
                    'value': 'TestEvent',
                  },
                ],
              },
            },
          ],
        },
      );
      final notifier = await _notifier(tester, core);
      await _pumpDetail(tester, notifier, globalId: 'Hero');

      // The raw tag is no longer the headline: the card explains the action and
      // derives a readable quest subject even when no extracted game catalog is
      // available in this isolated widget test.
      expect(find.textContaining('Quest started'), findsOneWidget);
      expect(find.text('Quest'), findsOneWidget);
      expect(find.text('Day 1, 01:01:01'), findsOneWidget);
      expect(find.text('Memory.Quest.Started'), findsNothing);

      await tester.tap(find.byIcon(Icons.expand_more));
      await tester.pumpAndSettle();

      expect(find.text('Details'), findsOneWidget);
      expect(find.text('Tags'), findsOneWidget);
      expect(find.text('Memory.Quest.Started'), findsOneWidget);
      expect(find.text('Hero'), findsWidgets);
      expect(find.text('Diego'), findsWidgets);
      expect(find.text('00:01:30'), findsOneWidget);
      expect(find.text('Position'), findsOneWidget);
      expect(find.textContaining('X 10.5'), findsOneWidget);
      expect(
        find.textContaining('Payload · /Script/G1R.TestPayload'),
        findsOneWidget,
      );
      expect(find.text('EventName'), findsOneWidget);
      expect(find.text('TestEvent'), findsOneWidget);
    },
  );
}
