import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

class UiSettings {
  const UiSettings({this.gameExePath, this.appLocale = 'en'});

  factory UiSettings.fromJson(Map<String, Object?> json) {
    return UiSettings(
      gameExePath: switch (json['gameExePath']) {
        final String path when path.isNotEmpty => path,
        _ => null,
      },
      appLocale: switch (json['appLocale']) {
        // Trim so a stored " de " still matches kGameLangs (else the picker
        // would silently fall back to English).
        final String code when code.trim().isNotEmpty => code.trim(),
        _ => 'en',
      },
    );
  }

  /// Path to the game's executable (.exe). Used to derive the game root that
  /// mods are deployed into; null until set by the user.
  final String? gameExePath;

  /// Selected app/game language code (one of [kGameLangs]); drives the
  /// MaterialApp locale.
  final String appLocale;

  UiSettings copyWith({
    String? gameExePath,
    bool clearGameExePath = false,
    String? appLocale,
  }) {
    return UiSettings(
      gameExePath: clearGameExePath ? null : gameExePath ?? this.gameExePath,
      appLocale: appLocale ?? this.appLocale,
    );
  }

  Map<String, Object?> toJson() => {
    if (gameExePath != null) 'gameExePath': gameExePath,
    'appLocale': appLocale,
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
/// side (`gore_loc::paths::shared_data_dir`) exactly:
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

/// Path to the game's executable (.exe), or null if unset. Used to derive the
/// game root for deploys. Persisted so the choice survives restarts.
final gameExePathProvider =
    StateNotifierProvider<GameExePathNotifier, String?>((ref) {
  return GameExePathNotifier(ref.watch(uiSettingsStoreProvider));
});

class GameExePathNotifier extends StateNotifier<String?> {
  GameExePathNotifier(this._store) : super(_store.read().gameExePath);

  final UiSettingsStore _store;

  void set(String path) {
    state = path;
    _store.write(_store.read().copyWith(gameExePath: path));
  }

  void clear() {
    state = null;
    _store.write(_store.read().copyWith(clearGameExePath: true));
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
