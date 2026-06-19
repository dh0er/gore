import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

class UiSettings {
  const UiSettings({
    this.themeMode = ThemeMode.light,
    this.uiScale = 1.0,
    this.windowSize,
    this.windowMaximized = false,
    this.dumpPath,
    this.locExtractPrompted = false,
  });

  factory UiSettings.fromJson(Map<String, Object?> json) {
    return UiSettings(
      themeMode: switch (json['themeMode']) {
        'dark' => ThemeMode.dark,
        'system' => ThemeMode.system,
        _ => ThemeMode.light,
      },
      uiScale: switch (json['uiScale']) {
        final num value => UiScaleNotifier.clampScale(value.toDouble()),
        _ => 1.0,
      },
      windowSize: switch ((json['windowWidth'], json['windowHeight'])) {
        (final num width, final num height) when width > 0 && height > 0 =>
          Size(width.toDouble(), height.toDouble()),
        _ => null,
      },
      windowMaximized: json['windowMaximized'] == true,
      dumpPath: switch (json['dumpPath']) {
        final String path when path.isNotEmpty => path,
        _ => null,
      },
      locExtractPrompted: json['locExtractPrompted'] == true,
    );
  }

  final ThemeMode themeMode;
  final double uiScale;

  /// Last known window size in logical pixels; null until first persisted.
  final Size? windowSize;
  final bool windowMaximized;

  /// Path to a user-loaded game-data dump that overrides the bundled model;
  /// null means use the bundled assets.
  final String? dumpPath;

  /// Whether the first-run localized-text extraction prompt has been shown.
  /// Persisted so the optional auto-prompt only fires once.
  final bool locExtractPrompted;

  UiSettings copyWith({
    ThemeMode? themeMode,
    double? uiScale,
    Size? windowSize,
    bool? windowMaximized,
    String? dumpPath,
    bool clearDumpPath = false,
    bool? locExtractPrompted,
  }) {
    return UiSettings(
      themeMode: themeMode ?? this.themeMode,
      uiScale: uiScale ?? this.uiScale,
      windowSize: windowSize ?? this.windowSize,
      windowMaximized: windowMaximized ?? this.windowMaximized,
      dumpPath: clearDumpPath ? null : dumpPath ?? this.dumpPath,
      locExtractPrompted: locExtractPrompted ?? this.locExtractPrompted,
    );
  }

  Map<String, Object?> toJson() => {
    'themeMode': switch (themeMode) {
      ThemeMode.dark => 'dark',
      ThemeMode.system => 'system',
      ThemeMode.light => 'light',
    },
    'uiScale': uiScale,
    if (windowSize case final size?) ...{
      'windowWidth': size.width,
      'windowHeight': size.height,
    },
    'windowMaximized': windowMaximized,
    if (dumpPath != null) 'dumpPath': dumpPath,
    'locExtractPrompted': locExtractPrompted,
  };
}

abstract class UiSettingsStore {
  UiSettings read();
  void write(UiSettings settings);
}

class NoopUiSettingsStore implements UiSettingsStore {
  const NoopUiSettingsStore();

  @override
  UiSettings read() => const UiSettings();

  @override
  void write(UiSettings settings) {}
}

/// Resolves the shared `gore-tools` umbrella data directory, matching the Rust
/// side (`gore_core::paths::shared_data_dir`) exactly:
/// - Windows: `%LOCALAPPDATA%` (fallback `%APPDATA%`) then `\gore-tools`
/// - macOS:   `$HOME/Library/Application Support/gore-tools`
/// - Linux:   `$XDG_DATA_HOME/gore-tools` else `$HOME/.local/share/gore-tools`
String sharedDataDir(Map<String, String> env) {
  final String base;
  if (Platform.isWindows) {
    base = env['LOCALAPPDATA'] ?? env['APPDATA'] ?? Directory.current.path;
  } else if (Platform.isMacOS) {
    final home = env['HOME'];
    base = home == null
        ? Directory.current.path
        : p.join(home, 'Library', 'Application Support');
  } else {
    final home = env['HOME'];
    final xdg = env['XDG_DATA_HOME'];
    base = (xdg != null && xdg.isNotEmpty)
        ? xdg
        : (home == null
              ? Directory.current.path
              : p.join(home, '.local', 'share'));
  }
  return p.join(base, 'gore-tools');
}

/// The previous per-app config directory (`<config>/gore-mod`), kept only so a
/// one-time migration can copy old files into the shared umbrella directory.
String _legacyAppDir(Map<String, String> env) {
  final String root;
  if (Platform.isWindows) {
    root = env['APPDATA'] ?? env['LOCALAPPDATA'] ?? Directory.current.path;
  } else if (Platform.isMacOS) {
    final home = env['HOME'];
    root = home == null
        ? Directory.current.path
        : p.join(home, 'Library', 'Application Support');
  } else {
    final home = env['HOME'];
    root =
        env['XDG_CONFIG_HOME'] ??
        (home == null ? Directory.current.path : p.join(home, '.config'));
  }
  return p.join(root, 'gore-mod');
}

