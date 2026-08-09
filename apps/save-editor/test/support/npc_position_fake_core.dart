import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/providers/data_providers.dart';

import 'ui_settings_test_store.dart';

/// Shared scaffolding for the NPC position widget tests: a fake core that
/// answers everything the editor shell needs (one save slot, decoded + typed-OK
/// private payload, a character index) plus a per-NPC `private.npc.position`
/// response supplied by the test, and the pump/navigation helpers.
class RecordedRequest {
  const RecordedRequest(this.command, this.payload);

  final String command;
  final Map<String, Object?> payload;
}

/// One NPC's fake saved pose. A null member is reported as JSON `null` while
/// its typed path is still returned, exactly like the core does for an absent
/// leaf.
class FakePose {
  const FakePose({
    this.location = const (1.0, 2.0, 3.0),
    this.rotation = const (0.0, 0.0, 0.0),
    this.spawnLocation,
    this.spawnRotation,
    this.routineClass,
    this.undo,
  });

  final (double, double, double)? location;
  final (double, double, double)? rotation;
  final (double, double, double)? spawnLocation;
  final (double, double, double)? spawnRotation;

  /// The NPC's current daily-routine class. Null means the save has no routine
  /// record for him, which is how the core reports "nothing to pin" — the
  /// routine path is then withheld too.
  final String? routineClass;

  /// A recorded undo, as `private.npc.position` reports it.
  final FakeUndo? undo;
}

/// A recorded placement undo for the fake core.
class FakeUndo {
  const FakeUndo({
    this.originalLocation = const (7.0, 8.0, 9.0),
    this.originalRoutineClass = '/Script/Angelscript.DailyRoutine_A_Start',
    this.originalRotation,
    this.restorable = true,
    this.routineRestorable,
  });

  final (double, double, double) originalLocation;

  /// The facing the move replaced, when it changed one.
  final (double, double, double)? originalRotation;
  final String? originalRoutineClass;
  final bool restorable;

  /// Whether the ROUTINE alone can be put back. Defaults to [restorable]: a
  /// changed position is the usual reason the whole move cannot be undone while
  /// the routine still can, so a test that wants them to differ says so.
  final bool? routineRestorable;
}

/// The inert routine class the fake core advertises, matching the real one.
const String kFakeInertRoutine = '/Script/Angelscript.DailyRoutine_Empty';

/// The typed path `private.npc.position` reports for the routine class leaf.
List<String> routinePath(String npcId) => [
  'm_GenericData',
  '{CharacterStates}',
  'AnyCharacterType',
  'DailyRoutineByGlobalId',
  '{$npcId}',
  'DailyRoutineClass',
];

/// The typed path shape `private.npc.position` reports for one pose member.
List<String> posePath(String npcId, String leaf) => [
  'm_GenericData',
  '{CharacterStates}',
  'AnyCharacterType',
  'PositionByGlobalId',
  '{$npcId}',
  leaf,
];

/// Fake core driving the position tests. [poses] maps GlobalId → saved pose;
/// its keys also become the character index (the master list rows the tests
/// tap) and the NPC list.
class NpcPositionCoreService implements GoresaveCoreService {
  NpcPositionCoreService(this.poses, {this.playerTransform});

  final Map<String, FakePose> poses;

  /// Optional saved player pose (`location`, `rotation`). When supplied,
  /// `inspect_save` reports a private player transform AND declares
  /// `private.player.setTransform` writable, which is what makes the Position
  /// tab render the editable player transform editor for the pinned Player row.
  /// Null (the default) keeps the fixture exactly as the NPC-only tests see it.
  final ((double, double, double), (double, double, double))? playerTransform;

  final requests = <RecordedRequest>[];

  List<String> get _ids => poses.keys.toList(growable: false);

  @override
  String get description => 'npc-position-fake-core';

  @override
  bool get isAvailable => true;

  Map<String, Object?>? _vec3((double, double, double)? triplet) =>
      triplet == null ? null : {'x': triplet.$1, 'y': triplet.$2, 'z': triplet.$3};

