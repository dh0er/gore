import 'dart:io';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;
import '../../core/core_service.dart';
import '../../core/providers.dart';
import '../../editor/domain/override_entry.dart';
import 'export_request.dart';
import 'mod_name.dart';

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

    // The mod name becomes the source-spec filename. Reject path-escaping names
    // before building any path with it.
    // The dialog validates the name live (with localized messages) before
    // enabling Export, so this is a safety net. It surfaces an English string
    // because the notifier has no BuildContext; the live dialog error is the
    // one users normally see.
    final nameError = validateModName(request.modName);
    if (nameError != null) {
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: modNameErrorEnglish(nameError)),
      );
      return;
    }

    // Generate the mod. Field-level validation already happened client-side in
    // the editor (only valid OverrideEntry values reach here), and the native
    // `validate` command needs a full ReflectionModel the GUI does not carry,
    // so we go straight to generation with the schema gore_core accepts:
    // `{meta, override:[{class, field, value_int|value_float|value_bool|value_str}]}`.
    final res = await _core.execute('generate_mod', payload: {
      'meta': {
        'name': request.modName,
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

    // generate_mod returns a source spec, not a deployable mod. Write that one
    // file and keep the core note. A folder or zip would look like a finished
    // mod and cannot be deployed.
    final files = (res['files'] as Map?)?.cast<String, Object?>();
    final spec = files?['spec.json'];
    if (files == null || files.length != 1 || spec is! String) {
      state = state.copyWith(
        isExporting: false,
        result: const ExportResult(
          error: 'generate_mod did not return a source spec.json',
        ),
      );
      return;
    }

    final uid = DateTime.now().microsecondsSinceEpoch.toString();
    try {
      final outputPath = _writeSpecAtomically(request, spec, uid);
      final note = res['note'];
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(
          outputPath: outputPath,
          note: note is String ? note : null,
        ),
      );
    } on FileSystemException catch (e) {
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: 'Failed to write spec: ${e.message}'),
      );
    }
  }

  String _writeSpecAtomically(ExportRequest request, String spec, String uid) {
    final outFile = File(p.join(request.targetDir, '${request.modName}.spec.json'));
    final staging = File('${outFile.path}.staging-$uid');
    final backup = File('${outFile.path}.backup-$uid');
    var oldMoved = false;
    var promoted = false;
    try {
      staging.writeAsStringSync(spec);
      if (outFile.existsSync()) {
        outFile.renameSync(backup.path);
        oldMoved = true;
      }
      staging.renameSync(outFile.path);
      promoted = true;
      if (oldMoved) backup.deleteSync();
      return outFile.path;
    } on FileSystemException {
      _rollback(
        promotedTarget: promoted ? outFile : null,
        staging: staging,
        oldMoved: oldMoved,
        backup: backup,
        target: outFile,
      );
      rethrow;
    }
  }

  /// Restore the prior state after a failed atomic write: undo a completed
  /// promotion, restore the moved-aside backup, and drop the staging artifact.
  /// [target]/[staging]/[backup]/[promotedTarget] are all File or Directory
  /// (FileSystemEntity). Backups are never deleted here — they hold the user's
  /// prior export. Each step is best-effort so the original error still
  /// surfaces.
  void _rollback({
    required FileSystemEntity? promotedTarget,
    required FileSystemEntity staging,
    required bool oldMoved,
    required FileSystemEntity backup,
    required FileSystemEntity target,
  }) {
    if (promotedTarget != null && promotedTarget.existsSync()) {
      try {
        promotedTarget.renameSync(staging.path);
      } on FileSystemException {/* best-effort */}
    }
    if (oldMoved && backup.existsSync() && !target.existsSync()) {
      try {
        backup.renameSync(target.path);
      } on FileSystemException {/* best-effort */}
    }
    try {
      if (staging.existsSync()) staging.deleteSync(recursive: true);
    } on FileSystemException {/* best-effort */}
  }

  void clearResult() => state = state.copyWith(clearResult: true);
}

final exportProvider =
    StateNotifierProvider<ExportNotifier, ExportState>((ref) {
  return ExportNotifier(ref.watch(coreServiceProvider));
});
