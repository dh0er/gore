/// Source-declared story state and its optional value in
/// `FSingleStorySaveGameData.StoryPropertyValues`.
///
/// The wire format is a sparse `TMap<FName, int32>`, but the game scripts
/// declare some of those values as [StorySemanticType.timeMarker]. Keeping the
/// source-level meaning separate from the serialized type prevents timestamps
/// from being presented as booleans or counters.
enum StorySemanticType { integer, timeMarker, chapter, unknown }

/// One value-addressed change inside the atomic `private.story.apply` edit.
///
/// Story state is serialized as a sparse map. [present] therefore distinguishes
/// setting a value (including zero) from removing the entry so the game falls
/// back to the source-level default. [expectedStored] and [expectedRawValue]
/// carry the inspection snapshot used by the core for optimistic concurrency:
/// a stale editor must not overwrite a value changed by another writer.
class StoryStateEdit {
  const StoryStateEdit({
    required this.id,
    required this.present,
    required this.rawValue,
    required this.expectedStored,
    required this.expectedRawValue,
    this.allowUnknownCreate = false,
  }) : assert(!present || rawValue != null),
       assert(!expectedStored || expectedRawValue != null);

  factory StoryStateEdit.fromJson(Map<String, Object?> json) {
    final id = (json['id'] as String? ?? '').trim();
    if (id.isEmpty) {
      throw const FormatException('Story-state edit id must not be empty');
    }
    final present = json['present'];
    if (present is! bool) {
      throw const FormatException('Story-state edit present must be a bool');
    }
    final rawValue = _storyInt(json['rawValue'], field: 'rawValue');
    if (present && rawValue == null) {
      throw const FormatException(
        'A present story-state edit requires rawValue',
      );
    }

    final expected = json['expected'];
    if (expected is! Map) {
      throw const FormatException(
        'Story-state edit expected must be an object',
      );
    }
    final expectedStored = expected['stored'];
    if (expectedStored is! bool) {
      throw const FormatException(
        'Story-state edit expected.stored must be a bool',
      );
    }
    final expectedRawValue = _storyInt(
      expected['rawValue'],
      field: 'expected.rawValue',
    );
    if (expectedStored && expectedRawValue == null) {
      throw const FormatException(
        'A stored expected story-state value requires rawValue',
      );
    }

    final rawAllowUnknownCreate = json['allowUnknownCreate'];
    if (rawAllowUnknownCreate != null && rawAllowUnknownCreate is! bool) {
      throw const FormatException(
        'Story-state edit allowUnknownCreate must be a bool',
      );
    }

    return StoryStateEdit(
      id: id,
      present: present,
      rawValue: present ? rawValue : null,
      expectedStored: expectedStored,
      expectedRawValue: expectedStored ? expectedRawValue : null,
      allowUnknownCreate: rawAllowUnknownCreate as bool? ?? false,
    );
  }

  /// Builds a change against the value returned by the current inspection.
  factory StoryStateEdit.fromValue(
    StoryStateValue original, {
    required bool present,
    int? rawValue,
    bool allowUnknownCreate = false,
  }) => StoryStateEdit(
    id: original.id,
    present: present,
    rawValue: present ? rawValue : null,
    expectedStored: original.stored,
    expectedRawValue: original.stored ? original.value : null,
    allowUnknownCreate: allowUnknownCreate,
  );

  final String id;
  final bool present;
  final int? rawValue;
  final bool expectedStored;
  final int? expectedRawValue;

  /// Required only when a caller deliberately creates an ID that is not in the
  /// bundled source catalog. Known-but-unset catalog values do not need it.
  final bool allowUnknownCreate;

  /// Case-insensitive identity used by both the game map and pending registry.
  String get normalizedId => normalizeStoryStateId(id);

  /// Whether the desired state is exactly the inspection snapshot. Such a
  /// change is removed from the central pending edit rather than serialized.
  bool get isNoop =>
      present == expectedStored && (!present || rawValue == expectedRawValue);

  Map<String, Object?> toJson() => {
    'id': id.trim(),
    'present': present,
    if (present) 'rawValue': rawValue,
    'expected': {
      'stored': expectedStored,
      if (expectedStored) 'rawValue': expectedRawValue,
    },
    if (allowUnknownCreate) 'allowUnknownCreate': true,
  };

  StoryStateEdit copyWith({
    String? id,
    bool? present,
    Object? rawValue = _storyUnchanged,
    bool? expectedStored,
    Object? expectedRawValue = _storyUnchanged,
    bool? allowUnknownCreate,
  }) => StoryStateEdit(
    id: id ?? this.id,
    present: present ?? this.present,
    rawValue: identical(rawValue, _storyUnchanged)
        ? this.rawValue
        : rawValue as int?,
    expectedStored: expectedStored ?? this.expectedStored,
    expectedRawValue: identical(expectedRawValue, _storyUnchanged)
        ? this.expectedRawValue
        : expectedRawValue as int?,
    allowUnknownCreate: allowUnknownCreate ?? this.allowUnknownCreate,
  );

  @override
  bool operator ==(Object other) =>
      other is StoryStateEdit &&
      other.id == id &&
      other.present == present &&
      other.rawValue == rawValue &&
      other.expectedStored == expectedStored &&
      other.expectedRawValue == expectedRawValue &&
      other.allowUnknownCreate == allowUnknownCreate;

  @override
  int get hashCode => Object.hash(
    id,
    present,
    rawValue,
    expectedStored,
    expectedRawValue,
    allowUnknownCreate,
  );
}

const storyStatePendingKey = 'story-state';
const storyStateApplyPath = 'private.story.apply';
const _storyUnchanged = Object();

