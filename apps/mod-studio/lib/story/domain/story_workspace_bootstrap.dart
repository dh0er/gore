import 'dart:convert';
import 'dart:io';
import 'dart:math';

import '../../core/mod_ffi.dart';
import '../../project/managed_project_session.dart';
import 'story_catalog_adapter.dart';
import 'story_workspace_controller.dart';

const int _maxProjectNameBytes = 256;
const int _maxProjectVersionBytes = 128;
const int _maxProjectAuthorBytes = 256;
const int _maxAuthoringLocales = 64;
const int _maxProjectIdAttempts = 32;

final RegExp _projectIdPattern = RegExp(r'^[0-9a-f]{32}$');
final RegExp _sha256Pattern = RegExp(r'^[0-9a-f]{64}$');

final class StoryWorkspaceBootstrapException implements Exception {
  const StoryWorkspaceBootstrapException(this.message);

  final String message;

  @override
  String toString() => 'StoryWorkspaceBootstrapException: $message';
}

/// Closed, bounded metadata for one newly-created Story workspace.
final class StoryProjectMetadata {
  StoryProjectMetadata._({
    required this.name,
    required this.version,
    required this.author,
    required this.authoringLocales,
  });

  factory StoryProjectMetadata({
    required String name,
    required String version,
    required String author,
    Iterable<String> authoringLocales = const <String>[],
  }) {
    _boundedVisibleText(
      name,
      _maxProjectNameBytes,
      'project name',
      requireNonEmpty: true,
    );
    _boundedVisibleText(version, _maxProjectVersionBytes, 'project version');
    _boundedVisibleText(author, _maxProjectAuthorBytes, 'project author');
    final locales = <String>[];
    final seen = <String>{};
    for (final locale in authoringLocales) {
      if (locales.length >= _maxAuthoringLocales) {
        throw const FormatException(
          'authoring locale count exceeds its 64-entry limit',
        );
      }
      _requireCanonicalLocale(locale);
      if (!seen.add(locale)) {
        throw FormatException('duplicate authoring locale: $locale');
      }
      locales.add(locale);
    }
    locales.sort();
    return StoryProjectMetadata._(
      name: name,
      version: version,
      author: author,
      authoringLocales: List<String>.unmodifiable(locales),
    );
  }

  final String name;
  final String version;
  final String author;
  final List<String> authoringLocales;
}

abstract interface class StoryProjectIdSource {
  String nextProjectId();
}

/// Secure default ProjectId allocator with a finite adversarial-RNG budget.
final class SecureStoryProjectIdSource implements StoryProjectIdSource {
  SecureStoryProjectIdSource({Random? random})
    : _random = random ?? Random.secure();

  final Random _random;

  @override
  String nextProjectId() {
    for (var attempt = 0; attempt < _maxProjectIdAttempts; attempt++) {
      final out = StringBuffer();
      for (var index = 0; index < 16; index++) {
        final byte = _random.nextInt(256);
        if (byte < 0 || byte >= 256) {
          throw StateError('secure ProjectId source returned an invalid byte');
        }
        out.write(byte.toRadixString(16).padLeft(2, '0'));
      }
      final value = out.toString();
      if (!_isZeroProjectId(value)) return value;
    }
    throw StateError('secure ProjectId source exhausted non-zero retry limit');
  }
}

final class StoryWorkspaceHandle {
  const StoryWorkspaceHandle._({
    required this.session,
    required this.controller,
    required this.adapter,
  });

  final ManagedAuthoringProjectSession session;
  final StoryWorkspaceController controller;
  final StoryCatalogAdapter adapter;

  bool get isClosed => session.isClosed;

  Future<void> close() => session.close();
}

