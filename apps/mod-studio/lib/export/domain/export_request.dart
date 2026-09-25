class ExportRequest {
  const ExportRequest({
    required this.modName,
    required this.targetDir,
    this.delayMs = 0,
  });

  final String modName;
  final String targetDir;

  /// 0 = apply on first tick; >0 = ExecuteWithDelay in ms.
  final int delayMs;
}

class ExportResult {
  const ExportResult({this.outputPath, this.note, this.error});

  final String? outputPath;

  /// Core note that this file is a source spec, not a deployable mod.
  final String? note;
  final String? error;

  bool get success => error == null && outputPath != null;
}
