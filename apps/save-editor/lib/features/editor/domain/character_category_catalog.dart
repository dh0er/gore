import 'dart:convert';

import 'package:flutter/services.dart';

enum CharacterCategory { human, creature, other }

/// Lightweight lookup over the bundled character-definition catalog.
///
/// Save memory events refer to actors in several forms (plain definition id,
/// spawned waypoint id, or id plus GUID). Resolving those forms against the
/// catalog lets statistics distinguish human NPCs from monsters without
/// guessing from display names.
class CharacterCategoryCatalog {
  const CharacterCategoryCatalog(this._categories, this._teachers);

  final Map<String, CharacterCategory> _categories;
  final Set<String> _teachers;

  CharacterCategory? categoryFor(String? reference) {
    if (reference == null) return null;
    for (final candidate in _referenceCandidates(reference)) {
      final category = _categories[candidate.toLowerCase()];
      if (category != null) return category;
    }
    return null;
  }

  bool isHuman(String? reference) =>
      categoryFor(reference) == CharacterCategory.human;

  bool isTeacher(String? reference) {
    if (reference == null) return false;
    return _referenceCandidates(
      reference,
    ).any((candidate) => _teachers.contains(candidate.toLowerCase()));
  }

  static Iterable<String> _referenceCandidates(String raw) sync* {
    var value = raw.trim().replaceAll("'", '');
    if (value.isEmpty) return;
    if (value.contains('/')) value = value.split('/').last;
    if (value.contains('.')) value = value.split('.').last;
    if (value.toLowerCase().endsWith('_c')) {
      value = value.substring(0, value.length - 2);
    }
    yield value;

    final lower = value.toLowerCase();
    final worldPointAt = lower.indexOf('-worldpointactor_');
    if (worldPointAt >= 0) {
      yield value.substring(0, worldPointAt);
      final actor = value.substring(worldPointAt + '-worldpointactor_'.length);
      if (actor.isNotEmpty) yield actor;
    }
    final waypointAt = lower.indexOf('-wp_');
    if (waypointAt >= 0) yield value.substring(0, waypointAt);
    final guid = RegExp(
      r'^(.*?)-[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$',
      caseSensitive: false,
    ).firstMatch(value);
    if (guid != null && guid.group(1)!.isNotEmpty) yield guid.group(1)!;
  }
}

const characterCategoryCatalogAsset = 'assets/npc_catalog.json';
const characterRoleCatalogAsset = 'assets/glossary_npc_catalog.json';

CharacterCategoryCatalog? _cachedCatalog;

Future<CharacterCategoryCatalog> loadCharacterCategoryCatalog({
  AssetBundle? bundle,
}) async {
  if (bundle != null) return _loadCharacterCategoryCatalog(bundle);
  final cached = _cachedCatalog;
  if (cached != null) return cached;
  final loaded = await _loadCharacterCategoryCatalog(rootBundle);
  _cachedCatalog = loaded;
  return loaded;
}

Future<CharacterCategoryCatalog> _loadCharacterCategoryCatalog(
  AssetBundle bundle,
) async {
  // Keep Flutter's AssetBundle from retaining a Future that belongs to a
  // disposed widget/test zone. This catalog has its own successful-result
  // cache above, so the bundle-level Future cache is both redundant and able
  // to poison a later load when an earlier widget is torn down mid-request.
  final texts = await Future.wait([
    bundle.loadString(characterCategoryCatalogAsset, cache: false),
    bundle.loadString(characterRoleCatalogAsset, cache: false),
  ]);
  final decoded = jsonDecode(texts.first);
  if (decoded is! List) {
    throw const FormatException('Character category catalog must be a list');
  }
  final categories = <String, CharacterCategory>{};
  for (final raw in decoded.whereType<Map>()) {
    final id = raw['id'];
    final categoryName = raw['category'];
    if (id is! String || id.isEmpty || categoryName is! String) continue;
    final category = CharacterCategory.values
        .where((value) => value.name == categoryName)
        .firstOrNull;
    if (category != null) categories[id.toLowerCase()] = category;
  }

  final roleDecoded = jsonDecode(texts.last);
  if (roleDecoded is! List) {
    throw const FormatException('Character role catalog must be a list');
  }
  final teachers = <String>{};
  for (final raw in roleDecoded.whereType<Map>()) {
    final segments = raw['segments'];
    if (segments is! List ||
        !segments.whereType<Map>().any(
          (segment) => (segment['roles'] as List?)?.contains('teacher') == true,
        )) {
      continue;
    }
    for (final identity in [raw['id'], raw['uniqueName']]) {
      if (identity is String && identity.isNotEmpty) {
        teachers.add(identity.toLowerCase());
      }
    }
  }
  return CharacterCategoryCatalog(
    Map.unmodifiable(categories),
    Set.unmodifiable(teachers),
  );
}
