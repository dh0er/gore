import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/providers/data_providers.dart';

/// Regression test for the first-knowledge-add race (Codex P2):
///
/// Adding an entry for a character with NO knowledge set yet goes
/// `_addEntry` → `_ensureCharacterEntry` → `applyAddKnowledgeCharacter`
/// (an immediate core write that REFRESHES the inspection) → internal
/// `loadKnowledgeEntries` reload → queue the pending entry-add.
///
/// The refresh produces a new inspection, so `KnowledgeDetail.didUpdateWidget`
/// fires (reloadKey changed) WHILE the internal reload is still in flight.
/// Before the fix that handler called `_selectCharacter` (bumping
/// `_entriesEpoch`) and cleared `_pending`; `_ensureCharacterEntry`'s stale
/// guard then saw a foreign epoch, returned false, and `_addEntry` bailed —
/// the save was left with a freshly created EMPTY knowledge set and the
/// user's typed entry was silently dropped.
///
/// The fake core GATES the post-write knowledge reload (mirroring the
/// mid-fetch harnesses in editor_notifier_test.dart) so the refresh-driven
/// `didUpdateWidget` provably fires BEFORE the internal reload completes;
/// the test then asserts the entry-add still lands in the notifier's pending
/// registry under 'progression.knowledge'.
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
    'first knowledge add survives the inspection refresh its own write '
    'triggers (entry-add is queued, not dropped)',
    (tester) async {
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
      // Kick off the first-add flow. Plain pump()s from here on —
      // pumpAndSettle would spin forever on the gated reload below.
      await tester.tap(find.byTooltip('Add'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));
      await tester.pump(const Duration(milliseconds: 400));

      // At this point the flow is parked INSIDE _ensureCharacterEntry: the
      // addCharacter write committed and its refresh already delivered a new
      // inspection (so didUpdateWidget has fired with a changed reloadKey),
      // but the internal entries reload is still gated. Nothing may be queued
      // yet.
      expect(core.addCharacterWrites, hasLength(1));
      expect(core.postWriteEntryLoads, greaterThanOrEqualTo(1));
      final container = ProviderScope.containerOf(
        tester.element(find.byType(GoresaveApp)),
      );
      expect(
        container.read(editorProvider).pendingEdits,
        isNot(contains('progression.knowledge')),
      );

      // Release the gated reload; the flow resumes (reload → duplicate check
      // → queue) and must NOT treat the same-character refresh as staleness.
      core.entriesGate.complete();
      await tester.pumpAndSettle();

      // The pending registry carries the entry-add against the POST-refresh
      // setPath — the user's entry was queued, not silently dropped.
      final pending = container.read(editorProvider).pendingEdits;
      expect(pending, contains('progression.knowledge'));
      final edits = pending['progression.knowledge']!.edits;
      expect(edits, hasLength(1));
      expect(edits.single['path'], 'private.typed.setAdd');
      final value = (edits.single['value'] as Map).cast<String, Object?>();
      expect(value['value'], 'Voiceline_info_new');
      expect(value['path'], [
        'CharacterKnowledgeByUniqueName',
        '{Hero}',
        'Knowledge',
      ]);

      // And the UI agrees: the pending add renders and the global Save
      // button counts exactly this one edit. Only the addCharacter write ever
      // hit the core — the entry itself is pending, not written.
      expect(find.text('Voiceline_info_new'), findsOneWidget);
      expect(find.widgetWithText(FilledButton, 'Save (1)'), findsOneWidget);
      expect(
        core.requests.where((r) => r.command == 'write_save'),
        hasLength(1),
      );
    },
  );
}

class _RecordedRequest {
  const _RecordedRequest(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}

/// Fake core for the first-add race: one save whose Hero has NO knowledge
/// entry until a `private.knowledge.addCharacter` write creates it. Knowledge
/// queries AFTER that write block on [entriesGate], so the test controls the
/// interleaving between the write's inspection refresh (didUpdateWidget) and
/// `_ensureCharacterEntry`'s internal reload.
class _FirstAddKnowledgeCoreService implements GoresaveCoreService {
  final requests = <_RecordedRequest>[];

  /// Recorded `write_save` payloads carrying the addCharacter edit.
  final addCharacterWrites = <Map<String, Object?>>[];

  /// Completing this releases every knowledge-entries query issued after the
  /// addCharacter write (they all await it; completed = pass-through).
  final entriesGate = Completer<void>();

  /// Number of knowledge-entries queries issued AFTER the addCharacter write
  /// (i.e. loads that hit the gate).
  int postWriteEntryLoads = 0;

  bool _characterCreated = false;

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
        final edits = (payload['edits'] as List?) ?? const [];
        final isAddCharacter = edits.whereType<Map>().any(
          (e) => e['path'] == 'private.knowledge.addCharacter',
        );
        if (isAddCharacter) {
          _characterCreated = true;
          addCharacterWrites.add(Map<String, Object?>.from(payload));
        }
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      case 'query_progression':
        if (payload['section'] == 'knowledge' &&
            payload['character'] == 'Hero') {
          if (!_characterCreated) {
            // The benign shape the core emits for a character the hero never
            // interacted with — KnowledgeDetail folds this into its
            // no-knowledge-yet state and still offers the add affordance.
            return {
              'ok': false,
              'error': {
                'message': "Character 'Hero' has no knowledge entry",
              },
            };
          }
          // Post-write loads park here until the test opens the gate; the
          // duplicate-check queries run after that and pass straight through.
          postWriteEntryLoads += 1;
          await entriesGate.future;
          return {
            'ok': true,
            'data': {
              'section': 'knowledge',
              'character': 'Hero',
              'total': 0,
              'offset': payload['offset'] ?? 0,
              'limit': payload['limit'] ?? 200,
              'count': 0,
              'entries': <String>[],
              'setPath': [
                'CharacterKnowledgeByUniqueName',
                '{Hero}',
                'Knowledge',
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
