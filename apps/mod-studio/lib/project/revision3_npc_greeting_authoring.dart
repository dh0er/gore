import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_dialog_line_authoring.dart';

typedef Revision3NpcGreetingContentLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3NpcGreetingLocalizationReader =
    Future<AuthoringRevision3DialogLocalizationReadResult> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    });

typedef Revision3NpcGreetingReplacePublisher =
    Future<Revision3NpcGreetingPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required Revision3NpcGreetingReplaceTechnicalPlan plan,
    });

typedef Revision3NpcGreetingCreatePublisher =
    Future<Revision3NpcGreetingPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required Revision3NpcGreetingCreateTechnicalPlan plan,
    });

final class Revision3NpcGreetingRequiresReopenException implements Exception {
  const Revision3NpcGreetingRequiresReopenException();
}

final class Revision3NpcGreetingStaleCheckpointException implements Exception {
  const Revision3NpcGreetingStaleCheckpointException();
}

/// Friendly authored-text and Voice coverage for one locale. These are
/// projected facts only; no entity identity is exposed.
final class Revision3NpcGreetingLocaleCoverage {
  const Revision3NpcGreetingLocaleCoverage({
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

/// UI-safe DialogLine choice. [localizationStableKey] is an opaque handoff
/// token; callers must never render it or [lineId].
final class Revision3NpcGreetingLineChoice {
  const Revision3NpcGreetingLineChoice._({
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
  final List<Revision3NpcGreetingLocaleCoverage> localeCoverage;
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

final class Revision3NpcGreetingRow {
  const Revision3NpcGreetingRow({required this.line});

  final Revision3NpcGreetingLineChoice line;

  String get lineId => line.lineId;
  String get displayLabel => line.displayLabel;
  String? get speakerLabel => line.speakerLabel;
  String get localizationStableKey => line.localizationStableKey;
  List<String> get authoredLocales => line.authoredLocales;
  List<Revision3NpcGreetingLocaleCoverage> get localeCoverage =>
      line.localeCoverage;
  int get voiceSlotCount => line.voiceSlotCount;
  int get voiceTakeCount => line.voiceTakeCount;
  int get selectedVoiceTakeCount => line.selectedVoiceTakeCount;
}

final class Revision3NpcGreetingProjection {
  const Revision3NpcGreetingProjection._({
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.availableLines,
    required this.rows,
    required this.checkpointIdentity,
    required this._fingerprint,
    required this._moduleId,
    required this._moduleRevision,
  });

  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;
  final List<Revision3NpcGreetingLineChoice> availableLines;
  final List<Revision3NpcGreetingRow> rows;
  final String checkpointIdentity;
  final String _fingerprint;
  final String _moduleId;
  final int _moduleRevision;
}

final class Revision3NpcGreetingDraftRow {
  const Revision3NpcGreetingDraftRow({required this.line});

  final Revision3NpcGreetingLineChoice line;
}

/// Mutable review-dialog model bound to exactly one projected checkpoint.
final class Revision3NpcGreetingDraft {
  Revision3NpcGreetingDraft.fromProjection(
    Revision3NpcGreetingProjection projection,
  ) : _projection = projection,
      _rows = [
        for (final row in projection.rows)
          Revision3NpcGreetingDraftRow(line: row.line),
      ];

  final Revision3NpcGreetingProjection _projection;
  final List<Revision3NpcGreetingDraftRow> _rows;

  Revision3NpcGreetingProjection get projection => _projection;
  List<Revision3NpcGreetingDraftRow> get rows => List.unmodifiable(_rows);
  List<Revision3NpcGreetingLineChoice> get unboundChoices {
    final bound = _rows.map((row) => row.line.lineId).toSet();
    return List.unmodifiable(
      _projection.availableLines.where((line) => !bound.contains(line.lineId)),
    );
  }

  void attach(Revision3NpcGreetingLineChoice choice, {int? index}) {
    _requireChoice(choice);
    if (_rows.any((row) => row.line.lineId == choice.lineId) ||
        _rows.length >= 256) {
      throw const FormatException(
        'This dialog line is already attached or the NPC reached 256 greetings.',
      );
    }
    final insertionIndex = index ?? _rows.length;
    if (insertionIndex < 0 || insertionIndex > _rows.length) {
      throw RangeError('Greeting insertion index is out of range.');
    }
    _rows.insert(insertionIndex, Revision3NpcGreetingDraftRow(line: choice));
  }

  void reorder({required int fromIndex, required int toIndex}) {
    if (fromIndex < 0 ||
        fromIndex >= _rows.length ||
        toIndex < 0 ||
        toIndex >= _rows.length) {
      throw RangeError('Greeting reorder index is out of range.');
    }
    if (fromIndex == toIndex) return;
    final row = _rows.removeAt(fromIndex);
    _rows.insert(toIndex, row);
  }

  Revision3NpcGreetingDraftRow detachAt(int index) {
    if (index < 0 || index >= _rows.length) {
      throw RangeError('Greeting row index is out of range.');
    }
    return _rows.removeAt(index);
  }

  void _requireChoice(Revision3NpcGreetingLineChoice choice) {
    if (!_projection.availableLines.any(
      (candidate) => identical(candidate, choice),
    )) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
  }
}

final class Revision3NpcGreetingLocalePreview {
  const Revision3NpcGreetingLocalePreview({
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

final class Revision3NpcGreetingTextPreview {
  const Revision3NpcGreetingTextPreview({
    required this.displayLabel,
    required this.locales,
  });

  final String displayLabel;
  final List<Revision3NpcGreetingLocalePreview> locales;
}

final class Revision3NpcGreetingReplaceTechnicalPlan {
  const Revision3NpcGreetingReplaceTechnicalPlan._({
    required this.npcId,
    required this.expectedNpcRevision,
    required this.expectedModuleId,
    required this.expectedModuleRevision,
    required this.bindings,
  });

  final String npcId;
  final int expectedNpcRevision;
  final String expectedModuleId;
  final int expectedModuleRevision;
  final List<AuthoringRevision3NpcGreetingBindingV1> bindings;
}

final class Revision3NpcGreetingCreateTechnicalPlan {
  const Revision3NpcGreetingCreateTechnicalPlan._({
    required this.npcId,
    required this.expectedNpcRevision,
    required this.expectedModuleId,
    required this.expectedModuleRevision,
    required this.expectedGreetingCount,
    required this.index,
    required this.line,
  });

  final String npcId;
  final int expectedNpcRevision;
  final String expectedModuleId;
  final int expectedModuleRevision;
  final int expectedGreetingCount;
  final int index;
  final Revision3DialogLineEntryTechnicalPlan line;
}

final class Revision3NpcGreetingPublication {
  const Revision3NpcGreetingPublication({
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.greetingCount,
    required this.createdLineId,
    required this.createdLocalizationId,
    required this.createdVoiceSlotId,
    required this.localizationAction,
  });

  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3NpcGreetingMode mode;
  final int greetingCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
}

/// Exact-checkpoint workflow for attaching, ordering, detaching and atomically
/// creating one DialogLine in an NPC greeting list. It never claims runtime
/// topic construction or playable publication.
final class Revision3NpcGreetingAuthoringService {
  const Revision3NpcGreetingAuthoringService({
    required this.expectedHead,
    required this.loadContentIndex,
    required this.readExactLocalization,
    required this.publishReplace,
    required this.publishCreate,
  });

  final AuthoringWorkingHead expectedHead;
  final Revision3NpcGreetingContentLoader loadContentIndex;
  final Revision3NpcGreetingLocalizationReader readExactLocalization;
  final Revision3NpcGreetingReplacePublisher publishReplace;
  final Revision3NpcGreetingCreatePublisher publishCreate;

  Future<Revision3NpcGreetingProjection> load({
    required String npcId,
    required int expectedNpcRevision,
  }) async {
    try {
      return _npcGreetingProjectionFromIndex(
        await loadContentIndex(),
        expectedHead: expectedHead,
        npcId: npcId,
        expectedNpcRevision: expectedNpcRevision,
      );
    } on Revision3ContentRequiresReopenException {
      throw const Revision3NpcGreetingRequiresReopenException();
    }
  }

  Future<Revision3NpcGreetingTextPreview> loadTextPreview({
    required Revision3NpcGreetingProjection projection,
    required Revision3NpcGreetingRow row,
  }) async {
    _requireBoundProjection(projection);
    if (!projection.rows.any((candidate) => identical(candidate, row))) {
      throw const Revision3NpcGreetingStaleCheckpointException();
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
        !_npcGreetingSameStrings(
          exact.locales.map((locale) => locale.locale).toList(),
          line.authoredLocales,
        )) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    return Revision3NpcGreetingTextPreview(
      displayLabel: line.displayLabel,
      locales: List.unmodifiable([
        for (final locale in exact.locales)
          Revision3NpcGreetingLocalePreview(
            locale: locale.locale,
            text: locale.preview,
            truncated: locale.truncated,
            hasNonemptyText: locale.hasNonemptyText,
          ),
      ]),
    );
  }

  Future<Revision3NpcGreetingPublication> replace({
    required Revision3NpcGreetingProjection projection,
    required Revision3NpcGreetingDraft draft,
  }) async {
    _requireBoundProjection(projection);
    if (!identical(draft.projection, projection)) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    final fresh = await load(
      npcId: projection.npcId,
      expectedNpcRevision: projection.npcRevision,
    );
    if (fresh._fingerprint != projection._fingerprint) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    if (_npcGreetingDraftMatchesProjection(draft, projection)) {
      throw const FormatException('Change the NPC greetings before saving.');
    }
    final plan = Revision3NpcGreetingReplaceTechnicalPlan._(
      npcId: projection.npcId,
      expectedNpcRevision: projection.npcRevision,
      expectedModuleId: projection._moduleId,
      expectedModuleRevision: projection._moduleRevision,
      bindings: List.unmodifiable([
        for (final row in draft.rows)
          AuthoringRevision3NpcGreetingBindingV1(
            projectId: projection.projectId,
            lineId: row.line.lineId,
          ),
      ]),
    );
    final publication = await publishReplace(
      expectedProjectId: projection.projectId,
      expectedProjectRevision: projection.projectRevision,
      expectedHead: expectedHead,
      plan: plan,
    );
    _npcGreetingRequirePublication(
      projection,
      publication,
      mode: AuthoringRevision3NpcGreetingMode.replace,
      expectedCount: draft.rows.length,
    );
    return publication;
  }

  Future<Revision3NpcGreetingPublication> createAndInsert({
    required Revision3NpcGreetingProjection projection,
    required int index,
    required Revision3DialogLineEntryTechnicalPlan line,
  }) async {
    _requireBoundProjection(projection);
    if (index < 0 || index > projection.rows.length) {
      throw RangeError('Greeting insertion index is out of range.');
    }
    final fresh = await load(
      npcId: projection.npcId,
      expectedNpcRevision: projection.npcRevision,
    );
    if (fresh._fingerprint != projection._fingerprint) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    final publication = await publishCreate(
      expectedProjectId: projection.projectId,
      expectedProjectRevision: projection.projectRevision,
      expectedHead: expectedHead,
      plan: Revision3NpcGreetingCreateTechnicalPlan._(
        npcId: projection.npcId,
        expectedNpcRevision: projection.npcRevision,
        expectedModuleId: projection._moduleId,
        expectedModuleRevision: projection._moduleRevision,
        expectedGreetingCount: projection.rows.length,
        index: index,
        line: line,
      ),
    );
    _npcGreetingRequirePublication(
      projection,
      publication,
      mode: AuthoringRevision3NpcGreetingMode.createAndInsert,
      expectedCount: projection.rows.length + 1,
      expectedLine: line,
    );
    return publication;
  }

  Revision3DialogLineEntryTechnicalPublisher createAndInsertPublisher({
    required Revision3NpcGreetingProjection projection,
    required int index,
  }) =>
      ({
        required String expectedProjectId,
        required int expectedProjectRevision,
        required Revision3DialogLineEntryTechnicalPlan plan,
      }) async {
        if (expectedProjectId != projection.projectId ||
            expectedProjectRevision != projection.projectRevision) {
          throw const Revision3NpcGreetingStaleCheckpointException();
        }
        final published = await createAndInsert(
          projection: projection,
          index: index,
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

  void _requireBoundProjection(Revision3NpcGreetingProjection value) {
    if (value.checkpointIdentity != expectedHead.canonicalJson) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
  }
}

Revision3NpcGreetingProjection _npcGreetingProjectionFromIndex(
  Revision3ContentIndex index, {
  required AuthoringWorkingHead expectedHead,
  required String npcId,
  required int expectedNpcRevision,
}) {
  final npc = index.entityById(npcId);
  if (npc == null ||
      npc.kind != Revision3ContentEntityKind.npcDraft ||
      npc.revision != expectedNpcRevision ||
      npc.problemCount != 0 ||
      npc.summary.npcDraft == null) {
    throw const Revision3NpcGreetingStaleCheckpointException();
  }
  final npcFacts = npc.summary.npcDraft!;
  final moduleReferences = npc.references
      .where((reference) => reference.role == 'draft_script_module')
      .toList(growable: false);
  if (moduleReferences.length != 1) {
    throw const FormatException(
      'The selected NPC has no exact generated-script relationship.',
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
      'The selected NPC generated script is not intact.',
    );
  }

  final candidates = <_NpcGreetingProjectedLine>[];
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
      npc.id,
      npcFacts.uniqueName,
      npcFacts.moduleNamespace,
      module.id,
      module.summary.primaryIdentity,
      module.summary.secondaryText,
      entity.id,
      localization.id,
      localization.summary.primaryIdentity,
      for (final reference in entity.references.where(
        (reference) => reference.role == 'dialog_voice_slot',
      ))
        reference.target.entityId,
    };
    final displayName = _visibleNpcGreetingValue(
      entity.displayName,
      forbiddenValues: forbidden,
    );
    final speaker = _visibleNpcGreetingValue(
      entity.summary.dialogLine!.speaker,
      forbiddenValues: forbidden,
    );
    candidates.add(
      _NpcGreetingProjectedLine(
        entity: entity,
        localization: localization,
        baseLabel: displayName ?? speaker ?? 'Dialog line',
        speaker: speaker,
        authoredLocales: authoredLocales,
        localeCoverage: List.unmodifiable([
          for (final locale in locales)
            Revision3NpcGreetingLocaleCoverage(
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
  final lines = <Revision3NpcGreetingLineChoice>[];
  for (final candidate in candidates) {
    final folded = candidate.baseLabel.toLowerCase();
    final ordinal = (ordinals[folded] ?? 0) + 1;
    ordinals[folded] = ordinal;
    lines.add(
      Revision3NpcGreetingLineChoice._(
        lineId: candidate.entity.id,
        displayLabel: labelCounts[folded] == 1
            ? candidate.baseLabel
            : '${candidate.baseLabel} ($ordinal)',
        speakerLabel: candidate.speaker,
        localizationStableKey: _npcGreetingLocalizationStableKey(
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
  final linesById = <String, Revision3NpcGreetingLineChoice>{
    for (final line in lines) line.lineId: line,
  };
  final rows = <Revision3NpcGreetingRow>[];
  for (final reference in npc.references.where(
    (reference) => reference.role == 'npc_greeting_line',
  )) {
    final line = linesById[reference.target.entityId];
    if (line == null ||
        reference.qualifier != null ||
        reference.target.projectId != index.projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.dialogLine ||
        reference.resolution != Revision3ContentReferenceResolution.resolved) {
      throw const FormatException(
        'The selected NPC greeting list contains an unavailable dialog line.',
      );
    }
    rows.add(Revision3NpcGreetingRow(line: line));
  }
  if (rows.length != npcFacts.greetingCount) {
    throw const FormatException(
      'The selected NPC greeting projection is incomplete.',
    );
  }
  final checkpointIdentity = expectedHead.canonicalJson;
  final fingerprint = crypto.sha256
      .convert(
        utf8.encode(
          jsonEncode(<Object?>[
            checkpointIdentity,
            npc.id,
            npc.revision,
            module.id,
            module.revision,
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
            for (final row in rows) row.lineId,
          ]),
        ),
      )
      .toString();
  return Revision3NpcGreetingProjection._(
    projectId: index.projectId,
    projectRevision: index.projectRevision,
    npcId: npc.id,
    npcRevision: npc.revision,
    availableLines: List.unmodifiable(lines),
    rows: List.unmodifiable(rows),
    checkpointIdentity: checkpointIdentity,
    fingerprint: fingerprint,
    moduleId: module.id,
    moduleRevision: module.revision,
  );
}

final class _NpcGreetingProjectedLine {
  const _NpcGreetingProjectedLine({
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
  final List<Revision3NpcGreetingLocaleCoverage> localeCoverage;
}

void _npcGreetingRequirePublication(
  Revision3NpcGreetingProjection projection,
  Revision3NpcGreetingPublication publication, {
  required AuthoringRevision3NpcGreetingMode mode,
  required int expectedCount,
  Revision3DialogLineEntryTechnicalPlan? expectedLine,
}) {
  if (publication.projectId != projection.projectId ||
      publication.projectRevision != projection.projectRevision + 1 ||
      publication.npcId != projection.npcId ||
      publication.npcRevision != projection.npcRevision + 1 ||
      publication.moduleId != projection._moduleId ||
      publication.moduleRevision != projection._moduleRevision ||
      publication.mode != mode ||
      publication.greetingCount != expectedCount ||
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
    throw const Revision3NpcGreetingRequiresReopenException();
  }
}

bool _npcGreetingDraftMatchesProjection(
  Revision3NpcGreetingDraft draft,
  Revision3NpcGreetingProjection projection,
) {
  final rows = draft.rows;
  if (rows.length != projection.rows.length) return false;
  for (var index = 0; index < rows.length; index++) {
    if (rows[index].line.lineId != projection.rows[index].lineId) return false;
  }
  return true;
}

final _standaloneNpcGreetingTechnicalId = RegExp(
  r'(^|[^A-Za-z0-9_])[0-9a-fA-F]{32,64}(?=$|[^A-Za-z0-9_])',
);

String? _visibleNpcGreetingValue(
  String? value, {
  required Set<String> forbiddenValues,
}) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty ||
      normalized.runes.any((rune) => rune < 0x20 || rune == 0x7f) ||
      _standaloneNpcGreetingTechnicalId.hasMatch(normalized)) {
    return null;
  }
  final folded = normalized.toLowerCase();
  for (final forbidden in forbiddenValues) {
    final token = forbidden.trim().toLowerCase();
    if (token.isNotEmpty && folded.contains(token)) return null;
  }
  return normalized;
}

String _npcGreetingLocalizationStableKey({
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

bool _npcGreetingSameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
