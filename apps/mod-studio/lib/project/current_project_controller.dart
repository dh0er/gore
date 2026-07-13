import 'dart:async';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import 'managed_project_session.dart';
import 'project_controller.dart';

enum CurrentProjectKind { none, legacyFormat1, managedRevision3 }

sealed class CurrentProjectState {
  const CurrentProjectState();

  CurrentProjectKind get kind;
}

final class NoCurrentProjectState extends CurrentProjectState {
  const NoCurrentProjectState();

  @override
  CurrentProjectKind get kind => CurrentProjectKind.none;
}

/// Snapshot of the compatibility `.goremod` session.
///
/// The legacy provider graph remains owned by [ProjectSessionController]; this
/// state deliberately does not reinterpret it as a managed authoring document.
final class LegacyCurrentProjectState extends CurrentProjectState {
  const LegacyCurrentProjectState({
    required this.path,
    required this.hasUnsavedChanges,
  });

  final String? path;
  final bool hasUnsavedChanges;

  @override
  CurrentProjectKind get kind => CurrentProjectKind.legacyFormat1;
}

/// Durable identity of the exact revision-3 checkpoint owned by the app.
///
/// No diagnostics, readiness, runtime, deployment, or publication authority is
/// inferred from this checkpoint-only state.
final class ManagedRevision3CurrentProjectState extends CurrentProjectState {
  const ManagedRevision3CurrentProjectState({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    required this.requiresReopen,
  });

  final Directory root;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;
  final bool requiresReopen;

  @override
  CurrentProjectKind get kind => CurrentProjectKind.managedRevision3;
}

class CurrentProjectCoordinatorException implements Exception {
  const CurrentProjectCoordinatorException(this.message);

  final String message;

  @override
  String toString() => 'CurrentProjectCoordinatorException: $message';
}

final class NoCurrentProjectException
    extends CurrentProjectCoordinatorException {
  const NoCurrentProjectException()
    : super('there is no current project to operate on');
}

final class CurrentProjectOperationUnsupportedException
    extends CurrentProjectCoordinatorException {
  const CurrentProjectOperationUnsupportedException(super.message);
}

final class CurrentProjectCoordinatorClosedException
    extends CurrentProjectCoordinatorException {
  const CurrentProjectCoordinatorClosedException()
    : super('the current-project coordinator is shutting down or disposed');
}

/// Diagnostic evidence that one terminal lease close failed.
///
/// The coordinator deliberately retains no reference to the lease itself and
/// never retries it: production leases memoize their one permitted close
/// attempt. The error and stack trace remain available for reporting.
final class CurrentProjectCleanupFailure {
  const CurrentProjectCleanupFailure({
    required this.projectKind,
    required this.error,
    required this.stackTrace,
  });

  final CurrentProjectKind projectKind;
  final Object error;
  final StackTrace stackTrace;
}

/// Minimal ownership seam around the existing format-1 compatibility session.
abstract interface class LegacyCurrentProjectLease {
  String? get currentPath;
  bool get hasUnsavedChanges;

  Future<void> saveCurrent();
  Future<void> close();
}

/// Minimal ownership seam around one fully-opened managed revision-3 session.
abstract interface class ManagedRevision3CurrentProjectLease {
  Directory get root;
  String get projectId;
  int get projectRevision;
  AuthoringWorkingHead get head;
  bool get requiresReopen;

  Future<void> verifyCurrentHead();
  Future<void> close();
}

typedef ManagedRevision3CurrentProjectOpener =
    Future<ManagedRevision3CurrentProjectLease> Function(Directory root);

typedef LegacyCurrentProjectLeaseFactory = LegacyCurrentProjectLease Function();

