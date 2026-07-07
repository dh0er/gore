import 'dart:async';
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

  /// One long-lived worker isolate that opens the DLL ONCE and services every
  /// request, instead of `Isolate.run` per call (which spawned a fresh isolate
  /// and re-opened + re-resolved the DLL every time — ~10-20 ms of pure overhead
  /// on top of the native work). Created lazily on first use.
  _CoreWorker? _worker;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final request = jsonEncode({'command': command, 'payload': payload});
    final worker = _worker ??= _CoreWorker(description);
    final response = await worker.send(request);
    final decoded = jsonDecode(response);
    if (decoded is Map) {
      return decoded.cast<String, Object?>();
    }
    throw const FormatException('Native core response was not an object');
  }
}

/// Payload thrown when the worker isolate reports a native failure. Mirrors the
/// old `Isolate.run` behaviour where the error surfaced to the awaiting caller.
class CoreWorkerException implements Exception {
  const CoreWorkerException(this.message);
  final String message;
  @override
  String toString() => message;
}

class _WorkerInit {
  const _WorkerInit(this.sendPort, this.libraryPath);
  final SendPort sendPort;
  final String libraryPath;
}

/// A persistent background isolate that owns the loaded core DLL. The main
/// isolate sends `[id, requestJson]`; the worker replies `[id, ok, payload]`
/// where `payload` is the response JSON (ok) or an error string. Requests are
/// id-correlated so concurrent callers (e.g. a loc lookup racing an inspect)
/// each get their own reply, and the worker processes them one at a time —
/// matching the core's internal Mutex — without blocking the UI isolate.
class _CoreWorker {
  _CoreWorker(this._libraryPath) {
    _readyFuture = _spawn();
  }

  final String _libraryPath;
  final ReceivePort _fromWorker = ReceivePort();
  SendPort? _toWorker;
  late final Future<void> _readyFuture;
  final Map<int, Completer<String>> _pending = {};
  int _nextId = 0;

  Future<void> _spawn() async {
    final ready = Completer<void>();
    _fromWorker.listen((message) {
      if (message is SendPort) {
        _toWorker = message;
        ready.complete();
        return;
      }
      final list = message as List<Object?>;
      final id = list[0] as int;
      final ok = list[1] as bool;
      final pending = _pending.remove(id);
      if (pending == null) return;
      if (ok) {
        pending.complete(list[2] as String);
      } else {
        pending.completeError(CoreWorkerException(list[2] as String));
      }
    });
    await Isolate.spawn(
      _coreWorkerEntry,
      _WorkerInit(_fromWorker.sendPort, _libraryPath),
    );
    return ready.future;
  }

  Future<String> send(String request) async {
    await _readyFuture;
    final id = _nextId++;
    final completer = Completer<String>();
    _pending[id] = completer;
    _toWorker!.send([id, request]);
    return completer.future;
  }
}

/// Worker isolate entry: open the DLL once, then serve requests forever.
void _coreWorkerEntry(_WorkerInit init) {
  final toMain = init.sendPort;
  final fromMain = ReceivePort();
  toMain.send(fromMain.sendPort);

  final library = DynamicLibrary.open(init.libraryPath);
  final execute = library.lookupFunction<_ExecuteNative, _ExecuteDart>(
    'goresave_execute',
  );
  final free = library.lookupFunction<_FreeNative, _FreeDart>('goresave_free');

  fromMain.listen((message) {
    final list = message as List<Object?>;
    final id = list[0] as int;
    final request = list[1] as String;
    try {
      toMain.send([id, true, _invokeNative(execute, free, request)]);
    } catch (error) {
      toMain.send([id, false, error.toString()]);
    }
  });
}

/// Single native round-trip against an already-open library.
String _invokeNative(_ExecuteDart execute, _FreeDart free, String request) {
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

/// Generic "run this off the UI isolate" helper. Kept for one-shot background
/// work (and covered by a test); the core itself now uses the persistent
/// [_CoreWorker] above rather than a fresh isolate per call.
Future<T> runCoreWorkOnBackgroundIsolate<T>(T Function() work) {
  return Isolate.run(work);
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
