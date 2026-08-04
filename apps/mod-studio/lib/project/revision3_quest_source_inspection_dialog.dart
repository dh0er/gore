import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/mod_ffi.dart';
import 'current_project_controller.dart';
import 'revision3_managed_compiler_check_panel.dart';

typedef Revision3QuestSourceInspectionLoader =
    Future<AuthoringRevision3QuestSourceInspectionResult> Function({
      required String gameRoot,
      required String questId,
    });

/// Read-only, non-technical presentation of one exact Quest source inspection.
///
/// A successful result proves source regeneration and its sealed inputs only.
/// It deliberately does not turn build, runtime, deployment, or publication
/// status into a positive claim.
class Revision3QuestSourceInspectionDialog extends StatefulWidget {
  const Revision3QuestSourceInspectionDialog({
    required this.questTitle,
    required this.questId,
    required this.gameRoot,
    required this.inspect,
    this.checkCompiler,
    super.key,
  });

  final String questTitle;
  final String questId;
  final String gameRoot;
  final Revision3QuestSourceInspectionLoader inspect;
  final Revision3ManagedCompilerChecker? checkCompiler;

  @override
  State<Revision3QuestSourceInspectionDialog> createState() =>
      _Revision3QuestSourceInspectionDialogState();
}

class _Revision3QuestSourceInspectionDialogState
    extends State<Revision3QuestSourceInspectionDialog> {
  late Future<AuthoringRevision3QuestSourceInspectionResult> _inspection;
  bool _compilerBusy = false;

  @override
  void initState() {
    super.initState();
    _inspection = _load();
  }

  Future<AuthoringRevision3QuestSourceInspectionResult> _load() =>
      widget.inspect(gameRoot: widget.gameRoot, questId: widget.questId);

  void _retry() {
    final next = _load();
    setState(() {
      _inspection = next;
    });
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_compilerBusy,
    child: AlertDialog(
      key: const Key('revision3-quest-source-inspection-dialog'),
      title: Text('Source & checks — ${widget.questTitle}'),
      content: SizedBox(
        width: 760,
        height: 620,
        child: FutureBuilder<AuthoringRevision3QuestSourceInspectionResult>(
          future: _inspection,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    CircularProgressIndicator(),
                    SizedBox(height: 16),
                    Text('Verifying the saved Quest and its source inputs…'),
                  ],
                ),
              );
            }
            final result = snapshot.data;
            if (result == null) {
              return _InspectionError(
                error: snapshot.error ?? StateError('inspection failed'),
                retry: _retry,
              );
            }
            return _InspectionResult(
              result: result,
              gameRoot: widget.gameRoot,
              checkCompiler: widget.checkCompiler,
              onCompilerBusyChanged: (busy) {
                if (!mounted || busy == _compilerBusy) return;
                setState(() => _compilerBusy = busy);
              },
            );
          },
        ),
      ),
      actions: [
        TextButton(
          key: const Key('revision3-quest-source-inspection-close'),
          onPressed: _compilerBusy ? null : () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    ),
  );
}

class _InspectionResult extends StatelessWidget {
  const _InspectionResult({
    required this.result,
    required this.gameRoot,
    required this.checkCompiler,
    required this.onCompilerBusyChanged,
  });

  final AuthoringRevision3QuestSourceInspectionResult result;
  final String gameRoot;
  final Revision3ManagedCompilerChecker? checkCompiler;
  final ValueChanged<bool> onCompilerBusyChanged;

