import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter_soloud/flutter_soloud.dart';

import 'revision3_voice_take_preview_authoring.dart';

/// Process-lifetime owner for the rare case where terminal modal teardown has
/// already released native playback but bounded filesystem cleanup still
/// fails. The next preview controller adopts and retries these obligations.
/// The obligation stays opaque here; its path/root is never rendered or
/// persisted.
final Set<Revision3VoiceTakePreviewCleanupObligation>
_deferredRevision3VoicePreviewCleanup =
    <Revision3VoiceTakePreviewCleanupObligation>{};
final Map<
  Revision3VoiceTakePreviewCleanupObligation,
  _Revision3VoicePreviewTerminalFailure
>
_deferredRevision3VoicePreviewTerminalFailures =
    HashMap<
      Revision3VoiceTakePreviewCleanupObligation,
      _Revision3VoicePreviewTerminalFailure
    >.identity();

/// Player-facing state only. Project identities, CAS seals and temporary paths
/// deliberately never enter this snapshot.
enum Revision3VoiceTakePreviewPlaybackPhase {
  idle,
  preparing,
  playing,
  paused,
  completed,
  failed,
}

enum Revision3VoiceTakePreviewFailureKind {
  materialize,
  playback,
  cleanup,
  staleCheckpoint,
  requiresReopen,
}

final class _Revision3VoicePreviewTerminalFailure {
  const _Revision3VoicePreviewTerminalFailure({
    required this.kind,
    required this.takeKey,
  });

  final Revision3VoiceTakePreviewFailureKind kind;
  final String takeKey;
}

@immutable
final class Revision3VoiceTakePreviewPlaybackSnapshot {
  const Revision3VoiceTakePreviewPlaybackSnapshot({
    required this.phase,
    this.activeTakeKey,
    this.position = Duration.zero,
    this.duration = Duration.zero,
    this.failure,
  });

  const Revision3VoiceTakePreviewPlaybackSnapshot.idle()
    : this(phase: Revision3VoiceTakePreviewPlaybackPhase.idle);

  final Revision3VoiceTakePreviewPlaybackPhase phase;
  final String? activeTakeKey;
  final Duration position;
  final Duration duration;
  final Revision3VoiceTakePreviewFailureKind? failure;

  bool isActive(String takeKey) => activeTakeKey == takeKey;

  bool get isBusy => phase == Revision3VoiceTakePreviewPlaybackPhase.preparing;

  bool get isPlaying => phase == Revision3VoiceTakePreviewPlaybackPhase.playing;
}

/// Small adapter around the native materialization capability. Keeping this
/// type in the playback layer makes all controller tests independent of FFI.
final class Revision3VoiceTakePreviewPlaybackLease
    implements Revision3VoiceTakePreviewCleanupObligation {
  factory Revision3VoiceTakePreviewPlaybackLease({
    required String path,
    required bool Function() isClosed,
    required Future<void> Function() close,
  }) => Revision3VoiceTakePreviewPlaybackLease._(path, isClosed, close);

  Revision3VoiceTakePreviewPlaybackLease._(
    this.path,
    this._isClosed,
    this._close,
  );

  final String path;
  final bool Function() _isClosed;
  final Future<void> Function() _close;

  bool get isClosed => _isClosed();

  Future<void> close() => _close();

  @override
  bool get isCleaned => isClosed;

  @override
  Future<void> retryCleanup() => close();
}

typedef Revision3VoiceTakePreviewMaterializer =
    Future<Revision3VoiceTakePreviewPlaybackLease> Function();

@immutable
final class Revision3VoiceTakePreviewPlayerSnapshot {
  const Revision3VoiceTakePreviewPlayerSnapshot({
    required this.phase,
    this.position = Duration.zero,
    this.duration = Duration.zero,
  });

  const Revision3VoiceTakePreviewPlayerSnapshot.idle()
    : this(phase: Revision3VoiceTakePreviewPlaybackPhase.idle);

  final Revision3VoiceTakePreviewPlaybackPhase phase;
  final Duration position;
  final Duration duration;
}