/// Creates or opens one managed schema-revision-2 Story workspace without any
/// picker, legacy project state, deployment, or runtime coupling.
abstract final class StoryWorkspaceBootstrap {
  static Future<StoryWorkspaceHandle> create({
    required Directory root,
    required ModFfi ffi,
    required AuthoringStoryCatalogSelections catalogSelections,
    required AuthoringValidationProfile profile,
    required StoryProjectMetadata metadata,
    StoryProjectIdSource? projectIdSource,
  }) async {
    final adapter = StoryCatalogAdapter.fromSelections(catalogSelections);
    final projectId = (projectIdSource ?? SecureStoryProjectIdSource())
        .nextProjectId();
    _requireProjectId(projectId);
    final projectJson = _emptyProjectJson(
      projectId: projectId,
      metadata: metadata,
      executable: catalogSelections.generation.executable,
    );
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: ModFfiManagedAuthoringStore(ffi),
      projectJson: projectJson,
      profile: profile,
    );
    try {
      final controller = StoryWorkspaceController(session: session, ffi: ffi);
      final state = controller.current;
      if (session.projectJson != projectJson ||
          state.revision != 0 ||
          state.drafts.isNotEmpty) {
        throw const StoryWorkspaceBootstrapException(
          'new Story workspace did not reopen as its exact empty document',
        );
      }
      _requireExactExecutableTarget(
        session.projectJson,
        catalogSelections.generation.executable,
      );
      return StoryWorkspaceHandle._(
        session: session,
        controller: controller,
        adapter: adapter,
      );
    } catch (error, stackTrace) {
      try {
        await session.close();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  static Future<StoryWorkspaceHandle> open({
    required Directory root,
    required ModFfi ffi,
    required AuthoringStoryCatalogSelections catalogSelections,
    required AuthoringValidationProfile profile,
  }) async {
    final adapter = StoryCatalogAdapter.fromSelections(catalogSelections);
    final session = await ManagedAuthoringProjectSession.open(
      root: root,
      store: ModFfiManagedAuthoringStore(ffi),
      profile: profile,
    );
    try {
      final controller = StoryWorkspaceController(session: session, ffi: ffi);
      controller.current;
      _requireExactExecutableTarget(
        session.projectJson,
        catalogSelections.generation.executable,
      );
      return StoryWorkspaceHandle._(
        session: session,
        controller: controller,
        adapter: adapter,
      );
    } catch (error, stackTrace) {
      try {
        await session.close();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }
}

String _emptyProjectJson({
  required String projectId,
  required StoryProjectMetadata metadata,
  required AuthoringDraftContentSeal executable,
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 2,
  'project_id': projectId,
  'revision': 0,
  'meta': <String, Object?>{
    'name': metadata.name,
    'version': metadata.version,
    'author': metadata.author,
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': executable.byteLength,
      'sha256': executable.sha256,
    },
  },
  'authoring_locales': metadata.authoringLocales,
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

void _requireExactExecutableTarget(
  String projectJson,
  AuthoringDraftContentSeal expected,
) {
  final Object? decoded;
  try {
    decoded = jsonDecode(projectJson);
  } on FormatException {
    throw const StoryWorkspaceBootstrapException(
      'managed Story project is not JSON',
    );
  }
  if (decoded is! Map) {
    throw const StoryWorkspaceBootstrapException(
      'managed Story project is not an object',
    );
  }
  final project = decoded.cast<String, Object?>();
  final target = _exactObject(project['target'], const <String>{
    'executable',
  }, 'target');
  final executable = _exactObject(target['executable'], const <String>{
    'byte_len',
    'sha256',
  }, 'target executable');
  final byteLength = executable['byte_len'];
  final sha256 = executable['sha256'];
  if (byteLength is! int ||
      byteLength <= 0 ||
      sha256 is! String ||
      !_sha256Pattern.hasMatch(sha256) ||
      byteLength != expected.byteLength ||
      sha256 != expected.sha256) {
    throw const StoryWorkspaceBootstrapException(
      'Story project target executable differs from the verified catalog generation',
    );
  }
}

Map<String, Object?> _exactObject(
  Object? value,
  Set<String> fields,
  String context,
) {
  if (value is! Map) {
    throw StoryWorkspaceBootstrapException('$context must be an object');
  }
  final object = value.cast<String, Object?>();
  if (object.length != fields.length || !fields.every(object.containsKey)) {
    throw StoryWorkspaceBootstrapException('$context has an invalid schema');
  }
  return object;
}

void _requireProjectId(String value) {
  if (!_projectIdPattern.hasMatch(value) || _isZeroProjectId(value)) {
    throw const FormatException(
      'ProjectId must be one non-zero lowercase 128-bit identifier',
    );
  }
}

bool _isZeroProjectId(String value) =>
    value == '00000000000000000000000000000000';

void _boundedVisibleText(
  String value,
  int maxBytes,
  String context, {
  bool requireNonEmpty = false,
}) {
  _boundedUtf8(value, maxBytes, context, requireNonEmpty: requireNonEmpty);
  for (final rune in value.runes) {
    if (rune < 0x20 || (rune >= 0x7f && rune <= 0x9f)) {
      throw FormatException('$context contains a control character');
    }
  }
}

void _requireCanonicalLocale(String value) {
  _boundedUtf8(value, 35, 'authoring locale', requireNonEmpty: true);
  for (var index = 0; index < value.length; index++) {
    if (value.codeUnitAt(index) > 0x7f) {
      throw FormatException('authoring locale is not ASCII: $value');
    }
  }
  final segments = value.split('-');
  final language = segments.first;
  if (language.length < 2 || language.length > 8 || !_allAsciiLower(language)) {
    throw FormatException('authoring locale has an invalid language: $value');
  }
  final canonical = StringBuffer(language);
  for (var index = 1; index < segments.length; index++) {
    final segment = segments[index];
    if (segment.isEmpty ||
        segment.length > 8 ||
        !_allAsciiAlphanumeric(segment)) {
      throw FormatException('authoring locale has an invalid segment: $value');
    }
    canonical.write('-');
    if (segment.length == 4 && _allAsciiAlphabetic(segment)) {
      canonical.write(
        '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}',
      );
    } else if (segment.length == 2 && _allAsciiAlphabetic(segment)) {
      canonical.write(segment.toUpperCase());
    } else {
      canonical.write(segment.toLowerCase());
    }
  }
  if (canonical.toString() != value) {
    throw FormatException('authoring locale is not canonical: $value');
  }
}

bool _allAsciiLower(String value) =>
    value.codeUnits.every((unit) => unit >= 0x61 && unit <= 0x7a);

bool _allAsciiAlphabetic(String value) => value.codeUnits.every(
  (unit) => (unit >= 0x41 && unit <= 0x5a) || (unit >= 0x61 && unit <= 0x7a),
);

bool _allAsciiAlphanumeric(String value) => value.codeUnits.every(
  (unit) =>
      (unit >= 0x30 && unit <= 0x39) ||
      (unit >= 0x41 && unit <= 0x5a) ||
      (unit >= 0x61 && unit <= 0x7a),
);

int _boundedUtf8(
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
    final unit = value.codeUnitAt(index);
    final int width;
    if (unit <= 0x7f) {
      width = 1;
    } else if (unit <= 0x7ff) {
      width = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException('$context contains malformed UTF-16');
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException('$context contains malformed UTF-16');
      }
      index++;
      width = 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw FormatException('$context contains malformed UTF-16');
    } else {
      width = 3;
    }
    length += width;
    if (length > maxBytes) {
      throw FormatException('$context exceeds its $maxBytes-byte limit');
    }
  }
  return length;
}
