import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';

typedef Revision3ProjectBuildPlanLoader =
    Future<AuthoringRevision3ProjectBuildPlanResult> Function();

typedef Revision3ProjectBuildPlanCountCopy = String Function(int count);
typedef Revision3ProjectBuildPlanProgressCopy =
    String Function(int ready, int blocked, int total);
typedef Revision3ProjectBuildPlanRevisionCopy = String Function(int revision);
typedef Revision3ProjectBuildPlanDomainCopy =
    String Function(AuthoringRevision3ProjectBuildDomain domain);
typedef Revision3ProjectBuildPlanReasonCopy =
    String Function(AuthoringRevision3ProjectBuildBlockReason reason);

/// The exact managed-project checkpoint a preview is allowed to describe.
///
/// The identity remains control data: the panel compares it with the returned
/// native basis but never renders it.
@immutable
final class Revision3ProjectBuildPlanCheckpoint {
  Revision3ProjectBuildPlanCheckpoint({
    required String projectId,
    required this.projectRevision,
    required String checkpointIdentity,
  }) : projectId = _requiredBuildPlanText(projectId, 'projectId'),
       checkpointIdentity = _requiredBuildPlanText(
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
      other is Revision3ProjectBuildPlanCheckpoint &&
      other.projectId == projectId &&
      other.projectRevision == projectRevision &&
      other.checkpointIdentity == checkpointIdentity;

  @override
  int get hashCode =>
      Object.hash(projectId, projectRevision, checkpointIdentity);
}

enum Revision3ProjectBuildPlanLoadState { loading, ready, failed }

@immutable
final class Revision3ProjectBuildPlanSnapshot {
  const Revision3ProjectBuildPlanSnapshot._({
    required this.checkpoint,
    required this.state,
    this.result,
    this.error,
  });

  final Revision3ProjectBuildPlanCheckpoint checkpoint;
  final Revision3ProjectBuildPlanLoadState state;
  final AuthoringRevision3ProjectBuildPlanResult? result;
  final Object? error;
}

/// Owns only an exact-checkpoint, evidence-only preview read.
///
/// It deliberately exposes no build, install, deployment, publication, game,
/// or save mutation operation. Late completions from a previous project
/// checkpoint are discarded.
final class Revision3ProjectBuildPlanController extends ChangeNotifier {
  Revision3ProjectBuildPlanController({
    required Revision3ProjectBuildPlanCheckpoint checkpoint,
    required this.loader,
  }) : _checkpoint = checkpoint,
       _snapshot = Revision3ProjectBuildPlanSnapshot._(
         checkpoint: checkpoint,
         state: Revision3ProjectBuildPlanLoadState.loading,
       );

  Revision3ProjectBuildPlanCheckpoint _checkpoint;
  Revision3ProjectBuildPlanLoader loader;
  Revision3ProjectBuildPlanSnapshot _snapshot;
  int _generation = 0;
  bool _disposed = false;

  Revision3ProjectBuildPlanSnapshot get snapshot => _snapshot;

  void synchronize({
    required Revision3ProjectBuildPlanCheckpoint checkpoint,
    required Revision3ProjectBuildPlanLoader load,
  }) {
    loader = load;
    if (checkpoint == _checkpoint || _disposed) return;
    _checkpoint = checkpoint;
    unawaited(refresh());
  }

  Future<void> refresh() async {
    if (_disposed) return;
    final generation = ++_generation;
    final checkpoint = _checkpoint;
    _publish(
      Revision3ProjectBuildPlanSnapshot._(
        checkpoint: checkpoint,
        state: Revision3ProjectBuildPlanLoadState.loading,
      ),
    );
    try {
      final result = await loader();
      if (!_isCurrent(generation, checkpoint)) return;
      _verifyExactCheckpoint(result, checkpoint);
      _publish(
        Revision3ProjectBuildPlanSnapshot._(
          checkpoint: checkpoint,
          state: Revision3ProjectBuildPlanLoadState.ready,
          result: result,
        ),
      );
    } catch (error) {
      if (!_isCurrent(generation, checkpoint)) return;
      _publish(
        Revision3ProjectBuildPlanSnapshot._(
          checkpoint: checkpoint,
          state: Revision3ProjectBuildPlanLoadState.failed,
          error: error,
        ),
      );
    }
  }

