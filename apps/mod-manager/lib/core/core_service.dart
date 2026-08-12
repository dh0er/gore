import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

import 'diagnostic_text.dart';

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
const _protocolAbi = 2;
const _maxRequestBytes = 64 * 1024 * 1024;
const _maxResponseBytes = 64 * 1024 * 1024;
const _maxCoreInfoBytes = 64 * 1024;
const _transportStatusOk = 0;
const _transportStatusInvalidArgument = 1;
const _transportStatusPanic = 2;
const _coreInfoRequest = '{"command":"core_info","payload":{}}';
const managerRequiredCoreCommands = <String>[
  'core_info',
  'mgr_analyze',
  'mgr_apply',
  'mgr_import',
  'mgr_library_list',
  'mgr_preflight_v1',
  'mgr_remove',
  'mgr_set_loadout',
  'mgr_status',
  'mgr_undeploy_all',
];

enum CoreBootstrapFailureReason {
  dllMissing,
  dllLoadFailed,
  transportAbiMismatch,
  coreInfoInvalid,
  protocolAbiMismatch,
  requiredCommandsMissing,
}

enum CoreCompatibilityDirection { managerTooOld, coreTooOld }

extension on CoreBootstrapFailureReason {
  String get wireCode => switch (this) {
    CoreBootstrapFailureReason.dllMissing => 'core_library_missing',
    CoreBootstrapFailureReason.dllLoadFailed => 'core_library_load_failed',
    CoreBootstrapFailureReason.transportAbiMismatch =>
      'core_transport_abi_mismatch',
    CoreBootstrapFailureReason.coreInfoInvalid => 'core_info_invalid',
    CoreBootstrapFailureReason.protocolAbiMismatch =>
      'core_protocol_abi_mismatch',
    CoreBootstrapFailureReason.requiredCommandsMissing =>
      'core_required_commands_missing',
  };
}

sealed class CoreBootstrapState {
  const CoreBootstrapState();
}

final class CoreBootstrapReady extends CoreBootstrapState {
  const CoreBootstrapReady({required this.libraryPath, this.coreVersion});

  final String libraryPath;
  final String? coreVersion;
}

final class CoreBootstrapBlocked extends CoreBootstrapState {
  const CoreBootstrapBlocked(this.failure);

  final CoreBootstrapFailure failure;
}

final class CoreBootstrapFailure {
  factory CoreBootstrapFailure({
    required CoreBootstrapFailureReason reason,
    String? candidatePath,
    int? observedTransportAbi,
    int? observedProtocolAbi,
    String? coreVersion,
    Iterable<String> missingCommands = const [],
    String? detail,
  }) {
    final boundedPath = boundedDiagnosticText(candidatePath, 512);
    final boundedVersion = boundedDiagnosticText(coreVersion, 128);
    final boundedDetail = boundedDiagnosticText(detail, 512);
    final missing =
        missingCommands
            .where(managerRequiredCoreCommands.contains)
            .toSet()
            .toList()
          ..sort();
    return CoreBootstrapFailure._(
      reason: reason,
      candidatePath: boundedPath.value,
      candidatePathTruncated: boundedPath.truncated,
      observedTransportAbi: observedTransportAbi,
      observedProtocolAbi: observedProtocolAbi,
      coreVersion: boundedVersion.value,
      coreVersionTruncated: boundedVersion.truncated,
      missingCommands: List.unmodifiable(missing),
      detail: boundedDetail.value,
      detailTruncated: boundedDetail.truncated,
    );
  }

  const CoreBootstrapFailure._({
    required this.reason,
    required this.candidatePath,
    required this.candidatePathTruncated,
    required this.observedTransportAbi,
    required this.observedProtocolAbi,
    required this.coreVersion,
    required this.coreVersionTruncated,
    required this.missingCommands,
    required this.detail,
    required this.detailTruncated,
  });

  final CoreBootstrapFailureReason reason;
  final String? candidatePath;
  final bool candidatePathTruncated;
  final int? observedTransportAbi;
  final int? observedProtocolAbi;
  final String? coreVersion;
  final bool coreVersionTruncated;
  final List<String> missingCommands;
  final String? detail;
  final bool detailTruncated;

