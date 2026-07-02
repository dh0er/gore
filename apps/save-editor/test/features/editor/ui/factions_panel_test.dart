import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/ui/world_tab.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../../../support/l10n_test_app.dart';

/// Minimal recording fake core: serves scan/inspect/list_backups/check_codec so
/// an [EditorNotifier] can settle on a selected save, plus a canned
/// `private.factions.list` response. Captures every request for assertions.
class _FactionsCore implements GoresaveCoreService {
  _FactionsCore({required this.guilds});

  final List<Map<String, Object?>> guilds;
  final requests = <_Req>[];

  @override
  String get description => 'factions-fake';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add(_Req(command, Map<String, Object?>.from(payload)));
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
      case 'private.factions.list':
        return {
          'ok': true,
          'data': {'guilds': guilds},
        };
      case 'query_progression':
        // The Quests pane is an always-mounted sibling of the Factions pane;
        // it queries progression in initState. Answer with an empty result so
        // its spinner settles (else pumpAndSettle never ends).
        return {
          'ok': true,
          'data': {
            'quests': <Object?>[],
            'events': <Object?>[],
            'entries': <Object?>[],
            'characters': <Object?>[],
            'total': 0,
            'offset': 0,
            'limit': 50,
          },
        };
      default:
        // Benign empty success for any other read a sibling pane may issue, so
        // no pane is left spinning during a factions-focused widget test.
        return {
          'ok': true,
          'data': {
            'total': 0,
            'offset': 0,
            'limit': 50,
            'characters': <Object?>[],
            'entries': <Object?>[],
            'events': <Object?>[],
            'quests': <Object?>[],
          },
        };
    }
  }
}

class _Req {
  const _Req(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

Map<String, Object?> _guild(
  String guild,
  String label,
  int total,
  int forgiven,
  int unforgiven, {
  bool isHostile = false,
  Map<String, Object?> crimes = const {},
}) => {
  'guild': guild,
  'label': label,
  'total': total,
  'forgiven': forgiven,
  'unforgiven': unforgiven,
  'isHostile': isHostile,
  'crimes': crimes,
};

Future<EditorNotifier> _notifier(WidgetTester tester, _FactionsCore core) async {
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
  // The World-tab sub-panes (Quests/Factions) are ConsumerStatefulWidgets
  // that ref.watch the editor providers, so they need a ProviderScope ancestor.
  child: MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: testLocalizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: SizedBox(width: 1000, height: 700, child: child)),
  ),
);

// The Factions pane loads via notifier.loadFactions(), which chains on the
// notifier's internal core queue — a real Future. The widget-test fake clock
// will not advance it, so interleave runAsync (lets the real microtasks/timers
// behind the queue complete) with pump (rebuilds the tree against new state).
Future<void> _settle(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
  }
}

void main() {
  testWidgets('Fraktionen entry sits in the World sidebar', (tester) async {
    final core = _FactionsCore(guilds: []);
    final notifier = await _notifier(tester, core);

    await tester.pumpWidget(
      _wrap(
        WorldTab(
          inspection: notifier.state.inspection!,
          notifier: notifier,
          editable: true,
        ),
      ),
    );
    await _settle(tester);

    // English label for factionsSidebar is "Factions".
    expect(find.text('Factions'), findsOneWidget);
  });

  testWidgets('Factions pane lists guilds with a Forgive button', (
    tester,
  ) async {
    final core = _FactionsCore(
      guilds: [
        // Hostile OldCamp: 1 un-forgiven murder + 1 theft.
        _guild(
          'Guild.Human.OldCamp',
          'OldCamp',
          3,
          1,
          2,
          isHostile: true,
          crimes: {'murder': 1, 'theft': 1},
        ),
        // Friendly NewCamp with a fully-forgiven record (no open crimes).
        _guild('Guild.Human.NewCamp', 'NewCamp', 1, 1, 0),
      ],
    );
    final notifier = await _notifier(tester, core);

    await tester.pumpWidget(
      _wrap(
        WorldTab(
          inspection: notifier.state.inspection!,
          notifier: notifier,
          editable: true,
        ),
      ),
    );
    await _settle(tester);

    // Open the Factions section.
    await tester.tap(find.text('Factions'));
    await _settle(tester);

    // Localized guild labels appear.
    expect(find.text('Old Camp'), findsOneWidget);
    expect(find.text('New Camp'), findsOneWidget);
    // Status badges: OldCamp hostile, NewCamp friendly.
    expect(find.text('Hostile'), findsOneWidget);
    expect(find.text('Friendly'), findsOneWidget);
    // Crime-type breakdown for OldCamp (un-forgiven only, zero categories
    // omitted).
    expect(find.text('1 murder · 1 theft'), findsOneWidget);

    // Two Forgive buttons exist; the one for the clean NewCamp guild is
    // disabled (unforgiven == 0), the OldCamp one is enabled.
    final buttons = tester
        .widgetList<FilledButton>(
          find.widgetWithText(FilledButton, 'Forgive'),
        )
        .toList();
    expect(buttons, hasLength(2));
    final enabledCount = buttons.where((b) => b.onPressed != null).length;
    expect(enabledCount, 1, reason: 'only the guild with open crimes is enabled');
  });

  testWidgets(
    'tapping Forgive registers a pending edit and reflects optimistically',
    (tester) async {
      final core = _FactionsCore(
        guilds: [
          _guild(
            'Guild.Human.OldCamp',
            'OldCamp',
            3,
            1,
            2,
            isHostile: true,
            crimes: {'murder': 1, 'assault': 1},
          ),
        ],
      );
      final notifier = await _notifier(tester, core);

      await tester.pumpWidget(
        _wrap(
          WorldTab(
            inspection: notifier.state.inspection!,
            notifier: notifier,
            editable: true,
          ),
        ),
      );
      await _settle(tester);
      await tester.tap(find.text('Factions'));
      await _settle(tester);

      // No write was issued before tapping.
      expect(
        core.requests.where((r) => r.command == 'write_save'),
        isEmpty,
      );

      await tester.tap(find.widgetWithText(FilledButton, 'Forgive'));
      await _settle(tester);

      // A PENDING edit is registered (no immediate write_save).
      expect(
        notifier.state.pendingEdits.containsKey(
          'factions.forgive:Guild.Human.OldCamp',
        ),
        isTrue,
      );
      expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);

      // Optimistic reflect: the row now shows the queued message and the
      // button is disabled.
      expect(find.text('being forgiven…'), findsOneWidget);
      final button = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, 'Forgive'),
      );
      expect(button.onPressed, isNull);
    },
  );

  testWidgets('empty state shows the no-crimes message', (tester) async {
    final core = _FactionsCore(guilds: []);
    final notifier = await _notifier(tester, core);

    await tester.pumpWidget(
      _wrap(
        WorldTab(
          inspection: notifier.state.inspection!,
          notifier: notifier,
          editable: true,
        ),
      ),
    );
    await _settle(tester);
    await tester.tap(find.text('Factions'));
    await _settle(tester);

    expect(find.text('No open crimes against factions.'), findsOneWidget);
  });
}
