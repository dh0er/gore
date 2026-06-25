import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path/path.dart' as p;
import 'package:url_launcher/url_launcher.dart';

/// Stable URL: releases/latest/download/ redirects to the newest GitHub
/// release's assets, where CI uploads the signed appcast.
const _appcastUrl =
    'https://github.com/dh0er/goresave/releases/latest/download/appcast-windows.xml';

/// Where the portable build sends users to grab the new build. The latest
/// release page lists both the installer and the portable zip.
const _releasesPageUrl = 'https://github.com/dh0er/goresave/releases/latest';

const _checkIntervalSeconds = 3600;

/// Attached to the GoRouter so a background update check (which has no widget
/// context of its own) can still surface a dialog. See [router.dart].
final updaterNavigatorKey = GlobalKey<NavigatorState>();

bool? _innoInstalled;
bool _feedConfigured = false;
Timer? _portableTimer;

/// Inno Setup always places an uninstaller (unins*.exe) next to the app;
/// the portable zip ships without one. Distinguishes the installed copy
/// (WinSparkle can replace it in place) from the portable copy (user copies
/// the new files over the old ones).
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

/// True for the installed (Inno) build, which updates itself via WinSparkle.
/// False for the portable zip, which checks-and-points-to-download instead.
bool get isInstalledBuild => _innoInstalled ??= _isInnoInstalled();

/// True when this build can check for updates at all: any Windows release
/// build, installed or portable. Dev runs stay update-free.
bool get isUpdateCheckAvailable => kReleaseMode && Platform.isWindows;

// --------------------------------------------------------------------------- #
// WinSparkle path (installed build only): silent download + install + relaunch.
// --------------------------------------------------------------------------- #

/// WinSparkle's shutdown callback only asks the app to quit; nothing closes
/// the process for us. Exiting here releases goresave.exe so the Inno
/// installer launched by WinSparkle can replace it.
class _UpdaterQuitListener with UpdaterListener {
  @override
  void onUpdaterError(UpdaterError? error) {
    debugPrint('goresave updater error: $error');
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
/// this is not the installed build.
Future<bool> _ensureFeedConfigured() async {
  if (_feedConfigured) {
    return true;
  }
  if (!isUpdateCheckAvailable || !isInstalledBuild) {
    return false;
  }
  autoUpdater.addListener(_UpdaterQuitListener());
  await autoUpdater.setFeedURL(_appcastUrl);
  _feedConfigured = true;
  return true;
}

// --------------------------------------------------------------------------- #
// Portable path: fetch the appcast, compare versions, point at the download.
// --------------------------------------------------------------------------- #

/// Fetches and parses the appcast. Returns the advertised version, or null on
/// any failure (network, redirect, malformed feed). The appcast is the same
/// signed feed WinSparkle reads; here we only need the version string.
Future<String?> _fetchLatestVersion() async {
  final client = HttpClient();
  try {
    final request = await client.getUrl(Uri.parse(_appcastUrl));
    final response = await request.close();
    if (response.statusCode != HttpStatus.ok) {
      return null;
    }
    final body = await response.transform(utf8.decoder).join();
    final match =
        RegExp(r'<sparkle:version>\s*([^<\s]+)\s*</sparkle:version>')
            .firstMatch(body);
    return match?.group(1);
  } catch (error) {
    debugPrint('goresave portable update check failed: $error');
    return null;
  } finally {
    client.close(force: true);
  }
}

/// Compares dotted numeric versions (e.g. "0.4.0"). Returns true when
/// [latest] is strictly newer than [current]. Non-numeric or missing
/// components are treated as 0, so "0.4" < "0.4.1".
bool _isNewer(String latest, String current) {
  final a = latest.split('.');
  final b = current.split('.');
  for (var i = 0; i < (a.length > b.length ? a.length : b.length); i++) {
    final x = i < a.length ? int.tryParse(a[i]) ?? 0 : 0;
    final y = i < b.length ? int.tryParse(b[i]) ?? 0 : 0;
    if (x != y) {
      return x > y;
    }
  }
  return false;
}

/// Runs a portable check. When an update exists, shows a dialog offering to
/// open the releases page in the browser. When [silentIfNoUpdate] is false
/// (manual check), also reports the up-to-date and failure cases.
Future<void> _runPortableCheck({required bool silentIfNoUpdate}) async {
  // Do every await up front so the context, once captured, is used without
  // an async gap (avoids stale-context use after the widget tree changes).
  final latest = await _fetchLatestVersion();
  final current = (await PackageInfo.fromPlatform()).version;

  final context = updaterNavigatorKey.currentContext;
  if (context == null || !context.mounted) {
    return;
  }
  final l10n = AppLocalizations.of(context);

  if (latest == null) {
    if (!silentIfNoUpdate) {
      await _showInfoDialog(context, l10n.updateCheckFailed);
    }
    return;
  }

  if (!_isNewer(latest, current)) {
    if (!silentIfNoUpdate) {
      await _showInfoDialog(context, l10n.updateUpToDate);
    }
    return;
  }

  await _showUpdateAvailableDialog(context, version: latest, current: current);
}

Future<void> _showInfoDialog(BuildContext context, String message) {
  final l10n = AppLocalizations.of(context);
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.updatesTitle),
      content: Text(message),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    ),
  );
}

