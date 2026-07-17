import 'dart:convert';

import 'package:crypto/crypto.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

const _maxVoiceSourcePathBytes = 32 * 1024;
const _maxVoiceTakeNameBytes = 256;
const _maxVoiceLogicalNameBytes = 1024;
const _maxVoiceDialogTextBytes = 64 * 1024;
const _maxVoiceSlotCandidates = 1024;

final _voiceEntityIdPattern = RegExp(r'^[0-9a-f]{32}$');

typedef Revision3VoiceContentIndexLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3VoiceTechnicalPublisher =
    Future<Revision3VoiceTakePublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTakeTechnicalPlan plan,
    });

typedef Revision3VoiceTargetTechnicalPublisher =
    Future<Revision3VoiceTargetPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3VoiceTargetTechnicalPlan plan,
    });

/// One normal-mode dialog-line choice projected from an exact content index.
///
/// Technical identities remain available to the transaction planner but are
/// deliberately not part of [displayLabel].
final class Revision3VoiceDialogLineChoice {
  Revision3VoiceDialogLineChoice._({
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.localizationIdentity,
    required this.displayName,
    required this.speaker,
    required this.displayLabel,
    required Map<String, String> slotIdsByLocale,
    required Map<String, Revision3VoiceExistingSlotSummary>
    slotSummariesByLocale,
    required Set<String> blockedExistingSlotLocales,
  }) : _slotIdsByLocale = Map<String, String>.unmodifiable(slotIdsByLocale),
       _slotSummariesByLocale =
           Map<String, Revision3VoiceExistingSlotSummary>.unmodifiable(
             slotSummariesByLocale,
           ),
       _blockedExistingSlotLocales = Set<String>.unmodifiable(
         blockedExistingSlotLocales,
       );

  final String lineId;
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String localizationIdentity;
  final String displayName;
  final String? speaker;
  final String displayLabel;
  final Map<String, String> _slotIdsByLocale;
  final Map<String, Revision3VoiceExistingSlotSummary> _slotSummariesByLocale;
  final Set<String> _blockedExistingSlotLocales;

  bool matches(String query) {
    final folded = query.trim().toLowerCase();
    if (folded.isEmpty) return false;
    return <String>[
      ?speaker,
      displayName,
      localizationIdentity,
      displayLabel,
    ].any((value) => value.toLowerCase().contains(folded));
  }

  String? slotIdForLocale(String locale) => _slotIdsByLocale[locale];

  Revision3VoiceExistingSlotSummary? slotSummaryForLocale(String locale) =>
      _slotSummariesByLocale[locale];

  List<String> get existingSlotLocales {
    final locales = _slotIdsByLocale.keys.toList(growable: false)..sort();
    return List.unmodifiable(locales);
  }

  /// False means this exact line already owns a projected slot for [locale],
  /// but that slot graph is not safe enough to extend.
  bool isLocaleAuthorable(String locale) {
    if (_blockedExistingSlotLocales.contains(locale)) return false;
    final summary = _slotSummariesByLocale[locale];
    return summary == null || summary.candidateCount < _maxVoiceSlotCandidates;
  }

  /// Whether this line owns one structurally intact existing slot for [locale].
  ///
  /// Unlike [isLocaleAuthorable], this deliberately ignores candidate
  /// capacity: resolving installed archive evidence does not add a take.
  bool isLocaleTargetable(String locale) =>
      !_blockedExistingSlotLocales.contains(locale) &&
      _slotIdsByLocale.containsKey(locale) &&
      _slotSummariesByLocale.containsKey(locale);
}

/// Friendly facts about an existing line/language Voice slot. The hidden slot
/// identity stays in the transaction planner.
final class Revision3VoiceExistingSlotSummary {
  Revision3VoiceExistingSlotSummary({
    required this.slotRevision,
    required this.isRemovableGeneratedSlot,
    required this.candidateCount,
    required this.hasSelectedTake,
    required this.targetResolution,
    required List<Revision3VoiceCandidateTake> candidates,
    required this.selectedTakeId,
  }) : candidates = List<Revision3VoiceCandidateTake>.unmodifiable(candidates) {
    if (candidateCount != this.candidates.length ||
        hasSelectedTake != (selectedTakeId != null) ||
        (selectedTakeId != null &&
            !this.candidates.any((take) => take.id == selectedTakeId))) {
      throw const FormatException('Voice slot selection facts disagree.');
    }
  }

  final int slotRevision;
  final bool isRemovableGeneratedSlot;
  final int candidateCount;
  final bool hasSelectedTake;
  final Revision3ContentVoiceTargetResolution targetResolution;
  final List<Revision3VoiceCandidateTake> candidates;

  /// Exact hidden identity used only to bind a selection transaction.
  final String? selectedTakeId;

  Revision3VoiceCandidateTake? candidate(String id) {
    for (final candidate in candidates) {
      if (candidate.id == id) return candidate;
    }
    return null;
  }
}