class JsonFileUiSettingsStore implements UiSettingsStore {
  const JsonFileUiSettingsStore(this.file);

  factory JsonFileUiSettingsStore.defaultForPlatform({
    Map<String, String>? environment,
  }) {
    final env = environment ?? Platform.environment;
    const fileName = 'ui_settings.json';
    final file = File(p.join(sharedDataDir(env), 'gore-mod', fileName));
    _migrateLegacyFile(
      newFile: file,
      oldFile: File(p.join(_legacyAppDir(env), fileName)),
    );
    return JsonFileUiSettingsStore(file);
  }

  /// One-time, best-effort migration: if the new file is missing but a legacy
  /// one exists, copy it into the shared umbrella dir. The old file is left in
  /// place as a backup. Any failure is swallowed so startup falls back to
  /// defaults rather than crashing.
  static void _migrateLegacyFile({
    required File newFile,
    required File oldFile,
  }) {
    try {
      if (newFile.existsSync() || !oldFile.existsSync()) return;
      newFile.parent.createSync(recursive: true);
      oldFile.copySync(newFile.path);
    } catch (_) {
      // Ignore: a failed migration must never block startup.
    }
  }

  final File file;

  @override
  UiSettings read() {
    try {
      if (!file.existsSync()) return const UiSettings();
      final decoded = jsonDecode(file.readAsStringSync());
      if (decoded is! Map) return const UiSettings();
      return UiSettings.fromJson(decoded.cast<String, Object?>());
    } catch (_) {
      return const UiSettings();
    }
  }

  @override
  void write(UiSettings settings) {
    file.parent.createSync(recursive: true);
    const encoder = JsonEncoder.withIndent('  ');
    file.writeAsStringSync('${encoder.convert(settings.toJson())}\n');
  }
}

final uiSettingsStoreProvider = Provider<UiSettingsStore>((ref) {
  // Widget tests pump the full app; don't touch the real settings file there.
  if (Platform.environment.containsKey('FLUTTER_TEST')) {
    return const NoopUiSettingsStore();
  }
  return JsonFileUiSettingsStore.defaultForPlatform();
});

final themeModeProvider = StateNotifierProvider<ThemeModeNotifier, ThemeMode>((
  ref,
) {
  return ThemeModeNotifier(ref.watch(uiSettingsStoreProvider));
});

class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  ThemeModeNotifier(this._store) : super(_store.read().themeMode);

  final UiSettingsStore _store;

  void setThemeMode(ThemeMode themeMode) {
    state = themeMode;
    _store.write(_store.read().copyWith(themeMode: themeMode));
  }
}

final uiScaleProvider = StateNotifierProvider<UiScaleNotifier, double>((ref) {
  return UiScaleNotifier(ref.watch(uiSettingsStoreProvider));
});

class UiScaleNotifier extends StateNotifier<double> {
  UiScaleNotifier(this._store) : super(_store.read().uiScale);

  final UiSettingsStore _store;

  static double clampScale(double value) => value.clamp(0.5, 2.0);

  // Snap to whole percent so repeated +/- steps don't accumulate float noise.
  static double _snap(double value) => (value * 100).roundToDouble() / 100;

  void set(double value) {
    final next = _snap(clampScale(value));
    state = next;
    _store.write(_store.read().copyWith(uiScale: next));
  }

  void increase({double step = 0.05}) => set(state + step);
  void decrease({double step = 0.05}) => set(state - step);
  void reset() => set(1.0);
}

/// Path to a user-loaded game-data dump (overrides the bundled model), or null
/// for the bundled assets. Persisted so a loaded dump survives restarts.
final dumpPathProvider = StateNotifierProvider<DumpPathNotifier, String?>((ref) {
  return DumpPathNotifier(ref.watch(uiSettingsStoreProvider));
});

class DumpPathNotifier extends StateNotifier<String?> {
  DumpPathNotifier(this._store) : super(_store.read().dumpPath);

  final UiSettingsStore _store;

  void set(String path) {
    state = path;
    _store.write(_store.read().copyWith(dumpPath: path));
  }

  void clear() {
    state = null;
    _store.write(_store.read().copyWith(clearDumpPath: true));
  }
}

/// Whether the optional first-run localized-text extraction prompt has already
/// been shown. Persisted so the auto-prompt only fires once; the manual extract
/// action stays available regardless.
final locExtractPromptedProvider =
    StateNotifierProvider<LocExtractPromptedNotifier, bool>((ref) {
  return LocExtractPromptedNotifier(ref.watch(uiSettingsStoreProvider));
});

class LocExtractPromptedNotifier extends StateNotifier<bool> {
  LocExtractPromptedNotifier(this._store)
      : super(_store.read().locExtractPrompted);

  final UiSettingsStore _store;

  void markPrompted() {
    if (state) return;
    state = true;
    _store.write(_store.read().copyWith(locExtractPrompted: true));
  }
}
