import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_dialog_line_authoring.dart';

typedef Revision3QuestTranscriptContentLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3QuestTranscriptLocalizationReader =
    Future<AuthoringRevision3DialogLocalizationReadResult> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    });

typedef Revision3QuestTranscriptReplacePublisher =
    Future<Revision3QuestTranscriptPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required Revision3QuestTranscriptReplaceTechnicalPlan plan,
    });

typedef Revision3QuestTranscriptCreatePublisher =
    Future<Revision3QuestTranscriptPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required Revision3QuestTranscriptCreateTechnicalPlan plan,
    });

final class Revision3QuestTranscriptRequiresReopenException
    implements Exception {
  const Revision3QuestTranscriptRequiresReopenException();
}

final class Revision3QuestTranscriptStaleCheckpointException
    implements Exception {
  const Revision3QuestTranscriptStaleCheckpointException();
}

final class Revision3QuestTranscriptObjective {
  const Revision3QuestTranscriptObjective({
    required this.slot,
    required this.title,
  });

  final int slot;
  final String title;
}

/// Friendly authored-text and Voice coverage for one locale. Counts and
/// status are projected facts; no entity identity is exposed.
final class Revision3QuestTranscriptLocaleCoverage {
  const Revision3QuestTranscriptLocaleCoverage({
    required this.locale,
    required this.hasAuthoredText,
    required this.hasVoiceSlot,
    required this.voiceTakeCount,
    required this.hasSelectedVoiceTake,
    required this.targetResolution,
  });

  final String locale;
  final bool hasAuthoredText;
  final bool hasVoiceSlot;
  final int voiceTakeCount;
  final bool hasSelectedVoiceTake;
  final Revision3ContentVoiceTargetResolution? targetResolution;
}

/// UI-safe line choice. [localizationStableKey] is opaque and is suitable for
/// workspace handoff; callers must never render it or [lineId].
final class Revision3QuestTranscriptLineChoice {
  const Revision3QuestTranscriptLineChoice._({
    required this.lineId,
    required this.displayLabel,
    required this.speakerLabel,
    required this.localizationStableKey,
    required this.authoredLocales,
    required this.localeCoverage,
    required this._lineRevision,
    required this._localizationId,
    required this._localizationRevision,
    required this._locId,
  });

  final String lineId;
  final String displayLabel;
  final String? speakerLabel;
  final String localizationStableKey;
  final List<String> authoredLocales;
  final List<Revision3QuestTranscriptLocaleCoverage> localeCoverage;
  final int _lineRevision;
  final String _localizationId;
  final int _localizationRevision;
  final String _locId;

  int get voiceSlotCount =>
      localeCoverage.where((locale) => locale.hasVoiceSlot).length;
  int get voiceTakeCount => localeCoverage.fold<int>(
    0,
    (total, locale) => total + locale.voiceTakeCount,
  );
  int get selectedVoiceTakeCount =>
      localeCoverage.where((locale) => locale.hasSelectedVoiceTake).length;
}

final class Revision3QuestTranscriptRow {
  const Revision3QuestTranscriptRow({
    required this.line,
    required this.objectiveSlot,
  });

  final Revision3QuestTranscriptLineChoice line;
  final int? objectiveSlot;

  String get lineId => line.lineId;
  String get displayLabel => line.displayLabel;
  String? get speakerLabel => line.speakerLabel;
  String get localizationStableKey => line.localizationStableKey;
  List<String> get authoredLocales => line.authoredLocales;
  List<Revision3QuestTranscriptLocaleCoverage> get localeCoverage =>
      line.localeCoverage;
  int get voiceSlotCount => line.voiceSlotCount;
  int get voiceTakeCount => line.voiceTakeCount;
  int get selectedVoiceTakeCount => line.selectedVoiceTakeCount;
}

final class Revision3QuestTranscriptProjection {
  const Revision3QuestTranscriptProjection._({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.questRevision,
    required this.objectives,
    required this.availableLines,
    required this.rows,
    required this.checkpointIdentity,
    required this._fingerprint,
    required this._moduleId,
    required this._moduleRevision,
  });

  final String projectId;
  final int projectRevision;
  final String questId;
  final int questRevision;
  final List<Revision3QuestTranscriptObjective> objectives;
  final List<Revision3QuestTranscriptLineChoice> availableLines;
  final List<Revision3QuestTranscriptRow> rows;
  final String checkpointIdentity;
  final String _fingerprint;
  final String _moduleId;
  final int _moduleRevision;