/// One visible candidate card backed by an exact hidden VoiceTake identity.
final class Revision3VoiceCandidateTake {
  const Revision3VoiceCandidateTake._({
    required this.id,
    required this.revision,
    required this.displayName,
    required this.displayLabel,
    required this.status,
    required this.previewFacts,
  });

  final String id;
  final int revision;
  final String displayName;
  final String displayLabel;
  final Revision3ContentVoiceTakeStatus status;

  /// Exact hidden CAS and Ogg facts used only by the preview planner. They are
  /// deliberately absent when the projected take does not expose one fully
  /// resolved `voice_audio` reference.
  final Revision3VoiceCandidatePreviewFacts? previewFacts;

  bool get canPreview => previewFacts != null;

  bool get isApproved => status == Revision3ContentVoiceTakeStatus.approved;

  String get statusLabel => switch (status) {
    Revision3ContentVoiceTakeStatus.draft => 'Draft',
    Revision3ContentVoiceTakeStatus.recorded => 'Recorded',
    Revision3ContentVoiceTakeStatus.reviewed => 'Reviewed',
    Revision3ContentVoiceTakeStatus.approved => 'Approved',
  };
}

/// Hidden exact-current VoiceTake facts. Presentation code must not render
/// these identities, seals, or logical filenames.
final class Revision3VoiceCandidatePreviewFacts {
  const Revision3VoiceCandidatePreviewFacts._({
    required this.assetSha256,
    required this.assetByteLength,
    required this.assetLogicalName,
    required this.codec,
    required this.channels,
    required this.sampleRate,
  });

  final String assetSha256;
  final int assetByteLength;
  final String assetLogicalName;
  final Revision3ContentVoiceOggCodec codec;
  final int channels;
  final int sampleRate;
}

/// Closed, friendly projection of all Voice-authorable lines in one exact R3
/// content checkpoint.
final class Revision3VoiceCatalog {
  Revision3VoiceCatalog._({
    required this.projectId,
    required this.projectRevision,
    required this.checkpointFingerprint,
    required this.lines,
    required this.suggestedLocales,
    required Set<String> entityIds,
    required Map<String, Set<String>> candidateSlotIdsByTake,
  }) : entityIds = Set<String>.unmodifiable(entityIds),
       _candidateSlotIdsByTake = Map<String, Set<String>>.unmodifiable(
         candidateSlotIdsByTake.map(
           (takeId, slotIds) =>
               MapEntry(takeId, Set<String>.unmodifiable(slotIds)),
         ),
       );

