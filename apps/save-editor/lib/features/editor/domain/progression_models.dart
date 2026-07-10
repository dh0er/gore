// Models for the Progression tab: the inspect overview, paged section
// queries (quests / knowledge / events), and the edit intents that map to
// core write ops. All pages carry an optional [error] (set by the notifier
// instead of throwing) so cards can render failures inline.

class ProgressionOverview {
  const ProgressionOverview({
    this.status,
    this.questTotal = 0,
    this.questStates = const {},
    this.knowledgeCharacters = 0,
    this.knowledgeEntries = 0,
    this.memoryCharacters = 0,
    this.memoryEvents = 0,
    this.writable = const [],
  });

  factory ProgressionOverview.fromJson(Map<String, Object?>? json) {
    final states = <String, int>{};
    (json?['questStates'] as Map?)?.forEach((key, value) {
      if (key is String && value is num) states[key] = value.toInt();
    });
    return ProgressionOverview(
      status: json?['status'] as String?,
      questTotal: (json?['questTotal'] as num?)?.toInt() ?? 0,
      questStates: states,
      knowledgeCharacters: (json?['knowledgeCharacters'] as num?)?.toInt() ?? 0,
      knowledgeEntries: (json?['knowledgeEntries'] as num?)?.toInt() ?? 0,
      memoryCharacters: (json?['memoryCharacters'] as num?)?.toInt() ?? 0,
      memoryEvents: (json?['memoryEvents'] as num?)?.toInt() ?? 0,
      writable:
          (json?['writable'] as List?)?.whereType<String>().toList() ??
          const [],
    );
  }

  final String? status;
  final int questTotal;
  final Map<String, int> questStates;
  final int knowledgeCharacters;
  final int knowledgeEntries;
  final int memoryCharacters;
  final int memoryEvents;
  final List<String> writable;

  bool get available => status == 'ok';
}

class ProgressionQuest {
  const ProgressionQuest({
    required this.questClass,
    required this.id,
    required this.group,
    required this.name,
    required this.statePath,
    this.currentState,
    this.writable = false,
  });

