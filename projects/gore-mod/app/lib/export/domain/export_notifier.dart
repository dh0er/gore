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

    final modDir = Directory(p.join(request.targetDir, request.modName));
    // Stage every file in a sibling temp directory and only swap it into place
    // once all writes succeed. Writing straight into an existing <modName>
    // would, on a mid-way failure (locked Scripts/main.lua, disk full), leave a
    // partially overwritten mod that UE4SS might still load.
    final staging = Directory(p.join(request.targetDir, '${request.modName}.tmp-export'));
    final backup = Directory(p.join(request.targetDir, '${request.modName}.bak-export'));
    String outputPath = modDir.path;
    try {
      if (staging.existsSync()) staging.deleteSync(recursive: true);
      for (final entry in files.entries) {
        final outFile = File(p.join(staging.path, entry.key));
        outFile.parent.createSync(recursive: true);
        outFile.writeAsStringSync(entry.value as String? ?? '');
      }
      // Promote with a safe swap: move any prior export aside to a backup,
      // rename the completed staging dir into place, then drop the backup. If
      // the promotion rename fails (lock, antivirus, cross-volume), the backup
      // is restored below so neither the old nor the new mod is lost.
      if (backup.existsSync()) backup.deleteSync(recursive: true);
      if (modDir.existsSync()) modDir.renameSync(backup.path);
      staging.renameSync(modDir.path);
      if (backup.existsSync()) backup.deleteSync(recursive: true);
      if (request.packageAsZip) {
        outputPath = _writeZip(request, files);
      }
    } on FileSystemException catch (e) {
      // Drop the incomplete staging dir, and if the old mod was moved aside but
      // not yet restored, put it back. The backup is never deleted on failure —
      // it holds the user's prior export.
      try {
        if (staging.existsSync()) staging.deleteSync(recursive: true);
      } on FileSystemException {
        // best-effort
      }
      try {
        if (backup.existsSync() && !modDir.existsSync()) {
          backup.renameSync(modDir.path);
        }
      } on FileSystemException {
        // best-effort; surface the original failure regardless
      }
      state = state.copyWith(
        isExporting: false,
        result: ExportResult(error: 'Failed to write mod files: ${e.message}'),
      );
      return;
    }

    state = state.copyWith(
      isExporting: false,
      result: ExportResult(outputPath: outputPath),
    );
  }

  /// Package the returned files into `<targetDir>/<modName>.zip`, with every
  /// entry nested under `<modName>/` (the same layout `gore-cli package`
  /// produces, so UE4SS sees a single mod folder when the archive is extracted
  /// into the Mods directory). Returns the zip path.
  String _writeZip(ExportRequest request, Map<String, Object?> files) {
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
    // Write to a temp file and rename over the target so a failed/partial write
    // never clobbers a previously good archive.
    final zipPath = p.join(request.targetDir, '${request.modName}.zip');
    final tmpZip = File('$zipPath.tmp-export');
    try {
      tmpZip.writeAsBytesSync(zipBytes);
      tmpZip.renameSync(zipPath);
    } on FileSystemException {
      if (tmpZip.existsSync()) tmpZip.deleteSync();
      rethrow;
    }
    return zipPath;
  }

  void clearResult() => state = state.copyWith(clearResult: true);
}

final exportProvider =
    StateNotifierProvider<ExportNotifier, ExportState>((ref) {
  return ExportNotifier(ref.watch(coreServiceProvider));
});
