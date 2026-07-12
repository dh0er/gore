import 'dart:convert';
import 'dart:math';

const int _maxCanonicalUnverifiedStoryObjectBytes = 16 * 1024 * 1024;
const int _maxStoryMutationBytes = 20 * 1024 * 1024;
const int _maxStoryMutationBaseRevision = 0x7ffffffffffffffe;
const int _maxStoryDisplayNameBytes = 256;
const int _maxModuleNamespaceBytes = 255;
const int _maxNpcUniqueNameBytes = 64;
const int _maxJsonDepth = 128;
const int _maxSecureIdAttempts = 32;

final RegExp _entityIdPattern = RegExp(r'^[0-9a-f]{32}$');

/// One bounded canonical JSON object whose game provenance is not verified.
///
/// Canonical syntax is the only guarantee. In particular, this type does not
/// claim that a parent came from a qualified catalog. The catalog adapter must
/// establish that qualification before constructing one. The canonical bytes
/// are embedded directly in a request, so a large fragment is never decoded or
/// encoded again during assembly.
final class CanonicalUnverifiedStoryJsonObject {
  CanonicalUnverifiedStoryJsonObject._(this.canonicalJson, this.utf8Length);

  final String canonicalJson;
  final int utf8Length;

  factory CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(String json) {
    final utf8Length = _boundedUtf8Length(
      json,
      _maxCanonicalUnverifiedStoryObjectBytes,
      'canonical unverified Story JSON object',
      requireNonEmpty: true,
    );
    final Object? decoded;
    try {
      decoded = jsonDecode(json);
    } on FormatException {
      throw const FormatException(
        'canonical unverified Story JSON is not valid JSON',
      );
    }
    if (decoded is! Map) {
      throw const FormatException(
        'canonical unverified Story JSON must be an object',
      );
    }
    _validateDecodedJsonStrings(decoded, 0);
    if (jsonEncode(decoded) != json) {
      throw const FormatException(
        'unverified Story JSON must use canonical JSON encoding',
      );
    }
    return CanonicalUnverifiedStoryJsonObject._(json, utf8Length);
  }
}

/// The two stable IDs allocated for one atomic Draft insertion.
final class StoryDraftEntityIds {
  StoryDraftEntityIds._({required this.draftId, required this.scriptModuleId});

  factory StoryDraftEntityIds({
    required String draftId,
    required String scriptModuleId,
  }) {
    _requireEntityId(draftId, 'draftId');
    _requireEntityId(scriptModuleId, 'scriptModuleId');
    if (draftId == scriptModuleId) {
      throw const FormatException(
        'Draft and generated ScriptModule IDs must differ',
      );
    }
    return StoryDraftEntityIds._(
      draftId: draftId,
      scriptModuleId: scriptModuleId,
    );
  }

  final String draftId;
  final String scriptModuleId;
}

/// Injectable ID seam. Tests and future project-wide allocators can provide a
/// deterministic implementation without weakening the production default.
abstract interface class StoryDraftIdSource {
  StoryDraftEntityIds next();
}

final class SecureStoryDraftIdSource implements StoryDraftIdSource {
  SecureStoryDraftIdSource({Random? random})
    : _random = random ?? Random.secure();

  final Random _random;

  @override
  StoryDraftEntityIds next() {
    final draftId = _nextNonZeroEntityId();
    for (var attempt = 0; attempt < _maxSecureIdAttempts; attempt++) {
      final scriptModuleId = _nextNonZeroEntityId();
      if (scriptModuleId != draftId) {
        return StoryDraftEntityIds(
          draftId: draftId,
          scriptModuleId: scriptModuleId,
        );
      }
    }
    throw StateError(
      'secure Story ID source exhausted distinct-ID retry limit',
    );
  }

