import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_content_index.dart';

typedef Revision3ProjectDashboardLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3ProjectDashboardActionTextBuilder =
    String Function(Revision3ContentIndex index);

/// All author-facing framework, status, count, semantics, and error copy used
/// by [Revision3ProjectDashboard]. Project metadata itself comes from the
/// exact-current [Revision3ContentIndex].
@immutable
final class Revision3ProjectDashboardCopy {
  const Revision3ProjectDashboardCopy({
    required this.untitledProjectLabel,
    required this.draftStatusLabel,
    required this.projectVersionLabel,
    required this.projectAuthorLabel,
    required this.notProvidedLabel,
    required this.contentCountsHeading,
    required this.npcDraftCountLabel,
    required this.questDraftCountLabel,
    required this.dialogLineCountLabel,
    required this.voiceTakeCountLabel,
    required this.assetCountLabel,
    required this.unresolvedReferenceCountLabel,
    required this.readinessHeading,
    required this.offlineAuthoringTitle,
    required this.offlineAuthoringDescription,
    required this.generalBuildBlockedTitle,
    required this.generalBuildBlockedDescription,
    required this.runtimeUnqualifiedTitle,
    required this.runtimeUnqualifiedDescription,
    required this.referenceIntegrityTitle,
    required this.referenceIntegrityDescription,
    required this.missingGameTitle,
    required this.missingGameDescription,
    required this.continueHeading,
    required this.loadingSemanticsLabel,
    required this.loadErrorSemanticsLabel,
    required this.loadErrorTitle,
    required this.loadErrorDescription,
    required this.retryLabel,
  });

  final String untitledProjectLabel;
  final String draftStatusLabel;
  final String projectVersionLabel;
  final String projectAuthorLabel;
  final String notProvidedLabel;
  final String contentCountsHeading;
  final String npcDraftCountLabel;
  final String questDraftCountLabel;
  final String dialogLineCountLabel;
  final String voiceTakeCountLabel;
  final String assetCountLabel;
  final String unresolvedReferenceCountLabel;
  final String readinessHeading;
  final String offlineAuthoringTitle;
  final String offlineAuthoringDescription;
  final String generalBuildBlockedTitle;
  final String generalBuildBlockedDescription;
  final String runtimeUnqualifiedTitle;
  final String runtimeUnqualifiedDescription;
  final String referenceIntegrityTitle;
  final String referenceIntegrityDescription;
  final String missingGameTitle;
  final String missingGameDescription;
  final String continueHeading;
  final String loadingSemanticsLabel;
  final String loadErrorSemanticsLabel;
  final String loadErrorTitle;
  final String loadErrorDescription;
  final String retryLabel;
}

/// One localized, stable dashboard action. A null callback keeps the action
/// visible but unavailable, preserving capability discoverability.
@immutable
final class Revision3ProjectDashboardAction {
  const Revision3ProjectDashboardAction({
    required this.id,
    required this.icon,
    required this.title,
    required this.description,
    required this.onPressed,
    this.controlKey,
    this.enabledFor,
    this.disabledReason,
    this.titleBuilder,
    this.descriptionBuilder,
  }) : assert(id != '');

  final String id;
  final IconData icon;
  final String title;
  final String description;
  final VoidCallback? onPressed;
  final Key? controlKey;

  /// Optional content-aware gate evaluated only after the exact-current
  /// project index has loaded.
  ///
  /// Keeping the action visible makes an unavailable workflow discoverable,
  /// while [disabledReason] explains the concrete prerequisite instead of
  /// presenting a button that can only fail in its first dialog.
  final bool Function(Revision3ContentIndex index)? enabledFor;
  final String? disabledReason;

  /// Optional exact-index-aware copy for task-first Home surfaces.
  ///
  /// These builders run only after the dashboard has accepted an exact index
  /// for its requested project and revision. The static copy remains the
  /// fallback and is also used by contexts such as the missing-game banner,
  /// where no content-dependent task copy is needed.
  final Revision3ProjectDashboardActionTextBuilder? titleBuilder;
  final Revision3ProjectDashboardActionTextBuilder? descriptionBuilder;
}

