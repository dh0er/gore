import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/domain/lock_catalog.dart';
import 'package:goresave/features/editor/ui/locks_panel.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../../../support/l10n_test_app.dart';

/// Minimal fake core: enough for an [EditorNotifier] to settle on a save, plus
/// a canned `private.locks.list` answer.
class _LocksCore implements GoresaveCoreService {
  _LocksCore({this.unlocked = const [], this.writable = true});

  final List<String> unlocked;
  final bool writable;

  @override
  String get description => 'locks-fake';

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
            'backend': 'kraken',
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
          },
        };
      case 'private.locks.list':
        return {
          'ok': true,
          'data': {
            'unlocked': unlocked,
            'writable': writable ? ['private.locks.setUnlocked'] : <String>[],
          },
        };
      default:
        return {
          'ok': true,
          'data': {
            'total': 0,
            'offset': 0,
            'limit': 50,
            'characters': <Object?>[],
            'entries': <Object?>[],
          },
        };
    }
  }
}

/// A stand-in catalog: two Old Camp locks and one New Camp door, enough to
/// exercise both filters and the region rail without the bundled asset.
LockCatalog _catalog() => LockCatalog.fromJsonString('''
{"version":1,"locks":[
  {"n":"IO_OC_CHEST_DEXTER","k":"chest","a":"OC","d":3,"l":"OC_Chest_Dexter_Lock"},
  {"n":"IO_OC_CHEST_GOMEZ_01","k":"chest","a":"OC","d":7,"keys":["ItKe_Gomez_01"]},
  {"n":"NC_Cave_Tavern_Door","k":"door","a":"NC","d":4},
  {"n":"OC_Guards_Cell_01_Door","k":"door","a":"OC","l":"OC_Guards_Cell_01_Lock"}
]}
''');

/// Just the two areas the stub catalog uses, so the rail can be asserted
/// without reaching for the 884 KB bundled asset through `rootBundle`.
LocationCatalog _locations() => LocationCatalog.fromJsonString('''
{"version":1,
 "areas":[{"id":"OC","label":"Old Camp","locId":null},
          {"id":"NC","label":"New Camp","locId":null}],
 "spots":[]}
''');

Future<EditorNotifier> _notifier(WidgetTester tester, _LocksCore core) async {
  final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
  addTearDown(() {
    // Panels using the provider override let Riverpod dispose the notifier.
    if (notifier.mounted) notifier.dispose();
  });
  // inspect() schedules real timers the widget-test clock will not advance.
  await tester.runAsync(() async {
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
  });
  return notifier;
}

Widget _wrap(Widget child, {String locale = 'en'}) => ProviderScope(
  child: MaterialApp(
    locale: Locale(locale),
    localizationsDelegates: testLocalizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: SizedBox(width: 1100, height: 760, child: child)),
  ),
);

/// The panel loads through the notifier's core queue — a real Future the
/// widget-test clock does not advance on its own.
Future<void> _settle(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
  }
}

/// The switch inside one lock row. The row is a ListTile whose trailing pairs
/// the state word with the switch, so the switch is addressed through the row.
Switch _switchOf(WidgetTester tester, String lock) => tester.widget<Switch>(
  find.descendant(
    of: find.byKey(ValueKey('lock-$lock')),
    matching: find.byType(Switch),
  ),
);

Future<LocksDetail> _panel(
  WidgetTester tester,
  EditorNotifier notifier, {
  bool editable = true,
}) async {
  final panel = LocksDetail(
    notifier: notifier,
    editable: editable,
    reloadKey: notifier.state.inspection!,
    theme: ThemeData.light(),
    catalogOverride: _catalog(),
    locationsOverride: _locations(),
  );
  await tester.pumpWidget(
    ProviderScope(
      overrides: [editorProvider.overrideWith((ref) => notifier)],
      child: _wrap(panel),
    ),
  );
  await _settle(tester);
  return panel;
}

