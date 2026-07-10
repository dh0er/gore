import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/providers/data_providers.dart';

/// The currently selected [GameLang], derived from the persisted locale code.
final currentGameLangProvider = Provider<GameLang>((ref) {
  return gameLangByCode(ref.watch(localeProvider));
});

/// Monotonic counter bumped after a successful localization extraction so the
/// catalog future re-runs and picks up the freshly written `loc_catalog.json`.
final locCatalogReloadProvider = StateProvider<int>((ref) => 0);

/// Loads the extracted localization catalog (`loc_catalog.json`) as
/// `id -> {set -> text}`. The id keys are lowercased to match
/// [resolveGameText], which lowercases the lookup id. Returns an empty map when
/// no catalog has been extracted yet (or on any read/parse error), so callers
/// can always fall back to their existing derived/raw names.
final locCatalogProvider =
    FutureProvider<Map<String, Map<String, String>>>((ref) async {
  // Re-run when an extraction completes.
  ref.watch(locCatalogReloadProvider);

  final core = ref.watch(coreServiceProvider);
  try {
    final response = await core.execute('loc_status');
    if (response['ok'] != true) return const {};
    final data = (response['data'] as Map?)?.cast<String, Object?>();
    if (data == null || data['present'] != true) return const {};
    final path = data['catalogPath'] as String?;
    if (path == null || path.isEmpty) return const {};
    final file = File(path);
    if (!file.existsSync()) return const {};
    final decoded = jsonDecode(await file.readAsString());
    if (decoded is! Map) return const {};
    final out = <String, Map<String, String>>{};
    decoded.forEach((id, sets) {
      if (id is! String || sets is! Map) return;
      final inner = <String, String>{};
      sets.forEach((set, text) {
        if (set is String && text is String) inner[set] = text;
      });
      if (inner.isNotEmpty) out[id.toLowerCase()] = inner;
    });
    return out;
  } catch (_) {
    return const {};
  }
});

/// Convenience: resolve a catalog class id to its localized name for the
/// current language, or null when unavailable. Synchronous against the
/// already-loaded catalog map (callers watch [locCatalogProvider] for the map).
String? localizedGameName(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String catalogId,
) {
  if (catalog.isEmpty || catalogId.isEmpty) return null;
  return resolveGameText(catalog, locIdForCatalogId(catalogId), lang);
}