  factory ProgressionQuest.fromJson(Map<String, Object?> json) {
    return ProgressionQuest(
      questClass: json['questClass'] as String? ?? '',
      id: json['id'] as String? ?? '',
      group: json['group'] as String? ?? '',
      name: json['name'] as String? ?? '',
      currentState: json['currentState'] as String?,
      statePath:
          (json['statePath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      writable: json['writable'] as bool? ?? false,
    );
  }

  final String questClass;
  final String id;
  final String group;
  final String name;
  final String? currentState;
  final List<String> statePath;
  final bool writable;
}

class ProgressionQuestPage {
  const ProgressionQuestPage({
    this.quests = const [],
    this.stateCounts = const {},
    this.groupCounts = const {},
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory ProgressionQuestPage.fromJson(Map<String, Object?> json) {
    final counts = <String, int>{};
    (json['stateCounts'] as Map?)?.forEach((key, value) {
      if (key is String && value is num) counts[key] = value.toInt();
    });
    final gCounts = <String, int>{};
    (json['groupCounts'] as Map?)?.forEach((key, value) {
      if (key is String && value is num) gCounts[key] = value.toInt();
    });
    return ProgressionQuestPage(
      quests:
          (json['quests'] as List?)
              ?.whereType<Map>()
              .map((e) => ProgressionQuest.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      stateCounts: counts,
      groupCounts: gCounts,
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<ProgressionQuest> quests;
  final Map<String, int> stateCounts;
  final Map<String, int> groupCounts;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + quests.length < total;
  bool get hasNext => offset + quests.length < total;
  bool get hasPrevious => offset > 0;
  int get pageIndex => limit == 0 ? 0 : offset ~/ limit;
  int get pageCount => total == 0 ? 1 : (total + limit - 1) ~/ limit;
}

class KnowledgeEntriesPage {
  const KnowledgeEntriesPage({
    this.character = '',
    this.entries = const [],
    this.setPath = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 200,
    this.error,
  });

  factory KnowledgeEntriesPage.fromJson(Map<String, Object?> json) {
    return KnowledgeEntriesPage(
      character: json['character'] as String? ?? '',
      entries:
          (json['entries'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      setPath:
          (json['setPath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 200,
    );
  }

  final String character;
  final List<String> entries;
  final List<String> setPath;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + entries.length < total;
  bool get hasNext => offset + entries.length < total;
  bool get hasPrevious => offset > 0;
  int get pageIndex => limit == 0 ? 0 : offset ~/ limit;
  int get pageCount => total == 0 ? 1 : (total + limit - 1) ~/ limit;
}

class MemoryEvent {
  const MemoryEvent({
    required this.index,
    this.tags = const [],
    this.magnitude,
    this.timeSeconds,
    this.durationSeconds,
    this.instigator,
    this.affected,
    this.optionalClass1,
    this.optionalClass2,
  });

  factory MemoryEvent.fromJson(Map<String, Object?> json) {
    return MemoryEvent(
      index: (json['index'] as num?)?.toInt() ?? 0,
      tags:
          (json['tags'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      magnitude: (json['magnitude'] as num?)?.toDouble(),
      timeSeconds: (json['timeSeconds'] as num?)?.toDouble(),
      durationSeconds: (json['durationSeconds'] as num?)?.toDouble(),
      instigator: json['instigator'] as String?,
      affected: json['affected'] as String?,
      optionalClass1: json['optionalClass1'] as String?,
      optionalClass2: json['optionalClass2'] as String?,
    );
  }

  final int index;
  final List<String> tags;
  final double? magnitude;
  final double? timeSeconds;
  final double? durationSeconds;
  final String? instigator;
  final String? affected;
  final String? optionalClass1;
  final String? optionalClass2;
}

class MemoryEventsPage {
  const MemoryEventsPage({
    this.character = '',
    this.events = const [],
    this.arrayPath = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory MemoryEventsPage.fromJson(Map<String, Object?> json) {
    return MemoryEventsPage(
      character: json['character'] as String? ?? '',
      events:
          (json['events'] as List?)
              ?.whereType<Map>()
              .map((e) => MemoryEvent.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      arrayPath:
          (json['arrayPath'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final String character;
  final List<MemoryEvent> events;
  final List<String> arrayPath;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + events.length < total;
  bool get hasNext => offset + events.length < total;
  bool get hasPrevious => offset > 0;
  int get pageIndex => limit == 0 ? 0 : offset ~/ limit;
  int get pageCount => total == 0 ? 1 : (total + limit - 1) ~/ limit;
}

/// Un-forgiven crime counts by most-severe type, from `private.factions.list`.
/// A single crime entry is counted once, in its worst category.
class CrimeBreakdown {
  const CrimeBreakdown({
    this.murder = 0,
    this.assault = 0,
    this.theft = 0,
    this.trespassing = 0,
    this.threat = 0,
    this.other = 0,
  });

  factory CrimeBreakdown.fromJson(Map<String, Object?> json) {
    return CrimeBreakdown(
      murder: (json['murder'] as num?)?.toInt() ?? 0,
      assault: (json['assault'] as num?)?.toInt() ?? 0,
      theft: (json['theft'] as num?)?.toInt() ?? 0,
      trespassing: (json['trespassing'] as num?)?.toInt() ?? 0,
      threat: (json['threat'] as num?)?.toInt() ?? 0,
      other: (json['other'] as num?)?.toInt() ?? 0,
    );
  }

  final int murder;
  final int assault;
  final int theft;
  final int trespassing;
  final int threat;
  final int other;

  int get total =>
      murder + assault + theft + trespassing + threat + other;
}

/// One guild's crime tally for the player, from `private.factions.list`.
class FactionGuild {
  const FactionGuild({
    required this.guild,
    required this.label,
    required this.total,
    required this.forgiven,
    required this.unforgiven,
    required this.isHostile,
    required this.crimes,
  });

  factory FactionGuild.fromJson(Map<String, Object?> json) {
    final crimesJson = json['crimes'];
    return FactionGuild(
      guild: json['guild'] as String? ?? '',
      label: json['label'] as String? ?? '',
      total: (json['total'] as num?)?.toInt() ?? 0,
      forgiven: (json['forgiven'] as num?)?.toInt() ?? 0,
      unforgiven: (json['unforgiven'] as num?)?.toInt() ?? 0,
      isHostile: json['isHostile'] as bool? ?? false,
      crimes: crimesJson is Map
          ? CrimeBreakdown.fromJson(crimesJson.cast<String, Object?>())
          : const CrimeBreakdown(),
    );
  }

  /// The camp-level guild tag (e.g. `Guild.Human.OldCamp`), or `Other` for the
  /// individual/unmappable bucket.
  final String guild;

  /// Short human label emitted by the core (camp name, e.g. `OldCamp`). The UI
  /// prefers a localized label keyed on [guild] and falls back to this/[guild].
  final String label;
  final int total;
  final int forgiven;
  final int unforgiven;

  /// Computed hostility flag (approximated from un-forgiven serious crimes).
  /// Drives the prominent Feindselig/Friedlich badge.
  final bool isHostile;

  /// Un-forgiven crime counts by most-severe type.
  final CrimeBreakdown crimes;
}

/// A page of guild crimes from `private.factions.list`. Carries an optional
/// [error] (set by the notifier instead of throwing) so the pane renders
/// failures inline, mirroring the other progression pages.
class FactionsPage {
  const FactionsPage({this.guilds = const [], this.error});

  factory FactionsPage.fromJson(Map<String, Object?> json) {
    return FactionsPage(
      guilds:
          (json['guilds'] as List?)
              ?.whereType<Map>()
              .map((e) => FactionGuild.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
    );
  }

  final List<FactionGuild> guilds;
  final String? error;
}

/// Pending quest-state change → `private.typed.setValue`.
class QuestStateChange {
  const QuestStateChange({required this.statePath, required this.state});

  final List<String> statePath;
  final String state;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.typed.setValue',
      'value': {'path': statePath, 'value': state},
    };
  }
}

/// Pending knowledge add/remove → `private.typed.setAdd` / `setRemove`.
class KnowledgeEntryEdit {
  const KnowledgeEntryEdit.add({required this.setPath, required this.entry})
    : isAdd = true;
  const KnowledgeEntryEdit.remove({required this.setPath, required this.entry})
    : isAdd = false;

  final List<String> setPath;
  final String entry;
  final bool isAdd;

  Map<String, Object?> toEditJson() {
    return {
      'path': isAdd ? 'private.typed.setAdd' : 'private.typed.setRemove',
      'value': {'path': setPath, 'value': entry},
    };
  }
}

/// Structural memory-event edit → `private.typed.arrayRemove` /
/// `arrayDuplicate`. Index-addressed, so the UI applies at most one per save
/// round (indices shift after each structural change).
class MemoryEventEdit {
  const MemoryEventEdit.remove({required this.arrayPath, required this.index})
    : isRemove = true;
  const MemoryEventEdit.duplicate({
    required this.arrayPath,
    required this.index,
  }) : isRemove = false;

  final List<String> arrayPath;
  final int index;
  final bool isRemove;

  Map<String, Object?> toEditJson() {
    return {
      'path': isRemove
          ? 'private.typed.arrayRemove'
          : 'private.typed.arrayDuplicate',
      'value': {'path': arrayPath, 'index': index},
    };
  }
}
