import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3DialogLocalizationEditContentLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3DialogLocalizationEditSeedLoader =
    Future<AuthoringRevision3DialogLocalizationEditSeed> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    });

typedef Revision3DialogLocalizationEditTechnicalPublisher =
    Future<Revision3DialogLocalizationEditPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3DialogLocalizationEditTechnicalPlan plan,
    });

final class Revision3DialogLocalizationEditRequiresReopenException
    implements Exception {
  const Revision3DialogLocalizationEditRequiresReopenException();
}

final class Revision3DialogLocalizationEditStaleCheckpointException
    implements Exception {
  const Revision3DialogLocalizationEditStaleCheckpointException();
}

final class Revision3DialogLocalizationEditLockedVoiceTextException
    implements Exception {
  const Revision3DialogLocalizationEditLockedVoiceTextException();
}

/// UI-safe choice for one project-authored text entry.
///
/// [stableKey] is deliberately opaque. The managed entity identity, LocID,
/// entity revision, and WorkingHead stay inside this workflow.
final class Revision3DialogLocalizationChoice {
  const Revision3DialogLocalizationChoice._({
    required this.stableKey,
    required this.displayLabel,
    required this.locales,
    required this._localizationId,
    required this._localizationRevision,
    required this._locId,
  });

  final String stableKey;
  final String displayLabel;
  final List<String> locales;
  final String _localizationId;
  final int _localizationRevision;
  final String _locId;

  bool matches(String query) {
    final folded = query.trim().toLowerCase();
    if (folded.isEmpty) return true;
    return <String>[
      displayLabel,
      ...locales,
    ].any((value) => value.toLowerCase().contains(folded));
  }
}

/// Exact-project catalog containing only safely editable authored texts.
final class Revision3DialogLocalizationEditCatalog {
  const Revision3DialogLocalizationEditCatalog._({
    required this.projectId,
    required this.projectRevision,
    required this.choices,
    required this._checkpointFingerprint,
  });

  factory Revision3DialogLocalizationEditCatalog.fromContentIndex(
    Revision3ContentIndex index,
  ) {
    final candidates =
        index.entities
            .where(
              (entity) =>
                  entity.kind == Revision3ContentEntityKind.localizationEntry &&
                  entity.origin.type == 'new' &&
                  entity.problemCount == 0 &&
                  entity.summary.localizationEntry != null &&
                  entity.summary.localizationEntry!.locales.isNotEmpty,
            )
            .toList(growable: false)
          ..sort((left, right) {
            final leftName = _localizationFriendlyName(left.displayName);
            final rightName = _localizationFriendlyName(right.displayName);
            final byName = leftName.toLowerCase().compareTo(
              rightName.toLowerCase(),
            );
            return byName != 0 ? byName : left.id.compareTo(right.id);
          });

    final duplicateCounts = <String, int>{};
    for (final entity in candidates) {
      final folded = _localizationFriendlyName(
        entity.displayName,
      ).toLowerCase();
      duplicateCounts.update(folded, (count) => count + 1, ifAbsent: () => 1);
    }
    final duplicateIndexes = <String, int>{};
    final choices = <Revision3DialogLocalizationChoice>[];
    for (final entity in candidates) {
      final baseLabel = _localizationFriendlyName(entity.displayName);
      final folded = baseLabel.toLowerCase();
      final duplicateIndex = duplicateIndexes.update(
        folded,
        (index) => index + 1,
        ifAbsent: () => 1,
      );
      final locales = entity.summary.localizationEntry!.locales;
      choices.add(
        Revision3DialogLocalizationChoice._(
          stableKey: _localizationStableKey(
            projectId: index.projectId,
            localizationId: entity.id,
            locId: entity.summary.primaryIdentity,
          ),
          displayLabel: duplicateCounts[folded] == 1
              ? baseLabel
              : '$baseLabel ($duplicateIndex)',
          locales: List<String>.unmodifiable(locales),
          localizationId: entity.id,
          localizationRevision: entity.revision,
          locId: entity.summary.primaryIdentity,
        ),
      );
    }
    final fingerprint = crypto.sha256
        .convert(
          utf8.encode(
            jsonEncode(<Object?>[
              index.projectId,
              index.projectRevision,
              for (final choice in choices)
                <Object?>[
                  choice._localizationId,
                  choice._localizationRevision,
                  choice._locId,
                  choice.locales,
                ],
            ]),
          ),
        )
        .toString();
    return Revision3DialogLocalizationEditCatalog._(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      choices: List<Revision3DialogLocalizationChoice>.unmodifiable(choices),
      checkpointFingerprint: fingerprint,
    );
  }

