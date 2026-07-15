import 'dart:async';

import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import 'script_compile_install_state.dart';
import 'script_compile_report.dart';

typedef ScriptCompileInstallStateLoader =
    Future<ScriptCompileInstallState> Function(String gameRoot);

enum ScriptCompileInstallSafetyPhase { noGame, loading, ready, failed }

/// Persistent app-scoped evidence that the selected installation may require
/// recovery before any later compiler or deploy mutation.
///
/// Managed revision-3 compiler checks deliberately return no mini-cache path,
/// so they cannot be represented by [ScriptCompileReport]. This smaller closed
/// record keeps the shared safety gate recovery-dominant without inventing
/// artifact authority. A fresh native install-state probe is the only clear
/// operation.
final class ScriptCompileRecoveryEvidence {
  const ScriptCompileRecoveryEvidence({
    required this.code,
    required this.message,
    required this.installRestore,
    this.legacyReport,
  });

  final String code;
  final String message;
  final ScriptCompileInstallRestore installRestore;
  final ScriptCompileReport? legacyReport;

  factory ScriptCompileRecoveryEvidence.fromLegacyReport(
    ScriptCompileReport report,
  ) {
    final failure = report.failure;
    return ScriptCompileRecoveryEvidence(
      code: failure?.code ?? 'COMPILE_INSTALL_RECOVERY_REQUIRED',
      message:
          failure?.message ??
          'The previous compiler attempt requires installation recovery.',
      installRestore: report.installRestore,
      legacyReport: report,
    );
  }
}

final class ScriptCompileInstallSafetyState {
  const ScriptCompileInstallSafetyState._({
    required this.gameRoot,
    required this.phase,
    required this.installState,
    required this.recoveryEvidence,
    required this.errorMessage,
  });

  const ScriptCompileInstallSafetyState.noGame()
    : this._(
        gameRoot: null,
        phase: ScriptCompileInstallSafetyPhase.noGame,
        installState: null,
        recoveryEvidence: null,
        errorMessage: null,
      );

  final String? gameRoot;
  final ScriptCompileInstallSafetyPhase phase;
  final ScriptCompileInstallState? installState;
  final ScriptCompileRecoveryEvidence? recoveryEvidence;
  final String? errorMessage;

  /// Compatibility accessor for the legacy Script-tab report dialog.
  ScriptCompileReport? get recoveryReport => recoveryEvidence?.legacyReport;

  bool get recoveryRequired => recoveryEvidence != null;

  bool get liveMutationAllowed =>
      gameRoot != null &&
      phase == ScriptCompileInstallSafetyPhase.ready &&
      installState?.safeToCompile == true &&
      recoveryEvidence == null;

  bool get showBlockingBanner => gameRoot != null && !liveMutationAllowed;

  ScriptCompileInstallSafetyState copyWith({
    String? gameRoot,
    ScriptCompileInstallSafetyPhase? phase,
    ScriptCompileInstallState? installState,
    bool clearInstallState = false,
    ScriptCompileRecoveryEvidence? recoveryEvidence,
    bool clearRecoveryEvidence = false,
    String? errorMessage,
    bool clearError = false,
  }) => ScriptCompileInstallSafetyState._(
    gameRoot: gameRoot ?? this.gameRoot,
    phase: phase ?? this.phase,
    installState: clearInstallState ? null : installState ?? this.installState,
    recoveryEvidence: clearRecoveryEvidence
        ? null
        : recoveryEvidence ?? this.recoveryEvidence,
    errorMessage: clearError ? null : errorMessage ?? this.errorMessage,
  );
}

