import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

final class GoreCoreResponseV2 extends Struct {
  external Pointer<Uint8> data;

  @UintPtr()
  external int len;

  external Pointer<Void> handle;
}

typedef _TransportProbeNative = Uint32 Function();
typedef _TransportProbeDart = int Function();
typedef _ExecuteV2Native =
    Uint32 Function(Pointer<Uint8>, UintPtr, Pointer<GoreCoreResponseV2>);
typedef GoreCoreExecuteV2 =
    int Function(Pointer<Uint8>, int, Pointer<GoreCoreResponseV2>);
typedef _FreeV2Native = Void Function(Pointer<Void>);
typedef GoreCoreFreeV2 = void Function(Pointer<Void>);

const goreCoreAbi = 1;
const goreCoreTransportAbiV2 = 2;
const goreCoreTransportMaxRequestBytes = 64 * 1024 * 1024;
const goreCoreTransportMaxResponseBytes = 64 * 1024 * 1024;
const _transportStatusOk = 0;
const _transportStatusInvalidArgument = 1;
const _transportStatusPanic = 2;
const _ffiRequestLimitResponse =
    '{"ok":false,"error":{"code":"FFI_REQUEST_LIMIT",'
    '"message":"native request exceeds the 67108864-byte transport limit"}}';
const _transportInvalidArgumentResponse =
    '{"ok":false,"error":{"code":"CORE_TRANSPORT_INVALID_ARGUMENT",'
    '"message":"native transport rejected its arguments"}}';
const _transportPanicResponse =
    '{"ok":false,"error":{"code":"CORE_TRANSPORT_PANIC",'
    '"message":"native transport caught an internal panic"}}';

/// Commands the current Studio can invoke. A core missing even one is skipped at startup instead
/// of failing later in an editor workflow.
const requiredStudioCoreCommands = <String>[
  'audio_extract',
  'audio_list',
  'authoring_draft_quest_skeleton_v1_generate',
  'authoring_logical_npc_clone_draft_v1_generate',
  'authoring_npc_archetype_catalog_v1_build_for_game_root',
  'authoring_project_check',
  'authoring_project_story_draft_insert_v1',
  'authoring_store_import_ogg',
  'authoring_store_open',
  'authoring_store_open_document',
  'authoring_store_open_head_bytes',
  'authoring_store_open_head_bytes_document',
  'authoring_store_prepare_checkpoint',
  'authoring_store_prepare_document_checkpoint',
  'authoring_store_verify_asset',
  'authoring_story_build_plan_v1_generate',
  'authoring_story_catalog_v1_build',
  'authoring_story_catalog_v1_build_for_game_root',
  'authoring_story_catalog_v1_read',
  'authoring_story_inventory_v1_build',
  'core_info',
  'dataasset_fixed_inspect_v1',
  'find_game',
  'generate_mod',
  'loc_extract',
  'loc_status',
  'mod_build',
  'mod_deploy',
  'mod_undeploy',
  'script_compile',
  'script_emit_module',
  'script_list_modules',
  'texture_extract',
  'texture_index',
  'voice_archive_match_line',
  'voice_ogg_inspect_v1',
];

const _maxCoreInfoResponseBytes = 64 * 1024;
const _maxCoreInfoCommands = 256;
const _maxCoreInfoCommandBytes = 64;
const _maxCoreInfoVersionBytes = 256;
const _coreInfoRequest = '{"command":"core_info","payload":{}}';
final _coreCommandPattern = RegExp(r'^[a-z][a-z0-9_]*$');
final _coreVersionPattern = RegExp(r'^[\x21-\x7e]+$');

/// Parsed result of the cheap, read-only `core_info` compatibility handshake.
class GoreCoreInfo {
  const GoreCoreInfo._({
    required this.abi,
    required this.version,
    required this.commands,
  });

  final int abi;
  final String version;
  final List<String> commands;

  bool get isStudioCompatible =>
      abi == goreCoreAbi && missingRequiredCommands.isEmpty;

  List<String> get missingRequiredCommands => List.unmodifiable(
    requiredStudioCoreCommands.where((command) => !commands.contains(command)),
  );