String normalizeStoryStateId(String id) => id.trim().toLowerCase();

int? _storyInt(Object? raw, {required String field}) {
  if (raw == null) return null;
  if (raw is int) return raw;
  if (raw is num && raw.isFinite && raw == raw.toInt()) return raw.toInt();
  throw FormatException('Story-state edit $field must be an integer');
}

/// Builds the one atomic core edit that owns every pending story-state change.
Map<String, Object?> storyStateApplyEdit(Iterable<StoryStateEdit> changes) => {
  'path': storyStateApplyPath,
  'value': {
    'changes': [for (final change in changes) change.toJson()],
  },
};

/// Parses changes from an aggregated pending/core edit.
///
/// A wrong path or malformed payload is rejected instead of being silently
/// interpreted as an empty list; callers can then avoid discarding a corrupted
/// pending draft.
List<StoryStateEdit> parseStoryStateApplyEdit(Map<String, Object?> edit) {
  if (edit['path'] != storyStateApplyPath) {
    throw const FormatException('Not a private.story.apply edit');
  }
  final value = edit['value'];
  final rawChanges = value is Map ? value['changes'] : null;
  if (rawChanges is! List) {
    throw const FormatException('Story-state apply changes must be a list');
  }
  final changes = rawChanges
      .map((raw) {
        if (raw is! Map) {
          throw const FormatException('Story-state change must be an object');
        }
        return StoryStateEdit.fromJson(raw.cast<String, Object?>());
      })
      .toList(growable: false);
  final ids = <String>{};
  for (final change in changes) {
    if (!ids.add(change.normalizedId)) {
      throw FormatException('Duplicate story-state edit id: ${change.id}');
    }
  }
  return List.unmodifiable(changes);
}

class StoryStateValue {
  const StoryStateValue({
    required this.id,
    required this.value,
    required this.stored,
    required this.catalogKnown,
    required this.path,
    required this.semanticType,
    required this.declaredType,
  });

  factory StoryStateValue.fromJson(Map<String, Object?> json) {
    final semanticName = json['semanticType'] as String? ?? 'integer';
    final rawValue = json['rawValue'] ?? json['value'];
    return StoryStateValue(
      id: json['id'] as String? ?? '',
      value: (rawValue as num?)?.toInt(),
      stored: json['stored'] as bool? ?? rawValue is num,
      catalogKnown: json['catalogKnown'] as bool? ?? semanticName != 'unknown',
      path:
          (json['path'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      semanticType: StorySemanticType.values.firstWhere(
        (type) => type.name == semanticName,
        orElse: () => StorySemanticType.unknown,
      ),
      declaredType:
          json['declaredType'] as String? ??
          (semanticName == 'unknown' ? 'unknown' : 'int32'),
    );
  }

  final String id;
  final int? value;
  final bool stored;
  final bool catalogKnown;
  final List<String> path;
  final StorySemanticType semanticType;
  final String declaredType;
}

class StoryStatePage {
  const StoryStatePage({
    this.values = const [],
    this.kindCounts = const {},
    this.total = 0,
    this.storedTotal = 0,
    this.catalogTotal = 0,
    this.unsetTotal = 0,
    this.unknownStoredTotal = 0,
    this.offset = 0,
    this.limit = 1000,
    this.currentGameTimeSeconds,
    this.writable = false,
    this.error,
  });

  factory StoryStatePage.fromJson(Map<String, Object?> json) {
    final counts = <StorySemanticType, int>{};
    final rawCounts =
        json['catalogSemanticTypeCounts'] ??
        json['storedSemanticTypeCounts'] ??
        json['semanticTypeCounts'] ??
        json['kindCounts'];
    (rawCounts as Map?)?.forEach((key, value) {
      if (key is! String || value is! num) return;
      for (final type in StorySemanticType.values) {
        if (type.name == key) counts[type] = value.toInt();
      }
    });
    final values =
        ((json['entries'] ?? json['values']) as List?)
            ?.whereType<Map>()
            .map(
              (value) =>
                  StoryStateValue.fromJson(value.cast<String, Object?>()),
            )
            .where((value) => value.id.isNotEmpty)
            .toList(growable: false) ??
        const <StoryStateValue>[];
    return StoryStatePage(
      values: values,
      kindCounts: counts.isEmpty
          ? {
              for (final type in StorySemanticType.values)
                type: values
                    .where((value) => value.semanticType == type)
                    .length,
            }
          : Map.unmodifiable(counts),
      total:
          (json['total'] as num?)?.toInt() ??
          (json['catalogTotal'] as num?)?.toInt() ??
          (json['storedTotal'] as num?)?.toInt() ??
          values.length,
      storedTotal:
          (json['storedTotal'] as num?)?.toInt() ??
          values.where((value) => value.stored).length,
      catalogTotal:
          (json['catalogTotal'] as num?)?.toInt() ??
          (json['storedTotal'] as num?)?.toInt() ??
          values.length,
      unsetTotal:
          (json['unsetTotal'] as num?)?.toInt() ??
          values.where((value) => !value.stored).length,
      unknownStoredTotal: (json['unknownStoredTotal'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 1000,
      currentGameTimeSeconds: (json['currentGameTimeSeconds'] as num?)
          ?.toDouble(),
      writable: json['writable'] as bool? ?? false,
    );
  }

  final List<StoryStateValue> values;
  final Map<StorySemanticType, int> kindCounts;
  final int total;
  final int storedTotal;
  final int catalogTotal;
  final int unsetTotal;
  final int unknownStoredTotal;
  final int offset;
  final int limit;
  final double? currentGameTimeSeconds;
  final bool writable;
  final String? error;
}
