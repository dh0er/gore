import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'app/domain/desktop_updater.dart';
import 'core/core_service.dart';
import 'core/providers.dart';
import 'gore_mod_app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  windowManager.waitUntilReadyToShow(
    const WindowOptions(
      // Sized so the three fixed panes (catalog + editor + overrides) plus the
      // editor's label column always fit without horizontal overflow.
      size: Size(1600, 900),
      minimumSize: Size(1340, 640),
      title: 'gore-mod',
      titleBarStyle: TitleBarStyle.hidden,
    ),
    () async {
      await windowManager.show();
      await windowManager.focus();
    },
  );
  runApp(
    ProviderScope(
      overrides: [coreServiceProvider.overrideWithValue(createCoreService())],
      child: const GoreModApp(),
    ),
  );
  // WinSparkle attaches its update UI to the main window, so initialize it
  // only after the first frame; earlier init can show an unowned dialog.
  WidgetsBinding.instance.addPostFrameCallback((_) {
    unawaited(initDesktopUpdater());
  });
}
