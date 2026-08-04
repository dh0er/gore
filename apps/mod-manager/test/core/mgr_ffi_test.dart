import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/models.dart';

/// mgr_library_list fixture: two mods that together cover every component
/// type of the contract (plus one unknown type), the raw_file target_file
/// externally-tagged variants, and a loadout with mixed enabled flags.
Map<String, Object?> _libraryListResponse() => {
      'ok': true,
      'mods': [
        {
          'id': 'mod-a',
          'kind': 'goremod',
          'name': 'Better Torches',
          'version': '1.2.0',
          'author': 'dh',
          'imported_at': '2026-07-01T12:00:00Z',
          'source': 'BetterTorches.goremod',
          // `targets` are conflict-analysis footprint keys, not file paths:
          // ue4ss dir name, loc "id|set", audio "bank|sample", asset paths,
          // AngelScript module names.
          'components': [
            {
              'type': 'ue4ss_lua',
              'name': 'torches',
              'rel': 'scripts/torches',
              'targets': ['torches'],
              'opaque': true,
            },
            {
              'type': 'loc_patch',
              'rel': 'loc/patch.json',
              'targets': ['itlstorch|german'],
            },
            {
              'type': 'audio_patch',
              'rel': 'audio/sfx_patch.json',
              'targets': ['SFX|vob_fire_torch'],
            },
            {
              'type': 'texture_patch',
              'rel': 'textures/torches',
              'targets': ['/Game/Gothic/Textures/T_Torch_D'],
            },
            {
              'type': 'angel_script_patch',
              'rel': 'scripts/patch.as',
              'targets': ['Game/Items/ItLsTorch'],
            },
          ],
        },
        {
          'id': 'mod-b',
          'kind': 'foreign_mixed',
          'name': 'Foreign Pack',
          'components': [
            {
              'type': 'triplet',
              'rel_base': 'triplet/pack',
              'targets': [
                '/Game/Gothic/Textures/T_Pack_A',
                '/Game/Gothic/Textures/T_Pack_B',
                '/Game/Gothic/Meshes/SM_Pack',
              ],
            },
            {
              'type': 'loose_pak',
              'rel': 'paks/loose_P.pak',
              'targets': ['/Game/Gothic/Textures/T_Loose'],
            },
            {
              'type': 'raw_file',
              'rel': 'raw/SFX.bank',
              'target_file': {
                'bank': {'name': 'SFX'},
              },
            },
            {
              'type': 'raw_file',
              'rel': 'raw/Game.lcache',
              'target_file': 'lcache',
            },
            {
              'type': 'raw_file',
              'rel': 'raw/cache.bin',
              'target_file': 'script_cache',
            },
            {
              // Unknown/future component type: must parse as a generic view.
              'type': 'hologram',
              'rel': 'future/thing',
            },
          ],
        },
      ],
      'loadout': {
        'format': 1,
        'entries': [
          {'id': 'mod-a', 'enabled': true},
          {'id': 'mod-b', 'enabled': false},
        ],
      },
    };