  factory Revision3VoiceCatalog.fromContentIndex(Revision3ContentIndex index) {
    final projectedLines = <_Revision3VoiceProjectedLine>[];
    final locales = <String>{...index.authoringLocales};
    final slotOwners = _voiceSlotOwners(index);
    final localInboundReferenceCounts = _voiceLocalInboundReferenceCounts(
      index,
    );
    final candidateSlotIdsByTake = _voiceCandidateSlotUses(index);
    for (final entity in index.entities) {
      if (entity.kind != Revision3ContentEntityKind.dialogLine ||
          entity.summary.dialogLine == null) {
        continue;
      }
      final localizationReferences = entity.references
          .where((reference) => reference.role == 'dialog_localization')
          .toList(growable: false);
      if (localizationReferences.length != 1) continue;
      final localization = localizationReferences.single;
      if (localization.qualifier != null ||
          localization.resolution !=
              Revision3ContentReferenceResolution.resolved ||
          localization.target.projectId != index.projectId ||
          localization.target.expectedKind !=
              Revision3ContentEntityKind.localizationEntry) {
        continue;
      }
      final localizationEntity = index.entityById(localization.target.entityId);
      if (localizationEntity == null ||
          localizationEntity.kind !=
              Revision3ContentEntityKind.localizationEntry ||
          localizationEntity.problemCount != 0 ||
          localization.target.entityId == entity.id ||
          !authoringRevision3VoiceArchiveBasenameStemIsSafe(
            localizationEntity.summary.primaryIdentity,
          )) {
        continue;
      }

      final slots = <String, String>{};
      final slotSummaries = <String, Revision3VoiceExistingSlotSummary>{};
      final blockedSlotLocales = <String>{};
      final referencesByLocale = <String, List<Revision3ContentReference>>{};
      var lineShapeInvalid = false;
      for (final reference in entity.references.where(
        (reference) => reference.role == 'dialog_voice_slot',
      )) {
        final locale = reference.qualifier;
        if (locale == null || !revision3VoiceLocaleIsCanonical(locale)) {
          lineShapeInvalid = true;
          break;
        }
        locales.add(locale);
        referencesByLocale
            .putIfAbsent(locale, () => <Revision3ContentReference>[])
            .add(reference);
      }
      if (lineShapeInvalid) continue;
      for (final entry in referencesByLocale.entries) {
        final locale = entry.key;
        final references = entry.value;
        if (references.length != 1) {
          blockedSlotLocales.add(locale);
          continue;
        }
        final reference = references.single;
        final summary = _voiceExistingSlotSummary(
          index: index,
          lineId: entity.id,
          locale: locale,
          reference: reference,
          owners: slotOwners[reference.target.entityId] ?? const [],
          localInboundReferenceCount:
              localInboundReferenceCounts[reference.target.entityId] ?? 0,
        );
        if (summary == null) {
          blockedSlotLocales.add(locale);
          continue;
        }
        slots[locale] = reference.target.entityId;
        slotSummaries[locale] = summary;
      }
      final localizationIdentity = localizationEntity.summary.primaryIdentity;
      projectedLines.add(
        _Revision3VoiceProjectedLine(
          lineId: entity.id,
          lineRevision: entity.revision,
          localizationId: localization.target.entityId,
          localizationRevision: localizationEntity.revision,
          localizationIdentity: localizationIdentity,
          displayName: entity.displayName,
          speaker: entity.summary.primaryIdentity,
          baseLabel: _voiceLineBaseLabel(
            speaker: entity.summary.primaryIdentity,
            displayName: entity.displayName,
          ),
          slotIdsByLocale: slots,
          slotSummariesByLocale: slotSummaries,
          blockedExistingSlotLocales: blockedSlotLocales,
        ),
      );
    }
    projectedLines.sort((left, right) {
      final byLabel = left.baseLabel.toLowerCase().compareTo(
        right.baseLabel.toLowerCase(),
      );
      return byLabel != 0 ? byLabel : left.lineId.compareTo(right.lineId);
    });
    if (projectedLines.isEmpty) {
      throw const FormatException(
        'This project has no intact dialog line that can receive a Voice take.',
      );
    }
    final duplicateCounts = <String, int>{};
    for (final line in projectedLines) {
      final key = line.baseLabel.toLowerCase();
      duplicateCounts[key] = (duplicateCounts[key] ?? 0) + 1;
    }
    final duplicateOrdinals = <String, int>{};
    final lines = <Revision3VoiceDialogLineChoice>[];
    for (final line in projectedLines) {
      final key = line.baseLabel.toLowerCase();
      final count = duplicateCounts[key]!;
      final ordinal = (duplicateOrdinals[key] ?? 0) + 1;
      duplicateOrdinals[key] = ordinal;
      lines.add(
        Revision3VoiceDialogLineChoice._(
          lineId: line.lineId,
          lineRevision: line.lineRevision,
          localizationId: line.localizationId,
          localizationRevision: line.localizationRevision,
          localizationIdentity: line.localizationIdentity,
          displayName: line.displayName,
          speaker: line.speaker,
          displayLabel: count == 1
              ? line.baseLabel
              : '${line.baseLabel} · $ordinal of $count',
          slotIdsByLocale: line.slotIdsByLocale,
          slotSummariesByLocale: line.slotSummariesByLocale,
          blockedExistingSlotLocales: line.blockedExistingSlotLocales,
        ),
      );
    }
    if (locales.isEmpty) locales.add('de');
    final sortedLocales = locales.toList(growable: false)..sort();
    return Revision3VoiceCatalog._(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      checkpointFingerprint: _contentFingerprint(index),
      lines: List<Revision3VoiceDialogLineChoice>.unmodifiable(lines),
      suggestedLocales: List<String>.unmodifiable(sortedLocales),
      entityIds: index.entities.map((entity) => entity.id).toSet(),
      candidateSlotIdsByTake: candidateSlotIdsByTake,
    );
  }

  final String projectId;
  final int projectRevision;
  final String checkpointFingerprint;
  final List<Revision3VoiceDialogLineChoice> lines;
  final List<String> suggestedLocales;
  final Set<String> entityIds;
  final Map<String, Set<String>> _candidateSlotIdsByTake;

  Revision3VoiceDialogLineChoice? line(String lineId) {
    for (final line in lines) {
      if (line.lineId == lineId) return line;
    }
    return null;
  }

  bool sameCheckpoint(Revision3VoiceCatalog other) =>
      projectId == other.projectId &&
      projectRevision == other.projectRevision &&
      checkpointFingerprint == other.checkpointFingerprint;

  /// Number of distinct local VoiceSlots that retain [takeId] as a candidate
  /// in this exact content checkpoint. `voice_selected` is deliberately not a
  /// second use: selection always points at the same slot candidate.
  int candidateSlotUseCount(String takeId) =>
      _candidateSlotIdsByTake[takeId]?.length ?? 0;

  bool candidateIsUsedBySlot(String takeId, String slotId) =>
      _candidateSlotIdsByTake[takeId]?.contains(slotId) ?? false;
}

final class _Revision3VoiceProjectedLine {
  const _Revision3VoiceProjectedLine({
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.localizationIdentity,
    required this.displayName,
    required this.speaker,
    required this.baseLabel,
    required this.slotIdsByLocale,
    required this.slotSummariesByLocale,
    required this.blockedExistingSlotLocales,
  });

  final String lineId;
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String localizationIdentity;
  final String displayName;
  final String? speaker;
  final String baseLabel;
  final Map<String, String> slotIdsByLocale;
  final Map<String, Revision3VoiceExistingSlotSummary> slotSummariesByLocale;
  final Set<String> blockedExistingSlotLocales;
}