  @override
  Widget build(BuildContext context) => ListView(
    key: const Key('revision3-quest-source-inspection-result'),
    children: [
      const _StatusCard(
        icon: Icons.verified_outlined,
        color: Colors.green,
        title: 'Saved source verified',
        body:
            'The script was regenerated from the saved Quest and matches the saved module exactly.',
      ),
      const SizedBox(height: 10),
      const _StatusCard(
        icon: Icons.inventory_2_outlined,
        color: Colors.green,
        title: 'Source inputs verified',
        body:
            'The collision evidence and installed game inputs were reopened and matched their sealed identities.',
      ),
      const SizedBox(height: 10),
      const _StatusCard(
        icon: Icons.lock_clock_outlined,
        color: Colors.green,
        title: 'Exact project version checked',
        body:
            'The project head was unchanged before and after this read-only inspection.',
      ),
      if (checkCompiler != null) ...[
        const SizedBox(height: 16),
        Revision3ManagedCompilerCheckPanel(
          gameRoot: gameRoot,
          check: checkCompiler!,
          onBusyChanged: onCompilerBusyChanged,
        ),
      ],
      const SizedBox(height: 16),
      Text(
        'What this does not prove',
        style: Theme.of(context).textTheme.titleMedium,
      ),
      const SizedBox(height: 8),
      if (checkCompiler == null)
        const _ClosedStatus(
          icon: Icons.code_off_outlined,
          title: 'Compilation was not run',
          body:
              'This view verifies generated source; it is not a compiler result.',
        ),
      const _ClosedStatus(
        icon: Icons.block_outlined,
        title: 'Build is still blocked',
        body: 'No build or deploy permission is granted by this inspection.',
      ),
      const _ClosedStatus(
        icon: Icons.sports_esports_outlined,
        title: 'In-game behavior is not qualified',
        body:
            'Runtime and publication support still require separate proven workflows.',
      ),
      const SizedBox(height: 16),
      ExpansionTile(
        key: const Key('revision3-quest-source-inspection-advanced'),
        tilePadding: EdgeInsets.zero,
        title: const Text('Advanced'),
        subtitle: const Text('Generated source, technical identity, and seals'),
        children: [
          _TechnicalValue(label: 'Module', value: result.moduleNamespace),
          _TechnicalValue(
            label: 'Source path',
            value: result.moduleRelativePath,
          ),
          _TechnicalValue(
            label: 'Quest ID',
            value: result.plan.module.quest.id,
          ),
          _TechnicalValue(
            label: 'Script module ID',
            value: result.plan.module.scriptModule.id,
          ),
          _TechnicalValue(
            label: 'Prior Quest evidence count',
            value: '${result.plan.provenance.collisionPriorQuestCount}',
          ),
          _TechnicalValue(
            label: 'Project SHA-256',
            value: result.projectSeal.sha256,
          ),
          _TechnicalValue(label: 'Plan SHA-256', value: result.planSeal.sha256),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: Text(
                  'Generated AngelScript',
                  style: Theme.of(context).textTheme.titleSmall,
                ),
              ),
              TextButton.icon(
                key: const Key('revision3-quest-source-copy'),
                onPressed: () async {
                  await Clipboard.setData(
                    ClipboardData(text: result.generatedSource),
                  );
                  if (!context.mounted) return;
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Generated source copied')),
                  );
                },
                icon: const Icon(Icons.copy_outlined),
                label: const Text('Copy source'),
              ),
            ],
          ),
          Container(
            key: const Key('revision3-quest-generated-source'),
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            color: Theme.of(context).colorScheme.surfaceContainerHighest,
            child: SelectableText(
              result.generatedSource,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
            ),
          ),
        ],
      ),
    ],
  );
}

class _InspectionError extends StatelessWidget {
  const _InspectionError({required this.error, required this.retry});

  final Object error;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) {
    final requiresRefresh = _requiresCloseAndRefresh(error);
    return Center(
      key: const Key('revision3-quest-source-inspection-error'),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.error_outline,
              size: 44,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: 12),
            Text(
              'Source verification could not be completed',
              style: Theme.of(context).textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(_friendlyError(error), textAlign: TextAlign.center),
            const SizedBox(height: 16),
            if (requiresRefresh)
              FilledButton.icon(
                key: const Key(
                  'revision3-quest-source-inspection-close-refresh',
                ),
                onPressed: () => Navigator.of(context).pop(),
                icon: const Icon(Icons.refresh),
                label: const Text('Close and refresh'),
              )
            else
              FilledButton.icon(
                key: const Key('revision3-quest-source-inspection-retry'),
                onPressed: retry,
                icon: const Icon(Icons.refresh),
                label: const Text('Try again'),
              ),
          ],
        ),
      ),
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    required this.icon,
    required this.color,
    required this.title,
    required this.body,
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

class _ClosedStatus extends StatelessWidget {
  const _ClosedStatus({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) => ListTile(
    dense: true,
    contentPadding: EdgeInsets.zero,
    leading: Icon(icon, color: Theme.of(context).colorScheme.onSurfaceVariant),
    title: Text(title),
    subtitle: Text(body),
  );
}

class _TechnicalValue extends StatelessWidget {
  const _TechnicalValue({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 3),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(width: 150, child: Text(label)),
        Expanded(child: SelectableText(value)),
      ],
    ),
  );
}

String _friendlyError(Object error) => switch (error) {
  Revision3QuestSourceInspectionStaleCheckpointException() =>
    'The project changed after this Quest was selected. Close this window, refresh the Library, and try again.',
  Revision3QuestSourceInspectionRequiresReopenException() =>
    'The project must be reopened before its source can be checked safely.',
  CurrentProjectOperationUnsupportedException(:final message) => message,
  ModFfiException(:final message) => message,
  _ =>
    'No project or game files were changed. You can retry after checking the configured game installation.',
};

bool _requiresCloseAndRefresh(Object error) =>
    error is Revision3QuestSourceInspectionStaleCheckpointException ||
    error is Revision3QuestSourceInspectionRequiresReopenException;
