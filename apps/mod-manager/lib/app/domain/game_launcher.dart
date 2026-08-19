import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

/// Starts the game executable at [exePath]. Returns false when the process
/// could not be spawned at all; a game that starts and then exits on its own
/// is not this function's problem.
typedef GameLauncher = Future<bool> Function(String exePath);

/// Injected so widget tests never spawn a real process.
final gameLauncherProvider = Provider<GameLauncher>((ref) => startGame);

/// Launches the game detached, from its own directory.
///
/// Detached on purpose: the Manager must not become the game's parent process,
/// or closing the Manager would take the game with it. The working directory is
/// the executable's own folder because the engine resolves relative content
/// paths from there.
Future<bool> startGame(String exePath) async {
  try {
    await Process.start(
      exePath,
      const [],
      workingDirectory: p.dirname(exePath),
      mode: ProcessStartMode.detached,
    );
    return true;
  } catch (error) {
    debugPrint('gore-manager could not start the game: $error');
    return false;
  }
}
