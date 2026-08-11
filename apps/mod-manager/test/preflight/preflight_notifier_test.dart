import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/preflight/domain/preflight_notifier.dart';

Map<String, Object?> _response({String detail = 'ready'}) {
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
        for (final id in ids)
          {
            'id': id,
            'state': id == 'write_access' ? 'unverified' : 'ok',
            'code': id == 'write_access' ? 'unverified_read_only' : 'ready',
            'action': id == 'write_access' ? 'verify_during_apply' : 'none',
            'detail': '$detail:$id',
            'items': <String>[],
          },
      ],
    },
  };
}

class _QueuedCore implements GoreCoreFfiService {
  final calls = <({String command, Map<String, Object?> payload})>[];
  final responses = <Completer<Map<String, Object?>>>[];

  @override
  String get description => 'queued-preflight';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    calls.add((command: command, payload: payload));
    final response = Completer<Map<String, Object?>>();
    responses.add(response);
    return response.future;
  }
}

void main() {
  test('null selection stays local and never calls native', () async {
    final core = _QueuedCore();
    final notifier = PreflightNotifier(MgrFfi(core));
    addTearDown(notifier.dispose);

    expect(notifier.state.pending, isFalse);
    await notifier.refresh();

    expect(core.calls, isEmpty);
    expect(notifier.state.busy, isFalse);
  });

  test('busy spans the physical read and success binds exact root', () async {
    final core = _QueuedCore();
    final notifier = PreflightNotifier(MgrFfi(core), initialRoot: 'A');
    addTearDown(notifier.dispose);

    final refresh = notifier.refresh();
    expect(notifier.state.busy, isTrue);
    expect(notifier.state.pending, isFalse);
    expect(core.calls.single.payload, {'game_root': 'A'});

    core.responses.single.complete(_response(detail: 'A'));
    await refresh;

    expect(notifier.state.busy, isFalse);
    expect(notifier.state.authoritative, isTrue);
    expect(notifier.state.reportRoot, 'A');
    expect(notifier.state.report!.checks.first.detail, 'A:game_root');
  });

  test('late old-root result cannot publish after a root switch', () async {
    final core = _QueuedCore();
    final notifier = PreflightNotifier(MgrFfi(core), initialRoot: 'A');
    addTearDown(notifier.dispose);

    final oldRead = notifier.refresh();
    notifier.selectRoot('B');
    expect(notifier.state.candidateRoot, 'B');
    expect(notifier.state.report, isNull);
    expect(notifier.state.pending, isTrue);
    expect(notifier.state.busy, isTrue);

    core.responses[0].complete(_response(detail: 'A'));
    await oldRead;
    expect(notifier.state.report, isNull);
    expect(notifier.state.pending, isTrue);
    expect(notifier.state.busy, isFalse);

    final newRead = notifier.refresh();
    core.responses[1].complete(_response(detail: 'B'));
    await newRead;
    expect(notifier.state.reportRoot, 'B');
    expect(notifier.state.report!.checks.first.detail, 'B:game_root');
  });

  test(
    'same-root invalidation clears authority and failure stays settled',
    () async {
      final core = _QueuedCore();
      final notifier = PreflightNotifier(MgrFfi(core), initialRoot: 'A');
      addTearDown(notifier.dispose);

      final first = notifier.refresh();
      core.responses[0].complete(_response());
      await first;
      expect(notifier.state.authoritative, isTrue);

      notifier.invalidateLibrary();
      expect(notifier.state.report, isNull);
      expect(notifier.state.pending, isTrue);

      final failed = notifier.refresh();
      core.responses[1].complete({
        'ok': false,
        'error': {'code': 'INSPECTION_FAILED', 'message': 'cannot inspect'},
      });
      await failed;

      expect(notifier.state.authoritative, isFalse);
      expect(notifier.state.error, contains('cannot inspect'));
      expect(notifier.state.pending, isFalse);
      expect(notifier.state.busy, isFalse);
      await notifier.refresh();
      expect(core.calls, hasLength(2), reason: 'failure must not auto-loop');
    },
  );

  test('retry clears stale error and requests exactly one new read', () async {
    final core = _QueuedCore();
    final notifier = PreflightNotifier(MgrFfi(core), initialRoot: 'A');
    addTearDown(notifier.dispose);

    final failed = notifier.refresh();
    core.responses[0].complete({
      'ok': false,
      'error': {'message': 'temporary'},
    });
    await failed;
    expect(notifier.state.error, isNotNull);

    notifier.retry();
    expect(notifier.state.error, isNull);
    expect(notifier.state.pending, isTrue);
    final retried = notifier.refresh();
    core.responses[1].complete(_response());
    await retried;

    expect(notifier.state.authoritative, isTrue);
    expect(core.calls, hasLength(2));
  });
}
