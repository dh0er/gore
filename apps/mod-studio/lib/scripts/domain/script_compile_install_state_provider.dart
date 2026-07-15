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

final class ScriptCompileInstallSafetyState {
  const ScriptCompileInstallSafetyState._({
    required this.gameRoot,
    required this.phase,
    required this.installState,
    required this.recoveryReport,
    required this.errorMessage,
  });

  const ScriptCompileInstallSafetyState.noGame()
    : this._(
        gameRoot: null,
        phase: ScriptCompileInstallSafetyPhase.noGame,
        installState: null,
        recoveryReport: null,
        errorMessage: null,
      );

  final String? gameRoot;
  final ScriptCompileInstallSafetyPhase phase;
  final ScriptCompileInstallState? installState;
  final ScriptCompileReport? recoveryReport;
  final String? errorMessage;

  bool get liveMutationAllowed =>
      gameRoot != null &&
      phase == ScriptCompileInstallSafetyPhase.ready &&
      installState?.safeToCompile == true &&
      recoveryReport == null;

  bool get showBlockingBanner => gameRoot != null && !liveMutationAllowed;

  ScriptCompileInstallSafetyState copyWith({
    String? gameRoot,
    ScriptCompileInstallSafetyPhase? phase,
    ScriptCompileInstallState? installState,
    bool clearInstallState = false,
    ScriptCompileReport? recoveryReport,
    bool clearRecoveryReport = false,
    String? errorMessage,
    bool clearError = false,
  }) => ScriptCompileInstallSafetyState._(
    gameRoot: gameRoot ?? this.gameRoot,
    phase: phase ?? this.phase,
    installState: clearInstallState ? null : installState ?? this.installState,
    recoveryReport: clearRecoveryReport
        ? null
        : recoveryReport ?? this.recoveryReport,
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
  final Map<String, ScriptCompileReport> _recoveryReports = {};
  int _generation = 0;
  bool _disposed = false;

  ScriptCompileInstallSafetyState get current => state;

  void setGameRoot(String? gameRoot, {bool refresh = true}) {
    final currentRoot = state.gameRoot;
    final currentReport = state.recoveryReport;
    if (currentRoot != null && currentReport != null) {
      _recoveryReports[currentRoot] = currentReport;
    }
    final normalized = gameRoot == null || gameRoot.isEmpty
        ? null
        : p.normalize(p.absolute(gameRoot));
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
      recoveryReport: _recoveryReports[normalized],
      errorMessage: refresh ? null : 'Install safety has not been checked.',
    );
    if (refresh) unawaited(this.refresh());
  }

  void recordCompileReport(ScriptCompileReport report) {
    if (!report.recoveryRequired || state.gameRoot == null) return;
    _recoveryReports[state.gameRoot!] = report;
    state = state.copyWith(recoveryReport: report);
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
      if (result.safeToCompile) _recoveryReports.remove(gameRoot);
      state = state.copyWith(
        phase: ScriptCompileInstallSafetyPhase.ready,
        installState: result,
        clearRecoveryReport: result.safeToCompile,
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
