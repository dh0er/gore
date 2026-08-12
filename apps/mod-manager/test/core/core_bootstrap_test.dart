import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';

class _Probe implements CoreBootstrapProbe {
  _Probe({required this.states, this.inspections = const {}});

  final Map<String, Object> states;
  final Map<String, Object> inspections;
  final List<String> stateCalls = [];
  final List<String> inspectionCalls = [];

  @override
  CoreCandidateState candidateState(String path) {
    stateCalls.add(path);
    final result = states[path] ?? CoreCandidateState.missing;
    if (result is CoreCandidateState) return result;
    throw result;
  }

  @override
  CoreProbeEvidence inspect(String path) {
    inspectionCalls.add(path);
    final result = inspections[path];
    if (result is CoreProbeEvidence) return result;
    throw result ?? StateError('missing inspection result for $path');
  }
}

String _coreInfo({
  int abi = 2,
  Object? commands,
  String version = '0.1.0',
  Map<String, Object?> extra = const {},
}) => jsonEncode({
  'ok': true,
  'abi': abi,
  'version': version,
  'commands': commands ?? managerRequiredCoreCommands,
  ...extra,
});

CoreProbeEvidence _currentEvidence({int transportAbi = 2, String? coreInfo}) =>
    CoreProbeEvidence(
      transportAbi: transportAbi,
      coreInfo: coreInfo ?? _coreInfo(),
    );

CoreBootstrapFailure _blocked(CoreBootstrapState state) {
  expect(state, isA<CoreBootstrapBlocked>());
  return (state as CoreBootstrapBlocked).failure;
}

String _repeat(String value, int count) => List.filled(count, value).join();

