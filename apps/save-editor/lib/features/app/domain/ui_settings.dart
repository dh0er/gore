import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/utils/gore_tools_paths.dart';
import 'package:path/path.dart' as p;

enum UiFontFamily { system, podkova, notoSerif }

bool uiFontFamilySupportedFor(UiFontFamily font, GameLang lang) =>
    font != UiFontFamily.podkova ||
    (lang.locale.languageCode != 'ja' && lang.locale.languageCode != 'zh');

UiFontFamily effectiveUiFontFamily(UiFontFamily font, GameLang lang) =>
    uiFontFamilySupportedFor(font, lang) ? font : UiFontFamily.notoSerif;

class UiSettings {
  const UiSettings({
    this.themeMode = ThemeMode.light,
    this.uiFontFamily = UiFontFamily.notoSerif,
    this.uiScale = 1.0,
    this.windowSize,
    this.windowMaximized = false,
    this.autoUpdateCheck = true,
    this.gameDataSourceNoticeShown = false,
    this.showObjectIds = false,
    this.appLocale,
  });

  factory UiSettings.fromJson(Map<String, Object?> json) {
    return UiSettings(
      themeMode: switch (json['themeMode']) {
        'dark' => ThemeMode.dark,
        'system' => ThemeMode.system,
        _ => ThemeMode.light,
      },
      uiFontFamily: switch (json['uiFontFamily']) {
        'system' => UiFontFamily.system,
        'podkova' => UiFontFamily.podkova,
        'notoSerif' => UiFontFamily.notoSerif,
        // Migrate the previous switch without unexpectedly changing an
        // existing user's appearance. New installs default to Noto Serif.
        _ when json.containsKey('gothicUiFont') =>
          json['gothicUiFont'] == true
              ? UiFontFamily.podkova
              : UiFontFamily.system,
        _ => UiFontFamily.notoSerif,
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
      autoUpdateCheck: json['autoUpdateCheck'] != false,
      // The old modal used `locExtractPrompted`. Treat an accepted legacy
      // prompt as an already-seen replacement notice so existing users are not
      // interrupted again after upgrading.
      gameDataSourceNoticeShown:
          json['gameDataSourceNoticeShown'] == true ||
          json['locExtractPrompted'] == true,
      showObjectIds: json['showObjectIds'] == true,
    );
  }

  final ThemeMode themeMode;
  final UiFontFamily uiFontFamily;
  final double uiScale;

  /// Last known window size in logical pixels; null until first persisted.
  final Size? windowSize;
  final bool windowMaximized;

  /// Whether the app checks for updates automatically (on by default).
  final bool autoUpdateCheck;

  /// Whether the one-time, non-modal hint for a missing localization source
  /// has been shown. Automatic source detection still retries on later starts.
  final bool gameDataSourceNoticeShown;

  /// Whether technical object identifiers are shown alongside localized names.
  /// Kept off by default so normal editor views stay focused on player-facing
  /// labels; individual panels consume [showObjectIdsProvider] when rendering.
  final bool showObjectIds;

  /// Selected UI + game-text language code (one of [kGameLangs]). Drives both
  /// the MaterialApp locale and which extracted game-text names (items, NPCs,
  /// knowledge) are shown. Null means "never chosen" — the app then follows the
  /// device language.
  final String? appLocale;

  UiSettings copyWith({
    ThemeMode? themeMode,
    UiFontFamily? uiFontFamily,
    double? uiScale,
    Size? windowSize,
    bool? windowMaximized,
    bool? autoUpdateCheck,
    bool? gameDataSourceNoticeShown,
    bool? showObjectIds,
    String? appLocale,
  }) {
    return UiSettings(
      themeMode: themeMode ?? this.themeMode,
      uiFontFamily: uiFontFamily ?? this.uiFontFamily,
      uiScale: uiScale ?? this.uiScale,
      windowSize: windowSize ?? this.windowSize,
      windowMaximized: windowMaximized ?? this.windowMaximized,
      autoUpdateCheck: autoUpdateCheck ?? this.autoUpdateCheck,
      gameDataSourceNoticeShown:
          gameDataSourceNoticeShown ?? this.gameDataSourceNoticeShown,
      showObjectIds: showObjectIds ?? this.showObjectIds,
      appLocale: appLocale ?? this.appLocale,
    );
  }

  Map<String, Object?> toJson() => {
    'themeMode': switch (themeMode) {
      ThemeMode.dark => 'dark',
      ThemeMode.system => 'system',
      ThemeMode.light => 'light',
    },
    'uiFontFamily': uiFontFamily.name,
    'uiScale': uiScale,
    if (windowSize case final size?) ...{
      'windowWidth': size.width,
      'windowHeight': size.height,
    },
    'windowMaximized': windowMaximized,
    'autoUpdateCheck': autoUpdateCheck,
    'gameDataSourceNoticeShown': gameDataSourceNoticeShown,
    'showObjectIds': showObjectIds,
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

class JsonFileUiSettingsStore implements UiSettingsStore {
  const JsonFileUiSettingsStore(this.file);

  factory JsonFileUiSettingsStore.defaultForPlatform({
    Map<String, String>? environment,
  }) {
    final env = environment ?? Platform.environment;
    const fileName = 'ui_settings.json';
    final file = File(p.join(goreSaveSettingsDir(environment: env), fileName));
    migrateLegacySettingsFile(_legacyFile(env, fileName), file);
    return JsonFileUiSettingsStore(file);
  }

  /// Previous per-app config location used before settings moved under the
  /// shared `gore` umbrella. Kept only to migrate old files once.
  static File _legacyFile(Map<String, String> env, String fileName) {
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
    return File(p.join(root, 'goresave', fileName));
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

final uiFontFamilyProvider =
    StateNotifierProvider<UiFontFamilyNotifier, UiFontFamily>((ref) {
      return UiFontFamilyNotifier(ref.watch(uiSettingsStoreProvider));
    });

class UiFontFamilyNotifier extends StateNotifier<UiFontFamily> {
  UiFontFamilyNotifier(this._store) : super(_store.read().uiFontFamily);

  final UiSettingsStore _store;

  void set(UiFontFamily font) {
    state = font;
    _store.write(_store.read().copyWith(uiFontFamily: font));
  }
}

/// Selected language code (one of [kGameLangs]). Persisted through the shared
/// Ui settings store, mirroring [themeModeProvider]. Drives both the app UI
/// locale and which extracted game-text names are shown.
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

final autoUpdateCheckProvider =
    StateNotifierProvider<AutoUpdateCheckNotifier, bool>((ref) {
      return AutoUpdateCheckNotifier(ref.watch(uiSettingsStoreProvider));
    });

/// Controls whether editor panels render technical NPC, item, knowledge and
/// quest identifiers. Persisted in [UiSettings] and intentionally disabled by
/// default; panels can watch this provider without depending on the settings
/// file implementation.
final showObjectIdsProvider =
    StateNotifierProvider<ShowObjectIdsNotifier, bool>((ref) {
      return ShowObjectIdsNotifier(ref.watch(uiSettingsStoreProvider));
    });

class ShowObjectIdsNotifier extends StateNotifier<bool> {
  ShowObjectIdsNotifier(this._store) : super(_store.read().showObjectIds);

  final UiSettingsStore _store;

  void set(bool enabled) {
    state = enabled;
    _store.write(_store.read().copyWith(showObjectIds: enabled));
  }
}

class AutoUpdateCheckNotifier extends StateNotifier<bool> {
  AutoUpdateCheckNotifier(this._store) : super(_store.read().autoUpdateCheck);

  final UiSettingsStore _store;

  void set(bool enabled) {
    state = enabled;
    _store.write(_store.read().copyWith(autoUpdateCheck: enabled));
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