String _voiceLineBaseLabel({
  required String? speaker,
  required String displayName,
}) {
  final visible = <String>[
    if (speaker case final value? when value != 'Dialog line') value,
    if (displayName.isNotEmpty && displayName != speaker) displayName,
  ];
  final lineLabel = visible.isEmpty
      ? 'Unnamed dialog line'
      : visible.join(' — ');
  return lineLabel;
}

final class _Revision3VoiceSlotOwner {
  const _Revision3VoiceSlotOwner({required this.lineId, required this.locale});

  final String lineId;
  final String? locale;
}

Map<String, List<_Revision3VoiceSlotOwner>> _voiceSlotOwners(
  Revision3ContentIndex index,
) {
  final owners = <String, List<_Revision3VoiceSlotOwner>>{};
  for (final entity in index.entities) {
    if (entity.kind != Revision3ContentEntityKind.dialogLine) continue;
    for (final reference in entity.references.where(
      (reference) => reference.role == 'dialog_voice_slot',
    )) {
      if (reference.target.projectId != index.projectId) continue;
      owners
          .putIfAbsent(
            reference.target.entityId,
            () => <_Revision3VoiceSlotOwner>[],
          )
          .add(
            _Revision3VoiceSlotOwner(
              lineId: entity.id,
              locale: reference.qualifier,
            ),
          );
    }
  }
  return owners;
}

Map<String, Set<String>> _voiceCandidateSlotUses(Revision3ContentIndex index) {
  final uses = <String, Set<String>>{};
  for (final entity in index.entities) {
    if (entity.kind != Revision3ContentEntityKind.voiceSlot) continue;
    for (final reference in entity.references.where(
      (reference) => reference.role == 'voice_candidate',
    )) {
      if (reference.qualifier != null ||
          reference.resolution !=
              Revision3ContentReferenceResolution.resolved ||
          reference.target.projectId != index.projectId ||
          reference.target.expectedKind !=
              Revision3ContentEntityKind.voiceTake) {
        continue;
      }
      uses
          .putIfAbsent(reference.target.entityId, () => <String>{})
          .add(entity.id);
    }
  }
  return uses;
}

Map<String, int> _voiceLocalInboundReferenceCounts(
  Revision3ContentIndex index,
) {
  final counts = <String, int>{};
  for (final entity in index.entities) {
    for (final reference in entity.references) {
      if (reference.target.projectId != index.projectId) continue;
      counts.update(
        reference.target.entityId,
        (count) => count + 1,
        ifAbsent: () => 1,
      );
    }
  }
  return counts;
}

