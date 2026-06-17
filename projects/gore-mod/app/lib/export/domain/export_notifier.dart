import 'package:flutter_riverpod/legacy.dart';
import '../../core/core_service.dart';
import '../../core/providers.dart';
import '../../editor/domain/override_entry.dart';
import 'export_request.dart';

class ExportState {
  const ExportState({
    this.isExporting = false,
    this.result,
    this.validationErrors = const [],
  });

  final bool isExporting;
  final ExportResult? result;

  /// Per-field validation errors from gore_core before generation.
  final List<String> validationErrors;

  ExportState copyWith({
    bool? isExporting,
    ExportResult? result,
    List<String>? validationErrors,
    bool clearResult = false,
  }) =>
      ExportState(
        isExporting:      isExporting ?? this.isExporting,
        result:           clearResult ? null : result ?? this.result,
        validationErrors: validationErrors ?? this.validationErrors,
      );
}

class ExportNotifier extends StateNotifier<ExportState> {
  ExportNotifier(this._core) : super(const ExportState());

  final GoreCoreFfiService _core;

  Future<void> export({
    required ExportRequest request,
    required List<OverrideEntry> overrides,
  }) async {
    state = state.copyWith(
      isExporting: true,
      validationErrors: [],
      clearResult: true,
    );

    // 1. Per-override validation via gore_core.
    final errors = <String>[];
    for (final override in overrides) {
      final res = await _core.execute('validate_override', payload: {
        'class': override.classId,
        'field': override.field,
        'value': override.newValue,
      });
      if (res['ok'] != true) {
        final msg = (res['error'] as Map?)
            ?['message'] as String? ?? 'Unknown validation error';
        errors.add('${override.classId}.${override.field}: $msg');
      }
    }

    if (errors.isNotEmpty) {
      state = state.copyWith(
        isExporting: false,
        validationErrors: errors,
      );
      return;
    }

    // 2. Generate the mod.
    final res = await _core.execute('generate_mod', payload: {
      'meta': {
        'name':     request.modName,
        'delay_ms': request.delayMs,
      },
      'overrides':   [for (final o in overrides) o.toJson()],
      'target_dir':  request.targetDir,
      'package_zip': request.packageAsZip,
    });

    if (res['ok'] != true) {
      final msg = (res['error'] as Map?)
          ?['message'] as String? ?? 'Generation failed';
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: msg),
      );
      return;
    }

    final outputPath = (res['data'] as Map?)
        ?['output_path'] as String? ?? request.targetDir;
    state = state.copyWith(
      isExporting: false,
      result: ExportResult(outputPath: outputPath),
    );
  }

  void clearResult() => state = state.copyWith(clearResult: true);
}

final exportProvider =
    StateNotifierProvider<ExportNotifier, ExportState>((ref) {
  return ExportNotifier(ref.watch(coreServiceProvider));
});