/// Content-first overview for one exact managed revision-3 checkpoint.
///
/// This surface reports only semantic project counts, exact reference
/// integrity, and deliberately closed readiness boundaries. It owns no
/// mutation, build, deployment, runtime, game-path, or save authority.
final class Revision3ProjectDashboard extends StatefulWidget {
  Revision3ProjectDashboard({
    required this.projectId,
    required this.projectRevision,
    required this.load,
    required this.gameConfigured,
    required this.copy,
    required List<Revision3ProjectDashboardAction> tasks,
    this.settingsAction,
    super.key,
  }) : tasks = List.unmodifiable(tasks),
       assert(projectId != ''),
       assert(projectRevision >= 0),
       assert(_actionIdsAreUnique(tasks, settingsAction));

  final String projectId;
  final int projectRevision;
  final Revision3ProjectDashboardLoader load;
  final bool gameConfigured;
  final Revision3ProjectDashboardCopy copy;
  final List<Revision3ProjectDashboardAction> tasks;
  final Revision3ProjectDashboardAction? settingsAction;

  @override
  State<Revision3ProjectDashboard> createState() =>
      _Revision3ProjectDashboardState();
}

class _Revision3ProjectDashboardState extends State<Revision3ProjectDashboard> {
  Revision3ContentIndex? _index;
  bool _loading = true;
  bool _loadFailed = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _startLoad(notify: false);
  }

  @override
  void didUpdateWidget(covariant Revision3ProjectDashboard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision) {
      _startLoad(notify: false);
    }
  }

  @override
  void dispose() {
    _loadGeneration++;
    super.dispose();
  }

  void _startLoad({required bool notify}) {
    final generation = ++_loadGeneration;
    final expectedProjectId = widget.projectId;
    final expectedProjectRevision = widget.projectRevision;
    final loader = widget.load;

    void markLoading() {
      _index = null;
      _loading = true;
      _loadFailed = false;
    }

    if (notify) {
      setState(markLoading);
    } else {
      markLoading();
    }

    unawaited(
      _finishLoad(
        generation: generation,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        loader: loader,
      ),
    );
  }

  Future<void> _finishLoad({
    required int generation,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required Revision3ProjectDashboardLoader loader,
  }) async {
    try {
      final index = await loader();
      if (!mounted || generation != _loadGeneration) return;
      if (index.projectId != expectedProjectId ||
          index.projectRevision != expectedProjectRevision) {
        _showSanitizedLoadFailure(generation);
        return;
      }
      setState(() {
        _index = index;
        _loading = false;
        _loadFailed = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      _showSanitizedLoadFailure(generation);
    }
  }

  void _showSanitizedLoadFailure(int generation) {
    if (!mounted || generation != _loadGeneration) return;
    setState(() {
      _index = null;
      _loading = false;
      _loadFailed = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return Center(
        key: const Key('revision3-project-dashboard-loading'),
        child: Semantics(
          liveRegion: true,
          label: widget.copy.loadingSemanticsLabel,
          child: const CircularProgressIndicator(),
        ),
      );
    }
    if (_loadFailed || _index == null) {
      return _DashboardLoadError(
        copy: widget.copy,
        onRetry: () => _startLoad(notify: true),
      );
    }
    return _DashboardContent(
      index: _index!,
      gameConfigured: widget.gameConfigured,
      copy: widget.copy,
      tasks: widget.tasks,
      settingsAction: widget.settingsAction,
    );
  }
}

class _DashboardContent extends StatelessWidget {
  const _DashboardContent({
    required this.index,
    required this.gameConfigured,
    required this.copy,
    required this.tasks,
    required this.settingsAction,
  });

  final Revision3ContentIndex index;
  final bool gameConfigured;
  final Revision3ProjectDashboardCopy copy;
  final List<Revision3ProjectDashboardAction> tasks;
  final Revision3ProjectDashboardAction? settingsAction;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final width = constraints.maxWidth.isFinite
          ? constraints.maxWidth
          : 1200.0;
      final edgePadding = width < 520 ? 12.0 : 20.0;
      final contentWidth = width > edgePadding * 2
          ? width - edgePadding * 2
          : width;
      return SingleChildScrollView(
        key: const Key('revision3-project-dashboard-scroll'),
        padding: EdgeInsets.all(edgePadding),
        child: Column(
          key: const Key('revision3-project-dashboard'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _ProjectHeader(index: index, copy: copy),
            if (!gameConfigured) ...[
              const SizedBox(height: 16),
              _MissingGameBanner(copy: copy, settingsAction: settingsAction),
            ],
            if (tasks.isNotEmpty) ...[
              const SizedBox(height: 20),
              _SectionHeading(copy.continueHeading),
              const SizedBox(height: 10),
              _TaskList(index: index, tasks: tasks),
            ],
            const SizedBox(height: 20),
            _SectionHeading(copy.contentCountsHeading),
            const SizedBox(height: 10),
            _CountGrid(index: index, copy: copy, availableWidth: contentWidth),
            const SizedBox(height: 24),
            _SectionHeading(copy.readinessHeading),
            const SizedBox(height: 10),
            _StatusGrid(index: index, copy: copy, availableWidth: contentWidth),
          ],
        ),
      );
    },
  );
}

