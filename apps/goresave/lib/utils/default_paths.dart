import 'dart:io';

import 'package:path/path.dart' as p;

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