Revision3VoiceExistingSlotSummary? _voiceExistingSlotSummary({
  required Revision3ContentIndex index,
  required String lineId,
  required String locale,
  required Revision3ContentReference reference,
  required List<_Revision3VoiceSlotOwner> owners,
  required int localInboundReferenceCount,
}) {
  if (reference.qualifier != locale ||
      reference.resolution != Revision3ContentReferenceResolution.resolved ||
      reference.target.projectId != index.projectId ||
      reference.target.expectedKind != Revision3ContentEntityKind.voiceSlot ||
      owners.length != 1 ||
      owners.single.lineId != lineId ||
      owners.single.locale != locale ||
      localInboundReferenceCount < 1) {
    return null;
  }
  final entity = index.entityById(reference.target.entityId);
  final details = entity?.summary.voiceSlot;
  if (entity == null ||
      entity.kind != Revision3ContentEntityKind.voiceSlot ||
      entity.problemCount != 0 ||
      entity.summary.primaryIdentity != locale ||
      details == null) {
    return null;
  }
  final generatedOwner = entity.origin.generatedOwner;
  final isRemovableGeneratedSlot =
      entity.origin.type == 'generated' &&
      entity.origin.label == 'gore-authoring.voice-slot' &&
      entity.origin.generatorVersion == 1 &&
      generatedOwner != null &&
      generatedOwner.projectId == index.projectId &&
      generatedOwner.entityId == lineId &&
      generatedOwner.expectedKind == Revision3ContentEntityKind.dialogLine &&
      localInboundReferenceCount == 1;

  final candidates = <String, Revision3ContentEntity>{};
  final orderedCandidates = <Revision3ContentEntity>[];
  for (final candidate in entity.references.where(
    (item) => item.role == 'voice_candidate',
  )) {
    if (candidate.qualifier != null ||
        candidate.resolution != Revision3ContentReferenceResolution.resolved ||
        candidate.target.projectId != index.projectId ||
        candidate.target.expectedKind != Revision3ContentEntityKind.voiceTake ||
        candidates.containsKey(candidate.target.entityId)) {
      return null;
    }
    final take = index.entityById(candidate.target.entityId);
    final takeFacts = take?.summary.voiceTake;
    if (take == null ||
        take.kind != Revision3ContentEntityKind.voiceTake ||
        take.problemCount != 0 ||
        takeFacts == null ||
        takeFacts.locale != locale) {
      return null;
    }
    candidates[candidate.target.entityId] = take;
    orderedCandidates.add(take);
  }
  if (candidates.length != details.candidateCount) return null;

  final selected = entity.references
      .where((item) => item.role == 'voice_selected')
      .toList(growable: false);
  if (selected.length != (details.hasSelectedTake ? 1 : 0)) return null;
  String? selectedTakeId;
  if (selected case [final chosen]) {
    if (chosen.qualifier != null ||
        chosen.resolution != Revision3ContentReferenceResolution.resolved ||
        chosen.target.projectId != index.projectId ||
        chosen.target.expectedKind != Revision3ContentEntityKind.voiceTake ||
        !candidates.containsKey(chosen.target.entityId)) {
      return null;
    }
    selectedTakeId = chosen.target.entityId;
  }
  final baseLabels = <String>[
    for (final take in orderedCandidates)
      take.displayName.trim().isEmpty ? 'Unnamed take' : take.displayName,
  ];
  final labelCounts = <String, int>{};
  for (final label in baseLabels) {
    final folded = label.toLowerCase();
    labelCounts[folded] = (labelCounts[folded] ?? 0) + 1;
  }
  final labelOrdinals = <String, int>{};
  final records = <Revision3VoiceCandidateTake>[];
  for (
    var candidateIndex = 0;
    candidateIndex < orderedCandidates.length;
    candidateIndex++
  ) {
    final take = orderedCandidates[candidateIndex];
    final label = baseLabels[candidateIndex];
    final folded = label.toLowerCase();
    final ordinal = (labelOrdinals[folded] ?? 0) + 1;
    labelOrdinals[folded] = ordinal;
    records.add(
      Revision3VoiceCandidateTake._(
        id: take.id,
        revision: take.revision,
        displayName: take.displayName,
        displayLabel: labelCounts[folded] == 1
            ? label
            : '$label · $ordinal of ${labelCounts[folded]}',
        status: take.summary.voiceTake!.status,
        previewFacts: _voiceCandidatePreviewFacts(index, take),
      ),
    );
  }
  return Revision3VoiceExistingSlotSummary(
    slotRevision: entity.revision,
    isRemovableGeneratedSlot: isRemovableGeneratedSlot,
    candidateCount: details.candidateCount,
    hasSelectedTake: details.hasSelectedTake,
    targetResolution: details.targetResolution,
    candidates: records,
    selectedTakeId: selectedTakeId,
  );
}

Revision3VoiceCandidatePreviewFacts? _voiceCandidatePreviewFacts(
  Revision3ContentIndex index,
  Revision3ContentEntity take,
) {
  final references = take.assetReferences
      .where((reference) => reference.role == 'voice_audio')
      .toList(growable: false);
  if (references.length != 1) return null;
  final reference = references.single;
  final logicalName = reference.logicalName;
  final asset = index.assetBySha256(reference.sha256);
  final takeFacts = take.summary.voiceTake;
  if (reference.resolution !=
          Revision3ContentAssetReferenceResolution.resolved ||
      reference.expectedMediaType != 'audio/ogg' ||
      logicalName == null ||
      !_voiceLogicalNameIsSafe(logicalName) ||
      asset == null ||
      asset.sha256 != reference.sha256 ||
      asset.byteLength != reference.byteLength ||
      asset.byteLength < 1 ||
      asset.mediaType != 'audio/ogg' ||
      asset.assetClass != Revision3ContentAssetClass.voiceAudio ||
      takeFacts == null) {
    return null;
  }
  return Revision3VoiceCandidatePreviewFacts._(
    assetSha256: reference.sha256,
    assetByteLength: reference.byteLength,
    assetLogicalName: logicalName,
    codec: takeFacts.codec,
    channels: takeFacts.channels,
    sampleRate: takeFacts.sampleRate,
  );
}

/// Friendly Voice input. The line identity originates only from a visible
/// catalog choice; slot/take IDs and the logical Ogg name are never typed by a
/// normal-mode user.
final class Revision3VoiceTakeAuthoringInput {
  Revision3VoiceTakeAuthoringInput._({
    required this.lineId,
    required this.locale,
    required this.sourcePath,
    required this.logicalName,
    required this.takeDisplayName,
    required this.status,
    required this.selectTake,
    required this.dialogText,
  });

