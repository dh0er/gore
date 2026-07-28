import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/l10n/app_localizations_en.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_project_compiler_check_panel.dart';
import 'package:gore_mod/project/revision3_test_release_workspace.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state_provider.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '11111111111111111111111111111111';
const _projectSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

enum _ReceiptKind {
  compiled,
  empty,
  rejected,
  fallbackRejected,
  preflight,
  preflightAfterRunner,
  runnerSetupFailure,
  recovery,
  outputRecovery,
  postReadStoreDrift,
}

void main() {
  test('maps every closed project compiler result lane', () async {
    final checkpoint = _checkpoint(7);

    Future<Revision3ProjectCompilerCheckSnapshot> run(
      _ReceiptKind kind, {
      String closingStore = 'exact',
      String closingGame = 'exact',
    }) async {
      final controller = _controller(checkpoint: checkpoint);
      addTearDown(controller.dispose);
      final observed = <Revision3ProjectCompilerOutcome>[];
      controller.addListener(() => observed.add(controller.snapshot.outcome));
      await controller.run(
        checkpoint: checkpoint,
        operation: () async => _receipt(
          checkpoint,
          kind,
          closingStore: closingStore,
          closingGame: closingGame,
        ),
      );
      expect(observed.first, Revision3ProjectCompilerOutcome.checking);
      return controller.snapshot;
    }

    final compiled = await run(_ReceiptKind.compiled);
    expect(compiled.outcome, Revision3ProjectCompilerOutcome.compiled);
    expect(compiled.checkState, Revision3TestReleaseCheckState.passed);
    final empty = await run(_ReceiptKind.empty);
    expect(empty.outcome, Revision3ProjectCompilerOutcome.empty);
    expect(empty.checkState, Revision3TestReleaseCheckState.passed);
    final rejected = await run(_ReceiptKind.rejected);
    expect(rejected.outcome, Revision3ProjectCompilerOutcome.rejected);
    expect(rejected.checkState, Revision3TestReleaseCheckState.needsAttention);
    expect(rejected.receipt?.result.compiler.runCount, 1);
    expect(rejected.failure?.message, 'Unexpected token in project script.');
    final preflight = await run(_ReceiptKind.preflight);
    expect(preflight.outcome, Revision3ProjectCompilerOutcome.preflightBlocked);
    final preflightAfterRunner = await run(_ReceiptKind.preflightAfterRunner);
    expect(
      preflightAfterRunner.outcome,
      Revision3ProjectCompilerOutcome.preflightBlocked,
      reason: 'run_count=1 is not a rejection without exact restoration',
    );
    final runnerSetupFailure = await run(_ReceiptKind.runnerSetupFailure);
    expect(
      runnerSetupFailure.outcome,
      Revision3ProjectCompilerOutcome.preflightBlocked,
      reason:
          'an exact restore without diagnostics or output is not a script rejection',
    );
    final recovery = await run(_ReceiptKind.recovery);
    expect(recovery.outcome, Revision3ProjectCompilerOutcome.recoveryRequired);
    final outputRecovery = await run(_ReceiptKind.outputRecovery);
    expect(
      outputRecovery.outcome,
      Revision3ProjectCompilerOutcome.recoveryRequired,
    );
    final postReadStoreDrift = await run(_ReceiptKind.postReadStoreDrift);
    expect(
      postReadStoreDrift.outcome,
      Revision3ProjectCompilerOutcome.requiresReopen,
      reason: 'the app-side post-read Store audit requires reopening',
    );
    for (final storeStatus in <String>['drift', 'inspection_failed']) {
      final storeAudit = await run(
        _ReceiptKind.rejected,
        closingStore: storeStatus,
      );
      expect(
        storeAudit.outcome,
        Revision3ProjectCompilerOutcome.requiresReopen,
        reason: 'closing store $storeStatus requires reopening',
      );
    }
    for (final gameStatus in <String>['drift', 'inspection_failed']) {
      final gameAudit = await run(
        _ReceiptKind.rejected,
        closingGame: gameStatus,
      );
      expect(
        gameAudit.outcome,
        Revision3ProjectCompilerOutcome.drifted,
        reason: 'closing game $gameStatus is retryable drift',
      );
    }
    for (final lane in <({String store, String game})>[
      (store: 'not_run', game: 'exact'),
      (store: 'exact', game: 'not_run'),
    ]) {
      final notRun = await run(
        _ReceiptKind.preflight,
        closingStore: lane.store,
        closingGame: lane.game,
      );
      expect(notRun.outcome, Revision3ProjectCompilerOutcome.preflightBlocked);
    }

    for (final snapshot in <Revision3ProjectCompilerCheckSnapshot>[
      compiled,
      empty,
      rejected,
      preflight,
      preflightAfterRunner,
      runnerSetupFailure,
      recovery,
      outputRecovery,
      postReadStoreDrift,
    ]) {
      final check = snapshot.toTestReleaseCheck(
        l10n: AppLocalizationsEn(),
        onPressed: () {},
      );
      expect(check.evidence?.scope, Revision3TestReleaseEvidenceScope.scripts);
      expect(
        check.evidence?.belongsTo(
          projectId: checkpoint.projectId,
          projectRevision: checkpoint.projectRevision,
          checkpointIdentity: checkpoint.checkpointIdentity,
          scope: Revision3TestReleaseEvidenceScope.scripts,
        ),
        isTrue,
      );
    }
    final disabled = compiled.toTestReleaseCheck(
      l10n: AppLocalizationsEn(),
      onPressed: null,
    );
    expect(disabled.onPressed, isNull);

    final reopen = _controller(checkpoint: checkpoint);
    addTearDown(reopen.dispose);
    await reopen.run(
      checkpoint: checkpoint,
      operation: () async =>
          throw const Revision3ProjectCompilerCheckRequiresReopenException(),
    );
    expect(
      reopen.snapshot.outcome,
      Revision3ProjectCompilerOutcome.requiresReopen,
    );

    final transport = _controller(checkpoint: checkpoint);
    addTearDown(transport.dispose);
    await transport.run(
      checkpoint: checkpoint,
      operation: () async => throw const FormatException('bad schema'),
    );
    expect(transport.snapshot.outcome, Revision3ProjectCompilerOutcome.failed);

    final gameDrift = _controller(checkpoint: checkpoint);
    addTearDown(gameDrift.dispose);
    await gameDrift.run(
      checkpoint: checkpoint,
      operation: () async => throw const ModFfiException(
        command: 'authoring_store_check_revision3_project_compiler_v1',
        code: 'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_DRIFT',
        message: 'sealed game inputs changed',
      ),
    );
    expect(gameDrift.snapshot.outcome, Revision3ProjectCompilerOutcome.drifted);

    final recoveryError = _controller(checkpoint: checkpoint);
    addTearDown(recoveryError.dispose);
    await recoveryError.run(
      checkpoint: checkpoint,
      operation: () async => throw const ModFfiException(
        command: 'authoring_store_check_revision3_project_compiler_v1',
        code: 'AUTHORING_REVISION3_PROJECT_COMPILER_RECOVERY_REQUIRED',
        message: 'exact install restoration requires recovery',
      ),
    );
    expect(
      recoveryError.snapshot.outcome,
      Revision3ProjectCompilerOutcome.recoveryRequired,
    );

    for (final code in <String>{
      'AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT',
      'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_INVALID',
      'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_UNAVAILABLE',
      'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_MISMATCH',
      'AUTHORING_REVISION3_PROJECT_COMPILER_INSTALL_UNAVAILABLE',
      'AUTHORING_REVISION3_PROJECT_COMPILER_STAGING_UNAVAILABLE',
      'AUTHORING_REVISION3_PROJECT_COMPILER_UNSUPPORTED_GENERATION',
    }) {
      final preflightError = _controller(checkpoint: checkpoint);
      addTearDown(preflightError.dispose);
      await preflightError.run(
        checkpoint: checkpoint,
        operation: () async => throw ModFfiException(
          command: 'authoring_store_check_revision3_project_compiler_v1',
          code: code,
          message: 'shared compiler preflight did not complete',
        ),
      );
      expect(
        preflightError.snapshot.outcome,
        Revision3ProjectCompilerOutcome.preflightBlocked,
        reason: code,
      );
    }

    final gameAuditFailure = _controller(checkpoint: checkpoint);
    addTearDown(gameAuditFailure.dispose);
    await gameAuditFailure.run(
      checkpoint: checkpoint,
      operation: () async => throw const ModFfiException(
        command: 'authoring_store_check_revision3_project_compiler_v1',
        code: 'AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_GAME_AUDIT_FAILED',
        message: 'sealed game inputs could not be inspected',
      ),
    );
    expect(
      gameAuditFailure.snapshot.outcome,
      Revision3ProjectCompilerOutcome.failed,
      reason: 'inspection failure is not evidence of drift',
    );
  });

  test(
    'project switch clears exact evidence and an old late result cannot attach',
    () async {
      final oldCheckpoint = _checkpoint(7);
      final newCheckpoint = Revision3ProjectCompilerCheckpoint(
        projectId: '22222222222222222222222222222222',
        projectRevision: 1,
        checkpointIdentity: _headJson(1),
      );
      final pending = Completer<ManagedRevision3ProjectCompilerCheckReceipt>();
      final controller = _controller(checkpoint: oldCheckpoint);
      addTearDown(controller.dispose);

      final oldRun = controller.run(
        checkpoint: oldCheckpoint,
        operation: () => pending.future,
      );
      expect(
        controller.snapshot.outcome,
        Revision3ProjectCompilerOutcome.checking,
      );

      controller.synchronize(
        checkpoint: newCheckpoint,
        gameRoot: _gameRoot,
        requiresReopen: false,
      );
      expect(controller.snapshot.checkpoint, newCheckpoint);
      expect(
        controller.snapshot.outcome,
        Revision3ProjectCompilerOutcome.notChecked,
      );
      expect(controller.snapshot.receipt, isNull);
      expect(controller.snapshot.attempted, isFalse);

      pending.complete(_receipt(oldCheckpoint, _ReceiptKind.compiled));
      await oldRun;
      expect(controller.snapshot.checkpoint, newCheckpoint);
      expect(
        controller.snapshot.outcome,
        Revision3ProjectCompilerOutcome.notChecked,
      );
      expect(controller.snapshot.receipt, isNull);
    },
  );

  test('no configured game is unavailable and cannot run', () async {
    final checkpoint = _checkpoint(7);
    final controller = Revision3ProjectCompilerCheckController(
      checkpoint: checkpoint,
      gameRoot: null,
      requiresReopen: false,
    );
    addTearDown(controller.dispose);
    var calls = 0;

    await controller.run(
      checkpoint: checkpoint,
      operation: () async {
        calls++;
        return _receipt(checkpoint, _ReceiptKind.compiled);
      },
    );

    expect(calls, 0);
    expect(
      controller.snapshot.checkState,
      Revision3TestReleaseCheckState.unavailable,
    );
    expect(controller.snapshot.isEvaluated, isFalse);
  });

  testWidgets(
    'clean rejection shows bounded diagnostics and normal fallback at 200% text',
    (tester) async {
      tester.view.physicalSize = const Size(480, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final checkpoint = _checkpoint(7);
      final controller = _controller(checkpoint: checkpoint);
      addTearDown(controller.dispose);
      final safety = _safety();
      String? checkedRoot;
      var checkCalls = 0;

      await _pumpDialog(
        tester,
        checkpoint: checkpoint,
        controller: controller,
        safety: safety,
        textScale: 2,
        check: ({required gameRoot}) async {
          checkCalls++;
          checkedRoot = gameRoot;
          return _receipt(checkpoint, _ReceiptKind.fallbackRejected);
        },
      );
      await tester.tap(find.byKey(const Key('revision3-project-compiler-run')));
      await tester.pumpAndSettle();

      expect(
        controller.snapshot.checkState,
        Revision3TestReleaseCheckState.needsAttention,
      );
      expect(checkCalls, 1);
      expect(checkedRoot, _gameRoot);
      expect(
        find.text('Unexpected token in project script.'),
        findsNWidgets(2),
      );
      expect(
        find.textContaining('normal game compiler fallback'),
        findsOneWidget,
      );
      expect(find.textContaining('File: Scripts/Quest.as'), findsOneWidget);
      expect(find.textContaining('Line: 17'), findsOneWidget);
      expect(find.textContaining('Error'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-project-compiler-diagnostics-omitted')),
        findsOneWidget,
      );
      expect(
        find.textContaining('4 additional compiler messages'),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'closing-audit transport failure is blocked, retryable, and refreshes safety',
    (tester) async {
      final checkpoint = _checkpoint(7);
      final controller = _controller(checkpoint: checkpoint);
      addTearDown(controller.dispose);
      var safetyLoads = 0;
      final safety = ScriptCompileInstallSafetyController(
        (_) async {
          safetyLoads++;
          return _safeInstall();
        },
        gameRoot: _gameRoot,
        autoRefresh: false,
      );
      await _pumpDialog(
        tester,
        checkpoint: checkpoint,
        controller: controller,
        safety: safety,
        check: ({required gameRoot}) async => throw const ModFfiException(
          command: 'authoring_store_check_revision3_project_compiler_v1',
          code:
              'AUTHORING_REVISION3_PROJECT_COMPILER_CLOSING_GAME_AUDIT_FAILED',
          message: 'closing audit failed',
        ),
      );

      await tester.tap(find.byKey(const Key('revision3-project-compiler-run')));
      await tester.pumpAndSettle();

      expect(
        controller.snapshot.outcome,
        Revision3ProjectCompilerOutcome.failed,
      );
      expect(
        controller.snapshot.checkState,
        Revision3TestReleaseCheckState.blocked,
      );
      expect(controller.snapshot.receipt, isNull);
      expect(safetyLoads, 2, reason: 'preflight plus closing refresh');
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-project-compiler-run')),
            )
            .onPressed,
        isNotNull,
      );
      expect(find.text('Retry compiler check'), findsOneWidget);
      expect(find.text('closing audit failed'), findsOneWidget);
    },
  );

  testWidgets('recovery is retained in the shared install safety gate', (
    tester,
  ) async {
    final checkpoint = _checkpoint(7);
    final controller = _controller(checkpoint: checkpoint);
    addTearDown(controller.dispose);
    final safety = _safety();
    await _pumpDialog(
      tester,
      checkpoint: checkpoint,
      controller: controller,
      safety: safety,
      check: ({required gameRoot}) async =>
          _receipt(checkpoint, _ReceiptKind.outputRecovery),
    );

    await tester.tap(find.byKey(const Key('revision3-project-compiler-run')));
    await tester.pumpAndSettle();

    expect(
      controller.snapshot.outcome,
      Revision3ProjectCompilerOutcome.recoveryRequired,
    );
    expect(safety.current.recoveryRequired, isTrue);
    expect(safety.current.liveMutationAllowed, isFalse);
    expect(
      safety.current.recoveryEvidence?.installRestore,
      ScriptCompileInstallRestore.restoredExact,
    );
    expect(
      safety.current.recoveryEvidence?.code,
      'COMPILE_OUTPUT_RECOVERY_REQUIRED',
    );
  });

  testWidgets('empty evidence never enables build or deployment', (
    tester,
  ) async {
    final checkpoint = _checkpoint(7);
    final controller = _controller(checkpoint: checkpoint);
    addTearDown(controller.dispose);
    await controller.run(
      checkpoint: checkpoint,
      operation: () async => _receipt(checkpoint, _ReceiptKind.empty),
    );
    final l10n = AppLocalizationsEn();
    final scripts = controller.snapshot.toTestReleaseCheck(
      l10n: l10n,
      onPressed: () {},
    );
    expect(scripts.evidence?.scope, Revision3TestReleaseEvidenceScope.scripts);
    expect(scripts.evidence?.projectId, checkpoint.projectId);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Revision3TestReleaseWorkspace(
            projectId: checkpoint.projectId,
            projectRevision: checkpoint.projectRevision,
            checkpointIdentity: checkpoint.checkpointIdentity,
            projectStructure: _idleCheck('Structure'),
            scripts: scripts,
            voice: _idleCheck('Voice'),
            dataAssets: _idleCheck('DataAssets'),
            playableBuild: Revision3TestReleaseCapability(
              title: 'Playable files',
              description: 'Description',
              blockedReason: 'Blocked',
              actionLabel: 'Build',
            ),
            deployment: Revision3TestReleaseCapability(
              title: 'Installation',
              description: 'Description',
              blockedReason: 'Blocked',
              actionLabel: 'Install',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final build = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-test-release-playable-build-action')),
    );
    final deploy = tester.widget<FilledButton>(
      find.byKey(const Key('revision3-test-release-deployment-action')),
    );
    expect(build.onPressed, isNull);
    expect(deploy.onPressed, isNull);
    final workspace = tester.widget<Revision3TestReleaseWorkspace>(
      find.byType(Revision3TestReleaseWorkspace),
    );
    expect(workspace.playableBuild.evidence, isNull);
    expect(workspace.playableBuild.onPressed, isNull);
    expect(workspace.deployment.evidence, isNull);
    expect(workspace.deployment.onPressed, isNull);
  });
}

Revision3ProjectCompilerCheckController _controller({
  required Revision3ProjectCompilerCheckpoint checkpoint,
}) => Revision3ProjectCompilerCheckController(
  checkpoint: checkpoint,
  gameRoot: _gameRoot,
  requiresReopen: false,
);

Revision3ProjectCompilerCheckpoint _checkpoint(int revision) =>
    Revision3ProjectCompilerCheckpoint(
      projectId: _projectId,
      projectRevision: revision,
      checkpointIdentity: _headJson(revision),
    );

String _headJson(int revision) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 4096 + revision,
    'sha256': _projectSha,
  },
});

