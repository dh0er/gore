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

/// Stable URL: CI keeps a fixed `gore-save-editor-appcast` release whose single
/// asset it overwrites on every release, so this feed is independent of which
/// product happens to hold the repo's "latest" pointer.
const _appcastUrl =
    'https://github.com/dh0er/gore/releases/download/gore-save-editor-appcast/appcast-windows.xml';

/// Where the portable build sends users to grab the new build. Pinned to the
/// advertised version's own release, which lists both the installer and the
/// portable zip — `releases/latest` would send them to whichever product
/// released most recently.
String _releasePageUrl(String version) =>
    'https://github.com/dh0er/gore/releases/tag/gore-save-editor-v$version';

const _checkIntervalSeconds = 3600;

/// Cap for each feed network step. Without it a stalled connection or a server
/// that opens a response and never finishes it would hang the check forever,
/// and with it the in-flight guard below.
const _feedTimeout = Duration(seconds: 20);

/// Attached to the GoRouter so a background update check (which has no widget
/// context of its own) can still surface a dialog. See [router.dart].
final updaterNavigatorKey = GlobalKey<NavigatorState>();

bool? _innoInstalled;
bool _feedConfigured = false;

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
  final client = HttpClient()..connectionTimeout = _feedTimeout;
  try {
    final request = await client
        .getUrl(Uri.parse(_appcastUrl))
        .timeout(_feedTimeout);
    final response = await request.close().timeout(_feedTimeout);
    if (response.statusCode != HttpStatus.ok) {
      return null;
    }
    // A response can stall mid-body, so bound the read as well as the connect.
    final body = await response
        .transform(utf8.decoder)
        .join()
        .timeout(_feedTimeout);
    final match = RegExp(
      r'<sparkle:version>\s*([^<\s]+)\s*</sparkle:version>',
    ).firstMatch(body);
    return match?.group(1);
  } catch (error) {
    debugPrint('goresave portable update check failed: $error');
    return null;
  } finally {
    client.close(force: true);
  }
}

/// What a finished portable check has to tell the user.
enum PortableUpdateReport { checkFailed, upToDate, downloadPageFailed }

/// Everything outside this file that a portable check touches: the feed, this
/// build's version, the two prompts, and the browser.
///
/// Injected because the interesting behaviour here is timing — a background
/// tick meeting a manual click, a stalled feed, a prompt left unanswered — and
/// none of that is reachable through the real network, real windows, and a
/// release-mode-only entry point.
@visibleForTesting
class PortableUpdateHooks {
  const PortableUpdateHooks({
    required this.fetchLatestVersion,
    required this.currentVersion,
    required this.report,
    required this.askDownload,
    required this.openReleasePage,
  });

  /// The advertised version, or null when the feed could not be read.
  final Future<String?> Function() fetchLatestVersion;
  final Future<String> Function() currentVersion;

  /// Tells the user how the check ended. [releaseUrl] is only meaningful for
  /// [PortableUpdateReport.downloadPageFailed].
  final Future<void> Function(PortableUpdateReport report, String releaseUrl)
  report;

  /// Shows the update prompt; true when the user chose Download.
  final Future<bool> Function(String latest, String current) askDownload;

  /// Opens the release page; false when the shell refused or failed.
  final Future<bool> Function(String latest) openReleasePage;
}

/// Runs portable update checks, one at a time.
///
/// The whole point of this class is the queueing rules, which is why they live
/// somewhere a test can reach:
///  * a background tick yields to a running check — it exists to be
///    unobtrusive, and the running check already covers this interval;
///  * a manual check waits its turn and then runs, because the button promises
///    a result; it re-reads the slot after each wait so two queued clicks
///    cannot both wake up and start overlapping checks;
///  * the slot is claimed before the check's first suspension point, since an
///    async body runs up to its first await before returning its future;
///  * a check never throws, because callers fire it unawaited and a manual
///    check may be awaiting someone else's.
@visibleForTesting
class PortableUpdateChecker {
  PortableUpdateChecker(
    this.hooks, {
    this.interval = const Duration(seconds: _checkIntervalSeconds),
  });

  final PortableUpdateHooks hooks;
  final Duration interval;

  Future<void>? _active;
  Timer? _timer;

  /// True while the background poll is armed.
  bool get isPolling => _timer != null;

  /// True while a check is running, including while its prompt is open.
  @visibleForTesting
  bool get isChecking => _active != null;

  Future<void> run({required bool background}) async {
    if (background && _active != null) return;
    var active = _active;
    while (active != null) {
      await active;
      active = _active;
    }
    final done = Completer<void>();
    _active = done.future;
    try {
      await _runOnce(background: background);
    } finally {
      if (identical(_active, done.future)) _active = null;
      done.complete();
    }
  }

  Future<void> _runOnce({required bool background}) async {
    try {
      await _body(background: background);
    } catch (error) {
      debugPrint('goresave portable update check failed: $error');
    }
  }

