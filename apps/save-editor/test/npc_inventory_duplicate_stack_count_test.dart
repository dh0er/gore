import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';
import 'support/detail_tabs.dart';

/// Regression for Codex (P2): an NPC MainContainer with two stacks that share
/// the SAME item id/path but differ by slotId/count. The count editors must be
/// keyed by the slot-aware row key so a rebuild never reuses one slot's
/// controller for the other equal-id row (which would show the wrong count).
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

  String fieldText(WidgetTester tester, Finder field) => tester
      .widget<EditableText>(
        find.descendant(of: field, matching: find.byType(EditableText)),
      )
      .controller
      .text;

  testWidgets('duplicate same-path NPC stacks keep independent count fields', (
    tester,
  ) async {
    final core = _DuplicateStackNpcInventoryCoreService();
    await pumpApp(tester, core);

    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();

    // Two rows for the same item (slot 0 = 3, slot 1 = 7).
    final fields = find.widgetWithText(TextField, 'Count');
    expect(fields, findsNWidgets(2));
    expect(fieldText(tester, fields.at(0)), '3');
    expect(fieldText(tester, fields.at(1)), '7');

    // Edit slot 0 → slot 1's field must keep its own count (no controller
    // bleed between the two equal-id rows).
    await tester.enterText(fields.at(0), '99');
    await tester.pump();
    expect(fieldText(tester, fields.at(0)), '99');
    expect(fieldText(tester, fields.at(1)), '7');

    // Save carries the edit for slot 0 only, with its slotId discriminator.
    await tester.tap(find.widgetWithText(FilledButton, 'Save (1)'));
    await tester.pumpAndSettle();
    final write = core.requests.lastWhere((r) => r.command == 'write_save');
    final edits = (write.payload['edits'] as List).cast<Map<String, Object?>>();
    expect(edits, hasLength(1));
    final value = edits.single['value'] as Map<String, Object?>;
    expect(value['count'], 99);
    expect(value['slotId'], 0);
  });

  testWidgets('queuing removal of one duplicate stack hides ONLY that slot', (
    tester,
  ) async {
    final core = _DuplicateStackNpcInventoryCoreService();
    await pumpApp(tester, core);

    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(detailTab('Inventory'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Lizard-A'));
    await tester.pumpAndSettle();

    // Two same-path rows (counts 3 and 7).
    expect(find.widgetWithText(TextField, 'Count'), findsNWidgets(2));

    // Queue removal of the FIRST stack (slot 0).
    await tester.tap(find.byIcon(Icons.delete_outline).first);
    await tester.pumpAndSettle();

    // Slot-aware hide: only slot 0 leaves the list; slot 1 stays — shown as a
    // static "×7" (a queued removal blocks count editing, so the TextFields
    // become plain text). A path-based hide would have dropped BOTH equal-path
    // rows, leaving no "×7" at all.
    expect(find.text('×7'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Count'), findsNothing);
  });

  testWidgets(
    'inventory count rows stay compact and keep values beside item names',
    (tester) async {
      final core = _DuplicateStackNpcInventoryCoreService();
      await pumpApp(tester, core);

      // Explicit selection also makes this geometry regression independent of
      // whether startup auto-opens the active profile's first save.
      if (find.widgetWithText(Tab, 'Characters').evaluate().isEmpty) {
        await tester.tap(find.text('Save').first);
        await tester.pumpAndSettle();
      }
      await tester.tap(find.widgetWithText(Tab, 'Characters'));
      await tester.pumpAndSettle();
      await tester.tap(detailTab('Inventory'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Lizard-A'));
      await tester.pumpAndSettle();

      Finder countFields() => find.widgetWithText(TextField, 'Count');
      Finder rows() =>
          find.ancestor(of: countFields(), matching: find.byType(ListTile));
      final countTargets = find.byKey(
        const ValueKey('inventory-count-editor-touch-target'),
      );

      expect(countFields(), findsNWidgets(2));
      expect(rows(), findsNWidgets(2));
      expect(countTargets, findsNWidgets(2));

      final firstRow = tester.getRect(rows().at(0));
      final secondRow = tester.getRect(rows().at(1));
      final firstField = tester.getRect(countFields().at(0));
      final firstName = tester.getRect(find.text('Cheese').first);

      // On a wide detail pane the value column must not drift all the way to
      // the card's far edge. Rows stay left-aligned and deliberately bounded.
      expect(firstRow.width, lessThanOrEqualTo(560));
      expect(firstField.left - firstName.right, lessThan(360));

      // Dense here means no decorative inter-row whitespace, not undersized
      // controls: the labeled input keeps a 48 px touch target.
      expect(secondRow.top - firstRow.bottom, lessThanOrEqualTo(0.01));
      expect(
        tester.getSize(countTargets.first).height,
        greaterThanOrEqualTo(48),
      );

      // The intermediate width used to leave too little title space beside the
      // trailing count/delete controls, especially with an equipped badge.
      // It now switches to the stacked row before that breakpoint gap.
      await tester.binding.setSurfaceSize(const Size(1100, 800));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final mediumRow = tester.getRect(rows().first);
      final mediumField = tester.getRect(countFields().first);
      expect(mediumField.right, lessThanOrEqualTo(mediumRow.right));

      // The same row remains bounded by its available pane at the minimum
      // desktop width; neither the labeled field nor its delete target spills
      // beyond the row.
      await tester.binding.setSurfaceSize(const Size(960, 800));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final narrowRow = tester.getRect(rows().first);
      final narrowField = tester.getRect(countFields().first);
      expect(narrowField.right, lessThanOrEqualTo(narrowRow.right));
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// NPC MainContainer with TWO stacks of the same item (id/path) at slots 0/1.
class _DuplicateStackNpcInventoryCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  @override
  String get description => 'duplicate-stack-npc-inventory-fake-core';

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
                'writable': ['private.inventory.setItemCount'],
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
        // Backs the Charaktere master list (one unpaginated response). The
        // globalId is rendered as the row subtitle, so tests select a row by
        // tapping its GlobalId text (e.g. 'Lizard-A').
        return {
          'ok': true,
          'data': {
            'total': 2,
            'characters': [
              {
                'globalId': 'Lizard-A',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': false,
                'hasKnowledge': false,
                'hasEvents': false,
              },
              {
                'globalId': 'Lizard-B',
                'uniqueName': 'Lizard',
                'isDead': false,
                'hasInventory': false,
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
            'itemStackCount': 2,
            'items': [
              {
                'id': 'Cheese',
                'path': '/Script/Angelscript.ItFo_Cheese',
                'count': 3,
                'equipped': true,
                'removable': true,
                'slotId': 0,
              },
              {
                'id': 'Cheese',
                'path': '/Script/Angelscript.ItFo_Cheese',
                'count': 7,
                'removable': true,
                'slotId': 1,
              },
            ],
            'mainContainerPaths': <String>[],
            'writable': [
              'private.inventory.setItemCount',
              'private.inventory.addItem',
              'private.inventory.removeItem',
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
