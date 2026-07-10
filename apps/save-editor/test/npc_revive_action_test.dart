import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/npc_attributes_panel.dart';

import 'support/l10n_test_app.dart';

/// Tests for the NPC "Revive" action. It now lives on the Status row at the top
/// of the core ("Hauptwerte") group detail (no longer a separate sidebar pane).
/// It drives a STANDALONE structural edit (`private.npc.revive`) the core won't
/// batch with peers. Invoking it registers a PENDING edit under the per-NPC key
/// `npc.revive:$id` — the global Save button applies it via saveAllPending's
/// splicing split — rather than writing the file immediately on tap. The row
/// reflects a queued revive optimistically.
void main() {
  const savePath = r'C:\tmp\saves\G1R-001.sav';

  // A single core (Health) attribute so the core group renders and is selected
  // by default — the Status row sits at its top.
  NpcAttributeRow healthRow() {
    final prefix = [
      'm_GenericData',
      '{CharacterStates}',
      'AnyCharacterType',
      'AttributeSetsByClass',
      '{/Script/G1R.AttributeSet_Health}',
      'Attributes',
      '{Health}',
    ];
    return NpcAttributeRow(
      key: 'Health',
      base: 40,
      current: 25,
      basePath: [...prefix, 'BaseValue'],
      currentPath: [...prefix, 'CurrentValue'],
    );
  }

  Widget panel({
    required NpcStatusConfig status,
  }) {
    return wrapWithL10n(
      Scaffold(
        body: SizedBox(
          width: 900,
          height: 600,
          child: NpcAttributesPanel(
            load: () async =>
                NpcAttributesResult(attributes: [healthRow()]),
            onPendingChanged: (_, _) {},
            editable: true,
            reloadKey: 'k',
            status: status,
          ),
        ),
      ),
    );
  }

  test('setPendingNpcRevive registers a pending edit and writes nothing',
      () async {
    final core = _StructuralCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(savePath);
    final writesBefore =
        core.requests.where((r) => r.command == 'write_save').length;

    notifier.setPendingNpcRevive('Lizard-WP_EF_001');

    // No write fired on tap.
    final writesAfter =
        core.requests.where((r) => r.command == 'write_save').length;
    expect(writesAfter, writesBefore);
    // A pending edit is registered under the per-NPC key.
    final pending = notifier.state.pendingEdits['npc.revive:Lizard-WP_EF_001'];
    expect(pending, isNotNull);
    expect(pending!.edits, hasLength(1));
    final edit = pending.edits.single;
    expect(edit['path'], 'private.npc.revive');
    expect(edit['value'], {'id': 'Lizard-WP_EF_001'});
  });

  test('setPendingNpcRevive is allowed even with other pending edits present',
      () async {
    final core = _StructuralCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(savePath);
    notifier.setPendingEdit(
      'x',
      const PendingSaveEdit(
        edits: [
          {'path': 'public.m_PlayerSaveName', 'value': 'Draft'},
        ],
      ),
    );

    notifier.setPendingNpcRevive('Lizard-WP_EF_001');

    // Both the unrelated edit and the new NPC edit coexist; nothing was written.
    expect(notifier.state.pendingEdits.containsKey('x'), isTrue);
    expect(
      notifier.state.pendingEdits.containsKey('npc.revive:Lizard-WP_EF_001'),
      isTrue,
    );
    expect(core.requests.where((r) => r.command == 'write_save'), isEmpty);
  });

  test('setPendingNpcRevive is idempotent for the same NPC', () async {
    final core = _StructuralCoreService();
    final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
    await notifier.inspect(savePath);

    notifier.setPendingNpcRevive('Lizard-WP_EF_001');
    notifier.setPendingNpcRevive('Lizard-WP_EF_001');

    final pending = notifier.state.pendingEdits['npc.revive:Lizard-WP_EF_001'];
    expect(pending, isNotNull);
    expect(pending!.edits, hasLength(1));
  });

  // ---------------------------------------------------------------------------
  // Status row in the core group
  // ---------------------------------------------------------------------------

  testWidgets('Status row shows "dead" + Revive enabled for a dead NPC',
      (tester) async {
    var revived = 0;
    await tester.pumpWidget(
      panel(
        status: NpcStatusConfig(
          npcId: 'Lizard-2',
          editable: true,
          reloadKey: 'k',
          // Substring filter returns several rows; row must pick Lizard-2.
          load: () async => const NpcActorsPage(
            npcs: [
              NpcActor(id: 'Lizard-1', isDead: false),
              NpcActor(id: 'Lizard-2', isDead: true, hp: 0, maxHp: 50),
            ],
          ),
          onRevive: () => revived++,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The status line reads "dead", and the row label "Status" is present.
    expect(find.text('Status'), findsOneWidget);
    expect(find.text('dead'), findsOneWidget);

    final button = find.widgetWithText(FilledButton, 'Revive');
    expect(button, findsOneWidget);
    expect(tester.widget<FilledButton>(button).onPressed, isNotNull);

    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(revived, 1);
  });

  testWidgets('Status row shows "alive" + Revive disabled for a living NPC',
      (tester) async {
    await tester.pumpWidget(
      panel(
        status: NpcStatusConfig(
          npcId: 'Lizard-2',
          editable: true,
          reloadKey: 'k',
          load: () async => const NpcActorsPage(
            npcs: [
              // Alive even though merely defeated/knocked-out: isDead is false.
              NpcActor(id: 'Lizard-2', isDead: false, hp: 50, maxHp: 50),
            ],
          ),
          onRevive: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('alive'), findsOneWidget);
    expect(find.text('dead'), findsNothing);
    final button = find.widgetWithText(FilledButton, 'Revive');
    expect(tester.widget<FilledButton>(button).onPressed, isNull);
  });

  testWidgets('a pending revive reflects optimistically and keeps Revive on',
      (tester) async {
    // The loaded summary still reports the NPC as dead (the revive is unsaved),
    // but the pending flag flips the status line + keeps Revive enabled.
    await tester.pumpWidget(
      panel(
        status: NpcStatusConfig(
          npcId: 'Lizard-2',
          editable: true,
          reloadKey: 'k',
          revivePending: true,
          load: () async => const NpcActorsPage(
            npcs: [
              NpcActor(id: 'Lizard-2', isDead: true, hp: 0, maxHp: 50),
            ],
          ),
          onRevive: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The optimistic "will be revived" status line is shown.
    expect(find.text('Will be revived on save'), findsOneWidget);
    final button = find.widgetWithText(FilledButton, 'Revive');
    expect(tester.widget<FilledButton>(button).onPressed, isNotNull);
  });

  testWidgets('Revive is disabled when not editable', (tester) async {
    await tester.pumpWidget(
      panel(
        status: NpcStatusConfig(
          npcId: 'Lizard-2',
          editable: false,
          reloadKey: 'k',
          load: () async => const NpcActorsPage(
            npcs: [
              NpcActor(id: 'Lizard-2', isDead: true, hp: 0, maxHp: 50),
            ],
          ),
          onRevive: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    final button = find.widgetWithText(FilledButton, 'Revive');
    expect(tester.widget<FilledButton>(button).onPressed, isNull);
  });
}

/// A recording core that handles the scan/inspect/backups commands needed by
/// EditorNotifier. write_save is recorded but should NOT fire on a revive tap.
class _StructuralCoreService implements GoresaveCoreService {
  final requests = <_Rec>[];

  @override
  String get description => 'structural-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add(_Rec(command, Map<String, Object?>.from(payload)));
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': payload['path'],
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'fileName': 'G1R-001.sav',
                'slot': 'G1R-001',
              },
            ],
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
            'sha1': 'abc',
            'private': {
              'status': 'decoded',
              'player': {'playerName': 'Hero', 'writable': <String>[]},
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {'path': payload['path'], 'backups': [], 'companionBackups': []},
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
      case 'write_save':
        return {
          'ok': true,
          'data': {'backupPath': r'C:\tmp\saves\G1R-001.sav.bak.1'},
        };
      default:
        return {
          'ok': false,
          'error': {'message': 'Unhandled command $command'},
        };
    }
  }
}

class _Rec {
  const _Rec(this.command, this.payload);
  final String command;
  final Map<String, Object?> payload;
}