/// Narrow seam used by normal widget tests. They inject a fake implementation
/// and therefore never initialize a real Windows audio device.
abstract interface class Revision3VoiceTakePreviewPlayer {
  Revision3VoiceTakePreviewPlayerSnapshot get snapshot;
  Stream<Revision3VoiceTakePreviewPlayerSnapshot> get snapshots;

  Future<void> open(String path);
  Future<void> play();
  Future<void> pause();
  Future<void> seek(Duration position);

  /// Must stop playback and release every native handle to the open file.
  Future<void> stopAndUnload();

  Future<void> dispose();
}

/// Process-lifetime ownership for a player that could not prove terminal
/// unload while its active temporary lease still had to remain alive.
///
/// It is intentionally also a cleanup obligation so the next controller can
/// adopt it through the same opaque retry channel. The lease is never touched
/// until either [Revision3VoiceTakePreviewPlayer.stopAndUnload] or
/// [Revision3VoiceTakePreviewPlayer.dispose] has proved that the old player no
/// longer owns a native file handle.
final class _DeferredRevision3VoicePreviewTeardown
    implements Revision3VoiceTakePreviewCleanupObligation {
  _DeferredRevision3VoicePreviewTeardown({
    required this.player,
    required this.lease,
    required this.playerUnloaded,
    required this.playerDisposed,
  });

  final Revision3VoiceTakePreviewPlayer player;
  final Revision3VoiceTakePreviewPlaybackLease? lease;
  bool playerUnloaded;
  bool playerDisposed;
  Future<void>? _retryFuture;

  @override
  bool get isCleaned => playerDisposed && (lease?.isCleaned ?? true);

  @override
  Future<void> retryCleanup() {
    if (isCleaned) return Future<void>.value();
    final inFlight = _retryFuture;
    if (inFlight != null) return inFlight;

    late final Future<void> attempt;
    attempt = _retryExact().then<void>(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        if (identical(_retryFuture, attempt)) _retryFuture = null;
        final wrapped = error is Revision3VoiceTakePreviewCleanupException
            ? error
            : Revision3VoiceTakePreviewCleanupException(error);
        Error.throwWithStackTrace(wrapped, stackTrace);
      },
    );
    _retryFuture = attempt;
    return attempt;
  }

  Future<void> _retryExact() async {
    if (!playerUnloaded) {
      try {
        await player.stopAndUnload();
        playerUnloaded = true;
      } catch (_) {
        // A successful terminal player dispose is the stronger unload proof.
        await player.dispose();
        playerUnloaded = true;
        playerDisposed = true;
      }
    }

    final retainedLease = lease;
    if (retainedLease != null && !retainedLease.isCleaned) {
      await retainedLease.retryCleanup();
    }

    if (!playerDisposed) {
      await player.dispose();
      playerDisposed = true;
    }
  }
}

/// Serializes preview replacement so that the previous native file handle is
/// stopped and unloaded before its managed temporary capability is closed.
/// Newer requests supersede older materializations without ever opening the
/// older result.
final class Revision3VoiceTakePreviewPlaybackController implements Listenable {
  factory Revision3VoiceTakePreviewPlaybackController({
    required Revision3VoiceTakePreviewPlayer player,
  }) => Revision3VoiceTakePreviewPlaybackController._(player);

  Revision3VoiceTakePreviewPlaybackController._(this._player) {
    _cleanupObligations.addAll(_deferredRevision3VoicePreviewCleanup);
    for (final obligation in _deferredRevision3VoicePreviewCleanup) {
      final terminal =
          _deferredRevision3VoicePreviewTerminalFailures[obligation];
      if (terminal != null) {
        _cleanupTerminalFailures[obligation] = terminal;
      }
    }
    _deferredRevision3VoicePreviewCleanup.clear();
    _deferredRevision3VoicePreviewTerminalFailures.clear();
    _playerSubscription = _player.snapshots.listen(
      _onPlayerSnapshot,
      onError: (_, _) => _onPlayerError(),
    );
  }

  factory Revision3VoiceTakePreviewPlaybackController.standard() =>
      Revision3VoiceTakePreviewPlaybackController(
        player: Revision3VoiceTakePreviewSoLoudPlayer(),
      );