  Revision3QuestTranscriptObjective? objectiveBySlot(int slot) {
    for (final objective in objectives) {
      if (objective.slot == slot) return objective;
    }
    return null;
  }
}

final class Revision3QuestTranscriptDraftRow {
  const Revision3QuestTranscriptDraftRow({
    required this.line,
    required this.objectiveSlot,
  });

  final Revision3QuestTranscriptLineChoice line;
  final int? objectiveSlot;

  Revision3QuestTranscriptDraftRow withObjectiveSlot(int? value) =>
      Revision3QuestTranscriptDraftRow(line: line, objectiveSlot: value);
}

/// Mutable review-dialog model. It remains bound to exactly one projection;
/// all operations reject foreign choices and inactive objective slots.
final class Revision3QuestTranscriptDraft {
  Revision3QuestTranscriptDraft.fromProjection(
    Revision3QuestTranscriptProjection projection,
  ) : _projection = projection,
      _rows = [
        for (final row in projection.rows)
          Revision3QuestTranscriptDraftRow(
            line: row.line,
            objectiveSlot: row.objectiveSlot,
          ),
      ];

  final Revision3QuestTranscriptProjection _projection;
  final List<Revision3QuestTranscriptDraftRow> _rows;

  Revision3QuestTranscriptProjection get projection => _projection;
  List<Revision3QuestTranscriptDraftRow> get rows => List.unmodifiable(_rows);
  List<Revision3QuestTranscriptLineChoice> get unboundChoices {
    final bound = _rows.map((row) => row.line.lineId).toSet();
    return List.unmodifiable(
      _projection.availableLines.where((line) => !bound.contains(line.lineId)),
    );
  }

  void attach(
    Revision3QuestTranscriptLineChoice choice, {
    int? objectiveSlot,
    int? index,
  }) {
    _requireChoice(choice);
    _requireObjectiveSlot(objectiveSlot);
    if (_rows.any((row) => row.line.lineId == choice.lineId) ||
        _rows.length >= 256) {
      throw const FormatException(
        'This dialog line is already attached or the Quest reached 256 lines.',
      );
    }
    final insertionIndex = index ?? _rows.length;
    if (insertionIndex < 0 || insertionIndex > _rows.length) {
      throw RangeError('Transcript insertion index is out of range.');
    }
    _rows.insert(
      insertionIndex,
      Revision3QuestTranscriptDraftRow(
        line: choice,
        objectiveSlot: objectiveSlot,
      ),
    );
  }

  void reorder({required int fromIndex, required int toIndex}) {
    if (fromIndex < 0 ||
        fromIndex >= _rows.length ||
        toIndex < 0 ||
        toIndex >= _rows.length) {
      throw RangeError('Transcript reorder index is out of range.');
    }
    if (fromIndex == toIndex) return;
    final row = _rows.removeAt(fromIndex);
    _rows.insert(toIndex, row);
  }

  Revision3QuestTranscriptDraftRow detachAt(int index) {
    if (index < 0 || index >= _rows.length) {
      throw RangeError('Transcript row index is out of range.');
    }
    return _rows.removeAt(index);
  }

  void setObjectiveSlot({required int index, required int? objectiveSlot}) {
    if (index < 0 || index >= _rows.length) {
      throw RangeError('Transcript row index is out of range.');
    }
    _requireObjectiveSlot(objectiveSlot);
    _rows[index] = _rows[index].withObjectiveSlot(objectiveSlot);
  }

  void _requireChoice(Revision3QuestTranscriptLineChoice choice) {
    if (!_projection.availableLines.any(
      (candidate) => identical(candidate, choice),
    )) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
  }

  void _requireObjectiveSlot(int? slot) {
    if (slot != null && _projection.objectiveBySlot(slot) == null) {
      throw const FormatException('Choose an active Quest objective.');
    }
  }
}

final class Revision3QuestTranscriptLocalePreview {
  const Revision3QuestTranscriptLocalePreview({
    required this.locale,
    required this.text,
    required this.truncated,
    required this.hasNonemptyText,
  });

  final String locale;
  final String text;
  final bool truncated;
  final bool hasNonemptyText;
}

final class Revision3QuestTranscriptTextPreview {
  const Revision3QuestTranscriptTextPreview({
    required this.displayLabel,
    required this.locales,
  });

