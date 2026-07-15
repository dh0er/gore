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
    this.position,
    this.payload,
  });

  factory MemoryEvent.fromJson(Map<String, Object?> json) {
    return MemoryEvent(
      index: (json['index'] as num?)?.toInt() ?? 0,
      tags:
          (json['tags'] as List?)
              ?.whereType<String>()
              .map(_normalizedMemoryName)
              .whereType<String>()
              .toList(growable: false) ??
          const [],
      magnitude: _normalizedMemoryNumber(json['magnitude']),
      timeSeconds: _normalizedMemoryNumber(json['timeSeconds']),
      durationSeconds: _normalizedMemoryNumber(json['durationSeconds']),
      instigator: _normalizedMemoryName(json['instigator']),
      affected: _normalizedMemoryName(json['affected']),
      optionalClass1: _normalizedMemoryName(json['optionalClass1']),
      optionalClass2: _normalizedMemoryName(json['optionalClass2']),
      position: json['position'] is Map
          ? MemoryEventPosition.fromJson(
              (json['position'] as Map).cast<String, Object?>(),
            )
          : null,
      payload: json['payload'] is Map
          ? MemoryEventPayload.fromJson(
              (json['payload'] as Map).cast<String, Object?>(),
            )
          : null,
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
  final MemoryEventPosition? position;
  final MemoryEventPayload? payload;
}

class MemoryEventPosition {
  const MemoryEventPosition({
    required this.x,
    required this.y,
    required this.z,
  });

  factory MemoryEventPosition.fromJson(Map<String, Object?> json) {
    return MemoryEventPosition(
      x: (json['x'] as num?)?.toDouble() ?? 0,
      y: (json['y'] as num?)?.toDouble() ?? 0,
      z: (json['z'] as num?)?.toDouble() ?? 0,
    );
  }

  final double x;
  final double y;
  final double z;
}

/// Bounded, display-oriented view of an event's dynamic FInstancedStruct
/// payload. The core deliberately limits recursion/items, so even the one
/// known item-inspection payload with hundreds of map entries stays cheap.
class MemoryEventPayload {
  const MemoryEventPayload({
    this.type,
    this.fieldCount = 0,
    this.fields = const [],
    this.truncated = false,
  });

  factory MemoryEventPayload.fromJson(Map<String, Object?> json) {
    return MemoryEventPayload(
      type: _normalizedMemoryName(json['type']),
      fieldCount: (json['fieldCount'] as num?)?.toInt() ?? 0,
      fields:
          (json['fields'] as List?)
              ?.whereType<Map>()
              .map(
                (field) => MemoryEventPayloadField.fromJson(
                  field.cast<String, Object?>(),
                ),
              )
              .toList(growable: false) ??
          const [],
      truncated: json['truncated'] as bool? ?? false,
    );
  }

  final String? type;
  final int fieldCount;
  final List<MemoryEventPayloadField> fields;
  final bool truncated;

  Object? valueFor(String name) {
    final lower = name.toLowerCase();
    for (final field in fields) {
      if (field.name.toLowerCase() == lower) return field.value;
    }
    return null;
  }
}

class MemoryEventPayloadField {
  const MemoryEventPayloadField({
    required this.name,
    required this.type,
    this.value,
  });

  factory MemoryEventPayloadField.fromJson(Map<String, Object?> json) {
    return MemoryEventPayloadField(
      name: json['name'] as String? ?? '',
      type: json['type'] as String? ?? '',
      value: json['value'],
    );
  }

  final String name;
  final String type;
  final Object? value;
}

/// Unreal uses `-DBL_MAX` for unset event timestamps and the FName `None` for
/// absent object references. Keep those serialization details out of the UI.
double? _normalizedMemoryNumber(Object? raw) {
  if (raw is! num) return null;
  final value = raw.toDouble();
  if (!value.isFinite || value.abs() > 1e300) return null;
  return value;
}

String? _normalizedMemoryName(Object? raw) {
  if (raw is! String) return null;
  final value = raw.trim();
  if (value.isEmpty || value.toLowerCase() == 'none') return null;
  return value;
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

  int get total => murder + assault + theft + trespassing + threat + other;
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

/// Pending, value-addressed knowledge add/remove. The core resolves the
/// character's knowledge set at save time and creates the map entry on the
/// first add when necessary, so this never needs an immediate preparatory
/// write just to discover a typed path.
class KnowledgeEntryEdit {
  const KnowledgeEntryEdit.add({required this.character, required this.entry})
    : isAdd = true;
  const KnowledgeEntryEdit.remove({
    required this.character,
    required this.entry,
  }) : isAdd = false;

  final String character;
  final String entry;
  final bool isAdd;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.knowledge.setEntry',
      'value': {'character': character, 'entry': entry, 'present': isAdd},
    };
  }
}

/// Pending structural memory-event edit → `private.typed.arrayRemove` /
/// `arrayDuplicate`. Removes keep their original on-disk index while pending;
/// the save orchestrator applies multiple removes for one array from the
/// highest index down so earlier writes cannot retarget later ones.
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

  static MemoryEventEdit? fromEditJson(Map<String, Object?> json) {
    final op = json['path'];
    if (op != 'private.typed.arrayRemove' &&
        op != 'private.typed.arrayDuplicate') {
      return null;
    }
    final value = json['value'];
    if (value is! Map) return null;
    final rawPath = value['path'];
    final index = value['index'];
    if (rawPath is! List || index is! num) return null;
    final path = rawPath.whereType<String>().toList();
    if (path.length != rawPath.length) return null;
    return op == 'private.typed.arrayRemove'
        ? MemoryEventEdit.remove(arrayPath: path, index: index.toInt())
        : MemoryEventEdit.duplicate(arrayPath: path, index: index.toInt());
  }

  Map<String, Object?> toEditJson() {
    return {
      'path': isRemove
          ? 'private.typed.arrayRemove'
          : 'private.typed.arrayDuplicate',
      'value': {'path': arrayPath, 'index': index},
    };
  }
}
