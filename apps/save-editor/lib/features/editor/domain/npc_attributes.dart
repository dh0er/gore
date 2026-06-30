/// One pending `private.typed.setValue` edit produced by the NPC attribute
/// editor. Mirrors [TypedValueEdit] in hero_attributes.dart but kept local so
/// the NPC panel has no dependency on the player-only hero types.
class NpcTypedEdit {
  const NpcTypedEdit({required this.path, required this.value});

  final List<String> path;
  final double value;
}

/// One NPC gameplay attribute returned by the core `private.npc.attributes`
/// command: a Base/Current value pair plus the FULL typed paths that
/// `private.typed.setValue` resolves. NPC attribute keys share the player's id
/// namespace, so the panel groups these via `heroAttributeGroup(key)` into the
/// same Main stats / Combat / Resistances / Thieving / Advanced sidebar the
/// player uses (NPC-specific extras fall into the Advanced catch-all).
class NpcAttributeRow {
  const NpcAttributeRow({
    required this.key,
    required this.base,
    required this.current,
    required this.basePath,
    required this.currentPath,
  });

  factory NpcAttributeRow.fromJson(Map<String, Object?> json) {
    return NpcAttributeRow(
      key: json['key'] as String? ?? '',
      base: (json['base'] as num?)?.toDouble() ?? 0,
      current: (json['current'] as num?)?.toDouble() ?? 0,
      basePath: _stringList(json['basePath']),
      currentPath: _stringList(json['currentPath']),
    );
  }

  /// Attribute name (e.g. `Health`). Doubles as the row label.
  final String key;
  final double base;
  final double current;

  /// Full typed path to the BaseValue leaf (`private.typed.setValue` resolves).
  final List<String> basePath;

  /// Full typed path to the CurrentValue leaf.
  final List<String> currentPath;

  static List<String> _stringList(Object? value) {
    if (value is! List) return const [];
    return value.whereType<String>().toList(growable: false);
  }
}

/// Result of loading an NPC's attribute rows. Carries an inline [error] instead
/// of throwing, mirroring [HeroAttributesResult].
class NpcAttributesResult {
  const NpcAttributesResult({this.attributes = const [], this.error});

  factory NpcAttributesResult.fromJson(Map<String, Object?> json) {
    final raw = (json['attributes'] as List?) ?? const [];
    return NpcAttributesResult(
      attributes: raw
          .whereType<Map>()
          .map((m) => NpcAttributeRow.fromJson(m.cast<String, Object?>()))
          .toList(growable: false),
    );
  }

  final List<NpcAttributeRow> attributes;
  final String? error;
}
