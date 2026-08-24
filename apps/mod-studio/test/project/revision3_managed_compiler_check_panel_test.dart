import 'dart:convert';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_managed_compiler_check_panel.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state_provider.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';
import 'package:gore_mod/scripts/ui/script_compile_install_state_banner.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '00000000000000000000000000000031';
const _questId = '00000000000000000000000000000071';
const _moduleId = '00000000000000000000000000000091';
const _projectSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

String _headJson() => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 4096, 'sha256': _projectSha},
});

ScriptCompileInstallState _safeInstall() =>
    ScriptCompileInstallState.fromJson(<String, Object?>{
      'ok': true,
      'disposition': 'safe_to_compile',
      'safe_to_compile': true,
      'game_process': 'not_running',
      'artifacts': <Object?>[],
      'issues': <Object?>[],
    });

enum _ReceiptKind {
  accepted,
  staleCompiled,
  recovery,
  compilerNotRun,
  compilerRejected,
  outputCleanupFailed,
}

ManagedRevision3CompilerCheckReceipt _receipt({
  _ReceiptKind kind = _ReceiptKind.accepted,
  ScriptCompilerBackendMode? backendMode,
}) {
  final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());
  final compiler = switch (kind) {
    _ReceiptKind.accepted || _ReceiptKind.staleCompiled => <String, Object?>{
      'outcome': 'compiled_evidence_only',
      'compile_error': null,
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[
          <String, Object?>{
            'file': 'GoreMods/Quests/CompilerCheck.as',
            'line': 12,
            'column': 7,
            'severity': 'warning',
            'message': 'bounded diagnostic',
          },
        ],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_discarded': true,
    },
    _ReceiptKind.recovery => <String, Object?>{
      'outcome': 'failed',
      'compile_error': <String, Object?>{
        'code': 'INSTALL_RESTORE_FAILED',
        'message': 'exact restore could not be proven',
      },
      'compiler_diagnostics': null,
      'install_restore': 'recovery_required_restore_failed',
      'recovery_required': true,
      'output_discarded': true,
    },
    _ReceiptKind.compilerNotRun => <String, Object?>{
      'outcome': 'failed',
      'compile_error': <String, Object?>{
        'code': 'COMPILE_GAME_PROCESS_RUNNING',
        'message': 'Close the game before checking this source.',
      },
      'compiler_diagnostics': null,
      'install_restore': 'not_started',
      'recovery_required': false,
      'output_discarded': true,
    },
    _ReceiptKind.compilerRejected => <String, Object?>{
      'outcome': 'failed',
      'compile_error': <String, Object?>{
        'code': 'COMPILE_FAILED',
        'message': 'The game compiler rejected this source.',
      },
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[
          <String, Object?>{
            'file': 'GoreMods/Quests/CompilerCheck.as',
            'line': 4,
            'column': 2,
            'severity': 'error',
            'message': 'unexpected token',
          },
        ],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_discarded': true,
    },
    _ReceiptKind.outputCleanupFailed => <String, Object?>{
      'outcome': 'failed',
      'compile_error': <String, Object?>{
        'code': 'COMPILE_OUTPUT_UNSAFE',
        'message': 'Exact compiler output disposal was not proven.',
      },
      'compiler_diagnostics': null,
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_discarded': false,
    },
  };
  if (backendMode != null) {
    compiler['compiler_backend'] = switch (backendMode) {
      ScriptCompilerBackendMode.game => <String, Object?>{
        'requested_mode': 'game',
        'result_backend': 'game',
        'standalone_attempted': false,
        'game_attempted': true,
        'qualified_package': null,
        'fallback_reason': null,
      },
      ScriptCompilerBackendMode.standalone => <String, Object?>{
        'requested_mode': 'standalone',
        'result_backend': 'standalone',
        'standalone_attempted': true,
        'game_attempted': false,
        'qualified_package': null,
        'fallback_reason': null,
      },
      ScriptCompilerBackendMode.standaloneThenGame => <String, Object?>{
        'requested_mode': 'standalone_then_game',
        'result_backend': 'game',
        'standalone_attempted': false,
        'game_attempted': true,
        'qualified_package': null,
        'fallback_reason': <String, Object?>{
          'failed_backend': 'standalone',
          'failure_kind': 'unavailable',
          'detail': 'No qualified standalone compiler package is installed.',
        },
      },
    };
  }
  final result = AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'compiler_check_only',
      'exact_current': kind == _ReceiptKind.accepted,
      'head_json': _headJson(),
      'project': <String, Object?>{
        'id': _projectId,
        'revision': 14,
        'seal': <String, Object?>{'byte_len': 4096, 'sha256': _projectSha},
      },
      'entity': <String, Object?>{
        'kind': 'quest_draft',
        'id': _questId,
        'revision': 8,
      },
      'module': <String, Object?>{
        'id': _moduleId,
        'revision': 9,
        'namespace': 'GoreMods.Quests.CompilerCheck',
        'relative_path': 'GoreMods/Quests/CompilerCheck.as',
        'source_sha256':
            'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      },
      'compiler': compiler,
      'scope': 'compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedEntityId: _questId,
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.questDraft,
    expectedBackend: backendMode,
  );
  return ManagedRevision3CompilerCheckReceipt(
    result: result,
    storeStillExactCurrent: true,
  );
}

