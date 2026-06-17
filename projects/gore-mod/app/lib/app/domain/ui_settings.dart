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
    );
  }

  final ThemeMode themeMode;
  final double uiScale;

  /// Last known window size in logical pixels; null until first persisted.
  final Size? windowSize;
  final bool windowMaximized;

  UiSettings copyWith({
    ThemeMode? themeMode,
    double? uiScale,
    Size? windowSize,
    bool? windowMaximized,
  }) {
    return UiSettings(
      themeMode: themeMode ?? this.themeMode,
      uiScale: uiScale ?? this.uiScale,
      windowSize: windowSize ?? this.windowSize,
      windowMaximized: windowMaximized ?? this.windowMaximized,
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

class JsonFileUiSettingsStore implements UiSettingsStore {
  const JsonFileUiSettingsStore(this.file);

  factory JsonFileUiSettingsStore.defaultForPlatform({
    Map<String, String>? environment,
  }) {
    final env = environment ?? Platform.environment;
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
    return JsonFileUiSettingsStore(
      File(p.join(root, 'gore-mod', 'ui_settings.json')),
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
