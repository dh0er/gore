import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'app/domain/desktop_updater.dart';
import 'app/domain/ui_settings.dart';
import 'app/domain/window_state_persistence.dart';
import 'app/ui/window_chrome.dart';
import 'core/core_service.dart';
import 'core/providers.dart';
import 'gore_manager_app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (windowChromeEnabled) {
    await windowManager.ensureInitialized();
    final settingsStore = JsonFileUiSettingsStore.defaultForPlatform();
    final settings = settingsStore.read();
    final windowOptions = WindowOptions(
      size: settings.windowSize ?? const Size(1280, 800),
      minimumSize: const Size(1100, 600),
      center: true,
      title: 'GORE Mod Manager',
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
  runApp(
    ProviderScope(
      overrides: [coreServiceProvider.overrideWithValue(createCoreService())],
      child: const GoreManagerApp(),
    ),
  );
  // WinSparkle attaches its update UI to the main window, so initialize it
  // only after the first frame; earlier init can show an unowned dialog.
  WidgetsBinding.instance.addPostFrameCallback((_) {
    unawaited(initDesktopUpdater());
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