  final Revision3VoiceTakePreviewPlayer _player;
  final Set<VoidCallback> _listeners = <VoidCallback>{};
  late final StreamSubscription<Revision3VoiceTakePreviewPlayerSnapshot>
  _playerSubscription;
  Revision3VoiceTakePreviewPlaybackLease? _lease;
  final List<Revision3VoiceTakePreviewCleanupObligation> _cleanupObligations =
      <Revision3VoiceTakePreviewCleanupObligation>[];
  final Map<
    Revision3VoiceTakePreviewCleanupObligation,
    _Revision3VoicePreviewTerminalFailure
  >
  _cleanupTerminalFailures =
      HashMap<
        Revision3VoiceTakePreviewCleanupObligation,
        _Revision3VoicePreviewTerminalFailure
      >.identity();
  Revision3VoiceTakePreviewPlaybackSnapshot _snapshot =
      const Revision3VoiceTakePreviewPlaybackSnapshot.idle();
  Future<void> _operationTail = Future<void>.value();
  Future<void>? _disposeFuture;
  int _requestEpoch = 0;
  int _seekSequence = 0;
  bool _acceptPlayerEvents = false;
  bool _disposed = false;

  Revision3VoiceTakePreviewPlaybackSnapshot get snapshot => _snapshot;

  @override
  void addListener(VoidCallback listener) {
    if (!_disposed) _listeners.add(listener);
  }

  @override
  void removeListener(VoidCallback listener) => _listeners.remove(listener);

