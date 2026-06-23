class ExportRequest {
  const ExportRequest({
    required this.modName,
    required this.targetDir,
    this.delayMs = 0,
    this.packageAsZip = false,
  });

  final String modName;
  final String targetDir;

  /// 0 = apply on first tick; >0 = ExecuteWithDelay in ms.
  final int delayMs;
  final bool packageAsZip;
}

class ExportResult {
  const ExportResult({this.outputPath, this.error});

  final String? outputPath;
  final String? error;

  bool get success => error == null && outputPath != null;
}
