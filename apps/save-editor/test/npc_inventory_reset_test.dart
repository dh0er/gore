import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';
import 'support/detail_tabs.dart';

/// Task 15: proves the "Reset inventory" button (Task 14) queues a single
/// `private.inventory.reset` pending edit under the player's `'inventory'`
/// pending key, carrying the notifier's resolved `activeResourcesLevel()`
/// (which falls back to 'Gothic' with no active profile — see
/// `editor_notifier_test.dart`'s `activeResourcesLevel` fallback case), and
/// that a queued reset is mutually exclusive with every other pending
/// inventory edit in both directions.
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

  Future<void> openPlayerInventory(WidgetTester tester) async {
    // The Player row is pinned + selected by default in the shared Charaktere
    // master list, so opening the Inventory sub-tab shows the player card
    // with no further selection needed.
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
  }

  testWidgets(
    'reset queues a solo pending edit under the player inventory key',
    (tester) async {
      final core = _ResettableInventoryCoreService();
      await pumpApp(tester, core);
      await openPlayerInventory(tester);

      // Sanity: the fixture's item is visible and the Reset button is enabled
      // (canReset gated open by writable + privateEditable + typedVerified +
      // canCompress).
      expect(find.text('Orenugget'), findsOneWidget);
      final resetFinder = find.widgetWithText(
        OutlinedButton,
        'Reset inventory',
      );
      expect(resetFinder, findsOneWidget);
      expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);

      await tester.tap(resetFinder);
      await tester.pumpAndSettle();

      final notifier = _findNotifier(tester);
      final pending = notifier.pendingEditFor('inventory');
      expect(pending, isNotNull);
      expect(pending!.edits, hasLength(1));
      expect(pending.edits.single['path'], 'private.inventory.reset');
      final value = pending.edits.single['value'] as Map<String, Object?>;
      // No active profile in this fixture's scan → activeResourcesLevel()
      // falls back to 'Gothic'.
      expect(value['resourcesLevel'], 'Gothic');
      // Player edits carry no actorId.
      expect(value.containsKey('actorId'), isFalse);

      // The global Save button reflects exactly one queued edit.
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
    },
  );

  testWidgets('a queued reset disables Add and hides count editors', (
    tester,
  ) async {
    final core = _ResettableInventoryCoreService();
    await pumpApp(tester, core);
    await openPlayerInventory(tester);

    // Before reset: the count editor renders and Add is enabled.
    expect(find.widgetWithText(TextField, 'Count'), findsOneWidget);
    final addFinder = find.widgetWithText(FilledButton, 'Add item');
    expect(addFinder, findsOneWidget);
    expect(tester.widget<FilledButton>(addFinder).onPressed, isNotNull);

    await tester.tap(find.widgetWithText(OutlinedButton, 'Reset inventory'));
    await tester.pumpAndSettle();

    // Reset supersedes every other inventory edit: the structural-edit-gated
    // count editor disappears (the row falls back to plain text) and Add is
    // blocked.
    expect(find.widgetWithText(TextField, 'Count'), findsNothing);
    expect(
      tester.widget<FilledButton>(addFinder).onPressed,
      isNull,
      reason: 'Add must be blocked while a reset is pending',
    );

    // The pending-reset summary row is shown in place of the item list.
    expect(find.text('Reset to game-start inventory'), findsOneWidget);
  });

  testWidgets('a pending count edit blocks the Reset button until cleared', (
    tester,
  ) async {
    final core = _ResettableInventoryCoreService();
    await pumpApp(tester, core);
    await openPlayerInventory(tester);

    final resetFinder = find.widgetWithText(OutlinedButton, 'Reset inventory');
    expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);

    // Queue an ordinary count change on the fixture's single item.
    await tester.enterText(find.widgetWithText(TextField, 'Count'), '99');
    await tester.pump();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    // Reset is now blocked (mutual exclusion the other direction).
    expect(
      tester.widget<OutlinedButton>(resetFinder).onPressed,
      isNull,
      reason: 'Reset must be blocked while a count edit is pending',
    );

    // Undo the pending count edit via the header's undo action — Reset must
    // become available again.
    await tester.tap(find.byIcon(Icons.undo_outlined));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save'), findsOneWidget);
    expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);
  });

  testWidgets(
    'an empty but resolvable inventory still renders the Reset button, '
    'not the no-stacks message pane',
    (tester) async {
      // Bug fix: the card (with its Reset button) must appear even when the
      // inventory is EMPTY — a user who emptied it wants to reset back to the
      // game-start kit. The core advertises ONLY `private.inventory.reset`
      // (empty inventory ⇒ no setItemCount/addItem/removeItem), and the card's
      // empty-inventory early-return must not fire while reset is available.
      final core = _EmptyResettableInventoryCoreService();
      await pumpApp(tester, core);
      await openPlayerInventory(tester);

      // The card renders: the Reset button is present and enabled, and the
      // "no item stacks" message pane is NOT shown.
      final resetFinder = find.widgetWithText(
        OutlinedButton,
        'Reset inventory',
      );
      expect(resetFinder, findsOneWidget);
      expect(tester.widget<OutlinedButton>(resetFinder).onPressed, isNotNull);
      expect(
        find.text('No item stacks found in the decoded private payload.'),
        findsNothing,
        reason:
            'the empty-inventory message pane must not suppress the card '
            'when reset is available',
      );
      // The empty-inventory body text is shown inside the card instead.
      expect(find.text('This inventory is empty.'), findsOneWidget);

      // And the Reset button still queues the solo reset edit.
      await tester.tap(resetFinder);
      await tester.pumpAndSettle();
      final notifier = _findNotifier(tester);
      final pending = notifier.pendingEditFor('inventory');
      expect(pending, isNotNull);
      expect(pending!.edits, hasLength(1));
      expect(pending.edits.single['path'], 'private.inventory.reset');
    },
  );
}

