import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import '../l10n/app_localizations.dart';
import '../scripts/domain/script_compile_install_state_provider.dart';
import '../scripts/domain/script_compile_report.dart';
import 'current_project_controller.dart';
import 'managed_project_session.dart';
import 'revision3_test_release_workspace.dart';

typedef Revision3ProjectCompilerChecker =
    Future<ManagedRevision3ProjectCompilerCheckReceipt> Function({
      required String gameRoot,
      required ScriptCompilerBackendMode compilerBackend,
    });

@immutable
final class Revision3ProjectCompilerCheckpoint {
  Revision3ProjectCompilerCheckpoint({
    required String projectId,
    required this.projectRevision,
    required String checkpointIdentity,
  }) : projectId = _requiredText(projectId, 'projectId'),
       checkpointIdentity = _requiredText(
         checkpointIdentity,
         'checkpointIdentity',
       ) {
    if (projectRevision < 0) {
      throw ArgumentError.value(projectRevision, 'projectRevision');
    }
  }

  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;

  @override
  bool operator ==(Object other) =>
      other is Revision3ProjectCompilerCheckpoint &&
      other.projectId == projectId &&
      other.projectRevision == projectRevision &&
      other.checkpointIdentity == checkpointIdentity;

  @override
  int get hashCode =>
      Object.hash(projectId, projectRevision, checkpointIdentity);
}

enum Revision3ProjectCompilerOutcome {
  notChecked,
  unavailable,
  checking,
  compiled,
  empty,
  rejected,
  preflightBlocked,
  drifted,
  requiresReopen,
  recoveryRequired,
  safetyBlocked,
  failed,
}

@immutable
final class Revision3ProjectCompilerCheckSnapshot {
  const Revision3ProjectCompilerCheckSnapshot._({
    required this.checkpoint,
    required this.outcome,
    required this.attempted,
    this.receipt,
    this.error,
  });

  final Revision3ProjectCompilerCheckpoint checkpoint;
  final Revision3ProjectCompilerOutcome outcome;
  final bool attempted;
  final ManagedRevision3ProjectCompilerCheckReceipt? receipt;
  final Object? error;

  Revision3TestReleaseCheckState get checkState => switch (outcome) {
    Revision3ProjectCompilerOutcome.notChecked =>
      Revision3TestReleaseCheckState.notEvaluated,
    Revision3ProjectCompilerOutcome.unavailable =>
      Revision3TestReleaseCheckState.unavailable,
    Revision3ProjectCompilerOutcome.checking =>
      Revision3TestReleaseCheckState.checking,
    Revision3ProjectCompilerOutcome.compiled ||
    Revision3ProjectCompilerOutcome.empty =>
      Revision3TestReleaseCheckState.passed,
    Revision3ProjectCompilerOutcome.rejected =>
      Revision3TestReleaseCheckState.needsAttention,
    Revision3ProjectCompilerOutcome.preflightBlocked ||
    Revision3ProjectCompilerOutcome.drifted ||
    Revision3ProjectCompilerOutcome.requiresReopen ||
    Revision3ProjectCompilerOutcome.recoveryRequired ||
    Revision3ProjectCompilerOutcome.safetyBlocked ||
    Revision3ProjectCompilerOutcome.failed =>
      Revision3TestReleaseCheckState.blocked,
  };

  bool get isEvaluated => switch (checkState) {
    Revision3TestReleaseCheckState.passed ||
    Revision3TestReleaseCheckState.needsAttention ||
    Revision3TestReleaseCheckState.blocked => true,
    _ => false,
  };

  bool get isRunning => outcome == Revision3ProjectCompilerOutcome.checking;

  ScriptCompilerDiagnostics? get diagnostics =>
      receipt?.result.compiler.diagnostics;

  ScriptCompileFailure? get failure => receipt?.result.compiler.failure;

  String? get transportFailureDetail => switch (error) {
    ModFfiException(:final message) => message,
    _ => null,
  };