class _ProjectHeader extends StatelessWidget {
  const _ProjectHeader({required this.index, required this.copy});

  final Revision3ContentIndex index;
  final Revision3ProjectDashboardCopy copy;

  @override
  Widget build(BuildContext context) => Card(
    key: const Key('revision3-project-dashboard-header'),
    margin: EdgeInsets.zero,
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final identity = Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Semantics(
                header: true,
                child: Text(
                  index.projectName.isEmpty
                      ? copy.untitledProjectLabel
                      : index.projectName,
                  key: const Key('revision3-project-dashboard-project-name'),
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
              ),
              const SizedBox(height: 14),
              Wrap(
                spacing: 24,
                runSpacing: 12,
                children: [
                  _ProjectFact(
                    key: const Key(
                      'revision3-project-dashboard-project-version',
                    ),
                    label: copy.projectVersionLabel,
                    value: index.projectVersion.isEmpty
                        ? copy.notProvidedLabel
                        : index.projectVersion,
                  ),
                  _ProjectFact(
                    key: const Key(
                      'revision3-project-dashboard-project-author',
                    ),
                    label: copy.projectAuthorLabel,
                    value: index.projectAuthor.isEmpty
                        ? copy.notProvidedLabel
                        : index.projectAuthor,
                  ),
                ],
              ),
            ],
          );
          final status = Chip(
            key: const Key('revision3-project-dashboard-draft-status'),
            avatar: const Icon(Icons.edit_note_outlined, size: 18),
            label: Text(copy.draftStatusLabel),
          );
          if (constraints.maxWidth < 560) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [identity, const SizedBox(height: 14), status],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(child: identity),
              const SizedBox(width: 16),
              status,
            ],
          );
        },
      ),
    ),
  );
}

class _ProjectFact extends StatelessWidget {
  const _ProjectFact({required this.label, required this.value, super.key});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(label, style: Theme.of(context).textTheme.labelLarge),
      const SizedBox(height: 2),
      Text(value),
    ],
  );
}

class _MissingGameBanner extends StatelessWidget {
  const _MissingGameBanner({required this.copy, required this.settingsAction});

  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardAction? settingsAction;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-project-dashboard-missing-game'),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: scheme.tertiaryContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final message = Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.info_outline, color: scheme.onTertiaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      copy.missingGameTitle,
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        color: scheme.onTertiaryContainer,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      copy.missingGameDescription,
                      style: TextStyle(color: scheme.onTertiaryContainer),
                    ),
                  ],
                ),
              ),
            ],
          );
          final action = settingsAction;
          if (action == null) return message;
          final button = Semantics(
            button: true,
            enabled: action.onPressed != null,
            label: action.description,
            child: OutlinedButton.icon(
              key: const Key('revision3-project-dashboard-settings-action'),
              onPressed: action.onPressed,
              icon: Icon(action.icon),
              label: Text(action.title),
            ),
          );
          if (constraints.maxWidth < 620) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                message,
                const SizedBox(height: 12),
                Align(alignment: Alignment.centerLeft, child: button),
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Expanded(child: message),
              const SizedBox(width: 16),
              button,
            ],
          );
        },
      ),
    );
  }
}

class _SectionHeading extends StatelessWidget {
  const _SectionHeading(this.label);

  final String label;

  @override
  Widget build(BuildContext context) => Semantics(
    header: true,
    child: Text(label, style: Theme.of(context).textTheme.titleLarge),
  );
}

