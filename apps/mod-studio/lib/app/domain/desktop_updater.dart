import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;

/// gore-mod cannot use releases/latest/ — goresave owns the repo's "latest"
/// release pointer. Instead CI keeps a fixed `gore-mod-appcast` release whose
/// appcast-windows.xml asset it overwrites on every gore-mod release, so this
/// URL is stable regardless of which product released most recently.
const _appcastUrl =
    'https://github.com/dh0er/gore/releases/download/gore-mod-appcast/appcast-windows.xml';

const _checkIntervalSeconds = 3600;

bool? _innoInstalled;
bool _feedConfigured = false;

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
    debugPrint('gore-mod updater install check failed: $error');
    return false;
  }
}

/// True when this build can offer updates: Windows release build installed
/// via the Inno installer. Dev runs and the portable zip stay update-free.
bool get isDesktopUpdaterAvailable {
  if (!kReleaseMode || !Platform.isWindows) {
    return false;
  }
  return _innoInstalled ??= _isInnoInstalled();
}

/// WinSparkle's shutdown callback only asks the app to quit; nothing closes
/// the process for us. Exiting here releases gore_mod.exe so the Inno
/// installer launched by WinSparkle can replace it.
class _UpdaterQuitListener with UpdaterListener {
  @override
  void onUpdaterError(UpdaterError? error) {
    debugPrint('gore-mod updater error: $error');
  }

  @override
  void onUpdaterCheckingForUpdate(Appcast? appcast) {}

  @override
  void onUpdaterUpdateAvailable(AppcastItem? appcastItem) {}

  @override
  void onUpdaterUpdateNotAvailable(UpdaterError? error) {}

  @override
  void onUpdaterUpdateDownloaded(AppcastItem? appcastItem) {}

  @override
  void onUpdaterBeforeQuitForUpdate(AppcastItem? appcastItem) {
    exit(0);
  }
}

/// Sets the feed URL (which initializes WinSparkle) once. Returns false when
/// updates are unavailable for this build.
Future<bool> _ensureFeedConfigured() async {
  if (_feedConfigured) {
    return true;
  }
  if (!isDesktopUpdaterAvailable) {
    return false;
  }
  autoUpdater.addListener(_UpdaterQuitListener());
  await autoUpdater.setFeedURL(_appcastUrl);
  _feedConfigured = true;
  return true;
}

/// Initializes WinSparkle-based auto-updates. Best-effort: failures are
/// logged and never block startup. No-op when [isDesktopUpdaterAvailable]
/// is false.
///
/// Call only after the main window is shown — WinSparkle attaches its
/// update UI to the existing window.
Future<void> initDesktopUpdater() async {
  try {
    if (!await _ensureFeedConfigured()) {
      return;
    }
    await autoUpdater.setScheduledCheckInterval(_checkIntervalSeconds);
    // Silent check on startup: WinSparkle shows its own dialog only when
    // an update actually exists.
    await autoUpdater.checkForUpdates(inBackground: true);
  } catch (error) {
    debugPrint('gore-mod updater init failed: $error');
  }
}

/// User-triggered check: always shows WinSparkle's dialog, including the
/// "you're up to date" case.
Future<void> checkForUpdatesManually() async {
  try {
    if (!await _ensureFeedConfigured()) {
      return;
    }
    await autoUpdater.checkForUpdates();
  } catch (error) {
    debugPrint('gore-mod manual update check failed: $error');
  }
}