  final String projectId;
  final int projectRevision;
  final List<Revision3DialogLocalizationChoice> choices;
  final String _checkpointFingerprint;

  Revision3DialogLocalizationChoice? choiceByStableKey(String stableKey) {
    for (final choice in choices) {
      if (choice.stableKey == stableKey) return choice;
    }
    return null;
  }
}

/// One locale row with the exact VoiceSlot constraints required by the editor.
final class Revision3DialogLocalizationLocaleSeed {
  const Revision3DialogLocalizationLocaleSeed._({
    required this.locale,
    required this.text,
    required this.hasVoiceSlot,
    required this.candidateCount,
    required this.textLocked,
    required this.canRemove,
  });

  /// Creates a UI-only row for a language newly added by the user.
  factory Revision3DialogLocalizationLocaleSeed.added({
    required String locale,
    required String text,
  }) => Revision3DialogLocalizationLocaleSeed._(
    locale: _localizationLocale(locale),
    text: _localizationText(text),
    hasVoiceSlot: false,
    candidateCount: 0,
    textLocked: false,
    canRemove: true,
  );

  final String locale;
  final String text;
  final bool hasVoiceSlot;
  final int candidateCount;
  final bool textLocked;
  final bool canRemove;
}

/// Friendly backlink shown to authors when one text is shared by dialog lines.
final class Revision3DialogLocalizationLineBacklink {
  const Revision3DialogLocalizationLineBacklink._({
    required this.lineId,
    required this.displayName,
    required this.displayLabel,
    required this.speakerLabel,
    required this.voiceSlotLocales,
  });

  /// Exact hidden orchestration identity for contextual Voice actions.
  ///
  /// Normal presentation must never render this value. It is exposed only so
  /// the host can carry the author's visible line selection into another
  /// exact managed workflow without asking them to search for it again.
  final String lineId;
  final String displayName;
  final String displayLabel;
  final String? speakerLabel;
  final List<String> voiceSlotLocales;
}

/// Exact-current editor seed with all technical mutation authority kept private.
final class Revision3DialogLocalizationEditSeed {
  const Revision3DialogLocalizationEditSeed._({
    required this.choice,
    required this.locales,
    required this.lineBacklinks,
    required this._projectId,
    required this._projectRevision,
    required this._checkpointFingerprint,
    required this._expectedHead,
    required this._localizationId,
    required this._localizationRevision,
    required this._locId,
  });

  final Revision3DialogLocalizationChoice choice;
  final List<Revision3DialogLocalizationLocaleSeed> locales;
  final List<Revision3DialogLocalizationLineBacklink> lineBacklinks;
  final String _projectId;
  final int _projectRevision;
  final String _checkpointFingerprint;
  final AuthoringWorkingHead _expectedHead;
  final String _localizationId;
  final int _localizationRevision;
  final String _locId;

  Revision3DialogLocalizationLocaleSeed? locale(String value) {
    for (final item in locales) {
      if (item.locale == value) return item;
    }
    return null;
  }
}

/// Complete locale/text replacement entered by the author.
final class Revision3DialogLocalizationEditInput {
  Revision3DialogLocalizationEditInput({required Map<String, String> texts})
    : texts = _localizationTexts(texts);

