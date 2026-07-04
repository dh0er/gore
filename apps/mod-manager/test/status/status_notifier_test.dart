import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/status/domain/status_notifier.dart';

StatusNotifier _notifier(FakeGoreCoreFfiService fake) =>
    StatusNotifier(MgrFfi(fake));

void main() {
  group('StatusNotifier.refresh', () {
    test('null gameRoot records the set-path sentinel without calling FFI',
        () async {
      final fake = FakeGoreCoreFfiService(responses: const {});
      final n = _notifier(fake);
      await n.refresh(null);
      expect(n.state.error, StatusNotifier.noGamePath);
      expect(fake.calls, isEmpty);
    });

    test('maps each state to its status variant', () async {
      Future<ManagerStatusView?> statusFor(
        Map<String, Object?> status,
      ) async {
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
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_apply': {
            'ok': false,
            'error': {'code': 'STUDIO_DEPLOY_ACTIVE', 'message': 'studio'},
          },
          'mgr_status': {
            'ok': true,
            'status': {'state': 'nothing_deployed'},
          },
        },
      );
      final n = _notifier(fake);
      // A blocked apply arms studioActive.
      await n.apply('C:/game');
      expect(n.state.studioActive, isTrue);
      // A later refresh where the install has no studio deploy must disarm it,
      // or the take-over prompt stays wrongly available.
      await n.refresh('C:/game');
      expect(n.state.studioActive, isFalse);
      expect(n.state.status, isA<ManagerStatusNothingDeployed>());
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

    test('STUDIO_DEPLOY_ACTIVE sets the studioActive flag', () async {
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
      final n = _notifier(fake);
      await n.apply('C:/game');
      expect(n.state.studioActive, isTrue);
      expect(n.state.error, contains('studio mod'));
      expect(n.state.busy, isFalse);
    });

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
    });
  });

  group('StatusNotifier.undeployAll', () {
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
  });
}