/// Reaches into the widget tree to fetch the live [EditorNotifier] instance so
/// tests can assert against `pendingEditFor` directly, mirroring how the
/// sibling inventory tests read state via `core.requests` post-save. Reset's
/// solo/mutual-exclusion behaviour is local-only (no write until Save), so the
/// notifier itself — not a write_save payload — is the thing to inspect.
EditorNotifier _findNotifier(WidgetTester tester) {
  final element = tester.element(find.byType(GoresaveApp));
  final container = ProviderScope.containerOf(element);
  return container.read(editorProvider.notifier);
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// Fake core backing a player inventory with ONE removable/count-editable
/// item and `writable` advertising `setItemCount` + `addItem` + `reset` so
/// both the count editor, the Add button, and the Reset button all render —
/// letting the mutual-exclusion assertions exercise the real gates.
class _ResettableInventoryCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'resettable-inventory-fake-core';

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
                'playerSaveName': 'Save',
                'chapterId': 1,
                'autoSave': true,
                'slotName': 'G1R-001',
              },
            ],
            // No profiles: EditorNotifier.activeResourcesLevel() falls back to
            // 'Gothic' (matches editor_notifier_test.dart's fallback case).
            'profiles': <Object?>[],
            'activeProfileId': null,
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
                'itemStackCount': 1,
                'items': [
                  {
                    'id': 'ItMi_Orenugget',
                    'path': '/Script/Angelscript.ItMi_Orenugget',
                    'count': 42,
                    'removable': true,
                    'slotId': 0,
                    'containerType': 'MainContainer',
                  },
                ],
                'mainContainerPaths': ['/Script/Angelscript.ItMi_Orenugget'],
                'writable': [
                  'private.inventory.setItemCount',
                  'private.inventory.addItem',
                  'private.inventory.reset',
                ],
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
      case 'private.characters.list':
        // Player row is pinned by the shared master list itself; no spawned
        // actors are needed for this fixture.
        return {
          'ok': true,
          'data': {'total': 0, 'characters': <Object?>[]},
        };
      case 'write_save':
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

/// Fake core backing an EMPTY but typed-resolvable player inventory: zero item
/// stacks, no rows, and `writable` advertising ONLY `private.inventory.reset`
/// (an empty MainContainer resolves, so reset is offered while setItemCount /
/// addItem / removeItem are not). Proves the card + Reset button still render
/// for an empty inventory instead of the "no item stacks" message pane.
class _EmptyResettableInventoryCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'empty-resettable-inventory-fake-core';

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
                'playerSaveName': 'Save',
                'chapterId': 1,
                'autoSave': true,
                'slotName': 'G1R-001',
              },
            ],
            'profiles': <Object?>[],
            'activeProfileId': null,
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
              // Empty inventory: no stacks, no rows, no candidates — hasData is
              // false — but the typed MainContainer still resolves, so reset is
              // the only advertised edit.
              'inventory': {
                'itemStackCount': 0,
                'items': <Object?>[],
                'candidates': <Object?>[],
                'scriptPaths': <Object?>[],
                'properties': <Object?>[],
                'mainContainerPaths': <Object?>[],
                'writable': ['private.inventory.reset'],
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
      case 'private.characters.list':
        return {
          'ok': true,
          'data': {'total': 0, 'characters': <Object?>[]},
        };
      case 'write_save':
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}
