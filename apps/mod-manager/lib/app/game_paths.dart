import 'dart:io';
import 'package:path/path.dart' as p;

/// Derive the game root (the folder containing `G1R/`) from the configured exe path.
/// The exe lives at `<root>/G1R/Binaries/Win64/G1R-Win64-Shipping.exe`.
String? gameRootFromExe(String? exePath) {
  if (exePath == null || exePath.isEmpty) return null;
  var dir = p.dirname(exePath);
  for (var i = 0; i < 8; i++) {
    if (Directory(p.join(dir, 'G1R')).existsSync()) return dir;
    if (p.basename(dir) == 'G1R') return p.dirname(dir);
    final parent = p.dirname(dir);
    if (parent == dir) break;
    dir = parent;
  }
  return null;
}

/// The loose FMOD bank directory, or null if the game root can't be resolved.
String? fmodDesktopDir(String? exePath) {
  final root = gameRootFromExe(exePath);
  return root == null ? null : p.join(root, 'G1R', 'Content', 'FMOD', 'Desktop');
}

/// The bank files a user can mod (excludes Master/mixer banks with no audio).
const List<String> kModdableBanks = [
  'SFX.bank',
  'Music.bank',
  'CINEMATICS.bank',
  'VO.bank',
];
