import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import '../story/domain/story_catalog_adapter.dart';
import '../story/domain/story_npc_archetype_index.dart';

const _maxCatalogIdBytes = 256;
const _maxChoiceDisplayNameBytes = 256;
const _maxNpcDisplayNameBytes = 256;

final _revision3NpcEntityIdPattern = RegExp(r'^[0-9a-f]{32}$');

typedef Revision3NpcCatalogLoader =
    Future<Revision3NpcCatalog> Function(String gameRoot);

typedef Revision3NpcDraftPublisher =
    Future<Revision3NpcDraftPublication> Function({
      required String gameRoot,
      required Revision3NpcDraftAuthoringInput input,
    });

/// One selectable, offline-qualified NPC archetype projected for normal UI.
///
/// The native catalog identity remains an opaque selector. Runtime class names,
/// source paths and provenance are intentionally absent from this normal-mode
/// value; the qualification-aware archetype picker owns their optional display.
final class Revision3NpcCatalogChoice {
  Revision3NpcCatalogChoice({
    required String catalogId,
    required String displayName,
  }) : catalogId = _boundedCatalogText(
         catalogId,
         _maxCatalogIdBytes,
         'NPC archetype identity',
       ),
       displayName = _boundedCatalogText(
         displayName,
         _maxChoiceDisplayNameBytes,
         'NPC archetype name',
       );

  final String catalogId;
  final String displayName;
}

/// Closed picker projection joined from one exact Story and broad archetype
/// catalog generation.
final class Revision3NpcCatalog {
  Revision3NpcCatalog({
    required Iterable<Revision3NpcCatalogChoice> choices,
    this.archetypeIndex,
  }) : choices = _closedChoices(choices) {
    final index = archetypeIndex;
    if (index != null) {
      for (final choice in this.choices) {
        if (index.selectableForCatalogId(choice.catalogId) == null) {
          throw const FormatException(
            'An NPC archetype choice lacks exact catalog linkage.',
          );
        }
      }
    }
  }

  factory Revision3NpcCatalog.fromStoryCatalog(StoryCatalogAdapter adapter) {
    final index = adapter.npcArchetypeIndex;
    if (index == null) {
      throw const FormatException(
        'The broad NPC archetype catalog is unavailable.',
      );
    }
    return Revision3NpcCatalog(
      choices: adapter.npcChoices.map(
        (choice) => Revision3NpcCatalogChoice(
          catalogId: choice.catalogId,
          displayName: choice.displayName,
        ),
      ),
      archetypeIndex: index,
    );
  }

  final List<Revision3NpcCatalogChoice> choices;
  final StoryNpcArchetypeIndex? archetypeIndex;

  bool contains(String catalogId) =>
      choices.any((choice) => choice.catalogId == catalogId);

  Revision3NpcCatalogChoice? choice(String catalogId) {
    for (final choice in choices) {
      if (choice.catalogId == catalogId) return choice;
    }
    return null;
  }
}

/// Rebuilds and fail-closed joins the pinned Story catalog and the broad NPC
/// linkage catalog. Both read-only native scans start together.
final class Revision3NpcCatalogService {
  const Revision3NpcCatalogService(this._ffi);

  final ModFfi _ffi;

  Future<Revision3NpcCatalog> load(String gameRoot) async {
    if (gameRoot.isEmpty) {
      throw const FormatException(
        'A configured game installation is required.',
      );
    }
    final storyFuture = _ffi.authoringStoryCatalogV1BuildAndReadForGameRoot(
      gameRoot: gameRoot,
    );
    final archetypeFuture = _ffi.authoringNpcArchetypeCatalogV1BuildForGameRoot(
      gameRoot: gameRoot,
    );
    final results = await Future.wait<Object>([
      storyFuture,
      archetypeFuture,
    ], eagerError: false);
    final story = results[0] as AuthoringStoryCatalogSelections;
    final archetypes = results[1] as AuthoringNpcArchetypeCatalogBuildResult;
    return Revision3NpcCatalog.fromStoryCatalog(
      StoryCatalogAdapter.fromSelectionsAndArchetypes(story, archetypes),
    );
  }
}

final revision3NpcCatalogLoaderProvider = Provider<Revision3NpcCatalogLoader>(
  (ref) =>
      Revision3NpcCatalogService(ModFfi(ref.read(coreServiceProvider))).load,
);

/// Friendly NPC Draft input. Normal UI supplies no entity ID, module namespace,
/// source path, generated class, runtime class or authored runtime identity.
final class Revision3NpcDraftAuthoringInput {
  Revision3NpcDraftAuthoringInput._({
    required this.parentCatalogId,
    required this.displayName,
  });

  factory Revision3NpcDraftAuthoringInput({
    required String parentCatalogId,
    required String displayName,
  }) => Revision3NpcDraftAuthoringInput._(
    parentCatalogId: _boundedCatalogText(
      parentCatalogId,
      _maxCatalogIdBytes,
      'NPC archetype',
    ),
    displayName: _friendlyNpcName(displayName),
  );

  final String parentCatalogId;
  final String displayName;
}

