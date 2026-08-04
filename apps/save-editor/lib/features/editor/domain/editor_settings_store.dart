import 'dart:convert';
import 'dart:io';

import 'package:goresave/utils/gore_tools_paths.dart';
import 'package:path/path.dart' as p;

class EditorSettings {
  const EditorSettings({
    this.saveDir,
    this.externalSavePaths = const [],
    this.hiddenOtherSavePaths = const [],
  });

  factory EditorSettings.fromJson(Map<String, Object?> json) {
    return EditorSettings(
      saveDir: _stringOrNull(json['saveDir']),
      externalSavePaths: _stringList(json['externalSavePaths']),
      hiddenOtherSavePaths: _stringList(json['hiddenOtherSavePaths']),
    );
  }

  final String? saveDir;

  /// Arbitrary files opened outside the configured game save folder.
  final List<String> externalSavePaths;

  /// Profileless saves from the configured folder that the user explicitly
  /// removed from the Other saves list. They stay on disk and remain hidden.
  final List<String> hiddenOtherSavePaths;

  Map<String, Object?> toJson() => {
    if (saveDir != null) 'saveDir': saveDir,
    'externalSavePaths': externalSavePaths,
    'hiddenOtherSavePaths': hiddenOtherSavePaths,
  };

  static String? _stringOrNull(Object? value) {
    if (value is! String || value.trim().isEmpty) return null;
    return value;
  }

  static List<String> _stringList(Object? value) {
    if (value is! List) return const [];
    final result = <String>[];
    for (final candidate in value.whereType<String>()) {
      final path = candidate.trim();
      if (path.isNotEmpty && !result.contains(path)) result.add(path);
    }
    return List.unmodifiable(result);
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
    const fileName = 'settings.json';
    final file = File(p.join(goreSaveSettingsDir(environment: env), fileName));
    migrateLegacySettingsFile(_legacyFile(env, fileName), file);
    return JsonFileEditorSettingsStore(file);
  }

  /// Previous per-app config location used before settings moved under the
  /// shared `gore` umbrella. Kept only to migrate old files once.
  static File _legacyFile(Map<String, String> env, String fileName) {
    final root =
        env['APPDATA'] ?? env['LOCALAPPDATA'] ?? Directory.current.path;
    return File('$root\\goresave\\$fileName');
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