  String localizedDetail(AppLocalizations l10n) => switch (outcome) {
    Revision3ProjectCompilerOutcome.notChecked ||
    Revision3ProjectCompilerOutcome.checking =>
      l10n.managedTestReleaseScriptsDescription,
    Revision3ProjectCompilerOutcome.unavailable =>
      l10n.managedProjectCompilerNoGame,
    Revision3ProjectCompilerOutcome.compiled =>
      l10n.managedProjectCompilerCompiled,
    Revision3ProjectCompilerOutcome.empty => l10n.managedProjectCompilerEmpty,
    Revision3ProjectCompilerOutcome.rejected =>
      l10n.managedProjectCompilerRejected,
    Revision3ProjectCompilerOutcome.preflightBlocked =>
      l10n.managedProjectCompilerPreflightBlocked,
    Revision3ProjectCompilerOutcome.drifted =>
      l10n.managedProjectCompilerDrifted,
    Revision3ProjectCompilerOutcome.requiresReopen =>
      l10n.managedProjectCompilerRequiresReopen,
    Revision3ProjectCompilerOutcome.recoveryRequired =>
      l10n.managedProjectCompilerRecoveryRequired,
    Revision3ProjectCompilerOutcome.safetyBlocked =>
      l10n.managedProjectCompilerSafetyBlocked,
    Revision3ProjectCompilerOutcome.failed => l10n.managedProjectCompilerFailed,
  };

  Revision3TestReleaseCheck toTestReleaseCheck({
    required AppLocalizations l10n,
    required VoidCallback? onPressed,
  }) {
    final state = checkState;
    final evidence = isEvaluated
        ? Revision3TestReleaseEvidence(
            projectId: checkpoint.projectId,
            projectRevision: checkpoint.projectRevision,
            checkpointIdentity: checkpoint.checkpointIdentity,
            scope: Revision3TestReleaseEvidenceScope.scripts,
            summary: localizedDetail(l10n),
          )
        : null;
    return Revision3TestReleaseCheck(
      state: state,
      title: l10n.managedTestReleaseScriptsTitle,
      description: localizedDetail(l10n),
      evidence: evidence,
      actionLabel: isRunning
          ? l10n.managedTestReleaseStatusChecking
          : attempted
          ? l10n.managedProjectCompilerReviewAction
          : l10n.managedTestReleaseScriptsAction,
      onPressed: onPressed,
    );
  }
}

/// Owns only bounded compiler-check evidence. It cannot produce build or
/// deployment capabilities.
final class Revision3ProjectCompilerCheckController extends ChangeNotifier {
  Revision3ProjectCompilerCheckController({
    required Revision3ProjectCompilerCheckpoint checkpoint,
    required String? gameRoot,
    required bool requiresReopen,
  }) : _checkpoint = checkpoint,
       _gameRoot = _normalizedRoot(gameRoot),
       _requiresReopen = requiresReopen,
       _snapshot = Revision3ProjectCompilerCheckSnapshot._(
         checkpoint: checkpoint,
         outcome: _initialOutcome(gameRoot, requiresReopen),
         attempted: false,
       );

  Revision3ProjectCompilerCheckpoint _checkpoint;
  String? _gameRoot;
  bool _requiresReopen;
  Revision3ProjectCompilerCheckSnapshot _snapshot;
  int _generation = 0;
  bool _disposed = false;

  Revision3ProjectCompilerCheckSnapshot get snapshot => _snapshot;

  bool get canRun =>
      !_disposed &&
      _gameRoot != null &&
      !_requiresReopen &&
      _snapshot.outcome != Revision3ProjectCompilerOutcome.checking;

  void synchronize({
    required Revision3ProjectCompilerCheckpoint checkpoint,
    required String? gameRoot,
    required bool requiresReopen,
  }) {
    final normalizedGameRoot = _normalizedRoot(gameRoot);
    if (_checkpoint == checkpoint &&
        _gameRoot == normalizedGameRoot &&
        _requiresReopen == requiresReopen) {
      return;
    }
    _generation++;
    _checkpoint = checkpoint;
    _gameRoot = normalizedGameRoot;
    _requiresReopen = requiresReopen;
    _publish(
      Revision3ProjectCompilerCheckSnapshot._(
        checkpoint: checkpoint,
        outcome: _initialOutcome(normalizedGameRoot, requiresReopen),
        attempted: false,
      ),
    );
  }