  Future<void> preview({
    required String takeKey,
    required Revision3VoiceTakePreviewMaterializer materialize,
  }) {
    if (_disposed) return Future<void>.value();
    final epoch = ++_requestEpoch;
    _acceptPlayerEvents = false;
    _setSnapshot(
      Revision3VoiceTakePreviewPlaybackSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.preparing,
        activeTakeKey: takeKey,
      ),
    );
    return _enqueue(() => _runPreview(epoch, takeKey, materialize));
  }

  Future<void> _runPreview(
    int epoch,
    String takeKey,
    Revision3VoiceTakePreviewMaterializer materialize,
  ) async {
    if (!_isCurrent(epoch)) return;
    var fallbackFailure = Revision3VoiceTakePreviewFailureKind.cleanup;
    try {
      final recoveredTerminal = await _unloadAndRelease();
      if (!_isCurrent(epoch)) return;
      if (recoveredTerminal != null) {
        _setFailure(recoveredTerminal.takeKey, recoveredTerminal.kind);
        return;
      }

      fallbackFailure = Revision3VoiceTakePreviewFailureKind.materialize;
      final lease = await materialize();
      if (!_isCurrent(epoch)) {
        await _closeSupersededLease(lease);
        return;
      }
      _lease = lease;
      fallbackFailure = Revision3VoiceTakePreviewFailureKind.cleanup;
      try {
        await _player.open(lease.path);
      } catch (_) {
        try {
          await _unloadAndRelease();
          if (_isCurrent(epoch)) {
            _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.playback);
          }
        } catch (_) {
          if (_isCurrent(epoch)) {
            _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.cleanup);
          }
        }
        return;
      }
      if (!_isCurrent(epoch)) {
        await _unloadAndRelease();
        return;
      }
      _acceptPlayerEvents = true;
      _applyPlayerSnapshot(takeKey, _player.snapshot);
    } on Revision3VoiceTakePreviewStaleCheckpointException catch (error) {
      await _retainMaterializationCleanup(
        epoch: epoch,
        takeKey: takeKey,
        obligation: error.cleanupObligation,
        failureAfterCleanup:
            Revision3VoiceTakePreviewFailureKind.staleCheckpoint,
      );
    } on Revision3VoiceTakePreviewRequiresReopenException catch (error) {
      await _retainMaterializationCleanup(
        epoch: epoch,
        takeKey: takeKey,
        obligation: error.cleanupObligation,
        failureAfterCleanup:
            Revision3VoiceTakePreviewFailureKind.requiresReopen,
      );
    } on Revision3VoiceTakePreviewCleanupObligation catch (obligation) {
      await _retainMaterializationCleanup(
        epoch: epoch,
        takeKey: takeKey,
        obligation: obligation,
        failureAfterCleanup: Revision3VoiceTakePreviewFailureKind.materialize,
      );
    } on Revision3VoiceTakePreviewCleanupException {
      if (_isCurrent(epoch)) {
        _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.cleanup);
      }
    } catch (_) {
      if (_isCurrent(epoch)) {
        _setFailure(takeKey, fallbackFailure);
      }
    }
  }

  Future<void> play() {
    final epoch = _requestEpoch;
    final takeKey = _snapshot.activeTakeKey;
    final lease = _lease;
    if (_disposed || takeKey == null || lease == null) {
      return Future<void>.value();
    }
    return _enqueue(() async {
      if (!_isTransportCurrent(epoch, takeKey, lease)) return;
      try {
        await _player.play();
        if (!_isTransportCurrent(epoch, takeKey, lease)) return;
        _acceptPlayerEvents = true;
        _applyPlayerSnapshot(takeKey, _player.snapshot);
      } catch (_) {
        if (_isTransportCurrent(epoch, takeKey, lease)) {
          _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.playback);
        }
      }
    });
  }

  Future<void> pause() {
    final epoch = _requestEpoch;
    final takeKey = _snapshot.activeTakeKey;
    final lease = _lease;
    if (_disposed || takeKey == null || lease == null) {
      return Future<void>.value();
    }
    return _enqueue(() async {
      if (!_isTransportCurrent(epoch, takeKey, lease)) return;
      try {
        await _player.pause();
        if (_isTransportCurrent(epoch, takeKey, lease)) {
          _applyPlayerSnapshot(takeKey, _player.snapshot);
        }
      } catch (_) {
        if (_isTransportCurrent(epoch, takeKey, lease)) {
          _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.playback);
        }
      }
    });
  }

  Future<void> seek(Duration position) {
    final epoch = _requestEpoch;
    final takeKey = _snapshot.activeTakeKey;
    final lease = _lease;
    if (_disposed || takeKey == null || lease == null) {
      return Future<void>.value();
    }
    final sequence = ++_seekSequence;
    final duration = _snapshot.duration;
    final clamped = position < Duration.zero
        ? Duration.zero
        : duration > Duration.zero && position > duration
        ? duration
        : position;
    return _enqueue(() async {
      // Slider drags may enqueue many values before the operation tail runs.
      // Only the newest value for the still-exact same take/lease is useful.
      if (sequence != _seekSequence ||
          !_isTransportCurrent(epoch, takeKey, lease)) {
        return;
      }
      try {
        await _player.seek(clamped);
        if (_isTransportCurrent(epoch, takeKey, lease)) {
          _applyPlayerSnapshot(takeKey, _player.snapshot);
        }
      } catch (_) {
        if (_isTransportCurrent(epoch, takeKey, lease)) {
          _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.playback);
        }
      }
    });
  }

  Future<void> stop() {
    if (_disposed) return Future<void>.value();
    final epoch = ++_requestEpoch;
    final takeKey = _snapshot.activeTakeKey;
    final failureBeforeStop = _snapshot.failure;
    final terminalBeforeStop =
        takeKey != null &&
            (failureBeforeStop ==
                    Revision3VoiceTakePreviewFailureKind.requiresReopen ||
                failureBeforeStop ==
                    Revision3VoiceTakePreviewFailureKind.staleCheckpoint)
        ? _Revision3VoicePreviewTerminalFailure(
            kind: failureBeforeStop!,
            takeKey: takeKey,
          )
        : null;
    _acceptPlayerEvents = false;
    _setSnapshot(const Revision3VoiceTakePreviewPlaybackSnapshot.idle());
    return _enqueue(() async {
      if (!_isCurrent(epoch)) return;
      try {
        final recoveredTerminal = await _unloadAndRelease();
        final terminal = recoveredTerminal ?? terminalBeforeStop;
        if (_isCurrent(epoch) && terminal != null) {
          _setFailure(terminal.takeKey, terminal.kind);
        }
      } catch (_) {
        if (_isCurrent(epoch)) {
          _setSnapshot(
            Revision3VoiceTakePreviewPlaybackSnapshot(
              phase: Revision3VoiceTakePreviewPlaybackPhase.failed,
              activeTakeKey: takeKey,
              failure: Revision3VoiceTakePreviewFailureKind.cleanup,
            ),
          );
        }
      }
    });
  }

  Future<_Revision3VoicePreviewTerminalFailure?> _unloadAndRelease() async {
    await _player.stopAndUnload();
    final lease = _lease;
    if (lease != null) {
      try {
        await lease.close();
      } catch (_) {
        if (!lease.isCleaned && !_cleanupObligations.contains(lease)) {
          _cleanupObligations.add(lease);
        }
        rethrow;
      } finally {
        if (lease.isClosed && identical(_lease, lease)) _lease = null;
      }
    }
    return _retryCleanupObligations();
  }

  Future<_Revision3VoicePreviewTerminalFailure?>
  _retryCleanupObligations() async {
    Object? firstError;
    StackTrace? firstStackTrace;
    _Revision3VoicePreviewTerminalFailure? recoveredTerminal;
    final cleaned = <Revision3VoiceTakePreviewCleanupObligation>[];
    for (final obligation
        in List<Revision3VoiceTakePreviewCleanupObligation>.of(
          _cleanupObligations,
        )) {
      if (!obligation.isCleaned) {
        try {
          await obligation.retryCleanup();
        } catch (error, stackTrace) {
          firstError ??= error;
          firstStackTrace ??= stackTrace;
        }
      }
      if (obligation.isCleaned) {
        cleaned.add(obligation);
        final terminal = _cleanupTerminalFailures[obligation];
        if (terminal != null &&
            (recoveredTerminal == null ||
                terminal.kind ==
                    Revision3VoiceTakePreviewFailureKind.requiresReopen)) {
          recoveredTerminal = terminal;
        }
      }
    }
    if (firstError != null) {
      for (final obligation in cleaned) {
        if (!_cleanupTerminalFailures.containsKey(obligation)) {
          _cleanupObligations.remove(obligation);
        }
      }
      Error.throwWithStackTrace(firstError, firstStackTrace!);
    }
    for (final obligation in cleaned) {
      _cleanupObligations.remove(obligation);
      _cleanupTerminalFailures.remove(obligation);
    }
    return recoveredTerminal;
  }

  Future<void> _retainMaterializationCleanup({
    required int epoch,
    required String takeKey,
    required Revision3VoiceTakePreviewCleanupObligation? obligation,
    required Revision3VoiceTakePreviewFailureKind failureAfterCleanup,
  }) async {
    if (obligation != null && !_cleanupObligations.contains(obligation)) {
      _cleanupObligations.add(obligation);
    }
    if (obligation != null &&
        (failureAfterCleanup ==
                Revision3VoiceTakePreviewFailureKind.requiresReopen ||
            failureAfterCleanup ==
                Revision3VoiceTakePreviewFailureKind.staleCheckpoint)) {
      _cleanupTerminalFailures[obligation] =
          _Revision3VoicePreviewTerminalFailure(
            kind: failureAfterCleanup,
            takeKey: takeKey,
          );
    }
    _Revision3VoicePreviewTerminalFailure? recoveredTerminal;
    try {
      recoveredTerminal = await _retryCleanupObligations();
    } catch (_) {
      // Retained and retried by replacement, Stop, and Dispose.
    }
    if (_isCurrent(epoch)) {
      _setFailure(
        recoveredTerminal?.takeKey ?? takeKey,
        _cleanupObligations.isEmpty
            ? recoveredTerminal?.kind ?? failureAfterCleanup
            : Revision3VoiceTakePreviewFailureKind.cleanup,
      );
    }
  }

  Future<void> _closeSupersededLease(
    Revision3VoiceTakePreviewPlaybackLease lease,
  ) async {
    try {
      await lease.close();
    } catch (_) {
      if (!lease.isCleaned && !_cleanupObligations.contains(lease)) {
        _cleanupObligations.add(lease);
      }
    }
  }

  void _onPlayerSnapshot(Revision3VoiceTakePreviewPlayerSnapshot player) {
    final takeKey = _snapshot.activeTakeKey;
    if (!_disposed &&
        _acceptPlayerEvents &&
        takeKey != null &&
        _lease != null) {
      _applyPlayerSnapshot(takeKey, player);
    }
  }

  void _onPlayerError() {
    final takeKey = _snapshot.activeTakeKey;
    if (!_disposed && _acceptPlayerEvents && takeKey != null) {
      _setFailure(takeKey, Revision3VoiceTakePreviewFailureKind.playback);
    }
  }

  void _applyPlayerSnapshot(
    String takeKey,
    Revision3VoiceTakePreviewPlayerSnapshot player,
  ) {
    if (_disposed) return;
    _setSnapshot(
      Revision3VoiceTakePreviewPlaybackSnapshot(
        phase: player.phase,
        activeTakeKey: takeKey,
        position: player.position,
        duration: player.duration,
      ),
    );
  }

  void _setFailure(
    String? takeKey,
    Revision3VoiceTakePreviewFailureKind failure,
  ) {
    _acceptPlayerEvents = false;
    _setSnapshot(
      Revision3VoiceTakePreviewPlaybackSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.failed,
        activeTakeKey: takeKey,
        position: _snapshot.position,
        duration: _snapshot.duration,
        failure: failure,
      ),
    );
  }

  bool _isCurrent(int epoch) => !_disposed && epoch == _requestEpoch;

  bool _isTransportCurrent(
    int epoch,
    String takeKey,
    Revision3VoiceTakePreviewPlaybackLease lease,
  ) =>
      _isCurrent(epoch) &&
      _snapshot.activeTakeKey == takeKey &&
      identical(_lease, lease);

  Future<void> _enqueue(Future<void> Function() operation) {
    final result = _operationTail.then((_) => operation());
    _operationTail = result.catchError((_) {});
    return result;
  }

  void _setSnapshot(Revision3VoiceTakePreviewPlaybackSnapshot value) {
    if (_disposed) return;
    _snapshot = value;
    for (final listener in List<VoidCallback>.of(_listeners)) {
      listener();
    }
  }

  /// Fully asynchronous teardown. It stops/unloads native playback before the
  /// temporary capability is closed, including when normal stop fails.
  Future<void> dispose() => _disposeFuture ??= _dispose();

  Future<void> _dispose() async {
    _disposed = true;
    _requestEpoch++;
    _seekSequence++;
    _acceptPlayerEvents = false;
    try {
      // Cancellation detaches the listener synchronously, while its returned
      // Future can wait for a later event-loop turn. Awaiting that Future from
      // a closing modal deadlocks Flutter's pump-and-settle action lifecycle.
      // The disposed gate already rejects every late event, and player dispose
      // below closes the source stream itself.
      unawaited(_playerSubscription.cancel().catchError((Object _) {}));
    } catch (_) {
      // Player teardown remains the authoritative native-resource boundary.
    }
    await _enqueue(() async {
      var unloaded = false;
      var playerDisposed = false;
      try {
        await _player.stopAndUnload();
        unloaded = true;
      } catch (_) {
        // Player disposal is the second, stronger unload boundary.
      }
      if (!unloaded) {
        try {
          await _player.dispose();
          unloaded = true;
          playerDisposed = true;
        } catch (_) {
          // Never delete a capability while a native handle may still exist.
        }
      }
      if (!unloaded) {
        final lease = _lease;
        if (lease != null) {
          _cleanupObligations.remove(lease);
          _lease = null;
        }
        _cleanupObligations.add(
          _DeferredRevision3VoicePreviewTeardown(
            player: _player,
            lease: lease,
            playerUnloaded: false,
            playerDisposed: false,
          ),
        );
        return;
      }
      if (unloaded) {
        final lease = _lease;
        if (lease != null) {
          try {
            await lease.close();
          } catch (_) {
            if (!lease.isCleaned && !_cleanupObligations.contains(lease)) {
              _cleanupObligations.add(lease);
            }
          } finally {
            if (lease.isClosed && identical(_lease, lease)) _lease = null;
          }
        }
        try {
          await _retryCleanupObligations();
        } catch (_) {
          // Handed to the process-lifetime owner below.
        }
      }
      if (!playerDisposed) {
        try {
          await _player.dispose();
          playerDisposed = true;
        } catch (_) {
          // The preview file is already unloaded, but the player object still
          // needs a process-lifetime owner until terminal dispose succeeds.
          _cleanupObligations.add(
            _DeferredRevision3VoicePreviewTeardown(
              player: _player,
              lease: null,
              playerUnloaded: true,
              playerDisposed: false,
            ),
          );
        }
      }
    });
    for (final obligation in _cleanupObligations) {
      if (!obligation.isCleaned) {
        _deferredRevision3VoicePreviewCleanup.add(obligation);
        final terminal = _cleanupTerminalFailures[obligation];
        if (terminal != null) {
          _deferredRevision3VoicePreviewTerminalFailures[obligation] = terminal;
        }
      }
    }
    _cleanupObligations.clear();
    _cleanupTerminalFailures.clear();
    _listeners.clear();
  }
}

