import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';

/// Stable URL: releases/latest/download/ redirects to the newest GitHub
/// release's assets, where CI uploads the signed appcast.
const _appcastUrl =
    'https://github.com/dh0er/goresave/releases/latest/download/appcast-windows.xml';

const _checkIntervalSeconds = 3600;

/// Initializes WinSparkle-based auto-updates. Best-effort: failures are
/// logged and never block startup. No-op outside Windows release builds
/// (dev runs are not installed, so an update prompt would be wrong).
Future<void> initDesktopUpdater() async {
  if (!kReleaseMode || !Platform.isWindows) {
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
