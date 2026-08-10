import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/status/domain/status_notifier.dart';

StatusNotifier _notifier(FakeGoreCoreFfiService fake) =>
    StatusNotifier(MgrFfi(fake));

class _ControlledCore implements GoreCoreFfiService {
  final requests =
      <
        ({
          String command,
          Map<String, Object?> payload,
          Completer<Map<String, Object?>> response,
        })
      >[];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'controlled-status-test';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    final response = Completer<Map<String, Object?>>();
    requests.add((command: command, payload: payload, response: response));
    return response.future;
  }
}

Map<String, Object?> _statusResponse(String state) => {
  'ok': true,
  'status': {'state': state},
};

Map<String, Object?> _errorResponse(String message) => {
  'ok': false,
  'error': {'code': 'IO', 'message': message},
};

Future<void> _waitForRequests(_ControlledCore core, int count) async {
  while (core.requests.length < count) {
    await Future<void>.delayed(Duration.zero);
  }
}

void main() {
  group('StatusNotifier.refresh', () {
    test(
      'null gameRoot records the set-path sentinel without calling FFI',
      () async {
        final fake = FakeGoreCoreFfiService(responses: const {});
        final n = _notifier(fake);
        await n.refresh(null);
        expect(n.state.error, StatusNotifier.noGamePath);
        expect(fake.calls, isEmpty);
      },
    );

    test('maps each state to its status variant', () async {
      Future<ManagerStatusView?> statusFor(Map<String, Object?> status) async {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_status': {'ok': true, 'status': status},
          },
        );
        final n = _notifier(fake);
        await n.refresh('C:/game');
        return n.state.status;
      }

      expect(
        await statusFor({'state': 'nothing_deployed'}),
        isA<ManagerStatusNothingDeployed>(),
      );
      expect(
        await statusFor({'state': 'recovery_required'}),
        isA<ManagerStatusRecoveryRequired>(),
      );
      expect(
        await statusFor({'state': 'in_sync', 'loadout': []}),
        isA<ManagerStatusInSync>(),
      );
      expect(
        await statusFor({'state': 'changes_pending'}),
        isA<ManagerStatusChangesPending>(),
      );
      expect(
        await statusFor({'state': 'game_updated', 'drifted': []}),
        isA<ManagerStatusGameUpdated>(),
      );
      expect(
        await statusFor({'state': 'studio_deploy_active', 'mod_name': 'M'}),
        isA<ManagerStatusStudioDeployActive>(),
      );
    });

    test('a successful refresh clears a stale studioActive flag', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final apply = n.apply('C:/game');
      core.requests.single.response.complete({
        'ok': false,
        'error': {'code': 'STUDIO_DEPLOY_ACTIVE', 'message': 'studio'},
      });
      await _waitForRequests(core, 2);
      core.requests[1].response.complete(_errorResponse('status unavailable'));
      await apply;
      // A blocked apply with no usable postflight needs the transient fallback.
      expect(n.state.studioActive, isTrue);

      // A later refresh where the install has no studio deploy must disarm it,
      // or the take-over prompt stays wrongly available.
      final refresh = n.refresh('C:/game');
      core.requests[2].response.complete(_statusResponse('nothing_deployed'));
      await refresh;
      expect(n.state.studioActive, isFalse);
      expect(n.state.status, isA<ManagerStatusNothingDeployed>());
    });

    test('an unknown refresh preserves unresolved studio ownership', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final apply = n.apply('C:/game');
      core.requests.single.response.complete({
        'ok': false,
        'error': {'code': 'STUDIO_DEPLOY_ACTIVE', 'message': 'studio'},
      });
      await _waitForRequests(core, 2);
      core.requests[1].response.complete(_errorResponse('status unavailable'));
      await apply;
      expect(n.state.studioActive, isTrue);

      final confirmedRefresh = n.refresh('C:/game');
      core.requests[2].response.complete({
        'ok': true,
        'status': {'state': 'studio_deploy_active', 'mod_name': 'Studio mod'},
      });
      await confirmedRefresh;
      expect(n.state.studioActive, isTrue);

      final unknownRefresh = n.refresh('C:/game');
      core.requests[3].response.complete(_statusResponse('future_state'));
      await unknownRefresh;

      expect(n.state.status, isA<ManagerStatusUnknown>());
      expect(n.state.studioActive, isTrue);
    });

    test('a null-root refresh clears a previously shown status', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_status': {
            'ok': true,
            'status': {'state': 'in_sync', 'loadout': []},
          },
        },
      );
      final n = _notifier(fake);
      await n.refresh('C:/game');
      expect(n.state.status, isA<ManagerStatusInSync>());
      // Clearing/never-setting the path must drop the stale chip, not keep
      // showing "In sync" while the banner asks for a game path.
      await n.refresh(null);
      expect(n.state.status, isNull);
      expect(n.state.error, StatusNotifier.noGamePath);
    });

    test(
      'newer root wins when overlapping reads finish out of order',
      () async {
        final core = _ControlledCore();
        final n = StatusNotifier(MgrFfi(core));

        final first = n.refresh('C:/game-a');
        final second = n.refresh('C:/game-b');
        expect(core.requests, hasLength(2));

        core.requests[1].response.complete(_statusResponse('changes_pending'));
        await second;
        expect(n.state.status, isA<ManagerStatusChangesPending>());
        expect(n.state.statusRoot, 'C:/game-b');
        expect(n.state.busy, isTrue);

        // The old read is ignored for publication, but it is still physically
        // inspecting native state and therefore blocks a write.
        await n.apply('C:/game-b');
        expect(core.requests, hasLength(2));

        core.requests[0].response.complete(_statusResponse('in_sync'));
        await first;
        expect(n.state.status, isA<ManagerStatusChangesPending>());
        expect(n.state.statusRoot, 'C:/game-b');
        expect(n.state.error, isNull);
        expect(n.state.busy, isFalse);
      },
    );

    test('newer same-root read survives an older late failure', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final first = n.refresh('C:/game');
      final second = n.refresh('C:/game');
      core.requests[1].response.complete(_statusResponse('in_sync'));
      await second;
      expect(n.state.busy, isTrue);

      core.requests[0].response.complete(_errorResponse('old failure'));
      await first;
      expect(n.state.status, isA<ManagerStatusInSync>());
      expect(n.state.statusRoot, 'C:/game');
      expect(n.state.error, isNull);
      expect(n.state.busy, isFalse);
    });

    test('null root invalidates a still-pending response', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final pending = n.refresh('C:/game');
      await n.refresh(null);
      expect(n.state.status, isNull);
      expect(n.state.statusRoot, isNull);
      expect(n.state.error, StatusNotifier.noGamePath);
      expect(n.state.busy, isTrue);

      core.requests.single.response.complete(_statusResponse('in_sync'));
      await pending;
      expect(n.state.status, isNull);
      expect(n.state.statusRoot, isNull);
      expect(n.state.error, StatusNotifier.noGamePath);
      expect(n.state.busy, isFalse);
    });

    test('failed refresh clears prior status authority', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final initial = n.refresh('C:/game');
      core.requests.single.response.complete(
        _statusResponse('changes_pending'),
      );
      await initial;
      expect(n.state.statusRoot, 'C:/game');

      final failed = n.refresh('C:/game');
      core.requests[1].response.complete(_errorResponse('cannot inspect'));
      await failed;
      expect(n.state.status, isNull);
      expect(n.state.statusRoot, isNull);
      expect(n.state.error, contains('cannot inspect'));
      expect(n.state.busy, isFalse);
    });
  });

  group('StatusNotifier.apply', () {
    test('records the report then refreshes status', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': true,
            'report': {
              'applied': ['A', 'B'],
              'warnings': ['w1'],
            },
          },
          'mgr_status': {
            'ok': true,
            'status': {'state': 'in_sync', 'loadout': []},
          },
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');

      expect(n.state.lastReport?.applied, ['A', 'B']);
      expect(n.state.lastReport?.warnings, ['w1']);
      // apply is followed by a status refresh.
      expect(n.state.status, isA<ManagerStatusInSync>());
      expect(
        fake.calls.map((c) => c.command),
        containsAllInOrder(['mgr_apply', 'mgr_status']),
      );
      expect(n.state.studioActive, isFalse);
      expect(n.state.busy, isFalse);
    });

    test('STUDIO_DEPLOY_ACTIVE re-queries and uses studio status', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': false,
            'error': {
              'code': 'STUDIO_DEPLOY_ACTIVE',
              'message': 'undeploy the studio mod first',
            },
          },
          // The blocked apply re-queries status so the chip + Apply gating
          // reflect the studio deploy instead of a stale changes_pending.
          'mgr_status': {
            'ok': true,
            'status': {'state': 'studio_deploy_active', 'mod_name': 'Solo'},
          },
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');
      // Preserve the explicit blocked-apply evidence even though this known
      // status currently drives the same take-over action. A later Unknown
      // refresh cannot otherwise retain that fail-closed fallback.
      expect(n.state.studioActive, isTrue);
      expect(n.state.error, contains('studio mod'));
      expect(n.state.status, isA<ManagerStatusStudioDeployActive>());
      expect(n.state.busy, isFalse);
    });

    test(
      'unknown refresh cannot erase a confirmed studio postflight',
      () async {
        final core = _ControlledCore();
        final n = StatusNotifier(MgrFfi(core));

        final apply = n.apply('C:/game');
        core.requests.single.response.complete({
          'ok': false,
          'error': {'code': 'STUDIO_DEPLOY_ACTIVE', 'message': 'studio'},
        });
        await _waitForRequests(core, 2);
        core.requests[1].response.complete({
          'ok': true,
          'status': {'state': 'studio_deploy_active', 'mod_name': 'Studio mod'},
        });
        await apply;
        expect(n.state.studioActive, isTrue);

        final refresh = n.refresh('C:/game');
        core.requests[2].response.complete(_statusResponse('future_state'));
        await refresh;

        expect(n.state.status, isA<ManagerStatusUnknown>());
        expect(n.state.statusRoot, 'C:/game');
        expect(n.state.studioActive, isTrue);
        expect(n.state.error, isNull);
      },
    );

    test('known non-studio postflight clears transient studio block', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': false,
            'error': {
              'code': 'STUDIO_DEPLOY_ACTIVE',
              'message': 'studio owned the install during apply',
            },
          },
          'mgr_status': _statusResponse('nothing_deployed'),
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');

      expect(n.state.status, isA<ManagerStatusNothingDeployed>());
      expect(n.state.statusRoot, 'C:/game');
      expect(n.state.studioActive, isFalse);
      expect(n.state.error, contains('studio owned'));
    });

    test(
      'studio block remains root-bound when postflight status fails',
      () async {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_apply': {
              'ok': false,
              'error': {
                'code': 'STUDIO_DEPLOY_ACTIVE',
                'message': 'studio still owns this install',
              },
            },
            'mgr_status': _errorResponse('postflight unavailable'),
          },
        );
        final n = _notifier(fake);
        await n.apply('C:/game');

        expect(n.state.gameRoot, 'C:/game');
        expect(n.state.status, isNull);
        expect(n.state.statusRoot, isNull);
        expect(n.state.studioActive, isTrue);
        expect(n.state.error, contains('studio still owns'));
      },
    );

    test('a non-studio error surfaces without the studio flag', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': false,
            'error': {'code': 'IO', 'message': 'disk full'},
          },
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');
      expect(n.state.studioActive, isFalse);
      expect(n.state.error, contains('disk full'));
      // A failed apply must not leave a stale success report behind.
      expect(n.state.lastReport, isNull);
      expect(
        fake.calls.map((call) => call.command),
        containsAllInOrder(['mgr_apply', 'mgr_status']),
      );
    });

    test(
      'root switches coalesce behind apply and latest root wins publication',
      () async {
        final core = _ControlledCore();
        final n = StatusNotifier(MgrFfi(core));

        final apply = n.apply('C:/game-a');
        expect(core.requests.single.command, 'mgr_apply');

        final switchedToB = n.refresh('C:/game-b');
        final switchedToC = n.refresh('C:/game-c');
        // Both reads are held behind the physical write and coalesce to C.
        expect(core.requests, hasLength(1));
        expect(n.state.status, isNull);
        expect(n.state.statusRoot, isNull);
        expect(n.state.busy, isTrue);

        core.requests[0].response.complete({
          'ok': true,
          'report': {
            'applied': ['A'],
            'warnings': <String>[],
          },
        });
        await _waitForRequests(core, 2);
        expect(core.requests[1].command, 'mgr_status');
        expect(core.requests[1].payload['game_root'], 'C:/game-a');
        core.requests[1].response.complete(_statusResponse('in_sync'));

        await _waitForRequests(core, 3);
        expect(core.requests[2].command, 'mgr_status');
        expect(core.requests[2].payload['game_root'], 'C:/game-c');
        core.requests[2].response.complete(_statusResponse('changes_pending'));
        await Future.wait([apply, switchedToB, switchedToC]);

        expect(n.state.status, isA<ManagerStatusChangesPending>());
        expect(n.state.statusRoot, 'C:/game-c');
        expect(n.state.lastReport, isNull);
        expect(n.state.error, isNull);
        expect(n.state.busy, isFalse);
      },
    );

    test(
      'queued unknown refresh preserves a known studio postflight fallback',
      () async {
        final core = _ControlledCore();
        final n = StatusNotifier(MgrFfi(core));

        final apply = n.apply('C:/game');
        final queuedRefresh = n.refresh('C:/game');
        core.requests.single.response.complete({
          'ok': false,
          'error': {'code': 'STUDIO_DEPLOY_ACTIVE', 'message': 'studio'},
        });
        await _waitForRequests(core, 2);
        core.requests[1].response.complete({
          'ok': true,
          'status': {'state': 'studio_deploy_active', 'mod_name': 'Studio mod'},
        });
        await _waitForRequests(core, 3);
        core.requests[2].response.complete(_statusResponse('future_state'));
        await Future.wait([apply, queuedRefresh]);

        expect(n.state.status, isA<ManagerStatusUnknown>());
        expect(n.state.statusRoot, 'C:/game');
        expect(n.state.studioActive, isTrue);
        expect(n.state.error, contains('studio'));
        expect(n.state.busy, isFalse);
      },
    );

    test('apply and undeploy cannot overlap physical writes', () async {
      final core = _ControlledCore();
      final n = StatusNotifier(MgrFfi(core));

      final apply = n.apply('C:/game');
      await n.undeployAll('C:/game');
      expect(
        core.requests.where(
          (request) =>
              request.command == 'mgr_apply' ||
              request.command == 'mgr_undeploy_all',
        ),
        hasLength(1),
      );

      core.requests.single.response.complete({
        'ok': true,
        'report': {'applied': <String>[], 'warnings': <String>[]},
      });
      await _waitForRequests(core, 2);
      core.requests[1].response.complete(_statusResponse('in_sync'));
      await apply;
      expect(n.state.busy, isFalse);
    });

    test('successful apply with failed postflight clears authority', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': true,
            'report': {
              'applied': ['A'],
              'warnings': <String>[],
            },
          },
          'mgr_status': _errorResponse('postflight unavailable'),
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');

      expect(n.state.lastReport?.applied, ['A']);
      expect(n.state.status, isNull);
      expect(n.state.statusRoot, isNull);
      expect(n.state.error, contains('postflight unavailable'));
      expect(n.state.busy, isFalse);
    });
  });

  group('StatusNotifier.undeployAll', () {
    test(
      'failure still runs postflight and keeps command error priority',
      () async {
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_undeploy_all': _errorResponse('undeploy command failed'),
            'mgr_status': _statusResponse('recovery_required'),
          },
        );
        final n = _notifier(fake);
        await n.undeployAll('C:/game');

        expect(
          fake.calls.map((call) => call.command),
          containsAllInOrder(['mgr_undeploy_all', 'mgr_status']),
        );
        expect(n.state.error, contains('undeploy command failed'));
        expect(n.state.status, isA<ManagerStatusRecoveryRequired>());
        expect(n.state.statusRoot, 'C:/game');
        expect(n.state.busy, isFalse);
      },
    );

    for (final postflightState in ['studio_deploy_active', 'future_state']) {
      test(
        'failed undeploy preserves studio evidence through $postflightState',
        () async {
          final core = _ControlledCore();
          final n = StatusNotifier(MgrFfi(core));

          final initialRefresh = n.refresh('C:/game');
          core.requests.single.response.complete({
            'ok': true,
            'status': {
              'state': 'studio_deploy_active',
              'mod_name': 'Studio mod',
            },
          });
          await initialRefresh;
          expect(n.state.studioActive, isTrue);

          final undeploy = n.undeployAll('C:/game');
          core.requests[1].response.complete(
            _errorResponse('undeploy command failed'),
          );
          await _waitForRequests(core, 3);
          core.requests[2].response.complete(
            postflightState == 'studio_deploy_active'
                ? {
                    'ok': true,
                    'status': {
                      'state': postflightState,
                      'mod_name': 'Studio mod',
                    },
                  }
                : _statusResponse(postflightState),
          );
          await undeploy;

          expect(n.state.studioActive, isTrue);
          expect(n.state.error, contains('undeploy command failed'));

          final unknownRefresh = n.refresh('C:/game');
          core.requests[3].response.complete(_statusResponse('later_future'));
          await unknownRefresh;
          expect(n.state.status, isA<ManagerStatusUnknown>());
          expect(n.state.studioActive, isTrue);
        },
      );
    }

    test('undeploys then refreshes and clears the studio flag', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_undeploy_all': {'ok': true, 'removed': 3},
          'mgr_status': {
            'ok': true,
            'status': {'state': 'nothing_deployed'},
          },
        },
      );
      final n = _notifier(fake);
      await n.undeployAll('C:/game');
      expect(
        fake.calls.map((c) => c.command),
        containsAllInOrder(['mgr_undeploy_all', 'mgr_status']),
      );
      expect(n.state.status, isA<ManagerStatusNothingDeployed>());
      expect(n.state.studioActive, isFalse);
    });

    test('clears a prior apply success report', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': true,
            'report': {
              'applied': ['A'],
              'warnings': <String>[],
            },
          },
          'mgr_undeploy_all': {'ok': true, 'removed': 1},
          'mgr_status': {
            'ok': true,
            'status': {'state': 'nothing_deployed'},
          },
        },
      );
      final n = _notifier(fake);
      await n.apply('C:/game');
      expect(n.state.lastReport?.applied, ['A']); // precondition
      // Undeploying everything must drop the stale "Applied N mods" report.
      await n.undeployAll('C:/game');
      expect(n.state.lastReport, isNull);
    });
  });
}
