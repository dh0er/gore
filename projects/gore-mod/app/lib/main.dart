import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'core/core_service.dart';
import 'core/providers.dart';
import 'gore_mod_app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  windowManager.waitUntilReadyToShow(
    const WindowOptions(
      size: Size(1280, 800),
      minimumSize: Size(900, 600),
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
}
