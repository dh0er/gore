import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_playback.dart';

void main() {
  group('Revision3VoiceTakePreviewPlaybackController', () {
    test('unused standard controller disposal completes', () async {
      final controller = Revision3VoiceTakePreviewPlaybackController.standard();

      await controller.dispose().timeout(const Duration(seconds: 2));
    });

    test(
      'unused standard controller disposal completes after unawaited stop',
      () async {
        final controller =
            Revision3VoiceTakePreviewPlaybackController.standard();

        unawaited(controller.stop());
        await controller.dispose().timeout(const Duration(seconds: 2));
      },
    );

    test(
      'stops and unloads before closing the old lease and opening new',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final first = _FakeLease('first.ogg', events);
        final second = _FakeLease('second.ogg', events);

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async {
            events.add('materialize:first');
            return first.value;
          },
        );
        events.clear();

        await controller.preview(
          takeKey: 'take-b',
          materialize: () async {
            events.add('materialize:second');
            return second.value;
          },
        );

        expect(events, <String>[
          'player:stop',
          'lease:close:first.ogg',
          'materialize:second',
          'player:open:second.ogg',
        ]);
        expect(first.closed, isTrue);
        expect(controller.snapshot.activeTakeKey, 'take-b');
        expect(
          controller.snapshot.phase,
          Revision3VoiceTakePreviewPlaybackPhase.playing,
        );
        await controller.dispose();
      },
    );

    test(
      'newer request closes a late result without ever opening it',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final lateMaterialization =
            Completer<Revision3VoiceTakePreviewPlaybackLease>();
        final late = _FakeLease('late.ogg', events);
        final newest = _FakeLease('newest.ogg', events);

        final firstRequest = controller.preview(
          takeKey: 'take-old',
          materialize: () => lateMaterialization.future,
        );
        await Future<void>.delayed(Duration.zero);
        final newestRequest = controller.preview(
          takeKey: 'take-new',
          materialize: () async {
            events.add('materialize:newest');
            return newest.value;
          },
        );
        lateMaterialization.complete(late.value);
        await Future.wait(<Future<void>>[firstRequest, newestRequest]);

        expect(late.closed, isTrue);
        expect(events, isNot(contains('player:open:late.ogg')));
        expect(events, contains('player:open:newest.ogg'));
        expect(controller.snapshot.activeTakeKey, 'take-new');
        await controller.dispose();
      },
    );

    test(
      'dispose closes a result that completes after disposal starts',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final materialization =
            Completer<Revision3VoiceTakePreviewPlaybackLease>();
        final late = _FakeLease('late-dispose.ogg', events);

        final request = controller.preview(
          takeKey: 'take-a',
          materialize: () => materialization.future,
        );
        await Future<void>.delayed(Duration.zero);
        final disposal = controller.dispose();
        materialization.complete(late.value);
        await Future.wait(<Future<void>>[request, disposal]);

        expect(late.closed, isTrue);
        expect(events, isNot(contains('player:open:late-dispose.ogg')));
        expect(events.last, 'player:dispose');
      },
    );

    test(
      'active disposal releases native playback before deleting lease',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final lease = _FakeLease('active.ogg', events);
        await controller.preview(
          takeKey: 'take-a',
          materialize: () async => lease.value,
        );
        events.clear();

        await controller.dispose();

        expect(events, <String>[
          'player:stop',
          'lease:close:active.ogg',
          'player:dispose',
        ]);
      },
    );

    test('pause seek and replay remain in one active take', () async {
      final events = <String>[];
      final player = _FakePreviewPlayer(events);
      final controller = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _FakeLease('transport.ogg', events);
      await controller.preview(
        takeKey: 'take-a',
        materialize: () async => lease.value,
      );

      await controller.pause();
      await controller.seek(const Duration(seconds: 4));
      player.complete();
      await Future<void>.delayed(Duration.zero);
      await controller.play();

      expect(
        events,
        containsAllInOrder(<String>[
          'player:pause',
          'player:seek:4000',
          'player:play',
        ]),
      );
      expect(controller.snapshot.activeTakeKey, 'take-a');
      expect(
        controller.snapshot.phase,
        Revision3VoiceTakePreviewPlaybackPhase.playing,
      );
      await controller.dispose();
    });

    test(
      'queued transport is dropped when another take supersedes it',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final first = _FakeLease('first-transport.ogg', events);
        final second = _FakeLease('second-transport.ogg', events);
        await controller.preview(
          takeKey: 'take-a',
          materialize: () async => first.value,
        );
        events.clear();

        player.pauseGate = Completer<void>();
        player.pauseStarted = Completer<void>();
        final blockingPause = controller.pause();
        await player.pauseStarted!.future;
        final queuedPlay = controller.play();
        final queuedSeekA = controller.seek(const Duration(seconds: 2));
        final queuedSeekB = controller.seek(const Duration(seconds: 5));
        final replacement = controller.preview(
          takeKey: 'take-b',
          materialize: () async {
            events.add('materialize:second');
            return second.value;
          },
        );
        player.pauseGate!.complete();

        await Future.wait(<Future<void>>[
          blockingPause,
          queuedPlay,
          queuedSeekA,
          queuedSeekB,
          replacement,
        ]);

        expect(events, isNot(contains('player:play')));
        expect(
          events.where((event) => event.startsWith('player:seek:')),
          isEmpty,
        );
        expect(
          events,
          containsAllInOrder(<String>[
            'player:pause',
            'player:stop',
            'lease:close:first-transport.ogg',
            'materialize:second',
            'player:open:second-transport.ogg',
          ]),
        );
        expect(controller.snapshot.activeTakeKey, 'take-b');
        await controller.dispose();
      },
    );

    test(
      'queued slider seeks coalesce to the newest exact-take value',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final lease = _FakeLease('coalesced-seek.ogg', events);
        await controller.preview(
          takeKey: 'take-a',
          materialize: () async => lease.value,
        );
        events.clear();

        player.pauseGate = Completer<void>();
        player.pauseStarted = Completer<void>();
        final blockingPause = controller.pause();
        await player.pauseStarted!.future;
        final firstSeek = controller.seek(const Duration(seconds: 1));
        final secondSeek = controller.seek(const Duration(seconds: 2));
        final finalSeek = controller.seek(const Duration(seconds: 3));
        player.pauseGate!.complete();
        await Future.wait(<Future<void>>[
          blockingPause,
          firstSeek,
          secondSeek,
          finalSeek,
        ]);

        expect(
          events.where((event) => event.startsWith('player:seek:')).toList(),
          <String>['player:seek:3000'],
        );
        await controller.dispose();
      },
    );

    test('decoder open failure unloads and closes its lease', () async {
      final events = <String>[];
      final player = _FakePreviewPlayer(events)..failOpen = true;
      final controller = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _FakeLease('decoder-error.ogg', events);

      await controller.preview(
        takeKey: 'take-a',
        materialize: () async => lease.value,
      );

      expect(
        events,
        containsAllInOrder(<String>[
          'player:open:decoder-error.ogg',
          'player:stop',
          'lease:close:decoder-error.ogg',
        ]),
      );
      expect(lease.closed, isTrue);
      expect(
        controller.snapshot.failure,
        Revision3VoiceTakePreviewFailureKind.playback,
      );
      await controller.dispose();
    });

    test(
      'decoder failure retains a failed lease cleanup until Stop retries',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events)..failOpen = true;
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final lease = _FakeLease(
          'decoder-cleanup-error.ogg',
          events,
          failuresBeforeClose: 1,
        );

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async => lease.value,
        );

        expect(lease.closed, isFalse);
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.stop();
        expect(lease.closeAttempts, 2);
        expect(lease.closed, isTrue);
        await controller.dispose();
      },
    );

    test(
      'classifies stale and requires-reopen materialization failures',
      () async {
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: _FakePreviewPlayer(<String>[]),
        );

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async =>
              throw const Revision3VoiceTakePreviewStaleCheckpointException(),
        );
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.staleCheckpoint,
        );

        await controller.preview(
          takeKey: 'take-b',
          materialize: () async =>
              throw const Revision3VoiceTakePreviewRequiresReopenException(),
        );
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.requiresReopen,
        );
        await controller.stop();
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.requiresReopen,
        );
        await controller.dispose();
      },
    );

    test(
      'retains failed materialization cleanup for stop and dispose retries',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final cleanup = _FakeCleanupObligation(failuresBeforeClean: 2);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async => throw cleanup,
        );

        expect(cleanup.attempts, 1);
        expect(cleanup.isCleaned, isFalse);
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.stop();
        expect(cleanup.attempts, 2);
        expect(cleanup.isCleaned, isFalse);
        expect(controller.snapshot.activeTakeKey, 'take-a');
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.dispose();
        expect(cleanup.attempts, 3);
        expect(cleanup.isCleaned, isTrue);
      },
    );

    test(
      'retains cleanup ownership carried by requires-reopen failure',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final cleanup = _FakeCleanupObligation(failuresBeforeClean: 1);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async =>
              throw Revision3VoiceTakePreviewRequiresReopenException(
                cause: StateError('fake receipt mismatch'),
                cleanupObligation: cleanup,
              ),
        );

        expect(cleanup.attempts, 1);
        expect(cleanup.isCleaned, isFalse);
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.stop();
        expect(cleanup.attempts, 2);
        expect(cleanup.isCleaned, isTrue);
        expect(controller.snapshot.activeTakeKey, 'take-a');
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.requiresReopen,
        );
        await controller.dispose();
      },
    );

    test(
      'retains stale cleanup ownership and reveals stale after retry',
      () async {
        final cleanup = _FakeCleanupObligation(failuresBeforeClean: 1);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: _FakePreviewPlayer(<String>[]),
        );

        await controller.preview(
          takeKey: 'take-a',
          materialize: () async =>
              throw Revision3VoiceTakePreviewStaleCheckpointException(
                cleanupObligation: cleanup,
              ),
        );

        expect(cleanup.attempts, 1);
        expect(cleanup.isCleaned, isFalse);
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.stop();
        expect(cleanup.attempts, 2);
        expect(cleanup.isCleaned, isTrue);
        expect(controller.snapshot.activeTakeKey, 'take-a');
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.staleCheckpoint,
        );
        await controller.dispose();
      },
    );

    test(
      'terminal dispose hands unresolved cleanup to the next controller',
      () async {
        final cleanup = _FakeCleanupObligation(failuresBeforeClean: 2);
        final first = Revision3VoiceTakePreviewPlaybackController(
          player: _FakePreviewPlayer(<String>[]),
        );
        await first.preview(
          takeKey: 'take-a',
          materialize: () async => throw cleanup,
        );

        await first.dispose();
        expect(cleanup.attempts, 2);
        expect(cleanup.isCleaned, isFalse);

        final second = Revision3VoiceTakePreviewPlaybackController(
          player: _FakePreviewPlayer(<String>[]),
        );
        await second.stop();

        expect(cleanup.attempts, 3);
        expect(cleanup.isCleaned, isTrue);
        await second.dispose();
      },
    );

    test('replacement retries cleanup of a superseded late lease', () async {
      final events = <String>[];
      final player = _FakePreviewPlayer(events);
      final controller = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lateMaterialization =
          Completer<Revision3VoiceTakePreviewPlaybackLease>();
      final late = _FakeLease('late.ogg', events, failuresBeforeClose: 1);
      final newest = _FakeLease('newest.ogg', events);

      final oldRequest = controller.preview(
        takeKey: 'old',
        materialize: () => lateMaterialization.future,
      );
      await Future<void>.delayed(Duration.zero);
      final newRequest = controller.preview(
        takeKey: 'new',
        materialize: () async => newest.value,
      );
      lateMaterialization.complete(late.value);
      await Future.wait(<Future<void>>[oldRequest, newRequest]);

      expect(late.closeAttempts, 2);
      expect(late.closed, isTrue);
      expect(events, contains('player:open:newest.ogg'));
      await controller.dispose();
    });

    test(
      'repeated superseded cleanup failure is classified as cleanup',
      () async {
        final events = <String>[];
        final player = _FakePreviewPlayer(events);
        final controller = Revision3VoiceTakePreviewPlaybackController(
          player: player,
        );
        final lateMaterialization =
            Completer<Revision3VoiceTakePreviewPlaybackLease>();
        final late = _FakeLease(
          'late-repeated.ogg',
          events,
          failuresBeforeClose: 2,
        );
        var newestMaterialized = false;

        final oldRequest = controller.preview(
          takeKey: 'old',
          materialize: () => lateMaterialization.future,
        );
        await Future<void>.delayed(Duration.zero);
        final newRequest = controller.preview(
          takeKey: 'new',
          materialize: () async {
            newestMaterialized = true;
            return _FakeLease('must-not-open.ogg', events).value;
          },
        );
        lateMaterialization.complete(late.value);
        await Future.wait(<Future<void>>[oldRequest, newRequest]);

        expect(late.closeAttempts, 2);
        expect(newestMaterialized, isFalse);
        expect(controller.snapshot.activeTakeKey, 'new');
        expect(
          controller.snapshot.failure,
          Revision3VoiceTakePreviewFailureKind.cleanup,
        );

        await controller.stop();
        expect(late.closeAttempts, 3);
        expect(late.closed, isTrue);
        await controller.dispose();
      },
    );

    test('terminal dispose retries an active lease close failure', () async {
      final events = <String>[];
      final player = _FakePreviewPlayer(events);
      final controller = Revision3VoiceTakePreviewPlaybackController(
        player: player,
      );
      final lease = _FakeLease(
        'active-retry.ogg',
        events,
        failuresBeforeClose: 1,
      );
      await controller.preview(
        takeKey: 'take-a',
        materialize: () async => lease.value,
      );

      await controller.dispose();

      expect(lease.closeAttempts, 2);
      expect(lease.closed, isTrue);
      expect(
        events.indexOf('player:stop'),
        lessThan(events.indexOf('lease:close:active-retry.ogg')),
      );
    });

    test(
      'terminal double player failure hands player and active lease to retry owner',
      () async {
        final events = <String>[];
        final oldPlayer = _FakePreviewPlayer(events, label: 'old');
        final firstController = Revision3VoiceTakePreviewPlaybackController(
          player: oldPlayer,
        );
        final lease = _FakeLease('retained-active.ogg', events);
        await firstController.preview(
          takeKey: 'take-a',
          materialize: () async => lease.value,
        );
        final stopAttemptsBeforeDispose = oldPlayer.stopAttempts;
        final disposeAttemptsBeforeDispose = oldPlayer.disposeAttempts;
        oldPlayer.failuresBeforeStop = stopAttemptsBeforeDispose + 1;
        oldPlayer.failuresBeforeDispose = disposeAttemptsBeforeDispose + 1;
        events.clear();

        await firstController.dispose();

        expect(oldPlayer.stopAttempts, stopAttemptsBeforeDispose + 1);
        expect(oldPlayer.disposeAttempts, disposeAttemptsBeforeDispose + 1);
        expect(lease.closeAttempts, 0);
        expect(lease.closed, isFalse);

        final nextController = Revision3VoiceTakePreviewPlaybackController(
          player: _FakePreviewPlayer(events, label: 'new'),
        );
        await nextController.stop();

        expect(oldPlayer.stopAttempts, stopAttemptsBeforeDispose + 2);
        expect(oldPlayer.disposeAttempts, disposeAttemptsBeforeDispose + 2);
        expect(lease.closeAttempts, 1);
        expect(lease.closed, isTrue);
        expect(
          events,
          containsAllInOrder(<String>[
            'player:stop:new',
            'player:stop:old',
            'lease:close:retained-active.ogg',
            'player:dispose:old',
          ]),
        );
        await nextController.dispose();
      },
    );
  });
}