  String technicalReport({String? managerVersion}) {
    final boundedManagerVersion = boundedDiagnosticText(managerVersion, 128);
    final report = <String, Object?>{
      'schema': 'gore-manager-core-bootstrap-v1',
      'reason': reason.wireCode,
      'manager_version': ?boundedManagerVersion.value,
      if (boundedManagerVersion.truncated) 'manager_version_truncated': true,
      'expected_transport_abi': _transportAbiV2,
      'observed_transport_abi': ?observedTransportAbi,
      'expected_protocol_abi': _protocolAbi,
      'observed_protocol_abi': ?observedProtocolAbi,
      if (compatibilityDirection case final direction?)
        'compatibility_direction': switch (direction) {
          CoreCompatibilityDirection.managerTooOld => 'manager_too_old',
          CoreCompatibilityDirection.coreTooOld => 'core_too_old',
        },
      'required_commands': managerRequiredCoreCommands,
      if (missingCommands.isNotEmpty) 'missing_commands': missingCommands,
      'dll_path': ?candidatePath,
      if (candidatePathTruncated) 'dll_path_truncated': true,
      'core_version': ?coreVersion,
      if (coreVersionTruncated) 'core_version_truncated': true,
      'detail': ?detail,
      if (detailTruncated) 'detail_truncated': true,
    };
    var encoded = jsonEncode(report);
    if (utf8.encode(encoded).length > 8 * 1024) {
      report
        ..remove('detail')
        ..remove('detail_truncated')
        ..remove('dll_path')
        ..remove('dll_path_truncated')
        ..['oversized_details_omitted'] = true;
      encoded = jsonEncode(report);
    }
    if (utf8.encode(encoded).length > 8 * 1024) {
      report
        ..clear()
        ..addAll({
          'schema': 'gore-manager-core-bootstrap-v1',
          'reason': reason.wireCode,
          'oversized_details_omitted': true,
        });
      encoded = jsonEncode(report);
    }
    return encoded;
  }

  CoreCompatibilityDirection? get compatibilityDirection {
    final observed = switch (reason) {
      CoreBootstrapFailureReason.transportAbiMismatch => observedTransportAbi,
      CoreBootstrapFailureReason.protocolAbiMismatch => observedProtocolAbi,
      _ => null,
    };
    final expected = reason == CoreBootstrapFailureReason.transportAbiMismatch
        ? _transportAbiV2
        : _protocolAbi;
    if (observed == null || observed == expected) return null;
    return observed > expected
        ? CoreCompatibilityDirection.managerTooOld
        : CoreCompatibilityDirection.coreTooOld;
  }
}

enum CoreCandidateState { missing, present }

final class CoreProbeEvidence {
  const CoreProbeEvidence({
    required this.transportAbi,
    this.coreInfo,
    this.coreInfoError,
  });

  final int transportAbi;
  final String? coreInfo;
  final String? coreInfoError;
}

/// Injection seam for deterministic bootstrap tests without loading native code.
abstract interface class CoreBootstrapProbe {
  CoreCandidateState candidateState(String path);
  CoreProbeEvidence inspect(String path);
}

abstract class GoreCoreFfiService {
  bool get isAvailable;
  String get description;
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  });
}

abstract interface class CoreBootstrapStateProvider {
  CoreBootstrapState get bootstrapState;
}

CoreBootstrapState coreBootstrapStateOf(GoreCoreFfiService service) {
  if (service case CoreBootstrapStateProvider(:final bootstrapState)) {
    return bootstrapState;
  }
  return service.isAvailable
      ? CoreBootstrapReady(libraryPath: service.description)
      : CoreBootstrapBlocked(
          CoreBootstrapFailure(reason: CoreBootstrapFailureReason.dllMissing),
        );
}

/// Live implementation — binds gore_ffi.dll via dart:ffi.
class NativeGoreCoreFfiService
    implements GoreCoreFfiService, CoreBootstrapStateProvider {
  NativeGoreCoreFfiService._(this.description, this.coreVersion);

  final String? coreVersion;

  @override
  final String description;

  @override
  bool get isAvailable => true;

  @override
  CoreBootstrapState get bootstrapState =>
      CoreBootstrapReady(libraryPath: description, coreVersion: coreVersion);

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    final request = jsonEncode({'command': command, 'payload': payload});
    return Isolate.run(
      () => decodeCanonicalGoreCoreResponse(
        _executeNativeRequest(description, request),
      ),
    );
  }
}

