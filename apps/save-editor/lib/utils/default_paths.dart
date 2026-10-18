import 'dart:io';

import 'package:path/path.dart' as p;

String resolveSaveRoot({
  String? override,
  String? stored,
  required String fallback,
  required bool isLinux,
}) {
  if (override != null) return override;
  final migrateLegacyLinuxDefault =
      isLinux &&
      stored != null &&
      !p.isAbsolute(stored) &&
      p.normalize(stored) == p.join('G1R', 'Saved', 'SaveGames') &&
      p.isAbsolute(fallback);
  return migrateLegacyLinuxDefault ? fallback : stored ?? fallback;
}

String defaultSaveRoot({Map<String, String>? environment, bool? isLinux}) {
  final env = environment ?? Platform.environment;
  final localAppData = env['LOCALAPPDATA'];
  if (localAppData != null && localAppData.isNotEmpty) {
    return '$localAppData\\G1R\\Saved\\SaveGames';
  }
  final userProfile = env['USERPROFILE'];
  if (userProfile != null && userProfile.isNotEmpty) {
    return '$userProfile\\AppData\\Local\\G1R\\Saved\\SaveGames';
  }
  if (isLinux ?? Platform.isLinux) {
    final protonSaveRoot = _findLinuxProtonSaveRoot(env);
    if (protonSaveRoot != null) return protonSaveRoot;
  }
  return p.join('G1R', 'Saved', 'SaveGames');
}

String? _findLinuxProtonSaveRoot(Map<String, String> environment) {
  final home = environment['HOME'];
  final steamRoots = <String>{};
  final explicitPrefix = environment['STEAM_COMPAT_DATA_PATH'];
  if (explicitPrefix != null && explicitPrefix.isNotEmpty) {
    steamRoots.add(explicitPrefix);
  }

  if (home != null && home.isNotEmpty) {
    steamRoots.addAll([
      p.join(home, '.steam', 'steam'),
      p.join(home, '.steam', 'root'),
      p.join(home, '.local', 'share', 'Steam'),
      p.join(
        home,
        '.var',
        'app',
        'com.valvesoftware.Steam',
        '.local',
        'share',
        'Steam',
      ),
    ]);
  }
  for (final key in ['STEAM_DIR', 'STEAM_PATH']) {
    final value = environment[key];
    if (value != null && value.isNotEmpty) steamRoots.add(value);
  }

  final compatDataRoots = <String>{};
  final explicitCompatData = explicitPrefix;
  if (explicitCompatData != null && explicitCompatData.isNotEmpty) {
    compatDataRoots.add(explicitCompatData);
  }
  for (final steamRoot in steamRoots.toList()) {
    final steamApps = p.join(steamRoot, 'steamapps');
    compatDataRoots.add(p.join(steamApps, 'compatdata'));
    final libraryFolders = File(p.join(steamApps, 'libraryfolders.vdf'));
    if (!libraryFolders.existsSync()) continue;
    try {
      final contents = libraryFolders.readAsStringSync();
      for (final match in RegExp(r'"path"\s+"([^"]+)"').allMatches(contents)) {
        final libraryPath = match.group(1);
        if (libraryPath != null && libraryPath.isNotEmpty) {
          compatDataRoots.add(p.join(libraryPath, 'steamapps', 'compatdata'));
        }
      }
    } on FileSystemException {
      // An unreadable library list should not prevent using another Steam root.
    }
  }

  String? firstExistingSaveRoot;
  String? mostPopulatedSaveRoot;
  var mostPopulatedSaveCount = 0;
  for (final compatDataRoot in compatDataRoots) {
    final isExplicitPrefix = compatDataRoot == explicitCompatData;
    final prefixes = isExplicitPrefix
        ? [Directory(compatDataRoot)]
        : _directoriesIn(Directory(compatDataRoot));
    for (final prefix in prefixes) {
      final usersDirectory = Directory(
        p.join(prefix.path, 'pfx', 'drive_c', 'users'),
      );
      final users = _directoriesIn(usersDirectory);
      if (users.isEmpty && isExplicitPrefix) {
        users.add(Directory(p.join(usersDirectory.path, 'steamuser')));
      }
      for (final userDirectory in users) {
        final saveRoot = p.join(
          userDirectory.path,
          'AppData',
          'Local',
          'G1R',
          'Saved',
          'SaveGames',
        );
        final directory = Directory(saveRoot);
        if (!directory.existsSync()) continue;
        firstExistingSaveRoot ??= saveRoot;
        final saveCount = _saveFileCount(directory);
        if (saveCount > mostPopulatedSaveCount) {
          mostPopulatedSaveRoot = saveRoot;
          mostPopulatedSaveCount = saveCount;
        }
      }
    }
  }

  // Prefer the prefix that actually contains saves when Steam has several
  // libraries or old Proton prefixes; otherwise use an existing save folder.
  return mostPopulatedSaveRoot ?? firstExistingSaveRoot;
}

List<Directory> _directoriesIn(Directory parent) {
  if (!parent.existsSync()) return [];
  try {
    return parent.listSync(followLinks: true).whereType<Directory>().toList();
  } on FileSystemException {
    return [];
  }
}

int _saveFileCount(Directory directory) {
  try {
    return directory
        .listSync(followLinks: false)
        .whereType<File>()
        .where((file) => p.extension(file.path).toLowerCase() == '.sav')
        .length;
  } on FileSystemException {
    return 0;
  }
}
