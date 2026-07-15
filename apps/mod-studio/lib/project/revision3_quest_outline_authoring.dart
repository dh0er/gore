import 'dart:convert';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

typedef Revision3QuestOutlineEditPublisher =
    Future<Revision3QuestOutlineEditPublication> Function({
      required Revision3QuestOutlineEditInput input,
    });

/// Exact visible Quest identity plus the only outline fields edit-v1 permits.
///
/// Technical IDs, module namespace, parent, giver, objective count and every
/// non-outline field come from the selected exact-current content projection;
/// the UI has no setters for them.
final class Revision3QuestOutlineEditInput {
  Revision3QuestOutlineEditInput._({
    required this.questId,
    required this.expectedQuestRevision,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.displayName,
    required this.title,
    required List<String> objectiveTitles,
    List<int>? objectiveSlots,
    AuthoringDraftContentSeal? expectedTransitionPlanSeal,
  }) : objectiveTitles = List<String>.unmodifiable(objectiveTitles),
       objectiveSlots = objectiveSlots == null
           ? null
           : List<int>.unmodifiable(objectiveSlots),
       expectedTransitionPlanSeal = _requireStableObjectivePair(
         objectiveSlots,
         expectedTransitionPlanSeal,
       );

  factory Revision3QuestOutlineEditInput.forQuest({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required String displayName,
    required String title,
    required List<String> objectiveTitles,
  }) {
    if (quest.kind != Revision3ContentEntityKind.questDraft ||
        index.entityById(quest.id) != quest) {
      throw const FormatException(
        'The selected item is not the exact Quest from this project view.',
      );
    }
    final summary = quest.summary.questDraft;
    if (summary == null) {
      throw const FormatException(
        'The selected Quest has no editable outline summary.',
      );
    }
    final moduleReferences = quest.references
        .where(
          (reference) =>
              reference.role == 'draft_script_module' &&
              reference.qualifier == null &&
              reference.resolution ==
                  Revision3ContentReferenceResolution.resolved &&
              reference.target.projectId == index.projectId &&
              reference.target.expectedKind ==
                  Revision3ContentEntityKind.scriptModule,
        )
        .toList(growable: false);
    if (moduleReferences.length != 1) {
      throw const FormatException(
        'The selected Quest does not own exactly one generated script.',
      );
    }
    final module = index.entityById(moduleReferences.single.target.entityId);
    if (module == null ||
        module.kind != Revision3ContentEntityKind.scriptModule) {
      throw const FormatException(
        'The selected Quest script is not available in this project view.',
      );
    }
    if (objectiveTitles.length != summary.objectiveTitles.length) {
      throw const FormatException(
        'This editor cannot add or remove objectives. Keep the existing count.',
      );
    }
    final validation = validateFields(
      displayName: displayName,
      title: title,
      objectiveTitles: objectiveTitles,
    );
    if (validation != null) throw FormatException(validation);
    return Revision3QuestOutlineEditInput._(
      questId: quest.id,
      expectedQuestRevision: quest.revision,
      moduleId: module.id,
      expectedModuleRevision: module.revision,
      displayName: displayName,
      title: title,
      objectiveTitles: objectiveTitles,
    );
  }

