import 'dart:io';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;
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

    // Generate the mod. Field-level validation already happened client-side in
    // the editor (only valid OverrideEntry values reach here), and the native
    // `validate` command needs a full ReflectionModel the GUI does not carry,
    // so we go straight to generation with the schema gore_core accepts:
    // `{meta, override:[{class, field, value_int|value_float|value_bool|value_str}]}`.
    final res = await _core.execute('generate_mod', payload: {
      'meta': {
        'name':     request.modName,
        'delay_ms': request.delayMs,
      },
      'override': [for (final o in overrides) o.toFfiJson()],
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

    // gore_core returns the mod as an in-memory `files` map (relative path ->
    // contents); it does not touch the filesystem. Materialize those files
    // under <targetDir>/<modName>/ before reporting success — otherwise the
    // chosen folder stays empty.
    final files = (res['files'] as Map?)?.cast<String, Object?>();
    if (files == null) {
      state = state.copyWith(
        isExporting: false,
        result: const ExportResult(error: 'gore_core returned no files'),
      );
      return;
    }

    final modDir = p.join(request.targetDir, request.modName);
    try {
      for (final entry in files.entries) {
        final outFile = File(p.join(modDir, entry.key));
        outFile.parent.createSync(recursive: true);
        outFile.writeAsStringSync(entry.value as String? ?? '');
      }
    } on FileSystemException catch (e) {
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: 'Failed to write mod files: ${e.message}'),
      );
      return;
    }

    state = state.copyWith(
      isExporting: false,
      result: ExportResult(outputPath: modDir),
    );
  }

  void clearResult() => state = state.copyWith(clearResult: true);
}

final exportProvider =
    StateNotifierProvider<ExportNotifier, ExportState>((ref) {
  return ExportNotifier(ref.watch(coreServiceProvider));
});