/// App-scoped safety state shared by every Scripts surface and Deploy dialog.
///
/// Recovery reports are retained per configured install across tab/selection
/// changes and can be cleared only by a fresh native probe proving the install
/// safe. A process restart reconstructs the state from that same native probe.
final class ScriptCompileInstallSafetyController
    extends StateNotifier<ScriptCompileInstallSafetyState> {
  ScriptCompileInstallSafetyController(
    this._load, {
    String? gameRoot,
    bool autoRefresh = true,
  }) : super(const ScriptCompileInstallSafetyState.noGame()) {
    setGameRoot(gameRoot, refresh: autoRefresh);
  }

  final ScriptCompileInstallStateLoader _load;
  final p.PathMap<ScriptCompileRecoveryEvidence> _recoveryEvidence =
      p.PathMap<ScriptCompileRecoveryEvidence>();
  int _generation = 0;
  bool _disposed = false;

  ScriptCompileInstallSafetyState get current => state;

  void setGameRoot(String? gameRoot, {bool refresh = true}) {
    final currentRoot = state.gameRoot;
    final currentRecovery = state.recoveryEvidence;
    if (currentRoot != null && currentRecovery != null) {
      _recoveryEvidence[currentRoot] = currentRecovery;
    }
    final normalized = gameRoot == null || gameRoot.isEmpty
        ? null
        : _normalizeGameRoot(gameRoot);
    _generation++;
    if (normalized == null) {
      state = const ScriptCompileInstallSafetyState.noGame();
      return;
    }
    state = ScriptCompileInstallSafetyState._(
      gameRoot: normalized,
      phase: refresh
          ? ScriptCompileInstallSafetyPhase.loading
          : ScriptCompileInstallSafetyPhase.failed,
      installState: null,
      recoveryEvidence: _recoveryEvidence[normalized],
      errorMessage: refresh ? null : 'Install safety has not been checked.',
    );
    if (refresh) unawaited(this.refresh());
  }

  void recordCompileReport(
    ScriptCompileReport report, {
    required String gameRoot,
  }) {
    if (!report.recoveryRequired) return;
    final evidence = ScriptCompileRecoveryEvidence.fromLegacyReport(report);
    _recordRecoveryForRoot(gameRoot, evidence);
  }

  /// Retain recovery from an evidence-only managed compiler check.
  ///
  /// This accepts only the recovery projection, never a staging or output path.
  void recordManagedRecovery({
    required String gameRoot,
    required String code,
    required String message,
    required ScriptCompileInstallRestore installRestore,
  }) {
    final evidence = ScriptCompileRecoveryEvidence(
      code: code,
      message: message,
      installRestore: installRestore,
    );
    _recordRecoveryForRoot(gameRoot, evidence);
  }

  void _recordRecoveryForRoot(
    String gameRoot,
    ScriptCompileRecoveryEvidence evidence,
  ) {
    final normalized = _normalizeGameRoot(gameRoot);

    _recoveryEvidence[normalized] = evidence;
    if (state.gameRoot == null || !p.equals(state.gameRoot!, normalized)) {
      return;
    }

    // Recovery is newer, mutation-derived evidence. Invalidate every probe for
    // this selected install that started before it so a late safe result cannot
    // erase the record. A probe for another selected install remains valid.
    _generation++;

    // The pre-mutation install probe is no longer authoritative. A new native
    // probe is the only operation that may restore the ready state and clear
    // recovery evidence.
    state = state.copyWith(
      phase: ScriptCompileInstallSafetyPhase.failed,
      clearInstallState: true,
      recoveryEvidence: evidence,
      errorMessage: 'Installation recovery must be rechecked.',
    );
  }

  Future<ScriptCompileInstallSafetyState> refresh() async {
    final gameRoot = state.gameRoot;
    if (gameRoot == null) return state;
    final generation = ++_generation;
    state = state.copyWith(
      phase: ScriptCompileInstallSafetyPhase.loading,
      clearError: true,
    );
    try {
      final result = await _load(gameRoot);
      if (_disposed ||
          generation != _generation ||
          state.gameRoot != gameRoot) {
        return state;
      }
      if (result.safeToCompile) _recoveryEvidence.remove(gameRoot);
      state = state.copyWith(
        phase: ScriptCompileInstallSafetyPhase.ready,
        installState: result,
        clearRecoveryEvidence: result.safeToCompile,
        clearError: true,
      );
    } catch (error) {
      if (_disposed ||
          generation != _generation ||
          state.gameRoot != gameRoot) {
        return state;
      }
      state = state.copyWith(
        phase: ScriptCompileInstallSafetyPhase.failed,
        clearInstallState: true,
        errorMessage: _boundedInstallStateError(error),
      );
    }
    return state;
  }

  @override
  void dispose() {
    _disposed = true;
    _generation++;
    super.dispose();
  }
}

final scriptCompileInstallSafetyProvider =
    StateNotifierProvider<
      ScriptCompileInstallSafetyController,
      ScriptCompileInstallSafetyState
    >((ref) {
      final controller = ScriptCompileInstallSafetyController(
        (gameRoot) => ModFfi(
          ref.read(coreServiceProvider),
        ).scriptCompileInstallStateV1(gameDir: gameRoot),
        gameRoot: gameRootFromExe(ref.read(gameExePathProvider)),
      );
      ref.listen<String?>(gameExePathProvider, (previous, next) {
        controller.setGameRoot(gameRootFromExe(next));
      });
      return controller;
    });

String _boundedInstallStateError(Object error) {
  final text = '$error';
  if (text.length <= 4096) return text;
  return '${text.substring(0, 4093)}...';
}

String _normalizeGameRoot(String gameRoot) => p.normalize(p.absolute(gameRoot));