  bool _isCurrent(
    int generation,
    Revision3ProjectBuildPlanCheckpoint checkpoint,
  ) => !_disposed && generation == _generation && checkpoint == _checkpoint;

  void _publish(Revision3ProjectBuildPlanSnapshot snapshot) {
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

/// All user-facing copy for the standalone preview panel.
///
/// English and German are built in so this bounded surface does not need to
/// couple itself to the application's generated localization layer. The host
/// can still inject localized copy later.
@immutable
final class Revision3ProjectBuildPlanCopy {
  const Revision3ProjectBuildPlanCopy({
    required this.title,
    required this.refreshTooltip,
    required this.checkingSemanticsLabel,
    required this.loadErrorTitle,
    required this.loadErrorBody,
    required this.retryLabel,
    required this.previewBoundaryTitle,
    required this.previewBoundaryBody,
    required this.emptyTitle,
    required this.emptyBody,
    required this.blockedTitle,
    required this.blockedBody,
    required this.coverageCompleteTitle,
    required this.coverageCompleteBody,
    required this.productionCount,
    required this.domainsTitle,
    required this.domainName,
    required this.notPresentLabel,
    required this.domainProgress,
    required this.authorBlockersTitle,
    required this.authorBlockersBody,
    required this.toolkitBlockersTitle,
    required this.toolkitBlockersBody,
    required this.affectedCount,
    required this.blockerReason,
    required this.technicalDetailsTitle,
    required this.technicalDetailsBody,
    required this.inputSealLabel,
    required this.planSealLabel,
    required this.exactRevision,
  });

  const Revision3ProjectBuildPlanCopy.english()
    : title = 'Project build preview',
      refreshTooltip = 'Refresh project build preview',
      checkingSemanticsLabel = 'Checking the exact project build preview',
      loadErrorTitle = 'Preview unavailable',
      loadErrorBody =
          'Build readiness could not be verified for this exact project version.',
      retryLabel = 'Retry',
      previewBoundaryTitle = 'Preview only',
      previewBoundaryBody =
          'No files were created. The game and save remain unchanged. This view has no build or install authority.',
      emptyTitle = 'No production content yet',
      emptyBody =
          'Add project content to see what the toolkit can prepare for a future build.',
      blockedTitle = 'Preparation required',
      blockedBody =
          'Some project content still needs author work or toolkit support.',
      coverageCompleteTitle = 'Content coverage complete',
      coverageCompleteBody =
          'The current content is semantically covered. Creating and installing a build remains a separate future step.',
      productionCount = _englishProductionCount,
      domainsTitle = 'Content areas',
      domainName = _englishDomainName,
      notPresentLabel = 'Not present',
      domainProgress = _englishDomainProgress,
      authorBlockersTitle = 'Needs project work',
      authorBlockersBody =
          'These items can be resolved by editing the project content.',
      toolkitBlockersTitle = 'Needs toolkit support',
      toolkitBlockersBody =
          'These content types are not lowered by the toolkit yet.',
      affectedCount = _englishAffectedCount,
      blockerReason = _englishBlockerReason,
      technicalDetailsTitle = 'Technical verification',
      technicalDetailsBody =
          'Deterministic seals bind this preview to its exact inputs and result.',
      inputSealLabel = 'Input seal',
      planSealLabel = 'Plan seal',
      exactRevision = _englishExactRevision;

  const Revision3ProjectBuildPlanCopy.german()
    : title = 'Projekt-Bauvorschau',
      refreshTooltip = 'Projekt-Bauvorschau aktualisieren',
      checkingSemanticsLabel = 'Exakte Projekt-Bauvorschau wird gepr\u00fcft',
      loadErrorTitle = 'Vorschau nicht verf\u00fcgbar',
      loadErrorBody =
          'Die Baubereitschaft konnte f\u00fcr diese exakte Projektversion nicht gepr\u00fcft werden.',
      retryLabel = 'Erneut versuchen',
      previewBoundaryTitle = 'Nur Vorschau',
      previewBoundaryBody =
          'Es wurden keine Dateien erstellt. Spiel und Spielstand bleiben unver\u00e4ndert. Diese Ansicht darf weder bauen noch installieren.',
      emptyTitle = 'Noch keine Produktionsinhalte',
      emptyBody =
          'F\u00fcge Projektinhalte hinzu, um ihre Vorbereitung f\u00fcr einen sp\u00e4teren Build zu pr\u00fcfen.',
      blockedTitle = 'Vorbereitung erforderlich',
      blockedBody =
          'Einige Projektinhalte ben\u00f6tigen noch Bearbeitung oder Toolkit-Unterst\u00fctzung.',
      coverageCompleteTitle = 'Inhaltliche Abdeckung vollst\u00e4ndig',
      coverageCompleteBody =
          'Die aktuellen Inhalte sind semantisch abgedeckt. Build und Installation bleiben ein eigener sp\u00e4terer Schritt.',
      productionCount = _germanProductionCount,
      domainsTitle = 'Inhaltsbereiche',
      domainName = _germanDomainName,
      notPresentLabel = 'Nicht vorhanden',
      domainProgress = _germanDomainProgress,
      authorBlockersTitle = 'Projekt muss bearbeitet werden',
      authorBlockersBody =
          'Diese Punkte lassen sich durch Bearbeiten der Projektinhalte beheben.',
      toolkitBlockersTitle = 'Toolkit-Unterst\u00fctzung fehlt',
      toolkitBlockersBody =
          'Diese Inhaltstypen kann das Toolkit noch nicht umsetzen.',
      affectedCount = _germanAffectedCount,
      blockerReason = _germanBlockerReason,
      technicalDetailsTitle = 'Technische Verifikation',
      technicalDetailsBody =
          'Deterministische Siegel binden diese Vorschau an ihre exakten Eingaben und ihr Ergebnis.',
      inputSealLabel = 'Eingabesiegel',
      planSealLabel = 'Plansiegel',
      exactRevision = _germanExactRevision;

  final String title;
  final String refreshTooltip;
  final String checkingSemanticsLabel;
  final String loadErrorTitle;
  final String loadErrorBody;
  final String retryLabel;
  final String previewBoundaryTitle;
  final String previewBoundaryBody;
  final String emptyTitle;
  final String emptyBody;
  final String blockedTitle;
  final String blockedBody;
  final String coverageCompleteTitle;
  final String coverageCompleteBody;
  final Revision3ProjectBuildPlanCountCopy productionCount;
  final String domainsTitle;
  final Revision3ProjectBuildPlanDomainCopy domainName;
  final String notPresentLabel;
  final Revision3ProjectBuildPlanProgressCopy domainProgress;
  final String authorBlockersTitle;
  final String authorBlockersBody;
  final String toolkitBlockersTitle;
  final String toolkitBlockersBody;
  final Revision3ProjectBuildPlanCountCopy affectedCount;
  final Revision3ProjectBuildPlanReasonCopy blockerReason;
  final String technicalDetailsTitle;
  final String technicalDetailsBody;
  final String inputSealLabel;
  final String planSealLabel;
  final Revision3ProjectBuildPlanRevisionCopy exactRevision;
}

/// Compact, read-only aggregate readiness for the exact managed checkpoint.
///
/// This widget cannot build, install, deploy, publish, or mutate game/save
/// state. Stable project identity is used only for result validation.
class Revision3ProjectBuildPlanPanel extends StatefulWidget {
  const Revision3ProjectBuildPlanPanel({
    required this.checkpoint,
    required this.load,
    this.copy = const Revision3ProjectBuildPlanCopy.english(),
    super.key,
  });

  final Revision3ProjectBuildPlanCheckpoint checkpoint;
  final Revision3ProjectBuildPlanLoader load;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  State<Revision3ProjectBuildPlanPanel> createState() =>
      _Revision3ProjectBuildPlanPanelState();
}

class _Revision3ProjectBuildPlanPanelState
    extends State<Revision3ProjectBuildPlanPanel> {
  late final Revision3ProjectBuildPlanController _controller;

  @override
  void initState() {
    super.initState();
    _controller = Revision3ProjectBuildPlanController(
      checkpoint: widget.checkpoint,
      loader: widget.load,
    );
    unawaited(_controller.refresh());
  }

  @override
  void didUpdateWidget(covariant Revision3ProjectBuildPlanPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.synchronize(checkpoint: widget.checkpoint, load: widget.load);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _controller,
    builder: (context, _) {
      final snapshot = _controller.snapshot;
      final scheme = Theme.of(context).colorScheme;
      return Material(
        key: const Key('revision3-project-build-plan-panel'),
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14),
          side: BorderSide(color: scheme.outlineVariant),
        ),
        clipBehavior: Clip.antiAlias,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _BuildPlanHeader(
                copy: widget.copy,
                loading:
                    snapshot.state ==
                    Revision3ProjectBuildPlanLoadState.loading,
                refresh: _controller.refresh,
              ),
              const SizedBox(height: 10),
              _PreviewBoundary(copy: widget.copy),
              const SizedBox(height: 10),
              switch (snapshot.state) {
                Revision3ProjectBuildPlanLoadState.loading => Semantics(
                  liveRegion: true,
                  label: widget.copy.checkingSemanticsLabel,
                  child: const LinearProgressIndicator(
                    key: Key('revision3-project-build-plan-loading'),
                  ),
                ),
                Revision3ProjectBuildPlanLoadState.failed => _BuildPlanError(
                  copy: widget.copy,
                  retry: _controller.refresh,
                ),
                Revision3ProjectBuildPlanLoadState.ready => _BuildPlanReport(
                  result: snapshot.result!,
                  copy: widget.copy,
                ),
              },
            ],
          ),
        ),
      );
    },
  );
}

