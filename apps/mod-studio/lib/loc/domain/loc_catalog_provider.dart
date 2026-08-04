import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../catalog/domain/item_entry.dart';
import '../../core/providers.dart';
import '../game_lang.dart';

/// Loads the extracted localization catalog (`loc_catalog.json`) so item names
/// can be shown in the user's chosen language. The catalog is a flat map of
/// `{id: {set: text}}`; ids are lowercased on load so [resolveGameText] (which
/// looks up by the lowercased class id) finds them regardless of the original
/// casing in the `.lcache`.
///
/// Resolves the file path via the native `loc_status` command (`catalog_path`).
/// Returns an empty map when no catalog has been extracted yet, the file is
/// missing, or the native core is unavailable — callers then fall back to the
/// derived display name.
final locCatalogProvider =
    FutureProvider<Map<String, Map<String, String>>>((ref) async {
  final core = ref.watch(coreServiceProvider);
  final status = await core.execute('loc_status');
  if (status['present'] != true) return const {};
  final path = status['catalog_path'] as String?;
  if (path == null || path.isEmpty) return const {};

  final file = File(path);
  if (!file.existsSync()) return const {};

  try {
    final decoded = jsonDecode(await file.readAsString());
    if (decoded is! Map) return const {};
    final out = <String, Map<String, String>>{};
    decoded.forEach((id, sets) {
      if (id is String && sets is Map) {
        final inner = <String, String>{};
        sets.forEach((set, text) {
          if (set is String && text is String) inner[set] = text;
        });
        out[id.toLowerCase()] = inner;
      }
    });
    return out;
  } catch (_) {
    return const {};
  }
});

/// The name to show for [item]: its localized game name from [catalog] for
/// [lang] (falling back to English inside [resolveGameText]), or the derived
/// [CatalogItem.displayName] when the catalog has no entry for it.
String displayNameForItem(
  CatalogItem item,
  Map<String, Map<String, String>> catalog,
  GameLang lang,
) {
  final localized =
      resolveGameText(catalog, locIdForCatalogId(item.id), lang);
  return localized ?? item.displayName;
}
