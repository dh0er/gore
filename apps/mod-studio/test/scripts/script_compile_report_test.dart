import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';

Map<String, Object?> _report({
  String outcome = 'failed',
  Object? miniPath,
  Object? module,
  Object? compileError = const {
    'code': 'COMPILER_REGEN_FAILED',
    'message': 'compiler rejected the source',
  },
  Object? diagnostics = const {
    'capture': 'captured',
    'messages': [
      {
        'file': 'GoreMods/Probe.as',
        'line': 12,
        'column': 7,
        'severity': 'error',
        'message': 'Expected expression',
      },
    ],
    'omitted': 0,
  },
  String installRestore = 'restored_exact',
  bool recoveryRequired = false,
}) => {
  'ok': true,
  'outcome': outcome,
  'mini_path': miniPath,
  'module': module,
  'compile_error': compileError,
  'compiler_diagnostics': diagnostics,
  'install_restore': installRestore,
  'recovery_required': recoveryRequired,
};

Map<String, Object?> _qualifiedPackage() => {
  'catalog_sha256':
      '1111111111111111111111111111111111111111111111111111111111111111',
  'sidecar_byte_len': 1966592,
  'sidecar_sha256':
      '2222222222222222222222222222222222222222222222222222222222222222',
  'request_version': 2,
  'response_version': 1,
  'manifest_byte_len': 4096,
  'manifest_sha256':
      '3333333333333333333333333333333333333333333333333333333333333333',
  'profile_sha256':
      '4444444444444444444444444444444444444444444444444444444444444444',
  'target': {
    'target': {
      'steam_app_id': 1297900,
      'steam_build_id': 24539464,
      'depot_id': 1297901,
      'depot_manifest_gid': 1585071322101748861,
      'platform': 'windows',
      'architecture': 'x86_64',
      'build_configuration': 'shipping',
    },
    'pe_codeview': {'guid': 'be78fe0a-46ac-6643-9685-97e85c7e5b3f', 'age': 1},
  },
};