class _BuildPlanHeader extends StatelessWidget {
  const _BuildPlanHeader({
    required this.copy,
    required this.loading,
    required this.refresh,
  });

  final Revision3ProjectBuildPlanCopy copy;
  final bool loading;
  final VoidCallback refresh;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Padding(
        padding: const EdgeInsets.only(top: 9),
        child: Icon(
          Icons.inventory_2_outlined,
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
      const SizedBox(width: 10),
      Expanded(
        child: Padding(
          padding: const EdgeInsets.only(top: 8),
          child: Text(
            copy.title,
            style: Theme.of(context).textTheme.titleMedium,
          ),
        ),
      ),
      IconButton(
        key: const Key('revision3-project-build-plan-refresh'),
        tooltip: copy.refreshTooltip,
        onPressed: loading ? null : refresh,
        icon: const Icon(Icons.refresh),
      ),
    ],
  );
}

class _PreviewBoundary extends StatelessWidget {
  const _PreviewBoundary({required this.copy});

  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      child: DecoratedBox(
        key: const Key('revision3-project-build-plan-boundary'),
        decoration: BoxDecoration(
          color: scheme.secondaryContainer,
          borderRadius: BorderRadius.circular(10),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.shield_outlined, color: scheme.onSecondaryContainer),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      copy.previewBoundaryTitle,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        color: scheme.onSecondaryContainer,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      copy.previewBoundaryBody,
                      style: TextStyle(color: scheme.onSecondaryContainer),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _BuildPlanError extends StatelessWidget {
  const _BuildPlanError({required this.copy, required this.retry});

  final Revision3ProjectBuildPlanCopy copy;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) => Row(
    key: const Key('revision3-project-build-plan-error'),
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Icon(Icons.error_outline, color: Theme.of(context).colorScheme.error),
      const SizedBox(width: 10),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              copy.loadErrorTitle,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 2),
            Text(copy.loadErrorBody),
            const SizedBox(height: 4),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton(
                key: const Key('revision3-project-build-plan-retry'),
                onPressed: retry,
                child: Text(copy.retryLabel),
              ),
            ),
          ],
        ),
      ),
    ],
  );
}