  final String displayLabel;
  final List<Revision3QuestTranscriptLocalePreview> locales;
}

final class Revision3QuestTranscriptReplaceTechnicalPlan {
  const Revision3QuestTranscriptReplaceTechnicalPlan._({
    required this.questId,
    required this.expectedQuestRevision,
    required this.expectedModuleId,
    required this.expectedModuleRevision,
    required this.bindings,
  });

  final String questId;
  final int expectedQuestRevision;
  final String expectedModuleId;
  final int expectedModuleRevision;
  final List<AuthoringRevision3QuestTranscriptBindingV1> bindings;
}

final class Revision3QuestTranscriptCreateTechnicalPlan {
  const Revision3QuestTranscriptCreateTechnicalPlan._({
    required this.questId,
    required this.expectedQuestRevision,
    required this.expectedModuleId,
    required this.expectedModuleRevision,
    required this.expectedTranscriptCount,
    required this.index,
    required this.objectiveSlot,
    required this.line,
  });

  final String questId;
  final int expectedQuestRevision;
  final String expectedModuleId;
  final int expectedModuleRevision;
  final int expectedTranscriptCount;
  final int index;
  final int? objectiveSlot;
  final Revision3DialogLineEntryTechnicalPlan line;
}

final class Revision3QuestTranscriptPublication {
  const Revision3QuestTranscriptPublication({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.transcriptCount,
    required this.createdLineId,
    required this.createdLocalizationId,
    required this.createdVoiceSlotId,
    required this.localizationAction,
  });

  final String projectId;
  final int projectRevision;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3QuestTranscriptMode mode;
  final int transcriptCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
}

/// Exact-checkpoint authoring workflow for ordering/grouping existing lines
/// and atomically creating + inserting one new line.
final class Revision3QuestTranscriptAuthoringService {
  const Revision3QuestTranscriptAuthoringService({
    required this.expectedHead,
    required this.loadContentIndex,
    required this.readExactLocalization,
    required this.publishReplace,
    required this.publishCreate,
  });

  final AuthoringWorkingHead expectedHead;
  final Revision3QuestTranscriptContentLoader loadContentIndex;
  final Revision3QuestTranscriptLocalizationReader readExactLocalization;
  final Revision3QuestTranscriptReplacePublisher publishReplace;
  final Revision3QuestTranscriptCreatePublisher publishCreate;

  Future<Revision3QuestTranscriptProjection> load({
    required String questId,
    required int expectedQuestRevision,
  }) async {
    try {
      return _projectionFromIndex(
        await loadContentIndex(),
        expectedHead: expectedHead,
        questId: questId,
        expectedQuestRevision: expectedQuestRevision,
      );
    } on Revision3ContentRequiresReopenException {
      throw const Revision3QuestTranscriptRequiresReopenException();
    }
  }

  Future<Revision3QuestTranscriptTextPreview> loadTextPreview({
    required Revision3QuestTranscriptProjection projection,
    required Revision3QuestTranscriptRow row,
  }) async {
    _requireBoundProjection(projection);
    if (!projection.rows.any((candidate) => identical(candidate, row))) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    final line = row.line;
    final exact = await readExactLocalization(
      expectedProjectId: projection.projectId,
      expectedProjectRevision: projection.projectRevision,
      expectedHead: expectedHead,
      localizationId: line._localizationId,
      expectedLocalizationRevision: line._localizationRevision,
      expectedLocId: line._locId,
    );
    if (exact.head.canonicalJson != expectedHead.canonicalJson ||
        exact.projectId != projection.projectId ||
        exact.projectRevision != projection.projectRevision ||
        exact.localizationId != line._localizationId ||
        exact.localizationRevision != line._localizationRevision ||
        exact.locId != line._locId ||
        !_sameStrings(
          exact.locales.map((locale) => locale.locale).toList(),
          line.authoredLocales,
        )) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    return Revision3QuestTranscriptTextPreview(
      displayLabel: line.displayLabel,
      locales: List.unmodifiable([
        for (final locale in exact.locales)
          Revision3QuestTranscriptLocalePreview(
            locale: locale.locale,
            text: locale.preview,
            truncated: locale.truncated,
            hasNonemptyText: locale.hasNonemptyText,
          ),
      ]),
    );
  }

