import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import '../story/domain/story_catalog_adapter.dart';

const _maxCatalogIdBytes = 256;
const _maxChoiceDisplayNameBytes = 256;
const _maxQuestTitleBytes = 128;
const _maxQuestDescriptionBytes = 512;
const _maxQuestObjectiveBytes = 128;
const _maxQuestObjectives = 8;
const _maxQuestObjectiveTitlesBytes =
    _maxQuestObjectives * _maxQuestObjectiveBytes;

final _projectIdPattern = RegExp(r'^[0-9a-f]{32}$');

typedef Revision3QuestCatalogLoader =
    Future<Revision3QuestCatalog> Function(String gameRoot);

typedef Revision3QuestDraftPublisher =
    Future<Revision3QuestDraftPublication> Function({
      required String gameRoot,
      required Revision3QuestDraftAuthoringInput input,
    });

/// One safe picker row projected from a freshly verified Story catalog.
///
/// The technical catalog identity is retained for the native selector, but UI
/// surfaces should present only [displayName].
final class Revision3QuestCatalogChoice {
  Revision3QuestCatalogChoice({
    required String catalogId,
    required String displayName,
  }) : catalogId = _boundedCatalogText(
         catalogId,
         _maxCatalogIdBytes,
         'catalog choice identity',
       ),
       displayName = _boundedCatalogText(
         displayName,
         _maxChoiceDisplayNameBytes,
         'catalog choice name',
       );

  final String catalogId;
  final String displayName;
}

/// Closed, display-safe picker projection for the R3 Quest wizard.
final class Revision3QuestCatalog {
  Revision3QuestCatalog({
    required Iterable<Revision3QuestCatalogChoice> parents,
    required Iterable<Revision3QuestCatalogChoice> givers,
  }) : parents = _closedChoices(parents, 'Quest family'),
       givers = _closedChoices(givers, 'Quest giver');

  factory Revision3QuestCatalog.fromStoryCatalog(StoryCatalogAdapter adapter) {
    final availability = adapter.questAvailability;
    return Revision3QuestCatalog(
      parents: availability.parents.map(
        (choice) => Revision3QuestCatalogChoice(
          catalogId: choice.catalogId,
          displayName: choice.displayName,
        ),
      ),
      givers: availability.givers.map(
        (choice) => Revision3QuestCatalogChoice(
          catalogId: choice.catalogId,
          displayName: choice.displayName,
        ),
      ),
    );
  }

  final List<Revision3QuestCatalogChoice> parents;
  final List<Revision3QuestCatalogChoice> givers;

  bool containsParent(String catalogId) =>
      parents.any((choice) => choice.catalogId == catalogId);

  bool containsGiver(String catalogId) =>
      givers.any((choice) => choice.catalogId == catalogId);
}

/// Rebuilds the read-only Story catalog for the configured game generation.
///
/// The older Story workspace keeps Quest mutation disabled because it lacks a
/// collision inventory. The dedicated R3 transaction independently requires
/// and verifies its PatchReceipt collision artifact; this projection grants no
/// build or runtime authority.
final class Revision3QuestCatalogService {
  const Revision3QuestCatalogService(this._ffi);

  final ModFfi _ffi;

  Future<Revision3QuestCatalog> load(String gameRoot) async {
    if (gameRoot.isEmpty) {
      throw const FormatException(
        'A configured game installation is required.',
      );
    }
    final selections = await _ffi
        .authoringStoryCatalogV1BuildAndReadForGameRoot(gameRoot: gameRoot);
    return Revision3QuestCatalog.fromStoryCatalog(
      StoryCatalogAdapter.fromSelections(selections),
    );
  }
}

/// Injectable fresh-catalog boundary used by the visible R3 wizard.
final revision3QuestCatalogLoaderProvider =
    Provider<Revision3QuestCatalogLoader>(
      (ref) => Revision3QuestCatalogService(
        ModFfi(ref.read(coreServiceProvider)),
      ).load,
    );

/// Friendly author input. No entity ID, namespace, path, or generated symbol is
/// accepted from the UI.
final class Revision3QuestDraftAuthoringInput {
  Revision3QuestDraftAuthoringInput._({
    required this.parentCatalogId,
    required this.giverCatalogId,
    required this.title,
    required this.description,
    required this.objectiveTitle,
    required this.additionalObjectiveTitles,
  });

