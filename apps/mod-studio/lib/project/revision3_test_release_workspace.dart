import 'package:flutter/material.dart';

/// Author-facing outcome of one check against a managed revision-3 project.
///
/// An evaluated outcome ([passed], [needsAttention], or [blocked]) must carry
/// [Revision3TestReleaseEvidence]. The workspace additionally verifies that
/// the evidence belongs to its exact project checkpoint before rendering the
/// outcome as current.
enum Revision3TestReleaseCheckState {
  notEvaluated,
  checking,
  passed,
  needsAttention,
  blocked,
  unavailable,
}

/// Closed purpose boundary for managed revision-3 evidence.
///
/// Evidence can authorize only the check or capability that produced it. The
/// checkpoint identity alone is deliberately insufficient because build and
/// deployment evidence have different authority.
enum Revision3TestReleaseEvidenceScope {
  projectStructure,
  scripts,
  voice,
  dataAssets,
  playableBuild,
  deployment,
}

/// Evidence binding one visible result or capability to an exact managed
/// revision-3 checkpoint.
@immutable
final class Revision3TestReleaseEvidence {
  Revision3TestReleaseEvidence({
    required String projectId,
    required int projectRevision,
    required String checkpointIdentity,
    required this.scope,
    required String summary,
  }) : projectId = _requireText(projectId, 'projectId'),
       projectRevision = _requireRevision(projectRevision),
       checkpointIdentity = _requireText(
         checkpointIdentity,
         'checkpointIdentity',
       ),
       summary = _requireText(summary, 'summary');

  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3TestReleaseEvidenceScope scope;
  final String summary;

  bool belongsTo({
    required String projectId,
    required int projectRevision,
    required String checkpointIdentity,
    required Revision3TestReleaseEvidenceScope scope,
  }) =>
      this.projectId == projectId &&
      this.projectRevision == projectRevision &&
      this.checkpointIdentity == checkpointIdentity &&
      this.scope == scope;
}

/// Presentation data for one project check.
///
/// [onPressed] is a navigation or continuation affordance only. This model
/// does not grant build, deployment, project-mutation, or runtime authority.
@immutable
final class Revision3TestReleaseCheck {
  Revision3TestReleaseCheck({
    required this.state,
    required String title,
    required String description,
    Revision3TestReleaseEvidence? evidence,
    String? actionLabel,
    this.onPressed,
  }) : title = _requireText(title, 'title'),
       description = _requireText(description, 'description'),
       evidence = _validateEvidenceForState(state, evidence),
       actionLabel = _optionalText(actionLabel, 'actionLabel');

  final Revision3TestReleaseCheckState state;
  final String title;
  final String description;
  final Revision3TestReleaseEvidence? evidence;
  final String? actionLabel;
  final VoidCallback? onPressed;
}

/// One separately qualified playable-output or deployment capability.
///
/// A callback alone never enables the action. Both a matching explicit
/// [evidence] object and a callback are required. Build evidence and deploy
/// evidence are passed as separate capability instances, so one cannot
/// silently authorize the other.
@immutable
final class Revision3TestReleaseCapability {
  Revision3TestReleaseCapability({
    required String title,
    required String description,
    required String blockedReason,
    required String actionLabel,
    this.evidence,
    this.onPressed,
  }) : title = _requireText(title, 'title'),
       description = _requireText(description, 'description'),
       blockedReason = _requireText(blockedReason, 'blockedReason'),
       actionLabel = _requireText(actionLabel, 'actionLabel');

  final String title;
  final String description;
  final String blockedReason;
  final String actionLabel;
  final Revision3TestReleaseEvidence? evidence;
  final VoidCallback? onPressed;
}

/// Framework copy for [Revision3TestReleaseWorkspace]. Domain-specific check
/// and capability copy stays next to the evidence supplied by the caller.
@immutable
final class Revision3TestReleaseCopy {
  const Revision3TestReleaseCopy({
    required this.title,
    required this.description,
    required this.evidenceBoundary,
    required this.checksHeading,
    required this.releaseHeading,
    required this.notEvaluatedLabel,
    required this.checkingLabel,
    required this.passedLabel,
    required this.needsAttentionLabel,
    required this.blockedLabel,
    required this.unavailableLabel,
    required this.availableLabel,
    required this.evidenceLabel,
    required this.staleEvidenceDescription,
    required this.actionNotConnectedDescription,
    required this.problemsHeading,
    required this.voiceContinuationHeading,
  });