/// Compatibility adapter kept intentionally narrow so the existing provider
/// graph and archive session do not have to know about managed projects.
final class ProjectSessionLegacyCurrentProjectLease
    implements LegacyCurrentProjectLease {
  ProjectSessionLegacyCurrentProjectLease(this._session);

  final ProjectSessionController _session;
  Future<void>? _closeFuture;
  bool _closed = false;

  @override
  String? get currentPath {
    _requireOpen();
    return _session.currentPath;
  }

  @override
  bool get hasUnsavedChanges {
    _requireOpen();
    return _session.hasUnsavedChanges;
  }

  @override
  Future<void> saveCurrent() async {
    _requireOpen();
    await _session.saveToCurrentPath();
  }

  @override
  Future<void> close() {
    final existing = _closeFuture;
    if (existing != null) return existing;
    _closed = true;
    return _closeFuture = Future<void>.sync(_session.newProject);
  }

  void _requireOpen() {
    if (_closed) {
      throw StateError('legacy current-project lease is already closed');
    }
  }
}

final class _ManagedRevision3SessionLease
    implements ManagedRevision3CurrentProjectLease {
  const _ManagedRevision3SessionLease(this._session);

  final ManagedRevision3AuthoringProjectSession _session;

  @override
  AuthoringWorkingHead get head => _session.head;

  @override
  String get projectId => _session.projectId;

  @override
  int get projectRevision => _session.projectRevision;

  @override
  bool get requiresReopen => _session.requiresReopen;

  @override
  Directory get root => _session.root;

  @override
  Future<void> verifyCurrentHead() => _session.verifyCurrentHead();

  @override
  Future<void> close() => _session.close();
}

/// Produces a fresh one-shot ownership token for each legacy adoption.
///
/// The underlying compatibility session remains provider-scoped, but a closed
/// lease can never be adopted again or accidentally skip a later `newProject`.
final legacyCurrentProjectLeaseFactoryProvider =
    Provider<LegacyCurrentProjectLeaseFactory>((ref) {
      final session = ref.read(projectSessionProvider);
      return () => ProjectSessionLegacyCurrentProjectLease(session);
    });

final managedRevision3CurrentProjectOpenerProvider =
    Provider<ManagedRevision3CurrentProjectOpener>((ref) {
      final store = ModFfiManagedRevision3AuthoringStore(
        ModFfi(ref.read(coreServiceProvider)),
      );
      return (root) async => _ManagedRevision3SessionLease(
        await ManagedRevision3AuthoringProjectSession.open(
          root: root,
          store: store,
        ),
      );
    });

final currentProjectCoordinatorProvider =
    StateNotifierProvider<CurrentProjectCoordinator, CurrentProjectState>((
      ref,
    ) {
      return CurrentProjectCoordinator(
        initialLegacy: ref.read(legacyCurrentProjectLeaseFactoryProvider)(),
        openManagedRevision3: ref.read(
          managedRevision3CurrentProjectOpenerProvider,
        ),
      );
    });