Future<void> _showUpdateAvailableDialog(
  BuildContext context, {
  required String version,
  required String current,
}) {
  final l10n = AppLocalizations.of(context);
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.updateAvailableTitle),
      content: Text(l10n.updateAvailableMessage(version, current)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.updateLater),
        ),
        FilledButton(
          onPressed: () {
            Navigator.of(context).pop();
            unawaited(
              launchUrl(
                Uri.parse(_releasesPageUrl),
                mode: LaunchMode.externalApplication,
              ),
            );
          },
          child: Text(l10n.updateDownload),
        ),
      ],
    ),
  );
}

// --------------------------------------------------------------------------- #
// Public API: branches on installed vs portable; the UI stays agnostic.
// --------------------------------------------------------------------------- #

/// Startup check. Best-effort: failures are logged and never block startup.
/// No-op when checks are unavailable (dev) or the user disabled auto-check.
///
/// Installed build: WinSparkle attaches its update UI to the existing window,
/// so call only after the main window is shown. Portable build: polls the
/// appcast in the background and shows a Flutter dialog if an update exists.
Future<void> initDesktopUpdater({required bool autoCheckEnabled}) async {
  if (!autoCheckEnabled || !isUpdateCheckAvailable) {
    return;
  }
  if (!isInstalledBuild) {
    unawaited(_pollPortablePeriodically());
    return;
  }
  try {
    if (!await _ensureFeedConfigured()) {
      return;
    }
    await autoUpdater.setScheduledCheckInterval(_checkIntervalSeconds);
    // Silent check on startup: WinSparkle shows its own dialog only when
    // an update actually exists.
    await autoUpdater.checkForUpdates(inBackground: true);
  } catch (error) {
    debugPrint('goresave updater init failed: $error');
  }
}

/// Portable background poll: one check now, then every hour. WinSparkle owns
/// its own scheduler; for portable we run a plain timer loop. Guarded so a
/// re-enable can't stack a second loop; [_stopPortablePolling] cancels it.
Future<void> _pollPortablePeriodically() async {
  if (_portableTimer != null) {
    return;
  }
  _portableTimer =
      Timer.periodic(const Duration(seconds: _checkIntervalSeconds), (_) {
    unawaited(_runPortableCheck(silentIfNoUpdate: true));
  });
  await _runPortableCheck(silentIfNoUpdate: true);
}

/// Stops the portable background poll so disabling auto-check takes effect
/// immediately, matching WinSparkle's interval-0 behavior on the installed
/// build.
void _stopPortablePolling() {
  _portableTimer?.cancel();
  _portableTimer = null;
}

/// User-triggered check: always reports the result, including the up-to-date
/// and failure cases. Works even while automatic checks are off.
Future<void> checkForUpdatesManually() async {
  if (!isUpdateCheckAvailable) {
    return;
  }
  if (!isInstalledBuild) {
    await _runPortableCheck(silentIfNoUpdate: false);
    return;
  }
  try {
    if (!await _ensureFeedConfigured()) {
      return;
    }
    await autoUpdater.checkForUpdates();
  } catch (error) {
    debugPrint('goresave manual update check failed: $error');
  }
}

/// Applies the auto-check setting at runtime. Installed: interval 0 stops
/// WinSparkle's scheduled checks; re-enabling restores the interval and
/// checks once. Portable: starts the background poll on enable and cancels
/// the running poll on disable, so the change takes effect immediately.
Future<void> setAutoUpdateCheckEnabled(bool enabled) async {
  if (!isUpdateCheckAvailable) {
    return;
  }
  if (!isInstalledBuild) {
    if (enabled) {
      unawaited(_pollPortablePeriodically());
    } else {
      _stopPortablePolling();
    }
    return;
  }
  try {
    if (!await _ensureFeedConfigured()) {
      return;
    }
    await autoUpdater.setScheduledCheckInterval(
      enabled ? _checkIntervalSeconds : 0,
    );
    if (enabled) {
      await autoUpdater.checkForUpdates(inBackground: true);
    }
  } catch (error) {
    debugPrint('goresave updater toggle failed: $error');
  }
}