void main() {
  testWidgets('shows exact acceptance and reports busy state', (tester) async {
    expect(_receipt().acceptedAtExactCurrent, isTrue);
    final safety = ScriptCompileInstallSafetyController(
      (_) async => _safeInstall(),
      gameRoot: _gameRoot,
      autoRefresh: false,
    );
    await safety.refresh();
    bool? accepted;
    final busyChanges = <bool>[];
    await _pumpPanel(
      tester,
      safety: safety,
      check: ({required compilerBackend}) async => _receipt(),
      onAcceptanceChanged: (value) => accepted = value,
      onBusyChanged: busyChanges.add,
    );
    await _runPanel(tester);

    expect(find.text('Exact source accepted by the compiler'), findsOneWidget);
    expect(find.text('bounded diagnostic'), findsOneWidget);
    expect(find.textContaining('build, runtime, deploy'), findsOneWidget);
    expect(accepted, isTrue);
    expect(busyChanges, <bool>[true, false]);
    expect(
      tester
          .widgetList<Semantics>(find.byType(Semantics))
          .any((widget) => widget.properties.liveRegion == true),
      isTrue,
    );
  });

  testWidgets('records managed restore recovery in the shared safety gate', (
    tester,
  ) async {
    expect(_receipt(kind: _ReceiptKind.recovery).recoveryRequired, isTrue);
    expect(
      _receipt(kind: _ReceiptKind.recovery).gameInstallRecoveryRequired,
      isTrue,
    );
    final safety = ScriptCompileInstallSafetyController(
      (_) async => _safeInstall(),
      gameRoot: _gameRoot,
      autoRefresh: false,
    );
    await safety.refresh();
    await _pumpPanel(
      tester,
      safety: safety,
      check: ({required compilerBackend}) async =>
          _receipt(kind: _ReceiptKind.recovery),
    );

    await tester.tap(
      find.byKey(const Key('revision3-managed-compiler-check-run')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-managed-compiler-confirm')),
    );
    await tester.pump();
    await tester.runAsync(() async {
      for (var attempt = 0; attempt < 100; attempt++) {
        if (safety.current.recoveryRequired) return;
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      fail('managed recovery was not recorded');
    });
    await tester.pump();

    expect(safety.current.recoveryRequired, isTrue);
    expect(safety.current.liveMutationAllowed, isFalse);
    expect(find.text('Game installation recovery required'), findsWidgets);
    expect(find.textContaining('INSTALL_RESTORE_FAILED'), findsWidgets);
    expect(
      find.byKey(const Key('script-compile-install-state-view-report')),
      findsNothing,
    );
  });

  for (final scenario in <(_ReceiptKind, String)>[
    (_ReceiptKind.staleCompiled, 'Compiler result is no longer current'),
    (_ReceiptKind.compilerNotRun, 'Compiler check did not run'),
    (_ReceiptKind.compilerRejected, 'Compiler rejected this source'),
    (_ReceiptKind.outputCleanupFailed, 'Compiler output cleanup failed'),
  ]) {
    testWidgets('shows the precise ${scenario.$2.toLowerCase()} outcome', (
      tester,
    ) async {
      final safety = ScriptCompileInstallSafetyController(
        (_) async => _safeInstall(),
        gameRoot: _gameRoot,
        autoRefresh: false,
      );
      await safety.refresh();
      await _pumpPanel(
        tester,
        safety: safety,
        check: ({required compilerBackend}) async =>
            _receipt(kind: scenario.$1),
      );
      await _runPanel(tester);

      expect(find.text(scenario.$2), findsOneWidget);
      if (scenario.$1 != _ReceiptKind.compilerRejected) {
        expect(find.text('Compiler rejected this source'), findsNothing);
      }
    });
  }

  testWidgets(
    'binds recovery to the attempted root after the selected root changes',
    (tester) async {
      final pending = Completer<ManagedRevision3CompilerCheckReceipt>();
      final safety = ScriptCompileInstallSafetyController(
        (_) async => _safeInstall(),
        gameRoot: _gameRoot,
        autoRefresh: false,
      );
      await safety.refresh();
      var checkStarted = false;
      await _pumpPanel(
        tester,
        safety: safety,
        check: ({required compilerBackend}) {
          checkStarted = true;
          return pending.future;
        },
      );

      await tester.tap(
        find.byKey(const Key('revision3-managed-compiler-check-run')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-managed-compiler-confirm')),
      );
      for (var attempt = 0; attempt < 10 && !checkStarted; attempt++) {
        await tester.pump();
      }
      expect(checkStarted, isTrue);

      safety.setGameRoot(r'C:\Games\Another Gothic Install', refresh: false);
      await safety.refresh();
      expect(safety.current.recoveryRequired, isFalse);
      pending.complete(_receipt(kind: _ReceiptKind.recovery));
      await tester.pumpAndSettle();

      expect(safety.current.gameRoot, contains('Another Gothic Install'));
      expect(safety.current.recoveryRequired, isFalse);
      expect(find.text('Game installation recovery required'), findsOneWidget);
      safety.setGameRoot(_gameRoot, refresh: false);
      expect(safety.current.recoveryRequired, isTrue);
      expect(safety.current.recoveryEvidence?.code, 'INSTALL_RESTORE_FAILED');
    },
  );

  testWidgets('strict standalone never consults live-install safety', (
    tester,
  ) async {
    var safetyLoads = 0;
    final safety = ScriptCompileInstallSafetyController(
      (_) async {
        safetyLoads++;
        throw StateError('standalone must not inspect live mutation safety');
      },
      gameRoot: _gameRoot,
      autoRefresh: false,
    );
    ScriptCompilerBackendMode? requestedBackend;
    await _pumpPanel(
      tester,
      safety: safety,
      check: ({required compilerBackend}) async {
        requestedBackend = compilerBackend;
        return _receipt();
      },
    );

    await tester.tap(
      find.byKey(const Key('revision3-managed-compiler-backend')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Standalone compiler').last);
    await tester.pumpAndSettle();
    await _runPanel(tester);

    expect(requestedBackend, ScriptCompilerBackendMode.standalone);
    expect(safetyLoads, 0);
    expect(find.byType(ScriptCompileInstallStateBanner), findsNothing);
  });

  testWidgets('fallback result names both backends and the reason', (
    tester,
  ) async {
    final safety = ScriptCompileInstallSafetyController(
      (_) async => _safeInstall(),
      gameRoot: _gameRoot,
      autoRefresh: false,
    );
    ScriptCompilerBackendMode? requestedBackend;
    await _pumpPanel(
      tester,
      safety: safety,
      check: ({required compilerBackend}) async {
        requestedBackend = compilerBackend;
        return _receipt(backendMode: compilerBackend);
      },
    );

    await _runPanel(tester);

    expect(requestedBackend, ScriptCompilerBackendMode.productDefault);
    expect(
      find.byKey(const Key('revision3-managed-compiler-backend-result')),
      findsOneWidget,
    );
    expect(
      find.textContaining(
        'No qualified standalone compiler package is installed.',
      ),
      findsOneWidget,
    );
  });
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required ScriptCompileInstallSafetyController safety,
  required Revision3ManagedCompilerChecker check,
  ValueChanged<bool>? onAcceptanceChanged,
  ValueChanged<bool>? onBusyChanged,
}) => tester.pumpWidget(
  ProviderScope(
    overrides: [
      scriptCompileInstallSafetyProvider.overrideWith((ref) => safety),
    ],
    child: MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(
          child: Revision3ManagedCompilerCheckPanel(
            gameRoot: _gameRoot,
            check: check,
            onAcceptanceChanged: onAcceptanceChanged,
            onBusyChanged: onBusyChanged,
          ),
        ),
      ),
    ),
  ),
);

Future<void> _runPanel(WidgetTester tester) async {
  await tester.tap(
    find.byKey(const Key('revision3-managed-compiler-check-run')),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('revision3-managed-compiler-confirm')));
  await tester.pumpAndSettle();
}
