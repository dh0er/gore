import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

class _FakeCoreService implements GoresaveCoreService {
  _FakeCoreService(this.responses);

  final Map<String, Object> responses;
  final List<String> commands = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    final response = responses[command];
    if (response is Exception) {
      throw response;
    }
    return (response as Map<String, Object?>?) ?? {'ok': false};
  }
}

void main() {
  test('downloads silently and becomes ready when update available', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'updateAvailable', 'version': '0.2.0'},
      },
      'update_download': {
        'ok': true,
        'data': {'downloaded': true, 'version': '0.2.0'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(fake.commands, ['update_check', 'update_download']);
    expect(notifier.state, isA<UpdateReady>());
    expect((notifier.state as UpdateReady).version, '0.2.0');
  });

  test('stays idle when updater disabled', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'disabled'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(fake.commands, ['update_check']);
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle when up to date', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'upToDate'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle and does not throw on check failure', () async {
    final fake = _FakeCoreService({'update_check': Exception('offline')});
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('stays idle when download fails', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'updateAvailable', 'version': '0.2.0'},
      },
      'update_download': {
        'ok': false,
        'error': {'code': 'UPDATE_ERROR', 'message': 'network'},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    expect(notifier.state, isA<UpdateIdle>());
  });

  test('applyAndRestart sends update_apply_restart', () async {
    final fake = _FakeCoreService({
      'update_check': {
        'ok': true,
        'data': {'status': 'disabled'},
      },
      'update_apply_restart': {
        'ok': true,
        'data': {'applied': true},
      },
    });
    final notifier = UpdateNotifier(fake);
    await pumpEventQueue();
    await notifier.applyAndRestart();
    expect(fake.commands.last, 'update_apply_restart');
  });
}
