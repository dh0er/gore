import 'dart:async';

import 'package:flutter_riverpod/legacy.dart';

import '../../core/mgr_ffi.dart';
import '../../core/providers.dart';
import '../../library/domain/models.dart';

/// Deployment status of the game install relative to the current loadout,
/// plus the outcome of the last apply.
class StatusState {
  const StatusState({
    this.status,
    this.statusRoot,
    this.gameRoot,
    this.busy = false,
    this.error,
    this.lastReport,
    this.studioActive = false,
  });

  /// The parsed `mgr_status`, or null without current status authority.
  final ManagerStatusView? status;

  /// Exact game root for which [status] is authoritative.
  ///
  /// This is null whenever a refresh fails or a different root is selected.
  final String? statusRoot;

  /// Root selected for root-bound reports and transient operation evidence.
  /// Unlike [statusRoot], this can remain set when status authority is absent.
  final String? gameRoot;

  /// True while any physical status read or the exclusive mutation is in flight.
  final bool busy;

  /// Message of the last failure (including "no game path"), or null.
  final String? error;

  /// Report from the last successful [StatusNotifier.apply].
  final ApplyReportView? lastReport;

  /// True when a blocked apply or recognized status established Studio
  /// ownership and current authority has not established a known non-studio
  /// state. Drives the fail-closed take-over fallback.
  final bool studioActive;

  StatusState copyWith({
    ManagerStatusView? status,
    String? statusRoot,
    String? gameRoot,
    bool? busy,
    String? error,
    ApplyReportView? lastReport,
    bool? studioActive,
    bool clearError = false,
    bool clearReport = false,
    bool clearStatus = false,
    bool clearStatusRoot = false,
    bool clearGameRoot = false,
  }) {
    return StatusState(
      status: clearStatus ? null : status ?? this.status,
      statusRoot: clearStatusRoot ? null : statusRoot ?? this.statusRoot,
      gameRoot: clearGameRoot ? null : gameRoot ?? this.gameRoot,
      busy: busy ?? this.busy,
      error: clearError ? null : error ?? this.error,
      lastReport: clearReport ? null : lastReport ?? this.lastReport,
      studioActive: studioActive ?? this.studioActive,
    );
  }
}

class _QueuedRefresh {
  _QueuedRefresh(this.gameRoot);

  String? gameRoot;
  final waiters = <Completer<void>>[];
}

/// Owns deployment status and the exclusive apply/undeploy mutation lane.
class StatusNotifier extends StateNotifier<StatusState> {
  StatusNotifier(this._mgr) : super(const StatusState());

  final MgrFfi _mgr;
  int _statusGeneration = 0;
  String? _selectedRoot;
  int _activeStatusReads = 0;
  bool _mutationInFlight = false;
  _QueuedRefresh? _queuedRefresh;

  /// Sentinel error set when refresh/apply is attempted without a game path.
  /// The UI maps it to the localized `errorSetGamePath`.
  static const noGamePath = 'NO_GAME_PATH';

  /// Refresh `mgr_status` for [gameRoot].
  ///
  /// Reads may overlap; only the newest generation for the currently selected
  /// root can publish. During a native write, refreshes are coalesced behind
  /// that write so a status read never observes a partial mutation.
  Future<void> refresh(String? gameRoot) {
    _selectRoot(gameRoot);
    if (_mutationInFlight) {
      _invalidateStatusReads();
      if (gameRoot == null) {
        state = state.copyWith(
          error: noGamePath,
          clearStatus: true,
          clearStatusRoot: true,
        );
      } else {
        state = state.copyWith(clearError: true);
      }
      _syncBusy();

      final waiter = Completer<void>();
      final queued = _queuedRefresh ??= _QueuedRefresh(gameRoot);
      queued.gameRoot = gameRoot;
      queued.waiters.add(waiter);
      return waiter.future;
    }
    return _runStatusRefresh(gameRoot);
  }

  /// Apply the current loadout and establish status again before settling.
  /// A second physical mutation is refused while this exclusive lane is busy.
  Future<void> apply(String gameRoot) => _runMutation(
    gameRoot,
    clearStudioAtStart: true,
    command: () => _mgr.apply(gameRoot),
  );

  /// Remove everything the manager deployed and establish status again before
  /// settling. Shares the same exclusive physical mutation lane as [apply].
  Future<void> undeployAll(String gameRoot) => _runMutation(
    gameRoot,
    command: () async {
      await _mgr.undeployAll(gameRoot);
      return null;
    },
  );

