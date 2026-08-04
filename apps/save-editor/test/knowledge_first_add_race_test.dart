import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

/// Regression test for first knowledge adds. A missing character knowledge-map
/// entry must not trigger a preparatory write; one value-addressed pending edit
/// creates the map entry and adds the requested token atomically on global Save.
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

  testWidgets('first knowledge add is queued without writing the save', (
    tester,
  ) async {
    final core = _FirstAddKnowledgeCoreService();
    await pumpApp(tester, core);

    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();

    // Player is selected by default; open the Wissen sub-tab. The Hero has
    // no knowledge entry yet (benign "has no knowledge entry" core error),
    // so the add affordance is enabled in the no-knowledge-yet state.
    await tester.tap(find.widgetWithText(Tab, 'Dialog Knowledge'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.widgetWithText(TextField, 'Add knowledge entry'),
      'Voiceline_info_new',
    );
    await tester.tap(find.byTooltip('Add'));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(GoresaveApp)),
    );
    final pending = container.read(editorProvider).pendingEdits;
    expect(pending, contains('progression.knowledge'));
    final edits = pending['progression.knowledge']!.edits;
    expect(edits, hasLength(1));
    expect(edits.single['path'], 'private.knowledge.setEntry');
    final value = (edits.single['value'] as Map).cast<String, Object?>();
    expect(value, {
      'character': 'Hero',
      'entry': 'Voiceline_info_new',
      'present': true,
    });

    // And the UI agrees: the pending add renders and the global Save
    // button counts exactly this one edit. No write hit the core.
    expect(find.text('Info New'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
  });

  testWidgets('knowledge rows show a badge for every entry type', (
    tester,
  ) async {
    final core = _FirstAddKnowledgeCoreService(
      knowledgeEntries: const [
        'ChoiceDiegoHello',
        'Info_Diego_Hello',
        'Voiceline_info_diego',
        'Topic_Diego_209799',
        'UnclassifiedKnowledge',
      ],
    );
    await pumpApp(tester, core);

    await tester.tap(find.widgetWithText(Tab, 'Characters'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(Tab, 'Dialog Knowledge'));
    await tester.pumpAndSettle();

    final badges = <String, String>{
      'knowledge-type-choice-ChoiceDiegoHello': 'Choice',
      'knowledge-type-info-Info_Diego_Hello': 'Information',
      'knowledge-type-voiceLine-Voiceline_info_diego': 'Voice line',
      'knowledge-type-topic-Topic_Diego_209799': 'Topic',
      'knowledge-type-other-UnclassifiedKnowledge': 'Other',
    };
    final badgeWidths = <double>[];
    for (final badge in badges.entries) {
      final finder = find.byKey(ValueKey(badge.key));
      expect(finder, findsOneWidget);
      badgeWidths.add(tester.getSize(finder).width);
      expect(
        find.descendant(of: finder, matching: find.text(badge.value)),
        findsOneWidget,
      );
    }
    expect(
      badgeWidths.toSet(),
      hasLength(1),
      reason: 'Every dialog-knowledge type badge must have the same width.',
    );
  });
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// Fake core for one save whose Hero has no knowledge-map entry yet.
class _FirstAddKnowledgeCoreService implements GoresaveCoreService {
  _FirstAddKnowledgeCoreService({this.knowledgeEntries});

  final requests = <_RecordedRequest>[];
  final List<String>? knowledgeEntries;

  @override
  String get description => 'first-add-knowledge-fake-core';

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
        // A NEW map each call → a new SaveInspection instance → the details'
        // reloadKey provably changes on every refresh (as in the real app).
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
        return {
          'ok': true,
          'data': {
            'total': 1,
            'characters': [
              {
                'globalId': 'Hero',
                'uniqueName': 'Hero',
                'isDead': false,
                'hasInventory': false,
                'hasKnowledge': false,
                'hasEvents': true,
              },
            ],
          },
        };
      case 'write_save':
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      case 'query_progression':
        if (payload['section'] == 'knowledge' &&
            payload['character'] == 'Hero') {
          final entries = knowledgeEntries;
          if (entries != null) {
            return {
              'ok': true,
              'data': {
                'section': 'knowledge',
                'character': 'Hero',
                'total': entries.length,
                'offset': 0,
                'limit': 50,
                'count': entries.length,
                'entries': entries,
                'setPath': const [
                  'CharacterKnowledgeByUniqueName',
                  '{Hero}',
                  'Knowledge',
                ],
              },
            };
          }
          return {
            'ok': false,
            'error': {'message': "Character 'Hero' has no knowledge entry"},
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
