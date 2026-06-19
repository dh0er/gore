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
      // WinSparkle attaches to the existing window, so init only after show.
      unawaited(initDesktopUpdater());
    },
  );
  runApp(
    ProviderScope(
      overrides: [coreServiceProvider.overrideWithValue(createCoreService())],
      child: const GoreModApp(),
    ),
  );
}