  Future<void> run({
    required Revision3ProjectCompilerCheckpoint checkpoint,
    required Future<ManagedRevision3ProjectCompilerCheckReceipt> Function()
    operation,
  }) async {
    if (_disposed || !canRun || checkpoint != _checkpoint) return;
    final generation = ++_generation;
    _publish(
      Revision3ProjectCompilerCheckSnapshot._(
        checkpoint: checkpoint,
        outcome: Revision3ProjectCompilerOutcome.checking,
        attempted: true,
      ),
    );
    try {
      final receipt = await operation();
      if (!_isCurrent(generation, checkpoint)) return;
      _publish(_snapshotForReceipt(checkpoint, receipt));
    } catch (error) {
      if (!_isCurrent(generation, checkpoint)) return;
      _publish(
        Revision3ProjectCompilerCheckSnapshot._(
          checkpoint: checkpoint,
          outcome: _outcomeForError(error),
          attempted: true,
          error: error,
        ),
      );
    }
  }

  bool _isCurrent(
    int generation,
    Revision3ProjectCompilerCheckpoint checkpoint,
  ) => !_disposed && generation == _generation && checkpoint == _checkpoint;

  Revision3ProjectCompilerCheckSnapshot _snapshotForReceipt(
    Revision3ProjectCompilerCheckpoint checkpoint,
    ManagedRevision3ProjectCompilerCheckReceipt receipt,
  ) {
    final result = receipt.result;
    final compiler = result.compiler;
    final diagnostics = compiler.diagnostics;
    final sameCheckpoint =
        result.project.id == checkpoint.projectId &&
        result.project.revision == checkpoint.projectRevision &&
        result.head.canonicalJson == checkpoint.checkpointIdentity;
    if (!sameCheckpoint) {
      return Revision3ProjectCompilerCheckSnapshot._(
        checkpoint: checkpoint,
        outcome: Revision3ProjectCompilerOutcome.drifted,
        attempted: true,
      );
    }
    final preflightBlocked =
        result.closingAudit.store ==
            AuthoringRevision3ProjectCompilerClosingAuditStatus.notRun ||
        result.closingAudit.game ==
            AuthoringRevision3ProjectCompilerClosingAuditStatus.notRun ||
        compiler.runCount == 0 ||
        (diagnostics == null &&
            compiler.outputDisposition ==
                AuthoringRevision3ProjectCompilerOutputDisposition.notCreated);
    final attemptedRestoreIsSafe = switch (compiler.backend) {
      null =>
        compiler.installRestore == ScriptCompileInstallRestore.restoredExact,
      final backend when backend.gameAttempted =>
        compiler.installRestore == ScriptCompileInstallRestore.restoredExact,
      _ => compiler.installRestore == ScriptCompileInstallRestore.notStarted,
    };
    final cleanCompilerRejection =
        compiler.runCount > 0 &&
        attemptedRestoreIsSafe &&
        compiler.failure?.code == 'COMPILER_REGEN_FAILED' &&
        diagnostics != null &&
        (diagnostics.capture == ScriptCompileCaptureDisposition.captured ||
            diagnostics.capture ==
                ScriptCompileCaptureDisposition.unavailableFallback) &&
        (compiler.outputDisposition ==
                AuthoringRevision3ProjectCompilerOutputDisposition.discarded ||
            compiler.outputDisposition ==
                AuthoringRevision3ProjectCompilerOutputDisposition.notCreated);
    final outcome = compiler.recoveryRequired
        ? Revision3ProjectCompilerOutcome.recoveryRequired
        : !receipt.storeStillExactCurrent ||
              result.closingAudit.storeRequiresReopen
        ? Revision3ProjectCompilerOutcome.requiresReopen
        : compiler.compiledEvidenceOnly && receipt.exactCurrent
        ? Revision3ProjectCompilerOutcome.compiled
        : compiler.notNeededEmpty && receipt.exactCurrent
        ? Revision3ProjectCompilerOutcome.empty
        : preflightBlocked
        ? Revision3ProjectCompilerOutcome.preflightBlocked
        : !receipt.exactCurrent
        ? Revision3ProjectCompilerOutcome.drifted
        : cleanCompilerRejection
        ? Revision3ProjectCompilerOutcome.rejected
        : Revision3ProjectCompilerOutcome.failed;
    return Revision3ProjectCompilerCheckSnapshot._(
      checkpoint: checkpoint,
      outcome: outcome,
      attempted: true,
      receipt: receipt,
    );
  }