ManagedRevision3ProjectCompilerCheckReceipt _receipt(
  Revision3ProjectCompilerCheckpoint checkpoint,
  _ReceiptKind kind, {
  String closingStore = 'exact',
  String closingGame = 'exact',
}) {
  final head = AuthoringWorkingHead.fromCanonicalJson(
    checkpoint.checkpointIdentity,
  );
  final empty = kind == _ReceiptKind.empty;
  final compiler = switch (kind) {
    _ReceiptKind.compiled ||
    _ReceiptKind.postReadStoreDrift => <String, Object?>{
      'outcome': 'compiled_evidence_only',
      'run_count': 1,
      'compile_error': null,
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_disposition': 'discarded',
    },
    _ReceiptKind.empty => <String, Object?>{
      'outcome': 'not_needed_empty',
      'run_count': 0,
      'compile_error': null,
      'compiler_diagnostics': null,
      'install_restore': 'not_started',
      'recovery_required': false,
      'output_disposition': 'not_created',
    },
    _ReceiptKind.rejected || _ReceiptKind.fallbackRejected => <String, Object?>{
      'outcome': 'failed',
      'run_count': 1,
      'compile_error': <String, Object?>{
        'code': 'COMPILER_REGEN_FAILED',
        'message': 'Unexpected token in project script.',
      },
      'compiler_diagnostics': <String, Object?>{
        'capture': kind == _ReceiptKind.fallbackRejected
            ? 'unavailable_fallback'
            : 'captured',
        'messages': <Object?>[
          <String, Object?>{
            'file': 'Scripts/Quest.as',
            'line': 17,
            'column': 4,
            'severity': 'error',
            'message': 'Unexpected token in project script.',
          },
        ],
        'omitted': kind == _ReceiptKind.fallbackRejected ? 4 : 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_disposition': 'discarded',
    },
    _ReceiptKind.preflight ||
    _ReceiptKind.preflightAfterRunner => <String, Object?>{
      'outcome': 'failed',
      'run_count': kind == _ReceiptKind.preflightAfterRunner ? 1 : 0,
      'compile_error': <String, Object?>{
        'code': 'COMPILE_GAME_PROCESS_RUNNING',
        'message': 'The game is still running.',
      },
      'compiler_diagnostics': null,
      'install_restore': 'not_started',
      'recovery_required': false,
      'output_disposition': 'not_created',
    },
    _ReceiptKind.runnerSetupFailure => <String, Object?>{
      'outcome': 'failed',
      'run_count': 1,
      'compile_error': <String, Object?>{
        'code': 'COMPILER_REGEN_FAILED',
        'message': 'Compiler transaction setup failed.',
      },
      'compiler_diagnostics': null,
      'install_restore': 'restored_exact',
      'recovery_required': false,
      'output_disposition': 'not_created',
    },
    _ReceiptKind.recovery => <String, Object?>{
      'outcome': 'failed',
      'run_count': 0,
      'compile_error': <String, Object?>{
        'code': 'COMPILE_INSTALL_RECOVERY_REQUIRED',
        'message': 'Exact restore could not be proven.',
      },
      'compiler_diagnostics': null,
      'install_restore': 'not_started',
      'recovery_required': true,
      'output_disposition': 'recovery_retained',
    },
    _ReceiptKind.outputRecovery => <String, Object?>{
      'outcome': 'failed',
      'run_count': 1,
      'compile_error': <String, Object?>{
        'code': 'COMPILE_OUTPUT_RECOVERY_REQUIRED',
        'message': 'Compiler output cleanup requires recovery.',
      },
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': true,
      'output_disposition': 'recovery_retained',
    },
  };
  final result = AuthoringRevision3ProjectCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'project_compiler_check_only',
      'exact_current': closingStore == 'exact' && closingGame == 'exact',
      'closing_audit': <String, Object?>{
        'store': closingStore,
        'game': closingGame,
      },
      'head_json': checkpoint.checkpointIdentity,
      'project': <String, Object?>{
        'id': checkpoint.projectId,
        'revision': checkpoint.projectRevision,
        'seal': <String, Object?>{
          'byte_len': head.snapshotByteLength,
          'sha256': head.snapshotSha256,
        },
      },
      'game_inputs': <String, Object?>{
        'executable': _seal('b', 1024),
        'shipping_cache': _seal('c', 2048),
        'binds_cache': _seal('d', 4096),
        'story_catalog': _seal('e', 512),
      },
      'coverage': <String, Object?>{
        'script_module_count': empty ? 0 : 1,
        'quest_module_count': empty ? 0 : 1,
        'npc_module_count': 0,
        'module_manifest': _seal('f', 256),
      },
      'compiler': compiler,
      'scope': 'project_compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
  );
  return ManagedRevision3ProjectCompilerCheckReceipt(
    result: result,
    storeStillExactCurrent: kind != _ReceiptKind.postReadStoreDrift,
  );
}