  const Revision3TestReleaseCopy.english()
    : title = 'Test & Release',
      description =
          'Check every part of your mod before creating playable files or installing them.',
      evidenceBoundary =
          'Nothing is assumed ready. A checked result applies only to this exact saved project version.',
      checksHeading = 'Project checks',
      releaseHeading = 'Playable output',
      notEvaluatedLabel = 'Not checked',
      checkingLabel = 'Checking',
      passedLabel = 'Checked',
      needsAttentionLabel = 'Needs attention',
      blockedLabel = 'Blocked',
      unavailableLabel = 'Not available',
      availableLabel = 'Available',
      evidenceLabel = 'Evidence',
      staleEvidenceDescription =
          'This result belongs to a different project version. Run the check again.',
      actionNotConnectedDescription =
          'Evidence exists, but this action is not connected in the current workspace.',
      problemsHeading = 'Problems to resolve',
      voiceContinuationHeading = 'Voice build check';

  const Revision3TestReleaseCopy.german()
    : title = 'Testen & Veröffentlichen',
      description =
          'Prüfe jeden Teil deiner Mod, bevor du spielbare Dateien erstellst oder installierst.',
      evidenceBoundary =
          'Nichts gilt automatisch als fertig. Ein Prüfergebnis gehört immer genau zu dieser gespeicherten Projektversion.',
      checksHeading = 'Projekt prüfen',
      releaseHeading = 'Spielbare Ausgabe',
      notEvaluatedLabel = 'Nicht geprüft',
      checkingLabel = 'Wird geprüft',
      passedLabel = 'Geprüft',
      needsAttentionLabel = 'Bitte prüfen',
      blockedLabel = 'Blockiert',
      unavailableLabel = 'Nicht verfügbar',
      availableLabel = 'Verfügbar',
      evidenceLabel = 'Nachweis',
      staleEvidenceDescription =
          'Dieses Ergebnis gehört zu einer anderen Projektversion. Bitte prüfe den Bereich erneut.',
      actionNotConnectedDescription =
          'Ein Nachweis ist vorhanden, aber diese Aktion ist hier noch nicht verfügbar.',
      problemsHeading = 'Probleme beheben',
      voiceContinuationHeading = 'Sprachausgabe prüfen';

  final String title;
  final String description;
  final String evidenceBoundary;
  final String checksHeading;
  final String releaseHeading;
  final String notEvaluatedLabel;
  final String checkingLabel;
  final String passedLabel;
  final String needsAttentionLabel;
  final String blockedLabel;
  final String unavailableLabel;
  final String availableLabel;
  final String evidenceLabel;
  final String staleEvidenceDescription;
  final String actionNotConnectedDescription;
  final String problemsHeading;
  final String voiceContinuationHeading;
}

/// Initial/deep-link focus within [Revision3TestReleaseWorkspace].
enum Revision3TestReleaseFocus { overview, checks, release, problems, voice }

/// Honest, presentation-only Test & Release surface for one exact managed
/// revision-3 checkpoint.
///
/// The six required cards are deliberately separate: project structure,
/// scripts, Voice, DataAssets, playable build, and deployment. This widget
/// never infers readiness from missing inputs and never combines unevaluated
/// checks into an aggregate "Ready" result.
@immutable
final class Revision3TestReleaseWorkspace extends StatefulWidget {
  Revision3TestReleaseWorkspace({
    required String projectId,
    required int projectRevision,
    required String checkpointIdentity,
    required this.projectStructure,
    required this.scripts,
    required this.voice,
    required this.dataAssets,
    required this.playableBuild,
    required this.deployment,
    this.copy = const Revision3TestReleaseCopy.english(),
    this.focus = Revision3TestReleaseFocus.overview,
    this.problemsBuilder,
    this.voiceContinuationBuilder,
    super.key,
  }) : projectId = _requireText(projectId, 'projectId'),
       projectRevision = _requireRevision(projectRevision),
       checkpointIdentity = _requireText(
         checkpointIdentity,
         'checkpointIdentity',
       );

  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3TestReleaseCheck projectStructure;
  final Revision3TestReleaseCheck scripts;
  final Revision3TestReleaseCheck voice;
  final Revision3TestReleaseCheck dataAssets;
  final Revision3TestReleaseCapability playableBuild;
  final Revision3TestReleaseCapability deployment;
  final Revision3TestReleaseCopy copy;
  final Revision3TestReleaseFocus focus;

