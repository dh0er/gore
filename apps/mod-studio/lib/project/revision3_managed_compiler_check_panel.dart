import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import '../scripts/domain/script_compile_install_state_provider.dart';
import '../scripts/domain/script_compile_report.dart';
import '../scripts/ui/script_compile_install_state_banner.dart';
import 'current_project_controller.dart';
import 'managed_project_session.dart';

typedef Revision3ManagedCompilerChecker =
    Future<ManagedRevision3CompilerCheckReceipt> Function();

typedef Revision3ManagedCompilerPublisher =
    Future<ManagedRevision3CompilerCheckReceipt> Function({
      required AuthoringRevision3ManagedCompilerEntityKind entityKind,
      required String entityId,
      required int expectedEntityRevision,
      required String expectedModuleId,
      required int expectedModuleRevision,
      required String gameRoot,
    });

/// Evidence-only compiler check for the exact Quest/NPC source shown by its
/// parent dialog. The temporary compiler output is discarded and can never be
/// adopted as a build or deploy input.
class Revision3ManagedCompilerCheckPanel extends ConsumerStatefulWidget {
  const Revision3ManagedCompilerCheckPanel({
    required this.gameRoot,
    required this.check,
    this.onAcceptanceChanged,
    this.onBusyChanged,
    super.key,
  });

  final String gameRoot;
  final Revision3ManagedCompilerChecker check;
  final ValueChanged<bool>? onAcceptanceChanged;
  final ValueChanged<bool>? onBusyChanged;

  @override
  ConsumerState<Revision3ManagedCompilerCheckPanel> createState() =>
      _Revision3ManagedCompilerCheckPanelState();
}

