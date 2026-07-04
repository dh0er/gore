import 'package:flutter_riverpod/legacy.dart';

import '../../core/mgr_ffi.dart';
import '../../core/providers.dart';
import '../../library/domain/models.dart';

/// Deployment status of the game install relative to the current loadout,
/// plus the outcome of the last apply.
class StatusState {
  const StatusState({
    this.status,
    this.busy = false,
    this.error,
    this.lastReport,
    this.studioActive = false,
  });

  /// The parsed `mgr_status`, or null before the first successful refresh.
  final ManagerStatusView? status;

  /// True while an FFI call is in flight.
  final bool busy;

  /// Message of the last failure (including "no game path"), or null.
  final String? error;

  /// Report from the last successful [StatusNotifier.apply].
  final ApplyReportView? lastReport;

  /// True when the last apply was blocked by an active studio deployment
  /// (FFI code `STUDIO_DEPLOY_ACTIVE`). Drives the take-over prompt.
  final bool studioActive;

  StatusState copyWith({
    ManagerStatusView? status,
    bool? busy,
    String? error,
    ApplyReportView? lastReport,
    bool? studioActive,
    bool clearError = false,
    bool clearReport = false,
    bool clearStatus = false,
  }) {
    return StatusState(
      status: clearStatus ? null : status ?? this.status,
      busy: busy ?? this.busy,
      error: clearError ? null : error ?? this.error,
      lastReport: clearReport ? null : lastReport ?? this.lastReport,
      studioActive: studioActive ?? this.studioActive,
    );
  }
}

/// Owns the deployment status and the apply/undeploy actions.
class StatusNotifier extends StateNotifier<StatusState> {
  StatusNotifier(this._mgr) : super(const StatusState());

  final MgrFfi _mgr;

  /// Sentinel error set when refresh/apply is attempted without a game path.
  /// The UI maps it to the localized `errorSetGamePath`.
  static const noGamePath = 'NO_GAME_PATH';

  /// Refresh `mgr_status`. A null [gameRoot] means the user hasn't set the
  /// game path yet; that parks [noGamePath] in the error rather than calling
  /// the FFI.
  Future<void> refresh(String? gameRoot) async {
    if (gameRoot == null) {
      // Drop any prior status too, so the chip can't keep showing a stale
      // "In sync" while the banner asks the user to set the game path.
      state = state.copyWith(error: noGamePath, clearStatus: true);
      return;
    }
    await _run(() async {
      final status = await _mgr.status(gameRoot);
      // The fresh status is authoritative for whether a studio deploy is active
      // (the `StudioDeployActive` variant drives the take-over chip). Clear the
      // transient [studioActive] flag left by an earlier blocked apply so a
      // later "no studio deploy" refresh can't keep the take-over prompt armed.
      state = state.copyWith(status: status, studioActive: false);
    });
  }

  /// Apply the current loadout to [gameRoot], record the report, then refresh
  /// status. If the apply is blocked by an active studio deployment, set
  /// [StatusState.studioActive] instead of surfacing a raw error.
  Future<void> apply(String gameRoot) async {
    state = state.copyWith(
      busy: true,
      clearError: true,
      studioActive: false,
      // Drop the previous apply's report up front: if this attempt fails, the
      // banner must not keep showing a stale "Applied N mods" next to the error.
      clearReport: true,
    );
    try {
      final report = await _mgr.apply(gameRoot);
      state = state.copyWith(lastReport: report);
      final status = await _mgr.status(gameRoot);
      state = state.copyWith(status: status);
    } on MgrFfiException catch (e) {
      if (e.code == 'STUDIO_DEPLOY_ACTIVE') {
        state = state.copyWith(studioActive: true, error: e.message);
        // The install really has a studio deploy — re-query so `status` reflects
        // StudioDeployActive (drives the chip + disables Apply) instead of a
        // stale e.g. changes_pending that would keep Apply enabled.
        try {
          final status = await _mgr.status(gameRoot);
          state = state.copyWith(status: status);
        } on MgrFfiException {
          // Best-effort; the studioActive flag still gates Apply.
        }
      } else {
        state = state.copyWith(error: e.message);
      }
    } finally {
      state = state.copyWith(busy: false);
    }
  }

  /// Remove everything the manager deployed from [gameRoot], then refresh.
  Future<void> undeployAll(String gameRoot) async {
    await _run(() async {
      await _mgr.undeployAll(gameRoot);
      final status = await _mgr.status(gameRoot);
      // Nothing is deployed anymore, so a prior "Applied N mods" report is stale.
      state = state.copyWith(status: status, studioActive: false, clearReport: true);
    });
  }

  Future<void> _run(Future<void> Function() body) async {
    state = state.copyWith(busy: true, clearError: true);
    try {
      await body();
    } on MgrFfiException catch (e) {
      state = state.copyWith(error: e.message);
    } finally {
      state = state.copyWith(busy: false);
    }
  }
}

final statusProvider =
    StateNotifierProvider<StatusNotifier, StatusState>((ref) {
  return StatusNotifier(ref.watch(mgrFfiProvider));
});