  Future<void> _body({required bool background}) async {
    final latest = await hooks.fetchLatestVersion();
    final current = await hooks.currentVersion();

    if (latest == null) {
      if (!background) {
        await hooks.report(PortableUpdateReport.checkFailed, '');
      }
      return;
    }
    if (!isNewerVersion(latest, current)) {
      if (!background) {
        await hooks.report(PortableUpdateReport.upToDate, '');
      }
      return;
    }
    // Auto-check may have been switched off while this tick was fetching; a
    // cancelled poll must not still produce an unsolicited prompt. A manual
    // check always shows.
    if (background && !isPolling) return;

    if (!await hooks.askDownload(latest, current)) return;
    // Still inside the lock: doing this in the prompt's own button callback
    // would outlive the prompt, freeing the slot while the follow-up message
    // is on screen.
    if (await hooks.openReleasePage(latest)) return;
    await hooks.report(
      PortableUpdateReport.downloadPageFailed,
      _releasePageUrl(latest),
    );
  }

  /// One check now, then one per [interval]. Guarded so re-enabling cannot
  /// stack a second poll.
  Future<void> startPolling() async {
    if (_timer != null) return;
    _timer = Timer.periodic(interval, (_) {
      unawaited(run(background: true));
    });
    await run(background: true);
  }

  /// Stops the poll so disabling auto-check takes effect immediately, matching
  /// WinSparkle's interval-0 behaviour on the installed build.
  void stopPolling() {
    _timer?.cancel();
    _timer = null;
  }
}

/// Compares dotted numeric versions (e.g. "0.4.0"). Returns true when [latest]
/// is strictly newer than [current]. Non-numeric or missing components count
/// as 0, so "0.4" < "0.4.1".
@visibleForTesting
bool isNewerVersion(String latest, String current) {
  final a = latest.split('.');
  final b = current.split('.');
  for (var i = 0; i < (a.length > b.length ? a.length : b.length); i++) {
    final x = i < a.length ? int.tryParse(a[i]) ?? 0 : 0;
    final y = i < b.length ? int.tryParse(b[i]) ?? 0 : 0;
    if (x != y) return x > y;
  }
  return false;
}

/// Production wiring: real feed, real package info, real dialogs, real browser.
final PortableUpdateHooks _realPortableHooks = PortableUpdateHooks(
  fetchLatestVersion: _fetchLatestVersion,
  currentVersion: () async => (await PackageInfo.fromPlatform()).version,
  report: (report, releaseUrl) async {
    final context = updaterNavigatorKey.currentContext;
    if (context == null || !context.mounted) return;
    final l10n = AppLocalizations.of(context);
    await _showInfoDialog(context, switch (report) {
      PortableUpdateReport.checkFailed => l10n.updateCheckFailed,
      PortableUpdateReport.upToDate => l10n.updateUpToDate,
      PortableUpdateReport.downloadPageFailed => l10n.updateOpenFailed(
        releaseUrl,
      ),
    });
  },
  askDownload: (latest, current) async {
    final context = updaterNavigatorKey.currentContext;
    if (context == null || !context.mounted) return false;
    return _showUpdateAvailableDialog(
      context,
      version: latest,
      current: current,
    );
  },
  openReleasePage: _openReleasePage,
);

PortableUpdateChecker _portable = PortableUpdateChecker(_realPortableHooks);

/// Swaps the checker so tests can drive the queueing rules directly. Passing
/// null restores the production wiring.
@visibleForTesting
void debugSetPortableUpdateChecker(PortableUpdateChecker? checker) {
  _portable.stopPolling();
  _portable = checker ?? PortableUpdateChecker(_realPortableHooks);
}

Future<void> _showInfoDialog(BuildContext context, String message) {
  final l10n = AppLocalizations.of(context);
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.updatesTitle),
      content: SelectionArea(child: Text(message)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.ok),
        ),
      ],
    ),
  );
}

/// Opens the release page in the user's browser. Returns false when the shell
/// refused or failed — `launchUrl` reports both by returning false and by
/// throwing — so the caller can say so rather than leave a dead button.
Future<bool> _openReleasePage(String version) async {
  try {
    return await launchUrl(
      Uri.parse(_releasePageUrl(version)),
      mode: LaunchMode.externalApplication,
    );
  } catch (error) {
    debugPrint('goresave could not open the release page: $error');
    return false;
  }
}

/// Asks whether to download. Returns true when the user chose Download; the
/// caller does the launching so it stays inside the check's lock.
Future<bool> _showUpdateAvailableDialog(
  BuildContext context, {
  required String version,
  required String current,
}) async {
  final l10n = AppLocalizations.of(context);
  final download = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.updateAvailableTitle),
      content: Text(l10n.updateAvailableMessage(version, current)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.updateLater),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.updateDownload),
        ),
      ],
    ),
  );
  return download ?? false;
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
    unawaited(_portable.startPolling());
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

/// User-triggered check: always reports the result, including the up-to-date
/// and failure cases. Works even while automatic checks are off.
Future<void> checkForUpdatesManually() async {
  if (!isUpdateCheckAvailable) {
    return;
  }
  if (!isInstalledBuild) {
    await _portable.run(background: false);
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
      unawaited(_portable.startPolling());
    } else {
      _portable.stopPolling();
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
