import 'dart:ui';

import 'ui_settings.dart';

/// Persists window size and maximized state into the [UiSettingsStore].
///
/// Resize events that come from maximizing are ignored so that the last
/// "normal" window size survives a maximize/unmaximize cycle.
class WindowStatePersister {
  const WindowStatePersister(this._store);

  final UiSettingsStore _store;

  void handleResized(Size size, {required bool isMaximized}) {
    if (isMaximized) return;
    _store.write(_store.read().copyWith(windowSize: size));
  }

  void handleMaximized() {
    _store.write(_store.read().copyWith(windowMaximized: true));
  }

  void handleUnmaximized() {
    _store.write(_store.read().copyWith(windowMaximized: false));
  }
}