  String _nextNonZeroEntityId() {
    for (var attempt = 0; attempt < _maxSecureIdAttempts; attempt++) {
      final out = StringBuffer();
      for (var index = 0; index < 16; index++) {
        final byte = _random.nextInt(256);
        if (byte < 0 || byte >= 256) {
          throw StateError('secure Story ID source returned an invalid byte');
        }
        out.write(byte.toRadixString(16).padLeft(2, '0'));
      }
      final value = out.toString();
      if (!_isZeroEntityId(value)) return value;
    }
    throw StateError('secure Story ID source exhausted non-zero retry limit');
  }
}

/// Values fixed by the exact latest project and the allocated insertion IDs.
final class StoryDraftMutationContext {
  StoryDraftMutationContext({
    required this.projectId,
    required this.revision,
    required this.ids,
  }) {
    _requireEntityId(projectId, 'projectId');
    if (revision < 0 || revision > _maxStoryMutationBaseRevision) {
      throw const FormatException('Story project revision is out of range');
    }
  }

  final String projectId;
  final int revision;
  final StoryDraftEntityIds ids;
}

/// Friendly NPC fields plus canonical, not-yet-qualified parent objects.
final class StoryNpcDraftInput {
  const StoryNpcDraftInput({
    required this.displayName,
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCharacterDefinition,
    required this.parentAiAgentConfig,
    required this.parentSpawnDefinition,
  });

  final String displayName;
  final String moduleNamespace;
  final String uniqueName;
  final CanonicalUnverifiedStoryJsonObject parentCharacterDefinition;
  final CanonicalUnverifiedStoryJsonObject parentAiAgentConfig;
  final CanonicalUnverifiedStoryJsonObject parentSpawnDefinition;
}

/// Injectable pure mutation seam used inside the managed session's serialized
/// derive lane.
abstract interface class StoryDraftMutationJsonBuilder {
  String buildNpc({
    required StoryDraftMutationContext context,
    required StoryNpcDraftInput input,
  });
}

final class ClosedStoryDraftMutationJsonBuilder
    implements StoryDraftMutationJsonBuilder {
  const ClosedStoryDraftMutationJsonBuilder();

  @override
  String buildNpc({
    required StoryDraftMutationContext context,
    required StoryNpcDraftInput input,
  }) => buildNpcStoryDraftMutationJson(context: context, input: input);
}

/// Build the exact closed ABI-1 NPC request without selecting or qualifying a
/// game-specific parent class on the caller's behalf.
String buildNpcStoryDraftMutationJson({
  required StoryDraftMutationContext context,
  required StoryNpcDraftInput input,
}) {
  final segments = <_StoryJsonSegment>[
    _literal('{"expected_project_id":'),
    _encodedJsonString(context.projectId, 32, 'projectId'),
    _literal(',"expected_revision":'),
    _literal(context.revision.toString()),
    _literal(',"draft_id":'),
    _encodedJsonString(context.ids.draftId, 32, 'draftId'),
    _literal(',"script_module_id":'),
    _encodedJsonString(context.ids.scriptModuleId, 32, 'scriptModuleId'),
    _literal(',"display_name":'),
    _encodedJsonString(
      input.displayName,
      _maxStoryDisplayNameBytes,
      'displayName',
    ),
    _literal(',"draft":{"kind":"npc","input":{"module_namespace":'),
    _encodedJsonString(
      input.moduleNamespace,
      _maxModuleNamespaceBytes,
      'moduleNamespace',
    ),
    _literal(',"unique_name":'),
    _encodedJsonString(input.uniqueName, _maxNpcUniqueNameBytes, 'uniqueName'),
    _literal(',"parent_character_definition":'),
    _fragment(input.parentCharacterDefinition),
    _literal(',"parent_ai_agent_config":'),
    _fragment(input.parentAiAgentConfig),
    _literal(',"parent_spawn_definition":'),
    _fragment(input.parentSpawnDefinition),
    _literal('}}}'),
  ];
  return _assembleBoundedMutation(segments);
}

final class _StoryJsonSegment {
  const _StoryJsonSegment(this.text, this.utf8Length);

  final String text;
  final int utf8Length;
}