  factory Revision3VoiceTakeAuthoringInput({
    required String lineId,
    required String locale,
    required String sourcePath,
    required String takeDisplayName,
    required AuthoringRevision3VoiceTakeStatus status,
    bool selectTake = false,
    String? dialogText,
  }) {
    if (!_voiceEntityIdPattern.hasMatch(lineId) || _isZeroId(lineId)) {
      throw const FormatException(
        'Choose a dialog line from the current project.',
      );
    }
    final normalizedLocale = locale.trim();
    if (!revision3VoiceLocaleIsCanonical(normalizedLocale)) {
      throw const FormatException(
        'Enter a canonical language code such as de or en-US.',
      );
    }
    if (sourcePath.isEmpty ||
        sourcePath.trim() != sourcePath ||
        utf8.encode(sourcePath).length > _maxVoiceSourcePathBytes ||
        sourcePath.contains('\u0000')) {
      throw const FormatException('Choose a bounded Ogg source file.');
    }
    final logicalName = _voiceSourceLeaf(sourcePath);
    if (!_voiceLogicalNameIsSafe(logicalName)) {
      throw const FormatException(
        'The Ogg file needs a safe single-file name and cannot use a Windows device name.',
      );
    }
    final normalizedName = takeDisplayName.trim();
    if (normalizedName.isEmpty ||
        utf8.encode(normalizedName).length > _maxVoiceTakeNameBytes ||
        normalizedName.runes.any(_voiceControl)) {
      throw const FormatException('Enter a valid take name.');
    }
    if (selectTake && status != AuthoringRevision3VoiceTakeStatus.approved) {
      throw const FormatException(
        'Only an approved take can become the selected take.',
      );
    }
    final normalizedText = dialogText?.trim();
    if (normalizedText != null &&
        (normalizedText.isEmpty ||
            utf8.encode(normalizedText).length > _maxVoiceDialogTextBytes ||
            normalizedText.contains('\u0000'))) {
      throw const FormatException(
        'Enter valid dialog text or leave text editing disabled.',
      );
    }
    return Revision3VoiceTakeAuthoringInput._(
      lineId: lineId,
      locale: normalizedLocale,
      sourcePath: sourcePath,
      logicalName: logicalName,
      takeDisplayName: normalizedName,
      status: status,
      selectTake: selectTake,
      dialogText: normalizedText,
    );
  }

  final String lineId;
  final String locale;
  final String sourcePath;
  final String logicalName;
  final String takeDisplayName;
  final AuthoringRevision3VoiceTakeStatus status;
  final bool selectTake;

  /// `null` preserves the LocalizationEntry exactly. A value is an explicit
  /// request to edit the selected locale's dialog text.
  final String? dialogText;
}

/// Collision-probed transaction details derived from one exact content
/// checkpoint. None of these identities are editable in normal UI.
final class Revision3VoiceTakeTechnicalPlan {
  Revision3VoiceTakeTechnicalPlan._({
    required this.sourcePath,
    required this.lineId,
    required this.slotId,
    required this.takeId,
    required this.locale,
    required this.text,
    required this.takeDisplayName,
    required this.logicalName,
    required this.status,
    required this.selectTake,
    required this.expectsSlotCreated,
  });

  factory Revision3VoiceTakeTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required Revision3VoiceTakeAuthoringInput input,
  }) {
    final line = catalog.line(input.lineId);
    if (line == null) {
      throw const Revision3VoiceTakeStaleCheckpointException();
    }
    if (!line.isLocaleAuthorable(input.locale)) {
      throw const FormatException(
        'This dialog line already has a Voice slot for that language, but its graph is not safe to extend.',
      );
    }
    final used = <String>{...catalog.entityIds};
    final seed = jsonEncode(<String, Object?>{
      'schema': 1,
      'project_id': catalog.projectId,
      'project_revision': catalog.projectRevision,
      'line_id': line.lineId,
      'locale': input.locale,
      'logical_name': input.logicalName,
      'take_display_name': input.takeDisplayName,
      'status': input.status.name,
      'select_take': input.selectTake,
      'text': ?input.dialogText,
    });
    final existingSlot = line.slotIdForLocale(input.locale);
    final slotId =
        existingSlot ??
        _deriveUnusedEntityId(
          'voice-slot',
          jsonEncode(<String, Object?>{
            'project_id': catalog.projectId,
            'line_id': line.lineId,
            'locale': input.locale,
          }),
          used,
        );
    used.add(slotId);
    final takeId = _deriveUnusedEntityId('voice-take', seed, used);
    return Revision3VoiceTakeTechnicalPlan._(
      sourcePath: input.sourcePath,
      lineId: line.lineId,
      slotId: slotId,
      takeId: takeId,
      locale: input.locale,
      text: input.dialogText,
      takeDisplayName: input.takeDisplayName,
      logicalName: input.logicalName,
      status: input.status,
      selectTake: input.selectTake,
      expectsSlotCreated: existingSlot == null,
    );
  }

  final String sourcePath;
  final String lineId;
  final String slotId;
  final String takeId;
  final String locale;
  final String? text;
  final String takeDisplayName;
  final String logicalName;
  final AuthoringRevision3VoiceTakeStatus status;
  final bool selectTake;
  final bool expectsSlotCreated;
}

final class Revision3VoiceTakePublication {
  Revision3VoiceTakePublication({
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.slotId,
    required this.takeId,
    required this.slotCreated,
    required this.selected,
  }) {
    if (!_voiceEntityIdPattern.hasMatch(projectId) ||
        _isZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        [
          lineId,
          slotId,
          takeId,
        ].any((id) => !_voiceEntityIdPattern.hasMatch(id) || _isZeroId(id))) {
      throw const FormatException('Voice publication identity is invalid.');
    }
  }

