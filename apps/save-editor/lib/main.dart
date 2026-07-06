import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/desktop_updater.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/domain/window_state_persistence.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/ui/window_chrome.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (windowChromeEnabled) {
    await windowManager.ensureInitialized();
    final settingsStore = JsonFileUiSettingsStore.defaultForPlatform();
    final settings = settingsStore.read();
    final windowOptions = WindowOptions(
      size: settings.windowSize ?? const Size(1600, 900),
      minimumSize: const Size(960, 600),
      center: true,
      title: 'GORE Save Editor',
      // The app bar acts as the title bar (drag area, window buttons).
      titleBarStyle: TitleBarStyle.hidden,
    );
    await windowManager.waitUntilReadyToShow(windowOptions, () async {
      if (settings.windowMaximized) {
        await windowManager.maximize();
      }
      await windowManager.show();
      await windowManager.focus();
    });
    windowManager.addListener(
      _WindowStateListener(WindowStatePersister(settingsStore)),
    );
  }
  runApp(const ProviderScope(child: GoresaveApp()));
  // WinSparkle attaches its update UI to the main window, so initialize it
  // only after the first frame; earlier init can show an unowned dialog.
  WidgetsBinding.instance.addPostFrameCallback((_) {
    final autoCheckEnabled = JsonFileUiSettingsStore.defaultForPlatform()
        .read()
        .autoUpdateCheck;
    unawaited(initDesktopUpdater(autoCheckEnabled: autoCheckEnabled));
  });
}

/// Forwards window_manager events to the persister so the window size and
/// maximized state survive app restarts.
class _WindowStateListener with WindowListener {
  _WindowStateListener(this._persister);

  final WindowStatePersister _persister;

  @override
  Future<void> onWindowResized() async {
    _persister.handleResized(
      await windowManager.getSize(),
      isMaximized: await windowManager.isMaximized(),
    );
  }

  @override
  void onWindowMaximize() => _persister.handleMaximized();

  @override
  void onWindowUnmaximize() => _persister.handleUnmaximized();
}