  Future<void> _runStatusRefresh(
    String? gameRoot, {
    bool preserveExistingError = false,
  }) async {
    _selectRoot(gameRoot);
    final generation = ++_statusGeneration;
    if (gameRoot == null) {
      state = state.copyWith(
        error: noGamePath,
        clearStatus: true,
        clearStatusRoot: true,
      );
      _syncBusy();
      return;
    }

    _activeStatusReads++;
    final keepError = preserveExistingError && state.error != null;
    state = state.copyWith(busy: true, clearError: !keepError);
    try {
      final status = await _mgr.status(gameRoot);
      if (!_ownsStatusRead(generation, gameRoot)) return;
      state = state.copyWith(
        status: status,
        statusRoot: gameRoot,
        // Remember recognized Studio ownership across later unknown future
        // states. Only a known non-studio state or root switch disproves it.
        studioActive: switch (status) {
          ManagerStatusStudioDeployActive() => true,
          ManagerStatusUnknown() => state.studioActive,
          _ => false,
        },
        clearError: !keepError,
      );
    } on Object catch (error) {
      if (!_ownsStatusRead(generation, gameRoot)) return;
      state = state.copyWith(
        error: keepError ? null : _message(error),
        clearStatus: true,
        clearStatusRoot: true,
      );
    } finally {
      _activeStatusReads--;
      // Publication is generation-bound; the physical-read count is not. An
      // older ignored read still blocks native writes until it settles.
      _syncBusy();
    }
  }

  Future<void> _runMutation(
    String gameRoot, {
    required Future<ApplyReportView?> Function() command,
    bool clearStudioAtStart = false,
  }) async {
    // UI gates these calls already. This guard makes the native safety rule
    // hold for direct/provider calls too: no overlapping physical writes, and
    // no write starts while a status read is inspecting the install.
    if (_mutationInFlight || _activeStatusReads > 0) return;

    _selectRoot(gameRoot);
    _invalidateStatusReads();
    _mutationInFlight = true;
    state = state.copyWith(
      busy: true,
      clearError: true,
      clearReport: true,
      clearStatus: true,
      clearStatusRoot: true,
      studioActive: clearStudioAtStart ? false : state.studioActive,
    );

    ApplyReportView? report;
    Object? commandError;
    try {
      report = await command();
    } on Object catch (error) {
      commandError = error;
    }

    // A native write can fail after touching disk. Always inspect the exact
    // mutation root afterward; the command error remains the user-facing
    // priority if this best-effort postflight also fails.
    ManagerStatusView? postflightStatus;
    Object? postflightError;
    try {
      postflightStatus = await _mgr.status(gameRoot);
    } on Object catch (error) {
      postflightError = error;
    }

    final queued = _queuedRefresh;
    final stillSelected = _selectedRoot == gameRoot;
    final studioBlocked =
        commandError is MgrFfiException &&
        commandError.code == 'STUDIO_DEPLOY_ACTIVE';
    final unresolvedStudioEvidence = studioBlocked || state.studioActive;
    final studioEvidenceAfterPostflight = switch (postflightStatus) {
      ManagerStatusStudioDeployActive() => true,
      ManagerStatusUnknown() ||
      null => commandError != null && unresolvedStudioEvidence,
      _ => false,
    };

    if (stillSelected) {
      if (report != null && commandError == null) {
        state = state.copyWith(lastReport: report);
      }

      final priorityError = commandError ?? postflightError;
      state = state.copyWith(
        error: priorityError == null ? null : _message(priorityError),
        clearError: priorityError == null,
        studioActive: studioEvidenceAfterPostflight,
      );

      // A refresh explicitly requested during the write owns the next
      // publication. Otherwise the mutation postflight establishes authority.
      if (queued == null) {
        if (postflightStatus == null) {
          state = state.copyWith(clearStatus: true, clearStatusRoot: true);
        } else {
          state = state.copyWith(
            status: postflightStatus,
            statusRoot: gameRoot,
          );
        }
      }
    }

    _mutationInFlight = false;
    if (queued == null) {
      _syncBusy();
      return;
    }

    _queuedRefresh = null;
    try {
      await _runStatusRefresh(
        queued.gameRoot,
        preserveExistingError:
            queued.gameRoot == gameRoot && commandError != null,
      );
    } finally {
      for (final waiter in queued.waiters) {
        if (!waiter.isCompleted) waiter.complete();
      }
    }
  }

  void _selectRoot(String? gameRoot) {
    if (_selectedRoot == gameRoot) return;
    _selectedRoot = gameRoot;
    // Status, reports, and the transient studio flag are installation-bound.
    state = state.copyWith(
      clearStatus: true,
      clearStatusRoot: true,
      clearReport: true,
      clearError: true,
      studioActive: false,
      gameRoot: gameRoot,
      clearGameRoot: gameRoot == null,
    );
  }

  void _invalidateStatusReads() {
    _statusGeneration++;
  }

  bool _ownsStatusRead(int generation, String gameRoot) =>
      generation == _statusGeneration && _selectedRoot == gameRoot;

  void _syncBusy() {
    final busy = _mutationInFlight || _activeStatusReads > 0;
    if (state.busy != busy) state = state.copyWith(busy: busy);
  }

  String _message(Object error) =>
      error is MgrFfiException ? error.message : error.toString();
}

final statusProvider = StateNotifierProvider<StatusNotifier, StatusState>((
  ref,
) {
  return StatusNotifier(ref.watch(mgrFfiProvider));
});
