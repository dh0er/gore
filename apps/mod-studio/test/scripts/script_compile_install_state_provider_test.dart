import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state_provider.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';
import 'package:path/path.dart' as p;

ScriptCompileInstallState _safeState() =>
    ScriptCompileInstallState.fromJson(<String, Object?>{
      'ok': true,
      'disposition': 'safe_to_compile',
      'safe_to_compile': true,
      'game_process': 'not_running',
      'artifacts': <Object?>[],
      'issues': <Object?>[],
    });

ScriptCompileInstallState _unsafeState() =>
    ScriptCompileInstallState.fromJson(<String, Object?>{
      'ok': true,
      'disposition': 'recovery_artifacts_present',
      'safe_to_compile': false,
      'game_process': 'not_running',
      'artifacts': <Object?>[
        <String, Object?>{
          'kind': 'recovery_journal',
          'display_path': r'C:\Game\.gore-compile-recovery.json',
          'path_truncated': false,
        },
      ],
      'issues': <Object?>[],
    });

ScriptCompileReport _recoveryReport() =>
    ScriptCompileReport.fromJson(<String, Object?>{
      'ok': true,
      'outcome': 'failed',
      'mini_path': null,
      'module': null,
      'compile_error': <String, Object?>{
        'code': 'INSTALL_RESTORE_FAILED',
        'message': 'exact restore could not be proven',
      },
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[],
        'omitted': 0,
      },
      'install_restore': 'recovery_required_restore_failed',
      'recovery_required': true,
    });

