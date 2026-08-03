import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

import 'support/ui_settings_test_store.dart';

/// Backups can be named and deleted from the Backups tab. The label and the
/// file name stay separate: the label heads the entry, the file name stays
/// visible as a fact, and an unnamed backup keeps heading itself with its file
/// name.
void main() {
  Future<void> openBackups(WidgetTester tester, GoresaveCoreService core) async {
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
    await tester.tap(find.widgetWithText(Tab, 'Backups'));
    await tester.pumpAndSettle();
  }

  testWidgets('an unnamed backup is headed by its file name', (tester) async {
    await openBackups(tester, _BackupCoreService());

    expect(find.text('G1R-001.sav.bak.100'), findsNWidgets(2));
    expect(find.text('File'), findsOneWidget);
  });

  testWidgets('a named backup keeps the file name as a fact', (tester) async {
    await openBackups(tester, _BackupCoreService(name: 'before the boss'));

    expect(find.text('before the boss'), findsOneWidget);
    // Still readable below the title, never replaced by the label.
    expect(find.text('G1R-001.sav.bak.100'), findsOneWidget);
  });

  testWidgets('naming a backup reaches the core and keeps the file', (
    tester,
  ) async {
    final core = _BackupCoreService();
    await openBackups(tester, core);

    await tester.tap(find.byIcon(Icons.edit_outlined));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).last, 'before the boss');
    await tester.tap(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.widgetWithText(FilledButton, 'Save'),
      ),
    );
    await tester.pumpAndSettle();

    final request = core.requests.lastWhere((r) => r.command == 'rename_backup');
    expect(request.payload['backupPath'], r'C:\tmp\saves\G1R-001.sav.bak.100');
    expect(request.payload['name'], 'before the boss');
  });

  testWidgets('deleting asks first and only then reaches the core', (
    tester,
  ) async {
    final core = _BackupCoreService();
    await openBackups(tester, core);

    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);

    // Backing out must not touch anything.
    await tester.tap(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.widgetWithText(TextButton, 'Cancel'),
      ),
    );
    await tester.pumpAndSettle();
    expect(core.requests.any((r) => r.command == 'delete_backup'), isFalse);

    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.widgetWithText(FilledButton, 'Delete'),
      ),
    );
    await tester.pumpAndSettle();

    final request = core.requests.lastWhere((r) => r.command == 'delete_backup');
    expect(request.payload['backupPath'], r'C:\tmp\saves\G1R-001.sav.bak.100');
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// One save with exactly one backup, optionally carrying a label.
class _BackupCoreService implements GoresaveCoreService {
  _BackupCoreService({this.name});

  final String? name;
  final requests = <_RecordedRequest>[];

  static const _backupPath = r'C:\tmp\saves\G1R-001.sav.bak.100';

  @override
  String get description => 'backup-manage-fake-core';

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
            'backups': [
              {
                'path': _backupPath,
                'fileName': 'G1R-001.sav.bak.100',
                if (name != null) 'name': name,
                'fileSize': 1024,
                'sha1': 'deadbeefdeadbeef',
                'createdEpoch': 100,
                'status': 'ok',
                'scope': 'save',
                'playerSaveName': 'Save',
                'slotName': 'G1R-001',
              },
            ],
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
      case 'delete_backup':
        return {
          'ok': true,
          'data': {'path': _backupPath, 'deleted': true},
        };
      case 'rename_backup':
        return {
          'ok': true,
          'data': {'path': _backupPath, 'name': payload['name']},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}
