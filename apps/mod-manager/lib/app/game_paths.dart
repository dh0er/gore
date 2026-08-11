import 'dart:io';
import 'package:path/path.dart' as p;

final _windowsPath = p.Context(style: p.Style.windows);

/// Derive the game root (the folder containing `G1R/`) from the configured path.
///
/// That path may be the install **root** itself — the shared `game_path` set by
/// `gore config set game-path`/`detect` (and by another gore app) is the install
/// root, not an exe — OR the game **.exe** at
/// `<root>/G1R/Binaries/Win64/G1R-Win64-Shipping.exe`. The walk starts at the
/// path itself and climbs to the nearest ancestor holding a `G1R/` child, so a
/// root resolves to itself and an exe resolves to its root.
String? gameRootFromExe(String? path) {
  if (path == null || path.isEmpty) return null;
  var dir = path;
  for (var i = 0; i < 9; i++) {
    if (Directory(p.join(dir, 'G1R')).existsSync()) return dir;
    if (p.basename(dir) == 'G1R') return p.dirname(dir);
    final parent = p.dirname(dir);
    if (parent == dir) break;
    dir = parent;
  }
  return null;
}

/// Candidate root for read-only Manager diagnosis.
///
/// Existing selections use the same root normalization as status and Apply so
/// every Manager lane judges the same installation. A stale or malformed value
/// must still reach native preflight instead of being collapsed into "no path",
/// so an unresolved exact executable is normalized lexically and every other
/// non-blank value is forwarded unchanged.
String? diagnosticGameRootCandidate(String? path) {
  if (path == null || path.trim().isEmpty) return null;
  final existingRoot = gameRootFromExe(path);
  if (existingRoot != null &&
      Directory(p.join(existingRoot, 'G1R')).existsSync()) {
    return existingRoot;
  }
  final normalized = _windowsPath.normalize(path);
  final parts = _windowsPath.split(normalized);
  const tail = ['G1R', 'Binaries', 'Win64', 'G1R-Win64-Shipping.exe'];
  if (parts.length >= tail.length) {
    final offset = parts.length - tail.length;
    var exactExe = true;
    for (var index = 0; index < tail.length; index++) {
      if (parts[offset + index].toLowerCase() != tail[index].toLowerCase()) {
        exactExe = false;
        break;
      }
    }
    if (exactExe) {
      var root = normalized;
      for (var index = 0; index < tail.length; index++) {
        root = _windowsPath.dirname(root);
      }
      return root;
    }
  }
  return path;
}

/// The loose FMOD bank directory, or null if the game root can't be resolved.
String? fmodDesktopDir(String? exePath) {
  final root = gameRootFromExe(exePath);
  return root == null
      ? null
      : p.join(root, 'G1R', 'Content', 'FMOD', 'Desktop');
}

/// The bank files a user can mod (excludes Master/mixer banks with no audio).
const List<String> kModdableBanks = [
  'SFX.bank',
  'Music.bank',
  'CINEMATICS.bank',
  'VO.bank',
];
