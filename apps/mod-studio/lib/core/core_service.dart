import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

typedef _ExecuteNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _ExecuteDart = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

const goreCoreAbi = 1;

/// Commands the current Studio can invoke. A core missing even one is skipped at startup instead
/// of failing later in an editor workflow.
const requiredStudioCoreCommands = <String>[
  'audio_extract',
  'audio_list',
  'authoring_project_check',
  'core_info',
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

    final Object? decoded;
    try {
      decoded = jsonDecode(response);
    } on FormatException {
      throw const FormatException('core_info response is not valid JSON');
    }
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException('core_info response is not an object');
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

  /// Pure candidate-decision seam used by startup and unit tests. Legacy cores answer
  /// `UNKNOWN_COMMAND`, which intentionally parses as incompatible rather than being executed
  /// through a costly feature command.
  static GoreCoreInfo? tryParseCompatibleResponse(String response) {
    try {
      final info = GoreCoreInfo.parseResponse(response);
      return info.isStudioCompatible ? info : null;
    } on FormatException {
      return null;
    }
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
        final execute = lib.lookupFunction<_ExecuteNative, _ExecuteDart>(
          'gore_core_execute',
        );
        final free = lib.lookupFunction<_FreeNative, _FreeDart>(
          'gore_core_free',
        );
        final response = _executeNativeRequestWithBindings(
          execute,
          free,
          _coreInfoRequest,
        );
        final info = GoreCoreInfo.tryParseCompatibleResponse(response);
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
      '$libraryPath (core ${coreInfo.version}, ABI ${coreInfo.abi})';

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
    final decoded = jsonDecode(response);
    if (decoded is Map) return decoded.cast<String, Object?>();
    throw const FormatException('gore_ffi returned a non-object response');
  }
}

String _executeNativeRequest(String libPath, String request) {
  final lib = DynamicLibrary.open(libPath);
  final execute = lib.lookupFunction<_ExecuteNative, _ExecuteDart>(
    'gore_core_execute',
  );
  final free = lib.lookupFunction<_FreeNative, _FreeDart>('gore_core_free');
  return _executeNativeRequestWithBindings(execute, free, request);
}

String _executeNativeRequestWithBindings(
  _ExecuteDart execute,
  _FreeDart free,
  String request,
) {
  final reqPtr = request.toNativeUtf8();
  Pointer<Utf8> resPtr = nullptr;
  try {
    resPtr = execute(reqPtr);
    if (resPtr == nullptr) {
      throw const FormatException('gore_ffi returned null');
    }
    return resPtr.toDartString();
  } finally {
    malloc.free(reqPtr);
    if (resPtr != nullptr) {
      free(resPtr);
    }
  }
}

/// Stub returned when gore_ffi.dll is not found (dev / CI without the DLL).
class MissingGoreCoreFfiService implements GoreCoreFfiService {
  @override
  bool get isAvailable => false;
  @override
  String get description => 'gore_ffi.dll not loaded';
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => {
    'ok': false,
    'error': {'code': 'CORE_UNAVAILABLE', 'message': 'gore_ffi.dll not found'},
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
