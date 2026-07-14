import 'game_lang.dart';

/// Resolves a gameplay attribute id to the same localized label the game uses.
///
/// Attribute strings are stored under keys such as
/// `attributeset_health_maxhealth`. The save carries both the attribute id
/// (`MaxHealth`) and, on typed saves, its owning class
/// (`/Script/G1R.AttributeSet_Health`), so the exact catalog key can normally
/// be reconstructed without guessing.
String localizedAttributeName(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String attributeId, {
  String? setClass,
}) {
  final fallback = readableAttributeName(attributeId);
  if (catalog.isEmpty || attributeId.trim().isEmpty) return fallback;

  final id = _catalogPart(attributeId);
  if (id.isEmpty) return fallback;

  final set = _attributeSetName(setClass);
  if (set != null) {
    final exact = resolveGameText(catalog, 'attributeset_${set}_$id', lang);
    if (_nonBlank(exact) case final value?) return value;
  }

  // Legacy summaries do not include AttributeSetByClass. Cover every
  // player-facing attribute currently present in the extracted game catalog.
  final knownSet = _knownAttributeSets[id];
  if (knownSet != null) {
    final known = resolveGameText(
      catalog,
      'attributeset_${knownSet}_$id',
      lang,
    );
    if (_nonBlank(known) case final value?) return value;
  }

  // Future game versions may add a localized attribute without updating the
  // known map. A unique catalog suffix is still safe to use; ambiguous matches
  // deliberately fall back to a readable id instead of showing the wrong text.
  final suffix = '_$id';
  String? matchingKey;
  for (final key in catalog.keys) {
    if (!key.startsWith('attributeset_') ||
        key.endsWith('_description') ||
        !key.endsWith(suffix)) {
      continue;
    }
    if (matchingKey != null) return fallback;
    matchingKey = key;
  }
  if (matchingKey != null) {
    final matched = resolveGameText(catalog, matchingKey, lang);
    if (_nonBlank(matched) case final value?) return value;
  }
  return fallback;
}

/// Human-friendly fallback for attributes absent from the loc catalog.
/// Technical ids remain stable in the underlying edit paths; only their label
/// is prettified here (`DamageMultiplier` -> `Damage multiplier`).
String readableAttributeName(String attributeId) {
  final trimmed = attributeId.trim();
  if (trimmed.isEmpty) return attributeId;
  if (trimmed == 'SkillPoints') return 'Skill points (LP)';

  var text = trimmed.replaceAll(RegExp(r'[_-]+'), ' ');
  text = text.replaceAllMapped(
    RegExp(r'([A-Z]+)([A-Z][a-z])'),
    (match) => '${match[1]} ${match[2]}',
  );
  text = text.replaceAllMapped(
    RegExp(r'([a-z0-9])([A-Z])'),
    (match) => '${match[1]} ${match[2]}',
  );
  text = text.replaceAll(RegExp(r'\s+'), ' ').trim();
  if (text.isEmpty) return trimmed;
  return '${text[0].toUpperCase()}${text.substring(1).toLowerCase()}';
}

String? _attributeSetName(String? setClass) {
  var value = setClass?.trim() ?? '';
  if (value.isEmpty) return null;
  if (value.startsWith('{') && value.endsWith('}')) {
    value = value.substring(1, value.length - 1);
  }
  value = value.split('/').last.split('.').last;
  const prefix = 'attributeset_';
  if (value.toLowerCase().startsWith(prefix)) {
    value = value.substring(prefix.length);
  }
  final normalized = _catalogPart(value);
  return normalized.isEmpty ? null : normalized;
}

String _catalogPart(String value) => value
    .trim()
    .toLowerCase()
    .replaceAll(RegExp(r'[^a-z0-9_]+'), '_')
    .replaceAll(RegExp(r'^_+|_+$'), '');

String? _nonBlank(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}

const _knownAttributeSets = <String, String>{
  'health': 'health',
  'maxhealth': 'health',
  'mana': 'mana',
  'maxmana': 'mana',
  'magicianlevel': 'mana',
  'strength': 'strength',
  'dexterity': 'dexterity',
  'level': 'levelprogression',
  'experience': 'levelprogression',
  'skillpoints': 'levelprogression',
  'toughness': 'levelprogression',
  'resistance_blunt': 'armor',
  'resistance_edge': 'armor',
  'resistance_point': 'armor',
  'resistance_fire': 'armor',
  'resistance_energy': 'armor',
  'resistance_ice': 'armor',
  'resistance_wind': 'armor',
  'resistance_falling': 'armor',
};
