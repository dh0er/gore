import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/ui/slot_repair_banner.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';

/// A savegame an older build damaged — slots whose id no longer matches their
/// position — must be called out, and the repair must reach the core as
/// `private.inventory.repairSlots`. A healthy save must stay quiet.
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
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
        ],
        child: const GoresaveApp(),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> openPlayerInventory(WidgetTester tester) async {
    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
  }

  testWidgets('a damaged save is flagged and the repair reaches the core', (
    tester,
  ) async {
    final core = _InventoryCoreService(misalignedSlots: 3);
    await pumpApp(tester, core);
    await openPlayerInventory(tester);

    expect(find.text('Damaged inventory slots'), findsOneWidget);
    expect(
      find.textContaining('3 inventory slots whose id no longer matches'),
      findsOneWidget,
    );

    // One press queues it — no confirmation step, Discard takes it back.
    await tester.tap(find.widgetWithText(FilledButton, 'Repair'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.textContaining('Repair queued'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();
    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(1));
    expect(edits.single['path'], 'private.inventory.repairSlots');
  });

  testWidgets('the overview warns first, and shares one queued repair', (
    tester,
  ) async {
    // The damage is save-wide, so someone who never opens the Inventory tab has
    // to meet it on the overview — and queueing there must show up in the
    // inventory's copy of the banner as the same pending edit, not a second one.
    await pumpApp(tester, _InventoryCoreService(misalignedSlots: 3));

    expect(find.byType(SlotRepairBanner), findsOneWidget);
    // Topmost: nothing else on the overview may sit above the warning.
    final overview = find
        .ancestor(
          of: find.byType(SlotRepairBanner),
          matching: find.byType(ListView),
        )
        .last;
    final bannerTop = tester.getRect(find.byType(SlotRepairBanner)).top;
    final ownCards = tester
        .widgetList<Card>(
          find.descendant(
            of: find.byType(SlotRepairBanner),
            matching: find.byType(Card),
          ),
        )
        .toSet();
    for (final card in tester.widgetList<Card>(
      find.descendant(of: overview, matching: find.byType(Card)),
    )) {
      if (ownCards.contains(card)) continue;
      expect(
        bannerTop,
        lessThan(tester.getRect(find.byWidget(card)).top),
        reason: 'the warning belongs above everything else on the overview',
      );
    }

    await tester.tap(find.widgetWithText(FilledButton, 'Repair'));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);

    await openPlayerInventory(tester);
    expect(find.textContaining('Repair queued'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
  });

  testWidgets('the warning also shows while an NPC inventory is selected', (
    tester,
  ) async {
    // The damage is reported and repaired save-wide, so it must not disappear
    // just because the Characters tab is showing an NPC.
    await pumpApp(tester, _InventoryCoreService(misalignedSlots: 3));
    await openPlayerInventory(tester);
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();

    expect(find.text('Damaged inventory slots'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Repair'), findsOneWidget);
  });

  testWidgets('the repair is applied last, after id-addressed edits', (
    tester,
  ) async {
    // The repair rewrites every misaligned m_Id, so an NPC removal pinned to the
    // id the UI showed has to be applied FIRST or it would no longer match.
    // The two now batch into one write_save, which the core applies
    // sequentially — so position inside that payload is what carries the
    // requirement, and the repair must be the LAST edit in it.
    final core = _InventoryCoreService(misalignedSlots: 3, npcRemovable: true);
    await pumpApp(tester, core);
    await openPlayerInventory(tester);

    await tester.tap(find.widgetWithText(FilledButton, 'Repair'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();
    // The bundled item stats decide what a row IS, and until they answer the
    // editor offers no removal — reading them needs real async.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.delete_outline).first);
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(FilledButton, 'Save (2)'));
    await tester.pumpAndSettle();

    final writes = core.requests.where((r) => r.command == 'write_save').map((
      r,
    ) {
      return (r.payload['edits'] as List)
          .cast<Map<String, Object?>>()
          .map((e) => e['path'])
          .toList();
    }).toList();
    // Every edit of this Save, flattened in the exact order the core applies
    // them across the (now single) write.
    final applied = [for (final write in writes) ...write];
    expect(applied, [
      'private.inventory.removeItem',
      'private.inventory.repairSlots',
    ]);
  });

  testWidgets('without write capability the warning stays, the action goes', (
    tester,
  ) async {
    // The damage is there either way and the game will act on the wrong item,
    // so the reader must still be told. Only the action — which could be queued
    // but never applied — is withheld, with the reason in its place.
    await pumpApp(
      tester,
      _InventoryCoreService(misalignedSlots: 3, canCompress: false),
    );
    await openPlayerInventory(tester);

    expect(find.text('Damaged inventory slots'), findsOneWidget);
    expect(find.textContaining('cannot be written'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Repair'), findsNothing);
  });

  testWidgets('a repair that is not offered says so, not "cannot be written"', (
    tester,
  ) async {
    // Two different obstacles: this save could take a write, the repair is just
    // not on offer for it. Saying it cannot be written would be untrue.
    await pumpApp(
      tester,
      _InventoryCoreService(misalignedSlots: 3, offersRepair: false),
    );
    await openPlayerInventory(tester);

    expect(
      find.textContaining('not available for this savegame'),
      findsOneWidget,
    );
    expect(find.textContaining('cannot be written'), findsNothing);
  });

  testWidgets('damage without an offered repair still warns', (tester) async {
    // The count and the op are separate signals. A save reported as damaged by
    // a core that offers no repair must still say so — silence would leave the
    // reader believing the save is fine.
    await pumpApp(
      tester,
      _InventoryCoreService(misalignedSlots: 3, offersRepair: false),
    );
    await openPlayerInventory(tester);

    expect(find.text('Damaged inventory slots'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Repair'), findsNothing);
  });

  testWidgets('a healthy save shows no repair banner', (tester) async {
    await pumpApp(tester, _InventoryCoreService(misalignedSlots: 0));
    await openPlayerInventory(tester);

    expect(find.text('Damaged inventory slots'), findsNothing);
    expect(find.widgetWithText(FilledButton, 'Repair'), findsNothing);
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// Player inventory with one item; `misalignedSlots` drives the damage report.
class _InventoryCoreService implements GoresaveCoreService {
  _InventoryCoreService({
    required this.misalignedSlots,
    this.canCompress = true,
    this.npcRemovable = false,
    this.offersRepair = true,
  });

  final int misalignedSlots;

  /// Whether the codec can compress; without it no private edit can be written.
  final bool canCompress;

  /// Whether the NPC inventory offers removal (an id-addressed edit).
  final bool npcRemovable;

  /// Whether the core advertises the repair op alongside the damage count.
  final bool offersRepair;
  final requests = <_RecordedRequest>[];

  static const _cheesePath = '/Script/Angelscript.ItFo_Cheese';

  @override
  String get description => 'slot-repair-fake-core';

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
                'itemStackCount': 1,
                'itemScope': 'player_inventory_region',
                'items': [
                  {
                    'id': 'ItFo_Cheese',
                    'path': _cheesePath,
                    'count': 3,
                    'removable': true,
                    'slotId': 0,
                    'containerType': 'MainContainer',
                  },
                ],
                'mainContainerPaths': [_cheesePath],
                'slotIntegrity': {
                  'misalignedSlots': misalignedSlots,
                  'containers': misalignedSlots > 0 ? 1 : 0,
                },
                'writable': [
                  'private.inventory.setItemCount',
                  if (misalignedSlots > 0 && offersRepair)
                    'private.inventory.repairSlots',
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
            'canCompress': canCompress,
            'status': 'ready',
            'adapter': 'pure_rust_kraken',
            'message': 'Codec host is ready.',
          },
        };
      case 'private.characters.list':
        return {
          'ok': true,
          'data': {
            'total': 1,
            'characters': [
              {
                'globalId': 'Lizard-A',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': true,
                'hasKnowledge': false,
                'hasEvents': false,
              },
            ],
          },
        };
      case 'private.npc.list':
        return {
          'ok': true,
          'data': {
            'total': 1,
            'offset': 0,
            'limit': payload['limit'] ?? 100,
            'count': 1,
            'npcs': [
              {'id': 'Lizard-A', 'name': 'Lizard A'},
            ],
          },
        };
      case 'private.npc.inventory':
        return {
          'ok': true,
          'data': {
            'id': payload['id'],
            'itemStackCount': 1,
            'items': [
              {
                'id': 'ItFo_Cheese',
                'path': _cheesePath,
                'count': 1,
                'removable': true,
                'slotId': 0,
                'containerType': 'MainContainer',
              },
            ],
            'mainContainerPaths': [_cheesePath],
            'writable': [
              'private.inventory.setItemCount',
              if (npcRemovable) 'private.inventory.removeItem',
            ],
          },
        };
      case 'private.npc.attributes':
        return {
          'ok': true,
          'data': {'attributes': <Object?>[]},
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