class _BuildPlanReport extends StatelessWidget {
  const _BuildPlanReport({required this.result, required this.copy});

  final AuthoringRevision3ProjectBuildPlanResult result;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) {
    final plan = result.plan;
    final authorBlockers = plan.blockers
        .where(
          (blocker) =>
              blocker.category ==
              AuthoringRevision3ProjectBuildBlockerCategory.authorProject,
        )
        .toList(growable: false);
    final toolkitBlockers = plan.blockers
        .where(
          (blocker) =>
              blocker.category ==
              AuthoringRevision3ProjectBuildBlockerCategory.toolkitSupport,
        )
        .toList(growable: false);
    return Column(
      key: const Key('revision3-project-build-plan-report'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _BuildPlanSummary(plan: plan, copy: copy),
        const SizedBox(height: 14),
        Text(copy.domainsTitle, style: Theme.of(context).textTheme.titleSmall),
        const SizedBox(height: 6),
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Column(
            children: [
              for (var index = 0; index < plan.domains.length; index++) ...[
                _BuildPlanDomainRow(summary: plan.domains[index], copy: copy),
                if (index != plan.domains.length - 1) const Divider(height: 1),
              ],
            ],
          ),
        ),
        if (authorBlockers.isNotEmpty) ...[
          const SizedBox(height: 14),
          _BuildPlanBlockerGroup(
            key: const Key('revision3-project-build-plan-author-blockers'),
            icon: Icons.edit_note_outlined,
            title: copy.authorBlockersTitle,
            description: copy.authorBlockersBody,
            blockers: authorBlockers,
            copy: copy,
          ),
        ],
        if (toolkitBlockers.isNotEmpty) ...[
          const SizedBox(height: 14),
          _BuildPlanBlockerGroup(
            key: const Key('revision3-project-build-plan-toolkit-blockers'),
            icon: Icons.construction_outlined,
            title: copy.toolkitBlockersTitle,
            description: copy.toolkitBlockersBody,
            blockers: toolkitBlockers,
            copy: copy,
          ),
        ],
        const SizedBox(height: 8),
        _TechnicalSeals(plan: plan, copy: copy),
        const SizedBox(height: 4),
        Text(
          copy.exactRevision(plan.projectRevision),
          key: const Key('revision3-project-build-plan-revision'),
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

class _BuildPlanSummary extends StatelessWidget {
  const _BuildPlanSummary({required this.plan, required this.copy});

  final AuthoringRevision3ProjectBuildPlan plan;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) {
    final (icon, color, title, body) = switch (plan.outcome) {
      AuthoringRevision3ProjectBuildOutcome.empty => (
        Icons.inbox_outlined,
        Theme.of(context).colorScheme.onSurfaceVariant,
        copy.emptyTitle,
        copy.emptyBody,
      ),
      AuthoringRevision3ProjectBuildOutcome.blocked => (
        Icons.warning_amber_rounded,
        Theme.of(context).colorScheme.tertiary,
        copy.blockedTitle,
        copy.blockedBody,
      ),
      AuthoringRevision3ProjectBuildOutcome.coverageComplete => (
        Icons.task_alt_outlined,
        Theme.of(context).colorScheme.primary,
        copy.coverageCompleteTitle,
        copy.coverageCompleteBody,
      ),
    };
    return Semantics(
      container: true,
      liveRegion: true,
      child: Row(
        key: const Key('revision3-project-build-plan-summary'),
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(icon, color: color),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 2),
                Text(body),
                const SizedBox(height: 3),
                Text(
                  copy.productionCount(plan.productionContentCount),
                  key: const Key('revision3-project-build-plan-count'),
                  style: Theme.of(context).textTheme.labelLarge,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _BuildPlanDomainRow extends StatelessWidget {
  const _BuildPlanDomainRow({required this.summary, required this.copy});

  final AuthoringRevision3ProjectBuildDomainSummary summary;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final (icon, color, status) = switch (summary.status) {
      AuthoringRevision3ProjectBuildDomainStatus.notPresent => (
        Icons.remove_circle_outline,
        scheme.onSurfaceVariant,
        copy.notPresentLabel,
      ),
      AuthoringRevision3ProjectBuildDomainStatus.ready => (
        Icons.check_circle_outline,
        scheme.primary,
        copy.domainProgress(
          summary.readyCount,
          summary.blockedCount,
          summary.contentCount,
        ),
      ),
      AuthoringRevision3ProjectBuildDomainStatus.blocked => (
        Icons.error_outline,
        scheme.error,
        copy.domainProgress(
          summary.readyCount,
          summary.blockedCount,
          summary.contentCount,
        ),
      ),
    };
    final domain = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 20, color: color),
        const SizedBox(width: 8),
        Flexible(
          child: Text(
            copy.domainName(summary.domain),
            style: Theme.of(context).textTheme.labelLarge,
          ),
        ),
      ],
    );
    final progress = Text(
      status,
      key: ValueKey(
        'revision3-project-build-plan-domain-${summary.domain.name}',
      ),
      style: Theme.of(context).textTheme.bodySmall?.copyWith(color: color),
    );
    final scaled = MediaQuery.textScalerOf(context).scale(16) >= 24;
    return LayoutBuilder(
      builder: (context, constraints) {
        final stacked = constraints.maxWidth < 420 || scaled;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: stacked
              ? Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [domain, const SizedBox(height: 4), progress],
                )
              : Row(
                  children: [
                    Expanded(child: domain),
                    const SizedBox(width: 12),
                    Flexible(child: progress),
                  ],
                ),
        );
      },
    );
  }
}

