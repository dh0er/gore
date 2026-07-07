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

  test('worker with an unloadable library fails fast and recovers', () async {
    // The persistent worker opens the DLL before signalling ready; a bad path
    // must surface as a thrown error (via onError/onExit) rather than an eternal
    // await, and a second call must not hang either (a fresh worker is spawned).
    final core = NativeGoresaveCoreService.withLibraryPath(
      'gore_save_does_not_exist_${DateTime.now().microsecondsSinceEpoch}.dll',
    );

    await expectLater(
      core
          .execute('scan_save_dir')
          .timeout(const Duration(seconds: 20)),
      throwsA(isA<CoreWorkerException>()),
    );
    // A retry recovers to another fast failure, not a wedged call.
    await expectLater(
      core
          .execute('scan_save_dir')
          .timeout(const Duration(seconds: 20)),
      throwsA(isA<CoreWorkerException>()),
    );
  });
}
