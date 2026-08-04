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

void main() {
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