class _BuildPlanBlockerGroup extends StatelessWidget {
  const _BuildPlanBlockerGroup({
    required this.icon,
    required this.title,
    required this.description,
    required this.blockers,
    required this.copy,
    super.key,
  });

  final IconData icon;
  final String title;
  final String description;
  final List<AuthoringRevision3ProjectBuildBlocker> blockers;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainer,
      borderRadius: BorderRadius.circular(10),
    ),
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(icon, size: 22),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: Theme.of(context).textTheme.titleSmall),
                    const SizedBox(height: 2),
                    Text(description),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          for (var index = 0; index < blockers.length; index++) ...[
            _BuildPlanBlockerRow(blocker: blockers[index], copy: copy),
            if (index != blockers.length - 1) const Divider(height: 12),
          ],
        ],
      ),
    ),
  );
}

class _BuildPlanBlockerRow extends StatelessWidget {
  const _BuildPlanBlockerRow({required this.blocker, required this.copy});

  final AuthoringRevision3ProjectBuildBlocker blocker;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(
        copy.blockerReason(blocker.reason),
        style: Theme.of(context).textTheme.bodyMedium,
      ),
      const SizedBox(height: 2),
      Wrap(
        spacing: 8,
        runSpacing: 2,
        children: [
          Text(
            copy.domainName(blocker.domain),
            style: Theme.of(context).textTheme.labelSmall,
          ),
          Text(
            copy.affectedCount(blocker.affectedCount),
            style: Theme.of(context).textTheme.labelSmall,
          ),
        ],
      ),
    ],
  );
}

