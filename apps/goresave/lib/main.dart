import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/ui/goresave_app.dart';
import 'package:goresave/features/app/ui/window_chrome.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (windowChromeEnabled) {
    await windowManager.ensureInitialized();
    const windowOptions = WindowOptions(
      size: Size(1280, 720),
      minimumSize: Size(960, 600),
      center: true,
      title: 'Gothic Remake Savegame Editor',
      // The app bar acts as the title bar (drag area, window buttons).
      titleBarStyle: TitleBarStyle.hidden,
    );
    await windowManager.waitUntilReadyToShow(windowOptions, () async {
      await windowManager.show();
      await windowManager.focus();
    });
  }
  runApp(const ProviderScope(child: GoresaveApp()));
}
