import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

import '../../loc/game_lang.dart';
import 'shared_config.dart';

class UiSettings {
  const UiSettings({
    this.themeMode = ThemeMode.light,
    this.uiScale = 1.0,
    this.windowSize,
    this.windowMaximized = false,
    this.dumpPath,
    this.locExtractPrompted = false,
    this.appLocale,
  });

  factory UiSettings.fromJson(Map<String, Object?> json) {
    return UiSettings(
      themeMode: switch (json['themeMode']) {
        'dark' => ThemeMode.dark,
        'system' => ThemeMode.system,
        _ => ThemeMode.light,
      },
      appLocale: switch (json['appLocale']) {
        // A missing or blank value stays null ("never chosen") so the app
        // follows the device language until the user picks one; a stored code
        // is trimmed so " de " still matches kGameLangs.
        final String code when code.trim().isNotEmpty => code.trim(),
        _ => null,
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

  /// Selected app/game language code (one of [kGameLangs]); drives both the
  /// MaterialApp locale and which extracted game-text names are shown. Null
  /// means "never chosen" — the app then follows the device language.
  final String? appLocale;

  UiSettings copyWith({
    ThemeMode? themeMode,
    double? uiScale,
    Size? windowSize,
    bool? windowMaximized,
    String? dumpPath,
    bool clearDumpPath = false,
    bool? locExtractPrompted,
    String? appLocale,
  }) {
    return UiSettings(
      themeMode: themeMode ?? this.themeMode,
      uiScale: uiScale ?? this.uiScale,
      windowSize: windowSize ?? this.windowSize,
      windowMaximized: windowMaximized ?? this.windowMaximized,
      dumpPath: clearDumpPath ? null : dumpPath ?? this.dumpPath,
      locExtractPrompted: locExtractPrompted ?? this.locExtractPrompted,
      appLocale: appLocale ?? this.appLocale,
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
    'appLocale': ?appLocale,
  };
}

abstract class UiSettingsStore {
  UiSettings read();
  void write(UiSettings settings);
}

class NoopUiSettingsStore implements UiSettingsStore {
  const NoopUiSettingsStore();

  @override
  UiSettings read() => const UiSettings(appLocale: 'en');

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
    const fileName = 'ui_settings.json';
    final file = File(p.join(sharedDataDir(env), 'gore-mod', fileName));
    return JsonFileUiSettingsStore(file);
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
    // Widget tests must not touch the real shared config. Use a UNIQUE temp file
    // per container so tests never leak persisted game-path state into one
    // another via a shared fixed path; each starts from a clean default.
    final dir = Directory.systemTemp.createTempSync('gore_test_cfg');
    return SharedConfig(File(p.join(dir.path, 'config.json')));
  }
  return SharedConfig.defaultForPlatform();
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
final dumpPathProvider = StateNotifierProvider<DumpPathNotifier, String?>((
  ref,
) {
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

/// Path to the game's executable (.exe), or null if unset. Used as the hint for
/// localized-text auto-detection and game-install discovery. Persisted (in the
/// shared `config.json`) so the choice survives restarts and is shared with the
/// `gore` CLI and other apps.
final gameExePathProvider = StateNotifierProvider<GameExePathNotifier, String?>(
  (ref) {
    return GameExePathNotifier(ref.watch(sharedConfigProvider));
  },
);

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

/// Selected app/game language code (one of [kGameLangs.code]). Drives both the
/// MaterialApp locale and which extracted game-text names are shown. Persisted
/// so the choice survives restarts.
final localeProvider = StateNotifierProvider<LocaleNotifier, String>((ref) {
  return LocaleNotifier(ref.watch(uiSettingsStoreProvider));
});

class LocaleNotifier extends StateNotifier<String> {
  LocaleNotifier(this._store)
    : super(
        _store.read().appLocale ??
            deviceLanguageCode(
              WidgetsBinding.instance.platformDispatcher.locales,
            ),
      );

  final UiSettingsStore _store;

  void setLocale(String code) {
    state = code;
    _store.write(_store.read().copyWith(appLocale: code));
  }
}
