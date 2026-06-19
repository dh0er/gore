import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/providers/data_providers.dart';

/// Lifecycle of a localization extraction request.
enum LocalizationPhase { idle, running, done, error }

/// Immutable view of the localized-text extraction feature.
///
/// `present` mirrors the core's `loc_status.present`; `meta` holds the last
/// known catalog metadata (id/language counts, source path) and is populated
/// from both `loc_status` and a successful `loc_extract`. `message` carries the
/// last human-readable success/error line for SnackBars.
class LocalizationState {
  const LocalizationState({
    this.phase = LocalizationPhase.idle,
    this.present = false,
    this.meta,
    this.message,
  });

  final LocalizationPhase phase;
  final bool present;
  final Map<String, Object?>? meta;
  final String? message;

  bool get isRunning => phase == LocalizationPhase.running;

  /// Number of extracted string ids, or null when no catalog is present.
  int? get idCount => (meta?['id_count'] as num?)?.toInt();

  /// Number of languages covered by the catalog, or null when absent.
  int? get languageCount => (meta?['languages'] as List?)?.length;

  LocalizationState copyWith({
    LocalizationPhase? phase,
    bool? present,
    Map<String, Object?>? meta,
    String? message,
    bool clearMessage = false,
  }) {
    return LocalizationState(
      phase: phase ?? this.phase,
      present: present ?? this.present,
      meta: meta ?? this.meta,
      message: clearMessage ? null : (message ?? this.message),
    );
  }
}

/// Result of an [LocalizationController.extract] attempt.
///
/// `notFound` is the distinct signal that auto-detection failed (the `.lcache`
/// wasn't located); the UI uses it to fall back to a file picker. Other
/// failures surface as `success == false` with a `message`.
class LocalizationExtractResult {
  const LocalizationExtractResult({
    required this.success,
    this.notFound = false,
    this.message,
  });

  final bool success;
  final bool notFound;
  final String? message;
}

/// Wraps the three localized-text core commands (`loc_status`, `loc_find`,
/// `loc_extract`). The extracted strings are written by the Rust side into a
/// shared user-local dir; this controller only triggers extraction and reports
/// status.
class LocalizationController extends StateNotifier<LocalizationState> {
  LocalizationController(this._core) : super(const LocalizationState());

  final GoresaveCoreService _core;

  /// Refresh status from the core. Returns the latest `present` value (false on
  /// any error, so callers can decide whether to offer extraction).
  Future<bool> status() async {
    try {
      final response = await _core.execute('loc_status');
      if (response['ok'] != true) {
        state = state.copyWith(message: _errorMessage(response));
        return state.present;
      }
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final present = data?['present'] == true;
      final meta = (data?['meta'] as Map?)?.cast<String, Object?>();
      state = state.copyWith(present: present, meta: meta);
      return present;
    } catch (error) {
      state = state.copyWith(message: 'Localization status failed: $error');
      return state.present;
    }
  }

  /// Trigger extraction. With no [lcacheHint] the core auto-detects via Steam;
  /// pass a hint (the `.lcache` file, the game dir, or a Steam library) to
  /// override. On a not-found auto-detect failure the result carries
  /// `notFound: true` so the caller can prompt for a file.
  Future<LocalizationExtractResult> extract({String? lcacheHint}) async {
    state = state.copyWith(phase: LocalizationPhase.running, clearMessage: true);
    try {
      final response = await _core.execute(
        'loc_extract',
        payload: {'lcache': ?lcacheHint},
      );
      if (response['ok'] != true) {
        final error = (response['error'] as Map?)?.cast<String, Object?>();
        final code = error?['code'] as String?;
        final message = error?['message'] as String? ?? 'Unknown core error';
        // INVALID_REQUEST is raised when the .lcache wasn't found (e.g. Steam
        // auto-detect came up empty); treat that as the file-picker fallback
        // signal rather than a hard error.
        final notFound = code == 'INVALID_REQUEST' && lcacheHint == null;
        state = state.copyWith(
          phase: LocalizationPhase.error,
          message: message,
        );
        return LocalizationExtractResult(
          success: false,
          notFound: notFound,
          message: message,
        );
      }
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final meta = (data?['meta'] as Map?)?.cast<String, Object?>();
      state = state.copyWith(
        phase: LocalizationPhase.done,
        present: true,
        meta: meta,
        clearMessage: true,
      );
      final idCount = (meta?['id_count'] as num?)?.toInt() ?? 0;
      final languageCount = (meta?['languages'] as List?)?.length ?? 0;
      final message =
          'Extracted $idCount ids across $languageCount languages';
      state = state.copyWith(message: message);
      return LocalizationExtractResult(success: true, message: message);
    } catch (error) {
      final message = 'Extraction failed: $error';
      state = state.copyWith(phase: LocalizationPhase.error, message: message);
      return LocalizationExtractResult(success: false, message: message);
    }
  }

  String _errorMessage(Map<String, Object?> response) {
    final error = (response['error'] as Map?)?.cast<String, Object?>();
    return error?['message'] as String? ?? 'Unknown core error';
  }
}

final localizationControllerProvider =
    StateNotifierProvider<LocalizationController, LocalizationState>((ref) {
      return LocalizationController(ref.watch(coreServiceProvider));
    });