  final String projectId;
  final int projectRevision;
  final String lineId;
  final String slotId;
  final String takeId;
  final bool slotCreated;
  final bool selected;
}

/// Exact installed-archive target intent derived from a fresh content index.
/// Normal-mode users choose only the visible line and language.
final class Revision3VoiceTargetTechnicalPlan {
  Revision3VoiceTargetTechnicalPlan._({
    required this.lineId,
    required this.slotId,
    required this.locale,
    required this.locId,
  });

  factory Revision3VoiceTargetTechnicalPlan.forCheckpoint({
    required Revision3VoiceCatalog catalog,
    required String lineId,
    required String locale,
  }) {
    final line = catalog.line(lineId);
    if (line == null) {
      throw const Revision3VoiceTargetStaleCheckpointException();
    }
    if (!line.isLocaleTargetable(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(
          line.localizationIdentity,
        )) {
      throw const FormatException(
        'Choose an intact existing Voice slot from the current project.',
      );
    }
    final slotId = line.slotIdForLocale(locale)!;
    return Revision3VoiceTargetTechnicalPlan._(
      lineId: line.lineId,
      slotId: slotId,
      locale: locale,
      locId: line.localizationIdentity,
    );
  }

  final String lineId;
  final String slotId;
  final String locale;
  final String locId;
}

final class Revision3VoiceTargetPublication {
  Revision3VoiceTargetPublication({
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.slotId,
    required this.locale,
    required this.locId,
    required this.resolution,
    required this.matchCount,
  }) {
    if (!_voiceEntityIdPattern.hasMatch(projectId) ||
        _isZeroId(projectId) ||
        projectRevision < 0 ||
        projectRevision > 0x7fffffffffffffff ||
        [
          lineId,
          slotId,
        ].any((id) => !_voiceEntityIdPattern.hasMatch(id) || _isZeroId(id)) ||
        !revision3VoiceLocaleIsCanonical(locale) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        matchCount < 0 ||
        (resolution ==
                AuthoringRevision3VoiceTargetResolutionState.unresolved &&
            matchCount != 0) ||
        (resolution == AuthoringRevision3VoiceTargetResolutionState.resolved &&
            matchCount != 1) ||
        (resolution == AuthoringRevision3VoiceTargetResolutionState.ambiguous &&
            matchCount < 2)) {
      throw const FormatException('Voice target publication is invalid.');
    }
  }

  final String projectId;
  final int projectRevision;
  final String lineId;
  final String slotId;
  final String locale;
  final String locId;
  final AuthoringRevision3VoiceTargetResolutionState resolution;
  final int matchCount;
}

final class Revision3VoiceTakeRequiresReopenException implements Exception {
  const Revision3VoiceTakeRequiresReopenException();
}

final class Revision3VoiceTakeStaleCheckpointException implements Exception {
  const Revision3VoiceTakeStaleCheckpointException();
}

final class Revision3VoiceTargetRequiresReopenException implements Exception {
  const Revision3VoiceTargetRequiresReopenException();
}

final class Revision3VoiceTargetStaleCheckpointException implements Exception {
  const Revision3VoiceTargetStaleCheckpointException();
}

final class Revision3VoiceBuildRequiresReopenException implements Exception {
  const Revision3VoiceBuildRequiresReopenException();
}

final class Revision3VoiceBuildStaleCheckpointException implements Exception {
  const Revision3VoiceBuildStaleCheckpointException();
}

/// Fresh-index service boundary for the visible Voice wizard. The injected
/// technical publisher is responsible for enforcing the expected project
/// identity and revision again inside the current-project coordinator lane.
final class Revision3VoiceAuthoringService {
  const Revision3VoiceAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async =>
      Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());

  Future<Revision3VoiceTakePublication> publish({
    required Revision3VoiceCatalog checkpoint,
    required Revision3VoiceTakeAuthoringInput input,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTakeStaleCheckpointException();
    }
    final plan = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
      catalog: fresh,
      input: input,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.slotId != plan.slotId ||
        publication.takeId != plan.takeId ||
        publication.slotCreated != plan.expectsSlotCreated ||
        publication.selected != plan.selectTake) {
      throw const Revision3VoiceTakeRequiresReopenException();
    }
    return publication;
  }
}

/// Fresh-index service for installed Voice target resolution.
final class Revision3VoiceTargetAuthoringService {
  const Revision3VoiceTargetAuthoringService({
    required this.loadContentIndex,
    required this.publishTechnicalPlan,
  });

  final Revision3VoiceContentIndexLoader loadContentIndex;
  final Revision3VoiceTargetTechnicalPublisher publishTechnicalPlan;

  Future<Revision3VoiceCatalog> loadCatalog() async {
    try {
      return Revision3VoiceCatalog.fromContentIndex(await loadContentIndex());
    } on Revision3ContentRequiresReopenException {
      throw const Revision3VoiceTargetRequiresReopenException();
    }
  }