void main() {
  group('canonical native response decoding', () {
    test('retains one compact exact object', () {
      expect(decodeCanonicalGoreCoreResponse('{"ok":true,"value":"x"}'), {
        'ok': true,
        'value': 'x',
      });
    });

    test('rejects duplicates and normalized JSON spellings', () {
      for (final response in [
        '{"ok":true,"ok":false}',
        ' {"ok":true}',
        '{"ok" : true}',
        '{"ok":true,"value":"\\u0078"}',
        '[]',
        '{',
      ]) {
        expect(
          () => decodeCanonicalGoreCoreResponse(response),
          throwsFormatException,
        );
      }
    });
  });

  group('MgrFfi.libraryList', () {
    test('parses mods, every component type, and the loadout', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {'mgr_library_list': _libraryListResponse()},
      );
      final (mods, loadout) = await MgrFfi(fake).libraryList();

      expect(mods, hasLength(2));

      final a = mods[0];
      expect(a.id, 'mod-a');
      expect(a.kind, 'goremod');
      expect(a.name, 'Better Torches');
      expect(a.version, '1.2.0');
      expect(a.author, 'dh');
      expect(a.importedAt, '2026-07-01T12:00:00Z');
      expect(a.source, 'BetterTorches.goremod');
      expect(
        a.components.map((c) => c.kind),
        [
          'ue4ss_lua',
          'loc_patch',
          'audio_patch',
          'texture_patch',
          'angel_script_patch',
        ],
      );
      final lua = a.components[0];
      expect(lua.name, 'torches');
      expect(lua.rel, 'scripts/torches');
      expect(lua.targets, ['torches']);
      expect(lua.opaque, isTrue);
      expect(lua.displayLabel, 'torches');
      final loc = a.components[1];
      expect(loc.rel, 'loc/patch.json');
      expect(loc.targets, ['itlstorch|german']);
      expect(loc.opaque, isFalse);
      expect(loc.displayLabel, 'loc/patch.json');

      final b = mods[1];
      expect(b.kind, 'foreign_mixed');
      expect(b.version, isNull);
      expect(b.author, isNull);
      expect(b.components, hasLength(6));
      final triplet = b.components[0];
      expect(triplet.kind, 'triplet');
      expect(triplet.rel, 'triplet/pack'); // rel_base surfaces as rel
      expect(triplet.targets, hasLength(3));
      final loosePak = b.components[1];
      expect(loosePak.kind, 'loose_pak');
      expect(loosePak.rel, 'paks/loose_P.pak');
      final rawBank = b.components[2];
      expect(rawBank.kind, 'raw_file');
      expect(rawBank.rawFileTarget?.kind, 'bank');
      expect(rawBank.rawFileTarget?.bankName, 'SFX');
      final rawLcache = b.components[3];
      expect(rawLcache.rawFileTarget?.kind, 'lcache');
      expect(rawLcache.rawFileTarget?.bankName, isNull);
      final rawScriptCache = b.components[4];
      expect(rawScriptCache.rawFileTarget?.kind, 'script_cache');
      // Unknown component type: tolerated, raw kind preserved, no throw.
      final unknown = b.components[5];
      expect(unknown.kind, 'hologram');
      expect(unknown.rel, 'future/thing');
      expect(unknown.rawFileTarget, isNull);
      expect(unknown.raw['type'], 'hologram');

      expect(loadout.format, 1);
      expect(loadout.entries, hasLength(2));
      expect(loadout.entries[0].id, 'mod-a');
      expect(loadout.entries[0].enabled, isTrue);
      expect(loadout.entries[1].id, 'mod-b');
      expect(loadout.entries[1].enabled, isFalse);
    });
  });

  group('MgrFfi.status', () {
    Future<ManagerStatusView> parse(Map<String, Object?> status) {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_status': {'ok': true, 'status': status},
        },
      );
      return MgrFfi(fake).status('C:/game');
    }

    test('parses nothing_deployed', () async {
      final s = await parse({'state': 'nothing_deployed'});
      expect(s, isA<ManagerStatusNothingDeployed>());
      expect(s.state, 'nothing_deployed');
    });

    test('parses studio_deploy_active with mod_name', () async {
      final s = await parse({
        'state': 'studio_deploy_active',
        'mod_name': 'MyStudioMod',
      });
      expect(s, isA<ManagerStatusStudioDeployActive>());
      expect((s as ManagerStatusStudioDeployActive).modName, 'MyStudioMod');
    });

    test('parses in_sync with loadout (wire shape: entry ARRAY)', () async {
      final s = await parse({
        'state': 'in_sync',
        'loadout': [
          {'id': 'mod-a', 'enabled': true},
        ],
      });
      expect(s, isA<ManagerStatusInSync>());
      final inSync = s as ManagerStatusInSync;
      expect(inSync.loadout?.entries.single.id, 'mod-a');
      expect(inSync.loadout?.entries.single.enabled, isTrue);
    });

    test('in_sync also tolerates the {format, entries} map shape', () async {
      final s = await parse({
        'state': 'in_sync',
        'loadout': {
          'format': 1,
          'entries': [
            {'id': 'mod-a', 'enabled': true},
          ],
        },
      });
      expect((s as ManagerStatusInSync).loadout?.entries.single.id, 'mod-a');
    });

    test('parses changes_pending with deployed/target entry ARRAYS', () async {
      final s = await parse({
        'state': 'changes_pending',
        'deployed': [
          {'id': 'mod-a', 'enabled': true},
        ],
        'target': [
          {'id': 'mod-a', 'enabled': true},
          {'id': 'mod-b', 'enabled': true},
        ],
      });
      expect(s, isA<ManagerStatusChangesPending>());
      final pending = s as ManagerStatusChangesPending;
      expect(pending.deployed?.entries, hasLength(1));
      expect(pending.target?.entries, hasLength(2));
      expect(pending.target?.entries[1].id, 'mod-b');
    });

    test('parses game_updated with drifted files', () async {
      final s = await parse({
        'state': 'game_updated',
        'drifted': ['G1R/Content/FMOD/Desktop/SFX.bank'],
      });
      expect(s, isA<ManagerStatusGameUpdated>());
      expect(
        (s as ManagerStatusGameUpdated).drifted,
        ['G1R/Content/FMOD/Desktop/SFX.bank'],
      );
    });

    test('parses a future state as ManagerStatusUnknown, keeping raw', () async {
      final s = await parse({'state': 'reticulating_splines', 'foo': 1});
      expect(s, isA<ManagerStatusUnknown>());
      expect(s.state, 'reticulating_splines');
      expect(s.raw['foo'], 1);
    });
  });

  group('MgrFfi errors', () {
    test('error envelope throws MgrFfiException with command + code', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': false,
            'error': {
              'code': 'STUDIO_DEPLOY_ACTIVE',
              'message': 'undeploy the studio mod first',
            },
          },
        },
      );
      expect(
        () => MgrFfi(fake).apply('C:/game'),
        throwsA(
          isA<MgrFfiException>()
              .having(
                (e) => e.message,
                'message',
                allOf(
                  contains('mgr_apply'),
                  contains('undeploy the studio mod first'),
                ),
              )
              // The UI branches on this code.
              .having((e) => e.code, 'code', 'STUDIO_DEPLOY_ACTIVE'),
        ),
      );
    });

    test('non-map error value is stringified; code falls back', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_remove': {'ok': false, 'error': 'disk on fire'},
        },
      );
      expect(
        () => MgrFfi(fake).remove('mod-a'),
        throwsA(
          isA<MgrFfiException>()
              .having(
                (e) => e.message,
                'message',
                allOf(contains('mgr_remove'), contains('disk on fire')),
              )
              .having((e) => e.code, 'code', 'UNKNOWN'),
        ),
      );
    });

    test('unknown command (fake default, no code) also throws', () async {
      final fake = FakeGoreCoreFfiService(responses: {});
      expect(
        () => MgrFfi(fake).analyze(),
        throwsA(
          isA<MgrFfiException>()
              .having((e) => e.message, 'message', contains('mgr_analyze'))
              .having((e) => e.code, 'code', 'UNKNOWN'),
        ),
      );
    });
  });

  group('MgrFfi.setLoadout', () {
    test('sends the exact loadout payload', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_set_loadout': {'ok': true},
        },
      );
      const loadout = LoadoutView(
        format: 1,
        entries: [
          LoadoutEntryView(id: 'mod-b', enabled: true),
          LoadoutEntryView(id: 'mod-a', enabled: false),
        ],
      );
      await MgrFfi(fake).setLoadout(loadout);

      expect(fake.calls, hasLength(1));
      expect(fake.calls.single.command, 'mgr_set_loadout');
      expect(fake.calls.single.payload, {
        'loadout': {
          'format': 1,
          'entries': [
            {'id': 'mod-b', 'enabled': true},
            {'id': 'mod-a', 'enabled': false},
          ],
        },
      });
    });
  });

  group('MgrFfi remaining commands', () {
    test('import returns the new entry', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_import': {
            'ok': true,
            'entry': {
              'id': 'mod-c',
              'kind': 'foreign_pak',
              'name': 'loose_P',
              'components': [
                {
                  'type': 'loose_pak',
                  'rel': 'paks/loose_P.pak',
                  'targets': ['G1R/Content/Paks/~mods/loose_P.pak'],
                },
              ],
            },
          },
        },
      );
      final entry = await MgrFfi(fake).import('D:/downloads/loose_P.pak');
      expect(entry.id, 'mod-c');
      expect(entry.kind, 'foreign_pak');
      expect(fake.calls.single.payload, {'path': 'D:/downloads/loose_P.pak'});
    });

    test('remove/undeployAll report whether anything happened', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_remove': {'ok': true, 'removed': true},
          'mgr_undeploy_all': {'ok': true, 'removed': 0},
        },
      );
      final mgr = MgrFfi(fake);
      expect(await mgr.remove('mod-a'), isTrue);
      expect(await mgr.undeployAll('C:/game'), isFalse);
      expect(fake.calls[0].payload, {'id': 'mod-a'});
      expect(fake.calls[1].payload, {'game_root': 'C:/game'});
    });

    test('analyze parses conflicts', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_analyze': {
            'ok': true,
            // kind ∈ loc|audio|asset|cdo|script_module|raw_file|
            // ue4ss_dir_name; severity ∈ soft|hard|info; target is the
            // footprint key ("bank|sample" here).
            'conflicts': [
              {
                'kind': 'audio',
                'target': 'SFX|vob_fire_torch',
                'mods': ['mod-a', 'mod-b'],
                'severity': 'soft',
              },
            ],
          },
        },
      );
      final conflicts = await MgrFfi(fake).analyze();
      expect(conflicts, hasLength(1));
      expect(conflicts.single.kind, 'audio');
      expect(conflicts.single.target, 'SFX|vob_fire_torch');
      expect(conflicts.single.modIds, ['mod-a', 'mod-b']);
      expect(conflicts.single.severity, 'soft');
    });

    test('apply parses the report', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': true,
            'report': {
              'applied': ['Better Torches', 'Foreign Pack'],
              'warnings': ['SFX.bank written by 2 mods; last wins'],
            },
          },
        },
      );
      final report = await MgrFfi(fake).apply('C:/game');
      expect(report.applied, ['Better Torches', 'Foreign Pack']);
      expect(report.warnings, hasLength(1));
    });
  });
}