  Future<Revision3QuestTranscriptPublication> replace({
    required Revision3QuestTranscriptProjection projection,
    required Revision3QuestTranscriptDraft draft,
  }) async {
    _requireBoundProjection(projection);
    if (!identical(draft.projection, projection)) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    final fresh = await load(
      questId: projection.questId,
      expectedQuestRevision: projection.questRevision,
    );
    if (fresh._fingerprint != projection._fingerprint) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    if (_draftMatchesProjection(draft, projection)) {
      throw const FormatException('Change the Quest transcript before saving.');
    }
    final plan = Revision3QuestTranscriptReplaceTechnicalPlan._(
      questId: projection.questId,
      expectedQuestRevision: projection.questRevision,
      expectedModuleId: projection._moduleId,
      expectedModuleRevision: projection._moduleRevision,
      bindings: List.unmodifiable([
        for (final row in draft.rows)
          AuthoringRevision3QuestTranscriptBindingV1(
            projectId: projection.projectId,
            lineId: row.line.lineId,
            objectiveSlot: row.objectiveSlot,
          ),
      ]),
    );
    final publication = await publishReplace(
      expectedProjectId: projection.projectId,
      expectedProjectRevision: projection.projectRevision,
      expectedHead: expectedHead,
      plan: plan,
    );
    _requirePublication(
      projection,
      publication,
      mode: AuthoringRevision3QuestTranscriptMode.replace,
      expectedCount: draft.rows.length,
    );
    return publication;
  }

  Future<Revision3QuestTranscriptPublication> createAndInsert({
    required Revision3QuestTranscriptProjection projection,
    required int index,
    required int? objectiveSlot,
    required Revision3DialogLineEntryTechnicalPlan line,
  }) async {
    _requireBoundProjection(projection);
    if (index < 0 || index > projection.rows.length) {
      throw RangeError('Transcript insertion index is out of range.');
    }
    if (objectiveSlot != null &&
        projection.objectiveBySlot(objectiveSlot) == null) {
      throw const FormatException('Choose an active Quest objective.');
    }
    final fresh = await load(
      questId: projection.questId,
      expectedQuestRevision: projection.questRevision,
    );
    if (fresh._fingerprint != projection._fingerprint) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    final publication = await publishCreate(
      expectedProjectId: projection.projectId,
      expectedProjectRevision: projection.projectRevision,
      expectedHead: expectedHead,
      plan: Revision3QuestTranscriptCreateTechnicalPlan._(
        questId: projection.questId,
        expectedQuestRevision: projection.questRevision,
        expectedModuleId: projection._moduleId,
        expectedModuleRevision: projection._moduleRevision,
        expectedTranscriptCount: projection.rows.length,
        index: index,
        objectiveSlot: objectiveSlot,
        line: line,
      ),
    );
    _requirePublication(
      projection,
      publication,
      mode: AuthoringRevision3QuestTranscriptMode.createAndInsert,
      expectedCount: projection.rows.length + 1,
      expectedLine: line,
    );
    return publication;
  }

  Revision3DialogLineEntryTechnicalPublisher createAndInsertPublisher({
    required Revision3QuestTranscriptProjection projection,
    required int index,
    required int? objectiveSlot,
  }) =>
      ({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3DialogLineEntryTechnicalPlan plan,
      }) async {
        if (expectedProjectId != projection.projectId ||
            expectedProjectRevision != projection.projectRevision) {
          throw const Revision3QuestTranscriptStaleCheckpointException();
        }
        final published = await createAndInsert(
          projection: projection,
          index: index,
          objectiveSlot: objectiveSlot,
          line: plan,
        );
        return Revision3DialogLineEntryPublication(
          projectId: published.projectId,
          projectRevision: published.projectRevision,
          lineId: published.createdLineId!,
          localizationId: published.createdLocalizationId!,
          localizationAction: published.localizationAction!,
          voiceSlotId: published.createdVoiceSlotId,
          locale: plan.locale,
        );
      };

  void _requireBoundProjection(Revision3QuestTranscriptProjection value) {
    final expectedIdentity = _checkpointIdentity(expectedHead);
    if (value.checkpointIdentity != expectedIdentity) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
  }
}

