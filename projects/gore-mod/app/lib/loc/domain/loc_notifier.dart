import 'package:flutter_riverpod/legacy.dart';
import '../../core/core_service.dart';
import '../../core/providers.dart';

/// Phase of the localized-text extraction flow.
enum LocPhase { idle, running, done, error }

/// Outcome of an extraction attempt that needs the caller to take action.
/// [needsManualFile] means auto-detect could not find the .lcache and the
/// caller should open a file picker and retry with a hint path.
class LocExtractOutcome {
  const LocExtractOutcome({
    required this.success,
    this.needsManualFile = false,
    this.message,
    this.idCount,
    this.languageCount,
  });

  final bool success;
  final bool needsManualFile;
  final String? message;
  final int? idCount;
  final int? languageCount;
}

/// State surfaced to the UI: a phase plus the last meta/message for display.
class LocState {
  const LocState({
    this.phase = LocPhase.idle,
    this.present = false,
    this.message,
    this.idCount,
    this.languageCount,
  });

  final LocPhase phase;

  /// Whether a catalog has already been extracted (from `loc_status`).
  final bool present;

  /// Last error or status message for display.
  final String? message;
  final int? idCount;
  final int? languageCount;

  bool get isRunning => phase == LocPhase.running;

  LocState copyWith({
    LocPhase? phase,
    bool? present,
    String? message,
    int? idCount,
    int? languageCount,
    bool clearMessage = false,
    bool clearCounts = false,
  }) =>
      LocState(
        phase: phase ?? this.phase,
        present: present ?? this.present,
        message: clearMessage ? null : message ?? this.message,
        idCount: clearCounts ? null : idCount ?? this.idCount,
        languageCount: clearCounts ? null : languageCount ?? this.languageCount,
      );
}

/// Wraps the three native localized-text commands (`loc_status`, `loc_find`,
/// `loc_extract`). Extraction writes into a shared user-local dir on the Rust
/// side; this notifier only triggers it and reports status.
class LocNotifier extends StateNotifier<LocState> {
  LocNotifier(this._core) : super(const LocState());

  final GoreCoreFfiService _core;

  /// Refresh whether a catalog is already present. Returns true/false on a
  /// successful query, or null when the status call itself failed (e.g. the
  /// native core is unavailable) — callers must not treat null as "absent".
  Future<bool?> status() async {
    final res = await _core.execute('loc_status');
    if (res['ok'] != true) {
      return null;
    }
    final present = res['present'] == true;
    final meta = (present ? res['meta'] : null) as Map?;
    // Clear the cached counts when there's no metadata (catalog absent or its
    // sidecar missing) so they don't reflect a previous extraction.
    state = state.copyWith(
      present: present,
      idCount: (meta?['id_count'] as num?)?.toInt(),
      languageCount: (meta?['languages'] as List?)?.length,
      clearCounts: meta == null,
    );
    return present;
  }

  /// Run extraction. With no [lcacheHint] the Rust side auto-detects via Steam;
  /// pass a hint (the .lcache file, the game dir, or a Steam library) to
  /// override. Returns an outcome the caller uses to drive the UI (success,
  /// needs-manual-file, or a plain error message).
  Future<LocExtractOutcome> extract({String? lcacheHint}) async {
    state = state.copyWith(phase: LocPhase.running, clearMessage: true);

    final res = await _core.execute(
      'loc_extract',
      payload: lcacheHint == null ? const {} : {'lcache': lcacheHint},
    );

    if (res['ok'] == true) {
      final meta = (res['meta'] as Map?)?.cast<String, Object?>() ?? const {};
      final idCount = (meta['id_count'] as num?)?.toInt();
      final languageCount = (meta['languages'] as List?)?.length;
      // Return the counts and let the UI format them with AppLocalizations,
      // rather than embedding an English sentence here.
      state = state.copyWith(
        phase: LocPhase.done,
        present: true,
        clearMessage: true,
        idCount: idCount,
        languageCount: languageCount,
      );
      return LocExtractOutcome(
        success: true,
        idCount: idCount,
        languageCount: languageCount,
      );
    }

    final error = (res['error'] as Map?)?.cast<String, Object?>();
    final code = error?['code'] as String?;
    final message = error?['message'] as String? ?? 'Extraction failed';

    if (code == 'LCACHE_NOT_FOUND' && lcacheHint == null) {
      // Auto-detect failed; let the caller pick the file and retry. Stay in a
      // non-error phase so the manual picker flow reads as a continuation.
      state = state.copyWith(phase: LocPhase.idle, clearMessage: true);
      return const LocExtractOutcome(success: false, needsManualFile: true);
    }

    state = state.copyWith(phase: LocPhase.error, message: message);
    return LocExtractOutcome(success: false, message: message);
  }
}

final locProvider = StateNotifierProvider<LocNotifier, LocState>((ref) {
  return LocNotifier(ref.watch(coreServiceProvider));
});
