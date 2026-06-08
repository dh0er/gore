import 'dart:convert';
import 'dart:io';

class EditorSettings {
  const EditorSettings({this.saveDir, this.codecHostPath, this.gameExePath});

  factory EditorSettings.fromJson(Map<String, Object?> json) {
    return EditorSettings(
      saveDir: _stringOrNull(json['saveDir']),
      codecHostPath: _stringOrNull(json['codecHostPath']),
      gameExePath: _stringOrNull(json['gameExePath']),
    );
  }

  final String? saveDir;
  final String? codecHostPath;
  final String? gameExePath;

  Map<String, Object?> toJson() => {
    if (saveDir != null) 'saveDir': saveDir,
    if (codecHostPath != null) 'codecHostPath': codecHostPath,
    if (gameExePath != null) 'gameExePath': gameExePath,
  };

  static String? _stringOrNull(Object? value) {
    if (value is! String || value.trim().isEmpty) return null;
    return value;
  }
}

abstract class EditorSettingsStore {
  EditorSettings read();
  void write(EditorSettings settings);
}

class NoopEditorSettingsStore implements EditorSettingsStore {
  const NoopEditorSettingsStore();

  @override
  EditorSettings read() => const EditorSettings();

  @override
  void write(EditorSettings settings) {}
}

class JsonFileEditorSettingsStore implements EditorSettingsStore {
  const JsonFileEditorSettingsStore(this.file);

  factory JsonFileEditorSettingsStore.defaultForPlatform({
    Map<String, String>? environment,
  }) {
    final env = environment ?? Platform.environment;
    final root =
        env['APPDATA'] ?? env['LOCALAPPDATA'] ?? Directory.current.path;
    return JsonFileEditorSettingsStore(File('$root\\goresave\\settings.json'));
  }

  final File file;

  @override
  EditorSettings read() {
    try {
      if (!file.existsSync()) return const EditorSettings();
      final decoded = jsonDecode(file.readAsStringSync());
      if (decoded is! Map) return const EditorSettings();
      return EditorSettings.fromJson(decoded.cast<String, Object?>());
    } catch (_) {
      return const EditorSettings();
    }
  }

  @override
  void write(EditorSettings settings) {
    file.parent.createSync(recursive: true);
    const encoder = JsonEncoder.withIndent('  ');
    file.writeAsStringSync('${encoder.convert(settings.toJson())}\n');
  }
}
