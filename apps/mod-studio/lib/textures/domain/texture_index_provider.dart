import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// Loads (building on first use) the texture index for the configured game. Returns
/// asset_path -> packageIdString. Long on first build; the DLL call runs off the UI isolate.
final textureIndexProvider = FutureProvider<Map<String, String>>((ref) async {
  final gamePath = ref.watch(gameExePathProvider);
  if (gamePath == null || gamePath.isEmpty) return {};
  final gameDir = gameRootFromExe(gamePath);
  if (gameDir == null) return {};
  final ffi = ModFfi(ref.read(coreServiceProvider));
  return ffi.textureIndex(gameDir);
});