Map<String, Object?> _seal(String digit, int bytes) => <String, Object?>{
  'byte_len': bytes,
  'sha256': List<String>.filled(64, digit).join(),
};

ScriptCompileInstallSafetyController _safety() =>
    ScriptCompileInstallSafetyController(
      (_) async => _safeInstall(),
      gameRoot: _gameRoot,
      autoRefresh: false,
    );

ScriptCompileInstallState _safeInstall() =>
    ScriptCompileInstallState.fromJson(<String, Object?>{
      'ok': true,
      'disposition': 'safe_to_compile',
      'safe_to_compile': true,
      'game_process': 'not_running',
      'artifacts': <Object?>[],
      'issues': <Object?>[],
    });

Future<void> _pumpDialog(
  WidgetTester tester, {
  required Revision3ProjectCompilerCheckpoint checkpoint,
  required Revision3ProjectCompilerCheckController controller,
  required ScriptCompileInstallSafetyController safety,
  required Revision3ProjectCompilerChecker check,
  double textScale = 1,
}) => tester.pumpWidget(
  ProviderScope(
    overrides: [
      scriptCompileInstallSafetyProvider.overrideWith((ref) => safety),
    ],
    child: MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(textScale)),
        child: child!,
      ),
      home: Scaffold(
        body: Revision3ProjectCompilerCheckDialog(
          controller: controller,
          checkpoint: checkpoint,
          gameRoot: _gameRoot,
          check: check,
        ),
      ),
    ),
  ),
);

Revision3TestReleaseCheck _idleCheck(String title) => Revision3TestReleaseCheck(
  state: Revision3TestReleaseCheckState.notEvaluated,
  title: title,
  description: 'Description',
);
