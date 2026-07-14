/// Pure domain projection of the flat save-game quest map into the hierarchy
/// used by the in-game journal.
library;

import 'dart:collection';

import 'progression_models.dart';

/// Resolves player-facing quest text. Returning `null` means that the save row
/// has no localized text for the active game language.
typedef QuestJournalTextResolver = String? Function(ProgressionQuest quest);

/// Decides whether a top-level, localized quest is a real journal quest.
///
/// Descendants with localized labels are retained below their nearest named
/// ancestor even when this predicate returns false. A caller with a loc catalog
/// can therefore use the presence of a localized quest description as the
/// robust main-quest predicate while still keeping named objectives.
typedef QuestJournalPredicate = bool Function(ProgressionQuest quest);

/// The five sections shown by the game's quest journal.
enum QuestJournalSection { oldCamp, newCamp, swampCamp, colony, completed }

/// Immutable result of projecting a flat quest page into journal data.
class QuestJournal {
  const QuestJournal({required this.roots, this.tutorials = const []});

  final QuestJournalRoots roots;

  /// Tutorial rows are deliberately kept out of [roots]. Consumers can show
  /// them in the glossary without having to split the original query again.
  /// This list includes the optional `Quest_Tutorials` scaffolding row.
  final List<ProgressionQuest> tutorials;

  /// Every visible journal node in stable, section-first depth-first order.
  Iterable<QuestJournalNode> get flattenDepthFirst => roots.flattenDepthFirst;

  /// Whether a root or anything assigned below it matches [query].
  ///
  /// This is useful for filtering without flattening the tree and losing the
  /// context of a matching objective or technical save row.
  Iterable<QuestJournalNode> search(String query) =>
      roots.all.where((root) => root.matchesQuery(query));
}

/// Sectioned top-level journal quests. Counts intentionally count roots, not
/// save rows or objectives, matching the numbers shown by the game.
class QuestJournalRoots {
  QuestJournalRoots({
    List<QuestJournalNode> oldCamp = const [],
    List<QuestJournalNode> newCamp = const [],
    List<QuestJournalNode> swampCamp = const [],
    List<QuestJournalNode> colony = const [],
    List<QuestJournalNode> completed = const [],
  }) : oldCamp = List.unmodifiable(oldCamp),
       newCamp = List.unmodifiable(newCamp),
       swampCamp = List.unmodifiable(swampCamp),
       colony = List.unmodifiable(colony),
       completed = List.unmodifiable(completed);

  final List<QuestJournalNode> oldCamp;
  final List<QuestJournalNode> newCamp;
  final List<QuestJournalNode> swampCamp;
  final List<QuestJournalNode> colony;
  final List<QuestJournalNode> completed;

  List<QuestJournalNode> forSection(QuestJournalSection section) =>
      switch (section) {
        QuestJournalSection.oldCamp => oldCamp,
        QuestJournalSection.newCamp => newCamp,
        QuestJournalSection.swampCamp => swampCamp,
        QuestJournalSection.colony => colony,
        QuestJournalSection.completed => completed,
      };

  Iterable<QuestJournalNode> get all sync* {
    for (final section in QuestJournalSection.values) {
      yield* forSection(section);
    }
  }

  Iterable<QuestJournalNode> get flattenDepthFirst sync* {
    for (final root in all) {
      yield* root.flattenDepthFirst;
    }
  }

  int countFor(QuestJournalSection section) => forSection(section).length;

  Map<QuestJournalSection, int> get counts => UnmodifiableMapView({
    for (final section in QuestJournalSection.values)
      section: countFor(section),
  });
}

/// A named quest or objective in the projected journal tree.
class QuestJournalNode {
  QuestJournalNode({
    required this.quest,
    required this.label,
    this.description,
    List<QuestJournalNode> children = const [],
    List<ProgressionQuest> technicalDescendants = const [],
  }) : children = List.unmodifiable(children),
       technicalDescendants = List.unmodifiable(technicalDescendants);

  final ProgressionQuest quest;
  final String label;
  final String? description;

  /// Recursively named subquests/objectives. An unnamed structural row between
  /// two named rows is skipped, so the child remains attached to the nearest
  /// player-facing ancestor.
  final List<QuestJournalNode> children;