class _Revision3ManagedCompilerCheckPanelState
    extends ConsumerState<Revision3ManagedCompilerCheckPanel> {
  bool _busy = false;
  ManagedRevision3CompilerCheckReceipt? _receipt;
  Object? _error;

  @override
  Widget build(BuildContext context) {
    final safety = ref.watch(scriptCompileInstallSafetyProvider);
    final legacyRecoveryReport = safety.recoveryReport;
    return Column(
      key: const Key('revision3-managed-compiler-check-panel'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Game compiler check',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 6),
        const Text(
          'Check this exact saved source with the game compiler. The result is evidence only: no compiled output is kept, and build, runtime, and deploy remain blocked.',
        ),
        if (safety.showBlockingBanner) ...[
          const SizedBox(height: 8),
          ScriptCompileInstallStateBanner(
            state: safety,
            onRecheck: () => unawaited(
              ref.read(scriptCompileInstallSafetyProvider.notifier).refresh(),
            ),
            onViewRecoveryReport: legacyRecoveryReport == null
                ? null
                : () => showScriptCompileReportDialog(
                    context,
                    legacyRecoveryReport,
                  ),
          ),
        ],
        const SizedBox(height: 10),
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.icon(
            key: const Key('revision3-managed-compiler-check-run'),
            onPressed: _busy ? null : _run,
            icon: _busy
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.fact_check_outlined),
            label: Text(_busy ? 'Checking…' : 'Check with game compiler'),
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: 10),
          Semantics(
            container: true,
            liveRegion: true,
            child: _CompilerOutcomeCard(
              key: const Key('revision3-managed-compiler-check-error'),
              icon: Icons.error_outline,
              color: Theme.of(context).colorScheme.error,
              title: 'Compiler check could not be completed',
              body:
                  'No compiler output was adopted. ${_friendlyCompilerCheckError(_error!)}',
            ),
          ),
        ],
        if (_receipt != null) ...[
          const SizedBox(height: 10),
          Semantics(
            container: true,
            liveRegion: true,
            child: _ManagedCompilerReceiptView(receipt: _receipt!),
          ),
        ],
      ],
    );
  }

  Future<void> _run() async {
    final onBusyChanged = widget.onBusyChanged;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('revision3-managed-compiler-confirmation'),
        title: const Text('Check with the game compiler?'),
        content: const Text(
          'Close Gothic 1 Remake first. Mod Studio will temporarily install only this generated source, run the compiler, restore every touched game path, and discard the compiler output. Your save is not loaded or changed. If exact restoration cannot be proven, all further live changes stay blocked.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('revision3-managed-compiler-confirm'),
            onPressed: () {
              onBusyChanged?.call(true);
              Navigator.pop(dialogContext, true);
            },
            child: const Text('Check source'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    try {
      if (!mounted) return;

      setState(() {
        _busy = true;
        _receipt = null;
        _error = null;
      });
      widget.onAcceptanceChanged?.call(false);

      ManagedRevision3CompilerCheckReceipt? receipt;
      Object? error;
      final attemptedGameRoot = widget.gameRoot;
      final safetyController = ref.read(
        scriptCompileInstallSafetyProvider.notifier,
      );
      try {
        final checked = await safetyController.refresh();
        if (!_sameRoot(checked.gameRoot, attemptedGameRoot) ||
            !checked.liveMutationAllowed) {
          throw StateError(
            'The selected game installation is not currently safe to check. Close the game, resolve any recovery warning, and choose Recheck.',
          );
        }
        receipt = await widget.check();
        final compiler = receipt.result.compiler;
        if (compiler.recoveryRequired) {
          final failure = compiler.failure;
          safetyController.recordManagedRecovery(
            gameRoot: attemptedGameRoot,
            code: failure?.code ?? 'COMPILE_INSTALL_RECOVERY_REQUIRED',
            message:
                failure?.message ??
                'Exact restoration of the game installation could not be proven.',
            installRestore: compiler.installRestore,
          );
        } else if (_sameRoot(
          safetyController.current.gameRoot,
          attemptedGameRoot,
        )) {
          await safetyController.refresh();
        }
      } catch (caught) {
        error = caught;
        if (_sameRoot(safetyController.current.gameRoot, attemptedGameRoot)) {
          await safetyController.refresh();
        }
      }
      if (!mounted) return;
      setState(() {
        _busy = false;
        _receipt = receipt;
        _error = error;
      });
      widget.onAcceptanceChanged?.call(receipt?.acceptedAtExactCurrent == true);
    } finally {
      onBusyChanged?.call(false);
    }
  }
}

class _ManagedCompilerReceiptView extends StatelessWidget {
  const _ManagedCompilerReceiptView({required this.receipt});

  final ManagedRevision3CompilerCheckReceipt receipt;

  @override
  Widget build(BuildContext context) {
    final compiler = receipt.result.compiler;
    final failure = compiler.failure;
    final diagnostics = compiler.diagnostics;
    final accepted = receipt.acceptedAtExactCurrent;
    final recovery = receipt.recoveryRequired;
    final compilerRejected =
        diagnostics?.messages.any(
          (message) =>
              message.severity == ScriptCompilerDiagnosticSeverity.error,
        ) ==
        true;
    late final String title;
    late final String body;
    late final IconData icon;
    late final Color color;
    if (recovery) {
      title = 'Game installation recovery required';
      body =
          'The attempt did not prove exact restoration. Further compiler and deploy mutations are blocked until Recheck proves the installation safe.';
      icon = Icons.restore_outlined;
      color = Theme.of(context).colorScheme.error;
    } else if (accepted) {
      title = 'Exact source accepted by the compiler';
      body =
          'The compiler accepted this module at the exact project checkpoint. Its output was discarded; build, runtime, deploy, and publication remain blocked.';
      icon = Icons.verified_outlined;
      color = Colors.green;
    } else if (compiler.compiledEvidenceOnly) {
      title = 'Compiler result is no longer current';
      body =
          'The compiler accepted the attempted source, but the project changed before exact-current evidence could be closed. Close this dialog and refresh before checking again.';
      icon = Icons.history_outlined;
      color = Colors.orange;
    } else if (!compiler.outputDiscarded) {
      title = 'Compiler output cleanup failed';
      body =
          '${failure?.message ?? 'The compiler check failed.'} Exact disposal of compiler output was not proven, so no artifact is authorized for build or deploy.';
      icon = Icons.delete_forever_outlined;
      color = Theme.of(context).colorScheme.error;
    } else if (compiler.installRestore ==
            ScriptCompileInstallRestore.notStarted &&
        diagnostics == null) {
      title = 'Compiler check did not run';
      body =
          failure?.message ??
          'The game compiler was not started for this source.';
      icon = Icons.not_started_outlined;
      color = Theme.of(context).colorScheme.error;
    } else if (compilerRejected) {
      title = 'Compiler rejected this source';
      body =
          failure?.message ??
          'The compiler rejected the selected generated source.';
      icon = Icons.code_off_outlined;
      color = Theme.of(context).colorScheme.error;
    } else {
      title = 'Compiler check failed';
      body =
          failure?.message ?? 'The compiler check did not accept this source.';
      icon = Icons.error_outline;
      color = Theme.of(context).colorScheme.error;
    }

    return Column(
      key: const Key('revision3-managed-compiler-check-result'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _CompilerOutcomeCard(
          icon: icon,
          color: color,
          title: title,
          body: body,
        ),
        if (failure != null) ...[
          const SizedBox(height: 6),
          SelectableText(
            '${failure.code}: ${failure.message}',
            key: const Key('revision3-managed-compiler-failure'),
          ),
        ],
        if (diagnostics != null) ...[
          const SizedBox(height: 8),
          Text(
            _captureSummary(diagnostics),
            key: const Key('revision3-managed-compiler-capture-summary'),
          ),
          for (final diagnostic in diagnostics.messages.take(100))
            _CompilerDiagnosticTile(diagnostic: diagnostic),
          if (diagnostics.messages.length > 100 || diagnostics.omitted > 0)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                '${(diagnostics.messages.length > 100 ? diagnostics.messages.length - 100 : 0) + diagnostics.omitted} additional diagnostic(s) omitted from this view.',
              ),
            ),
        ],
      ],
    );
  }
}

