/// One row of `private.characters.list`: an actor (or a knowledge-only orphan,
/// `globalId == null`) with per-aspect availability flags. Backs the Charaktere
/// master list.
class CharacterRow {
  const CharacterRow({
    required this.globalId,
    required this.uniqueName,
    required this.isDead,
    required this.hasInventory,
    required this.hasKnowledge,
    required this.hasEvents,
  });

  factory CharacterRow.fromJson(Map<String, Object?> json) {
    return CharacterRow(
      globalId: json['globalId'] as String?,
      uniqueName: json['uniqueName'] as String? ?? '',
      isDead: json['isDead'] == true,
      hasInventory: json['hasInventory'] == true,
      hasKnowledge: json['hasKnowledge'] == true,
      hasEvents: json['hasEvents'] == true,
    );
  }

  final String? globalId;
  final String uniqueName;
  final bool isDead;
  final bool hasInventory;
  final bool hasKnowledge;
  final bool hasEvents;

  bool get isOrphan => globalId == null;
}

/// The full unified character index (one `private.characters.list` response).
class CharacterIndexPage {
  const CharacterIndexPage({
    this.characters = const [],
    this.total = 0,
    this.error,
  });

  factory CharacterIndexPage.fromJson(Map<String, Object?> json) {
    return CharacterIndexPage(
      characters: (json['characters'] as List?)
              ?.whereType<Map>()
              .map((e) => CharacterRow.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
    );
  }

  final List<CharacterRow> characters;
  final int total;
  final String? error;
}
