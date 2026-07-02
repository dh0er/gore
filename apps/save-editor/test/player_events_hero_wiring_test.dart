import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/providers/data_providers.dart';

/// The player's memory events live under the save's own "Hero" ACTOR GlobalId.
/// The Charaktere master list hides that actor row (the pinned Player row
/// represents it) and `loadAllCharacters` stashes its GlobalId, which the tab
/// wires into the player's Ereignisse detail — so with the Player selected,
/// opening Events must query the Hero actor's events, not show an empty state.
void main() {
  Future<void> pumpApp(WidgetTester tester, GoresaveCoreService core) async {
    await tester.binding.setSurfaceSize(const Size(1400, 1000));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          coreServiceProvider.overrideWithValue(core),
          editorSettingsStoreProvider.overrideWithValue(
            const NoopEditorSettingsStore(),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'selecting the Player and opening Events queries the Hero actor GlobalId',
    (tester) async {
      final core = _HeroEventsCoreService();
      await pumpApp(tester, core);

      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();

      // Master-list dedup: the Hero actor row is NOT listed (no 'Hero' title
      // or subtitle anywhere in the list); the normal NPC is.
      final inMasterList = find.byType(CharacterMasterList);
      expect(
        find.descendant(of: inMasterList, matching: find.text('Hero')),
        findsNothing,
      );
      expect(
        find.descendant(of: inMasterList, matching: find.text('Lizard')),
        findsOneWidget,
      );

      // The Player is selected by default — open the Events sub-tab.
      await tester.tap(find.widgetWithText(Tab, 'Events'));
      await tester.pumpAndSettle();

      // The events detail queried the events for the HERO actor's GlobalId
      // (stashed when the character index loaded), not a null selection.
      final eventCalls = core.requests.where(
        (r) =>
            r.command == 'query_progression' &&
            r.payload['section'] == 'events' &&
            r.payload['character'] == 'Hero',
      );
      expect(eventCalls, isNotEmpty);

      // And the canned Hero event actually renders (header + tag row) — the
      // old empty state ("select a character") is gone for the player.
      expect(find.text('Events — Hero'), findsOneWidget);
      expect(find.text('MEMORY_HERO_EVENT'), findsOneWidget);
      expect(find.text('Select a character to see events'), findsNothing);
    },
  );

  testWidgets(
    'a late-arriving hero GlobalId re-keys the already-open Events detail',
    (tester) async {
      // The hero id is stashed only when the character index LOADS. If the
      // user opens Events before that (slow decompress), the detail builds
      // with a null id; when the id lands, EventsDetail.didUpdateWidget must
      // re-select and load the Hero events — no manual refresh needed.
      final core = _GatedHeroEventsCoreService();
      await pumpApp(tester, core);

      // Open Characters while the character index is still loading (gated).
      // Plain pump()s — pumpAndSettle would spin on the loading indicator.
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      await tester.pump(const Duration(milliseconds: 400));
      await tester.tap(find.widgetWithText(Tab, 'Events'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      await tester.pump(const Duration(milliseconds: 400));

      // No hero id yet → the detail's own empty state, and no events query.
      expect(find.text('Select a character to see events'), findsOneWidget);
      expect(
        core.requests.where((r) => r.command == 'query_progression'),
        isEmpty,
      );

      // The index lands: heroGlobalId is stashed → the tab re-passes it →
      // the detail re-selects (null → 'Hero') and queries the Hero events.
      core.charactersListGate.complete();
      await tester.pumpAndSettle();

      expect(
        core.requests.where(
          (r) =>
              r.command == 'query_progression' &&
              r.payload['section'] == 'events' &&
              r.payload['character'] == 'Hero',
        ),
        isNotEmpty,
      );
      expect(find.text('Events — Hero'), findsOneWidget);
      expect(find.text('MEMORY_HERO_EVENT'), findsOneWidget);
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// Minimal fake core for the Charaktere tab: one save, a character index that
/// carries the save's own Hero ACTOR row + one NPC, and a one-event memory
/// page for the Hero GlobalId.
class _HeroEventsCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'hero-events-fake-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add(_RecordedRequest(command, Map<String, Object?>.from(payload)));
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': r'C:\tmp\saves',
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 914367,
                'sha1': 'abc',
                'status': 'ok',
                'persistentProfileId': 0,
                'playerSaveName': 'Save',
                'chapterId': 1,
                'autoSave': true,
                'slotName': 'G1R-001',
              },
            ],
            'profiles': [
              {
                'profileId': 0,
                'profileName': '0',
                'quickSaveSlots': <String>[],
                'autoSaveSlots': <String>[],
                'savedSlots': ['G1R-001'],
              },
            ],
            'activeProfileId': 0,
          },
        };
      case 'inspect_save':
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 914367,
            'sha1': 'abc',
            'public': {'slotName': 'G1R-001', 'playerSaveName': 'Save'},
            'private': {
              'status': 'decoded',
              'preview': false,
              'decompressedSize': 9,
              'typedParse': {'status': 'ok', 'propertyCount': 1, 'maxDepth': 1},
              'player': {
                'saveVersionNumber': 17,
                'playerName': 'Hero',
                'attributes': <Object?>[],
                'writable': <String>[],
              },
              'inventory': {
                'itemStackCount': 0,
                'items': <Object?>[],
                'writable': <String>[],
              },
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
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
            'adapter': 'pure_rust_kraken',
            'message': 'Codec host is ready.',
          },
        };
      case 'search_typed_properties':
        // The player Attribute sub-tab (initially visible) searches the hero
        // attribute subtree; an empty result keeps the pane quiet.
        return {
          'ok': true,
          'data': {
            'query': payload['query'],
            'offset': 0,
            'limit': 1000,
            'total': 0,
            'count': 0,
            'results': <Object?>[],
          },
        };
      case 'private.characters.list':
        // The index INCLUDES the save's own Hero actor row (as real saves do)
        // so the master list must dedup it and stash its GlobalId.
        return {
          'ok': true,
          'data': {
            'total': 2,
            'characters': [
              {
                'globalId': 'Hero',
                'uniqueName': 'Hero',
                'isDead': false,
                'hasInventory': false,
                'hasKnowledge': true,
                'hasEvents': true,
              },
              {
                'globalId': 'Lizard-WP_A',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': false,
                'hasKnowledge': false,
                'hasEvents': false,
              },
            ],
          },
        };
      case 'query_progression':
        if (payload['section'] == 'events' && payload['character'] == 'Hero') {
          return {
            'ok': true,
            'data': {
              'section': 'events',
              'character': 'Hero',
              'arrayPath': <String>['MemorizedEvents'],
              'total': 1,
              'offset': 0,
              'limit': payload['limit'] ?? 50,
              'events': [
                {
                  'index': 0,
                  'tags': ['MEMORY_HERO_EVENT'],
                  'magnitude': 1.0,
                  'timeSeconds': 42.0,
                },
              ],
            },
          };
        }
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake progression query'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

/// [_HeroEventsCoreService] whose character index does not respond until
/// [charactersListGate] completes — the "slow decompress" case where the user
/// reaches the Events sub-tab before the hero GlobalId is known.
class _GatedHeroEventsCoreService extends _HeroEventsCoreService {
  final charactersListGate = Completer<void>();

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'private.characters.list') {
      await charactersListGate.future;
    }
    return super.execute(command, payload: payload);
  }
}
