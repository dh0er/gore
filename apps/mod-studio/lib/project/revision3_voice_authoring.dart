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

/// One normal-mode dialog-line choice projected from an exact content index.
///
/// Technical identities remain available to the transaction planner but are
/// deliberately not part of [displayLabel].
final class Revision3VoiceDialogLineChoice {
  Revision3VoiceDialogLineChoice._({
    required this.lineId,
    required this.localizationId,
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
  final String localizationId;
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

  /// False means this exact line already owns a projected slot for [locale],
  /// but that slot graph is not safe enough to extend.
  bool isLocaleAuthorable(String locale) =>
      !_blockedExistingSlotLocales.contains(locale);
}

/// Friendly facts about an existing line/language Voice slot. The hidden slot
/// identity stays in the transaction planner.
final class Revision3VoiceExistingSlotSummary {
  const Revision3VoiceExistingSlotSummary({
    required this.candidateCount,
    required this.hasSelectedTake,
  });

  final int candidateCount;
  final bool hasSelectedTake;
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
  }) : entityIds = Set<String>.unmodifiable(entityIds);

  factory Revision3VoiceCatalog.fromContentIndex(Revision3ContentIndex index) {
    final projectedLines = <_Revision3VoiceProjectedLine>[];
    final locales = <String>{...index.authoringLocales};
    final slotOwners = _voiceSlotOwners(index);
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
          !_voiceLocalizationIdentityIsSafe(
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
          localizationId: localization.target.entityId,
          localizationIdentity: localizationIdentity,
          displayName: entity.displayName,
          speaker: entity.summary.primaryIdentity,
          baseLabel: _voiceLineBaseLabel(
            speaker: entity.summary.primaryIdentity,
            displayName: entity.displayName,
            localizationIdentity: localizationIdentity,
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
          localizationId: line.localizationId,
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
    );
  }

  final String projectId;
  final int projectRevision;
  final String checkpointFingerprint;
  final List<Revision3VoiceDialogLineChoice> lines;
  final List<String> suggestedLocales;
  final Set<String> entityIds;

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
}

final class _Revision3VoiceProjectedLine {
  const _Revision3VoiceProjectedLine({
    required this.lineId,
    required this.localizationId,
    required this.localizationIdentity,
    required this.displayName,
    required this.speaker,
    required this.baseLabel,
    required this.slotIdsByLocale,
    required this.slotSummariesByLocale,
    required this.blockedExistingSlotLocales,
  });

  final String lineId;
  final String localizationId;
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
  required String localizationIdentity,
}) {
  final visible = <String>[
    if (speaker case final value? when value != 'Dialog line') value,
    if (displayName.isNotEmpty && displayName != speaker) displayName,
  ];
  final lineLabel = visible.isEmpty
      ? 'Unnamed dialog line'
      : visible.join(' — ');
  return '$lineLabel · $localizationIdentity';
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

bool _voiceLocalizationIdentityIsSafe(String value) =>
    value.isNotEmpty &&
    utf8.encode(value).length <= 1024 &&
    !value.runes.any(_voiceControl);

Revision3VoiceExistingSlotSummary? _voiceExistingSlotSummary({
  required Revision3ContentIndex index,
  required String lineId,
  required String locale,
  required Revision3ContentReference reference,
  required List<_Revision3VoiceSlotOwner> owners,
}) {
  if (reference.qualifier != locale ||
      reference.resolution != Revision3ContentReferenceResolution.resolved ||
      reference.target.projectId != index.projectId ||
      reference.target.expectedKind != Revision3ContentEntityKind.voiceSlot ||
      owners.length != 1 ||
      owners.single.lineId != lineId ||
      owners.single.locale != locale) {
    return null;
  }
  final entity = index.entityById(reference.target.entityId);
  final details = entity?.summary.voiceSlot;
  if (entity == null ||
      entity.kind != Revision3ContentEntityKind.voiceSlot ||
      entity.problemCount != 0 ||
      entity.summary.primaryIdentity != locale ||
      details == null ||
      details.targetResolution !=
          Revision3ContentVoiceTargetResolution.unresolved ||
      details.candidateCount >= _maxVoiceSlotCandidates) {
    return null;
  }

  final candidates = <String, Revision3ContentEntity>{};
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
  }
  if (candidates.length != details.candidateCount) return null;

  final selected = entity.references
      .where((item) => item.role == 'voice_selected')
      .toList(growable: false);
  if (selected.length != (details.hasSelectedTake ? 1 : 0)) return null;
  if (selected case [final chosen]) {
    if (chosen.qualifier != null ||
        chosen.resolution != Revision3ContentReferenceResolution.resolved ||
        chosen.target.projectId != index.projectId ||
        chosen.target.expectedKind != Revision3ContentEntityKind.voiceTake ||
        !candidates.containsKey(chosen.target.entityId) ||
        candidates[chosen.target.entityId]!.summary.voiceTake!.status !=
            Revision3ContentVoiceTakeStatus.approved) {
      return null;
    }
  }
  return Revision3VoiceExistingSlotSummary(
    candidateCount: details.candidateCount,
    hasSelectedTake: details.hasSelectedTake,
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

final class Revision3VoiceTakeRequiresReopenException implements Exception {
  const Revision3VoiceTakeRequiresReopenException();
}

final class Revision3VoiceTakeStaleCheckpointException implements Exception {
  const Revision3VoiceTakeStaleCheckpointException();
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