/// Single app-wide owner for compatibility and managed project lifetimes.
///
/// Candidate opens complete before adoption, so a failed open cannot disturb
/// the current project. Open/adopt/save/verify/close operations share one lane;
/// after adoption the previous lease is closed before the next transition can
/// run. At most one lease is authoritative. Because leases have terminal,
/// memoized close semantics, cleanup failures are retained only as diagnostics
/// and are never misleadingly retried.
final class CurrentProjectCoordinator
    extends StateNotifier<CurrentProjectState> {
  factory CurrentProjectCoordinator({
    LegacyCurrentProjectLease? initialLegacy,
    required ManagedRevision3CurrentProjectOpener openManagedRevision3,
  }) {
    final initial = initialLegacy == null
        ? null
        : _OwnedLegacyCurrentProject(initialLegacy);
    return CurrentProjectCoordinator._(
      current: initial,
      initialState: initial == null
          ? const NoCurrentProjectState()
          : _stateOf(initial),
      openManagedRevision3: openManagedRevision3,
    );
  }

  CurrentProjectCoordinator._({
    required this._current,
    required CurrentProjectState initialState,
    required this._openManagedRevision3,
  }) : super(initialState);

  final ManagedRevision3CurrentProjectOpener _openManagedRevision3;
  _OwnedCurrentProject? _current;
  final List<CurrentProjectCleanupFailure> _terminalCleanupFailures =
      <CurrentProjectCleanupFailure>[];
  Future<void> _tail = Future<void>.value();
  Future<void>? _shutdownFuture;
  bool _shutdownRequested = false;
  bool _notifierDisposed = false;

  /// Terminal close failures retained for diagnostics. No failed lease is
  /// retained or closed again because both production lease types memoize the
  /// first close attempt.
  List<CurrentProjectCleanupFailure> get terminalCleanupFailures =>
      List<CurrentProjectCleanupFailure>.unmodifiable(_terminalCleanupFailures);

  bool get isShutdownRequested => _shutdownRequested;

  /// Fully open and verify [root], then atomically make it the current project.
  /// A failed candidate open or candidate snapshot leaves the current lease and
  /// public state unchanged.
  Future<ManagedRevision3CurrentProjectState> openManagedRevision3(
    Directory root,
  ) => _enqueue(() async {
    ManagedRevision3CurrentProjectLease? candidateLease;
    var adopted = false;
    try {
      candidateLease = await _openManagedRevision3(root);
      final candidate = _OwnedManagedRevision3CurrentProject(candidateLease);
      final candidateState = _stateOf(candidate);
      await _adopt(candidate, candidateState);
      adopted = true;
      return candidateState as ManagedRevision3CurrentProjectState;
    } catch (error, stackTrace) {
      if (candidateLease != null && !adopted) {
        await _closeUnadopted(
          _OwnedManagedRevision3CurrentProject(candidateLease),
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Adopt an independently-opened compatibility lease.
  ///
  /// Ownership transfers when this invocation reaches the serialized lane.
  Future<LegacyCurrentProjectState> adoptLegacy(
    LegacyCurrentProjectLease lease,
  ) => _enqueue(() async {
    final current = _current;
    if (current is _OwnedLegacyCurrentProject &&
        identical(current.lease, lease)) {
      final refreshed = _stateOf(current) as LegacyCurrentProjectState;
      _publish(refreshed);
      return refreshed;
    }
    final candidate = _OwnedLegacyCurrentProject(lease);
    try {
      final candidateState = _stateOf(candidate) as LegacyCurrentProjectState;
      await _adopt(candidate, candidateState);
      return candidateState;
    } catch (error, stackTrace) {
      if (!identical(_current, candidate)) await _closeUnadopted(candidate);
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Ctrl+S-sized durability action for the active backend.
  ///
  /// Compatibility projects write their captured provider snapshot. Managed
  /// revision-3 projects already publish every semantic transaction, so this
  /// performs only an exact-head, full-asset reopen verification.
  Future<CurrentProjectState> saveCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    late CurrentProjectState refreshed;
    try {
      switch (current) {
        case _OwnedLegacyCurrentProject(:final lease):
          await lease.saveCurrent();
        case _OwnedManagedRevision3CurrentProject(:final lease):
          await lease.verifyCurrentHead();
      }
    } finally {
      refreshed = _refreshCurrentIfUnchanged(current);
    }
    return refreshed;
  });

  /// Read-only exact-head verification for a managed revision-3 current
  /// project. Legacy archives have no equivalent full-reopen contract.
  Future<ManagedRevision3CurrentProjectState>
  verifyCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'exact current-head verification is available only for managed revision-3 projects',
      );
    }
    late ManagedRevision3CurrentProjectState refreshed;
    try {
      await current.lease.verifyCurrentHead();
    } finally {
      refreshed =
          _refreshCurrentIfUnchanged(current)
              as ManagedRevision3CurrentProjectState;
    }
    return refreshed;
  });

  /// Detach and close the current lease in the operation lane.
  Future<void> closeCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) return;
    _current = null;
    _publish(const NoCurrentProjectState());
    try {
      await _closeOwned(current);
    } catch (error, stackTrace) {
      _recordCleanupFailure(current, error, stackTrace);
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Stop accepting work, drain accepted transitions, and close every lease.
  /// Idempotent and safe to await in tests or an orderly application shutdown.
  Future<void> shutdown() {
    final existing = _shutdownFuture;
    if (existing != null) return existing;
    _shutdownRequested = true;
    final result = _tail.then((_) async {
      final closing = _current;
      _current = null;
      _publish(const NoCurrentProjectState());
      if (closing != null) {
        try {
          await _closeOwned(closing);
        } catch (error, stackTrace) {
          _recordCleanupFailure(closing, error, stackTrace);
          Error.throwWithStackTrace(error, stackTrace);
        }
      }
    });
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    _shutdownFuture = result;
    return result;
  }

  Future<void> _adopt(
    _OwnedCurrentProject candidate,
    CurrentProjectState candidateState,
  ) async {
    final previous = _current;
    _current = candidate;
    _publish(candidateState);
    if (previous != null) await _retire(previous);
  }

  Future<void> _retire(_OwnedCurrentProject owned) async {
    try {
      await _closeOwned(owned);
    } catch (error, stackTrace) {
      _recordCleanupFailure(owned, error, stackTrace);
    }
  }

  Future<void> _closeUnadopted(_OwnedCurrentProject owned) async {
    try {
      await _closeOwned(owned);
    } catch (error, stackTrace) {
      _recordCleanupFailure(owned, error, stackTrace);
    }
  }

  void _recordCleanupFailure(
    _OwnedCurrentProject owned,
    Object error,
    StackTrace stackTrace,
  ) {
    _terminalCleanupFailures.add(
      CurrentProjectCleanupFailure(
        projectKind: switch (owned) {
          _OwnedLegacyCurrentProject() => CurrentProjectKind.legacyFormat1,
          _OwnedManagedRevision3CurrentProject() =>
            CurrentProjectKind.managedRevision3,
        },
        error: error,
        stackTrace: stackTrace,
      ),
    );
  }

  CurrentProjectState _refreshCurrentIfUnchanged(
    _OwnedCurrentProject expected,
  ) {
    if (!identical(_current, expected)) {
      throw const CurrentProjectCoordinatorException(
        'current project changed inside the serialized operation lane',
      );
    }
    final refreshed = _stateOf(expected);
    _publish(refreshed);
    return refreshed;
  }

  void _publish(CurrentProjectState next) {
    if (!_notifierDisposed) state = next;
  }

  Future<T> _enqueue<T>(Future<T> Function() operation) {
    if (_shutdownRequested) {
      return Future<T>.error(const CurrentProjectCoordinatorClosedException());
    }
    final result = _tail.then((_) => operation());
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }

  @override
  void dispose() {
    _notifierDisposed = true;
    unawaited(
      shutdown().then<void>((_) {}, onError: (Object _, StackTrace _) {}),
    );
    super.dispose();
  }
}

sealed class _OwnedCurrentProject {
  const _OwnedCurrentProject();
}

final class _OwnedLegacyCurrentProject extends _OwnedCurrentProject {
  const _OwnedLegacyCurrentProject(this.lease);

  final LegacyCurrentProjectLease lease;
}

final class _OwnedManagedRevision3CurrentProject extends _OwnedCurrentProject {
  const _OwnedManagedRevision3CurrentProject(this.lease);

  final ManagedRevision3CurrentProjectLease lease;
}

CurrentProjectState _stateOf(_OwnedCurrentProject owned) => switch (owned) {
  _OwnedLegacyCurrentProject(:final lease) => LegacyCurrentProjectState(
    path: lease.currentPath,
    hasUnsavedChanges: lease.hasUnsavedChanges,
  ),
  _OwnedManagedRevision3CurrentProject(:final lease) =>
    ManagedRevision3CurrentProjectState(
      root: lease.root,
      projectId: lease.projectId,
      projectRevision: lease.projectRevision,
      head: lease.head,
      requiresReopen: lease.requiresReopen,
    ),
};

Future<void> _closeOwned(_OwnedCurrentProject owned) => switch (owned) {
  _OwnedLegacyCurrentProject(:final lease) => lease.close(),
  _OwnedManagedRevision3CurrentProject(:final lease) => lease.close(),
};