  final Map<String, String> texts;
}

/// Hidden technical plan derived only from a freshly reopened exact seed.
///
/// The managed session still binds current_project_json inside its serialized
/// mutation lane before it invokes the native prepare-only command.
final class Revision3DialogLocalizationEditTechnicalPlan {
  const Revision3DialogLocalizationEditTechnicalPlan._({
    required this.expectedHead,
    required this.localizationId,
    required this.expectedLocalizationRevision,
    required this.expectedLocId,
    required this.texts,
  });

  final AuthoringWorkingHead expectedHead;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String expectedLocId;
  final Map<String, String> texts;
}

final class Revision3DialogLocalizationEditPublication {
  Revision3DialogLocalizationEditPublication({
    required this.projectId,
    required this.projectRevision,
    required this.localizationId,
    required this.localizationRevision,
    required List<String> addedLocales,
    required List<String> removedLocales,
  }) : addedLocales = _localizationLocaleList(addedLocales),
       removedLocales = _localizationLocaleList(removedLocales) {
    _localizationEntityId(projectId);
    _localizationEntityId(localizationId);
    if (projectRevision < 1 || localizationRevision < 1) {
      throw const FormatException(
        'Localization publication has an invalid revision.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final String localizationId;
  final int localizationRevision;
  final List<String> addedLocales;
  final List<String> removedLocales;
}

/// Safe author-facing orchestration for discovering and editing project text.
final class Revision3DialogLocalizationEditAuthoringService {
  const Revision3DialogLocalizationEditAuthoringService({
    required this.loadContentIndex,
    required this.loadExactSeed,
    required this.publishTechnicalPlan,
  });

  final Revision3DialogLocalizationEditContentLoader loadContentIndex;
  final Revision3DialogLocalizationEditSeedLoader loadExactSeed;
  final Revision3DialogLocalizationEditTechnicalPublisher publishTechnicalPlan;

  Future<Revision3DialogLocalizationEditCatalog> loadCatalog() async {
    try {
      return Revision3DialogLocalizationEditCatalog.fromContentIndex(
        await loadContentIndex(),
      );
    } on Revision3ContentRequiresReopenException {
      throw const Revision3DialogLocalizationEditRequiresReopenException();
    }
  }

  Future<Revision3DialogLocalizationEditSeed> loadSeed({
    required Revision3DialogLocalizationEditCatalog catalog,
    required Revision3DialogLocalizationChoice choice,
  }) async {
    final exactChoice = catalog.choiceByStableKey(choice.stableKey);
    if (exactChoice == null || !identical(exactChoice, choice)) {
      throw const Revision3DialogLocalizationEditStaleCheckpointException();
    }
    final exact = await loadExactSeed(
      expectedProjectId: catalog.projectId,
      expectedProjectRevision: catalog.projectRevision,
      localizationId: choice._localizationId,
      expectedLocalizationRevision: choice._localizationRevision,
      expectedLocId: choice._locId,
    );
    return _localizationSeedFromExact(
      catalog: catalog,
      choice: choice,
      exact: exact,
    );
  }

  Future<Revision3DialogLocalizationEditPublication> publish({
    required Revision3DialogLocalizationEditSeed seed,
    required Revision3DialogLocalizationEditInput input,
  }) async {
    final freshCatalog = await loadCatalog();
    if (seed._projectId != freshCatalog.projectId ||
        seed._projectRevision != freshCatalog.projectRevision ||
        seed._checkpointFingerprint != freshCatalog._checkpointFingerprint) {
      throw const Revision3DialogLocalizationEditStaleCheckpointException();
    }
    final freshChoice = freshCatalog.choiceByStableKey(seed.choice.stableKey);
    if (freshChoice == null) {
      throw const Revision3DialogLocalizationEditStaleCheckpointException();
    }
    final freshSeed = await loadSeed(
      catalog: freshCatalog,
      choice: freshChoice,
    );
    final currentTexts = <String, String>{
      for (final locale in freshSeed.locales) locale.locale: locale.text,
    };
    if (_localizationSameTexts(currentTexts, input.texts)) {
      throw const FormatException('Change at least one dialog text.');
    }
    for (final entry in input.texts.entries) {
      final current = currentTexts[entry.key];
      if (entry.value.trim().isEmpty &&
          (current == null || current.trim().isNotEmpty)) {
        throw const FormatException(
          'A new or previously written dialog text must not be blank.',
        );
      }
    }
    for (final locale in freshSeed.locales) {
      final replacement = input.texts[locale.locale];
      if (locale.hasVoiceSlot &&
          (replacement == null || replacement.trim().isEmpty)) {
        throw const Revision3DialogLocalizationEditLockedVoiceTextException();
      }
      if (locale.textLocked && replacement != locale.text) {
        throw const Revision3DialogLocalizationEditLockedVoiceTextException();
      }
    }

    final plan = Revision3DialogLocalizationEditTechnicalPlan._(
      expectedHead: freshSeed._expectedHead,
      localizationId: freshSeed._localizationId,
      expectedLocalizationRevision: freshSeed._localizationRevision,
      expectedLocId: freshSeed._locId,
      texts: input.texts,
    );
    final publication = await publishTechnicalPlan(
      expectedProjectId: freshSeed._projectId,
      expectedProjectRevision: freshSeed._projectRevision,
      plan: plan,
    );
    final expectedAdded = input.texts.keys
        .where((locale) => !currentTexts.containsKey(locale))
        .toList(growable: false);
    final expectedRemoved = currentTexts.keys
        .where((locale) => !input.texts.containsKey(locale))
        .toList(growable: false);
    if (publication.projectId != freshSeed._projectId ||
        publication.projectRevision != freshSeed._projectRevision + 1 ||
        publication.localizationId != freshSeed._localizationId ||
        publication.localizationRevision !=
            freshSeed._localizationRevision + 1 ||
        !_localizationSameList(publication.addedLocales, expectedAdded) ||
        !_localizationSameList(publication.removedLocales, expectedRemoved)) {
      throw const Revision3DialogLocalizationEditRequiresReopenException();
    }
    return publication;
  }
}

Revision3DialogLocalizationEditSeed _localizationSeedFromExact({
  required Revision3DialogLocalizationEditCatalog catalog,
  required Revision3DialogLocalizationChoice choice,
  required AuthoringRevision3DialogLocalizationEditSeed exact,
}) {
  final exactLocales = exact.locales.map((locale) => locale.locale).toList();
  if (exact.projectId != catalog.projectId ||
      exact.projectRevision != catalog.projectRevision ||
      exact.localizationId != choice._localizationId ||
      exact.localizationRevision != choice._localizationRevision ||
      exact.locId != choice._locId ||
      !_localizationSameList(exactLocales, choice.locales)) {
    throw const Revision3DialogLocalizationEditStaleCheckpointException();
  }
  final locales = exact.locales
      .map(
        (locale) => Revision3DialogLocalizationLocaleSeed._(
          locale: locale.locale,
          text: locale.text,
          hasVoiceSlot: locale.voiceSlotPresent,
          candidateCount: locale.candidateCount,
          textLocked: locale.candidateCount > 0,
          canRemove: !locale.voiceSlotPresent,
        ),
      )
      .toList(growable: false);
  final duplicateCounts = <String, int>{};
  for (final line in exact.lineBacklinks) {
    final key = _localizationBacklinkVisibleKey(
      line.displayName,
      line.speakerHint,
    );
    duplicateCounts[key] = (duplicateCounts[key] ?? 0) + 1;
  }
  final duplicateOrdinals = <String, int>{};
  final lines = <Revision3DialogLocalizationLineBacklink>[];
  for (final line in exact.lineBacklinks) {
    final key = _localizationBacklinkVisibleKey(
      line.displayName,
      line.speakerHint,
    );
    final count = duplicateCounts[key]!;
    final ordinal = (duplicateOrdinals[key] ?? 0) + 1;
    duplicateOrdinals[key] = ordinal;
    lines.add(
      Revision3DialogLocalizationLineBacklink._(
        lineId: line.lineId,
        displayName: line.displayName,
        displayLabel: count == 1
            ? line.displayName
            : '${line.displayName} · $ordinal of $count',
        speakerLabel: line.speakerHint,
        voiceSlotLocales: line.voiceSlotLocales,
      ),
    );
  }
  return Revision3DialogLocalizationEditSeed._(
    choice: choice,
    locales: List<Revision3DialogLocalizationLocaleSeed>.unmodifiable(locales),
    lineBacklinks: List<Revision3DialogLocalizationLineBacklink>.unmodifiable(
      lines,
    ),
    projectId: exact.projectId,
    projectRevision: exact.projectRevision,
    checkpointFingerprint: catalog._checkpointFingerprint,
    expectedHead: exact.head,
    localizationId: exact.localizationId,
    localizationRevision: exact.localizationRevision,
    locId: exact.locId,
  );
}

String _localizationBacklinkVisibleKey(String displayName, String? speaker) =>
    '${displayName.trim().toLowerCase()}\u0000${speaker?.trim().toLowerCase() ?? ''}';

String _localizationFriendlyName(String value) {
  final normalized = value.trim();
  return normalized.isEmpty ? 'Project text' : normalized;
}

String _localizationStableKey({
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

Map<String, String> _localizationTexts(Map<String, String> input) {
  if (input.isEmpty || input.length > 1000) {
    throw const FormatException('Add at least one dialog language.');
  }
  final keys = input.keys.toList(growable: false)..sort();
  final texts = <String, String>{};
  var totalBytes = 0;
  var hasNonblank = false;
  for (final rawLocale in keys) {
    final locale = _localizationLocale(rawLocale);
    if (locale != rawLocale || texts.containsKey(locale)) {
      throw const FormatException('Use canonical unique language codes.');
    }
    final text = _localizationText(input[rawLocale]!);
    totalBytes += utf8.encode(text).length;
    if (totalBytes > 512 * 1024) {
      throw const FormatException('Dialog text is too large.');
    }
    hasNonblank |= text.trim().isNotEmpty;
    texts[locale] = text;
  }
  if (!hasNonblank) {
    throw const FormatException('Enter at least one dialog text.');
  }
  return Map<String, String>.unmodifiable(texts);
}

String _localizationLocale(String value) {
  if (!revision3VoiceLocaleIsCanonical(value)) {
    throw const FormatException(
      'Choose a canonical language such as de or en-US.',
    );
  }
  return value;
}

String _localizationText(String value) {
  if (value.contains('\u0000') || utf8.encode(value).length > 64 * 1024) {
    throw const FormatException('Enter valid dialog text.');
  }
  return value;
}

String _localizationEntityId(String value) {
  if (!RegExp(r'^[0-9a-f]{32}$').hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw const FormatException('Invalid managed project identity.');
  }
  return value;
}

List<String> _localizationLocaleList(List<String> input) {
  if (input.length > 1000) {
    throw const FormatException('Too many changed languages.');
  }
  final result = <String>[];
  String? previous;
  for (final rawLocale in input) {
    final locale = _localizationLocale(rawLocale);
    if (previous != null && previous.compareTo(locale) >= 0) {
      throw const FormatException('Changed languages are not canonical.');
    }
    result.add(locale);
    previous = locale;
  }
  return List<String>.unmodifiable(result);
}

bool _localizationSameTexts(
  Map<String, String> left,
  Map<String, String> right,
) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    if (right[entry.key] != entry.value) return false;
  }
  return true;
}

bool _localizationSameList(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