  void _publish(Revision3ProjectCompilerCheckSnapshot snapshot) {
    _snapshot = snapshot;
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _generation++;
    super.dispose();
  }
}

Future<void> showRevision3ProjectCompilerCheckDialog(
  BuildContext context, {
  required Revision3ProjectCompilerCheckController controller,
  required Revision3ProjectCompilerCheckpoint checkpoint,
  required String? gameRoot,
  required Revision3ProjectCompilerChecker check,
}) => showDialog<void>(
  context: context,
  barrierDismissible: false,
  builder: (_) => Revision3ProjectCompilerCheckDialog(
    controller: controller,
    checkpoint: checkpoint,
    gameRoot: gameRoot,
    check: check,
  ),
);

class Revision3ProjectCompilerCheckDialog extends ConsumerStatefulWidget {
  const Revision3ProjectCompilerCheckDialog({
    required this.controller,
    required this.checkpoint,
    required this.gameRoot,
    required this.check,
    super.key,
  });

  final Revision3ProjectCompilerCheckController controller;
  final Revision3ProjectCompilerCheckpoint checkpoint;
  final String? gameRoot;
  final Revision3ProjectCompilerChecker check;

  @override
  ConsumerState<Revision3ProjectCompilerCheckDialog> createState() =>
      _Revision3ProjectCompilerCheckDialogState();
}

