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
  _EventsErrorCore({required this.eventsError});

  /// The error message returned for every events query.
  final String eventsError;

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
          return {
            'ok': false,
            'error': {'message': eventsError},
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
}
