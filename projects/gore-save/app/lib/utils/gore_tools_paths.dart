import 'dart:io';

import 'package:path/path.dart' as p;

/// Shared on-disk locations for the gore-tools suite.
///
/// All tools (gore-cli, gore-save, gore-mod) keep their per-user data under ONE
/// shared directory named `gore-tools` so a single localization extraction
/// serves every tool. This app's persisted JSON lives in the `gore-save`
/// subfolder; the loc catalog stays directly in `gore-tools`.
///
/// The base-dir resolution mirrors the Rust side (`gore_core::paths`) exactly:
/// - Windows: `%LOCALAPPDATA%` (falls back to `%APPDATA%`) then `\gore-tools`
/// - macOS:   `$HOME/Library/Application Support/gore-tools`
/// - Linux:   `$XDG_DATA_HOME/gore-tools` (if set) else `$HOME/.local/share/gore-tools`

/// The shared `gore-tools` data directory root.
String goreToolsDir({Map<String, String>? environment}) {
  final env = environment ?? Platform.environment;
  final String base;
  if (Platform.isWindows) {
    base = env['LOCALAPPDATA'] ?? env['APPDATA'] ?? Directory.current.path;
  } else if (Platform.isMacOS) {
    final home = env['HOME'];
    base = home == null
        ? Directory.current.path
        : p.join(home, 'Library', 'Application Support');
  } else {
    final xdg = env['XDG_DATA_HOME'];
    final home = env['HOME'];
    base = (xdg != null && xdg.isNotEmpty)
        ? xdg
        : (home == null
              ? Directory.current.path
              : p.join(home, '.local', 'share'));
  }
  return p.join(base, 'gore-tools');
}

/// This app's settings folder under the shared umbrella: `<gore-tools>/gore-save`.
String goreSaveSettingsDir({Map<String, String>? environment}) {
  return p.join(goreToolsDir(environment: environment), 'gore-save');
}

/// Best-effort one-time migration of a settings file from a previous location.
///
/// If [target] is missing but [legacy] exists, copy legacy → target (creating
/// parent dirs as needed) and keep the old file as a backup. Any failure is
/// swallowed so a migration problem never crashes startup.
void migrateLegacySettingsFile(File legacy, File target) {
  try {
    if (target.existsSync() || !legacy.existsSync()) return;
    target.parent.createSync(recursive: true);
    legacy.copySync(target.path);
  } catch (_) {
    // Fall back to defaults; the legacy file is left untouched.
  }
}
