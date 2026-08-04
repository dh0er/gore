import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/providers/data_providers.dart';

AppLocalizations _defaultEnglishLocalizations() => AppLocalizationsEn();

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
    bool clearMeta = false,
    String? message,
    bool clearMessage = false,
  }) {
    return LocalizationState(
      phase: phase ?? this.phase,
      present: present ?? this.present,
      meta: clearMeta ? null : (meta ?? this.meta),
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
    this.idCount,
    this.languageCount,
  });

  final bool success;
  final bool notFound;
  final String? message;
  final int? idCount;
  final int? languageCount;
}

/// Wraps the three localized-text core commands (`loc_status`, `loc_find`,
/// `loc_extract`). The extracted strings are written by the Rust side into a
/// shared user-local dir; this controller only triggers extraction and reports
/// status.
class LocalizationController extends StateNotifier<LocalizationState> {
  LocalizationController(
    this._core, {
    AppLocalizations Function()? localizations,
  }) : _localizations = localizations ?? _defaultEnglishLocalizations,
       super(const LocalizationState());

  final GoresaveCoreService _core;
  final AppLocalizations Function() _localizations;

  AppLocalizations get _l10n => _localizations();

  /// Refresh status from the core. Returns the latest `present` value, or null
  /// when the status query itself failed (e.g. the native core is unavailable) —
  /// callers must not treat null as "not extracted".
  Future<bool?> status() async {
    try {
      final response = await _core.execute('loc_status');
      if (response['ok'] != true) {
        // Leave `present` untouched: a catalog may still exist from an earlier
        // extraction. The first-run prompt keys off the null return, not this.
        state = state.copyWith(
          message: _l10n.localizationStatusFailed(_errorDetails(response)),
        );
        return null;
      }
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final present = data?['present'] == true;
      final meta = (data?['meta'] as Map?)?.cast<String, Object?>();
      // Clear cached metadata whenever the status carries none — not only when
      // the catalog is absent: a present catalog can legitimately have no
      // loc_meta.json (e.g. after a meta write failure), and copyWith(meta:null)
      // would otherwise keep stale id/language counts from a different catalog.
      state = state.copyWith(
        present: present,
        meta: meta,
        clearMeta: meta == null,
      );
      return present;
    } catch (error) {
      // Leave `present` untouched (a catalog may still exist on disk).
      state = state.copyWith(message: _l10n.localizationStatusFailed('$error'));
      return null;
    }
  }

  /// Trigger extraction. With no [lcacheHint] the core auto-detects via Steam;
  /// pass a hint (the `.lcache` file, the game dir, or a Steam library) to
  /// override. On a not-found auto-detect failure the result carries
  /// `notFound: true` so the caller can prompt for a file.
  Future<LocalizationExtractResult> extract({String? lcacheHint}) async {
    state = state.copyWith(
      phase: LocalizationPhase.running,
      clearMessage: true,
    );
    try {
      final response = await _core.execute(
        'loc_extract',
        payload: {'lcache': ?lcacheHint},
      );
      if (response['ok'] != true) {
        final error = (response['error'] as Map?)?.cast<String, Object?>();
        final code = error?['code'] as String?;
        final message = _l10n.localizationExtractionFailed(
          _errorDetails(response),
        );
        // INVALID_REQUEST is raised when the .lcache wasn't found — whether we
        // were auto-detecting (no hint) or resolving from a hint that pointed at
        // no cache. Signal it regardless of the hint so the caller can tell a
        // "no cache here, try elsewhere" outcome (retry auto-detect / pick a
        // file) apart from a real read/decode failure of a cache that WAS found.
        final notFound = code == 'INVALID_REQUEST';
        if (notFound) {
          // Expected case: auto-detect came up empty and the caller will open
          // the file picker. Stay idle and clear the message so cancelling the
          // picker leaves no red error line (matches gore-mod).
          state = state.copyWith(
            phase: LocalizationPhase.idle,
            clearMessage: true,
          );
        } else {
          state = state.copyWith(
            phase: LocalizationPhase.error,
            message: message,
          );
        }
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
      // Return the counts and let the UI format them with AppLocalizations,
      // rather than embedding an English sentence here.
      return LocalizationExtractResult(
        success: true,
        idCount: idCount,
        languageCount: languageCount,
      );
    } catch (error) {
      final message = _l10n.localizationExtractionFailed('$error');
      state = state.copyWith(phase: LocalizationPhase.error, message: message);
      return LocalizationExtractResult(success: false, message: message);
    }
  }

  String _errorDetails(Map<String, Object?> response) {
    final error = (response['error'] as Map?)?.cast<String, Object?>();
    return error?['message'] as String? ?? _l10n.coreUnknownError;
  }
}

final localizationControllerProvider =
    StateNotifierProvider<LocalizationController, LocalizationState>((ref) {
      return LocalizationController(
        ref.watch(coreServiceProvider),
        localizations: () => ref.read(appLocalizationsProvider),
      );
    });