  /// Optional current Problems surface. Supplying this builder grants no
  /// readiness or mutation authority.
  final WidgetBuilder? problemsBuilder;

  /// Optional current Voice readiness/build continuation surface. Supplying
  /// this builder grants no general build or deployment authority.
  final WidgetBuilder? voiceContinuationBuilder;

  @override
  State<Revision3TestReleaseWorkspace> createState() =>
      _Revision3TestReleaseWorkspaceState();
}

class _Revision3TestReleaseWorkspaceState
    extends State<Revision3TestReleaseWorkspace> {
  final GlobalKey _overviewTarget = GlobalKey();
  final GlobalKey _checksTarget = GlobalKey();
  final GlobalKey _releaseTarget = GlobalKey();
  final GlobalKey _problemsTarget = GlobalKey();
  final GlobalKey _voiceTarget = GlobalKey();

  @override
  void initState() {
    super.initState();
    _scheduleFocus();
  }

  @override
  void didUpdateWidget(covariant Revision3TestReleaseWorkspace oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.focus != widget.focus ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.checkpointIdentity != widget.checkpointIdentity) {
      _scheduleFocus();
    }
  }

  void _scheduleFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final target = switch (widget.focus) {
        Revision3TestReleaseFocus.overview => _overviewTarget.currentContext,
        Revision3TestReleaseFocus.checks => _checksTarget.currentContext,
        Revision3TestReleaseFocus.release => _releaseTarget.currentContext,
        Revision3TestReleaseFocus.problems => _problemsTarget.currentContext,
        Revision3TestReleaseFocus.voice => _voiceTarget.currentContext,
      };
      if (target == null) return;
      Scrollable.ensureVisible(target, duration: Duration.zero, alignment: 0);
    });
  }

  @override
  Widget build(BuildContext context) {
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final checkpointIdentity = widget.checkpointIdentity;
    final projectStructure = widget.projectStructure;
    final scripts = widget.scripts;
    final voice = widget.voice;
    final dataAssets = widget.dataAssets;
    final playableBuild = widget.playableBuild;
    final deployment = widget.deployment;
    final copy = widget.copy;
    final problemsBuilder = widget.problemsBuilder;
    final voiceContinuationBuilder = widget.voiceContinuationBuilder;

    return LayoutBuilder(
      builder: (context, constraints) {
        final availableWidth = constraints.maxWidth.isFinite
            ? constraints.maxWidth
            : _maximumContentWidth;
        final horizontalPadding = switch (availableWidth) {
          < 480 => 12.0,
          < 900 => 20.0,
          _ => 32.0,
        };
        final innerWidth = (availableWidth - (horizontalPadding * 2))
            .clamp(0.0, _maximumContentWidth)
            .toDouble();
        final cardWidth = innerWidth >= 760
            ? (innerWidth - 16) / 2
            : innerWidth;

        return SingleChildScrollView(
          key: const Key('revision3-test-release-scroll'),
          padding: EdgeInsets.symmetric(
            horizontal: horizontalPadding,
            vertical: availableWidth < 480 ? 16 : 28,
          ),
          child: Align(
            alignment: Alignment.topCenter,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: _maximumContentWidth),
              child: Semantics(
                key: const Key('revision3-test-release-workspace'),
                container: true,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    KeyedSubtree(
                      key: _overviewTarget,
                      child: _WorkspaceHeader(copy: copy),
                    ),
                    const SizedBox(height: 28),
                    KeyedSubtree(
                      key: _checksTarget,
                      child: _SectionHeading(
                        key: const Key('revision3-test-release-checks-heading'),
                        text: copy.checksHeading,
                      ),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      key: const Key('revision3-test-release-checks'),
                      spacing: 16,
                      runSpacing: 16,
                      children: [
                        SizedBox(
                          width: cardWidth,
                          child: _CheckCard(
                            id: 'project-structure',
                            icon: Icons.account_tree_outlined,
                            expectedScope: Revision3TestReleaseEvidenceScope
                                .projectStructure,
                            check: projectStructure,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                        SizedBox(
                          width: cardWidth,
                          child: _CheckCard(
                            id: 'scripts',
                            icon: Icons.code_outlined,
                            expectedScope:
                                Revision3TestReleaseEvidenceScope.scripts,
                            check: scripts,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                        SizedBox(
                          width: cardWidth,
                          child: _CheckCard(
                            id: 'voice',
                            icon: Icons.record_voice_over_outlined,
                            expectedScope:
                                Revision3TestReleaseEvidenceScope.voice,
                            check: voice,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                        SizedBox(
                          width: cardWidth,
                          child: _CheckCard(
                            id: 'data-assets',
                            icon: Icons.data_object_outlined,
                            expectedScope:
                                Revision3TestReleaseEvidenceScope.dataAssets,
                            check: dataAssets,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 28),
                    KeyedSubtree(
                      key: _releaseTarget,
                      child: _SectionHeading(
                        key: const Key(
                          'revision3-test-release-release-heading',
                        ),
                        text: copy.releaseHeading,
                      ),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      key: const Key('revision3-test-release-capabilities'),
                      spacing: 16,
                      runSpacing: 16,
                      children: [
                        SizedBox(
                          width: cardWidth,
                          child: _CapabilityCard(
                            id: 'playable-build',
                            icon: Icons.inventory_2_outlined,
                            expectedScope:
                                Revision3TestReleaseEvidenceScope.playableBuild,
                            capability: playableBuild,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                        SizedBox(
                          width: cardWidth,
                          child: _CapabilityCard(
                            id: 'deployment',
                            icon: Icons.install_desktop_outlined,
                            expectedScope:
                                Revision3TestReleaseEvidenceScope.deployment,
                            capability: deployment,
                            projectId: projectId,
                            projectRevision: projectRevision,
                            checkpointIdentity: checkpointIdentity,
                            copy: copy,
                          ),
                        ),
                      ],
                    ),
                    if (problemsBuilder != null) ...[
                      const SizedBox(height: 28),
                      KeyedSubtree(
                        key: _problemsTarget,
                        child: _ContinuationSection(
                          key: const Key(
                            'revision3-test-release-problems-slot',
                          ),
                          heading: copy.problemsHeading,
                          child: problemsBuilder(context),
                        ),
                      ),
                    ],
                    if (voiceContinuationBuilder != null) ...[
                      const SizedBox(height: 28),
                      KeyedSubtree(
                        key: _voiceTarget,
                        child: _ContinuationSection(
                          key: const Key(
                            'revision3-test-release-voice-continuation-slot',
                          ),
                          heading: copy.voiceContinuationHeading,
                          child: voiceContinuationBuilder(context),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _WorkspaceHeader extends StatelessWidget {
  const _WorkspaceHeader({required this.copy});

  final Revision3TestReleaseCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key('revision3-test-release-header'),
      color: scheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(20),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(22),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                DecoratedBox(
                  decoration: BoxDecoration(
                    color: scheme.primaryContainer,
                    borderRadius: BorderRadius.circular(14),
                  ),
                  child: SizedBox.square(
                    dimension: 48,
                    child: Icon(
                      Icons.fact_check_outlined,
                      color: scheme.onPrimaryContainer,
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Semantics(
                        header: true,
                        child: Text(
                          copy.title,
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(copy.description),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Semantics(
              key: const Key('revision3-test-release-evidence-boundary'),
              container: true,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: scheme.tertiaryContainer,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        Icons.info_outline,
                        color: scheme.onTertiaryContainer,
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          copy.evidenceBoundary,
                          style: TextStyle(color: scheme.onTertiaryContainer),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SectionHeading extends StatelessWidget {
  const _SectionHeading({required this.text, super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Semantics(
    header: true,
    child: Text(text, style: Theme.of(context).textTheme.titleLarge),
  );
}

class _CheckCard extends StatelessWidget {
  const _CheckCard({
    required this.id,
    required this.icon,
    required this.expectedScope,
    required this.check,
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.copy,
  });

  final String id;
  final IconData icon;
  final Revision3TestReleaseEvidenceScope expectedScope;
  final Revision3TestReleaseCheck check;
  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3TestReleaseCopy copy;

  @override
  Widget build(BuildContext context) {
    final hasCurrentEvidence =
        check.evidence?.belongsTo(
          projectId: projectId,
          projectRevision: projectRevision,
          checkpointIdentity: checkpointIdentity,
          scope: expectedScope,
        ) ??
        false;
    final evaluated = _isEvaluated(check.state);
    final stale = evaluated && !hasCurrentEvidence;
    final state = stale
        ? Revision3TestReleaseCheckState.notEvaluated
        : check.state;
    final visual = _CheckVisual.forState(context, copy, state);
    final detail = stale ? copy.staleEvidenceDescription : check.description;
    final evidence = evaluated && hasCurrentEvidence
        ? check.evidence?.summary
        : null;

    return Material(
      color: visual.background,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      clipBehavior: Clip.antiAlias,
      child: Semantics(
        key: Key('revision3-test-release-$id-card'),
        container: true,
        explicitChildNodes: true,
        label: check.title,
        value: visual.label,
        hint: detail,
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _CardTitle(
                icon: icon,
                title: check.title,
                statusLabel: visual.label,
                foreground: visual.foreground,
              ),
              const SizedBox(height: 12),
              Text(detail),
              if (evidence != null) ...[
                const SizedBox(height: 12),
                _EvidenceLine(copy: copy, summary: evidence),
              ],
              if (check.actionLabel != null) ...[
                const SizedBox(height: 16),
                Align(
                  alignment: Alignment.centerLeft,
                  child: OutlinedButton.icon(
                    key: Key('revision3-test-release-$id-action'),
                    onPressed: check.onPressed,
                    icon: const Icon(Icons.arrow_forward_outlined),
                    label: Text(check.actionLabel!),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _CapabilityCard extends StatelessWidget {
  const _CapabilityCard({
    required this.id,
    required this.icon,
    required this.expectedScope,
    required this.capability,
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.copy,
  });

  final String id;
  final IconData icon;
  final Revision3TestReleaseEvidenceScope expectedScope;
  final Revision3TestReleaseCapability capability;
  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3TestReleaseCopy copy;

  @override
  Widget build(BuildContext context) {
    final evidenceCurrent =
        capability.evidence?.belongsTo(
          projectId: projectId,
          projectRevision: projectRevision,
          checkpointIdentity: checkpointIdentity,
          scope: expectedScope,
        ) ??
        false;
    final enabled = evidenceCurrent && capability.onPressed != null;
    final scheme = Theme.of(context).colorScheme;
    final background = enabled
        ? scheme.primaryContainer
        : evidenceCurrent
        ? scheme.surfaceContainerHighest
        : scheme.errorContainer;
    final foreground = enabled
        ? scheme.onPrimaryContainer
        : evidenceCurrent
        ? scheme.onSurfaceVariant
        : scheme.onErrorContainer;
    final statusLabel = enabled
        ? copy.availableLabel
        : evidenceCurrent
        ? copy.unavailableLabel
        : copy.blockedLabel;
    final detail = !evidenceCurrent
        ? capability.blockedReason
        : capability.onPressed == null
        ? copy.actionNotConnectedDescription
        : capability.description;

    return Material(
      color: background,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      clipBehavior: Clip.antiAlias,
      child: Semantics(
        key: Key('revision3-test-release-$id-card'),
        container: true,
        explicitChildNodes: true,
        label: capability.title,
        value: statusLabel,
        hint: detail,
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _CardTitle(
                icon: icon,
                title: capability.title,
                statusLabel: statusLabel,
                foreground: foreground,
              ),
              const SizedBox(height: 12),
              Text(detail),
              if (evidenceCurrent) ...[
                const SizedBox(height: 12),
                _EvidenceLine(
                  copy: copy,
                  summary: capability.evidence!.summary,
                ),
              ],
              const SizedBox(height: 16),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.icon(
                  key: Key('revision3-test-release-$id-action'),
                  onPressed: enabled ? capability.onPressed : null,
                  icon: Icon(
                    id == 'deployment'
                        ? Icons.install_desktop_outlined
                        : Icons.build_outlined,
                  ),
                  label: Text(capability.actionLabel),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CardTitle extends StatelessWidget {
  const _CardTitle({
    required this.icon,
    required this.title,
    required this.statusLabel,
    required this.foreground,
  });

  final IconData icon;
  final String title;
  final String statusLabel;
  final Color foreground;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, color: foreground),
          const SizedBox(width: 10),
          Expanded(
            child: Text(title, style: Theme.of(context).textTheme.titleMedium),
          ),
        ],
      ),
      const SizedBox(height: 10),
      DecoratedBox(
        decoration: BoxDecoration(
          color: foreground.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
          child: Text(
            statusLabel,
            style: Theme.of(
              context,
            ).textTheme.labelMedium?.copyWith(color: foreground),
          ),
        ),
      ),
    ],
  );
}

class _EvidenceLine extends StatelessWidget {
  const _EvidenceLine({required this.copy, required this.summary});

  final Revision3TestReleaseCopy copy;
  final String summary;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(Icons.verified_outlined, size: 18, color: scheme.primary),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            '${copy.evidenceLabel}: $summary',
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
      ],
    );
  }
}

class _ContinuationSection extends StatelessWidget {
  const _ContinuationSection({
    required this.heading,
    required this.child,
    super.key,
  });

  final String heading;
  final Widget child;

  @override
  Widget build(BuildContext context) => Semantics(
    container: true,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _SectionHeading(text: heading),
        const SizedBox(height: 12),
        child,
      ],
    ),
  );
}

final class _CheckVisual {
  const _CheckVisual({
    required this.label,
    required this.background,
    required this.foreground,
  });

  factory _CheckVisual.forState(
    BuildContext context,
    Revision3TestReleaseCopy copy,
    Revision3TestReleaseCheckState state,
  ) {
    final scheme = Theme.of(context).colorScheme;
    return switch (state) {
      Revision3TestReleaseCheckState.notEvaluated => _CheckVisual(
        label: copy.notEvaluatedLabel,
        background: scheme.surfaceContainerHighest,
        foreground: scheme.onSurfaceVariant,
      ),
      Revision3TestReleaseCheckState.checking => _CheckVisual(
        label: copy.checkingLabel,
        background: scheme.secondaryContainer,
        foreground: scheme.onSecondaryContainer,
      ),
      Revision3TestReleaseCheckState.passed => _CheckVisual(
        label: copy.passedLabel,
        background: scheme.primaryContainer,
        foreground: scheme.onPrimaryContainer,
      ),
      Revision3TestReleaseCheckState.needsAttention => _CheckVisual(
        label: copy.needsAttentionLabel,
        background: scheme.tertiaryContainer,
        foreground: scheme.onTertiaryContainer,
      ),
      Revision3TestReleaseCheckState.blocked => _CheckVisual(
        label: copy.blockedLabel,
        background: scheme.errorContainer,
        foreground: scheme.onErrorContainer,
      ),
      Revision3TestReleaseCheckState.unavailable => _CheckVisual(
        label: copy.unavailableLabel,
        background: scheme.surfaceContainerHighest,
        foreground: scheme.onSurfaceVariant,
      ),
    };
  }

  final String label;
  final Color background;
  final Color foreground;
}

bool _isEvaluated(Revision3TestReleaseCheckState state) => switch (state) {
  Revision3TestReleaseCheckState.passed ||
  Revision3TestReleaseCheckState.needsAttention ||
  Revision3TestReleaseCheckState.blocked => true,
  _ => false,
};

Revision3TestReleaseEvidence? _validateEvidenceForState(
  Revision3TestReleaseCheckState state,
  Revision3TestReleaseEvidence? evidence,
) {
  if (_isEvaluated(state) && evidence == null) {
    throw ArgumentError.value(
      evidence,
      'evidence',
      'is required for an evaluated check state',
    );
  }
  if (!_isEvaluated(state) && evidence != null) {
    throw ArgumentError.value(
      evidence,
      'evidence',
      'is only valid for an evaluated check state',
    );
  }
  return evidence;
}

String _requireText(String value, String name) {
  final normalized = value.trim();
  if (normalized.isEmpty) {
    throw ArgumentError.value(value, name, 'must not be empty');
  }
  return normalized;
}

String? _optionalText(String? value, String name) =>
    value == null ? null : _requireText(value, name);

int _requireRevision(int value) {
  if (value < 0) {
    throw ArgumentError.value(value, 'projectRevision', 'must not be negative');
  }
  return value;
}

const double _maximumContentWidth = 1180;
