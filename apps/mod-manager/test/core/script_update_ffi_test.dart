import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';

import '../support/script_update_fixture.dart';

void main() {
  test(
    'script-update refusal exposes exact typed token and every module',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'mgr_apply': scriptUpdateRefusal()},
      );
      try {
        await MgrFfi(core).apply('C:/game');
        fail('expected confirmation');
      } on MgrFfiException catch (error) {
        expect(error.code, 'SCRIPT_REBUILD_CONFIRMATION_REQUIRED');
        final details = error.details as MgrScriptRebuildConfirmation;
        expect(details.token, 'exact-update-token');
        expect(details.warnings.map((warning) => warning.module), [
          'NPC/Diego',
          'Quests/OldQuest',
        ]);
        expect(details.warnings.first.modId, 'dialog-mod');
        expect(details.warnings.first.modName, 'Diego Dialog');
        expect(details.warnings.first.reason, 'vanilla_module_changed');
        expect(
          details.warnings.first.originalSha256,
          List.filled(64, 'a').join(),
        );
        expect(
          details.warnings.first.currentSha256,
          List.filled(64, 'b').join(),
        );
        expect(details.warnings.last.currentSha256, isNull);
      }
      expect(core.calls.single.payload, {'game_root': 'C:/game'});
    },
  );

  test(
    'apply forwards only the supplied exact token and retains rebuild warnings',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': true,
            'report': {
              'applied': ['Diego Dialog'],
              'warnings': ['Recompiled NPC/Diego'],
            },
          },
        },
      );
      final report = await MgrFfi(
        core,
      ).apply('C:/game', scriptRebuildConfirmation: ' exact opaque token ');
      expect(core.calls.single.payload, {
        'game_root': 'C:/game',
        'script_rebuild_confirmation': ' exact opaque token ',
      });
      expect(report.warnings, ['Recompiled NPC/Diego']);
    },
  );

  test(
    'malformed warning selections never expose confirmation authority',
    () async {
      final malformed = <Object?>[
        null,
        {},
        {'token': '', 'warnings': []},
      ];
      for (final field in [
        'mod_id',
        'mod_name',
        'module',
        'reason',
        'original_sha256',
        'current_sha256',
      ]) {
        final details = scriptUpdateDetails();
        ((details['warnings'] as List).first as Map)[field] = 7;
        malformed.add(details);
      }
      for (final field in ['token', 'warnings']) {
        final details = scriptUpdateDetails()..remove(field);
        malformed.add(details);
      }
      final missingHash = scriptUpdateDetails();
      ((missingHash['warnings'] as List).first as Map).remove('current_sha256');
      malformed.add(missingHash);
      malformed.add(scriptUpdateDetails()..['warnings'] = []);
      for (final details in malformed) {
        final core = FakeGoreCoreFfiService(
          responses: {
            'mgr_apply': {
              'ok': false,
              'error': {
                'code': 'SCRIPT_REBUILD_CONFIRMATION_REQUIRED',
                'details': details,
              },
            },
          },
        );
        await expectLater(
          MgrFfi(core).apply('C:/game'),
          throwsA(
            isA<MgrFfiException>().having(
              (error) => error.details,
              'details',
              isNull,
            ),
          ),
        );
      }
    },
  );
}
