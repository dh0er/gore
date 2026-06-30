import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// The PRISTINE precompiled-script cache path for [root]. When a mod is deployed the live
/// `PrecompiledScript_Shipping.Cache` is the patched file and the vanilla bytes are preserved in
/// the deploy backup `…Cache.gore-bak`; prefer that backup so module listing and emitted source
/// reflect vanilla (matching the gore-as compile path, which also bases off the backup).
String _pristineScriptCachePath(String root) {
  final live = p.join(root, 'G1R', 'Script', 'PrecompiledScript_Shipping.Cache');
  final bak = '$live.gore-bak';
  return File(bak).existsSync() ? bak : live;
}

/// The pristine precompiled-script cache path for the configured game, or null if no game is set.
String? scriptCachePath(WidgetRef ref) {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return null;
  return _pristineScriptCachePath(root);
}

/// Lists vanilla modules for the "edit existing" picker. Empty list if no game / cache.
///
/// `autoDispose`: the list is dropped when the Scripts tab is no longer watched, so leaving
/// and re-entering the tab re-queries the Rust side against the current on-disk cache.
final scriptModulesProvider =
    FutureProvider.autoDispose<List<ScriptModuleInfo>>((ref) async {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return const [];
  final cache = _pristineScriptCachePath(root);
  // A configured game whose script cache is absent/unreadable must not throw an uncaught async
  // error into the "Edit existing" flow — return the documented empty list instead.
  if (!File(cache).existsSync()) return const [];
  final ffi = ModFfi(ref.read(coreServiceProvider));
  try {
    return await ffi.scriptListModules(cache);
  } catch (_) {
    return const [];
  }
});