  Map<String, Object?>? _rot3((double, double, double)? triplet) => triplet == null
      ? null
      : {'pitch': triplet.$1, 'yaw': triplet.$2, 'roll': triplet.$3};

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    requests.add(RecordedRequest(command, Map<String, Object?>.from(payload)));
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
                if (playerTransform != null)
                  'transform': {
                    'location': _vec3(playerTransform!.$1),
                    'rotation': _rot3(playerTransform!.$2),
                  },
                'writable': [
                  'private.typed.setValue',
                  if (playerTransform != null) 'private.player.setTransform',
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
        // Backs the Charaktere master list. The globalId is rendered as the row
        // subtitle, so tests select a row by tapping its GlobalId text.
        return {
          'ok': true,
          'data': {
            'total': _ids.length,
            'characters': [
              for (final id in _ids)
                {
                  'globalId': id,
                  'uniqueName': id.split('-').first,
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
            'total': _ids.length,
            'offset': 0,
            'limit': payload['limit'] ?? 100,
            'count': _ids.length,
            'npcs': [
              for (final id in _ids) {'id': id, 'name': id},
            ],
          },
        };
      case 'private.npc.position':
        final id = payload['id'] as String;
        final pose = poses[id];
        if (pose == null) {
          return {
            'ok': false,
            'error': {'message': 'NPC "$id" not found in _Position map'},
          };
        }
        return {
          'ok': true,
          'data': {
            'pose': {
              'location': _vec3(pose.location),
              'rotation': _rot3(pose.rotation),
              'spawnLocation': _vec3(pose.spawnLocation),
              'spawnRotation': _rot3(pose.spawnRotation),
              'locationPath': posePath(id, 'CharacterLocation'),
              'rotationPath': posePath(id, 'CharacterRotation'),
              'spawnLocationPath': posePath(id, 'SpawnLocation'),
              'spawnRotationPath': posePath(id, 'SpawnRotation'),
            },
            'routineClass': pose.routineClass,
            if (pose.routineClass != null) 'routineClassPath': routinePath(id),
            'inertRoutineClass': kFakeInertRoutine,
            if (pose.undo != null)
              'undo': {
                'originalLocation': _vec3(pose.undo!.originalLocation),
                if (pose.undo!.originalRotation != null)
                  'originalRotation': _rot3(pose.undo!.originalRotation),
                'originalRoutineClass': pose.undo!.originalRoutineClass,
                'restorable': pose.undo!.restorable,
                'routineRestorable':
                    pose.undo!.routineRestorable ?? pose.undo!.restorable,
              },
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
          'error': {'message': 'Unhandled fake command $command'},
        };
    }
  }
}

/// Mount the whole app against [core] at a desktop-sized surface.
Future<void> pumpPositionApp(
  WidgetTester tester,
  GoresaveCoreService core,
) async {
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

/// The bundled location catalog, read the way the picker reads it.
///
/// Must go through [WidgetTester.runAsync]: `testWidgets` runs its body inside
/// FakeAsync, where the real file I/O behind `rootBundle` never completes and a
/// plain `await` would hang forever. Tests use the returned catalog as the
/// source of truth for expected coordinates rather than hard-coding numbers the
/// asset owns.
Future<LocationCatalog> loadBundledCatalog(WidgetTester tester) async {
  late final LocationCatalog catalog;
  await tester.runAsync(() async {
    catalog = await LocationCatalog.loadBundled();
  });
  return catalog;
}

/// Open the shared location picker from the button above the position fields
/// and choose the spot named [spotName], optionally ticking the opt-in
/// "apply the spot's orientation" box first.
Future<void> pickLocationSpot(
  WidgetTester tester,
  String spotName, {
  bool applyRotation = false,
}) async {
  await tester.tap(find.widgetWithText(OutlinedButton, 'Choose location…'));
  // The dialog reads the bundled asset from disk. That real I/O cannot land
  // inside FakeAsync, so pumpAndSettle alone would spin on the spinner until it
  // times out; runAsync gives the load a window of REAL time to finish.
  await tester.pump();
  await tester.runAsync(
    () => Future<void>.delayed(const Duration(milliseconds: 300)),
  );
  await tester.pumpAndSettle();
  if (applyRotation) {
    await tester.tap(find.text("Also apply the spot's orientation"));
    await tester.pumpAndSettle();
  }
  await tester.enterText(
    find.widgetWithText(TextField, 'Search entries'),
    spotName,
  );
  // Let the search debounce elapse.
  await tester.pump(const Duration(milliseconds: 250));
  await tester.pumpAndSettle();
  // Scoped to the row: the query itself is also on screen, inside the search
  // field, and a bare find.text would match both.
  await tester.tap(find.widgetWithText(ListTile, spotName));
  await tester.pumpAndSettle();
}

/// Charaktere → Position (the fifth sub-tab).
Future<void> openPositionTab(WidgetTester tester) async {
  await tester.tap(find.widgetWithText(Tab, 'Characters'));
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(Tab, 'Position'));
  await tester.pumpAndSettle();
}

/// Finder for one of the six editable position fields, e.g. `location:x`.
Finder positionField(String id) => find.byKey(ValueKey('npc-position:$id'));

/// The live text of one position field.
String positionFieldText(WidgetTester tester, String id) => tester
    .widget<EditableText>(
      find.descendant(of: positionField(id), matching: find.byType(EditableText)),
    )
    .controller
    .text;

/// The live editor state (for asserting on pending-registry KEYS).
ProviderContainer positionContainer(WidgetTester tester) =>
    ProviderScope.containerOf(tester.element(find.byType(GoresaveApp)));
