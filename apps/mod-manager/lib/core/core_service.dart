import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

final class _GoreCoreResponseV2 extends Struct {
  external Pointer<Uint8> data;

  @UintPtr()
  external int len;

  external Pointer<Void> handle;
}

typedef _TransportProbeNative = Uint32 Function();
typedef _TransportProbeDart = int Function();
typedef _ExecuteV2Native =
    Uint32 Function(Pointer<Uint8>, UintPtr, Pointer<_GoreCoreResponseV2>);
typedef _ExecuteV2Dart =
    int Function(Pointer<Uint8>, int, Pointer<_GoreCoreResponseV2>);
typedef _FreeV2Native = Void Function(Pointer<Void>);
typedef _FreeV2Dart = void Function(Pointer<Void>);

const _transportAbiV2 = 2;
const _protocolAbi = 1;
const _maxRequestBytes = 64 * 1024 * 1024;
const _maxResponseBytes = 64 * 1024 * 1024;
const _maxCoreInfoBytes = 64 * 1024;
const _transportStatusOk = 0;
const _transportStatusInvalidArgument = 1;
const _transportStatusPanic = 2;
const _coreInfoRequest = '{"command":"core_info","payload":{}}';
const _requiredManagerCommands = <String>{
  'core_info',
  'mgr_analyze',
  'mgr_apply',
  'mgr_import',
  'mgr_library_list',
  'mgr_remove',
  'mgr_set_loadout',
  'mgr_status',
  'mgr_undeploy_all',
};

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
  NativeGoreCoreFfiService._(this.description);

  static NativeGoreCoreFfiService? tryCreate() {
    for (final candidate in _candidateLibraryPaths()) {
      try {
        final lib = DynamicLibrary.open(candidate);
        final probe = lib
            .lookupFunction<_TransportProbeNative, _TransportProbeDart>(
              'gore_core_transport_abi_v2',
            );
        if (probe() != _transportAbiV2) continue;
        final execute = lib.lookupFunction<_ExecuteV2Native, _ExecuteV2Dart>(
          'gore_core_execute_v2',
        );
        final free = lib.lookupFunction<_FreeV2Native, _FreeV2Dart>(
          'gore_core_response_free_v2',
        );
        final coreInfo = _executeV2(
          execute,
          free,
          _coreInfoRequest,
          responseLimit: _maxCoreInfoBytes,
        );
        if (!_isCurrentCoreInfo(coreInfo)) continue;
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
    throw const FormatException('gore_ffi returned a non-object response');
  }
}

String _executeNativeRequest(String libPath, String request) {
  final lib = DynamicLibrary.open(libPath);
  final probe = lib.lookupFunction<_TransportProbeNative, _TransportProbeDart>(
    'gore_core_transport_abi_v2',
  );
  if (probe() != _transportAbiV2) {
    throw const FormatException('gore_ffi transport ABI changed after load');
  }
  final execute = lib.lookupFunction<_ExecuteV2Native, _ExecuteV2Dart>(
    'gore_core_execute_v2',
  );
  final free = lib.lookupFunction<_FreeV2Native, _FreeV2Dart>(
    'gore_core_response_free_v2',
  );
  return _executeV2(execute, free, request);
}

String _executeV2(
  _ExecuteV2Dart execute,
  _FreeV2Dart free,
  String request, {
  int responseLimit = _maxResponseBytes,
}) {
  final requestBytes = utf8.encode(request);
  if (requestBytes.length > _maxRequestBytes) {
    return '{"ok":false,"error":{"code":"FFI_REQUEST_LIMIT",'
        '"message":"native request exceeds the 67108864-byte transport limit"}}';
  }
  final out = calloc<_GoreCoreResponseV2>();
  if (out == nullptr) {
    throw StateError('failed to allocate native response descriptor');
  }
  Pointer<Uint8> requestPointer = nullptr;
  try {
    if (requestBytes.isNotEmpty) {
      requestPointer = malloc<Uint8>(requestBytes.length);
      if (requestPointer == nullptr) {
        throw StateError('failed to allocate native request buffer');
      }
      requestPointer.asTypedList(requestBytes.length).setAll(0, requestBytes);
    }
    final status = execute(requestPointer, requestBytes.length, out);
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
        _transportStatusInvalidArgument =>
          '{"ok":false,"error":{"code":"CORE_TRANSPORT_INVALID_ARGUMENT",'
              '"message":"native transport rejected its arguments"}}',
        _transportStatusPanic =>
          '{"ok":false,"error":{"code":"CORE_TRANSPORT_PANIC",'
              '"message":"native transport caught an internal panic"}}',
        _ => throw FormatException(
          'gore_ffi returned unknown transport status $status',
        ),
      };
    }
    if (responseData == nullptr || responseHandle == nullptr) {
      throw const FormatException('gore_ffi returned an incomplete response');
    }
    if (responseLength <= 0 || responseLength > responseLimit) {
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
      if (requestPointer != nullptr) malloc.free(requestPointer);
    } finally {
      final responseHandle = out.ref.handle;
      try {
        if (responseHandle != nullptr) free(responseHandle);
      } finally {
        calloc.free(out);
      }
    }
  }
}

bool _isCurrentCoreInfo(String response) {
  final Object? decoded;
  try {
    decoded = jsonDecode(response);
  } on FormatException {
    return false;
  }
  if (decoded is! Map ||
      decoded['ok'] != true ||
      decoded['abi'] != _protocolAbi) {
    return false;
  }
  final commands = decoded['commands'];
  if (commands is! List || commands.any((command) => command is! String)) {
    return false;
  }
  final advertised = commands.cast<String>().toSet();
  return _requiredManagerCommands.every(advertised.contains);
}

/// Stub returned when no current bounded gore_ffi.dll is available.
class MissingGoreCoreFfiService implements GoreCoreFfiService {
  @override
  bool get isAvailable => false;
  @override
  String get description => 'current bounded gore_ffi.dll not available';
  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => {
    'ok': false,
    'error': {
      'code': 'CORE_UNAVAILABLE',
      'message': 'current bounded gore_ffi.dll not available',
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
          'error': {'message': 'unknown command'},
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
