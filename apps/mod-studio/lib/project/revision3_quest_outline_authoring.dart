import 'dart:convert';

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
  }) : objectiveTitles = List<String>.unmodifiable(objectiveTitles);

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

  final String questId;
  final int expectedQuestRevision;
  final String moduleId;
  final int expectedModuleRevision;
  final String displayName;
  final String title;
  final List<String> objectiveTitles;

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
