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
  CharacterCategoryCatalog(this._categories, this._teachers)
    : _unprefixed = _strippedIndex(_categories, compact: false),
      _unprefixedCompact = _strippedIndex(_categories, compact: true),
      _unnumbered = _strippedIndex(
        _categories,
        compact: false,
        unnumbered: true,
      );

  final Map<String, CharacterCategory> _categories;
  final Set<String> _teachers;

  /// The same table with each definition's leading kind segment removed —
  /// `Creature_Bloodfly` also under `bloodfly`, `Orc_OW_OPS_OrcPeasantM01_2068`
  /// under `ow_ops_orcpeasantm01_2068`. That is the form the save writes.
  final Map<String, CharacterCategory> _unprefixed;

  /// The same again folded to letters and digits only, for the ids that dropped
  /// the underscores (`LizardFire` for `Creature_Lizard_Fire`) or spell
  /// themselves out with spaces (`Minecrawler Nymph`).
  final Map<String, CharacterCategory> _unprefixedCompact;

  /// The same again with the definition's trailing instance number removed —
  /// `OC_GRD_Guard18_238` also under `grd_guard18`. A save can carry the same
  /// character under a different number (`FM_GRD_Guard18_300N`), which no
  /// exact form ever matches.
  final Map<String, CharacterCategory> _unnumbered;

  /// A definition's trailing instance number, e.g. `_238` or `_300N`.
  static final RegExp _instanceNumber = RegExp(r'_\d+n?$');

  /// Letters and digits only, lowercased.
  static String _fold(String value) =>
      value.toLowerCase().replaceAll(RegExp('[^a-z0-9]'), '');

  /// A stripped key that two definitions of DIFFERENT kinds would share is left
  /// out: guessing there would mark somebody the wrong species.
  static Map<String, CharacterCategory> _strippedIndex(
    Map<String, CharacterCategory> categories, {
    required bool compact,
    bool unnumbered = false,
  }) {
    final seen = <String, CharacterCategory>{};
    final ambiguous = <String>{};
    for (final entry in categories.entries) {
      final cut = entry.key.indexOf('_');
      if (cut <= 0 || cut == entry.key.length - 1) continue;
      var key = entry.key.substring(cut + 1);
      if (unnumbered) {
        final trimmed = key.replaceFirst(_instanceNumber, '');
        if (trimmed == key || trimmed.isEmpty) continue;
        key = trimmed;
      }
      if (compact) key = _fold(key);
      final previous = seen[key];
      if (previous != null && previous != entry.value) {
        ambiguous.add(key);
      } else {
        seen[key] = entry.value;
      }
    }
    for (final key in ambiguous) {
      seen.remove(key);
    }
    return seen;
  }

  /// A reference reduced to the form [_unnumbered] is keyed by: everything
  /// after the leading segment, without the trailing instance number. Empty
  /// when the reference carries neither, so it can never match.
  static String _unnumberedKey(String lower) {
    final cut = lower.indexOf('_');
    if (cut <= 0 || cut == lower.length - 1) return '';
    final tail = lower.substring(cut + 1);
    final trimmed = tail.replaceFirst(_instanceNumber, '');
    return trimmed == tail ? '' : trimmed;
  }

  CharacterCategory? categoryFor(String? reference) {
    if (reference == null) return null;
    for (final candidate in _referenceCandidates(reference)) {
      final lower = candidate.toLowerCase();
      final category = _categories[lower];
      if (category != null) return category;
      // A save id drops the definition's leading kind segment — `Lizard-WP_…`
      // for `Creature_Lizard`, `OW_OPS_OrcPeasantM01_2068-…` for the `Orc_`
      // definition of the same name — and sometimes its underscores too
      // (`LizardFire` for `Creature_Lizard_Fire`).
      final stripped = _unprefixed[lower] ?? _unprefixedCompact[_fold(lower)];
      if (stripped != null) return stripped;
      // Last resort, the same key without its own leading segment and without
      // the instance number: the save's `FM_GRD_Guard18_300N` is the catalog's
      // `OC_GRD_Guard18_238` — same guard, different world, different number.
      final unnumbered = _unnumbered[_unnumberedKey(lower)];
      if (unnumbered != null) return unnumbered;
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
    // Last, the leading segment on its own — the definition name the save
    // spawned this actor from. Every rule above reads a particular id SHAPE,
    // so whether a character resolved at all came down to which shape it had:
    // `Wolf-WP_…` resolved through the waypoint rule while `Wolf-OW_…_WP-1`
    // resolved through nothing, and the same animal came out a creature 57
    // times and an unknown 9 times. Yielded last, so a more specific candidate
    // still wins — the mercenary `NC_ORG_Wolf_855-WorldPointActor_wolf` is a
    // man before his waypoint's name can make him a wolf.
    final firstSegment = value.indexOf('-');
    if (firstSegment > 0) yield value.substring(0, firstSegment);
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