class _TechnicalSeals extends StatelessWidget {
  const _TechnicalSeals({required this.plan, required this.copy});

  final AuthoringRevision3ProjectBuildPlan plan;
  final Revision3ProjectBuildPlanCopy copy;

  @override
  Widget build(BuildContext context) => ExpansionTile(
    key: const Key('revision3-project-build-plan-technical'),
    tilePadding: EdgeInsets.zero,
    childrenPadding: const EdgeInsets.only(bottom: 8),
    title: Text(copy.technicalDetailsTitle),
    subtitle: Text(copy.technicalDetailsBody),
    children: [
      _SealRow(label: copy.inputSealLabel, seal: plan.inputSeal),
      const SizedBox(height: 6),
      _SealRow(label: copy.planSealLabel, seal: plan.planSeal),
    ],
  );
}

class _SealRow extends StatelessWidget {
  const _SealRow({required this.label, required this.seal});

  final String label;
  final AuthoringRevision3ProjectBuildSeal seal;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Text(label, style: Theme.of(context).textTheme.labelMedium),
      const SizedBox(height: 2),
      SelectableText(
        '${seal.byteLength} bytes \u00b7 ${seal.sha256}',
        style: Theme.of(context).textTheme.bodySmall,
      ),
    ],
  );
}

void _verifyExactCheckpoint(
  AuthoringRevision3ProjectBuildPlanResult result,
  Revision3ProjectBuildPlanCheckpoint checkpoint,
) {
  final plan = result.plan;
  if (plan.projectId != checkpoint.projectId ||
      plan.projectRevision != checkpoint.projectRevision ||
      result.basisHead.canonicalJson != checkpoint.checkpointIdentity) {
    throw const FormatException(
      'Project build preview does not match the current checkpoint.',
    );
  }
}