class _CountGrid extends StatelessWidget {
  const _CountGrid({
    required this.index,
    required this.copy,
    required this.availableWidth,
  });

  final Revision3ContentIndex index;
  final Revision3ProjectDashboardCopy copy;
  final double availableWidth;

  @override
  Widget build(BuildContext context) {
    final counts = <({String id, String label, int value, IconData icon})>[
      (
        id: 'npc-drafts',
        label: copy.npcDraftCountLabel,
        value: _entityCount(index, Revision3ContentEntityKind.npcDraft),
        icon: Icons.person_outline,
      ),
      (
        id: 'quest-drafts',
        label: copy.questDraftCountLabel,
        value: _entityCount(index, Revision3ContentEntityKind.questDraft),
        icon: Icons.assignment_outlined,
      ),
      (
        id: 'dialog-lines',
        label: copy.dialogLineCountLabel,
        value: _entityCount(index, Revision3ContentEntityKind.dialogLine),
        icon: Icons.chat_bubble_outline,
      ),
      (
        id: 'voice-takes',
        label: copy.voiceTakeCountLabel,
        value: _entityCount(index, Revision3ContentEntityKind.voiceTake),
        icon: Icons.graphic_eq_outlined,
      ),
      (
        id: 'assets',
        label: copy.assetCountLabel,
        value: index.assets.length,
        icon: Icons.inventory_2_outlined,
      ),
      (
        id: 'unresolved-references',
        label: copy.unresolvedReferenceCountLabel,
        value: index.problemCount,
        icon: Icons.account_tree_outlined,
      ),
    ];
    final tileWidth = _responsiveTileWidth(
      availableWidth,
      wideColumns: 6,
      mediumColumns: 3,
    );
    return Wrap(
      key: const Key('revision3-project-dashboard-counts'),
      spacing: 12,
      runSpacing: 12,
      children: [
        for (final count in counts)
          SizedBox(
            width: tileWidth,
            child: Card(
              key: Key('revision3-project-dashboard-count-${count.id}'),
              margin: EdgeInsets.zero,
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: Row(
                  children: [
                    Icon(count.icon),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            '${count.value}',
                            key: Key(
                              'revision3-project-dashboard-count-${count.id}-value',
                            ),
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                          Text(count.label),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _StatusGrid extends StatelessWidget {
  const _StatusGrid({
    required this.index,
    required this.copy,
    required this.availableWidth,
  });

  final Revision3ContentIndex index;
  final Revision3ProjectDashboardCopy copy;
  final double availableWidth;

  @override
  Widget build(BuildContext context) {
    final statuses =
        <({String id, String title, String description, IconData icon})>[
          (
            id: 'offline-authoring',
            title: copy.offlineAuthoringTitle,
            description: copy.offlineAuthoringDescription,
            icon: Icons.edit_note_outlined,
          ),
          (
            id: 'general-build-blocked',
            title: copy.generalBuildBlockedTitle,
            description: copy.generalBuildBlockedDescription,
            icon: Icons.block_outlined,
          ),
          (
            id: 'runtime-unqualified',
            title: copy.runtimeUnqualifiedTitle,
            description: copy.runtimeUnqualifiedDescription,
            icon: Icons.science_outlined,
          ),
          (
            id: 'reference-integrity',
            title: copy.referenceIntegrityTitle,
            description: copy.referenceIntegrityDescription,
            icon: index.problemCount == 0
                ? Icons.account_tree_outlined
                : Icons.warning_amber_rounded,
          ),
        ];
    final tileWidth = _responsiveTileWidth(
      availableWidth,
      wideColumns: 4,
      mediumColumns: 2,
    );
    return Wrap(
      key: const Key('revision3-project-dashboard-statuses'),
      spacing: 12,
      runSpacing: 12,
      children: [
        for (final status in statuses)
          SizedBox(
            width: tileWidth,
            child: Card(
              key: Key('revision3-project-dashboard-status-${status.id}'),
              margin: EdgeInsets.zero,
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(status.icon),
                    const SizedBox(height: 10),
                    Text(
                      status.title,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 6),
                    Text(status.description),
                    if (status.id == 'reference-integrity') ...[
                      const SizedBox(height: 10),
                      Row(
                        children: [
                          Text(
                            '${index.problemCount}',
                            key: const Key(
                              'revision3-project-dashboard-reference-status-count',
                            ),
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                          const SizedBox(width: 6),
                          Expanded(
                            child: Text(copy.unresolvedReferenceCountLabel),
                          ),
                        ],
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _TaskList extends StatelessWidget {
  const _TaskList({required this.index, required this.tasks});

  final Revision3ContentIndex index;
  final List<Revision3ProjectDashboardAction> tasks;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key('revision3-project-dashboard-tasks'),
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: scheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (var taskIndex = 0; taskIndex < tasks.length; taskIndex++) ...[
            if (taskIndex > 0) const Divider(height: 1),
            _DashboardTaskRow(index: index, action: tasks[taskIndex]),
          ],
        ],
      ),
    );
  }
}

class _DashboardTaskRow extends StatelessWidget {
  const _DashboardTaskRow({required this.index, required this.action});

  final Revision3ContentIndex index;
  final Revision3ProjectDashboardAction action;

  @override
  Widget build(BuildContext context) {
    final title = action.titleBuilder?.call(index) ?? action.title;
    final description =
        action.descriptionBuilder?.call(index) ?? action.description;
    final contentGateOpen = action.enabledFor?.call(index) ?? true;
    final enabled = action.onPressed != null && contentGateOpen;
    final effectiveDescription = !enabled && action.disabledReason != null
        ? action.disabledReason!
        : description;
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      button: true,
      enabled: enabled,
      label: title,
      hint: effectiveDescription,
      excludeSemantics: true,
      child: ListTile(
        key:
            action.controlKey ??
            Key('revision3-project-dashboard-task-${action.id}'),
        enabled: enabled,
        onTap: enabled ? action.onPressed : null,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        minVerticalPadding: 12,
        leading: Icon(action.icon),
        title: Text(title, style: Theme.of(context).textTheme.titleMedium),
        subtitle: Padding(
          padding: const EdgeInsets.only(top: 4),
          child: Text(effectiveDescription),
        ),
        trailing: Icon(
          enabled ? Icons.arrow_forward_outlined : Icons.lock_outline,
          color: enabled ? scheme.primary : scheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _DashboardLoadError extends StatelessWidget {
  const _DashboardLoadError({required this.copy, required this.onRetry});

  final Revision3ProjectDashboardCopy copy;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SingleChildScrollView(
      child: ConstrainedBox(
        constraints: BoxConstraints(minHeight: constraints.maxHeight),
        child: Center(
          child: Semantics(
            container: true,
            liveRegion: true,
            label: copy.loadErrorSemanticsLabel,
            child: ConstrainedBox(
              key: const Key('revision3-project-dashboard-error'),
              constraints: const BoxConstraints(maxWidth: 560),
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.error_outline,
                      size: 42,
                      color: Theme.of(context).colorScheme.error,
                    ),
                    const SizedBox(height: 12),
                    Text(
                      copy.loadErrorTitle,
                      textAlign: TextAlign.center,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      copy.loadErrorDescription,
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 16),
                    FilledButton.icon(
                      key: const Key('revision3-project-dashboard-retry'),
                      onPressed: onRetry,
                      icon: const Icon(Icons.refresh),
                      label: Text(copy.retryLabel),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

int _entityCount(
  Revision3ContentIndex index,
  Revision3ContentEntityKind kind,
) => index.entities.where((entity) => entity.kind == kind).length;

double _responsiveTileWidth(
  double availableWidth, {
  required int wideColumns,
  required int mediumColumns,
}) {
  final columns = switch (availableWidth) {
    >= 1100 => wideColumns,
    >= 620 => mediumColumns,
    _ => 1,
  };
  return (availableWidth - ((columns - 1) * 12)) / columns;
}

bool _actionIdsAreUnique(
  List<Revision3ProjectDashboardAction> tasks,
  Revision3ProjectDashboardAction? settingsAction,
) {
  final ids = <String>{};
  for (final action in <Revision3ProjectDashboardAction>[
    ...tasks,
    ?settingsAction,
  ]) {
    if (action.id.isEmpty || !ids.add(action.id)) return false;
  }
  return true;
}
