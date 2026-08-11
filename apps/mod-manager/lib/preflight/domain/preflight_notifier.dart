import 'package:flutter_riverpod/legacy.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mgr_ffi.dart';
import '../../core/providers.dart';
import 'models.dart';

class PreflightState {
  const PreflightState({
    this.candidateRoot,
    this.report,
    this.reportRoot,
    this.busy = false,
    this.error,
    this.pending = false,
    this.generation = 0,
  });

  final String? candidateRoot;
  final ManagerPreflightView? report;
  final String? reportRoot;

  /// True until the single physical native read settles. Home mutation gates
  /// include this because the native snapshot internally reads deployment
  /// status and must not overlap an Apply/Undeploy write.
  final bool busy;
  final String? error;

  /// A root/library change requested one new read. Failures settle this flag so
  /// rebuilds do not create an automatic retry loop.
  final bool pending;

  /// Monotonic selection/library request identity. UI focus restoration binds
  /// to this so an old retry cannot affect a newer root or result action.
  final int generation;

  bool get authoritative => report != null && reportRoot == candidateRoot;

  PreflightState copyWith({
    String? candidateRoot,
    ManagerPreflightView? report,
    String? reportRoot,
    bool? busy,
    String? error,
    bool? pending,
    int? generation,
    bool clearReport = false,
    bool clearReportRoot = false,
    bool clearError = false,
  }) {
    return PreflightState(
      candidateRoot: candidateRoot ?? this.candidateRoot,
      report: clearReport ? null : report ?? this.report,
      reportRoot: clearReportRoot ? null : reportRoot ?? this.reportRoot,
      busy: busy ?? this.busy,
      error: clearError ? null : error ?? this.error,
      pending: pending ?? this.pending,
      generation: generation ?? this.generation,
    );
  }
}

/// Owns the read-only native preflight snapshot.
///
/// Reads are physically single-flight. Selection/library invalidation advances
/// a generation immediately, so a late old-root response cannot publish; the
/// pending newest read starts once the other Manager lanes are idle.
class PreflightNotifier extends StateNotifier<PreflightState> {
  PreflightNotifier(this._mgr, {String? initialRoot})
    : super(
        PreflightState(
          candidateRoot: initialRoot,
          pending: initialRoot != null,
        ),
      );

  final MgrFfi _mgr;
  int _generation = 0;
  bool _readInFlight = false;

  void selectRoot(String? root) {
    if (state.candidateRoot == root) return;
    _invalidate(root);
  }

  /// Library/loadout authority changed for the same selected installation.
  void invalidateLibrary() => _invalidate(state.candidateRoot);

  /// Explicit user retry after an unavailable or actionable snapshot.
  void retry() => _invalidate(state.candidateRoot);

  void _invalidate(String? root) {
    _generation++;
    state = PreflightState(
      candidateRoot: root,
      busy: _readInFlight,
      pending: root != null,
      generation: _generation,
    );
  }

  Future<void> refresh() async {
    final root = state.candidateRoot;
    if (root == null || !state.pending || _readInFlight) return;

    final generation = _generation;
    _readInFlight = true;
    state = PreflightState(
      candidateRoot: root,
      busy: true,
      generation: generation,
    );
    try {
      final report = await _mgr.preflight(root);
      if (_owns(generation, root)) {
        state = PreflightState(
          candidateRoot: root,
          report: report,
          reportRoot: root,
          busy: true,
          generation: generation,
        );
      }
    } on Object catch (error) {
      if (_owns(generation, root)) {
        state = PreflightState(
          candidateRoot: root,
          busy: true,
          error: error is MgrFfiException ? error.message : error.toString(),
          generation: generation,
        );
      }
    } finally {
      _readInFlight = false;
      if (state.busy) state = state.copyWith(busy: false);
    }
  }

  bool _owns(int generation, String root) =>
      generation == _generation && state.candidateRoot == root;
}

final preflightProvider =
    StateNotifierProvider<PreflightNotifier, PreflightState>((ref) {
      final initialRoot = diagnosticGameRootCandidate(
        ref.read(gameExePathProvider),
      );
      return PreflightNotifier(
        ref.watch(mgrFfiProvider),
        initialRoot: initialRoot,
      );
    });