void main() {
  test('parses a closed product-qualified standalone package identity', () {
    final evidence = ScriptCompilerBackendEvidence.fromJson({
      'requested_mode': 'standalone',
      'result_backend': 'standalone',
      'standalone_attempted': true,
      'game_attempted': false,
      'qualified_package': _qualifiedPackage(),
      'fallback_reason': null,
    }, expectedMode: ScriptCompilerBackendMode.standalone);

    expect(evidence.qualifiedPackage!.requestVersion, 2);
    expect(evidence.qualifiedPackage!.target.steamBuildId, 24539464);
  });

  test(
    'accepts untouched installation state for a qualified standalone result',
    () {
      final raw = _report(
        outcome: 'compiled',
        miniPath: r'C:\Temp\mini.cache',
        module: 'GoreMods.Probe',
        compileError: null,
        diagnostics: const {
          'capture': 'captured',
          'messages': <Object?>[],
          'omitted': 0,
        },
        installRestore: 'not_started',
      );
      raw['compiler_backend'] = {
        'requested_mode': 'standalone',
        'result_backend': 'standalone',
        'standalone_attempted': true,
        'game_attempted': false,
        'qualified_package': _qualifiedPackage(),
        'fallback_reason': null,
      };

      final report = ScriptCompileReport.fromJson(
        raw,
        expectedBackend: ScriptCompilerBackendMode.standalone,
      );
      expect(report.compiled, isTrue);
      expect(report.installRestore, ScriptCompileInstallRestore.notStarted);
      expect(report.backend!.qualifiedPackage, isNotNull);
    },
  );

  test('requires package authority for a claimed standalone attempt', () {
    expect(
      () => ScriptCompilerBackendEvidence.fromJson({
        'requested_mode': 'standalone',
        'result_backend': 'standalone',
        'standalone_attempted': true,
        'game_attempted': false,
        'qualified_package': null,
        'fallback_reason': null,
      }, expectedMode: ScriptCompilerBackendMode.standalone),
      throwsFormatException,
    );
  });

  test(
    'standalone-then-game permits a global preflight before either backend',
    () {
      final evidence = ScriptCompilerBackendEvidence.fromJson({
        'requested_mode': 'standalone_then_game',
        'result_backend': null,
        'standalone_attempted': false,
        'game_attempted': false,
        'qualified_package': _qualifiedPackage(),
        'fallback_reason': null,
      }, expectedMode: ScriptCompilerBackendMode.standaloneThenGame);

      expect(evidence.resultBackend, isNull);
      expect(evidence.standaloneAttempted, isFalse);
      expect(evidence.gameAttempted, isFalse);
      expect(evidence.fallbackReason, isNull);
    },
  );

  test('standalone-then-game still requires fallback evidence after a run', () {
    expect(
      () => ScriptCompilerBackendEvidence.fromJson({
        'requested_mode': 'standalone_then_game',
        'result_backend': 'game',
        'standalone_attempted': false,
        'game_attempted': true,
        'qualified_package': _qualifiedPackage(),
        'fallback_reason': null,
      }, expectedMode: ScriptCompilerBackendMode.standaloneThenGame),
      throwsFormatException,
    );
  });

  test('parses normal compiler diagnostics independently from restore', () {
    final report = ScriptCompileReport.fromJson(_report());

    expect(report.compiled, isFalse);
    expect(report.recoveryRequired, isFalse);
    expect(report.installRestore, ScriptCompileInstallRestore.restoredExact);
    expect(report.failure!.code, 'COMPILER_REGEN_FAILED');
    expect(
      report.diagnostics!.capture,
      ScriptCompileCaptureDisposition.captured,
    );
    expect(
      report.diagnostics!.messages.single.location,
      'GoreMods/Probe.as(12,7)',
    );
    expect(
      report.diagnostics!.messages.single.severity,
      ScriptCompilerDiagnosticSeverity.error,
    );
  });

  test('keeps a game attempt when metadata was unavailable', () {
    final raw = _report(installRestore: 'not_started', diagnostics: null);
    raw['compiler_backend'] = <String, Object?>{
      'requested_mode': 'game',
      'result_backend': 'game',
      'standalone_attempted': false,
      'game_attempted': true,
      'qualified_package': null,
      'fallback_reason': null,
    };

    final report = ScriptCompileReport.fromJson(
      raw,
      expectedBackend: ScriptCompilerBackendMode.game,
    );
    expect(report.compiled, isFalse);
    expect(report.installRestore, ScriptCompileInstallRestore.notStarted);
    expect(report.backend!.resultBackend, ScriptCompilerBackendName.game);
    expect(report.backend!.gameAttempted, isTrue);
  });

  test('accepts a compiled result that used the normal fallback', () {
    final report = ScriptCompileReport.fromJson(
      _report(
        outcome: 'compiled',
        miniPath: r'C:\Temp\mini.cache',
        module: 'GoreMods.Probe',
        compileError: null,
        diagnostics: const {
          'capture': 'unavailable_fallback',
          'messages': <Object?>[],
          'omitted': 0,
        },
      ),
    );

    expect(report.compiled, isTrue);
    expect(report.diagnostics!.usedNormalFallback, isTrue);
    expect(report.installRestore, ScriptCompileInstallRestore.restoredExact);
  });

  test('keeps process-exit recovery dominant over compiler messages', () {
    final report = ScriptCompileReport.fromJson(
      _report(
        diagnostics: const {
          'capture': 'process_exit_unconfirmed',
          'messages': <Object?>[],
          'omitted': 0,
        },
        installRestore: 'recovery_required_process_exit_unconfirmed',
        recoveryRequired: true,
      ),
    );

    expect(report.recoveryRequired, isTrue);
    expect(report.gameInstallRecoveryRequired, isTrue);
    expect(
      report.installRestore,
      ScriptCompileInstallRestore.recoveryRequiredProcessExitUnconfirmed,
    );
  });

  test('rejects compiled output without exact restoration evidence', () {
    expect(
      () => ScriptCompileReport.fromJson(
        _report(
          outcome: 'compiled',
          miniPath: r'C:\Temp\mini.cache',
          module: 'GoreMods.Probe',
          compileError: null,
          installRestore: 'not_started',
        ),
      ),
      throwsFormatException,
    );
  });

  test('rejects compiled output that still contains an error diagnostic', () {
    expect(
      () => ScriptCompileReport.fromJson(
        _report(
          outcome: 'compiled',
          miniPath: r'C:\Temp\mini.cache',
          module: 'GoreMods.Probe',
          compileError: null,
          diagnostics: const {
            'capture': 'captured',
            'messages': [
              {
                'file': 'GoreMods/Probe.as',
                'line': 1,
                'column': 1,
                'severity': 'error',
                'message': 'rejected despite compiled outcome',
              },
            ],
            'omitted': 0,
          },
        ),
      ),
      throwsFormatException,
    );
  });

  test('rejects non-success capture dispositions on compiled output', () {
    for (final capture in const [
      'capture_invalid',
      'unavailable_without_fallback',
      'disabled',
    ]) {
      expect(
        () => ScriptCompileReport.fromJson(
          _report(
            outcome: 'compiled',
            miniPath: r'C:\Temp\mini.cache',
            module: 'GoreMods.Probe',
            compileError: null,
            diagnostics: {
              'capture': capture,
              'messages': <Object?>[],
              'omitted': 0,
            },
          ),
        ),
        throwsFormatException,
        reason: capture,
      );
    }
  });

  test(
    'rejects recovery booleans that disagree with the typed disposition',
    () {
      expect(
        () => ScriptCompileReport.fromJson(
          _report(installRestore: 'restored_exact', recoveryRequired: true),
        ),
        throwsFormatException,
      );
    },
  );

  test('accepts closed preexisting-recovery failures before mutation', () {
    for (final code in const [
      'COMPILE_BASE_RECOVERY_REQUIRED',
      'COMPILE_INSTALL_RECOVERY_REQUIRED',
      'COMPILE_INSTALL_GUARD_RELEASE_FAILED',
    ]) {
      final report = ScriptCompileReport.fromJson(
        _report(
          compileError: {
            'code': code,
            'message': 'existing recovery evidence blocks compilation',
          },
          diagnostics: null,
          installRestore: 'not_started',
          recoveryRequired: true,
        ),
      );

      expect(report.compiled, isFalse, reason: code);
      expect(report.recoveryRequired, isTrue, reason: code);
      expect(report.gameInstallRecoveryRequired, isTrue, reason: code);
      expect(report.installRestore, ScriptCompileInstallRestore.notStarted);
    }
  });

  test('rejects unknown not-started failures claiming recovery', () {
    expect(
      () => ScriptCompileReport.fromJson(
        _report(
          compileError: const {
            'code': 'UNRELATED_FAILURE',
            'message': 'not a closed recovery disposition',
          },
          diagnostics: null,
          installRestore: 'not_started',
          recoveryRequired: true,
        ),
      ),
      throwsFormatException,
    );
  });

  test('rejects unknown response and diagnostic fields', () {
    final response = _report()..['surprise'] = true;
    expect(() => ScriptCompileReport.fromJson(response), throwsFormatException);

    final diagnostics = Map<String, Object?>.from(
      _report()['compiler_diagnostics']! as Map,
    )..['surprise'] = true;
    expect(
      () => ScriptCompileReport.fromJson(_report(diagnostics: diagnostics)),
      throwsFormatException,
    );
  });
}