  /// Strictly parses the stable ABI-1 response. Bounds are deliberately small because this runs
  /// synchronously during startup, before a native library is trusted for normal work.
  factory GoreCoreInfo.parseResponse(String response) {
    if (response.length > _maxCoreInfoResponseBytes ||
        utf8.encode(response).length > _maxCoreInfoResponseBytes) {
      throw const FormatException('core_info response exceeds size limit');
    }

    final Map<String, Object?> decoded;
    try {
      decoded = decodeCanonicalGoreCoreResponse(response);
    } on FormatException {
      throw const FormatException('core_info response is not valid JSON');
    }
    const fields = {'ok', 'abi', 'version', 'commands'};
    if (decoded.length != fields.length || !fields.every(decoded.containsKey)) {
      throw const FormatException('core_info response has an invalid schema');
    }
    if (decoded['ok'] != true) {
      throw const FormatException('core_info command did not succeed');
    }

    final abi = decoded['abi'];
    if (abi is! int || abi <= 0 || abi > 0x7fffffff) {
      throw const FormatException('core_info abi is not a valid integer');
    }
    final version = decoded['version'];
    if (version is! String ||
        version.isEmpty ||
        version.length > _maxCoreInfoVersionBytes ||
        utf8.encode(version).length > _maxCoreInfoVersionBytes ||
        !_coreVersionPattern.hasMatch(version)) {
      throw const FormatException('core_info version is not a valid string');
    }

    final rawCommands = decoded['commands'];
    if (rawCommands is! List ||
        rawCommands.isEmpty ||
        rawCommands.length > _maxCoreInfoCommands) {
      throw const FormatException('core_info commands is not a bounded array');
    }
    final commands = <String>[];
    for (var index = 0; index < rawCommands.length; index++) {
      final command = rawCommands[index];
      if (command is! String ||
          command.isEmpty ||
          command.length > _maxCoreInfoCommandBytes ||
          utf8.encode(command).length > _maxCoreInfoCommandBytes ||
          !_coreCommandPattern.hasMatch(command)) {
        throw FormatException(
          'core_info command at index $index is not canonical',
        );
      }
      if (commands.isNotEmpty && commands.last.compareTo(command) >= 0) {
        throw const FormatException(
          'core_info commands are not sorted and unique',
        );
      }
      commands.add(command);
    }
    return GoreCoreInfo._(
      abi: abi,
      version: version,
      commands: List.unmodifiable(commands),
    );
  }

  /// Pure protocol-compatibility seam used after a transport has been independently accepted.
  /// Candidate loading additionally requires transport v2; this parser alone never authorizes a
  /// native library for feature commands.
  static GoreCoreInfo? tryParseCompatibleResponse(String response) {
    try {
      final info = GoreCoreInfo.parseResponse(response);
      return info.isStudioCompatible ? info : null;
    } on FormatException {
      return null;
    }
  }

  /// A valid protocol response alone must never make a legacy, unbounded transport operational.
  /// The loader uses this seam after resolving and invoking the exact versioned v2 probe.
  static GoreCoreInfo? tryParseCompatibleTransportV2Response(
    int transportAbi,
    String response,
  ) {
    if (transportAbi != goreCoreTransportAbiV2) return null;
    return tryParseCompatibleResponse(response);
  }
}

abstract class GoreCoreFfiService {
  bool get isAvailable;
  String get description;
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  });
}

/// Live implementation — binds gore_ffi.dll via dart:ffi.
class NativeGoreCoreFfiService implements GoreCoreFfiService {
  NativeGoreCoreFfiService._(this.libraryPath, this.coreInfo);

  static NativeGoreCoreFfiService? tryCreate() {
    for (final candidate in _candidateLibraryPaths()) {
      try {
        final lib = DynamicLibrary.open(candidate);
        final probe = lib
            .lookupFunction<_TransportProbeNative, _TransportProbeDart>(
              'gore_core_transport_abi_v2',
            );
        final transportAbi = probe();
        if (transportAbi != goreCoreTransportAbiV2) continue;
        final execute = lib.lookupFunction<_ExecuteV2Native, GoreCoreExecuteV2>(
          'gore_core_execute_v2',
        );
        final free = lib.lookupFunction<_FreeV2Native, GoreCoreFreeV2>(
          'gore_core_response_free_v2',
        );
        final response = executeGoreCoreV2WithBindings(
          execute,
          free,
          _coreInfoRequest,
          responseLimitBytes: _maxCoreInfoResponseBytes,
        );
        final info = GoreCoreInfo.tryParseCompatibleTransportV2Response(
          transportAbi,
          response,
        );
        if (info == null) continue;
        return NativeGoreCoreFfiService._(candidate, info);
      } catch (_) {
        continue;
      }
    }
    return null;
  }

