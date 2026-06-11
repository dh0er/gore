import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;

/// Stable URL: releases/latest/download/ redirects to the newest GitHub
/// release's assets, where CI uploads the signed appcast.
const _appcastUrl =
    'https://github.com/dh0er/goresave/releases/latest/download/appcast-windows.xml';

const _checkIntervalSeconds = 3600;

/// Inno Setup always places an uninstaller (unins*.exe) next to the app;
/// the portable zip ships without one. Limits update prompts to installed
/// copies — a portable build must stay self-contained.
bool _isInnoInstalled() {
  try {
    return File(Platform.resolvedExecutable).parent.listSync().any(
          (entry) =>
              entry is File &&
              p.basename(entry.path).toLowerCase().startsWith('unins') &&
              entry.path.toLowerCase().endsWith('.exe'),
        );
  } catch (error) {
    debugPrint('goresave updater install check failed: $error');
    return false;
  }
}

/// Initializes WinSparkle-based auto-updates. Best-effort: failures are
/// logged and never block startup. No-op outside Windows release builds
/// (dev runs are not installed, so an update prompt would be wrong) and
/// for portable-zip launches (no Inno uninstaller next to the exe).
Future<void> initDesktopUpdater() async {
  if (!kReleaseMode || !Platform.isWindows || !_isInnoInstalled()) {
    return;
  }
  try {
    await autoUpdater.setFeedURL(_appcastUrl);
    await autoUpdater.setScheduledCheckInterval(_checkIntervalSeconds);
    // Silent check on startup: WinSparkle shows its own dialog only when
    // an update actually exists.
    await autoUpdater.checkForUpdates(inBackground: true);
  } catch (error) {
    debugPrint('goresave updater init failed: $error');
  }
}
