import 'dart:convert';

import 'package:crypto/crypto.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3DialogLineEntryContentLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3DialogLineEntryTechnicalPublisher =
    Future<Revision3DialogLineEntryPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3DialogLineEntryTechnicalPlan plan,
    });

/// Exact, project-only read boundary. The Home layer binds the managed root
/// and WorkingHead; this workflow supplies only the visible catalog CAS data.
typedef Revision3DialogLineEntryLocalizationReader =
    Future<AuthoringRevision3DialogLocalizationReadResult> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    });

enum Revision3DialogLineEntryMode { create, reuseExact }

final class Revision3DialogLineEntryRequiresReopenException
    implements Exception {
  const Revision3DialogLineEntryRequiresReopenException();
}

final class Revision3DialogLineEntryStaleCheckpointException
    implements Exception {
  const Revision3DialogLineEntryStaleCheckpointException();
}

final class Revision3DialogLineEntryNoReusableTextException
    implements Exception {
  const Revision3DialogLineEntryNoReusableTextException();
}

/// UI-safe projection of one bounded locale preview. It intentionally carries
/// no entity ID or runtime LocID.
final class Revision3DialogReusableLocalePreview {
  const Revision3DialogReusableLocalePreview._({
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

/// Friendly exact-read result for the dialog. Technical identity stays inside
/// [Revision3DialogLineEntryAuthoringService].
final class Revision3DialogReusableLocalizationPreview {
  const Revision3DialogReusableLocalizationPreview._({required this.locales});

  final List<Revision3DialogReusableLocalePreview> locales;

  List<String> get authorableLocales => List<String>.unmodifiable(
    locales
        .where((locale) => locale.hasNonemptyText)
        .map((locale) => locale.locale),
  );

  Revision3DialogReusableLocalePreview? locale(String value) {
    for (final preview in locales) {
      if (preview.locale == value) return preview;
    }
    return null;
  }
}

/// Friendly exact-project choice. Technical identity and revision remain
/// hidden from normal UI but bind ReuseExact inside the managed session.
final class Revision3DialogReusableLocalizationChoice {
  const Revision3DialogReusableLocalizationChoice._({
    required this.id,
    required this.revision,
    required this.displayName,
    required this.locId,
    required this.locales,
  });

  final String id;
  final int revision;
  final String displayName;
  final String locId;
  final List<String> locales;

  String get displayLabel =>
      displayName.trim().isEmpty ? 'Existing project text' : displayName;

  bool matches(String query) {
    final folded = query.trim().toLowerCase();
    if (folded.isEmpty) return true;
    return <String>[
      displayName,
      locId,
      ...locales,
    ].any((value) => value.toLowerCase().contains(folded));
  }
}

/// Closed projection used by the guided dialog-line prerequisite workflow.
final class Revision3DialogLineEntryCatalog {
  const Revision3DialogLineEntryCatalog._({
    required this.projectId,
    required this.projectRevision,
    required this.checkpointFingerprint,
    required this.suggestedLocales,
    required this.reusableLocalizations,
    required this.entityIds,
    required this.primaryIdentitiesFolded,
  });

  factory Revision3DialogLineEntryCatalog.fromContentIndex(
    Revision3ContentIndex index,
  ) {
    final reusable = <Revision3DialogReusableLocalizationChoice>[];
    for (final entity in index.entities) {
      final summary = entity.summary.localizationEntry;
      if (entity.kind != Revision3ContentEntityKind.localizationEntry ||
          summary == null ||
          entity.problemCount != 0 ||
          summary.locales.isEmpty ||
          !authoringRevision3VoiceArchiveBasenameStemIsSafe(
            entity.summary.primaryIdentity,
          )) {
        continue;
      }
      final alreadyOwned = index
          .backlinksToEntity(entity.id)
          .any(
            (backlink) =>
                backlink.source.kind == Revision3ContentEntityKind.dialogLine &&
                backlink.reference.role == 'dialog_localization',
          );
      if (alreadyOwned) continue;
      reusable.add(
        Revision3DialogReusableLocalizationChoice._(
          id: entity.id,
          revision: entity.revision,
          displayName: entity.displayName,
          locId: entity.summary.primaryIdentity,
          locales: summary.locales,
        ),
      );
    }
    reusable.sort((left, right) {
      final label = left.displayLabel.toLowerCase().compareTo(
        right.displayLabel.toLowerCase(),
      );
      return label != 0 ? label : left.id.compareTo(right.id);
    });
    final locales = <String>{...index.authoringLocales};
    for (final choice in reusable) {
      locales.addAll(choice.locales);
    }
    if (locales.isEmpty) locales.add('de');
    final sortedLocales = locales.toList(growable: false)..sort();
    return Revision3DialogLineEntryCatalog._(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      checkpointFingerprint: _dialogEntryFingerprint(index),
      suggestedLocales: List<String>.unmodifiable(sortedLocales),
      reusableLocalizations: List.unmodifiable(reusable),
      entityIds: Set<String>.unmodifiable(
        index.entities.map((entity) => entity.id),
      ),
      primaryIdentitiesFolded: Set<String>.unmodifiable(
        index.entities.map(
          (entity) => entity.summary.primaryIdentity.toLowerCase(),
        ),
      ),
    );
  }

  final String projectId;
  final int projectRevision;
  final String checkpointFingerprint;
  final List<String> suggestedLocales;
  final List<Revision3DialogReusableLocalizationChoice> reusableLocalizations;
  final Set<String> entityIds;
  final Set<String> primaryIdentitiesFolded;

  Revision3DialogReusableLocalizationChoice? localization(String id) {
    for (final choice in reusableLocalizations) {
      if (choice.id == id) return choice;
    }
    return null;
  }

  bool sameCheckpoint(Revision3DialogLineEntryCatalog other) =>
      projectId == other.projectId &&
      projectRevision == other.projectRevision &&
      checkpointFingerprint == other.checkpointFingerprint;
}

sealed class Revision3DialogLineEntryInput {
  const Revision3DialogLineEntryInput._({
    required this.mode,
    required this.lineDisplayName,
    required this.speakerHint,
    required this.locale,
    required this.createVoiceSlot,
  });

  factory Revision3DialogLineEntryInput.create({
    required String lineDisplayName,
    String? speakerHint,
    required String locale,
    required String text,
    bool createVoiceSlot = true,
  }) => Revision3DialogLineEntryCreateInput._(
    lineDisplayName: _dialogEntryName(
      lineDisplayName,
      'line name',
      maxBytes: 192,
    ),
    speakerHint: _dialogEntryOptionalName(speakerHint, 'speaker'),
    locale: _dialogEntryLocale(locale),
    text: _dialogEntryText(text),
    createVoiceSlot: createVoiceSlot,
  );

  factory Revision3DialogLineEntryInput.reuseExact({
    required String lineDisplayName,
    String? speakerHint,
    required String locale,
    required String localizationId,
    bool createVoiceSlot = true,
  }) => Revision3DialogLineEntryReuseInput._(
    lineDisplayName: _dialogEntryName(
      lineDisplayName,
      'line name',
      maxBytes: 192,
    ),
    speakerHint: _dialogEntryOptionalName(speakerHint, 'speaker'),
    locale: _dialogEntryLocale(locale),
    localizationId: _dialogEntryEntityId(localizationId),
    createVoiceSlot: createVoiceSlot,
  );

  final Revision3DialogLineEntryMode mode;
  final String lineDisplayName;
  final String? speakerHint;
  final String locale;
  final bool createVoiceSlot;
}

final class Revision3DialogLineEntryCreateInput
    extends Revision3DialogLineEntryInput {
  const Revision3DialogLineEntryCreateInput._({
    required super.lineDisplayName,
    required super.speakerHint,
    required super.locale,
    required this.text,
    required super.createVoiceSlot,
  }) : super._(mode: Revision3DialogLineEntryMode.create);

  final String text;
}

final class Revision3DialogLineEntryReuseInput
    extends Revision3DialogLineEntryInput {
  const Revision3DialogLineEntryReuseInput._({
    required super.lineDisplayName,
    required super.speakerHint,
    required super.locale,
    required this.localizationId,
    required super.createVoiceSlot,
  }) : super._(mode: Revision3DialogLineEntryMode.reuseExact);

  final String localizationId;
}

/// Hidden technical intent derived solely from one fresh exact content index.
/// It deliberately lacks WorkingHead/currentProjectJson; the session binds
/// those only after entering its serialized publication lane.
final class Revision3DialogLineEntryTechnicalPlan {
  const Revision3DialogLineEntryTechnicalPlan._({
    required this.lineId,
    required this.lineDisplayName,
    required this.lineAuthoredIdentity,
    required this.speakerHint,
    required this.localization,
    required this.voiceSlot,
    required this.locale,
  });

  factory Revision3DialogLineEntryTechnicalPlan.forCheckpoint({
    required Revision3DialogLineEntryCatalog catalog,
    required Revision3DialogLineEntryInput input,
  }) {
    final used = <String>{...catalog.entityIds};
    final seed = jsonEncode(<String, Object?>{
      'project_id': catalog.projectId,
      'project_revision': catalog.projectRevision,
      'mode': input.mode.name,
      'name': input.lineDisplayName,
      'speaker': input.speakerHint,
      'locale': input.locale,
      if (input case Revision3DialogLineEntryCreateInput(:final text))
        'text': text,
      if (input case Revision3DialogLineEntryReuseInput(:final localizationId))
        'localization_id': localizationId,
    });
    final lineId = _dialogEntryUnusedId(
      'line',
      seed,
      used,
      occupiedPrimaryIdentitiesFolded: catalog.primaryIdentitiesFolded,
      primaryIdentityForCandidate: _dialogEntryLineIdentity,
    );
    used.add(lineId);
    final AuthoringRevision3DialogLocalizationIntentV1 localization;
    switch (input) {
      case Revision3DialogLineEntryCreateInput(:final text):
        final localizationId = _dialogEntryUnusedId(
          'localization',
          seed,
          used,
          occupiedPrimaryIdentitiesFolded: catalog.primaryIdentitiesFolded,
          primaryIdentityForCandidate: _dialogEntryLocalizationIdentity,
        );
        used.add(localizationId);
        localization = AuthoringRevision3DialogLocalizationCreateIntentV1(
          localizationId: localizationId,
          displayName: '${input.lineDisplayName} text',
          locId: _dialogEntryLocalizationIdentity(localizationId),
          texts: <String, String>{input.locale: text},
        );
      case Revision3DialogLineEntryReuseInput(:final localizationId):
        final choice = catalog.localization(localizationId);
        if (choice == null || !choice.locales.contains(input.locale)) {
          throw const Revision3DialogLineEntryStaleCheckpointException();
        }
        localization = AuthoringRevision3DialogLocalizationReuseExactIntentV1(
          localizationId: choice.id,
          expectedLocalizationRevision: choice.revision,
          expectedLocId: choice.locId,
        );
    }
    final slot = input.createVoiceSlot
        ? AuthoringRevision3DialogEmptyVoiceSlotIntentV1(
            slotId: _dialogEntryUnusedId('voice-slot', seed, used),
            locale: input.locale,
            displayName: '${input.lineDisplayName} ${input.locale} Voice',
          )
        : null;
    return Revision3DialogLineEntryTechnicalPlan._(
      lineId: lineId,
      lineDisplayName: input.lineDisplayName,
      lineAuthoredIdentity: _dialogEntryLineIdentity(lineId),
      speakerHint: input.speakerHint,
      localization: localization,
      voiceSlot: slot,
      locale: input.locale,
    );
  }

  final String lineId;
  final String lineDisplayName;
  final String lineAuthoredIdentity;
  final String? speakerHint;
  final AuthoringRevision3DialogLocalizationIntentV1 localization;
  final AuthoringRevision3DialogEmptyVoiceSlotIntentV1? voiceSlot;
  final String locale;
}

final class Revision3DialogLineEntryPublication {
  Revision3DialogLineEntryPublication({
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.localizationAction,
    required this.voiceSlotId,
    required this.locale,
  }) {
    _dialogEntryEntityId(projectId);
    _dialogEntryEntityId(lineId);
    _dialogEntryEntityId(localizationId);
    if (voiceSlotId != null) _dialogEntryEntityId(voiceSlotId!);
    _dialogEntryLocale(locale);
    if (projectRevision < 1) {
      throw const FormatException(
        'Dialog-line publication has an invalid project revision.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final AuthoringRevision3DialogLocalizationAction localizationAction;
  final String? voiceSlotId;
  final String locale;
}

final class Revision3DialogLineEntryAuthoringService {
  const Revision3DialogLineEntryAuthoringService({
    required this.loadContentIndex,
    required this.readExactLocalization,
    required this.publishTechnicalPlan,
  });

  final Revision3DialogLineEntryContentLoader loadContentIndex;
  final Revision3DialogLineEntryLocalizationReader readExactLocalization;
  final Revision3DialogLineEntryTechnicalPublisher publishTechnicalPlan;

  Future<Revision3DialogLineEntryCatalog> loadCatalog() async {
    try {
      return Revision3DialogLineEntryCatalog.fromContentIndex(
        await loadContentIndex(),
      );
    } on Revision3ContentRequiresReopenException {
      throw const Revision3DialogLineEntryRequiresReopenException();
    }
  }

  Future<Revision3DialogReusableLocalizationPreview>
  loadReusableLocalizationPreview({
    required Revision3DialogLineEntryCatalog checkpoint,
    required String localizationId,
  }) async {
    final choice = checkpoint.localization(localizationId);
    if (choice == null) {
      throw const Revision3DialogLineEntryStaleCheckpointException();
    }
    final exact = await readExactLocalization(
      expectedProjectId: checkpoint.projectId,
      expectedProjectRevision: checkpoint.projectRevision,
      localizationId: choice.id,
      expectedLocalizationRevision: choice.revision,
      expectedLocId: choice.locId,
    );
    return _dialogEntryPreviewForExactResult(
      checkpoint: checkpoint,
      choice: choice,
      exact: exact,
    );
  }

  Future<Revision3DialogLineEntryPublication> publish({
    required Revision3DialogLineEntryCatalog checkpoint,
    required Revision3DialogLineEntryInput input,
  }) async {
    final fresh = await loadCatalog();
    if (!checkpoint.sameCheckpoint(fresh)) {
      throw const Revision3DialogLineEntryStaleCheckpointException();
    }
    if (input case Revision3DialogLineEntryReuseInput(
      :final localizationId,
      :final locale,
    )) {
      final preview = await loadReusableLocalizationPreview(
        checkpoint: fresh,
        localizationId: localizationId,
      );
      if (preview.locale(locale)?.hasNonemptyText != true) {
        throw const Revision3DialogLineEntryNoReusableTextException();
      }
    }
    final plan = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
      catalog: fresh,
      input: input,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: fresh.projectId,
      expectedProjectRevision: fresh.projectRevision,
      plan: plan,
    );
    final expectedAction = input.mode == Revision3DialogLineEntryMode.create
        ? AuthoringRevision3DialogLocalizationAction.created
        : AuthoringRevision3DialogLocalizationAction.reusedExact;
    if (publication.projectId != fresh.projectId ||
        publication.projectRevision != fresh.projectRevision + 1 ||
        publication.lineId != plan.lineId ||
        publication.localizationId != plan.localization.localizationId ||
        publication.localizationAction != expectedAction ||
        publication.voiceSlotId != plan.voiceSlot?.slotId ||
        publication.locale != plan.locale) {
      throw const Revision3DialogLineEntryRequiresReopenException();
    }
    return publication;
  }
}

Revision3DialogReusableLocalizationPreview _dialogEntryPreviewForExactResult({
  required Revision3DialogLineEntryCatalog checkpoint,
  required Revision3DialogReusableLocalizationChoice choice,
  required AuthoringRevision3DialogLocalizationReadResult exact,
}) {
  final exactLocales = exact.locales.map((locale) => locale.locale).toList();
  if (exact.projectId != checkpoint.projectId ||
      exact.projectRevision != checkpoint.projectRevision ||
      exact.localizationId != choice.id ||
      exact.localizationRevision != choice.revision ||
      exact.locId != choice.locId ||
      exactLocales.length != choice.locales.length ||
      !_dialogEntrySameStrings(exactLocales, choice.locales)) {
    throw const Revision3DialogLineEntryStaleCheckpointException();
  }
  return Revision3DialogReusableLocalizationPreview._(
    locales: List.unmodifiable(
      exact.locales.map(
        (locale) => Revision3DialogReusableLocalePreview._(
          locale: locale.locale,
          text: locale.preview,
          truncated: locale.truncated,
          hasNonemptyText: locale.hasNonemptyText,
        ),
      ),
    ),
  );
}

bool _dialogEntrySameStrings(List<String> left, List<String> right) {
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

String _dialogEntryName(String value, String label, {int maxBytes = 256}) {
  final normalized = value.trim();
  if (normalized.isEmpty ||
      utf8.encode(normalized).length > maxBytes ||
      normalized.runes.any((rune) => rune < 0x20 || rune == 0x7f)) {
    throw FormatException('Enter a valid $label.');
  }
  return normalized;
}

String? _dialogEntryOptionalName(String? value, String label) {
  final normalized = value?.trim();
  return normalized == null || normalized.isEmpty
      ? null
      : _dialogEntryName(normalized, label);
}

String _dialogEntryText(String value) {
  if (value.trim().isEmpty ||
      value.contains('\u0000') ||
      utf8.encode(value).length > 64 * 1024) {
    throw const FormatException('Enter valid dialog text.');
  }
  return value;
}

String _dialogEntryLocale(String value) {
  final normalized = value.trim();
  if (!revision3VoiceLocaleIsCanonical(normalized)) {
    throw const FormatException(
      'Choose a canonical language such as de or en-US.',
    );
  }
  return normalized;
}

String _dialogEntryEntityId(String value) {
  if (!RegExp(r'^[0-9a-f]{32}$').hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw const FormatException('Choose exact current project text.');
  }
  return value;
}

String _dialogEntryUnusedId(
  String domain,
  String seed,
  Set<String> used, {
  Set<String> occupiedPrimaryIdentitiesFolded = const <String>{},
  String Function(String id)? primaryIdentityForCandidate,
}) {
  final lastCounter = used.length + occupiedPrimaryIdentitiesFolded.length + 1;
  for (var counter = 0; counter <= lastCounter; counter++) {
    final digest = sha256
        .convert(
          utf8.encode(
            'gore-mod-studio.r3-dialog-entry-$domain-v1\u0000$seed\u0000$counter',
          ),
        )
        .toString();
    final id = digest.substring(0, 32);
    final primaryIdentity = primaryIdentityForCandidate?.call(id);
    if (id != '00000000000000000000000000000000' &&
        !used.contains(id) &&
        (primaryIdentity == null ||
            !occupiedPrimaryIdentitiesFolded.contains(
              primaryIdentity.toLowerCase(),
            ))) {
      return id;
    }
  }
  throw const FormatException(
    'No collision-free project identity could be generated.',
  );
}

String _dialogEntryLineIdentity(String id) => 'GORE_DIALOG_${id.toUpperCase()}';

String _dialogEntryLocalizationIdentity(String id) =>
    'GORE_${id.toUpperCase()}';

String _dialogEntryFingerprint(Revision3ContentIndex index) {
  final canonical = jsonEncode(<String, Object?>{
    'project_id': index.projectId,
    'revision': index.projectRevision,
    'target_sha256': index.targetExecutableSha256,
    'target_bytes': index.targetExecutableByteLength,
    'locales': index.authoringLocales,
    'entities': [
      for (final entity in index.entities)
        <String, Object?>{
          'id': entity.id,
          'kind': entity.kind.wireName,
          'name': entity.displayName,
          'revision': entity.revision,
          'origin': entity.origin.type,
          'origin_label': entity.origin.label,
          'primary': entity.summary.primaryIdentity,
          'secondary': entity.summary.secondaryText,
          'references': [
            for (final reference in entity.references)
              <String, Object?>{
                'role': reference.role,
                'qualifier': reference.qualifier,
                'project_id': reference.target.projectId,
                'id': reference.target.entityId,
                'kind': reference.target.expectedKind.wireName,
                'resolution': reference.resolution.wireName,
              },
          ],
        },
    ],
  });
  return sha256.convert(utf8.encode(canonical)).toString();
}
