import 'dart:io';

import 'package:path/path.dart' as p;

/// Shared on-disk locations for the gore suite.
///
/// All tools (gore-cli, gore-save, gore-mod) keep their per-user data under ONE
/// shared directory named `gore` so a single localization extraction
/// serves every tool. This app's persisted JSON lives in the `gore-save`
/// subfolder; the loc catalog stays directly in `gore`.
///
/// The base-dir resolution mirrors the Rust side (`gore_loc::paths`) exactly:
/// - Windows: `%LOCALAPPDATA%` (falls back to `%APPDATA%`) then `\gore`
/// - macOS:   `$HOME/Library/Application Support/gore`
/// - Linux:   `$XDG_DATA_HOME/gore` (if set) else `$HOME/.local/share/gore`

/// The shared `gore` data directory root.
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
  return p.join(base, 'gore');
}

/// This app's settings folder under the shared umbrella: `<gore>/gore-save`.
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

/// One-time migration for the breaking `gore-tools` → `gore` shared-dir rename.
///
/// Copies every file from the legacy umbrella directory into the new one
/// wherever the new dir doesn't already have it, so users upgrading across the
/// rename keep their settings, language choice, and extracted loc/texture
/// caches (which live at the umbrella root, not per app). Best effort: never
/// throws, so a migration failure can't block startup.
void migrateLegacyUmbrellaDir(Map<String, String> env) {
  try {
    final newDir = Directory(goreToolsDir(environment: env));
    final legacy = Directory(p.join(p.dirname(newDir.path), 'gore-tools'));
    if (!legacy.existsSync()) return;
    final marker = File(p.join(newDir.path, '.migrated-from-gore-tools'));
    if (marker.existsSync()) return;
    for (final entity in legacy.listSync(recursive: true)) {
      if (entity is! File) continue;
      final rel = p.relative(entity.path, from: legacy.path);
      final target = File(p.join(newDir.path, rel));
      if (target.existsSync()) continue; // never clobber newer data
      target.parent.createSync(recursive: true);
      entity.copySync(target.path);
    }
    marker.createSync(recursive: true);
  } catch (_) {
    // A failed migration must never block startup.
  }
}

/// Runs [migrateLegacyUmbrellaDir] against the real environment, skipping
/// widget-test runs so tests never touch on-disk user data. Call once at
/// startup, before any settings or loc caches are read.
void migrateLegacyUmbrellaDirForPlatform() {
  if (Platform.environment.containsKey('FLUTTER_TEST')) return;
  migrateLegacyUmbrellaDir(Platform.environment);
}