  /// Unnamed save rows whose nearest named ancestor is this node. These remain
  /// available to an editor without polluting the player-facing hierarchy.
  /// Named descendants are represented in [children] instead.
  final List<ProgressionQuest> technicalDescendants;

  Iterable<QuestJournalNode> get flattenDepthFirst sync* {
    yield this;
    for (final child in children) {
      yield* child.flattenDepthFirst;
    }
  }

  /// This node's save row plus every named and technical descendant row.
  Iterable<ProgressionQuest> get relatedQuests sync* {
    yield quest;
    yield* technicalDescendants;
    for (final child in children) {
      yield* child.relatedQuests;
    }
  }

  /// Recursively searches localized text as well as raw ids/names. A matching
  /// descendant keeps this node in a filtered root list.
  bool matchesQuery(String query) {
    final needle = query.trim().toLowerCase();
    if (needle.isEmpty) return true;
    if (_containsText(label, needle) ||
        _containsText(description, needle) ||
        _questContains(quest, needle)) {
      return true;
    }
    if (technicalDescendants.any((row) => _questContains(row, needle))) {
      return true;
    }
    return children.any((child) => child.matchesQuery(needle));
  }
}

/// Builds the in-game-style journal hierarchy from flat quest save rows.
///
/// Structural parents are discovered by the longest existing id prefix at an
/// underscore boundary. Player-facing children then skip unnamed scaffolding
/// and attach to the nearest named ancestor. Only top-level candidates accepted
/// by [isJournalQuest] become roots; without a predicate, every localized,
/// non-scaffolding top-level candidate is accepted.
///
/// Root visibility follows the game: Running quests are split by camp,
/// Succeeded/Failed quests go to Completed, and Available/None/unknown roots
/// are hidden. Descendants remain attached regardless of their own state so an
/// editor can inspect and change the complete quest subtree.
QuestJournal buildQuestJournal(
  Iterable<ProgressionQuest> quests, {
  required QuestJournalTextResolver localizedLabel,
  QuestJournalTextResolver? localizedDescription,
  QuestJournalPredicate? isJournalQuest,
  bool allowRawFallback = false,
}) {
  final rows = quests.toList(growable: false);
  final tutorials = <ProgressionQuest>[];
  final entriesById = <String, _JournalEntry>{};
  final orderedEntries = <_JournalEntry>[];

  for (final quest in rows) {
    if (_isTutorial(quest)) {
      tutorials.add(quest);
      continue;
    }
    final entry = _JournalEntry(quest: quest);
    orderedEntries.add(entry);
    if (quest.id.isNotEmpty) {
      entriesById.putIfAbsent(quest.id.toLowerCase(), () => entry);
    }
  }

  // Resolve the complete structural tree before deciding which rows deserve a
  // visible node. Raw fallback uses this relationship to avoid turning every
  // internal row with a generated name into a journal root.
  for (final entry in orderedEntries) {
    entry.structuralParent = _longestPrefixParent(entry, entriesById);
  }

  for (final entry in orderedEntries) {
    if (_isScaffolding(entry.quest)) continue;
    final resolvedLabel = _nonBlank(localizedLabel(entry.quest));
    if (resolvedLabel != null) {
      entry.label = resolvedLabel;
      entry.description = _nonBlank(localizedDescription?.call(entry.quest));
      entry.localized = true;
      continue;
    }

    if (allowRawFallback && _isRawFallbackRoot(entry)) {
      entry.label = entry.quest.name.trim();
      entry.localized = false;
    }
  }

  final namedEntries = orderedEntries.where((entry) => entry.label != null);
  for (final entry in namedEntries) {
    entry.journalParent = _nearestNamedAncestor(entry.structuralParent);
    entry.journalParent?.namedChildren.add(entry);
  }

  // Preserve technical rows at their nearest meaningful journal node. Rows
  // above a named node (the group/chapter scaffolding) naturally have no named
  // ancestor and are therefore omitted.
  for (final entry in orderedEntries) {
    if (entry.label != null) continue;
    final owner = _nearestNamedAncestor(entry.structuralParent);
    owner?.technicalChildren.add(entry.quest);
  }

  final sectionEntries = {
    for (final section in QuestJournalSection.values)
      section: <_JournalEntry>[],
  };
  for (final entry in namedEntries) {
    if (entry.journalParent != null) continue;
    final accepted = entry.localized
        ? isJournalQuest?.call(entry.quest) ?? true
        : true;
    if (!accepted) continue;
    final section = _sectionFor(entry.quest);
    if (section != null) sectionEntries[section]!.add(entry);
  }

  QuestJournalNode freeze(_JournalEntry entry) => QuestJournalNode(
    quest: entry.quest,
    label: entry.label!,
    description: entry.description,
    children: entry.namedChildren.map(freeze).toList(growable: false),
    technicalDescendants: entry.technicalChildren,
  );

  List<QuestJournalNode> freezeSection(QuestJournalSection section) =>
      sectionEntries[section]!.map(freeze).toList(growable: false);

  return QuestJournal(
    roots: QuestJournalRoots(
      oldCamp: freezeSection(QuestJournalSection.oldCamp),
      newCamp: freezeSection(QuestJournalSection.newCamp),
      swampCamp: freezeSection(QuestJournalSection.swampCamp),
      colony: freezeSection(QuestJournalSection.colony),
      completed: freezeSection(QuestJournalSection.completed),
    ),
    tutorials: List.unmodifiable(tutorials),
  );
}