class _Revision3ProjectCompilerCheckDialogState
    extends ConsumerState<Revision3ProjectCompilerCheckDialog> {
  ScriptCompilerBackendMode _backend = ScriptCompilerBackendMode.productDefault;

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: widget.controller,
    builder: (context, _) {
      final l10n = AppLocalizations.of(context);
      final snapshot = widget.controller.snapshot;
      final safety = ref.watch(scriptCompileInstallSafetyProvider);
      final running = snapshot.isRunning;
      final requiresGameSafety =
          _backend != ScriptCompilerBackendMode.standalone;
      return PopScope(
        canPop: !running,
        child: AlertDialog(
          key: const Key('revision3-project-compiler-dialog'),
          title: Text(l10n.managedProjectCompilerDialogTitle),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 720, maxHeight: 620),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(l10n.managedProjectCompilerDialogIntroduction),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<ScriptCompilerBackendMode>(
                    key: const Key('revision3-project-compiler-backend'),
                    initialValue: _backend,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      labelText: 'Compiler backend',
                      border: OutlineInputBorder(),
                    ),
                    items: [
                      for (final backend in ScriptCompilerBackendMode.values)
                        DropdownMenuItem(
                          value: backend,
                          child: Text(backend.label),
                        ),
                    ],
                    onChanged: running
                        ? null
                        : (backend) {
                            if (backend == null) return;
                            setState(() => _backend = backend);
                          },
                  ),
                  const SizedBox(height: 6),
                  Text(
                    switch (_backend) {
                      ScriptCompilerBackendMode.standaloneThenGame =>
                        'Mod Studio tries the qualified standalone compiler first. If it cannot produce an accepted result, the reason stays visible and the game compiler is used as fallback.',
                      ScriptCompilerBackendMode.game =>
                        'This route uses the game compiler and requires an exact, safe installation.',
                      ScriptCompilerBackendMode.standalone =>
                        'This route never starts the game or mutates the installation. It returns native file, line, column, severity, and message diagnostics, and fails closed unless a qualified standalone package is available.',
                    },
                    key: const Key(
                      'revision3-project-compiler-backend-description',
                    ),
                  ),
                  if (requiresGameSafety && safety.showBlockingBanner) ...[
                    const SizedBox(height: 12),
                    _MessageCard(
                      key: const Key(
                        'revision3-project-compiler-safety-message',
                      ),
                      icon: Icons.shield_outlined,
                      color: Theme.of(context).colorScheme.error,
                      title: l10n.managedTestReleaseStatusBlocked,
                      message: l10n.managedProjectCompilerSafetyBlocked,
                    ),
                  ],
                  if (snapshot.outcome !=
                      Revision3ProjectCompilerOutcome.notChecked) ...[
                    const SizedBox(height: 12),
                    _OutcomeView(snapshot: snapshot),
                  ],
                ],
              ),
            ),
          ),
          actionsOverflowAlignment: OverflowBarAlignment.end,
          actionsOverflowButtonSpacing: 8,
          actions: [
            TextButton(
              key: const Key('revision3-project-compiler-close'),
              onPressed: running ? null : () => Navigator.pop(context),
              child: Text(l10n.managedProjectCompilerCloseAction),
            ),
            FilledButton.icon(
              key: const Key('revision3-project-compiler-run'),
              onPressed: running || !widget.controller.canRun
                  ? null
                  : () => unawaited(_run(ref)),
              icon: running
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.fact_check_outlined),
              label: Text(
                running
                    ? l10n.managedTestReleaseStatusChecking
                    : snapshot.attempted
                    ? l10n.managedProjectCompilerRetryAction
                    : l10n.managedTestReleaseScriptsAction,
              ),
            ),
          ],
        ),
      );
    },
  );

  Future<void> _run(WidgetRef ref) {
    final attemptedGameRoot = widget.gameRoot;
    if (attemptedGameRoot == null || attemptedGameRoot.isEmpty) {
      return Future<void>.value();
    }
    final attemptedBackend = _backend;
    final requiresGameSafety =
        attemptedBackend != ScriptCompilerBackendMode.standalone;
    final safetyController = ref.read(
      scriptCompileInstallSafetyProvider.notifier,
    );
    return widget.controller.run(
      checkpoint: widget.checkpoint,
      operation: () async {
        if (requiresGameSafety) {
          final checked = await safetyController.refresh();
          if (!_sameRoot(checked.gameRoot, attemptedGameRoot) ||
              !checked.liveMutationAllowed) {
            throw const Revision3ProjectCompilerSafetyBlockedException();
          }
        }
        ManagedRevision3ProjectCompilerCheckReceipt? receipt;
        try {
          receipt = await widget.check(
            gameRoot: attemptedGameRoot,
            compilerBackend: attemptedBackend,
          );
          if (requiresGameSafety && receipt.gameInstallRecoveryRequired) {
            final compiler = receipt.result.compiler;
            final failure = compiler.failure!;
            safetyController.recordManagedRecovery(
              gameRoot: attemptedGameRoot,
              code: failure.code,
              message: failure.message,
              installRestore: compiler.installRestore,
            );
          }
          return receipt;
        } finally {
          if (requiresGameSafety &&
              receipt?.gameInstallRecoveryRequired != true &&
              _sameRoot(safetyController.current.gameRoot, attemptedGameRoot)) {
            await safetyController.refresh();
          }
        }
      },
    );
  }
}

class _OutcomeView extends StatelessWidget {
  const _OutcomeView({required this.snapshot});

