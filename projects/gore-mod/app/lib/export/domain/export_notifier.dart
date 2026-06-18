import 'dart:convert';
import 'dart:io';
import 'package:archive/archive.dart';
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

    // The mod name becomes a directory component under the user-chosen folder
    // (and an entry prefix inside the .zip). Reject path-escaping names before
    // building any path with it.
    final nameError = validateModName(request.modName);
    if (nameError != null) {
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: 'Invalid mod name: $nameError'),
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
    // contents); it does not touch the filesystem. Materialize it before
    // reporting success — either as a single <modName>.zip or as the
    // <modName>/ folder — otherwise the chosen folder stays empty.
    final files = (res['files'] as Map?)?.cast<String, Object?>();
    if (files == null) {
      state = state.copyWith(
        isExporting: false,
        result: const ExportResult(error: 'gore_core returned no files'),
      );
      return;
    }

    // Every temp/backup path gets a unique suffix so a retry never collides
    // with (and deletes) a backup a previous failed export left behind.
    final uid = DateTime.now().microsecondsSinceEpoch.toString();
    try {
      final outputPath = request.packageAsZip
          ? _writeZipAtomically(request, files, uid)
          : _writeFolderAtomically(request, files, uid);
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(outputPath: outputPath),
      );
    } on FileSystemException catch (e) {
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: 'Failed to write mod files: ${e.message}'),
      );
    }
  }

  /// Write the mod as the `<targetDir>/<modName>/` folder. All files are staged
  /// in a unique sibling dir; only once they are all written is the prior
  /// export swapped out (moved to a unique backup, staging renamed in, backup
  /// dropped). Any failure rolls back to the prior state and throws.
  String _writeFolderAtomically(
    ExportRequest request,
    Map<String, Object?> files,
    String uid,
  ) {
    final modDir = Directory(p.join(request.targetDir, request.modName));
    final staging = Directory('${modDir.path}.staging-$uid');
    final backup = Directory('${modDir.path}.backup-$uid');
    var oldMoved = false;
    var promoted = false;
    try {
      for (final entry in files.entries) {
        final outFile = File(p.join(staging.path, entry.key));
        outFile.parent.createSync(recursive: true);
        outFile.writeAsStringSync(entry.value as String? ?? '');
      }
      if (modDir.existsSync()) {
        modDir.renameSync(backup.path);
        oldMoved = true;
      }
      staging.renameSync(modDir.path);
      promoted = true;
      if (oldMoved) backup.deleteSync(recursive: true);
      return modDir.path;
    } on FileSystemException {
      _rollback(
        promotedTarget: promoted ? modDir : null,
        staging: staging,
        oldMoved: oldMoved,
        backup: backup,
        target: modDir,
      );
      rethrow;
    }
  }

  /// Write the mod as a single `<targetDir>/<modName>.zip`, every entry nested
  /// under `<modName>/` (the layout `gore-cli package` produces). Encoding and
  /// the temp write happen first; the prior zip is only swapped out via
  /// rename once the new archive is complete, with rollback on any failure.
  String _writeZipAtomically(
    ExportRequest request,
    Map<String, Object?> files,
    String uid,
  ) {
    final zipFile = File(p.join(request.targetDir, '${request.modName}.zip'));
    final staging = File('${zipFile.path}.staging-$uid');
    final backup = File('${zipFile.path}.backup-$uid');
    final archive = Archive();
    for (final entry in files.entries) {
      final bytes = utf8.encode(entry.value as String? ?? '');
      archive.addFile(
        ArchiveFile('${request.modName}/${entry.key}', bytes.length, bytes),
      );
    }
    final zipBytes = ZipEncoder().encode(archive);
    if (zipBytes == null) {
      throw const FileSystemException('zip encoding failed');
    }
    var oldMoved = false;
    var promoted = false;
    try {
      staging.writeAsBytesSync(zipBytes);
      if (zipFile.existsSync()) {
        zipFile.renameSync(backup.path);
        oldMoved = true;
      }
      staging.renameSync(zipFile.path);
      promoted = true;
      if (oldMoved) backup.deleteSync();
      return zipFile.path;
    } on FileSystemException {
      _rollback(
        promotedTarget: promoted ? zipFile : null,
        staging: staging,
        oldMoved: oldMoved,
        backup: backup,
        target: zipFile,
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
