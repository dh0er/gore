/// Save-backed data for the dedicated World > Glossary editor.
///
/// Creature and location rows originate from the game's glossary quest trees;
/// NPC rows are joined in the UI from the bundled static catalog and
/// [GlossarySegmentUnlock] records.  In every case the authoritative persisted
/// visibility is a `Memory.Document.SegmentUnlocked` event.
library;

class GlossaryPage {
  const GlossaryPage({
    this.categories = const [],
    this.segmentUnlocks = const [],
    this.heroMemoryArrayPath = const [],
    this.writable = const [],
    this.total = 0,
    this.error,
  });

  factory GlossaryPage.fromJson(Map<String, Object?> json) {
    return GlossaryPage(
      categories:
          (json['categories'] as List?)
              ?.whereType<Map>()
              .map(
                (value) =>
                    GlossaryCategory.fromJson(value.cast<String, Object?>()),
              )
              .toList(growable: false) ??
          const [],
      segmentUnlocks:
          (json['segmentUnlocks'] as List?)
              ?.whereType<Map>()
              .map(
                (value) => GlossarySegmentUnlock.fromJson(
                  value.cast<String, Object?>(),
                ),
              )
              .toList(growable: false) ??
          const [],
      heroMemoryArrayPath:
          (json['heroMemoryArrayPath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      writable:
          (json['writable'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
    );
  }

  final List<GlossaryCategory> categories;
  final List<GlossarySegmentUnlock> segmentUnlocks;
  final List<String> heroMemoryArrayPath;
  final List<String> writable;
  final int total;
  final String? error;

  bool get canSetSegment => writable.contains('private.glossary.setSegment');

  GlossaryCategory? category(String id) {
    for (final category in categories) {
      if (category.id == id) return category;
    }
    return null;
  }
}

class GlossaryCategory {
  const GlossaryCategory({
    required this.id,
    required this.group,
    this.entries = const [],
    this.total = 0,
  });

  factory GlossaryCategory.fromJson(Map<String, Object?> json) {
    return GlossaryCategory(
      id: json['id'] as String? ?? '',
      group: json['group'] as String? ?? '',
      entries:
          (json['entries'] as List?)
              ?.whereType<Map>()
              .map(
                (value) =>
                    GlossaryEntry.fromJson(value.cast<String, Object?>()),
              )
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
    );
  }

  final String id;
  final String group;
  final List<GlossaryEntry> entries;
  final int total;
}

class GlossaryEntry {
  const GlossaryEntry({
    required this.id,
    required this.name,
    required this.documentClass,
    this.group = '',
    this.questClass = '',
    this.currentState,
    this.statePath = const [],
    this.segments = const [],
    this.writable = false,
    this.unlocked = false,
  });

  factory GlossaryEntry.fromJson(Map<String, Object?> json) {
    return GlossaryEntry(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      documentClass: json['documentClass'] as String? ?? '',
      group: json['group'] as String? ?? '',
      questClass: json['questClass'] as String? ?? '',
      currentState: json['currentState'] as String?,
      statePath:
          (json['statePath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      segments:
          (json['segments'] as List?)
              ?.whereType<Map>()
              .map(
                (value) =>
                    GlossarySegment.fromJson(value.cast<String, Object?>()),
              )
              .toList(growable: false) ??
          const [],
      writable: json['writable'] == true,
      unlocked: json['unlocked'] == true,
    );
  }

  final String id;
  final String name;
  final String documentClass;
  final String group;
  final String questClass;
  final String? currentState;
  final List<String> statePath;
  final List<GlossarySegment> segments;
  final bool writable;
  final bool unlocked;
}

class GlossarySegment {
  const GlossarySegment({
    required this.id,
    required this.name,
    required this.segmentClass,
    this.questClass = '',
    this.currentState,
    this.statePath = const [],
    this.eventIndices = const [],
    this.viewedEventIndices = const [],
    this.writable = false,
    this.unlocked = false,
  });

  factory GlossarySegment.fromJson(Map<String, Object?> json) {
    return GlossarySegment(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      segmentClass: json['segmentClass'] as String? ?? '',
      questClass: json['questClass'] as String? ?? '',
      currentState: json['currentState'] as String?,
      statePath:
          (json['statePath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      eventIndices: _intList(json['eventIndices']),
      viewedEventIndices: _intList(json['viewedEventIndices']),
      writable: json['writable'] == true,
      unlocked: json['unlocked'] == true,
    );
  }

  final String id;
  final String name;
  final String segmentClass;
  final String questClass;
  final String? currentState;
  final List<String> statePath;
  final List<int> eventIndices;
  final List<int> viewedEventIndices;
  final bool writable;
  final bool unlocked;
}

/// Raw Hero-memory join.  It deliberately also contains NPC documents, which
/// have no `QuestDataByClass` counterpart and are supplied by the static NPC
/// catalog.
class GlossarySegmentUnlock {
  const GlossarySegmentUnlock({
    required this.documentClass,
    required this.segmentClass,
    this.unlockedEventIndices = const [],
    this.viewedEventIndices = const [],
  });

  factory GlossarySegmentUnlock.fromJson(Map<String, Object?> json) {
    return GlossarySegmentUnlock(
      documentClass: json['documentClass'] as String? ?? '',
      segmentClass: json['segmentClass'] as String? ?? '',
      unlockedEventIndices: _intList(json['unlockedEventIndices']),
      viewedEventIndices: _intList(json['viewedEventIndices']),
    );
  }

  final String documentClass;
  final String segmentClass;
  final List<int> unlockedEventIndices;
  final List<int> viewedEventIndices;

  bool get unlocked => unlockedEventIndices.isNotEmpty;
}

/// Pending intent for the atomic core operation.  The optional quest path is
/// present for creature/location segments and absent for NPC segments.
class GlossarySegmentEdit {
  const GlossarySegmentEdit({
    required this.documentClass,
    required this.segmentClass,
    required this.unlocked,
    this.questStatePath = const [],
  });

  final String documentClass;
  final String segmentClass;
  final bool unlocked;
  final List<String> questStatePath;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.glossary.setSegment',
      'value': {
        'documentClass': documentClass,
        'segmentClass': segmentClass,
        'unlocked': unlocked,
        if (questStatePath.isNotEmpty) 'questStatePath': questStatePath,
      },
    };
  }
}

List<int> _intList(Object? value) =>
    (value as List?)
        ?.whereType<num>()
        .map((number) => number.toInt())
        .toList(growable: false) ??
    const [];
