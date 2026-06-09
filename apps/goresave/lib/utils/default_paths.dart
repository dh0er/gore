import 'dart:io';

import 'package:path/path.dart' as p;

const _codecHostExe = 'goresave_g1r_codec_host.exe';
const _gameExe = 'G1R-Win64-Shipping.exe';

String defaultSaveRoot() {
  final localAppData = Platform.environment['LOCALAPPDATA'];
  if (localAppData != null && localAppData.isNotEmpty) {
    return '$localAppData\\G1R\\Saved\\SaveGames';
  }
  final userProfile = Platform.environment['USERPROFILE'];
  if (userProfile != null && userProfile.isNotEmpty) {
    return '$userProfile\\AppData\\Local\\G1R\\Saved\\SaveGames';
  }
  return p.join('G1R', 'Saved', 'SaveGames');
}

String defaultCodecHostPath() {
  final candidates = codecHostPathCandidates();
  for (final candidate in candidates) {
    if (File(candidate).existsSync()) return candidate;
  }
  return candidates.first;
}

List<String> codecHostPathCandidates({
  String? executablePath,
  String? currentDirectory,
}) {
  final exePath = executablePath ?? Platform.resolvedExecutable;
  final cwd = currentDirectory ?? Directory.current.path;
  final executableDir = p.dirname(exePath);
  return _uniquePaths([
    // Trusted shipped location first. The bare current-directory candidate is
    // intentionally omitted: this helper is executed for private decode/compress,
    // so a same-named binary dropped in the working directory must not be picked
    // up (mirrors the goresave_core.dll search order).
    p.join(executableDir, _codecHostExe),
    p.normalize(p.join(cwd, 'target', 'release', _codecHostExe)),
    p.normalize(p.join(cwd, 'target', 'debug', _codecHostExe)),
    p.normalize(p.join(cwd, '..', '..', 'target', 'release', _codecHostExe)),
    p.normalize(p.join(cwd, '..', '..', 'target', 'debug', _codecHostExe)),
    p.normalize(
      p.join(cwd, '..', '..', '..', 'target', 'release', _codecHostExe),
    ),
    p.normalize(
      p.join(cwd, '..', '..', '..', 'target', 'debug', _codecHostExe),
    ),
  ]);
}

String defaultGameExePath() {
  final candidates = gameExePathCandidates(
    steamLibraryRoots: steamLibraryRoots(),
  );
  for (final candidate in candidates) {
    if (File(candidate).existsSync()) return candidate;
  }
  return candidates.first;
}

List<String> gameExePathCandidates({Iterable<String>? steamLibraryRoots}) {
  final roots = (steamLibraryRoots ?? steamLibraryRootsFromEnvironment())
      .toList();
  final candidates = <String>[];
  for (final root in roots) {
    candidates.add(
      p.join(
        root,
        'steamapps',
        'common',
        'Gothic 1 Remake',
        'G1R',
        'Binaries',
        'Win64',
        _gameExe,
      ),
    );
    candidates.add(
      p.join(root, 'steamapps', 'common', 'Gothic 1 Remake', _gameExe),
    );
  }
  return _uniquePaths(candidates);
}

List<String> steamLibraryRoots() {
  final roots = steamLibraryRootsFromEnvironment();
  final all = <String>[...roots];
  for (final root in roots) {
    final vdf = File(p.join(root, 'steamapps', 'libraryfolders.vdf'));
    if (!vdf.existsSync()) continue;
    try {
      all.addAll(steamLibraryRootsFromVdf(vdf.readAsStringSync()));
    } catch (_) {
      continue;
    }
  }
  return _uniquePaths(all);
}

List<String> steamLibraryRootsFromEnvironment({
  Map<String, String>? environment,
  Iterable<String>? driveLetters,
}) {
  final env = environment ?? Platform.environment;
  final drives =
      driveLetters ?? const ['C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L'];
  return _uniquePaths(
    [
      env['STEAM_DIR'],
      env['SteamPath'],
      env['ProgramFiles(x86)'] == null
          ? null
          : p.join(env['ProgramFiles(x86)']!, 'Steam'),
      env['ProgramFiles'] == null
          ? null
          : p.join(env['ProgramFiles']!, 'Steam'),
      r'C:\Program Files (x86)\Steam',
      r'C:\Program Files\Steam',
      for (final drive in drives) '${drive.toUpperCase()}:\\SteamLibrary',
    ].whereType<String>(),
  );
}

List<String> steamLibraryRootsFromVdf(String content) {
  final roots = <String>[];
  final pathPattern = RegExp(r'"path"\s*"([^"]+)"', caseSensitive: false);
  for (final match in pathPattern.allMatches(content)) {
    roots.add(_vdfPath(match.group(1)!));
  }

  final oldStylePattern = RegExp(r'"\d+"\s*"([^"]+)"');
  for (final match in oldStylePattern.allMatches(content)) {
    final value = match.group(1)!;
    if (value.contains(r'\') || value.contains('/')) {
      roots.add(_vdfPath(value));
    }
  }

  return _uniquePaths(roots);
}

String _vdfPath(String value) {
  return p.normalize(value.replaceAll(r'\\', r'\'));
}

List<String> _uniquePaths(Iterable<String> paths) {
  final seen = <String>{};
  final out = <String>[];
  for (final path in paths) {
    final normalized = p.normalize(path);
    final key = normalized.toLowerCase();
    if (seen.add(key)) out.add(normalized);
  }
  return out;
}