/// Decode only the compact canonical JSON emitted by gore-ffi.
///
/// Dart's normal decoder silently keeps the last duplicate key. Re-encoding
/// the insertion-ordered result and requiring byte equality also rejects
/// whitespace and alternate string spellings before manager DTOs can accept a
/// normalized hostile response.
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

CoreBootstrapState inspectCoreCandidates({
  required Iterable<String> candidates,
  required CoreBootstrapProbe probe,
}) {
  final candidateList = <String>[];
  final seen = <String>{};
  for (final candidate in candidates) {
    final key = p.normalize(candidate).toLowerCase();
    if (seen.add(key)) candidateList.add(candidate);
  }

  CoreBootstrapFailure? firstFailure;
  for (final candidate in candidateList) {
    final CoreCandidateState state;
    try {
      state = probe.candidateState(candidate);
    } catch (error) {
      firstFailure ??= CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.dllLoadFailed,
        candidatePath: candidate,
        detail: error.toString(),
      );
      continue;
    }
    if (state == CoreCandidateState.missing) continue;

    final CoreProbeEvidence evidence;
    try {
      evidence = probe.inspect(candidate);
    } catch (error) {
      firstFailure ??= CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.dllLoadFailed,
        candidatePath: candidate,
        detail: error.toString(),
      );
      continue;
    }

    final candidateResult = _evaluateCoreCandidate(candidate, evidence);
    if (candidateResult is CoreBootstrapReady) return candidateResult;
    firstFailure ??= (candidateResult as CoreBootstrapBlocked).failure;
  }

  return CoreBootstrapBlocked(
    firstFailure ??
        CoreBootstrapFailure(
          reason: CoreBootstrapFailureReason.dllMissing,
          candidatePath: candidateList.firstOrNull,
        ),
  );
}

CoreBootstrapState _evaluateCoreCandidate(
  String candidate,
  CoreProbeEvidence evidence,
) {
  if (evidence.transportAbi != _transportAbiV2) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.transportAbiMismatch,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
      ),
    );
  }
  if (evidence.coreInfoError case final error?) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        detail: error,
      ),
    );
  }
  final response = evidence.coreInfo;
  if (response == null) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        detail: 'core_info response is missing',
      ),
    );
  }

  final Map<String, Object?> decoded;
  try {
    decoded = decodeCanonicalGoreCoreResponse(response);
  } on FormatException catch (error) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        detail: error.message,
      ),
    );
  }

  final rawVersion = decoded['version'];
  final coreVersion = rawVersion is String ? rawVersion : null;
  if (decoded['ok'] != true) {
    final error = decoded['error'];
    final errorDetail = error is Map
        ? [error['code'], error['message']]
              .whereType<String>()
              .where((part) => part.trim().isNotEmpty)
              .join(': ')
        : '';
    final detail = errorDetail.isEmpty
        ? 'core_info returned ok=false'
        : errorDetail;
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        coreVersion: coreVersion,
        detail: detail,
      ),
    );
  }

  final rawProtocolAbi = decoded['abi'];
  if (rawProtocolAbi is! int ||
      rawProtocolAbi < 0 ||
      rawProtocolAbi > 0xffffffff) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        coreVersion: coreVersion,
        detail: 'core_info abi is missing or is not an unsigned integer',
      ),
    );
  }
  if (rawProtocolAbi != _protocolAbi) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.protocolAbiMismatch,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        observedProtocolAbi: rawProtocolAbi,
        coreVersion: coreVersion,
      ),
    );
  }

  final commands = decoded['commands'];
  if (commands is! List || commands.any((command) => command is! String)) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.coreInfoInvalid,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        observedProtocolAbi: rawProtocolAbi,
        coreVersion: coreVersion,
        detail: 'core_info commands are missing or invalid',
      ),
    );
  }
  final advertised = commands.cast<String>().toSet();
  final missing = managerRequiredCoreCommands
      .where((command) => !advertised.contains(command))
      .toList();
  if (missing.isNotEmpty) {
    return CoreBootstrapBlocked(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.requiredCommandsMissing,
        candidatePath: candidate,
        observedTransportAbi: evidence.transportAbi,
        observedProtocolAbi: rawProtocolAbi,
        coreVersion: coreVersion,
        missingCommands: missing,
      ),
    );
  }

  return CoreBootstrapReady(
    libraryPath: candidate,
    coreVersion: boundedDiagnosticText(coreVersion, 128).value,
  );
}