String _englishProductionCount(int count) => switch (count) {
  0 => 'No production records',
  1 => '1 production record',
  _ => '$count production records',
};

String _germanProductionCount(int count) => switch (count) {
  0 => 'Keine Produktionseintr\u00e4ge',
  1 => '1 Produktionseintrag',
  _ => '$count Produktionseintr\u00e4ge',
};

String _englishDomainProgress(int ready, int blocked, int total) =>
    '$ready ready \u00b7 $blocked blocked \u00b7 $total total';

String _germanDomainProgress(int ready, int blocked, int total) =>
    '$ready bereit \u00b7 $blocked blockiert \u00b7 $total gesamt';

String _englishAffectedCount(int count) => switch (count) {
  1 => '1 affected record',
  _ => '$count affected records',
};

String _germanAffectedCount(int count) => switch (count) {
  1 => '1 betroffener Eintrag',
  _ => '$count betroffene Eintr\u00e4ge',
};

String _englishExactRevision(int revision) =>
    'Exact project revision $revision';

String _germanExactRevision(int revision) => 'Exakte Projektversion $revision';

String _englishDomainName(AuthoringRevision3ProjectBuildDomain domain) =>
    switch (domain) {
      AuthoringRevision3ProjectBuildDomain.localization => 'Localization',
      AuthoringRevision3ProjectBuildDomain.dialog => 'Dialog',
      AuthoringRevision3ProjectBuildDomain.voice => 'Voice',
      AuthoringRevision3ProjectBuildDomain.npc => 'NPCs',
      AuthoringRevision3ProjectBuildDomain.quest => 'Quests',
      AuthoringRevision3ProjectBuildDomain.scripts => 'Scripts',
      AuthoringRevision3ProjectBuildDomain.items => 'Items',
      AuthoringRevision3ProjectBuildDomain.dataAssets => 'DataAssets',
    };

String _germanDomainName(AuthoringRevision3ProjectBuildDomain domain) =>
    switch (domain) {
      AuthoringRevision3ProjectBuildDomain.localization => 'Lokalisierung',
      AuthoringRevision3ProjectBuildDomain.dialog => 'Dialoge',
      AuthoringRevision3ProjectBuildDomain.voice => 'Sprachausgabe',
      AuthoringRevision3ProjectBuildDomain.npc => 'NPCs',
      AuthoringRevision3ProjectBuildDomain.quest => 'Quests',
      AuthoringRevision3ProjectBuildDomain.scripts => 'Skripte',
      AuthoringRevision3ProjectBuildDomain.items => 'Items',
      AuthoringRevision3ProjectBuildDomain.dataAssets => 'DataAssets',
    };

String _englishBlockerReason(
  AuthoringRevision3ProjectBuildBlockReason reason,
) => switch (reason) {
  AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable =>
    'Localization output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable =>
    'Dialog output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported =>
    'The project name is not supported for Voice output.',
  AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported =>
    'A dialog-line name is not supported for Voice output.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded =>
    'The Voice slot limit is exceeded.',
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved =>
    'A Voice target is unresolved.',
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous =>
    'A Voice target is ambiguous.',
  AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified =>
    'A Voice add target is not supported yet.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing =>
    'An approved Voice take still needs to be selected.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved =>
    'A selected Voice take is not approved.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeCodecUnqualified =>
    'A selected Voice take does not meet the codec requirements.',
  AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded =>
    'Voice content exceeds the supported payload budget.',
  AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable =>
    'NPC output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable =>
    'Quest output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable =>
    'Script output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable =>
    'Item-patch output is not implemented yet.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported =>
    'A DataAsset target is not supported.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch =>
    'A DataAsset selector no longer matches.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed =>
    'A DataAsset replacement is malformed.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite =>
    'A DataAsset replacement contains a non-finite value.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive =>
    'A DataAsset replacement must be positive.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetPreservedComponentChanged =>
    'A preserved DataAsset component was changed.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetReviewedPreparationFailed =>
    'Reviewed DataAsset preparation failed.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetDerivedReplacementMismatch =>
    'A derived DataAsset replacement no longer matches.',
};

