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

abstract class GoresaveCoreService {
  bool get isAvailable;
  String get description;

  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  });
}

class NativeGoresaveCoreService implements GoresaveCoreService {
  NativeGoresaveCoreService._(this.description);

  static NativeGoresaveCoreService? tryCreate() {
    for (final candidate in _candidateLibraryPaths()) {
      try {
        final library = DynamicLibrary.open(candidate);
        library.lookupFunction<_ExecuteNative, _ExecuteDart>(
          'goresave_execute',
        );
        library.lookupFunction<_FreeNative, _FreeDart>('goresave_free');
        return NativeGoresaveCoreService._(candidate);
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
    final response = await runCoreWorkOnBackgroundIsolate(
      () => _executeNativeRequest(description, request),
    );
    final decoded = jsonDecode(response);
    if (decoded is Map) {
      return decoded.cast<String, Object?>();
    }
    throw const FormatException('Native core response was not an object');
  }
}

Future<T> runCoreWorkOnBackgroundIsolate<T>(T Function() work) {
  return Isolate.run(work);
}

String _executeNativeRequest(String libraryPath, String request) {
  final library = DynamicLibrary.open(libraryPath);
  final execute = library.lookupFunction<_ExecuteNative, _ExecuteDart>(
    'goresave_execute',
  );
  final free = library.lookupFunction<_FreeNative, _FreeDart>('goresave_free');

  final requestPtr = request.toNativeUtf8();
  Pointer<Utf8> responsePtr = nullptr;
  try {
    responsePtr = execute(requestPtr);
    if (responsePtr == nullptr) {
      throw const FormatException('Native core returned a null response');
    }
    return responsePtr.toDartString();
  } finally {
    malloc.free(requestPtr);
    if (responsePtr != nullptr) {
      free(responsePtr);
    }
  }
}

class MissingGoresaveCoreService implements GoresaveCoreService {
  @override
  bool get isAvailable => false;

  @override
  String get description => 'gore_save.dll not loaded';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    return {
      'ok': false,
      'error': {
        'code': 'CORE_UNAVAILABLE',
        'message':
            'The native gore_save.dll is not available. Build crates/gore-save first.',
      },
    };
  }
}

List<String> _candidateLibraryPaths() {
  if (!Platform.isWindows) {
    return const [];
  }

  final candidates = <String>[
    // Trusted shipped location first; always a path with separators so the
    // Windows DLL search order is bypassed. A bare "gore_save.dll" is
    // intentionally omitted because it would let a same-named DLL on the
    // process search path bind instead of the core we ship.
    p.join(p.dirname(Platform.resolvedExecutable), 'gore_save.dll'),
  ];
  // Dev: the cargo workspace target/ is at the monorepo root. The runtime cwd
  // depth varies (app dir vs a subfolder like integration_test), so walk up
  // looking for it rather than hard-coding a level count.
  var dir = Directory.current.path;
  for (var i = 0; i < 6; i++) {
    for (final profile in const ['debug', 'release']) {
      candidates.add(p.join(dir, 'target', profile, 'gore_save.dll'));
    }
    final parent = p.dirname(dir);
    if (parent == dir) break;
    dir = parent;
  }
  return candidates;
}
