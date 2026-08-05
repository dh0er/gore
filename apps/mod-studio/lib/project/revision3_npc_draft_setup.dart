import 'revision3_content_index.dart';

/// The persisted steps of the bounded NPC Draft setup path.
///
/// These labels describe authored project facts only. They make no statement
/// about transactions, schemas, native integration, or runtime behavior.
enum Revision3NpcDraftSetupStepKind { characterDetails, firstGreeting }

/// Exact-current, read-only progress for the persistent NPC Draft setup path.
///
/// The projection consumes one already validated [Revision3ContentIndex] and
/// the exact NPC entity retained by that same index. It performs no reads or
/// mutations outside that projection and invents no separate completion flag.
final class Revision3NpcDraftSetup {
  const Revision3NpcDraftSetup._({
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.characterDetailsComplete,
    required this.firstGreetingComplete,
    required this.greetingLinkCount,
    required this.firstGreetingLineId,
    required this.firstGreetingLineRevision,
    required this.firstGreetingDetailsAvailable,
    required this.firstGreetingTextLanguageCount,
    required this.firstGreetingVoiceTakeCount,
    required this.firstGreetingSelectedVoiceTakeCount,
  });

  factory Revision3NpcDraftSetup.fromIndex({
    required Revision3ContentIndex index,
    required Revision3ContentEntity npc,
  }) {
    final facts = npc.summary.npcDraft;
    _requireNpcDraftSetupBinding(
      npc.kind == Revision3ContentEntityKind.npcDraft &&
          facts != null &&
          npc.problemCount == 0 &&
          identical(index.entityById(npc.id), npc),
    );

    final greetingReferences = npc.references
        .where((reference) => reference.role == 'npc_greeting_line')
        .toList(growable: false);
    _requireNpcDraftSetupBinding(
      greetingReferences.length == facts!.greetingCount,
    );

    _Revision3NpcDraftFirstGreetingFacts? firstGreeting;
    if (greetingReferences.isNotEmpty) {
      firstGreeting = _projectFirstGreeting(
        index: index,
        reference: greetingReferences.first,
      );
    }

    return Revision3NpcDraftSetup._(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      npcId: npc.id,
      npcRevision: npc.revision,
      characterDetailsComplete: <String>[
        npc.displayName,
        facts.uniqueName,
        facts.moduleNamespace,
        facts.parentCharacterDefinition,
        facts.parentAiAgentConfig,
        facts.parentSpawnDefinition,
      ].every((value) => value.trim().isNotEmpty),
      firstGreetingComplete: greetingReferences.isNotEmpty,
      greetingLinkCount: greetingReferences.length,
      firstGreetingLineId: firstGreeting?.lineId,
      firstGreetingLineRevision: firstGreeting?.lineRevision,
      firstGreetingDetailsAvailable: firstGreeting?.detailsAvailable ?? false,
      firstGreetingTextLanguageCount: firstGreeting?.authoredLocaleCount ?? 0,
      firstGreetingVoiceTakeCount: firstGreeting?.voiceTakeCount ?? 0,
      firstGreetingSelectedVoiceTakeCount:
          firstGreeting?.selectedVoiceTakeCount ?? 0,
    );
  }

  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;

  /// Whether all structured persisted Character facts are non-empty.
  final bool characterDetailsComplete;

  /// Whether the NPC has at least one authored greeting link.
  ///
  /// This is deliberately independent of localization and Voice coverage and
  /// must not be interpreted as conversation playability.
  final bool firstGreetingComplete;

  /// Total number of authored `npc_greeting_line` links in their stored order.
  final int greetingLinkCount;

  /// Exact identity of the first stored greeting link, when one exists.
  final String? firstGreetingLineId;
  final int? firstGreetingLineRevision;

  /// Whether the first line's text and Voice graph is fully resolved.
  final bool firstGreetingDetailsAvailable;

  /// Authored text and Voice coverage proven by the same exact content index.
  ///
  /// These counts are meaningful only when [firstGreetingDetailsAvailable] is
  /// true. A problem in a downstream Localization or Voice edge never removes
  /// the separately proven authored greeting link.
  final int firstGreetingTextLanguageCount;
  final int firstGreetingVoiceTakeCount;
  final int firstGreetingSelectedVoiceTakeCount;

  bool complete(Revision3NpcDraftSetupStepKind step) => switch (step) {
    Revision3NpcDraftSetupStepKind.characterDetails => characterDetailsComplete,
    Revision3NpcDraftSetupStepKind.firstGreeting => firstGreetingComplete,
  };