String _germanBlockerReason(
  AuthoringRevision3ProjectBuildBlockReason reason,
) => switch (reason) {
  AuthoringRevision3ProjectBuildBlockReason.localizationLoweringUnavailable =>
    'Die Ausgabe f\u00fcr Lokalisierung ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.dialogLoweringUnavailable =>
    'Die Ausgabe f\u00fcr Dialoge ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.voiceProjectNameUnsupported =>
    'Der Projektname wird f\u00fcr die Sprachausgabe nicht unterst\u00fctzt.',
  AuthoringRevision3ProjectBuildBlockReason.voiceLineLabelUnsupported =>
    'Der Name einer Dialogzeile wird f\u00fcr die Sprachausgabe nicht unterst\u00fctzt.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSlotLimitExceeded =>
    'Das Limit f\u00fcr Sprachausgabe-Slots ist \u00fcberschritten.',
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved =>
    'Ein Ziel f\u00fcr die Sprachausgabe ist nicht aufgel\u00f6st.',
  AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous =>
    'Ein Ziel f\u00fcr die Sprachausgabe ist mehrdeutig.',
  AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified =>
    'Ein neues Ziel f\u00fcr die Sprachausgabe wird noch nicht unterst\u00fctzt.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing =>
    'Eine freigegebene Sprachaufnahme muss noch ausgew\u00e4hlt werden.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved =>
    'Eine ausgew\u00e4hlte Sprachaufnahme ist nicht freigegeben.',
  AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeCodecUnqualified =>
    'Eine Sprachaufnahme erf\u00fcllt die Codec-Anforderungen nicht.',
  AuthoringRevision3ProjectBuildBlockReason.voicePayloadBudgetExceeded =>
    'Die Sprachausgabe \u00fcberschreitet das unterst\u00fctzte Datenbudget.',
  AuthoringRevision3ProjectBuildBlockReason.npcLoweringUnavailable =>
    'Die Ausgabe f\u00fcr NPCs ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.questLoweringUnavailable =>
    'Die Ausgabe f\u00fcr Quests ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.scriptLoweringUnavailable =>
    'Die Ausgabe f\u00fcr Skripte ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.itemPatchLoweringUnavailable =>
    'Die Ausgabe f\u00fcr Item-\u00c4nderungen ist noch nicht implementiert.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetTargetUnsupported =>
    'Ein DataAsset-Ziel wird nicht unterst\u00fctzt.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetSelectorMismatch =>
    'Ein DataAsset-Selektor stimmt nicht mehr \u00fcberein.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementMalformed =>
    'Ein DataAsset-Ersatzwert ist ung\u00fcltig.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonFinite =>
    'Ein DataAsset-Ersatzwert ist nicht endlich.',
  AuthoringRevision3ProjectBuildBlockReason.dataAssetReplacementNonPositive =>
    'Ein DataAsset-Ersatzwert muss positiv sein.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetPreservedComponentChanged =>
    'Eine gesch\u00fctzte DataAsset-Komponente wurde ver\u00e4ndert.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetReviewedPreparationFailed =>
    'Die Vorbereitung eines gepr\u00fcften DataAssets ist fehlgeschlagen.',
  AuthoringRevision3ProjectBuildBlockReason
      .dataAssetDerivedReplacementMismatch =>
    'Ein abgeleiteter DataAsset-Ersatzwert stimmt nicht mehr \u00fcberein.',
};

String _requiredBuildPlanText(String value, String name) {
  if (value.isEmpty) throw ArgumentError.value(value, name);
  return value;
}