void main() {
  test(
    'retains recovery report per install until a fresh safe probe',
    () async {
      var next = _unsafeState();
      final controller = ScriptCompileInstallSafetyController(
        (_) async => next,
        gameRoot: r'C:\GameA',
        autoRefresh: false,
      );
      addTearDown(controller.dispose);
      final report = _recoveryReport();

      controller.recordCompileReport(report, gameRoot: r'C:\GameA');
      expect(controller.current.recoveryReport, same(report));
      expect(controller.current.liveMutationAllowed, isFalse);

      controller.setGameRoot(r'C:\GameB', refresh: false);
      expect(controller.current.recoveryReport, isNull);
      controller.setGameRoot(r'C:\GameA', refresh: false);
      expect(controller.current.recoveryReport, same(report));

      await controller.refresh();
      expect(controller.current.installState?.safeToCompile, isFalse);
      expect(controller.current.recoveryReport, same(report));
      expect(controller.current.liveMutationAllowed, isFalse);

      next = _safeState();
      await controller.refresh();
      expect(controller.current.recoveryReport, isNull);
      expect(controller.current.liveMutationAllowed, isTrue);
    },
  );

  test('probe failure is fail-closed and keeps recovery evidence', () async {
    final controller = ScriptCompileInstallSafetyController(
      (_) async => throw StateError('inspection unavailable'),
      gameRoot: r'C:\Game',
      autoRefresh: false,
    );
    addTearDown(controller.dispose);
    final report = _recoveryReport();
    controller.recordCompileReport(report, gameRoot: r'C:\Game');

    await controller.refresh();

    expect(controller.current.phase, ScriptCompileInstallSafetyPhase.failed);
    expect(controller.current.recoveryReport, same(report));
    expect(controller.current.liveMutationAllowed, isFalse);
  });

  test(
    'managed recovery blocks every live mutation and survives install changes',
    () async {
      var next = _unsafeState();
      final controller = ScriptCompileInstallSafetyController(
        (_) async => next,
        gameRoot: r'C:\GameA',
        autoRefresh: false,
      );
      addTearDown(controller.dispose);

      controller.recordManagedRecovery(
        gameRoot: r'C:\GameA',
        code: 'INSTALL_RESTORE_FAILED',
        message: 'exact restore could not be proven',
        installRestore:
            ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
      );

      expect(controller.current.recoveryRequired, isTrue);
      expect(controller.current.recoveryReport, isNull);
      expect(
        controller.current.recoveryEvidence?.code,
        'INSTALL_RESTORE_FAILED',
      );
      expect(controller.current.liveMutationAllowed, isFalse);

      controller.setGameRoot(r'C:\GameB', refresh: false);
      expect(controller.current.recoveryRequired, isFalse);
      controller.setGameRoot(r'C:\GameA', refresh: false);
      expect(controller.current.recoveryRequired, isTrue);

      await controller.refresh();
      expect(controller.current.recoveryRequired, isTrue);
      expect(controller.current.liveMutationAllowed, isFalse);

      next = _safeState();
      await controller.refresh();
      expect(controller.current.recoveryRequired, isFalse);
      expect(controller.current.liveMutationAllowed, isTrue);
    },
  );

  test('managed recovery stays bound to the attempted installation', () async {
    final controller = ScriptCompileInstallSafetyController(
      (_) async => _unsafeState(),
      gameRoot: r'C:\GameB',
      autoRefresh: false,
    );
    addTearDown(controller.dispose);

    controller.recordManagedRecovery(
      gameRoot: r'C:\GameA',
      code: 'INSTALL_RESTORE_FAILED',
      message: 'Game A restore could not be proven',
      installRestore: ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
    );

    expect(controller.current.gameRoot, endsWith(r'GameB'));
    expect(controller.current.recoveryRequired, isFalse);

    controller.setGameRoot(r'C:\GameA', refresh: false);
    expect(controller.current.recoveryRequired, isTrue);
    expect(
      controller.current.recoveryEvidence?.message,
      'Game A restore could not be proven',
    );
  });

  test('late safe probe cannot erase newer managed recovery', () async {
    final probe = Completer<ScriptCompileInstallState>();
    final controller = ScriptCompileInstallSafetyController(
      (_) => probe.future,
      gameRoot: r'C:\GameA',
      autoRefresh: false,
    );
    addTearDown(controller.dispose);

    final pendingRefresh = controller.refresh();
    controller.recordManagedRecovery(
      gameRoot: r'C:\GameA',
      code: 'INSTALL_RESTORE_FAILED',
      message: 'newer recovery evidence',
      installRestore: ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
    );
    probe.complete(_safeState());
    await pendingRefresh;

    expect(controller.current.recoveryRequired, isTrue);
    expect(
      controller.current.recoveryEvidence?.message,
      'newer recovery evidence',
    );
    expect(controller.current.liveMutationAllowed, isFalse);
  });

  test(
    'recovery for another install does not cancel the selected probe',
    () async {
      final probe = Completer<ScriptCompileInstallState>();
      final controller = ScriptCompileInstallSafetyController(
        (_) => probe.future,
        gameRoot: r'C:\GameB',
        autoRefresh: false,
      );
      addTearDown(controller.dispose);

      final pendingRefresh = controller.refresh();
      controller.recordManagedRecovery(
        gameRoot: r'C:\GameA',
        code: 'INSTALL_RESTORE_FAILED',
        message: 'Game A recovery evidence',
        installRestore:
            ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
      );
      probe.complete(_safeState());
      await pendingRefresh;

      expect(controller.current.gameRoot, endsWith(r'GameB'));
      expect(controller.current.liveMutationAllowed, isTrue);
      controller.setGameRoot(r'C:\GameA', refresh: false);
      expect(controller.current.recoveryRequired, isTrue);
    },
  );

  test('path aliases cannot resurrect recovery after a safe probe', () async {
    final root = p.normalize(p.absolute('GameA'));
    final alias = p.style == p.Style.windows ? root.toUpperCase() : root;
    final controller = ScriptCompileInstallSafetyController(
      (_) async => _safeState(),
      gameRoot: root,
      autoRefresh: false,
    );
    addTearDown(controller.dispose);

    controller.recordManagedRecovery(
      gameRoot: alias,
      code: 'INSTALL_RESTORE_FAILED',
      message: 'alias recovery evidence',
      installRestore: ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
    );
    expect(controller.current.recoveryRequired, isTrue);

    await controller.refresh();
    expect(controller.current.recoveryRequired, isFalse);
    controller.setGameRoot(p.normalize(p.absolute('GameB')), refresh: false);
    controller.setGameRoot(alias, refresh: false);
    expect(controller.current.recoveryRequired, isFalse);
  });
}