  Future<Revision3VoiceTargetPublication> resolve({
    required Revision3VoiceCatalog checkpoint,
    required String lineId,
    required String locale,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3VoiceTargetStaleCheckpointException();
    }
    final plan = Revision3VoiceTargetTechnicalPlan.forCheckpoint(
      catalog: fresh,
      lineId: lineId,
      locale: locale,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.slotId != plan.slotId ||
        publication.locale != plan.locale ||
        publication.locId != plan.locId) {
      throw const Revision3VoiceTargetRequiresReopenException();
    }
    return publication;
  }
}

String _contentFingerprint(Revision3ContentIndex index) {
  final canonical = jsonEncode(<String, Object?>{
    'project_id': index.projectId,
    'project_revision': index.projectRevision,
    'target_sha256': index.targetExecutableSha256,
    'target_byte_len': index.targetExecutableByteLength,
    'authoring_locales': index.authoringLocales,
    'entities': [
      for (final entity in index.entities)
        <String, Object?>{
          'id': entity.id,
          'kind': entity.kind.wireName,
          'display_name': entity.displayName,
          'revision': entity.revision,
          'origin_type': entity.origin.type,
          'origin_label': entity.origin.label,
          'summary_primary': entity.summary.primaryIdentity,
          'summary_secondary': entity.summary.secondaryText,
          'references': [
            for (final reference in entity.references)
              <String, Object?>{
                'role': reference.role,
                'qualifier': reference.qualifier,
                'project_id': reference.target.projectId,
                'entity_id': reference.target.entityId,
                'expected_kind': reference.target.expectedKind.wireName,
                'resolution': reference.resolution.wireName,
              },
          ],
          'asset_references': [
            for (final reference in entity.assetReferences)
              <String, Object?>{
                'role': reference.role,
                'sha256': reference.sha256,
                'byte_len': reference.byteLength,
                'logical_name': reference.logicalName,
                'expected_media_type': reference.expectedMediaType,
                'resolution': reference.resolution.wireName,
              },
          ],
        },
    ],
    'assets': [
      for (final asset in index.assets)
        <String, Object?>{
          'sha256': asset.sha256,
          'byte_len': asset.byteLength,
          'media_type': asset.mediaType,
          'class': asset.assetClass.wireName,
        },
    ],
  });
  return sha256.convert(utf8.encode(canonical)).toString();
}

String _deriveUnusedEntityId(String domain, String seed, Set<String> used) {
  for (var counter = 0; counter <= used.length + 1; counter++) {
    final digest = sha256
        .convert(
          utf8.encode(
            'gore-mod-studio.r3-$domain-id-v1\u0000$seed\u0000$counter',
          ),
        )
        .toString();
    final candidate = digest.substring(0, 32);
    if (!_isZeroId(candidate) && !used.contains(candidate)) return candidate;
  }
  throw StateError('A collision-free Voice identity could not be derived.');
}

String _voiceSourceLeaf(String path) {
  final normalized = path.replaceAll('\\', '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

bool _voiceLogicalNameIsSafe(String value) {
  if (value.trim() != value ||
      utf8.encode(value).length > _maxVoiceLogicalNameBytes ||
      value.length <= 4 ||
      value.substring(value.length - 4).toLowerCase() != '.ogg') {
    return false;
  }
  const forbidden = <int>{0x22, 0x2a, 0x2f, 0x3a, 0x3c, 0x3e, 0x3f, 0x5c, 0x7c};
  if (value.runes.any(
    (rune) => _voiceControl(rune) || forbidden.contains(rune),
  )) {
    return false;
  }
  final stem = value.substring(0, value.length - 4);
  if (stem.isEmpty || stem == '.' || stem == '..') return false;
  final deviceStem = stem.split('.').first.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(deviceStem)) return false;
  return !RegExp(r'^(?:COM|LPT)[1-9]$').hasMatch(deviceStem);
}

/// Exact canonical locale rule shared by normal UI and the transaction DTO.
bool revision3VoiceLocaleIsCanonical(String value) {
  if (value.isEmpty ||
      value.length > 35 ||
      value.codeUnits.any((unit) => unit > 0x7f)) {
    return false;
  }
  final segments = value.split('-');
  if (!RegExp(r'^[a-z]{2,8}$').hasMatch(segments.first)) return false;
  final canonical = StringBuffer(segments.first);
  for (final segment in segments.skip(1)) {
    if (!RegExp(r'^[A-Za-z0-9]{1,8}$').hasMatch(segment)) return false;
    canonical.write('-');
    if (segment.length == 4 && RegExp(r'^[A-Za-z]+$').hasMatch(segment)) {
      canonical.write(
        '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}',
      );
    } else if (segment.length == 2 &&
        RegExp(r'^[A-Za-z]+$').hasMatch(segment)) {
      canonical.write(segment.toUpperCase());
    } else {
      canonical.write(segment.toLowerCase());
    }
  }
  return canonical.toString() == value;
}

bool _voiceControl(int rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

bool _isZeroId(String value) => value == '00000000000000000000000000000000';
