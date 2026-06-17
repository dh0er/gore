import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/domain/window_state_persistence.dart';

class _MemoryUiSettingsStore implements UiSettingsStore {
  UiSettings settings = const UiSettings();

  @override
  UiSettings read() => settings;

  @override
  void write(UiSettings value) => settings = value;
}

void main() {
  test('persists window size after a normal resize', () {
    final store = _MemoryUiSettingsStore();
    final persister = WindowStatePersister(store);

    persister.handleResized(const Size(1720, 980), isMaximized: false);

    expect(store.settings.windowSize, const Size(1720, 980));
    expect(store.settings.windowMaximized, isFalse);
  });

  test('keeps last normal size when resize comes from maximizing', () {
    final store = _MemoryUiSettingsStore()
      ..settings = const UiSettings(windowSize: Size(1600, 900));
    final persister = WindowStatePersister(store);

    persister.handleResized(const Size(2560, 1392), isMaximized: true);

    expect(store.settings.windowSize, const Size(1600, 900));
  });

  test('persists maximized flag across maximize/unmaximize', () {
    final store = _MemoryUiSettingsStore()
      ..settings = const UiSettings(windowSize: Size(1600, 900));
    final persister = WindowStatePersister(store);

    persister.handleMaximized();
    expect(store.settings.windowMaximized, isTrue);
    expect(store.settings.windowSize, const Size(1600, 900));

    persister.handleUnmaximized();
    expect(store.settings.windowMaximized, isFalse);
  });
}
