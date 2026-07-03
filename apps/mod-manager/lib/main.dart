import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'app/domain/desktop_updater.dart';
import 'core/core_service.dart';
import 'core/providers.dart';
import 'gore_manager_app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  await windowManager.waitUntilReadyToShow(
    const WindowOptions(
      size: Size(1280, 800),
      minimumSize: Size(1100, 600),
      title: 'gore-manager',
    ),
    () async {
      await windowManager.show();
      await windowManager.focus();
    },
  );
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
