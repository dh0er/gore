import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/preflight/domain/models.dart';

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
          'coverage': 'partial',
        },
        {
          'type': 'loc_patch',
          'rel': 'loc/patch.json',
          'targets': ['itlstorch|german'],
          'coverage': 'exact',
        },
        {
          'type': 'audio_patch',
          'rel': 'audio/sfx_patch.json',
          'targets': ['SFX|vob_fire_torch'],
          'coverage': 'exact',
        },
        {
          'type': 'texture_patch',
          'rel': 'textures/torches',
          'targets': ['/Game/Gothic/Textures/T_Torch_D'],
          'coverage': 'exact',
        },
        {
          'type': 'angel_script_patch',
          'rel': 'scripts/patch.as',
          'targets': ['Game/Items/ItLsTorch'],
          'coverage': 'exact',
        },
        {
          'type': 'file_patch',
          'rel': 'files',
          'targets': ['G1R/Content/Movies/Intro.bk2'],
          'coverage': 'exact',
        },
        {
          'type': 'pak_file_patch',
          'rel': 'pak_files',
          'targets': ['G1R/Content/Slate/Cursors/Normal/Normal.PNG'],
          'coverage': 'exact',
        },
        {
          'type': 'voice_archive_patch',
          'rel': 'voice',
          'targets': ['German.zip|NPC/Hero/hello.ogg'],
          'coverage': 'exact',
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
          'coverage': 'advisory',
        },
        {
          'type': 'loose_pak',
          'rel': 'paks/loose_P.pak',
          'targets': ['/Game/Gothic/Textures/T_Loose'],
          'coverage': 'exact',
        },
        {
          'type': 'raw_file',
          'rel': 'raw/SFX.bank',
          'target_file': {
            'bank': {'name': 'SFX'},
          },
          'coverage': 'exact',
        },
        {
          'type': 'raw_file',
          'rel': 'raw/Game.lcache',
          'target_file': 'lcache',
          'coverage': 'exact',
        },
        {
          'type': 'raw_file',
          'rel': 'raw/cache.bin',
          'target_file': 'script_cache',
          'coverage': 'exact',
        },
        {
          // Unknown/future component type: must parse as a generic view.
          'type': 'hologram',
          'rel': 'future/thing',
          'coverage': 'future_precision',
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

Map<String, Object?> _ownedGroup(
  List<Object?> items, {
  int? total,
  bool? truncated,
}) => {
  'items': items,
  'total': total ?? items.length,
  'truncated': truncated ?? false,
};

Map<String, Object?> _managerOwnedWire() => {
  'live': _ownedGroup(['C:/game/G1R/Story/VoiceOver/a.zip']),
  'backups': _ownedGroup(['C:/game/G1R/Story/VoiceOver/a.zip.gore-bak']),
  'additive': _ownedGroup(['C:/game/G1R/Content/Paks/~mods/a_P.pak']),
  'ue4ss': _ownedGroup(['C:/game/G1R/Binaries/Win64/ue4ss/Mods/A']),
  'recovery': _ownedGroup(
    ['C:/game/gore-mod.deployed.json'],
    total: 2,
    truncated: true,
  ),
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
      expect(a.components.map((c) => c.kind), [
        'ue4ss_lua',
        'loc_patch',
        'audio_patch',
        'texture_patch',
        'angel_script_patch',
        'file_patch',
        'pak_file_patch',
        'voice_archive_patch',
      ]);
      final lua = a.components[0];
      expect(lua.name, 'torches');
      expect(lua.rel, 'scripts/torches');
      expect(lua.targets, ['torches']);
      expect(lua.opaque, isTrue);
      expect(lua.coverage, FootprintCoverage.partial);
      expect(lua.displayLabel, 'torches');
      final loc = a.components[1];
      expect(loc.rel, 'loc/patch.json');
      expect(loc.targets, ['itlstorch|german']);
      expect(loc.opaque, isFalse);
      expect(loc.coverage, FootprintCoverage.exact);
      expect(loc.displayLabel, 'loc/patch.json');
      final filePatch = a.components[5];
      expect(filePatch.rel, 'files');
      expect(filePatch.targets, ['G1R/Content/Movies/Intro.bk2']);
      expect(filePatch.coverage, FootprintCoverage.exact);
      expect(filePatch.displayLabel, 'files');
      final pakFilePatch = a.components[6];
      expect(pakFilePatch.rel, 'pak_files');
      expect(pakFilePatch.targets, [
        'G1R/Content/Slate/Cursors/Normal/Normal.PNG',
      ]);
      expect(pakFilePatch.coverage, FootprintCoverage.exact);
      expect(pakFilePatch.displayLabel, 'pak_files');
      final voiceArchivePatch = a.components[7];
      expect(voiceArchivePatch.rel, 'voice');
      expect(voiceArchivePatch.targets, ['German.zip|NPC/Hero/hello.ogg']);
      expect(voiceArchivePatch.coverage, FootprintCoverage.exact);
      expect(voiceArchivePatch.displayLabel, 'voice');

      final b = mods[1];
      expect(b.kind, 'foreign_mixed');
      expect(b.version, isNull);
      expect(b.author, isNull);
      expect(b.components, hasLength(6));
      final triplet = b.components[0];
      expect(triplet.kind, 'triplet');
      expect(triplet.rel, 'triplet/pack'); // rel_base surfaces as rel
      expect(triplet.targets, hasLength(3));
      expect(triplet.coverage, FootprintCoverage.advisory);
      final loosePak = b.components[1];
      expect(loosePak.kind, 'loose_pak');
      expect(loosePak.rel, 'paks/loose_P.pak');
      expect(loosePak.coverage, FootprintCoverage.exact);
      final rawBank = b.components[2];
      expect(rawBank.kind, 'raw_file');
      expect(rawBank.rawFileTarget?.kind, 'bank');
      expect(rawBank.rawFileTarget?.bankName, 'SFX');
      expect(rawBank.coverage, FootprintCoverage.exact);
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
      expect(unknown.coverage, FootprintCoverage.opaque);

      expect(loadout.format, 1);
      expect(loadout.entries, hasLength(2));
      expect(loadout.entries[0].id, 'mod-a');
      expect(loadout.entries[0].enabled, isTrue);
      expect(loadout.entries[1].id, 'mod-b');
      expect(loadout.entries[1].enabled, isFalse);
    });

    test('malformed native store snapshots fail closed', () async {
      final malformed = <Map<String, Object?>>[
        {
          'ok': true,
          'mods': 7,
          'loadout': const {'format': 1, 'entries': []},
        },
        {'ok': true, 'mods': const [], 'loadout': null},
        {
          'ok': true,
          'mods': const [42],
          'loadout': const {'format': 1, 'entries': []},
        },
        {
          'ok': true,
          'mods': const [],
          'loadout': const {'format': 1.0, 'entries': []},
        },
        {
          'ok': true,
          'mods': const [],
          'loadout': const {
            'format': 1,
            'entries': [
              {'id': 'mod-a', 'enabled': 'yes'},
            ],
          },
        },
      ];
      for (final response in malformed) {
        final fake = FakeGoreCoreFfiService(
          responses: {'mgr_library_list': response},
        );
        await expectLater(
          MgrFfi(fake).libraryList(),
          throwsA(isA<MgrFfiException>()),
        );
      }
    });

    test(
      'older DLL coverage is inferred conservatively from component facts',
      () {
        FootprintCoverage coverage(Map<String, Object?> json) =>
            ComponentView.fromJson(json).coverage;

        expect(
          coverage({'type': 'loc_patch', 'targets': <String>[]}),
          FootprintCoverage.exact,
        );
        expect(
          coverage({'type': 'raw_file', 'target_file': 'lcache'}),
          FootprintCoverage.exact,
        );
        expect(
          coverage({
            'type': 'ue4ss_lua',
            'targets': ['Class.Field'],
            'opaque': true,
          }),
          FootprintCoverage.partial,
        );
        expect(
          coverage({'type': 'ue4ss_lua', 'opaque': true}),
          FootprintCoverage.opaque,
        );
        expect(
          coverage({
            'type': 'triplet',
            'targets': ['/Game/Observed'],
          }),
          FootprintCoverage.advisory,
        );
        expect(coverage({'type': 'triplet'}), FootprintCoverage.opaque);
        expect(
          coverage({
            'type': 'loose_pak',
            'targets': ['/Game/Indexed'],
          }),
          FootprintCoverage.exact,
        );
        expect(coverage({'type': 'loose_pak'}), FootprintCoverage.opaque);
        expect(
          coverage({'type': 'future_component'}),
          FootprintCoverage.opaque,
        );
        expect(
          coverage({'type': 'loc_patch', 'coverage': 'future_precision'}),
          FootprintCoverage.opaque,
          reason:
              'a present unknown grade must fail closed instead of inferring',
        );
      },
    );
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

    test('parses recovery_required as a known recovery state', () async {
      final s = await parse({'state': 'recovery_required'});
      expect(s, isA<ManagerStatusRecoveryRequired>());
      expect(s.state, 'recovery_required');
      expect(s.managerOwned, isNull);
    });

    test('parses all five bounded manager-owned groups additively', () async {
      final s = await parse({
        'state': 'recovery_required',
        'manager_owned': _managerOwnedWire(),
      });
      expect(s, isA<ManagerStatusRecoveryRequired>());
      final owned = s.managerOwned!;
      expect(owned.live.items, ['C:/game/G1R/Story/VoiceOver/a.zip']);
      expect(owned.backups.total, 1);
      expect(owned.additive.truncated, isFalse);
      expect(owned.ue4ss.items, hasLength(1));
      expect(owned.recovery.items, ['C:/game/gore-mod.deployed.json']);
      expect(owned.recovery.total, 2);
      expect(owned.recovery.truncated, isTrue);
    });

    test(
      'malformed ownership detail is hidden without losing base status',
      () async {
        final malformed = <Object?>[
          'not-an-object',
          {..._managerOwnedWire()}..remove('live'),
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup([7]),
          },
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup(List<Object?>.filled(129, 'C:/x')),
          },
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup([List.filled(4097, 'x').join()]),
          },
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup([
              for (var i = 0; i < 17; i++) '$i${List.filled(4094, 'x').join()}',
            ]),
          },
          {..._managerOwnedWire(), 'live': _ownedGroup([], total: -1)},
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup(['C:/x'], total: 0),
          },
          {
            ..._managerOwnedWire(),
            'live': _ownedGroup([], total: 1, truncated: false),
          },
        ];

        for (final managerOwned in malformed) {
          final s = await parse({
            'state': 'in_sync',
            'loadout': <Object?>[],
            'manager_owned': managerOwned,
          });
          expect(s, isA<ManagerStatusInSync>());
          expect(s.managerOwned, isNull, reason: '$managerOwned');
        }
      },
    );

    test('extra ownership fields remain forward compatible', () async {
      final s = await parse({
        'state': 'in_sync',
        'loadout': <Object?>[],
        'manager_owned': {
          ..._managerOwnedWire(),
          'future_group': _ownedGroup([]),
          'live': {..._ownedGroup([]), 'future_fact': 7},
        },
      });
      expect(s.managerOwned, isNotNull);
      expect(s.managerOwned!.live.total, 0);
    });

    test(
      'Nothing, Studio, and future states never adopt ownership detail',
      () async {
        for (final state in [
          'nothing_deployed',
          'studio_deploy_active',
          'future_state',
        ]) {
          final s = await parse({
            'state': state,
            'manager_owned': _managerOwnedWire(),
          });
          expect(s.managerOwned, isNull, reason: state);
        }
      },
    );

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
      expect((s as ManagerStatusGameUpdated).drifted, [
        'G1R/Content/FMOD/Desktop/SFX.bank',
      ]);
    });

    test(
      'parses a future state as ManagerStatusUnknown, keeping raw',
      () async {
        final s = await parse({'state': 'reticulating_splines', 'foo': 1});
        expect(s, isA<ManagerStatusUnknown>());
        expect(s.state, 'reticulating_splines');
        expect(s.raw['foo'], 1);
      },
    );
  });

  group('MgrFfi.preflight', () {
    Map<String, Object?> response({
      String futureState = 'ok',
      String futureAction = 'none',
    }) {
      const ids = [
        'game_root',
        'install',
        'loadout',
        'deployment',
        'install_mutation',
        'ue4ss',
        'write_access',
      ];
      return {
        'ok': true,
        'preflight': {
          'format': 1,
          'checks': [
            for (var index = 0; index < ids.length; index++)
              {
                'id': ids[index],
                'state': index == 1 ? futureState : 'ok',
                'code': index == 1 ? 'future_code' : 'ready',
                'action': index == 1 ? futureAction : 'none',
                'detail': 'evidence $index',
                'items': <String>['item $index'],
                'future_field': true,
              },
          ],
          'future_top_level': true,
        },
      };
    }

    test('sends only the explicit root and parses the fixed report', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {'mgr_preflight_v1': response()},
      );

      final report = await MgrFfi(fake).preflight(r'C:\game');

      expect(fake.calls.single.command, 'mgr_preflight_v1');
      expect(fake.calls.single.payload, {'game_root': r'C:\game'});
      expect(report.checks, hasLength(7));
      expect(report.check(PreflightCheckId.install).detail, 'evidence 1');
      expect(report.primarySetupFinding, isNull);
    });

    test('an unconfigured fake does not fabricate a healthy report', () async {
      final fake = FakeGoreCoreFfiService(responses: const {});

      await expectLater(
        MgrFfi(fake).preflight('C:/game'),
        throwsA(
          isA<MgrFfiException>()
              .having(
                (error) => error.message,
                'message',
                contains('mgr_preflight_v1'),
              )
              .having((error) => error.code, 'code', 'UNKNOWN'),
        ),
      );
    });

    test(
      'future vocabulary is preserved and has no executable action',
      () async {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_preflight_v1': response(
              futureState: 'future_attention',
              futureAction: 'format_drive',
            ),
          },
        );

        final report = await MgrFfi(fake).preflight('C:/game');
        final finding = report.primarySetupFinding!;
        expect(finding.rawState, 'future_attention');
        expect(finding.state, isNull);
        expect(finding.rawAction, 'format_drive');
        expect(finding.action, isNull);
      },
    );

    test('recover_install parses as the known manual recovery route', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_preflight_v1': response(
            futureState: 'problem',
            futureAction: 'recover_install',
          ),
        },
      );

      final report = await MgrFfi(fake).preflight('C:/game');

      expect(
        report.primarySetupFinding?.action,
        PreflightActionKind.recoverInstall,
      );
    });

    test('malformed format, order, or field types fail closed', () async {
      final malformed = <Map<String, Object?>>[];
      final wrongFormat = response();
      (wrongFormat['preflight'] as Map<String, Object?>)['format'] = 2;
      malformed.add(wrongFormat);

      final wrongOrder = response();
      final checks =
          (wrongOrder['preflight'] as Map<String, Object?>)['checks'] as List;
      (checks[0] as Map<String, Object?>)['id'] = 'install';
      malformed.add(wrongOrder);

      final wrongItems = response();
      final typedChecks =
          (wrongItems['preflight'] as Map<String, Object?>)['checks'] as List;
      (typedChecks[0] as Map<String, Object?>)['items'] = [1];
      malformed.add(wrongItems);

      for (final wire in malformed) {
        final fake = FakeGoreCoreFfiService(
          responses: {'mgr_preflight_v1': wire},
        );
        await expectLater(
          MgrFfi(fake).preflight('C:/game'),
          throwsA(
            isA<MgrFfiException>().having(
              (error) => error.code,
              'code',
              'MGR_PREFLIGHT_INVALID_RESPONSE',
            ),
          ),
        );
      }
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
    Map<String, Object?> importSuccess({
      required String disposition,
      required String matchedBy,
    }) => {
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
            'coverage': 'exact',
          },
        ],
      },
      'disposition': disposition,
      'matched_by': matchedBy,
    };

    test('import parses all three dispositions', () async {
      for (final (wire, match, expected) in [
        ('created', 'none', MgrImportDisposition.created),
        ('updated', 'source', MgrImportDisposition.updated),
        ('unchanged', 'content', MgrImportDisposition.unchanged),
      ]) {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_import': importSuccess(disposition: wire, matchedBy: match),
          },
        );
        final outcome = await MgrFfi(fake).import('D:/downloads/loose_P.pak');
        expect(outcome.disposition, expected);
        expect(outcome.entry.id, 'mod-c');
        expect(outcome.entry.kind, 'foreign_pak');
        expect(
          outcome.entry.components.single.coverage,
          FootprintCoverage.exact,
        );
      }
    });

    test('import parses all four verified match methods', () async {
      for (final (wire, disposition, expected) in [
        ('none', 'created', MgrImportMatchedBy.none),
        ('source', 'updated', MgrImportMatchedBy.source),
        ('content', 'updated', MgrImportMatchedBy.content),
        ('entry_id', 'updated', MgrImportMatchedBy.entryId),
      ]) {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_import': importSuccess(
              disposition: disposition,
              matchedBy: wire,
            ),
          },
        );
        final outcome = await MgrFfi(fake).import('D:/downloads/mod.zip');
        expect(outcome.matchedBy, expected);
        expect(outcome.matchedBy.wireName, wire);
        expect(fake.calls.single.payload, {'path': 'D:/downloads/mod.zip'});
      }
    });

    test('import rejects missing, future, or inconsistent outcomes', () async {
      for (final response in [
        importSuccess(disposition: 'future', matchedBy: 'none'),
        importSuccess(disposition: 'created', matchedBy: 'future'),
        importSuccess(disposition: 'created', matchedBy: 'source'),
        importSuccess(disposition: 'updated', matchedBy: 'none'),
        importSuccess(disposition: 'unchanged', matchedBy: 'none'),
        {
          'ok': true,
          'entry': {
            'id': 'mod-c',
            'kind': 'foreign_pak',
            'name': 'loose_P',
            'components': const [],
          },
        },
      ]) {
        await expectLater(
          MgrFfi(
            FakeGoreCoreFfiService(responses: {'mgr_import': response}),
          ).import('D:/downloads/mod.zip'),
          throwsA(
            isA<MgrFfiException>().having(
              (error) => error.code,
              'code',
              'IMPORT_INVALID_RESPONSE',
            ),
          ),
        );
      }
    });

    test('duplicate refusal exposes bounded typed candidates', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_import': {
            'ok': false,
            'error': {
              'code': 'IMPORT_DUPLICATE_AMBIGUOUS',
              'message': 'verified duplicate',
              'details': {
                'candidate_ids': ['alpha', 'beta'],
              },
            },
          },
        },
      );

      await expectLater(
        MgrFfi(fake).import('D:/downloads/mod.zip'),
        throwsA(
          isA<MgrFfiException>()
              .having(
                (error) => error.code,
                'code',
                'IMPORT_DUPLICATE_AMBIGUOUS',
              )
              .having(
                (error) => (error.details as MgrImportDuplicateAmbiguousDetails)
                    .candidates
                    .map((candidate) => candidate.id),
                'candidate ids',
                ['alpha', 'beta'],
              ),
        ),
      );
    });

    test(
      'identity refusal exposes candidate match roles without parsing text',
      () async {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_import': {
              'ok': false,
              'error': {
                'code': 'IMPORT_IDENTITY_CONFLICT',
                'message': 'opaque prose that names nothing useful',
                'details': {
                  'candidates': [
                    {
                      'id': 'alpha',
                      'matched_by': ['entry_id', 'source'],
                    },
                    {
                      'id': 'beta',
                      'matched_by': ['content'],
                    },
                  ],
                },
              },
            },
          },
        );

        try {
          await MgrFfi(fake).import('D:/downloads/mod.zip');
          fail('expected typed refusal');
        } on MgrFfiException catch (error) {
          expect(error.code, 'IMPORT_IDENTITY_CONFLICT');
          final details = error.details as MgrImportIdentityConflictDetails;
          expect(details.candidates.map((candidate) => candidate.id), [
            'alpha',
            'beta',
          ]);
          expect(details.candidates.first.matchedBy, [
            MgrImportMatchedBy.entryId,
            MgrImportMatchedBy.source,
          ]);
          expect(details.candidates.last.matchedBy, [
            MgrImportMatchedBy.content,
          ]);
        }
      },
    );

    test('malformed or oversized refusal candidates are not exposed', () async {
      for (final details in [
        {
          'candidate_ids': ['alpha', 'beta', 'gamma'],
        },
        {
          'candidate_ids': [List.filled(257, 'x').join()],
        },
        {
          'candidates': [
            {
              'id': 'alpha',
              'matched_by': ['none'],
            },
          ],
        },
      ]) {
        final code = details.containsKey('candidate_ids')
            ? 'IMPORT_DUPLICATE_AMBIGUOUS'
            : 'IMPORT_IDENTITY_CONFLICT';
        try {
          await MgrFfi(
            FakeGoreCoreFfiService(
              responses: {
                'mgr_import': {
                  'ok': false,
                  'error': {
                    'code': code,
                    'message': 'refused',
                    'details': details,
                  },
                },
              },
            ),
          ).import('D:/downloads/mod.zip');
          fail('expected refusal');
        } on MgrFfiException catch (error) {
          expect(error.code, code);
          expect(error.details, isNull);
        }
      }
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