  final String libraryPath;

  final GoreCoreInfo coreInfo;

  @override
  String get description =>
      '$libraryPath (core ${coreInfo.version}, protocol ABI ${coreInfo.abi}, '
      'transport ABI $goreCoreTransportAbiV2)';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final request = jsonEncode({'command': command, 'payload': payload});
    final response = await Isolate.run(
      () => _executeNativeRequest(libraryPath, request),
    );
    return decodeCanonicalGoreCoreResponse(response);
  }
}

/// Decode only the compact, duplicate-preserving-by-roundtrip JSON emitted by gore-ffi.
///
/// Dart's normal JSON decoder is last-key-wins. Re-encoding the insertion-ordered decoded map and
/// demanding byte equality rejects duplicate keys, whitespace, and alternate string spellings
/// before a typed command DTO can accidentally accept a normalized hostile response.
Map<String, Object?> decodeCanonicalGoreCoreResponse(String response) {
  final Object? decoded;
  try {
    decoded = jsonDecode(response);
  } on FormatException {
    throw const FormatException('gore_ffi returned invalid JSON');
  }
  if (decoded is! Map || jsonEncode(decoded) != response) {
    throw const FormatException('gore_ffi returned non-canonical JSON');
  }
  return decoded.cast<String, Object?>();
}

String _executeNativeRequest(String libPath, String request) {
  final lib = DynamicLibrary.open(libPath);
  final probe = lib.lookupFunction<_TransportProbeNative, _TransportProbeDart>(
    'gore_core_transport_abi_v2',
  );
  if (probe() != goreCoreTransportAbiV2) {
    throw const FormatException('gore_ffi transport ABI changed after load');
  }
  final execute = lib.lookupFunction<_ExecuteV2Native, GoreCoreExecuteV2>(
    'gore_core_execute_v2',
  );
  final free = lib.lookupFunction<_FreeV2Native, GoreCoreFreeV2>(
    'gore_core_response_free_v2',
  );
  return executeGoreCoreV2WithBindings(execute, free, request);
}

int? _boundedUtf8Length(String value, int limit) {
  if (limit < 0) return null;
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    final int encodedLength;
    if (codeUnit <= 0x7f) {
      encodedLength = 1;
    } else if (codeUnit <= 0x7ff) {
      encodedLength = 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      final hasLowSurrogate =
          index + 1 < value.length &&
          value.codeUnitAt(index + 1) >= 0xdc00 &&
          value.codeUnitAt(index + 1) <= 0xdfff;
      if (hasLowSurrogate) {
        encodedLength = 4;
        index++;
      } else {
        // Dart's UTF-8 encoder replaces an unpaired UTF-16 surrogate with U+FFFD.
        encodedLength = 3;
      }
    } else {
      // BMP scalars and unpaired low surrogates both encode to three bytes (the latter as U+FFFD).
      encodedLength = 3;
    }
    if (encodedLength > limit - length) return null;
    length += encodedLength;
  }
  return length;
}

