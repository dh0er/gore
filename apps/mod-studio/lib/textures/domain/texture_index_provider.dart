import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// Loads (building on first use) the texture index for the configured game. Returns
/// asset_path -> packageIdString. Long on first build; the DLL call runs off the UI isolate.
///
/// `autoDispose`: the map is dropped when the Textures tab is no longer watched, so leaving
/// and re-entering the tab re-queries the Rust side (which validates the on-disk index's
/// build_id against the current .usmap and rebuilds if a game patch made it stale) — the UI
/// no longer serves an outdated asset->package-id map for the whole app session. Callers can
/// also force a refresh with `ref.invalidate(textureIndexProvider)`.
final textureIndexProvider = FutureProvider.autoDispose<Map<String, String>>((ref) async {
  final gamePath = ref.watch(gameExePathProvider);
  if (gamePath == null || gamePath.isEmpty) return {};
  final gameDir = gameRootFromExe(gamePath);
  // A non-empty but unresolvable path is a misconfiguration, not an empty game.
  // Throw so the UI surfaces it via the error branch instead of silently showing
  // "0 textures" (which reads as a valid install with no assets).
  if (gameDir == null) {
    throw StateError(
      'Could not locate the game install from the configured path:\n$gamePath\n\n'
      'Check the game path in Settings — it should point to the game executable.',
    );
  }
  final ffi = ModFfi(ref.read(coreServiceProvider));
  return ffi.textureIndex(gameDir);
});
