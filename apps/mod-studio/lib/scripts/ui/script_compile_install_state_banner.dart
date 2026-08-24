import 'package:flutter/material.dart';

import '../domain/script_compile_install_state.dart';
import '../domain/script_compile_install_state_provider.dart';
import '../domain/script_compile_report.dart';

class ScriptCompileInstallStateBanner extends StatelessWidget {
  const ScriptCompileInstallStateBanner({
    required this.state,
    required this.onRecheck,
    this.onViewRecoveryReport,
    super.key,
  });

  final ScriptCompileInstallSafetyState state;
  final VoidCallback onRecheck;
  final VoidCallback? onViewRecoveryReport;

  @override
  Widget build(BuildContext context) {
    if (!state.showBlockingBanner) return const SizedBox.shrink();
    final theme = Theme.of(context);
    final loading = state.phase == ScriptCompileInstallSafetyPhase.loading;
    return Card(
      key: const Key('script-compile-install-state-banner'),
      margin: const EdgeInsets.all(8),
      color: theme.colorScheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              loading ? Icons.sync : Icons.report_gmailerrorred_outlined,
              color: theme.colorScheme.onErrorContainer,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _headline(state),
                    key: const Key('script-compile-install-state-headline'),
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onErrorContainer,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _details(state),
                    key: const Key('script-compile-install-state-details'),
                    style: TextStyle(color: theme.colorScheme.onErrorContainer),
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 4,
                    children: [
                      OutlinedButton.icon(
                        key: const Key('script-compile-install-state-recheck'),
                        onPressed: loading ? null : onRecheck,
                        icon: const Icon(Icons.refresh, size: 18),
                        label: Text(loading ? 'Checking…' : 'Recheck'),
                      ),
                      if (state.recoveryReport != null &&
                          onViewRecoveryReport != null)
                        OutlinedButton.icon(
                          key: const Key(
                            'script-compile-install-state-view-report',
                          ),
                          onPressed: onViewRecoveryReport,
                          icon: const Icon(
                            Icons.receipt_long_outlined,
                            size: 18,
                          ),
                          label: const Text('View compiler report'),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Future<void> showScriptCompileReportDialog(
  BuildContext context,
  ScriptCompileReport report,
) => showDialog<void>(
  context: context,
  builder: (dialogContext) => AlertDialog(
    title: Text(report.compiled ? 'Compiler report' : 'Compilation failed'),
    content: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 760, maxHeight: 520),
      child: SingleChildScrollView(
        child: SelectableText(
          _formatCompileReport(report),
          style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(dialogContext),
        child: const Text('Close'),
      ),
    ],
  ),
);

String _formatCompileReport(ScriptCompileReport report) {
  final diagnostics = report.diagnostics;
  final lines = <String>[
    'Outcome: ${report.compiled ? 'compiled' : 'failed'}',
    'Game install: ${_restoreLabel(report.installRestore)}',
    if (report.outputRecoveryRequired)
      'Private compiler output: recovery required',
    if (diagnostics != null)
      'Diagnostics: ${_captureLabel(diagnostics.capture)}',
    if (report.failure != null) '',
    if (report.failure != null)
      '${report.failure!.code}: ${report.failure!.message}',
  ];
  if (diagnostics != null && diagnostics.messages.isNotEmpty) {
    lines.add('');
    for (final diagnostic in diagnostics.messages) {
      lines.add(
        '${diagnostic.location}: ${diagnostic.severity.name}: ${diagnostic.message}',
      );
    }
    if (diagnostics.omitted > 0) {
      lines.add('... ${diagnostics.omitted} additional diagnostic(s) omitted');
    }
  }
  return lines.join('\n');
}

String _restoreLabel(ScriptCompileInstallRestore restore) => switch (restore) {
  ScriptCompileInstallRestore.notStarted => 'not touched',
  ScriptCompileInstallRestore.restoredExact => 'restored exactly',
  ScriptCompileInstallRestore.recoveryRequiredProcessExitUnconfirmed =>
    'recovery required (compiler exit unconfirmed)',
  ScriptCompileInstallRestore.recoveryRequiredRestoreFailed =>
    'recovery required (restore failed)',
};

String _captureLabel(ScriptCompileCaptureDisposition capture) =>
    switch (capture) {
      ScriptCompileCaptureDisposition.captured => 'captured',
      ScriptCompileCaptureDisposition.captureInvalid => 'capture invalid',
      ScriptCompileCaptureDisposition.unavailableFallback =>
        'hook unavailable; normal compiler fallback used',
      ScriptCompileCaptureDisposition.unavailableWithoutFallback =>
        'hook unavailable after compiler exit; no rerun needed',
      ScriptCompileCaptureDisposition.processExitUnconfirmed =>
        'compiler exit unconfirmed',
      ScriptCompileCaptureDisposition.disabled => 'disabled',
    };

String _headline(ScriptCompileInstallSafetyState state) {
  if (state.recoveryRequired) {
    return 'Game installation recovery required';
  }
  if (state.phase == ScriptCompileInstallSafetyPhase.loading) {
    return 'Checking game installation safety';
  }
  if (state.phase == ScriptCompileInstallSafetyPhase.failed) {
    return 'Game installation safety could not be verified';
  }
  return switch (state.installState?.disposition) {
    ScriptCompileInstallDisposition.gameProcessRunning =>
      'Gothic 1 Remake is still running',
    ScriptCompileInstallDisposition.recoveryArtifactsPresent =>
      'Compiler recovery files are present',
    ScriptCompileInstallDisposition.inspectionFailed =>
      'Game installation inspection failed',
    _ => 'Game installation mutation is blocked',
  };
}

String _details(ScriptCompileInstallSafetyState state) {
  final lines = <String>[];
  final recovery = state.recoveryEvidence;
  if (recovery != null) {
    lines.add(
      'Mod Studio will not compile or deploy until a fresh native check proves exact recovery.',
    );
    lines.add('${recovery.code}: ${recovery.message}');
  } else if (state.phase == ScriptCompileInstallSafetyPhase.loading) {
    lines.add(
      'Compile and Deploy stay disabled until the read-only check completes.',
    );
  } else if (state.errorMessage != null) {
    lines.add(state.errorMessage!);
  }
  final install = state.installState;
  if (install?.gameProcess == ScriptCompileGameProcessState.running) {
    lines.add('Close the game completely, then choose Recheck.');
  }
  if (install != null) {
    for (final artifact in install.artifacts.take(3)) {
      lines.add('Recovery artifact: ${artifact.displayPath}');
    }
    if (install.artifacts.length > 3) {
      lines.add('${install.artifacts.length - 3} more recovery artifact(s).');
    }
    for (final issue in install.issues.take(2)) {
      final path = issue.displayPath;
      lines.add(path == null ? issue.message : '$path: ${issue.message}');
    }
    if (install.issues.length > 2) {
      lines.add('${install.issues.length - 2} more inspection issue(s).');
    }
  }
  if (lines.isEmpty) {
    lines.add('Choose Recheck before compiling or deploying.');
  }
  return lines.join('\n');
}