  factory Revision3QuestDraftAuthoringInput({
    required String parentCatalogId,
    required String giverCatalogId,
    required String title,
    required String description,
    required String objectiveTitle,
    List<String> additionalObjectiveTitles = const <String>[],
  }) {
    final first = _friendlyQuestText(
      objectiveTitle,
      _maxQuestObjectiveBytes,
      'Objective 1',
    );
    if (additionalObjectiveTitles.length >= _maxQuestObjectives) {
      throw const FormatException('A Quest can contain at most 8 objectives.');
    }
    final additional = <String>[];
    final folded = <String>{first.toLowerCase()};
    var totalBytes = utf8.encode(first).length;
    for (var index = 0; index < additionalObjectiveTitles.length; index++) {
      final value = _friendlyQuestText(
        additionalObjectiveTitles[index],
        _maxQuestObjectiveBytes,
        'Objective ${index + 2}',
      );
      totalBytes += utf8.encode(value).length;
      if (totalBytes > _maxQuestObjectiveTitlesBytes) {
        throw const FormatException('Quest objectives are too long together.');
      }
      if (!folded.add(value.toLowerCase())) {
        throw FormatException(
          'Objective ${index + 2} duplicates another objective.',
        );
      }
      additional.add(value);
    }
    return Revision3QuestDraftAuthoringInput._(
      parentCatalogId: _boundedCatalogText(
        parentCatalogId,
        _maxCatalogIdBytes,
        'Quest family',
      ),
      giverCatalogId: _boundedCatalogText(
        giverCatalogId,
        _maxCatalogIdBytes,
        'Quest giver',
      ),
      title: _friendlyQuestText(title, _maxQuestTitleBytes, 'Quest name'),
      description: _friendlyQuestText(
        description,
        _maxQuestDescriptionBytes,
        'Quest description',
      ),
      objectiveTitle: first,
      additionalObjectiveTitles: List<String>.unmodifiable(additional),
    );
  }

  final String parentCatalogId;
  final String giverCatalogId;
  final String title;
  final String description;
  final String objectiveTitle;
  final List<String> additionalObjectiveTitles;

  List<String> get objectiveTitles => List<String>.unmodifiable(<String>[
    objectiveTitle,
    ...additionalObjectiveTitles,
  ]);
}

/// Internal technical request derived from the current project ID/revision and
/// authored intent. The coordinator separately binds publication to the exact
/// project root and canonical head.
final class Revision3QuestDraftTechnicalPlan {
  const Revision3QuestDraftTechnicalPlan._({
    required this.questId,
    required this.scriptModuleId,
    required this.displayName,
    required this.intent,
  });

  factory Revision3QuestDraftTechnicalPlan.forCheckpoint({
    required String projectId,
    required int projectRevision,
    required Revision3QuestDraftAuthoringInput input,
  }) {
    if (!_projectIdPattern.hasMatch(projectId) ||
        projectId == '00000000000000000000000000000000') {
      throw const FormatException('Quest draft has no valid project identity.');
    }
    if (projectRevision < 0 || projectRevision > 0x7fffffffffffffff) {
      throw const FormatException('Quest draft has no valid project revision.');
    }
    final seed = jsonEncode(<String, Object?>{
      'schema': 1,
      'project_id': projectId,
      'project_revision': projectRevision,
      'parent_catalog_id': input.parentCatalogId,
      'giver_catalog_id': input.giverCatalogId,
      'title': input.title,
      'description': input.description,
      'objective_title': input.objectiveTitle,
      if (input.additionalObjectiveTitles.isNotEmpty)
        'additional_objective_titles': input.additionalObjectiveTitles,
    });
    final nameDigest = sha256
        .convert(utf8.encode('gore-mod-studio.r3-quest-names-v1\u0000$seed'))
        .toString();
    final suffix = nameDigest.substring(0, 10).toUpperCase();
    final token = _technicalToken(input.title);
    final pascalToken = token
        .split('_')
        .map(
          (segment) =>
              '${segment.substring(0, 1).toUpperCase()}${segment.substring(1).toLowerCase()}',
        )
        .join();
    final technicalId = 'GORE_${token.toUpperCase()}_$suffix';
    final moduleNamespace = 'GoreMods.Quests.$pascalToken$suffix';
    final textHelper = 'Gore$pascalToken${suffix}QuestText';

    return Revision3QuestDraftTechnicalPlan._(
      questId: _derivedEntityId('quest', seed),
      scriptModuleId: _derivedEntityId('script-module', seed),
      displayName: input.title,
      intent: AuthoringRevision3QuestDraftIntentV3(
        moduleNamespace: moduleNamespace,
        technicalId: technicalId,
        textHelper: textHelper,
        parentCatalogId: input.parentCatalogId,
        giverCatalogId: input.giverCatalogId,
        title: input.title,
        description: input.description,
        objectiveTitle: input.objectiveTitle,
        additionalObjectiveTitles: input.additionalObjectiveTitles,
      ),
    );
  }