  bool get draftSetupComplete =>
      characterDetailsComplete && firstGreetingComplete;

  Revision3NpcDraftSetupStepKind get recommendedStep {
    if (!characterDetailsComplete) {
      return Revision3NpcDraftSetupStepKind.characterDetails;
    }
    return Revision3NpcDraftSetupStepKind.firstGreeting;
  }
}

/// One or more supposedly exact inputs no longer describe the same checkpoint.
final class Revision3NpcDraftSetupStaleCheckpointException
    implements Exception {
  const Revision3NpcDraftSetupStaleCheckpointException();
}

final class _Revision3NpcDraftFirstGreetingFacts {
  const _Revision3NpcDraftFirstGreetingFacts({
    required this.lineId,
    required this.lineRevision,
    required this.detailsAvailable,
    required this.authoredLocaleCount,
    required this.voiceTakeCount,
    required this.selectedVoiceTakeCount,
  });

  final String lineId;
  final int lineRevision;
  final bool detailsAvailable;
  final int authoredLocaleCount;
  final int voiceTakeCount;
  final int selectedVoiceTakeCount;
}

_Revision3NpcDraftFirstGreetingFacts _projectFirstGreeting({
  required Revision3ContentIndex index,
  required Revision3ContentReference reference,
}) {
  final line = index.entityById(reference.target.entityId);
  _requireNpcDraftSetupBinding(
    reference.qualifier == null &&
        reference.resolution == Revision3ContentReferenceResolution.resolved &&
        reference.target.projectId == index.projectId &&
        reference.target.expectedKind ==
            Revision3ContentEntityKind.dialogLine &&
        line != null &&
        line.kind == Revision3ContentEntityKind.dialogLine &&
        line.summary.dialogLine != null,
  );

  final localizationReferences = line!.references
      .where((candidate) => candidate.role == 'dialog_localization')
      .toList(growable: false);
  if (localizationReferences.length != 1) {
    return _unavailableFirstGreeting(line);
  }
  final localizationReference = localizationReferences.single;
  final localization = index.entityById(localizationReference.target.entityId);
  if (!(localizationReference.qualifier == null &&
      localizationReference.resolution ==
          Revision3ContentReferenceResolution.resolved &&
      localizationReference.target.projectId == index.projectId &&
      localizationReference.target.expectedKind ==
          Revision3ContentEntityKind.localizationEntry &&
      localization != null &&
      localization.kind == Revision3ContentEntityKind.localizationEntry &&
      localization.problemCount == 0 &&
      localization.summary.localizationEntry != null)) {
    return _unavailableFirstGreeting(line);
  }

  var voiceTakeCount = 0;
  var selectedVoiceTakeCount = 0;
  final seenLocales = <String>{};
  for (final slotReference in line.references.where(
    (candidate) => candidate.role == 'dialog_voice_slot',
  )) {
    final locale = slotReference.qualifier;
    final slot = index.entityById(slotReference.target.entityId);
    if (!(locale != null &&
        seenLocales.add(locale) &&
        slotReference.resolution ==
            Revision3ContentReferenceResolution.resolved &&
        slotReference.target.projectId == index.projectId &&
        slotReference.target.expectedKind ==
            Revision3ContentEntityKind.voiceSlot &&
        slot != null &&
        slot.kind == Revision3ContentEntityKind.voiceSlot &&
        slot.problemCount == 0 &&
        slot.summary.voiceSlot != null)) {
      return _unavailableFirstGreeting(line);
    }
    final slotFacts = slot.summary.voiceSlot!;
    voiceTakeCount += slotFacts.candidateCount;
    if (slotFacts.hasSelectedTake) selectedVoiceTakeCount++;
  }

  return _Revision3NpcDraftFirstGreetingFacts(
    lineId: line.id,
    lineRevision: line.revision,
    detailsAvailable: true,
    authoredLocaleCount: localization.summary.localizationEntry!.locales.length,
    voiceTakeCount: voiceTakeCount,
    selectedVoiceTakeCount: selectedVoiceTakeCount,
  );
}

_Revision3NpcDraftFirstGreetingFacts _unavailableFirstGreeting(
  Revision3ContentEntity line,
) => _Revision3NpcDraftFirstGreetingFacts(
  lineId: line.id,
  lineRevision: line.revision,
  detailsAvailable: false,
  authoredLocaleCount: 0,
  voiceTakeCount: 0,
  selectedVoiceTakeCount: 0,
);

void _requireNpcDraftSetupBinding(bool condition) {
  if (!condition) {
    throw const Revision3NpcDraftSetupStaleCheckpointException();
  }
}
