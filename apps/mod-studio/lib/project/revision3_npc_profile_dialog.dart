import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/mod_ffi.dart';
import 'current_project_controller.dart';

typedef Revision3NpcSourceInspectionLoader =
    Future<AuthoringRevision3NpcSourceInspectionResult> Function({
      required String npcId,
    });

/// Read-only profile for one managed NPC Draft. A successful inspection proves
/// saved project/source consistency only; the four remaining production and
/// runtime blockers stay explicit.
class Revision3NpcProfileDialog extends StatefulWidget {
  const Revision3NpcProfileDialog({
    required this.npcTitle,
    required this.npcId,
    required this.inspect,
    super.key,
  });

  final String npcTitle;
  final String npcId;
  final Revision3NpcSourceInspectionLoader inspect;

  @override
  State<Revision3NpcProfileDialog> createState() =>
      _Revision3NpcProfileDialogState();
}

class _Revision3NpcProfileDialogState extends State<Revision3NpcProfileDialog> {
  late Future<AuthoringRevision3NpcSourceInspectionResult> _inspection;

  @override
  void initState() {
    super.initState();
    _inspection = _load();
  }

  Future<AuthoringRevision3NpcSourceInspectionResult> _load() =>
      widget.inspect(npcId: widget.npcId);

  void _retry() {
    final next = _load();
    setState(() {
      _inspection = next;
    });
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const Key('revision3-npc-profile-dialog'),
    title: Text('Profile & checks — ${widget.npcTitle}'),
    content: SizedBox(
      width: 760,
      height: 650,
      child: FutureBuilder<AuthoringRevision3NpcSourceInspectionResult>(
        future: _inspection,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  CircularProgressIndicator(),
                  SizedBox(height: 16),
                  Text('Verifying the saved NPC Draft and generated source…'),
                ],
              ),
            );
          }
          final result = snapshot.data;
          if (result == null) {
            return _NpcInspectionError(
              error: snapshot.error ?? StateError('inspection failed'),
              retry: _retry,
            );
          }
          return _NpcInspectionResult(result: result);
        },
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Close'),
      ),
    ],
  );
}

class _NpcInspectionResult extends StatelessWidget {
  const _NpcInspectionResult({required this.result});

  final AuthoringRevision3NpcSourceInspectionResult result;