  final Revision3ProjectCompilerCheckSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final state = snapshot.checkState;
    final visual = switch (state) {
      Revision3TestReleaseCheckState.checking => (
        Icons.sync_outlined,
        Theme.of(context).colorScheme.primary,
        l10n.managedTestReleaseStatusChecking,
      ),
      Revision3TestReleaseCheckState.passed => (
        Icons.verified_outlined,
        Colors.green,
        l10n.managedTestReleaseStatusChecked,
      ),
      Revision3TestReleaseCheckState.needsAttention => (
        Icons.code_off_outlined,
        Theme.of(context).colorScheme.error,
        l10n.managedTestReleaseStatusNeedsAttention,
      ),
      Revision3TestReleaseCheckState.unavailable => (
        Icons.videogame_asset_off_outlined,
        Theme.of(context).colorScheme.onSurfaceVariant,
        l10n.managedTestReleaseStatusNotAvailable,
      ),
      _ => (
        Icons.block_outlined,
        Theme.of(context).colorScheme.error,
        l10n.managedTestReleaseStatusBlocked,
      ),
    };
    final diagnostics = snapshot.diagnostics;
    final failure = snapshot.failure;
    final backend = snapshot.receipt?.result.compiler.backend;
    final failureDetail = failure?.message ?? snapshot.transportFailureDetail;
    return Semantics(
      container: true,
      liveRegion: true,
      child: Column(
        key: const Key('revision3-project-compiler-result'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _MessageCard(
            icon: visual.$1,
            color: visual.$2,
            title: visual.$3,
            message: snapshot.localizedDetail(l10n),
          ),
          if (backend != null) ...[
            const SizedBox(height: 12),
            SelectableText(
              _compilerBackendSummary(backend),
              key: const Key('revision3-project-compiler-backend-result'),
            ),
          ],
          if (failureDetail != null) ...[
            const SizedBox(height: 12),
            Text(
              l10n.managedProjectCompilerFailureDetails,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 4),
            SelectableText(
              failureDetail,
              key: const Key('revision3-project-compiler-failure'),
            ),
          ],
          if (diagnostics != null) ...[
            const SizedBox(height: 16),
            Text(
              l10n.managedProjectCompilerDiagnosticsHeading,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 4),
            Text(
              _localizedCapture(l10n, diagnostics.capture),
              key: const Key('revision3-project-compiler-capture'),
            ),
            const SizedBox(height: 6),
            for (final diagnostic in diagnostics.messages.take(50))
              _DiagnosticTile(diagnostic: diagnostic),
            if (diagnostics.omitted > 0 || diagnostics.messages.length > 50)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  '${diagnostics.omitted + (diagnostics.messages.length - 50).clamp(0, diagnostics.messages.length)} ${l10n.managedProjectCompilerOmittedDiagnostics}',
                  key: const Key(
                    'revision3-project-compiler-diagnostics-omitted',
                  ),
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class _MessageCard extends StatelessWidget {
  const _MessageCard({
    required this.icon,
    required this.color,
    required this.title,
    required this.message,
    super.key,
  });

  final IconData icon;
  final Color color;
  final String title;
  final String message;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    child: ListTile(
      leading: Icon(icon, color: color),
      title: Text(title),
      subtitle: Text(message),
    ),
  );
}

class _DiagnosticTile extends StatelessWidget {
  const _DiagnosticTile({required this.diagnostic});

  final ScriptCompilerDiagnostic diagnostic;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final severity = switch (diagnostic.severity) {
      ScriptCompilerDiagnosticSeverity.error =>
        l10n.managedProjectCompilerSeverityError,
      ScriptCompilerDiagnosticSeverity.warning =>
        l10n.managedProjectCompilerSeverityWarning,
      ScriptCompilerDiagnosticSeverity.note =>
        l10n.managedProjectCompilerSeverityNote,
    };
    final location = <String>[
      if (diagnostic.file.isNotEmpty)
        '${l10n.managedProjectCompilerFileLabel}: ${diagnostic.file}',
      '${l10n.managedProjectCompilerLineLabel}: ${diagnostic.line}',
      if (diagnostic.column > 0)
        '${l10n.managedProjectCompilerColumnLabel}: ${diagnostic.column}',
      severity,
    ].join(' | ');
    return Card(
      key: ObjectKey(diagnostic),
      margin: const EdgeInsets.only(top: 6),
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(location, style: Theme.of(context).textTheme.labelMedium),
            const SizedBox(height: 4),
            SelectableText(diagnostic.message),
          ],
        ),
      ),
    );
  }
}

final class Revision3ProjectCompilerSafetyBlockedException
    implements Exception {
  const Revision3ProjectCompilerSafetyBlockedException();
}