final class NativeCoreBootstrapProbe implements CoreBootstrapProbe {
  const NativeCoreBootstrapProbe()
    : _candidateType = null,
      _confirmCandidate = null;

  const NativeCoreBootstrapProbe.forTesting(
    this._candidateType,
    this._confirmCandidate,
  );

  /// Narrow filesystem injection seam for deterministic path-probe tests.
  final FileSystemEntityType Function(String path)? _candidateType;
  final void Function(String path)? _confirmCandidate;

  @override
  CoreCandidateState candidateState(String path) {
    final type = (_candidateType ?? _nativeCoreCandidateType)(path);
    if (type != FileSystemEntityType.notFound) {
      return CoreCandidateState.present;
    }
    try {
      (_confirmCandidate ?? _confirmNativeCoreCandidate)(path);
      return CoreCandidateState.present;
    } on FileSystemException catch (error) {
      if (_isTrueNotFound(error)) return CoreCandidateState.missing;
      rethrow;
    }
  }

  @override
  CoreProbeEvidence inspect(String path) {
    final lib = DynamicLibrary.open(path);
    final transportProbe = lib
        .lookupFunction<_TransportProbeNative, _TransportProbeDart>(
          'gore_core_transport_abi_v2',
        );
    final transportAbi = transportProbe();
    if (transportAbi != _transportAbiV2) {
      return CoreProbeEvidence(transportAbi: transportAbi);
    }
    final execute = lib.lookupFunction<_ExecuteV2Native, _ExecuteV2Dart>(
      'gore_core_execute_v2',
    );
    final free = lib.lookupFunction<_FreeV2Native, _FreeV2Dart>(
      'gore_core_response_free_v2',
    );
    try {
      return CoreProbeEvidence(
        transportAbi: transportAbi,
        coreInfo: _executeV2(
          execute,
          free,
          _coreInfoRequest,
          responseLimit: _maxCoreInfoBytes,
        ),
      );
    } catch (error) {
      return CoreProbeEvidence(
        transportAbi: transportAbi,
        coreInfoError: error.toString(),
      );
    }
  }
}

FileSystemEntityType _nativeCoreCandidateType(String path) =>
    FileSystemEntity.typeSync(path, followLinks: true);

void _confirmNativeCoreCandidate(String path) {
  final file = File(path).openSync(mode: FileMode.read);
  file.closeSync();
}

bool _isTrueNotFound(FileSystemException error) {
  final errorCode = error.osError?.errorCode;
  return errorCode == 2 || (Platform.isWindows && errorCode == 3);
}

/// Stub returned when no current bounded gore_ffi.dll is available.
class MissingGoreCoreFfiService
    implements GoreCoreFfiService, CoreBootstrapStateProvider {
  MissingGoreCoreFfiService([CoreBootstrapFailure? failure])
    : failure =
          failure ??
          CoreBootstrapFailure(reason: CoreBootstrapFailureReason.dllMissing);

  final CoreBootstrapFailure failure;

  @override
  bool get isAvailable => false;
  @override
  String get description => 'current bounded gore_ffi.dll not available';
  @override
  CoreBootstrapState get bootstrapState => CoreBootstrapBlocked(failure);
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

/// Complete neutral preflight fixture for app/widget tests using the generic
/// fake core. Custom fakes can return this from their command switch too.
Map<String, Object?> fakeHealthyManagerPreflightResponse() {
  const ids = [
    'game_root',
    'install',
    'loadout',
    'deployment',
    'install_mutation',
    'ue4ss',
    'write_access',
  ];
  return {
    'ok': true,
    'preflight': {
      'format': 1,
      'checks': [
        for (final id in ids)
          {
            'id': id,
            'state': id == 'write_access' ? 'unverified' : 'ok',
            'code': id == 'write_access' ? 'unverified_read_only' : 'ready',
            'action': id == 'write_access' ? 'verify_during_apply' : 'none',
            'detail': 'test evidence',
            'items': <String>[],
          },
      ],
    },
  };
}

GoreCoreFfiService createCoreService() {
  final state = inspectCoreCandidates(
    candidates: _candidateLibraryPaths(),
    probe: const NativeCoreBootstrapProbe(),
  );
  return switch (state) {
    CoreBootstrapReady(:final libraryPath, :final coreVersion) =>
      NativeGoreCoreFfiService._(libraryPath, coreVersion),
    CoreBootstrapBlocked(:final failure) => MissingGoreCoreFfiService(failure),
  };
}

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