class _CompilerOutcomeCard extends StatelessWidget {
  const _CompilerOutcomeCard({
    required this.icon,
    required this.color,
    required this.title,
    required this.body,
    super.key,
  });

  final IconData icon;
  final Color color;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    child: ListTile(
      leading: Icon(icon, color: color),
      title: Text(title),
      subtitle: Text(body),
    ),
  );
}

class _CompilerDiagnosticTile extends StatelessWidget {
  const _CompilerDiagnosticTile({required this.diagnostic});

  final ScriptCompilerDiagnostic diagnostic;

  @override
  Widget build(BuildContext context) {
    final color = switch (diagnostic.severity) {
      ScriptCompilerDiagnosticSeverity.error => Theme.of(
        context,
      ).colorScheme.error,
      ScriptCompilerDiagnosticSeverity.warning => Colors.orange,
      ScriptCompilerDiagnosticSeverity.note => Theme.of(
        context,
      ).colorScheme.onSurfaceVariant,
    };
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: Icon(Icons.terminal_outlined, color: color),
      title: SelectableText(diagnostic.message),
      subtitle: SelectableText(
        '${diagnostic.location} · ${diagnostic.severity.name}',
      ),
    );
  }
}

bool _sameRoot(String? left, String right) {
  if (left == null) return false;
  return p.equals(
    p.normalize(p.absolute(left)),
    p.normalize(p.absolute(right)),
  );
}

String _captureSummary(ScriptCompilerDiagnostics diagnostics) {
  final method = switch (diagnostics.capture) {
    ScriptCompileCaptureDisposition.captured => 'Compiler diagnostics captured',
    ScriptCompileCaptureDisposition.captureInvalid =>
      'Compiler diagnostic capture was invalid',
    ScriptCompileCaptureDisposition.unavailableFallback =>
      'Diagnostics hook unavailable; normal compiler fallback used',
    ScriptCompileCaptureDisposition.unavailableWithoutFallback =>
      'Diagnostics hook unavailable after the compiler attempt',
    ScriptCompileCaptureDisposition.processExitUnconfirmed =>
      'Compiler process exit was not confirmed',
    ScriptCompileCaptureDisposition.disabled =>
      'Structured compiler diagnostics were disabled',
  };
  return '$method · ${diagnostics.messages.length} message(s)';
}

String _friendlyCompilerCheckError(Object error) {
  if (error is Revision3ManagedCompilerCheckStaleCheckpointException) {
    return 'The selected Quest or NPC changed. Close this dialog, refresh the Content library, and try again.';
  }
  if (error is Revision3ManagedCompilerCheckRequiresReopenException) {
    return 'The managed project must be closed and reopened before another exact compiler check.';
  }
  if (error is ManagedRevision3CompilerSelectionStaleException) {
    return 'The selected source module is no longer the exact current module. Refresh the Content library and try again.';
  }
  if (error is ModFfiException) return error.message;
  final text = '$error';
  if (text.length <= 1024) return text;
  return '${text.substring(0, 1021)}...';
}