// Implemented below as projection helpers so UI code never handles raw IDs.
Revision3QuestTranscriptProjection _projectionFromIndex(
  Revision3ContentIndex index, {
  required AuthoringWorkingHead expectedHead,
  required String questId,
  required int expectedQuestRevision,
}) {
  final quest = index.entityById(questId);
  if (quest == null ||
      quest.kind != Revision3ContentEntityKind.questDraft ||
      quest.revision != expectedQuestRevision ||
      quest.problemCount != 0 ||
      quest.summary.questDraft == null) {
    throw const Revision3QuestTranscriptStaleCheckpointException();
  }
  final questFacts = quest.summary.questDraft!;
  final moduleReferences = quest.references
      .where((reference) => reference.role == 'draft_script_module')
      .toList(growable: false);
  if (moduleReferences.length != 1) {
    throw const FormatException(
      'The selected Quest has no exact generated-script relationship.',
    );
  }
  final moduleReference = moduleReferences.single;
  final module = index.entityById(moduleReference.target.entityId);
  if (moduleReference.qualifier != null ||
      moduleReference.resolution !=
          Revision3ContentReferenceResolution.resolved ||
      moduleReference.target.projectId != index.projectId ||
      moduleReference.target.expectedKind !=
          Revision3ContentEntityKind.scriptModule ||
      module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule ||
      module.problemCount != 0) {
    throw const FormatException(
      'The selected Quest generated script is not intact.',
    );
  }

  final objectives = <Revision3QuestTranscriptObjective>[];
  if (questFacts.objectiveSlots.isNotEmpty) {
    if (questFacts.objectiveSlots.length != questFacts.objectiveTitles.length) {
      throw const FormatException(
        'The selected Quest objective projection is inconsistent.',
      );
    }
    for (
      var objectiveIndex = 0;
      objectiveIndex < questFacts.objectiveSlots.length;
      objectiveIndex++
    ) {
      objectives.add(
        Revision3QuestTranscriptObjective(
          slot: questFacts.objectiveSlots[objectiveIndex],
          title:
              _visibleTranscriptValue(
                questFacts.objectiveTitles[objectiveIndex],
                forbiddenValues: <String>{index.projectId, quest.id, module.id},
              ) ??
              'Objective ${objectiveIndex + 1}',
        ),
      );
    }
  }

  final candidates = <_QuestTranscriptProjectedLine>[];
  for (final entity in index.entities) {
    if (entity.kind != Revision3ContentEntityKind.dialogLine ||
        entity.problemCount != 0 ||
        entity.summary.dialogLine == null) {
      continue;
    }
    final localizationReferences = entity.references
        .where((reference) => reference.role == 'dialog_localization')
        .toList(growable: false);
    if (localizationReferences.length != 1) continue;
    final localizationReference = localizationReferences.single;
    final localization = index.entityById(
      localizationReference.target.entityId,
    );
    if (localizationReference.qualifier != null ||
        localizationReference.resolution !=
            Revision3ContentReferenceResolution.resolved ||
        localizationReference.target.projectId != index.projectId ||
        localizationReference.target.expectedKind !=
            Revision3ContentEntityKind.localizationEntry ||
        localization == null ||
        localization.kind != Revision3ContentEntityKind.localizationEntry ||
        localization.problemCount != 0 ||
        localization.summary.localizationEntry == null) {
      continue;
    }
    final authoredLocales = localization.summary.localizationEntry!.locales;
    final slotFacts = <String, Revision3ContentVoiceSlotSummary>{};
    var intact = true;
    for (final reference in entity.references.where(
      (reference) => reference.role == 'dialog_voice_slot',
    )) {
      final locale = reference.qualifier;
      final slot = index.entityById(reference.target.entityId);
      if (locale == null ||
          reference.resolution !=
              Revision3ContentReferenceResolution.resolved ||
          reference.target.projectId != index.projectId ||
          reference.target.expectedKind !=
              Revision3ContentEntityKind.voiceSlot ||
          slot == null ||
          slot.kind != Revision3ContentEntityKind.voiceSlot ||
          slot.problemCount != 0 ||
          slot.summary.voiceSlot == null ||
          slotFacts.containsKey(locale)) {
        intact = false;
        break;
      }
      slotFacts[locale] = slot.summary.voiceSlot!;
    }
    if (!intact) continue;
    final locales = <String>{...authoredLocales, ...slotFacts.keys}.toList()
      ..sort();
    final forbidden = <String>{
      index.projectId,
      entity.id,
      localization.id,
      localization.summary.primaryIdentity,
      for (final reference in entity.references.where(
        (reference) => reference.role == 'dialog_voice_slot',
      ))
        reference.target.entityId,
    };
    final displayName = _visibleTranscriptValue(
      entity.displayName,
      forbiddenValues: forbidden,
    );
    final speaker = _visibleTranscriptValue(
      entity.summary.dialogLine!.speaker,
      forbiddenValues: forbidden,
    );
    candidates.add(
      _QuestTranscriptProjectedLine(
        entity: entity,
        localization: localization,
        baseLabel: displayName ?? speaker ?? 'Dialog line',
        speaker: speaker,
        authoredLocales: authoredLocales,
        localeCoverage: List.unmodifiable([
          for (final locale in locales)
            Revision3QuestTranscriptLocaleCoverage(
              locale: locale,
              hasAuthoredText: authoredLocales.contains(locale),
              hasVoiceSlot: slotFacts.containsKey(locale),
              voiceTakeCount: slotFacts[locale]?.candidateCount ?? 0,
              hasSelectedVoiceTake: slotFacts[locale]?.hasSelectedTake ?? false,
              targetResolution: slotFacts[locale]?.targetResolution,
            ),
        ]),
      ),
    );
  }
  candidates.sort((left, right) {
    final byLabel = left.baseLabel.toLowerCase().compareTo(
      right.baseLabel.toLowerCase(),
    );
    return byLabel != 0 ? byLabel : left.entity.id.compareTo(right.entity.id);
  });
  final labelCounts = <String, int>{};
  for (final candidate in candidates) {
    final folded = candidate.baseLabel.toLowerCase();
    labelCounts[folded] = (labelCounts[folded] ?? 0) + 1;
  }
  final ordinals = <String, int>{};
  final lines = <Revision3QuestTranscriptLineChoice>[];
  for (final candidate in candidates) {
    final folded = candidate.baseLabel.toLowerCase();
    final ordinal = (ordinals[folded] ?? 0) + 1;
    ordinals[folded] = ordinal;
    lines.add(
      Revision3QuestTranscriptLineChoice._(
        lineId: candidate.entity.id,
        displayLabel: labelCounts[folded] == 1
            ? candidate.baseLabel
            : '${candidate.baseLabel} ($ordinal)',
        speakerLabel: candidate.speaker,
        localizationStableKey: _transcriptLocalizationStableKey(
          projectId: index.projectId,
          localizationId: candidate.localization.id,
          locId: candidate.localization.summary.primaryIdentity,
        ),
        authoredLocales: List.unmodifiable(candidate.authoredLocales),
        localeCoverage: candidate.localeCoverage,
        lineRevision: candidate.entity.revision,
        localizationId: candidate.localization.id,
        localizationRevision: candidate.localization.revision,
        locId: candidate.localization.summary.primaryIdentity,
      ),
    );
  }
  final linesById = <String, Revision3QuestTranscriptLineChoice>{
    for (final line in lines) line.lineId: line,
  };
  final rows = <Revision3QuestTranscriptRow>[];
  for (final reference in quest.references.where(
    (reference) => reference.role == 'quest_transcript_line',
  )) {
    final line = linesById[reference.target.entityId];
    final objectiveSlot = reference.qualifier == null
        ? null
        : int.tryParse(reference.qualifier!);
    if (line == null ||
        reference.target.projectId != index.projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.dialogLine ||
        reference.resolution != Revision3ContentReferenceResolution.resolved ||
        (reference.qualifier != null && objectiveSlot == null) ||
        (objectiveSlot != null &&
            !objectives.any((objective) => objective.slot == objectiveSlot))) {
      throw const FormatException(
        'The selected Quest transcript contains an unavailable dialog line.',
      );
    }
    rows.add(
      Revision3QuestTranscriptRow(line: line, objectiveSlot: objectiveSlot),
    );
  }
  if (rows.length != questFacts.transcriptCount) {
    throw const FormatException(
      'The selected Quest transcript projection is incomplete.',
    );
  }
  final checkpointIdentity = _checkpointIdentity(expectedHead);
  final fingerprint = crypto.sha256
      .convert(
        utf8.encode(
          jsonEncode(<Object?>[
            checkpointIdentity,
            quest.id,
            quest.revision,
            module.id,
            module.revision,
            for (final objective in objectives)
              <Object?>[objective.slot, objective.title],
            for (final line in lines)
              <Object?>[
                line.lineId,
                line._lineRevision,
                line._localizationId,
                line._localizationRevision,
                line._locId,
                line.displayLabel,
                line.speakerLabel,
                line.authoredLocales,
                for (final locale in line.localeCoverage)
                  <Object?>[
                    locale.locale,
                    locale.hasAuthoredText,
                    locale.hasVoiceSlot,
                    locale.voiceTakeCount,
                    locale.hasSelectedVoiceTake,
                    locale.targetResolution?.name,
                  ],
              ],
            for (final row in rows) <Object?>[row.lineId, row.objectiveSlot],
          ]),
        ),
      )
      .toString();
  return Revision3QuestTranscriptProjection._(
    projectId: index.projectId,
    projectRevision: index.projectRevision,
    questId: quest.id,
    questRevision: quest.revision,
    objectives: List.unmodifiable(objectives),
    availableLines: List.unmodifiable(lines),
    rows: List.unmodifiable(rows),
    checkpointIdentity: checkpointIdentity,
    fingerprint: fingerprint,
    moduleId: module.id,
    moduleRevision: module.revision,
  );
}