/// Internal exact-checkpoint plan for the native R3 NPC transaction.
final class Revision3NpcDraftTechnicalPlan {
  const Revision3NpcDraftTechnicalPlan._({
    required this.npcId,
    required this.scriptModuleId,
    required this.displayName,
    required this.intent,
  });

  factory Revision3NpcDraftTechnicalPlan.forCheckpoint({
    required String projectId,
    required int projectRevision,
    required Revision3NpcDraftAuthoringInput input,
  }) {
    if (!_revision3NpcEntityIdPattern.hasMatch(projectId) ||
        projectId == '00000000000000000000000000000000') {
      throw const FormatException('NPC draft has no valid project identity.');
    }
    if (projectRevision < 0 || projectRevision > 0x7fffffffffffffff) {
      throw const FormatException('NPC draft has no valid project revision.');
    }
    final seed = jsonEncode(<String, Object?>{
      'schema': 1,
      'project_id': projectId,
      'project_revision': projectRevision,
      'parent_catalog_id': input.parentCatalogId,
      'display_name': input.displayName,
    });
    final digest = sha256
        .convert(utf8.encode('gore-mod-studio.r3-npc-names-v1\u0000$seed'))
        .toString();
    final suffix = digest.substring(0, 10).toUpperCase();
    final token = _technicalToken(input.displayName);
    final pascalToken = token
        .split('_')
        .map(
          (segment) =>
              '${segment.substring(0, 1).toUpperCase()}${segment.substring(1).toLowerCase()}',
        )
        .join();

    return Revision3NpcDraftTechnicalPlan._(
      npcId: _derivedEntityId('npc', seed),
      scriptModuleId: _derivedEntityId('npc-script-module', seed),
      displayName: input.displayName,
      intent: AuthoringRevision3NpcDraftIntentV1(
        moduleNamespace: 'GoreMods.Npcs.$pascalToken$suffix',
        uniqueName: 'GORE_${token.toUpperCase()}_$suffix',
        parentCatalogId: input.parentCatalogId,
      ),
    );
  }

  final String npcId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringRevision3NpcDraftIntentV1 intent;
}

/// Exact published checkpoint identity returned without generated source or
/// runtime-class details.
final class Revision3NpcDraftPublication {
  Revision3NpcDraftPublication({
    required String projectId,
    required int projectRevision,
    required String npcId,
    required String scriptModuleId,
  }) : projectId = _requiredProjectId(projectId),
       projectRevision = _requiredRevision(projectRevision),
       npcId = _requiredEntityId(npcId, 'NPC'),
       scriptModuleId = _requiredEntityId(scriptModuleId, 'script module');

  final String projectId;
  final int projectRevision;
  final String npcId;
  final String scriptModuleId;
}

final class Revision3NpcDraftRequiresReopenException implements Exception {
  const Revision3NpcDraftRequiresReopenException();

  @override
  String toString() =>
      'The managed project must be reopened before NPC authoring can continue.';
}

final class Revision3NpcDraftStaleCheckpointException implements Exception {
  const Revision3NpcDraftStaleCheckpointException();

  @override
  String toString() =>
      'The NPC wizard must be reopened for the current managed checkpoint.';
}

List<Revision3NpcCatalogChoice> _closedChoices(
  Iterable<Revision3NpcCatalogChoice> source,
) {
  final choices = source.toList(growable: false);
  if (choices.isEmpty || choices.length > 100000) {
    throw const FormatException('NPC archetype choices are unavailable.');
  }
  final ids = <String>{};
  for (final choice in choices) {
    if (!ids.add(choice.catalogId)) {
      throw const FormatException(
        'NPC archetype choices contain a duplicate identity.',
      );
    }
  }
  return List<Revision3NpcCatalogChoice>.unmodifiable(choices);
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

String _friendlyNpcName(String value) {
  final normalized = value.trim();
  if (normalized.isEmpty) {
    throw const FormatException('Character name is required.');
  }
  if (utf8.encode(normalized).length > _maxNpcDisplayNameBytes) {
    throw const FormatException('Character name is too long.');
  }
  if (normalized.runes.any(_isUnicodeControl)) {
    throw const FormatException('Character name contains a control character.');
  }
  return normalized;
}

bool _isUnicodeControl(int rune) =>
    rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

String _technicalToken(String displayName) {
  final output = StringBuffer();
  var previousSeparator = false;
  for (final rune in displayName.runes) {
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
  if (token.isEmpty) token = 'Npc';
  if (RegExp(r'^[0-9]').hasMatch(token)) token = 'Npc_$token';
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
    throw StateError('Deterministic NPC identity derivation failed.');
  }
  return value;
}

String _requiredProjectId(String value) {
  if (!_revision3NpcEntityIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw const FormatException('NPC publication has no valid project ID.');
  }
  return value;
}

int _requiredRevision(int value) {
  if (value < 0 || value > 0x7fffffffffffffff) {
    throw const FormatException('NPC publication has no valid revision.');
  }
  return value;
}

String _requiredEntityId(String value, String context) {
  if (!_revision3NpcEntityIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw FormatException('$context publication identity is invalid.');
  }
  return value;
}