Future<void>? _soLoudInitialization;

Future<void> _ensureRevision3VoicePreviewSoLoudInitialized() {
  if (SoLoud.instance.isInitialized) return Future<void>.value();
  final pending = _soLoudInitialization;
  if (pending != null) return pending;
  final initialized = SoLoud.instance.init(automaticCleanup: false);
  _soLoudInitialization = initialized;
  return initialized.whenComplete(() {
    _soLoudInitialization = null;
  });
}

/// Production Ogg player. Initialization is deliberately lazy so ordinary
/// widget tests and users who never press Preview do not open an audio device.
final class Revision3VoiceTakePreviewSoLoudPlayer
    implements Revision3VoiceTakePreviewPlayer {
  final StreamController<Revision3VoiceTakePreviewPlayerSnapshot> _snapshots =
      StreamController<Revision3VoiceTakePreviewPlayerSnapshot>.broadcast();
  Revision3VoiceTakePreviewPlayerSnapshot _snapshot =
      const Revision3VoiceTakePreviewPlayerSnapshot.idle();
  AudioSource? _source;
  SoundHandle? _handle;
  StreamSubscription<StreamSoundEvent>? _soundSubscription;
  Timer? _positionTimer;
  Future<void>? _disposeFuture;
  bool _disposed = false;

  @override
  Revision3VoiceTakePreviewPlayerSnapshot get snapshot => _snapshot;

  @override
  Stream<Revision3VoiceTakePreviewPlayerSnapshot> get snapshots =>
      _snapshots.stream;

  @override
  Future<void> open(String path) async {
    if (_disposed) throw StateError('Preview player is disposed.');
    await _ensureRevision3VoicePreviewSoLoudInitialized();
    if (_source != null || _handle != null) {
      throw StateError('Unload the previous preview before opening another.');
    }
    final source = await SoLoud.instance.loadFile(
      path,
      mode: LoadMode.disk,
      autoDispose: false,
    );
    _source = source;
    try {
      _soundSubscription = source.soundEvents.listen(_onSoundEvent);
      final duration = SoLoud.instance.getLength(source);
      final handle = SoLoud.instance.play(source);
      _handle = handle;
      _emit(
        Revision3VoiceTakePreviewPlayerSnapshot(
          phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
          duration: duration,
        ),
      );
      _startPositionTimer();
    } catch (_) {
      await _releaseSourceAfterOpenFailure();
      rethrow;
    }
  }

  @override
  Future<void> play() async {
    final source = _source;
    if (_disposed || source == null) return;
    final current = _handle;
    if (current == null || !SoLoud.instance.getIsValidVoiceHandle(current)) {
      _handle = SoLoud.instance.play(source);
    } else {
      SoLoud.instance.setPause(current, false);
    }
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
        position: _currentPosition(),
        duration: SoLoud.instance.getLength(source),
      ),
    );
    _startPositionTimer();
  }

  @override
  Future<void> pause() async {
    final handle = _handle;
    if (_disposed || handle == null) return;
    if (SoLoud.instance.getIsValidVoiceHandle(handle)) {
      SoLoud.instance.setPause(handle, true);
    }
    _positionTimer?.cancel();
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.paused,
        position: _currentPosition(),
        duration: _source == null
            ? Duration.zero
            : SoLoud.instance.getLength(_source!),
      ),
    );
  }

  @override
  Future<void> seek(Duration position) async {
    final handle = _handle;
    if (_disposed || handle == null) return;
    if (SoLoud.instance.getIsValidVoiceHandle(handle)) {
      SoLoud.instance.seek(handle, position);
    }
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: _snapshot.phase,
        position: position,
        duration: _snapshot.duration,
      ),
    );
  }

  @override
  Future<void> stopAndUnload() async {
    _positionTimer?.cancel();
    _positionTimer = null;
    final handle = _handle;
    if (handle != null) {
      if (!SoLoud.instance.isInitialized ||
          !SoLoud.instance.getIsValidVoiceHandle(handle)) {
        if (_handle == handle) _handle = null;
      } else {
        await SoLoud.instance.stop(handle);
        if (_handle == handle) _handle = null;
      }
    }
    final subscription = _soundSubscription;
    if (subscription != null) {
      // Detachment is synchronous. Its Future can wait for a later event-loop
      // turn, so source disposal below is the actual awaited release boundary.
      unawaited(subscription.cancel().catchError((Object _) {}));
      if (identical(_soundSubscription, subscription)) {
        _soundSubscription = null;
      }
    }
    final source = _source;
    if (source != null) {
      if (SoLoud.instance.isInitialized) {
        await SoLoud.instance.disposeSource(source);
      }
      if (identical(_source, source)) _source = null;
    }
    _emit(const Revision3VoiceTakePreviewPlayerSnapshot.idle());
  }

  Future<void> _releaseSourceAfterOpenFailure() async {
    try {
      await stopAndUnload();
    } catch (_) {
      // Preserve the original decoder/player error.
    }
  }

  void _onSoundEvent(StreamSoundEvent event) {
    if (_disposed || event.event != SoundEventType.handleIsNoMoreValid) return;
    final handle = _handle;
    if (handle == null || event.handle != handle) return;
    _positionTimer?.cancel();
    _handle = null;
    _emit(
      Revision3VoiceTakePreviewPlayerSnapshot(
        phase: Revision3VoiceTakePreviewPlaybackPhase.completed,
        position: _snapshot.duration,
        duration: _snapshot.duration,
      ),
    );
  }

  void _startPositionTimer() {
    _positionTimer?.cancel();
    _positionTimer = Timer.periodic(const Duration(milliseconds: 150), (_) {
      if (_disposed ||
          _snapshot.phase != Revision3VoiceTakePreviewPlaybackPhase.playing) {
        return;
      }
      _emit(
        Revision3VoiceTakePreviewPlayerSnapshot(
          phase: Revision3VoiceTakePreviewPlaybackPhase.playing,
          position: _currentPosition(),
          duration: _snapshot.duration,
        ),
      );
    });
  }

  Duration _currentPosition() {
    final handle = _handle;
    if (handle == null ||
        !SoLoud.instance.isInitialized ||
        !SoLoud.instance.getIsValidVoiceHandle(handle)) {
      return _snapshot.position;
    }
    return SoLoud.instance.getPosition(handle);
  }

  void _emit(Revision3VoiceTakePreviewPlayerSnapshot value) {
    _snapshot = value;
    if (!_disposed && !_snapshots.isClosed) _snapshots.add(value);
  }

  @override
  Future<void> dispose() {
    if (_disposed) return Future<void>.value();
    return _disposeFuture ??= _disposeExact().whenComplete(() {
      if (!_disposed) _disposeFuture = null;
    });
  }

  Future<void> _disposeExact() async {
    await stopAndUnload();
    _disposed = true;
    await _snapshots.close();
  }
}
