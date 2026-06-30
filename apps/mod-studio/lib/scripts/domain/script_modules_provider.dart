import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// The vanilla precompiled-script cache path for the configured game, or null if no game is set.
String? scriptCachePath(WidgetRef ref) {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return null;
  return p.join(root, 'G1R', 'Script', 'PrecompiledScript_Shipping.Cache');
}

/// Lists vanilla modules for the "edit existing" picker. Empty list if no game / cache.
///
/// `autoDispose`: the list is dropped when the Scripts tab is no longer watched, so leaving
/// and re-entering the tab re-queries the Rust side against the current on-disk cache.
final scriptModulesProvider =
    FutureProvider.autoDispose<List<ScriptModuleInfo>>((ref) async {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return const [];
  final cache = p.join(root, 'G1R', 'Script', 'PrecompiledScript_Shipping.Cache');
  final ffi = ModFfi(ref.read(coreServiceProvider));
  return ffi.scriptListModules(cache);
});
