import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

typedef _ExecuteNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _ExecuteDart  = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative   = Void Function(Pointer<Utf8>);
typedef _FreeDart     = void Function(Pointer<Utf8>);

abstract class GoreCoreFfiService {
  bool get isAvailable;
  String get description;
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  });
}

/// Live implementation — binds gore_core.dll via dart:ffi.
class NativeGoreCoreFfiService implements GoreCoreFfiService {
  NativeGoreCoreFfiService._(this.description);

  static NativeGoreCoreFfiService? tryCreate() {
    for (final candidate in _candidateLibraryPaths()) {
      try {
        final lib = DynamicLibrary.open(candidate);
        lib.lookupFunction<_ExecuteNative, _ExecuteDart>('gore_core_execute');
        lib.lookupFunction<_FreeNative, _FreeDart>('gore_core_free');
        return NativeGoreCoreFfiService._(candidate);
      } catch (_) {
        continue;
      }
    }
    return null;
  }

  @override
  final String description;

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final request = jsonEncode({'command': command, 'payload': payload});
    final response = await Isolate.run(
      () => _executeNativeRequest(description, request),
    );
    final decoded = jsonDecode(response);
    if (decoded is Map) return decoded.cast<String, Object?>();
    throw const FormatException('gore_core returned a non-object response');
  }
}

String _executeNativeRequest(String libPath, String request) {
  final lib = DynamicLibrary.open(libPath);
  final execute = lib.lookupFunction<_ExecuteNative, _ExecuteDart>('gore_core_execute');
  final free    = lib.lookupFunction<_FreeNative, _FreeDart>('gore_core_free');
  final reqPtr  = request.toNativeUtf8();
  Pointer<Utf8> resPtr = nullptr;
  try {
    resPtr = execute(reqPtr);
    if (resPtr == nullptr) throw const FormatException('gore_core returned null');
    return resPtr.toDartString();
  } finally {
    malloc.free(reqPtr);
    if (resPtr != nullptr) free(resPtr);
  }
}

/// Stub returned when gore_core.dll is not found (dev / CI without the DLL).
class MissingGoreCoreFfiService implements GoreCoreFfiService {
  @override
  bool get isAvailable => false;
  @override
  String get description => 'gore_core.dll not loaded';
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => {
    'ok': false,
    'error': {'code': 'CORE_UNAVAILABLE', 'message': 'gore_core.dll not found'},
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
    return responses[command] ?? {'ok': false, 'error': {'message': 'unknown command'}};
  }
}

GoreCoreFfiService createCoreService() =>
    NativeGoreCoreFfiService.tryCreate() ?? MissingGoreCoreFfiService();

List<String> _candidateLibraryPaths() {
  if (!Platform.isWindows) return const [];
  final execDir = p.dirname(Platform.resolvedExecutable);
  final cwd     = Directory.current.path;
  return [
    p.join(execDir, 'gore_core.dll'),
    p.normalize(p.join(cwd, '..', '..', '..', '..', 'target', 'debug',   'gore_core.dll')),
    p.normalize(p.join(cwd, '..', '..', '..', '..', 'target', 'release', 'gore_core.dll')),
  ];
}
