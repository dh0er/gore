import 'dart:isolate';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

void main() {
  test('native core work runs on a background isolate', () async {
    final startedPort = ReceivePort();
    final startedSendPort = startedPort.sendPort;
    final stopwatch = Stopwatch()..start();

    final resultFuture = runCoreWorkOnBackgroundIsolate(() {
      startedSendPort.send('started');
      final workerStopwatch = Stopwatch()..start();
      while (workerStopwatch.elapsedMilliseconds < 300) {}
      return 42;
    });

    final elapsedAfterCall = stopwatch.elapsedMilliseconds;
    expect(elapsedAfterCall, lessThan(150));
    expect(await startedPort.first, 'started');
    expect(await resultFuture, 42);
    startedPort.close();
  });
}