void main() {
  test('all absent candidates produce an honest missing result', () {
    final probe = _Probe(
      states: {
        r'C:\app\gore_ffi.dll': CoreCandidateState.missing,
        r'C:\dev\gore_ffi.dll': CoreCandidateState.missing,
      },
    );

    final failure = _blocked(
      inspectCoreCandidates(
        candidates: const [r'C:\app\gore_ffi.dll', r'C:\dev\gore_ffi.dll'],
        probe: probe,
      ),
    );

    expect(failure.reason, CoreBootstrapFailureReason.dllMissing);
    expect(failure.candidatePath, r'C:\app\gore_ffi.dll');
    expect(probe.inspectionCalls, isEmpty);
  });

  test('stat or load failures stay distinct from a missing DLL', () {
    final statFailure = _Probe(
      states: {r'C:\app\gore_ffi.dll': FileSystemException('access denied')},
    );
    final stat = _blocked(
      inspectCoreCandidates(
        candidates: const [r'C:\app\gore_ffi.dll'],
        probe: statFailure,
      ),
    );
    expect(stat.reason, CoreBootstrapFailureReason.dllLoadFailed);
    expect(stat.detail, contains('access denied'));

    final loadFailure = _Probe(
      states: {r'C:\app\gore_ffi.dll': CoreCandidateState.present},
      inspections: {
        r'C:\app\gore_ffi.dll': ArgumentError('missing transport export'),
      },
    );
    final load = _blocked(
      inspectCoreCandidates(
        candidates: const [r'C:\app\gore_ffi.dll'],
        probe: loadFailure,
      ),
    );
    expect(load.reason, CoreBootstrapFailureReason.dllLoadFailed);
    expect(load.detail, contains('missing transport export'));
  });

  test('native path probe distinguishes absence from inspection failure', () {
    final absentProbe = NativeCoreBootstrapProbe.forTesting(
      (_) => FileSystemEntityType.notFound,
      (_) => throw const PathNotFoundException(
        'absent.dll',
        OSError('The system cannot find the file specified', 2),
      ),
    );
    final absent = _blocked(
      inspectCoreCandidates(
        candidates: const ['absent.dll'],
        probe: absentProbe,
      ),
    );
    expect(absent.reason, CoreBootstrapFailureReason.dllMissing);

    final deniedProbe = NativeCoreBootstrapProbe.forTesting(
      (_) => FileSystemEntityType.notFound,
      (_) => throw const PathAccessException(
        'blocked.dll',
        OSError('Access is denied', 5),
        'access denied',
      ),
    );
    final denied = _blocked(
      inspectCoreCandidates(
        candidates: const ['blocked.dll'],
        probe: deniedProbe,
      ),
    );
    expect(denied.reason, CoreBootstrapFailureReason.dllLoadFailed);
    expect(denied.candidatePath, 'blocked.dll');
    expect(denied.detail, contains('access denied'));
  });

  test('a later compatible candidate wins after earlier failures', () {
    final probe = _Probe(
      states: {
        r'C:\bundle\gore_ffi.dll': CoreCandidateState.present,
        r'C:\dev\gore_ffi.dll': CoreCandidateState.present,
      },
      inspections: {
        r'C:\bundle\gore_ffi.dll': StateError('blocked dependency'),
        r'C:\dev\gore_ffi.dll': _currentEvidence(),
      },
    );

    final state = inspectCoreCandidates(
      candidates: const [r'C:\bundle\gore_ffi.dll', r'C:\dev\gore_ffi.dll'],
      probe: probe,
    );

    expect(state, isA<CoreBootstrapReady>());
    expect((state as CoreBootstrapReady).libraryPath, r'C:\dev\gore_ffi.dll');
    expect(probe.inspectionCalls, hasLength(2));
  });

  test('transport and protocol mismatches retain update direction', () {
    final newerTransport = _Probe(
      states: {'new.dll': CoreCandidateState.present},
      inspections: {'new.dll': _currentEvidence(transportAbi: 3)},
    );
    final transport = _blocked(
      inspectCoreCandidates(
        candidates: const ['new.dll'],
        probe: newerTransport,
      ),
    );
    expect(transport.reason, CoreBootstrapFailureReason.transportAbiMismatch);
    expect(
      transport.compatibilityDirection,
      CoreCompatibilityDirection.managerTooOld,
    );

    final olderProtocol = _Probe(
      states: {'old.dll': CoreCandidateState.present},
      inspections: {'old.dll': _currentEvidence(coreInfo: _coreInfo(abi: 1))},
    );
    final protocol = _blocked(
      inspectCoreCandidates(
        candidates: const ['old.dll'],
        probe: olderProtocol,
      ),
    );
    expect(protocol.reason, CoreBootstrapFailureReason.protocolAbiMismatch);
    expect(protocol.observedProtocolAbi, 1);
    expect(
      protocol.compatibilityDirection,
      CoreCompatibilityDirection.coreTooOld,
    );
  });

  test('malformed, error, and invalid command handshakes fail closed', () {
    for (final response in [
      '{ "ok":true }',
      jsonEncode({
        'ok': false,
        'error': {'code': 'CORE_BROKEN', 'message': 'cannot inspect'},
      }),
      _coreInfo(abi: 0x100000000),
      _coreInfo(commands: [...managerRequiredCoreCommands, 7]),
    ]) {
      final probe = _Probe(
        states: {'bad.dll': CoreCandidateState.present},
        inspections: {'bad.dll': _currentEvidence(coreInfo: response)},
      );
      expect(
        _blocked(
          inspectCoreCandidates(candidates: const ['bad.dll'], probe: probe),
        ).reason,
        CoreBootstrapFailureReason.coreInfoInvalid,
      );
    }
  });

  test('a bounded core_info transport failure is a verification failure', () {
    final probe = _Probe(
      states: {'broken.dll': CoreCandidateState.present},
      inspections: const {
        'broken.dll': CoreProbeEvidence(
          transportAbi: 2,
          coreInfoError: 'response exceeded the bounded range',
        ),
      },
    );

    final failure = _blocked(
      inspectCoreCandidates(candidates: const ['broken.dll'], probe: probe),
    );

    expect(failure.reason, CoreBootstrapFailureReason.coreInfoInvalid);
    expect(failure.detail, contains('bounded range'));
  });

  test('missing required commands are stable and sorted', () {
    final commands = managerRequiredCoreCommands
        .where((command) => command != 'mgr_apply' && command != 'mgr_status')
        .toList();
    final probe = _Probe(
      states: {'partial.dll': CoreCandidateState.present},
      inspections: {
        'partial.dll': _currentEvidence(
          coreInfo: _coreInfo(commands: commands),
        ),
      },
    );

    final failure = _blocked(
      inspectCoreCandidates(candidates: const ['partial.dll'], probe: probe),
    );

    expect(failure.reason, CoreBootstrapFailureReason.requiredCommandsMissing);
    expect(failure.missingCommands, ['mgr_apply', 'mgr_status']);
  });

  test('a core without the consumed preflight command is blocked', () {
    final commands = managerRequiredCoreCommands
        .where((command) => command != 'mgr_preflight_v1')
        .toList();
    final probe = _Probe(
      states: {'old.dll': CoreCandidateState.present},
      inspections: {
        'old.dll': _currentEvidence(coreInfo: _coreInfo(commands: commands)),
      },
    );

    final failure = _blocked(
      inspectCoreCandidates(candidates: const ['old.dll'], probe: probe),
    );

    expect(failure.reason, CoreBootstrapFailureReason.requiredCommandsMissing);
    expect(failure.missingCommands, ['mgr_preflight_v1']);
  });

  test(
    'preflight is required while future fields and commands stay additive',
    () {
      expect(managerRequiredCoreCommands, contains('mgr_preflight_v1'));
      final probe = _Probe(
        states: {'future.dll': CoreCandidateState.present},
        inspections: {
          'future.dll': _currentEvidence(
            coreInfo: _coreInfo(
              commands: [...managerRequiredCoreCommands, 'future_command_v99'],
              extra: const {'future_field': true},
            ),
          ),
        },
      );

      final state = inspectCoreCandidates(
        candidates: const ['future.dll'],
        probe: probe,
      );
      expect(state, isA<CoreBootstrapReady>());
    },
  );

  test('technical report is stable, bounded, sanitized, and minimal', () {
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.dllLoadFailed,
      candidatePath: 'C:\\${_repeat('x', 700)}\nprivate-tail',
      coreVersion: 'v${_repeat('9', 300)}',
      missingCommands: const ['mgr_status', 'not-a-required-command'],
      detail:
          'load\r\nfailed\u0000\u0085\u061c\u200e\u200f\u2028\u2029'
          '\u202a\u202b\u202c\u202d\u202e\u2066\u2067\u2068\u2069'
          "${_repeat('z', 700)}",
    );

    final encoded = failure.technicalReport(
      managerVersion: '1.${_repeat('2', 300)}',
    );
    final report = jsonDecode(encoded) as Map<String, Object?>;

    expect(utf8.encode(encoded).length, lessThanOrEqualTo(8 * 1024));
    expect(report['schema'], 'gore-manager-core-bootstrap-v1');
    expect(report['reason'], 'core_library_load_failed');
    expect(report['dll_path_truncated'], true);
    expect(report['core_version_truncated'], true);
    expect(report['manager_version_truncated'], true);
    expect(report['detail_truncated'], true);
    final detail = report['detail'] as String;
    expect(detail, isNot(contains('\n')));
    expect(detail, isNot(contains('\u0085')));
    for (final bidiControl in const [
      '\u061c',
      '\u200e',
      '\u200f',
      '\u202a',
      '\u202b',
      '\u202c',
      '\u202d',
      '\u202e',
      '\u2066',
      '\u2067',
      '\u2068',
      '\u2069',
    ]) {
      expect(detail, isNot(contains(bidiControl)));
    }
    expect(report['missing_commands'], ['mgr_status']);
    expect(report['required_commands'], contains('mgr_preflight_v1'));
    expect(encoded, isNot(contains('private-tail')));
  });
}