  final String questId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringRevision3QuestDraftIntentV3 intent;
}

/// Exact published checkpoint identity returned to the UI without exposing
/// generated paths or source.
final class Revision3QuestDraftPublication {
  Revision3QuestDraftPublication({
    required String projectId,
    required int projectRevision,
    required String questId,
    required String scriptModuleId,
  }) : projectId = _requiredProjectId(projectId),
       projectRevision = _requiredRevision(projectRevision),
       questId = _requiredEntityId(questId, 'Quest'),
       scriptModuleId = _requiredEntityId(scriptModuleId, 'script module');

  final String projectId;
  final int projectRevision;
  final String questId;
  final String scriptModuleId;
}

/// Signals that a failed transaction poisoned exact-current verification. The
/// wizard must lock and the project must be reopened before any retry.
final class Revision3QuestDraftRequiresReopenException implements Exception {
  const Revision3QuestDraftRequiresReopenException();

  @override
  String toString() =>
      'The managed project must be reopened before Quest authoring can continue.';
}

/// Signals that the managed project advanced while this wizard was open.
///
/// The project itself remains usable, but an in-place retry would keep using
/// the wizard's old checkpoint-bound technical plan. A fresh wizard is needed.
final class Revision3QuestDraftStaleCheckpointException implements Exception {
  const Revision3QuestDraftStaleCheckpointException();

  @override
  String toString() =>
      'The Quest wizard must be reopened for the current managed checkpoint.';
}

List<Revision3QuestCatalogChoice> _closedChoices(
  Iterable<Revision3QuestCatalogChoice> source,
  String context,
) {
  final choices = source.toList(growable: false);
  if (choices.isEmpty || choices.length > 100000) {
    throw FormatException('$context choices are unavailable.');
  }
  final ids = <String>{};
  for (final choice in choices) {
    if (!ids.add(choice.catalogId)) {
      throw FormatException('$context choices contain a duplicate identity.');
    }
  }
  return List<Revision3QuestCatalogChoice>.unmodifiable(choices);
}

String _boundedCatalogText(String value, int maxBytes, String context) {
  if (value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > maxBytes ||
      value.runes.any((rune) => rune < 0x20 || rune == 0x7f)) {
    throw FormatException('$context is unavailable.');
  }
  return value;
}

String _friendlyQuestText(String value, int maxBytes, String context) {
  final normalized = value.trim();
  if (normalized.isEmpty) {
    throw FormatException('$context is required.');
  }
  if (utf8.encode(normalized).length > maxBytes) {
    throw FormatException('$context is too long.');
  }
  for (final rune in normalized.runes) {
    if (rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c) {
      throw FormatException(
        '$context currently supports plain text without line breaks, quotes, or backslashes.',
      );
    }
  }
  return normalized;
}

String _technicalToken(String title) {
  final output = StringBuffer();
  var previousSeparator = false;
  for (final rune in title.runes) {
    final isLetter =
        (rune >= 0x41 && rune <= 0x5a) || (rune >= 0x61 && rune <= 0x7a);
    final isDigit = rune >= 0x30 && rune <= 0x39;
    if (isLetter || isDigit) {
      if (output.length >= 24) break;
      output.writeCharCode(rune);
      previousSeparator = false;
    } else if (!previousSeparator && output.isNotEmpty && output.length < 24) {
      output.write('_');
      previousSeparator = true;
    }
  }
  var token = output.toString().replaceFirst(RegExp(r'_+$'), '');
  if (token.isEmpty) token = 'Quest';
  if (RegExp(r'^[0-9]').hasMatch(token)) token = 'Quest_$token';
  return token;
}

String _derivedEntityId(String domain, String seed) {
  var digest = sha256
      .convert(utf8.encode('gore-mod-studio.r3-$domain-id-v1\u0000$seed'))
      .toString();
  var value = digest.substring(0, 32);
  if (value == '00000000000000000000000000000000') {
    digest = sha256
        .convert(
          utf8.encode('gore-mod-studio.r3-$domain-id-v1-fallback\u0000$seed'),
        )
        .toString();
    value = digest.substring(0, 32);
  }
  if (value == '00000000000000000000000000000000') {
    throw StateError('Deterministic Quest identity derivation failed.');
  }
  return value;
}

String _requiredProjectId(String value) {
  if (!_projectIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw const FormatException('Quest publication has no valid project ID.');
  }
  return value;
}

int _requiredRevision(int value) {
  if (value < 0 || value > 0x7fffffffffffffff) {
    throw const FormatException('Quest publication has no valid revision.');
  }
  return value;
}

String _requiredEntityId(String value, String context) {
  if (!_projectIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw FormatException('$context publication identity is invalid.');
  }
  return value;
}