class _JournalEntry {
  _JournalEntry({required this.quest});

  final ProgressionQuest quest;
  _JournalEntry? structuralParent;
  _JournalEntry? journalParent;
  String? label;
  String? description;
  bool localized = false;
  final List<_JournalEntry> namedChildren = [];
  final List<ProgressionQuest> technicalChildren = [];
}

_JournalEntry? _longestPrefixParent(
  _JournalEntry entry,
  Map<String, _JournalEntry> entriesById,
) {
  final id = entry.quest.id.toLowerCase();
  var boundary = id.lastIndexOf('_');
  while (boundary > 0) {
    final candidate = entriesById[id.substring(0, boundary)];
    if (candidate != null && !identical(candidate, entry)) return candidate;
    boundary = id.lastIndexOf('_', boundary - 1);
  }
  return null;
}

_JournalEntry? _nearestNamedAncestor(_JournalEntry? entry) {
  var current = entry;
  while (current != null) {
    if (current.label != null) return current;
    current = current.structuralParent;
  }
  return null;
}

bool _isScaffolding(ProgressionQuest quest) {
  if (quest.id.toLowerCase() == 'quest_${quest.group}'.toLowerCase()) {
    return true;
  }
  return RegExp(r'chapter\d+$', caseSensitive: false).hasMatch(quest.id);
}

bool _isTutorial(ProgressionQuest quest) {
  if (quest.group.toLowerCase() == 'tutorials') return true;
  final id = quest.id.toLowerCase();
  return id == 'quest_tutorials' || id.startsWith('quest_tutorials_');
}

bool _isRawFallbackRoot(_JournalEntry entry) {
  if (_sectionFor(entry.quest) == null) return false;
  final name = entry.quest.name.trim();
  if (name.isEmpty) return false;
  final id = entry.quest.id.toUpperCase();
  if (id.contains('_OBJ_') || id.contains('_MAP')) return false;

  // A generated raw name is only trustworthy as a main quest when the nearest
  // existing structural ancestor is group/chapter scaffolding (or absent for a
  // standalone quest). This also filters internal `_NEW`, `_KILL`, etc. rows.
  final parent = entry.structuralParent;
  return parent == null || _isScaffolding(parent.quest);
}

QuestJournalSection? _sectionFor(ProgressionQuest quest) {
  final state = _shortState(quest.currentState).toLowerCase();
  if (state == 'succeeded' || state == 'failed') {
    return QuestJournalSection.completed;
  }
  if (state != 'running') return null;
  return switch (quest.group.toLowerCase()) {
    'oldcamp' => QuestJournalSection.oldCamp,
    'newcamp' => QuestJournalSection.newCamp,
    'swampcamp' => QuestJournalSection.swampCamp,
    _ => QuestJournalSection.colony,
  };
}

String _shortState(String? state) {
  if (state == null) return '';
  final separator = state.lastIndexOf('::');
  return separator < 0 ? state : state.substring(separator + 2);
}

String? _nonBlank(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}

bool _containsText(String? value, String needle) =>
    value?.toLowerCase().contains(needle) ?? false;

bool _questContains(ProgressionQuest quest, String needle) =>
    _containsText(quest.id, needle) ||
    _containsText(quest.name, needle) ||
    _containsText(quest.group, needle) ||
    _containsText(quest.questClass, needle);