final class _FakeCleanupObligation
    implements Revision3VoiceTakePreviewCleanupObligation, Exception {
  _FakeCleanupObligation({required this.failuresBeforeClean});

  final int failuresBeforeClean;
  int attempts = 0;
  bool _cleaned = false;

  @override
  bool get isCleaned => _cleaned;

  @override
  Future<void> retryCleanup() async {
    attempts++;
    if (attempts <= failuresBeforeClean) {
      throw StateError('fake retained cleanup failure');
    }
    _cleaned = true;
  }
}

final class _FakeLease {
  _FakeLease(this.path, this.events, {this.failuresBeforeClose = 0});

  final String path;
  final List<String> events;
  final int failuresBeforeClose;
  bool closed = false;
  int closeAttempts = 0;

  late final Revision3VoiceTakePreviewPlaybackLease value =
      Revision3VoiceTakePreviewPlaybackLease(
        path: path,
        isClosed: () => closed,
        close: () async {
          closeAttempts++;
          events.add('lease:close:$path');
          if (closeAttempts <= failuresBeforeClose) {
            throw StateError('fake lease cleanup failure');
          }
          closed = true;
        },
      );
}

final class _FakePreviewPlayer implements Revision3VoiceTakePreviewPlayer {
  _FakePreviewPlayer(this.events, {this.label = ''});