final class _QuestTranscriptProjectedLine {
  const _QuestTranscriptProjectedLine({
    required this.entity,
    required this.localization,
    required this.baseLabel,
    required this.speaker,
    required this.authoredLocales,
    required this.localeCoverage,
  });

  final Revision3ContentEntity entity;
  final Revision3ContentEntity localization;
  final String baseLabel;
  final String? speaker;
  final List<String> authoredLocales;
  final List<Revision3QuestTranscriptLocaleCoverage> localeCoverage;
}

void _requirePublication(
  Revision3QuestTranscriptProjection projection,
  Revision3QuestTranscriptPublication publication, {
  required AuthoringRevision3QuestTranscriptMode mode,
  required int expectedCount,
  Revision3DialogLineEntryTechnicalPlan? expectedLine,
}) {
  if (publication.projectId != projection.projectId ||
      publication.projectRevision != projection.projectRevision + 1 ||
      publication.questId != projection.questId ||
      publication.questRevision != projection.questRevision + 1 ||
      publication.moduleId != projection._moduleId ||
      publication.moduleRevision != projection._moduleRevision ||
      publication.mode != mode ||
      publication.transcriptCount != expectedCount ||
      (expectedLine == null &&
          (publication.createdLineId != null ||
              publication.createdLocalizationId != null ||
              publication.createdVoiceSlotId != null ||
              publication.localizationAction != null)) ||
      (expectedLine != null &&
          (publication.createdLineId != expectedLine.lineId ||
              publication.createdLocalizationId !=
                  expectedLine.localization.localizationId ||
              publication.createdVoiceSlotId !=
                  expectedLine.voiceSlot?.slotId ||
              publication.localizationAction == null))) {
    throw const Revision3QuestTranscriptRequiresReopenException();
  }
}