String _localizedCapture(
  AppLocalizations l10n,
  ScriptCompileCaptureDisposition capture,
) => switch (capture) {
  ScriptCompileCaptureDisposition.captured =>
    l10n.managedProjectCompilerCaptureCaptured,
  ScriptCompileCaptureDisposition.unavailableFallback =>
    l10n.managedProjectCompilerCaptureFallback,
  ScriptCompileCaptureDisposition.captureInvalid =>
    l10n.managedProjectCompilerCaptureInvalid,
  ScriptCompileCaptureDisposition.unavailableWithoutFallback =>
    l10n.managedProjectCompilerCaptureUnavailable,
  ScriptCompileCaptureDisposition.processExitUnconfirmed =>
    l10n.managedProjectCompilerCaptureExitUnconfirmed,
  ScriptCompileCaptureDisposition.disabled =>
    l10n.managedProjectCompilerCaptureDisabled,
};

String _compilerBackendSummary(ScriptCompilerBackendEvidence evidence) {
  final result = switch (evidence.resultBackend) {
    ScriptCompilerBackendName.game => 'Game compiler',
    ScriptCompilerBackendName.standalone => 'Standalone compiler',
    null => 'No compiler backend ran',
  };
  final fallback = evidence.fallbackReason;
  if (fallback == null) {
    return 'Requested: ${evidence.requestedMode.label}. Result: $result.';
  }
  return 'Requested: ${evidence.requestedMode.label}. Result: $result. '
      'Standalone fallback: ${fallback.detail}';
}

Revision3ProjectCompilerOutcome _outcomeForError(Object error) =>
    switch (error) {
      Revision3ProjectCompilerCheckStaleCheckpointException() =>
        Revision3ProjectCompilerOutcome.drifted,
      Revision3ProjectCompilerCheckRequiresReopenException() =>
        Revision3ProjectCompilerOutcome.requiresReopen,
      Revision3ProjectCompilerSafetyBlockedException() =>
        Revision3ProjectCompilerOutcome.safetyBlocked,
      ModFfiException(
        code: 'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_DRIFT',
      ) =>
        Revision3ProjectCompilerOutcome.drifted,
      ModFfiException(
        code: 'AUTHORING_REVISION3_PROJECT_COMPILER_RECOVERY_REQUIRED',
      ) =>
        Revision3ProjectCompilerOutcome.recoveryRequired,
      ModFfiException(:final code)
          when _revision3ProjectCompilerPreflightCodes.contains(code) =>
        Revision3ProjectCompilerOutcome.preflightBlocked,
      _ => Revision3ProjectCompilerOutcome.failed,
    };

const _revision3ProjectCompilerPreflightCodes = <String>{
  'AUTHORING_REVISION3_PROJECT_COMPILER_INPUT_LIMIT',
  'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_INVALID',
  'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_PROJECT_COMPILER_GAME_MISMATCH',
  'AUTHORING_REVISION3_PROJECT_COMPILER_INSTALL_UNAVAILABLE',
  'AUTHORING_REVISION3_PROJECT_COMPILER_STAGING_UNAVAILABLE',
  'AUTHORING_REVISION3_PROJECT_COMPILER_UNSUPPORTED_GENERATION',
};

Revision3ProjectCompilerOutcome _initialOutcome(
  String? gameRoot,
  bool requiresReopen,
) => gameRoot == null || gameRoot.isEmpty
    ? Revision3ProjectCompilerOutcome.unavailable
    : requiresReopen
    ? Revision3ProjectCompilerOutcome.requiresReopen
    : Revision3ProjectCompilerOutcome.notChecked;

String? _normalizedRoot(String? root) =>
    root == null || root.isEmpty ? null : p.normalize(p.absolute(root));

bool _sameRoot(String? left, String right) =>
    left != null &&
    p.equals(p.normalize(p.absolute(left)), p.normalize(p.absolute(right)));

String _requiredText(String value, String name) {
  if (value.isEmpty) throw ArgumentError.value(value, name);
  return value;
}
