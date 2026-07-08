import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

import 'shared_config.dart';

class UiSettings {
  const UiSettings({
    this.appLocale = 'en',
    this.themeMode = ThemeMode.light,
    this.uiScale = 1.0,
    this.windowSize,
    this.windowMaximized = false,
  });

  factory UiSettings.fromJson(Map<String, Object?> json) {
    return UiSettings(
      appLocale: switch (json['appLocale']) {
        // Trim so a stored " de " still matches kGameLangs (else the picker
        // would silently fall back to English).
        final String code when code.trim().isNotEmpty => code.trim(),
        _ => 'en',
      },
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
    );
  }

  /// Selected app/game language code (one of [kGameLangs]); drives the
  /// MaterialApp locale.
  final String appLocale;

  /// Selected theme mode (light/dark/system). Default light.
  final ThemeMode themeMode;

  /// Whole-UI zoom factor applied by [UiScaleRoot] (0.5–2.0). Default 1.0.
  final double uiScale;

  /// Last known window size in logical pixels; null until first persisted.
  final Size? windowSize;
  final bool windowMaximized;

  UiSettings copyWith({
    String? appLocale,
    ThemeMode? themeMode,
    double? uiScale,
    Size? windowSize,
    bool? windowMaximized,
  }) {
    return UiSettings(
      appLocale: appLocale ?? this.appLocale,
      themeMode: themeMode ?? this.themeMode,
      uiScale: uiScale ?? this.uiScale,
      windowSize: windowSize ?? this.windowSize,
      windowMaximized: windowMaximized ?? this.windowMaximized,
    );
  }

  Map<String, Object?> toJson() => {
    'appLocale': appLocale,
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

/// Resolves the shared `gore` umbrella data directory, matching the Rust
/// side (`gore_loc::paths::shared_data_dir`) exactly:
/// - Windows: `%LOCALAPPDATA%` (fallback `%APPDATA%`) then `\gore`
/// - macOS:   `$HOME/Library/Application Support/gore`
/// - Linux:   `$XDG_DATA_HOME/gore` else `$HOME/.local/share/gore`
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
  return p.join(base, 'gore');
}

class JsonFileUiSettingsStore implements UiSettingsStore {
  const JsonFileUiSettingsStore(this.file);

  factory JsonFileUiSettingsStore.defaultForPlatform({
    Map<String, String>? environment,
  }) {
    final env = environment ?? Platform.environment;
    return JsonFileUiSettingsStore(
      File(p.join(sharedDataDir(env), 'gore-manager', 'ui_settings.json')),
    );
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

/// The shared `config.json` the `gore` CLI and other apps also read/write.
final sharedConfigProvider = Provider<SharedConfig>((ref) {
  if (Platform.environment.containsKey('FLUTTER_TEST')) {
    // Widget tests must not touch the real shared config.
    return SharedConfig(File(p.join(Directory.systemTemp.path, 'gore-test', 'config.json')));
  }
  return SharedConfig.defaultForPlatform();
});

/// Path to the game's executable (.exe), or null if unset. Used to derive the
/// game root for deploys. Persisted (in the shared `config.json`) so the
/// choice survives restarts and is shared with the `gore` CLI and other apps.
final gameExePathProvider =
    StateNotifierProvider<GameExePathNotifier, String?>((ref) {
  return GameExePathNotifier(ref.watch(sharedConfigProvider));
});

class GameExePathNotifier extends StateNotifier<String?> {
  GameExePathNotifier(this._config) : super(_config.gamePath());

  final SharedConfig _config;

  void set(String path) {
    state = path;
    _config.setGamePath(path);
  }

  void clear() {
    state = null;
    _config.clearGamePath();
  }
}

/// Selected app/game language code (one of [kGameLangs.code]). Drives the
/// MaterialApp locale. Persisted so the choice survives restarts.
final localeProvider =
    StateNotifierProvider<LocaleNotifier, String>((ref) {
  return LocaleNotifier(ref.watch(uiSettingsStoreProvider));
});

class LocaleNotifier extends StateNotifier<String> {
  LocaleNotifier(this._store) : super(_store.read().appLocale);

  final UiSettingsStore _store;

  void setLocale(String code) {
    state = code;
    _store.write(_store.read().copyWith(appLocale: code));
  }
}

/// Selected theme mode (light/dark/system). Persisted so the choice survives
/// restarts. Defaults to [ThemeMode.light].
final themeModeProvider =
    StateNotifierProvider<ThemeModeNotifier, ThemeMode>((ref) {
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

/// Whole-UI zoom factor (0.5–2.0), applied by [UiScaleRoot]. Persisted so the
/// choice survives restarts. Defaults to 1.0.
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