bool _draftMatchesProjection(
  Revision3QuestTranscriptDraft draft,
  Revision3QuestTranscriptProjection projection,
) {
  final rows = draft.rows;
  if (rows.length != projection.rows.length) return false;
  for (var index = 0; index < rows.length; index++) {
    if (rows[index].line.lineId != projection.rows[index].lineId ||
        rows[index].objectiveSlot != projection.rows[index].objectiveSlot) {
      return false;
    }
  }
  return true;
}

final _standaloneTranscriptTechnicalId = RegExp(
  r'(^|[^A-Za-z0-9_])[0-9a-fA-F]{32,64}(?=$|[^A-Za-z0-9_])',
);

String? _visibleTranscriptValue(
  String? value, {
  required Set<String> forbiddenValues,
}) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty ||
      normalized.runes.any((rune) => rune < 0x20 || rune == 0x7f) ||
      _standaloneTranscriptTechnicalId.hasMatch(normalized)) {
    return null;
  }
  final folded = normalized.toLowerCase();
  for (final forbidden in forbiddenValues) {
    final token = forbidden.trim().toLowerCase();
    if (token.isNotEmpty && folded.contains(token)) return null;
  }
  return normalized;
}

String _transcriptLocalizationStableKey({
  required String projectId,
  required String localizationId,
  required String locId,
}) => crypto.sha256
    .convert(
      utf8.encode(
        'gore-mod-studio.localization-choice\u0000'
        '$projectId\u0000$localizationId\u0000$locId',
      ),
    )
    .toString()
    .substring(0, 24);

String _checkpointIdentity(AuthoringWorkingHead expectedHead) =>
    expectedHead.canonicalJson;

bool _sameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