  @override
  Widget build(BuildContext context) {
    final plan = result.plan;
    final basedOn = plan.knownParentLabel;
    return ListView(
      key: const Key('revision3-npc-profile-result'),
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            const Chip(
              avatar: Icon(Icons.edit_note_outlined, size: 18),
              label: Text('Draft only'),
            ),
            Chip(
              avatar: Icon(
                Icons.block_outlined,
                size: 18,
                color: Theme.of(context).colorScheme.error,
              ),
              label: const Text('Build blocked'),
            ),
            const Chip(
              avatar: Icon(Icons.location_off_outlined, size: 18),
              label: Text('Not spawned'),
            ),
          ],
        ),
        if (basedOn != null) ...[
          const SizedBox(height: 10),
          Text('Based on $basedOn (saved parent evidence)'),
        ],
        const SizedBox(height: 12),
        const _NpcStatusCard(
          title: 'Saved source verified',
          body:
              'The generated AngelScript exactly matches the ScriptModule saved in this project.',
        ),
        const SizedBox(height: 8),
        const _NpcStatusCard(
          title: 'Saved parent evidence verified',
          body:
              'All three persisted parent-class identities and source seals regenerated the same NPC module.',
        ),
        const SizedBox(height: 8),
        const _NpcStatusCard(
          title: 'Exact project version checked',
          body:
              'The project head and canonical project bytes stayed unchanged throughout this read-only check.',
        ),
        const SizedBox(height: 18),
        Text(
          'Build readiness — ${plan.diagnostics.length} blockers',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 6),
        for (final diagnostic in plan.diagnostics)
          _NpcBlocker(diagnostic: diagnostic),
        const SizedBox(height: 12),
        ExpansionTile(
          key: const Key('revision3-npc-profile-advanced'),
          tilePadding: EdgeInsets.zero,
          title: const Text('Advanced'),
          subtitle: const Text(
            'Technical identities, parent evidence, and source',
          ),
          children: [
            _NpcTechnicalValue(label: 'NPC ID', value: plan.npc.reference.id),
            _NpcTechnicalValue(
              label: 'NPC revision',
              value: '${plan.npc.entityRevision}',
            ),
            _NpcTechnicalValue(
              label: 'Script module ID',
              value: plan.module.reference.id,
            ),
            _NpcTechnicalValue(
              label: 'Module revision',
              value: '${plan.module.entityRevision}',
            ),
            _NpcTechnicalValue(
              label: 'Module',
              value: plan.module.generated.moduleNamespace,
            ),
            _NpcTechnicalValue(
              label: 'Source path',
              value: plan.module.generated.moduleRelativePath,
            ),
            _NpcTechnicalValue(
              label: 'Runtime name',
              value: plan.npc.input.uniqueName,
            ),
            _NpcTechnicalValue(
              label: 'Character parent',
              value: plan.npc.input.parentCharacterDefinition.runtimeClass,
            ),
            _NpcTechnicalValue(
              label: 'AI parent',
              value: plan.npc.input.parentAiAgentConfig.runtimeClass,
            ),
            _NpcTechnicalValue(
              label: 'Spawn parent',
              value: plan.npc.input.parentSpawnDefinition.runtimeClass,
            ),
            _NpcTechnicalValue(
              label: 'Project SHA-256',
              value: result.projectSeal.sha256,
            ),
            _NpcTechnicalValue(
              label: 'Plan SHA-256',
              value: result.planSeal.sha256,
            ),
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
                  key: const Key('revision3-npc-source-copy'),
                  onPressed: () async {
                    await Clipboard.setData(
                      ClipboardData(text: plan.generatedSource),
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
              key: const Key('revision3-npc-generated-source'),
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              child: SelectableText(
                plan.generatedSource,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _NpcStatusCard extends StatelessWidget {
  const _NpcStatusCard({required this.title, required this.body});

  final String title;
  final String body;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    child: ListTile(
      leading: const Icon(Icons.verified_outlined, color: Colors.green),
      title: Text(title),
      subtitle: Text(body),
    ),
  );
}

class _NpcBlocker extends StatelessWidget {
  const _NpcBlocker({required this.diagnostic});

  final AuthoringRevision3NpcInspectionDiagnostic diagnostic;

  @override
  Widget build(BuildContext context) => Card(
    margin: const EdgeInsets.only(bottom: 6),
    child: ListTile(
      dense: true,
      leading: Icon(
        diagnostic.severity ==
                AuthoringRevision3NpcInspectionDiagnosticSeverity.warning
            ? Icons.warning_amber_outlined
            : Icons.error_outline,
        color: Theme.of(context).colorScheme.error,
      ),
      title: Text(_npcBlockerTitle(diagnostic.code)),
      subtitle: Text(diagnostic.message),
    ),
  );
}

String _npcBlockerTitle(
  AuthoringRevision3NpcInspectionDiagnosticCode code,
) => switch (code) {
  AuthoringRevision3NpcInspectionDiagnosticCode.compilerNotRun =>
    'Compiler not run',
  AuthoringRevision3NpcInspectionDiagnosticCode.productionLoweringUnavailable =>
    'Production build unavailable',
  AuthoringRevision3NpcInspectionDiagnosticCode.runtimeResidenceUnqualified =>
    'In-game residence unqualified',
  AuthoringRevision3NpcInspectionDiagnosticCode.spawnUnavailable =>
    'Spawn mechanism unavailable',
};

class _NpcTechnicalValue extends StatelessWidget {
  const _NpcTechnicalValue({required this.label, required this.value});

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

class _NpcInspectionError extends StatelessWidget {
  const _NpcInspectionError({required this.error, required this.retry});

  final Object error;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) {
    final close =
        error is Revision3NpcSourceInspectionStaleCheckpointException ||
        error is Revision3NpcSourceInspectionRequiresReopenException;
    return Center(
      key: const Key('revision3-npc-profile-error'),
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
              'NPC verification could not be completed',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(_npcFriendlyError(error), textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton.icon(
              key: Key(
                close
                    ? 'revision3-npc-profile-close'
                    : 'revision3-npc-profile-retry',
              ),
              onPressed: close ? () => Navigator.of(context).pop() : retry,
              icon: const Icon(Icons.refresh),
              label: Text(close ? 'Close and refresh' : 'Try again'),
            ),
          ],
        ),
      ),
    );
  }
}

String _npcFriendlyError(Object error) => switch (error) {
  Revision3NpcSourceInspectionStaleCheckpointException() =>
    'The project changed after this NPC was selected. Close this window, refresh the Library, and try again.',
  Revision3NpcSourceInspectionRequiresReopenException() =>
    'The project must be reopened before this NPC can be checked safely.',
  CurrentProjectOperationUnsupportedException(:final message) => message,
  ModFfiException(:final message) => message,
  _ =>
    'No project, game, or save files were changed. The saved NPC may need repair before it can be inspected.',
};