void main() {
  testWidgets('every lock is listed, locked and unlocked alike', (
    tester,
  ) async {
    final core = _LocksCore(unlocked: ['IO_OC_CHEST_DEXTER']);
    await _panel(tester, await _notifier(tester, core));

    // The save knows one of the three; the other two come from the catalog and
    // would be invisible if the panel only read the save.
    expect(find.text('IO_OC_CHEST_DEXTER'), findsOneWidget);
    expect(find.text('IO_OC_CHEST_GOMEZ_01'), findsOneWidget);
    expect(find.text('NC_Cave_Tavern_Door'), findsOneWidget);
    expect(find.text('4 of 4'), findsOneWidget);

    expect(_switchOf(tester, 'IO_OC_CHEST_DEXTER').value, isTrue);
    expect(_switchOf(tester, 'NC_Cave_Tavern_Door').value, isFalse);

    // The switch alone does not say which way is which: each row spells its
    // state out beside it.
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')),
        matching: find.text('Unlocked'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('lock-NC_Cave_Tavern_Door')),
        matching: find.text('Locked'),
      ),
      findsOneWidget,
    );
  });

  testWidgets('unlocking a lock queues one setUnlocked edit', (tester) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    await tester.tap(find.byKey(const ValueKey('lock-IO_OC_CHEST_GOMEZ_01')));
    await _settle(tester);

    final pending = notifier.pendingEditFor('world.locks');
    expect(pending, isNotNull);
    expect(pending!.edits.single, {
      'path': 'private.locks.setUnlocked',
      'value': {'lock': 'IO_OC_CHEST_GOMEZ_01', 'unlocked': true},
    });
  });

  testWidgets('locking an opened lock again queues the reverse edit', (
    tester,
  ) async {
    final core = _LocksCore(unlocked: ['NC_Cave_Tavern_Door']);
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    await tester.tap(find.byKey(const ValueKey('lock-NC_Cave_Tavern_Door')));
    await _settle(tester);

    expect(notifier.pendingEditFor('world.locks')!.edits.single, {
      'path': 'private.locks.setUnlocked',
      'value': {'lock': 'NC_Cave_Tavern_Door', 'unlocked': false},
    });
  });

  testWidgets('toggling back to the saved value drops the edit', (
    tester,
  ) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    final row = find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER'));
    await tester.tap(row);
    await _settle(tester);
    expect(notifier.pendingEditFor('world.locks'), isNotNull);

    await tester.tap(row);
    await _settle(tester);
    expect(
      notifier.pendingEditFor('world.locks'),
      isNull,
      reason: 'a second tap is an undo, not a second stacked edit',
    );
  });

  testWidgets('clearing drafts elsewhere resets the displayed lock state', (
    tester,
  ) async {
    final notifier = await _notifier(tester, _LocksCore());
    await _panel(tester, notifier);
    await tester.tap(find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')));
    await _settle(tester);
    expect(_switchOf(tester, 'IO_OC_CHEST_DEXTER').value, isTrue);

    notifier.clearPendingEdit(EditorNotifier.pendingLocksKey);
    await _settle(tester);
    expect(_switchOf(tester, 'IO_OC_CHEST_DEXTER').value, isFalse);
    expect(find.byKey(const Key('locks-discard-pending')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('lock-NC_Cave_Tavern_Door')));
    await _settle(tester);
    expect(
      notifier
          .pendingEditFor(EditorNotifier.pendingLocksKey)!
          .edits
          .single['value'],
      {'lock': 'NC_Cave_Tavern_Door', 'unlocked': true},
    );
  });

  testWidgets('several toggles ride in one pending entry', (tester) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    await tester.tap(find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')));
    await _settle(tester);
    await tester.tap(find.byKey(const ValueKey('lock-NC_Cave_Tavern_Door')));
    await _settle(tester);

    // setUnlocked is value-addressed, so the whole panel batches into one
    // write instead of one write per row.
    expect(notifier.pendingEditFor('world.locks')!.edits, hasLength(2));
  });

  testWidgets('the region rail filters the list and counts each region', (
    tester,
  ) async {
    final core = _LocksCore();
    await _panel(tester, await _notifier(tester, core));

    // Area codes resolve to the game's own region names.
    expect(find.text('Old Camp'), findsOneWidget);
    expect(find.text('New Camp'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('locks-region-Old Camp')));
    await tester.pumpAndSettle();

    expect(find.text('IO_OC_CHEST_DEXTER'), findsOneWidget);
    expect(find.text('NC_Cave_Tavern_Door'), findsNothing);
    expect(find.text('3 of 4'), findsOneWidget);
  });

  testWidgets('the kind filter separates chests from doors', (tester) async {
    final core = _LocksCore();
    await _panel(tester, await _notifier(tester, core));

    await tester.tap(find.byKey(const ValueKey('locks-kind-doors')));
    await tester.pumpAndSettle();

    expect(find.text('NC_Cave_Tavern_Door'), findsOneWidget);
    expect(find.text('IO_OC_CHEST_DEXTER'), findsNothing);
    expect(find.text('2 of 4'), findsOneWidget);
  });

  testWidgets('the state filter separates opened from still-locked', (
    tester,
  ) async {
    final core = _LocksCore(unlocked: ['IO_OC_CHEST_DEXTER']);
    await _panel(tester, await _notifier(tester, core));

    await tester.tap(find.byKey(const ValueKey('locks-state-locked')));
    await tester.pumpAndSettle();

    expect(find.text('IO_OC_CHEST_DEXTER'), findsNothing);
    expect(find.text('IO_OC_CHEST_GOMEZ_01'), findsOneWidget);
    expect(find.text('3 of 4'), findsOneWidget);
  });

  testWidgets('search matches the lock name and its key', (tester) async {
    final core = _LocksCore();
    await _panel(tester, await _notifier(tester, core));

    await tester.enterText(find.byKey(const Key('locks-search')), 'Gomez');
    await tester.pumpAndSettle();

    expect(find.text('IO_OC_CHEST_GOMEZ_01'), findsOneWidget);
    expect(find.text('IO_OC_CHEST_DEXTER'), findsNothing);
  });

  testWidgets('a key is searchable by its raw id AND its localized name', (
    tester,
  ) async {
    // The row shows the game's own item name while the catalog only knows the
    // technical id. Both spellings have to reach the row, or the field promises
    // a key search that fails on exactly what the reader can see.
    // The localization is pinned here so the assertion does not depend on
    // whether the machine running the test has extracted the game's strings.
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          locCatalogProvider.overrideWith(
            (ref) async => {
              'itke_gomez_01': {'english': 'Ore Baron Key'},
            },
          ),
        ],
        child: MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: testLocalizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SizedBox(
              width: 1100,
              height: 760,
              child: LocksDetail(
                notifier: notifier,
                editable: true,
                reloadKey: notifier.state.inspection!,
                theme: ThemeData.light(),
                catalogOverride: _catalog(),
                locationsOverride: _locations(),
              ),
            ),
          ),
        ),
      ),
    );
    await _settle(tester);

    expect(find.textContaining('Ore Baron Key'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('locks-search')),
      'ItKe_Gomez_01',
    );
    await tester.pumpAndSettle();
    expect(find.text('IO_OC_CHEST_GOMEZ_01'), findsOneWidget);
    expect(find.text('1 of 4'), findsOneWidget);

    await tester.enterText(find.byKey(const Key('locks-search')), 'ore baron');
    await tester.pumpAndSettle();
    expect(find.text('IO_OC_CHEST_GOMEZ_01'), findsOneWidget);
    expect(find.text('1 of 4'), findsOneWidget);
  });

  testWidgets('technical ids add the key id beside its name', (tester) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    // Off by default: the row names the key, nothing else.
    expect(find.textContaining('ItKe_Gomez_01'), findsNothing);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(LocksDetail)),
    );
    container.read(showObjectIdsProvider.notifier).set(true);
    await _settle(tester);

    expect(find.textContaining('(ItKe_Gomez_01)'), findsOneWidget);
  });

  testWidgets('a truncated region name offers its full text on hover', (
    tester,
  ) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await tester.pumpWidget(
      _wrap(
        LocksDetail(
          notifier: notifier,
          editable: true,
          reloadKey: notifier.state.inspection!,
          theme: ThemeData.light(),
          catalogOverride: _catalog(),
          locationsOverride: LocationCatalog.fromJsonString('''
{"version":1,
 "areas":[{"id":"OC","label":"A name far too long for a two-hundred pixel rail to show","locId":null},
          {"id":"NC","label":"New Camp","locId":null}],
 "spots":[]}
'''),
        ),
      ),
    );
    await _settle(tester);

    // The cut label gets the tooltip; one that fits must not, or every row
    // would sprout a hover that repeats what is already on screen.
    expect(
      find.byTooltip(
        'A name far too long for a two-hundred pixel rail to show',
      ),
      findsOneWidget,
    );
    expect(find.byTooltip('New Camp'), findsNothing);
  });

  testWidgets('a lock with no authored tier omits the difficulty meter', (
    tester,
  ) async {
    // A lock id does not supply a pickable difficulty. Keep the lock row,
    // but do not turn a missing catalog tier into a displayed zero.
    final core = _LocksCore();
    await _panel(tester, await _notifier(tester, core));

    final meter = find.descendant(
      of: find.byKey(const ValueKey('lock-OC_Guards_Cell_01_Door')),
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is Tooltip &&
            (widget.message?.startsWith('Difficulty') ?? false),
      ),
    );
    expect(
      find.byKey(const ValueKey('lock-OC_Guards_Cell_01_Door')),
      findsOneWidget,
    );
    expect(meter, findsNothing);
  });

  testWidgets('the difficulty tooltip names both scales', (tester) async {
    // Tier 3 fills two of four pips; a tooltip claiming "3 of 7" beside two
    // filled bars is the confusion this wording exists to remove.
    final core = _LocksCore();
    await _panel(tester, await _notifier(tester, core));

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')),
        matching: find.byTooltip('Difficulty 2 of 4 (internal tier 3 of 7)'),
      ),
      findsOneWidget,
    );
  });

  testWidgets('an unknown lock stays unclassified and visible under All', (
    tester,
  ) async {
    // The catalog is cook-specific: a save from another build can carry a lock
    // this one has no row for, and dropping it would hide editable state.
    final core = _LocksCore(unlocked: ['IO_AM_CHEST_05']);
    await _panel(tester, await _notifier(tester, core));

    expect(find.text('IO_AM_CHEST_05'), findsOneWidget);
    expect(find.text('Not in this game version'), findsOneWidget);
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('lock-IO_AM_CHEST_05')),
        matching: find.byIcon(Icons.lock_outline),
      ),
      findsOneWidget,
    );
    for (final kind in ['chests', 'doors']) {
      await tester.tap(find.byKey(ValueKey('locks-kind-$kind')));
      await tester.pumpAndSettle();
      expect(find.text('IO_AM_CHEST_05'), findsNothing);
      expect(find.text('2 of 5'), findsOneWidget);
    }
    await tester.tap(find.byKey(const ValueKey('locks-kind-all')));
    await tester.pumpAndSettle();
    expect(find.text('IO_AM_CHEST_05'), findsOneWidget);
    expect(find.text('5 of 5'), findsOneWidget);
  });

  testWidgets('Other isolates unknown regions independently from All regions', (
    tester,
  ) async {
    final core = _LocksCore(unlocked: ['IO_AM_CHEST_05']);
    await _panel(tester, await _notifier(tester, core));
    final all = find.byKey(const ValueKey('locks-region-All regions'));
    final other = find.byKey(const ValueKey('locks-region-Other'));
    expect(tester.widget<ListTile>(all).selected, isTrue);
    expect(tester.widget<ListTile>(other).selected, isFalse);

    await tester.tap(other);
    await tester.pumpAndSettle();
    expect(tester.widget<ListTile>(all).selected, isFalse);
    expect(tester.widget<ListTile>(other).selected, isTrue);
    expect(find.text('IO_AM_CHEST_05'), findsOneWidget);
    expect(find.text('IO_OC_CHEST_DEXTER'), findsNothing);
    expect(find.text('1 of 5'), findsOneWidget);

    await tester.tap(all);
    await tester.pumpAndSettle();
    expect(tester.widget<ListTile>(all).selected, isTrue);
    expect(tester.widget<ListTile>(other).selected, isFalse);
    expect(find.text('IO_AM_CHEST_05'), findsOneWidget);
    expect(find.text('IO_OC_CHEST_DEXTER'), findsOneWidget);
    expect(find.text('5 of 5'), findsOneWidget);
  });

  testWidgets('a save with no editable lock set stays read-only', (
    tester,
  ) async {
    final core = _LocksCore(writable: false);
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    expect(find.text('This save has no editable lock set.'), findsOneWidget);
    expect(_switchOf(tester, 'IO_OC_CHEST_DEXTER').onChanged, isNull);

    await tester.tap(find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')));
    await _settle(tester);
    expect(notifier.pendingEditFor('world.locks'), isNull);
  });

  testWidgets('queued toggles can be discarded', (tester) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await _panel(tester, notifier);

    await tester.tap(find.byKey(const ValueKey('lock-IO_OC_CHEST_DEXTER')));
    await _settle(tester);
    expect(notifier.pendingEditFor('world.locks'), isNotNull);

    await tester.tap(find.byKey(const Key('locks-discard-pending')));
    await _settle(tester);
    expect(notifier.pendingEditFor('world.locks'), isNull);
  });

  testWidgets('the section is translated', (tester) async {
    final core = _LocksCore();
    final notifier = await _notifier(tester, core);
    await tester.pumpWidget(
      _wrap(
        LocksDetail(
          notifier: notifier,
          editable: true,
          reloadKey: notifier.state.inspection!,
          theme: ThemeData.light(),
          catalogOverride: _catalog(),
          locationsOverride: _locations(),
        ),
        locale: 'de',
      ),
    );
    await _settle(tester);

    expect(find.widgetWithText(ChoiceChip, 'Truhen'), findsOneWidget);
    expect(find.widgetWithText(ChoiceChip, 'Türen'), findsOneWidget);
    expect(find.widgetWithText(ChoiceChip, 'Verschlossen'), findsOneWidget);
    expect(find.text('Alle Regionen'), findsOneWidget);
    // Each row names its own state, so the switch is never read alone.
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('lock-NC_Cave_Tavern_Door')),
        matching: find.text('Verschlossen'),
      ),
      findsOneWidget,
    );
  });
}