  final List<String> events;
  final String label;
  int failuresBeforeStop = 0;
  int failuresBeforeDispose = 0;
  final StreamController<Revision3VoiceTakePreviewPlayerSnapshot> _snapshots =
      StreamController<Revision3VoiceTakePreviewPlayerSnapshot>.broadcast();
  Revision3VoiceTakePreviewPlayerSnapshot _snapshot =
      const Revision3VoiceTakePreviewPlayerSnapshot.idle();
  bool failOpen = false;
  Completer<void>? pauseGate;
  Completer<void>? pauseStarted;
  int stopAttempts = 0;
  int disposeAttempts = 0;

  String _event(String value) => label.isEmpty ? value : '$value:$label';

  @override
  Revision3VoiceTakePreviewPlayerSnapshot get snapshot => _snapshot;

  @override
  Stream<Revision3VoiceTakePreviewPlayerSnapshot> get snapshots =>
      _snapshots.stream;

  void _emit(Revision3VoiceTakePreviewPlayerSnapshot value) {
    _snapshot = value;
    _snapshots.add(value);
  }

  @override
  Future<void> open(String path) async {
    events.add(_event('player:open:$path'));
    if (failOpen) throw StateError('fake decoder failure');
    _emit(
      const Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
        duration: Duration(seconds: 10),
      ),
    );
  }

  @override
  Future<void> pause() async {
    events.add(_event('player:pause'));
    final started = pauseStarted;
    if (started != null && !started.isCompleted) started.complete();
    await pauseGate?.future;
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.paused,
        position: _snapshot.position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> play() async {
    events.add(_event('player:play'));
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
        position:
            _snapshot.phase == Revision3VoiceTakePreviewPlaybackPhase.completed
            ? Duration.zero
            : _snapshot.position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> seek(Duration position) async {
    events.add(_event('player:seek:${position.inMilliseconds}'));
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: _snapshot.phase,
        position: position,
        duration: _snapshot.duration,
      ),
    );
  }

  void complete() => _emit(
    Revision3VoiceTakePreviewPlayerSnapshot(
      phase: Revision3VoiceTakePreviewPlaybackPhase.completed,
      position: _snapshot.duration,
      duration: _snapshot.duration,
    ),
  );

  @override
  Future<void> stopAndUnload() async {
    stopAttempts++;
    events.add(_event('player:stop'));
    if (stopAttempts <= failuresBeforeStop) {
      throw StateError('fake player stop failure');
    }
    _snapshot = const Revision3VoiceTakePreviewPlayerSnapshot.idle();
  }

  @override
  Future<void> dispose() async {
    disposeAttempts++;
    events.add(_event('player:dispose'));
    if (disposeAttempts <= failuresBeforeDispose) {
      throw StateError('fake player dispose failure');
    }
    await _snapshots.close();
  }
}