_StoryJsonSegment _literal(String value) {
  for (var index = 0; index < value.length; index++) {
    if (value.codeUnitAt(index) > 0x7f) {
      throw StateError('Story JSON literal stopped being ASCII');
    }
  }
  return _StoryJsonSegment(value, value.length);
}

_StoryJsonSegment _encodedJsonString(
  String value,
  int maxRawBytes,
  String context,
) {
  _boundedUtf8Length(value, maxRawBytes, context, requireNonEmpty: true);
  final encoded = jsonEncode(value);
  return _StoryJsonSegment(
    encoded,
    _boundedUtf8Length(
      encoded,
      maxRawBytes * 6 + 2,
      '$context JSON encoding',
      requireNonEmpty: true,
    ),
  );
}

_StoryJsonSegment _fragment(CanonicalUnverifiedStoryJsonObject value) =>
    _StoryJsonSegment(value.canonicalJson, value.utf8Length);

String _assembleBoundedMutation(List<_StoryJsonSegment> segments) {
  var totalBytes = 0;
  for (final segment in segments) {
    totalBytes += segment.utf8Length;
    if (totalBytes > _maxStoryMutationBytes) {
      throw const FormatException(
        'Story mutation exceeds its 20971520-byte limit',
      );
    }
  }
  final out = StringBuffer();
  for (final segment in segments) {
    out.write(segment.text);
  }
  final encoded = out.toString();
  final measured = _boundedUtf8Length(
    encoded,
    _maxStoryMutationBytes,
    'Story mutation',
    requireNonEmpty: true,
  );
  if (measured != totalBytes) {
    throw StateError('Story mutation byte preflight disagrees with assembly');
  }
  return encoded;
}

void _validateDecodedJsonStrings(Object? value, int depth) {
  if (depth > _maxJsonDepth) {
    throw const FormatException(
      'canonical unverified Story JSON exceeds nesting limit',
    );
  }
  switch (value) {
    case String string:
      _boundedUtf8Length(
        string,
        _maxCanonicalUnverifiedStoryObjectBytes,
        'canonical unverified Story JSON string',
      );
    case List values:
      for (final item in values) {
        _validateDecodedJsonStrings(item, depth + 1);
      }
    case Map values:
      for (final entry in values.entries) {
        final key = entry.key;
        if (key is! String) {
          throw const FormatException(
            'canonical unverified Story JSON has a non-string key',
          );
        }
        _boundedUtf8Length(
          key,
          _maxCanonicalUnverifiedStoryObjectBytes,
          'canonical unverified Story JSON key',
        );
        _validateDecodedJsonStrings(entry.value, depth + 1);
      }
    case num() || bool() || null:
      return;
    default:
      throw const FormatException(
        'canonical unverified Story JSON contains a non-JSON value',
      );
  }
}

void _requireEntityId(String value, String field) {
  if (!_entityIdPattern.hasMatch(value) || _isZeroEntityId(value)) {
    throw FormatException(
      '$field must be one non-zero lowercase 128-bit entity ID',
    );
  }
}

bool _isZeroEntityId(String value) =>
    value == '00000000000000000000000000000000';

int _boundedUtf8Length(
  String value,
  int maxBytes,
  String context, {
  bool requireNonEmpty = false,
}) {
  if (requireNonEmpty && value.isEmpty) {
    throw FormatException('$context must not be empty');
  }
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    final int encodedLength;
    if (codeUnit <= 0x7f) {
      encodedLength = 1;
    } else if (codeUnit <= 0x7ff) {
      encodedLength = 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException('$context contains a malformed UTF-16 surrogate');
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException('$context contains a malformed UTF-16 surrogate');
      }
      index++;
      encodedLength = 4;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      throw FormatException('$context contains a malformed UTF-16 surrogate');
    } else {
      encodedLength = 3;
    }
    length += encodedLength;
    if (length > maxBytes) {
      throw FormatException('$context exceeds its $maxBytes-byte limit');
    }
  }
  return length;
}