  /// Creates a slot-preserving edit for a semantic Quest. Legacy seeds keep
  /// using the count-preserving V1 transaction; semantic seeds carry the exact
  /// active slot permutation and plan seal into the native V2 CAS boundary.
  factory Revision3QuestOutlineEditInput.forQuestWithTransitionSeed({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required AuthoringRevision3QuestTransitionsSeed seed,
    required String displayName,
    required String title,
    required List<Revision3QuestOutlineObjectiveEdit> objectives,
  }) {
    final base = Revision3QuestOutlineEditInput.forQuest(
      index: index,
      quest: quest,
      displayName: displayName,
      title: title,
      objectiveTitles: [for (final objective in objectives) objective.title],
    );
    if (seed.projectId != index.projectId ||
        seed.projectRevision != index.projectRevision ||
        seed.questId != base.questId ||
        seed.questRevision != base.expectedQuestRevision ||
        seed.moduleId != base.moduleId ||
        seed.moduleRevision != base.expectedModuleRevision ||
        objectives.length != seed.objectives.length) {
      throw const FormatException(
        'The Quest behavior seed no longer matches this project view.',
      );
    }
    final activeSlots = seed.objectives
        .map((objective) => objective.slot)
        .toSet();
    final requestedSlots = objectives
        .map((objective) => objective.slot)
        .toSet();
    if (requestedSlots.length != objectives.length ||
        requestedSlots.length != activeSlots.length ||
        !requestedSlots.containsAll(activeSlots)) {
      throw const FormatException(
        'Keep every existing Quest objective exactly once.',
      );
    }
    if (seed.legacySynthetic) return base;
    return Revision3QuestOutlineEditInput._(
      questId: base.questId,
      expectedQuestRevision: base.expectedQuestRevision,
      moduleId: base.moduleId,
      expectedModuleRevision: base.expectedModuleRevision,
      displayName: base.displayName,
      title: base.title,
      objectiveTitles: base.objectiveTitles,
      objectiveSlots: [for (final objective in objectives) objective.slot],
      expectedTransitionPlanSeal: seed.transitionPlanSeal,
    );
  }

  final String questId;
  final int expectedQuestRevision;
  final String moduleId;
  final int expectedModuleRevision;
  final String displayName;
  final String title;
  final List<String> objectiveTitles;
  final List<int>? objectiveSlots;
  final AuthoringDraftContentSeal? expectedTransitionPlanSeal;

  bool get usesStableObjectiveSlots => objectiveSlots != null;

  static String? validateFields({
    required String displayName,
    required String title,
    required List<String> objectiveTitles,
  }) {
    if (displayName.trim().isEmpty || displayName.trim() != displayName) {
      return 'Enter a name for the Quest in the project library.';
    }
    if (utf8.encode(displayName).length > 256 ||
        displayName.runes.any(_isControl)) {
      return 'The project name must be at most 256 bytes and contain no control characters.';
    }
    final titleProblem = _literalProblem(title, label: 'Quest title');
    if (titleProblem != null) return titleProblem;
    if (objectiveTitles.isEmpty || objectiveTitles.length > 8) {
      return 'A Quest must keep between 1 and 8 objectives.';
    }
    var totalBytes = 0;
    final folded = <String>{};
    for (var index = 0; index < objectiveTitles.length; index++) {
      final objective = objectiveTitles[index];
      final problem = _literalProblem(
        objective,
        label: 'Objective ${index + 1}',
      );
      if (problem != null) return problem;
      totalBytes += utf8.encode(objective).length;
      if (!folded.add(objective.toLowerCase())) {
        return 'Each objective needs a different title.';
      }
    }
    if (totalBytes > 1024) {
      return 'The objective titles are too long together.';
    }
    return null;
  }

  static String? _literalProblem(String value, {required String label}) {
    if (value.isEmpty || value.trim() != value) {
      return '$label cannot be empty or start/end with spaces.';
    }
    if (utf8.encode(value).length > 128) {
      return '$label must be at most 128 bytes.';
    }
    if (value.runes.any(
      (rune) => rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c,
    )) {
      return '$label can currently use plain ASCII text, without quotes or backslashes.';
    }
    return null;
  }

  static bool _isControl(int rune) =>
      rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);
}

final class Revision3QuestOutlineObjectiveEdit {
  const Revision3QuestOutlineObjectiveEdit({
    required this.slot,
    required this.title,
  });

  final int slot;
  final String title;
}

AuthoringDraftContentSeal? _requireStableObjectivePair(
  List<int>? objectiveSlots,
  AuthoringDraftContentSeal? expectedTransitionPlanSeal,
) {
  if ((objectiveSlots == null) != (expectedTransitionPlanSeal == null)) {
    throw const FormatException(
      'Stable objective slots and their transition plan seal must be supplied together.',
    );
  }
  return expectedTransitionPlanSeal;
}

final class Revision3QuestOutlineEditPublication {
  const Revision3QuestOutlineEditPublication({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
  });

  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
}

final class Revision3QuestOutlineRequiresReopenException implements Exception {
  const Revision3QuestOutlineRequiresReopenException();
}

final class Revision3QuestOutlineStaleCheckpointException implements Exception {
  const Revision3QuestOutlineStaleCheckpointException();
}