/// Executes transport v2 with injected bindings. Public so pointer ownership and malformed native
/// responses can be unit-tested without loading a real DLL; production resolves these exact
/// signatures from the selected candidate.
String executeGoreCoreV2WithBindings(
  GoreCoreExecuteV2 execute,
  GoreCoreFreeV2 free,
  String request, {
  int requestLimitBytes = goreCoreTransportMaxRequestBytes,
  int responseLimitBytes = goreCoreTransportMaxResponseBytes,
}) {
  final effectiveRequestLimit =
      requestLimitBytes > goreCoreTransportMaxRequestBytes
      ? goreCoreTransportMaxRequestBytes
      : requestLimitBytes;
  final effectiveResponseLimit = responseLimitBytes < 0
      ? 0
      : responseLimitBytes > goreCoreTransportMaxResponseBytes
      ? goreCoreTransportMaxResponseBytes
      : responseLimitBytes;
  final boundedRequestLength = _boundedUtf8Length(
    request,
    effectiveRequestLimit,
  );
  if (boundedRequestLength == null) {
    return _ffiRequestLimitResponse;
  }
  final requestBytes = utf8.encode(request);
  if (requestBytes.length != boundedRequestLength) {
    throw const FormatException('Dart UTF-8 length calculation disagreed');
  }
  final out = calloc<GoreCoreResponseV2>();
  if (out == nullptr) {
    throw StateError('failed to allocate native response descriptor');
  }
  Pointer<Uint8> reqPtr = nullptr;
  try {
    if (requestBytes.isNotEmpty) {
      reqPtr = malloc<Uint8>(requestBytes.length);
      if (reqPtr == nullptr) {
        throw StateError('failed to allocate native request buffer');
      }
      reqPtr.asTypedList(requestBytes.length).setAll(0, requestBytes);
    }
    final status = execute(reqPtr, requestBytes.length, out);
    final responseData = out.ref.data;
    final responseLength = out.ref.len;
    final responseHandle = out.ref.handle;

    if (status != _transportStatusOk) {
      if (responseData != nullptr ||
          responseLength != 0 ||
          responseHandle != nullptr) {
        throw const FormatException(
          'gore_ffi returned output with a failed transport status',
        );
      }
      return switch (status) {
        _transportStatusInvalidArgument => _transportInvalidArgumentResponse,
        _transportStatusPanic => _transportPanicResponse,
        _ => throw FormatException(
          'gore_ffi returned unknown transport status $status',
        ),
      };
    }

    if (responseData == nullptr || responseHandle == nullptr) {
      throw const FormatException('gore_ffi returned an incomplete response');
    }
    if (responseLength <= 0 || responseLength > effectiveResponseLimit) {
      throw FormatException(
        'gore_ffi response length $responseLength is outside the bounded range',
      );
    }
    return utf8.decode(
      responseData.asTypedList(responseLength),
      allowMalformed: false,
    );
  } finally {
    try {
      if (reqPtr != nullptr) {
        malloc.free(reqPtr);
      }
    } finally {
      // Read the pre-zeroed descriptor here, not only after a normal execute return: an injected
      // binding may publish ownership and then throw. Keep descriptor cleanup independent even if
      // the native free binding itself fails.
      final responseHandle = out.ref.handle;
      try {
        if (responseHandle != nullptr) {
          free(responseHandle);
        }
      } finally {
        calloc.free(out);
      }
    }
  }
}

/// Stub returned when no candidate exposes the exact bounded transport and compatible protocol.
/// This includes a missing DLL as well as stale legacy-only builds that are intentionally skipped.
class MissingGoreCoreFfiService implements GoreCoreFfiService {
  @override
  bool get isAvailable => false;
  @override
  String get description => 'no compatible bounded-transport core';
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => {
    'ok': false,
    'error': {
      'code': 'CORE_UNAVAILABLE',
      'message': 'no compatible bounded-transport core was found',
    },
  };
}

/// Injectable fake for widget tests — callers supply canned responses.
class FakeGoreCoreFfiService implements GoreCoreFfiService {
  FakeGoreCoreFfiService({required this.responses});
  final Map<String, Map<String, Object?>> responses;
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  bool get isAvailable => true;
  @override
  String get description => 'fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    return responses[command] ??
        {
          'ok': false,
          'error': {
            'code': 'UNKNOWN_COMMAND',
            'message': 'unknown command: $command',
          },
        };
  }
}

GoreCoreFfiService createCoreService() =>
    NativeGoreCoreFfiService.tryCreate() ?? MissingGoreCoreFfiService();

List<String> _candidateLibraryPaths() {
  if (!Platform.isWindows) return const [];
  final candidates = <String>[
    // Release/dev bundle: DLL copied next to the exe (see build.py).
    p.join(p.dirname(Platform.resolvedExecutable), 'gore_ffi.dll'),
  ];
  // Dev: the cargo workspace target/ is at the monorepo root. The runtime cwd
  // depth varies (app dir vs bundle dir), so walk up looking for it rather than
  // hard-coding a level count.
  var dir = Directory.current.path;
  for (var i = 0; i < 6; i++) {
    for (final profile in const ['debug', 'release']) {
      candidates.add(p.join(dir, 'target', profile, 'gore_ffi.dll'));
    }
    final parent = p.dirname(dir);
    if (parent == dir) break;
    dir = parent;
  }
  return candidates;
}
