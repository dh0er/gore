import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_my_mod_changes.dart';

typedef Revision3ProjectDashboardLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3ProjectDashboardDataAssetLoader =
    Future<List<AuthoringRevision3DataAssetStage>> Function();
typedef Revision3ProjectDashboardActionTextBuilder =
    String Function(Revision3ContentIndex index);
typedef Revision3ProjectDashboardOpenEntity =
    FutureOr<void> Function(Revision3ContentEntity entity);
typedef Revision3ProjectDashboardOpenItemPatch =
    FutureOr<void> Function(String vanillaClass);
typedef Revision3ProjectDashboardOpenDataAsset =
    FutureOr<void> Function(AuthoringRevision3DataAssetStage stage);

/// Read-only exact-target navigation for the accepted "My mod / Changes"
/// snapshot. The dashboard never mutates project, game, build, or runtime state.
@immutable
final class Revision3ProjectDashboardChangeActions {
  const Revision3ProjectDashboardChangeActions({
    this.openEntity,
    this.openItemPatch,
    this.openDataAsset,
  });

  final Revision3ProjectDashboardOpenEntity? openEntity;
  final Revision3ProjectDashboardOpenItemPatch? openItemPatch;
  final Revision3ProjectDashboardOpenDataAsset? openDataAsset;
}

/// All author-facing framework, changes-list, semantics, and error copy used by
/// [Revision3ProjectDashboard]. Project metadata and semantic entries come
/// only from the exact-current accepted snapshot.
@immutable
final class Revision3ProjectDashboardCopy {
  const Revision3ProjectDashboardCopy({
    required this.untitledProjectLabel,
    required this.draftStatusLabel,
    required this.projectVersionLabel,
    required this.projectAuthorLabel,
    required this.notProvidedLabel,
    required this.contentCountsHeading,
    required this.changesDescription,
    required this.npcDraftCountLabel,
    required this.questDraftCountLabel,
    required this.dialogLineCountLabel,
    required this.voiceTakeCountLabel,
    required this.assetCountLabel,
    required this.itemPatchLabel,
    required this.localizationEntryLabel,
    required this.voiceSlotLabel,
    required this.generatedScriptLabel,
    required this.selectedVoiceTakeLabel,
    required this.technicalContentLabel,
    required this.technicalContentDescription,
    required this.emptyChangesTitle,
    required this.emptyChangesDescription,
    required this.openChangeLabel,
    required this.changeActionFailedMessage,
    required this.unresolvedReferenceCountLabel,
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
  final String changesDescription;
  final String npcDraftCountLabel;
  final String questDraftCountLabel;
  final String dialogLineCountLabel;
  final String voiceTakeCountLabel;
  final String assetCountLabel;
  final String itemPatchLabel;
  final String localizationEntryLabel;
  final String voiceSlotLabel;
  final String generatedScriptLabel;
  final String selectedVoiceTakeLabel;
  final String technicalContentLabel;
  final String technicalContentDescription;
  final String emptyChangesTitle;
  final String emptyChangesDescription;
  final String openChangeLabel;
  final String changeActionFailedMessage;
  final String unresolvedReferenceCountLabel;
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
/// This surface shows the actual saved author-facing changes and keeps
/// generated or unassigned helpers visible as technical content. It owns no
/// mutation, build, deployment, runtime, game-path, or save authority.
final class Revision3ProjectDashboard extends StatefulWidget {
  Revision3ProjectDashboard({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
    required this.load,
    required this.loadDataAssetStages,
    required this.gameConfigured,
    required this.copy,
    required List<Revision3ProjectDashboardAction> tasks,
    this.changeActions = const Revision3ProjectDashboardChangeActions(),
    this.settingsAction,
    super.key,
  }) : tasks = List.unmodifiable(tasks),
       assert(projectRoot != ''),
       assert(projectId != ''),
       assert(projectRevision >= 0),
       assert(projectHeadCanonicalJson != ''),
       assert(_actionIdsAreUnique(tasks, settingsAction));

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Revision3ProjectDashboardLoader load;
  final Revision3ProjectDashboardDataAssetLoader loadDataAssetStages;
  final bool gameConfigured;
  final Revision3ProjectDashboardCopy copy;
  final List<Revision3ProjectDashboardAction> tasks;
  final Revision3ProjectDashboardChangeActions changeActions;
  final Revision3ProjectDashboardAction? settingsAction;

  @override
  State<Revision3ProjectDashboard> createState() =>
      _Revision3ProjectDashboardState();
}

class _Revision3ProjectDashboardState extends State<Revision3ProjectDashboard> {
  Revision3ContentIndex? _index;
  Revision3MyModChanges? _changes;
  _Revision3DashboardCheckpoint? _loadedCheckpoint;
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
    if (oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHeadCanonicalJson != widget.projectHeadCanonicalJson) {
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
    final checkpoint = _checkpoint;
    final loader = widget.load;
    final dataAssetLoader = widget.loadDataAssetStages;

    void markLoading() {
      _index = null;
      _changes = null;
      _loadedCheckpoint = null;
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
        checkpoint: checkpoint,
        loader: loader,
        dataAssetLoader: dataAssetLoader,
      ),
    );
  }

  Future<void> _finishLoad({
    required int generation,
    required _Revision3DashboardCheckpoint checkpoint,
    required Revision3ProjectDashboardLoader loader,
    required Revision3ProjectDashboardDataAssetLoader dataAssetLoader,
  }) async {
    try {
      late Revision3ContentIndex index;
      late List<AuthoringRevision3DataAssetStage> dataAssetStages;
      await Future.wait<void>([
        () async {
          index = await loader();
        }(),
        () async {
          dataAssetStages = await dataAssetLoader();
        }(),
      ]);
      if (!mounted || generation != _loadGeneration) return;
      if (checkpoint != _checkpoint ||
          index.projectId != checkpoint.projectId ||
          index.projectRevision != checkpoint.projectRevision) {
        _showSanitizedLoadFailure(generation);
        return;
      }
      final changes = Revision3MyModChanges.fromExactCurrent(
        contentIndex: index,
        dataAssetStages: dataAssetStages,
      );
      setState(() {
        _index = index;
        _changes = changes;
        _loadedCheckpoint = checkpoint;
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
      _changes = null;
      _loadedCheckpoint = null;
      _loading = false;
      _loadFailed = true;
    });
  }

  _Revision3DashboardCheckpoint get _checkpoint =>
      _Revision3DashboardCheckpoint(
        projectRoot: widget.projectRoot,
        projectId: widget.projectId,
        projectRevision: widget.projectRevision,
        projectHeadCanonicalJson: widget.projectHeadCanonicalJson,
      );

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
    if (_loadFailed ||
        _index == null ||
        _changes == null ||
        _loadedCheckpoint != _checkpoint) {
      return _DashboardLoadError(
        copy: widget.copy,
        onRetry: () => _startLoad(notify: true),
      );
    }
    return _DashboardContent(
      index: _index!,
      changes: _changes!,
      gameConfigured: widget.gameConfigured,
      copy: widget.copy,
      tasks: widget.tasks,
      changeActions: _changeActionsAtCheckpoint(_loadedCheckpoint!),
      settingsAction: widget.settingsAction,
    );
  }

  Revision3ProjectDashboardChangeActions _changeActionsAtCheckpoint(
    _Revision3DashboardCheckpoint checkpoint,
  ) {
    final actions = widget.changeActions;
    return Revision3ProjectDashboardChangeActions(
      openEntity: actions.openEntity == null
          ? null
          : (entity) => _invokeAtCheckpoint(
              checkpoint,
              () => actions.openEntity!(entity),
            ),
      openItemPatch: actions.openItemPatch == null
          ? null
          : (vanillaClass) => _invokeAtCheckpoint(
              checkpoint,
              () => actions.openItemPatch!(vanillaClass),
            ),
      openDataAsset: actions.openDataAsset == null
          ? null
          : (stage) => _invokeAtCheckpoint(
              checkpoint,
              () => actions.openDataAsset!(stage),
            ),
    );
  }

  Future<void> _invokeAtCheckpoint(
    _Revision3DashboardCheckpoint checkpoint,
    FutureOr<void> Function() action,
  ) async {
    _requireCurrentSnapshot(checkpoint);
    await Future<void>.sync(action);
    _requireCurrentSnapshot(checkpoint);
  }

  void _requireCurrentSnapshot(_Revision3DashboardCheckpoint checkpoint) {
    if (!mounted ||
        checkpoint != _checkpoint ||
        _loadedCheckpoint != checkpoint ||
        _changes == null) {
      throw StateError('The exact project changes are no longer current.');
    }
  }
}

@immutable
final class _Revision3DashboardCheckpoint {
  const _Revision3DashboardCheckpoint({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;

  @override
  bool operator ==(Object other) =>
      other is _Revision3DashboardCheckpoint &&
      other.projectRoot == projectRoot &&
      other.projectId == projectId &&
      other.projectRevision == projectRevision &&
      other.projectHeadCanonicalJson == projectHeadCanonicalJson;

  @override
  int get hashCode => Object.hash(
    projectRoot,
    projectId,
    projectRevision,
    projectHeadCanonicalJson,
  );
}

class _DashboardContent extends StatelessWidget {
  const _DashboardContent({
    required this.index,
    required this.changes,
    required this.gameConfigured,
    required this.copy,
    required this.tasks,
    required this.changeActions,
    required this.settingsAction,
  });

  final Revision3ContentIndex index;
  final Revision3MyModChanges changes;
  final bool gameConfigured;
  final Revision3ProjectDashboardCopy copy;
  final List<Revision3ProjectDashboardAction> tasks;
  final Revision3ProjectDashboardChangeActions changeActions;
  final Revision3ProjectDashboardAction? settingsAction;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final width = constraints.maxWidth.isFinite
          ? constraints.maxWidth
          : 1200.0;
      final edgePadding = width < 520 ? 12.0 : 20.0;
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
            const SizedBox(height: 24),
            _SectionHeading(copy.contentCountsHeading),
            const SizedBox(height: 4),
            Text(copy.changesDescription),
            const SizedBox(height: 12),
            _MyModChangesList(
              changes: changes,
              copy: copy,
              actions: changeActions,
            ),
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

class _MyModChangesList extends StatelessWidget {
  const _MyModChangesList({
    required this.changes,
    required this.copy,
    required this.actions,
  });

  final Revision3MyModChanges changes;
  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardChangeActions actions;

  @override
  Widget build(BuildContext context) {
    final groups = <_MyModGroup, List<Revision3MyModEntry>>{};
    for (final entry in changes.changes) {
      groups
          .putIfAbsent(_groupFor(entry.kind), () => <Revision3MyModEntry>[])
          .add(entry);
    }
    return Column(
      key: const Key('revision3-project-dashboard-changes'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (groups.isEmpty)
          _EmptyChanges(copy: copy)
        else
          for (final group in _MyModGroup.values)
            if (groups[group] case final entries?)
              Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: _MyModGroupList(
                  group: group,
                  entries: entries,
                  copy: copy,
                  actions: actions,
                ),
              ),
        if (changes.technical.isNotEmpty)
          _TechnicalChanges(
            entries: changes.technical,
            copy: copy,
            actions: actions,
          ),
      ],
    );
  }
}

enum _MyModGroup { quests, npcs, items, dataAssets, dialog, text, voice }

_MyModGroup _groupFor(Revision3MyModContentKind kind) => switch (kind) {
  Revision3MyModContentKind.quest => _MyModGroup.quests,
  Revision3MyModContentKind.npc => _MyModGroup.npcs,
  Revision3MyModContentKind.itemPatch => _MyModGroup.items,
  Revision3MyModContentKind.dataAsset => _MyModGroup.dataAssets,
  Revision3MyModContentKind.dialogLine => _MyModGroup.dialog,
  Revision3MyModContentKind.localization => _MyModGroup.text,
  Revision3MyModContentKind.voiceSlot ||
  Revision3MyModContentKind.voiceTake ||
  Revision3MyModContentKind.generatedScript => _MyModGroup.voice,
};

class _EmptyChanges extends StatelessWidget {
  const _EmptyChanges({required this.copy});

  final Revision3ProjectDashboardCopy copy;

  @override
  Widget build(BuildContext context) => Material(
    key: const Key('revision3-project-dashboard-changes-empty'),
    color: Theme.of(context).colorScheme.surfaceContainerLow,
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(12),
      side: BorderSide(color: Theme.of(context).colorScheme.outlineVariant),
    ),
    child: Padding(
      padding: const EdgeInsets.all(18),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.add_circle_outline),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  copy.emptyChangesTitle,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 4),
                Text(copy.emptyChangesDescription),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class _MyModGroupList extends StatelessWidget {
  const _MyModGroupList({
    required this.group,
    required this.entries,
    required this.copy,
    required this.actions,
  });

  final _MyModGroup group;
  final List<Revision3MyModEntry> entries;
  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardChangeActions actions;

  @override
  Widget build(BuildContext context) => Column(
    key: Key('revision3-project-dashboard-change-group-${group.name}'),
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Padding(
        padding: const EdgeInsets.only(left: 4, bottom: 6),
        child: Semantics(
          header: true,
          child: Text(
            '${_groupLabel(group, copy)} (${entries.length})',
            style: Theme.of(context).textTheme.titleMedium,
          ),
        ),
      ),
      Material(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: Theme.of(context).colorScheme.outlineVariant),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (var index = 0; index < entries.length; index++) ...[
              if (index > 0) const Divider(height: 1),
              _MyModEntryTree(
                entry: entries[index],
                copy: copy,
                actions: actions,
              ),
            ],
          ],
        ),
      ),
    ],
  );
}

class _TechnicalChanges extends StatelessWidget {
  const _TechnicalChanges({
    required this.entries,
    required this.copy,
    required this.actions,
  });

  final List<Revision3MyModEntry> entries;
  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardChangeActions actions;

  @override
  Widget build(BuildContext context) => Material(
    key: const Key('revision3-project-dashboard-technical'),
    color: Theme.of(context).colorScheme.surfaceContainerLow,
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(12),
      side: BorderSide(color: Theme.of(context).colorScheme.outlineVariant),
    ),
    clipBehavior: Clip.antiAlias,
    child: ExpansionTile(
      key: const Key('revision3-project-dashboard-technical-expansion'),
      initiallyExpanded: false,
      leading: const Icon(Icons.code_outlined),
      title: Text('${copy.technicalContentLabel} (${entries.length})'),
      subtitle: Text(copy.technicalContentDescription),
      children: [
        for (var index = 0; index < entries.length; index++) ...[
          if (index > 0) const Divider(height: 1),
          _MyModEntryTree(entry: entries[index], copy: copy, actions: actions),
        ],
      ],
    ),
  );
}

class _MyModEntryTree extends StatelessWidget {
  const _MyModEntryTree({
    required this.entry,
    required this.copy,
    required this.actions,
    this.depth = 0,
  });

  final Revision3MyModEntry entry;
  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardChangeActions actions;
  final int depth;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      Padding(
        padding: EdgeInsets.only(left: depth * 18.0),
        child: _MyModEntryTile(entry: entry, copy: copy, actions: actions),
      ),
      for (final child in entry.children)
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(
              left: BorderSide(
                color: Theme.of(context).colorScheme.outlineVariant,
              ),
            ),
          ),
          child: _MyModEntryTree(
            entry: child,
            copy: copy,
            actions: actions,
            depth: depth + 1,
          ),
        ),
    ],
  );
}

class _MyModEntryTile extends StatefulWidget {
  const _MyModEntryTile({
    required this.entry,
    required this.copy,
    required this.actions,
  });

  final Revision3MyModEntry entry;
  final Revision3ProjectDashboardCopy copy;
  final Revision3ProjectDashboardChangeActions actions;

  @override
  State<_MyModEntryTile> createState() => _MyModEntryTileState();
}

class _MyModEntryTileState extends State<_MyModEntryTile> {
  bool _opening = false;

  FutureOr<void> Function()? get _action {
    final entry = widget.entry;
    switch (entry.kind) {
      case Revision3MyModContentKind.itemPatch:
        final open = widget.actions.openItemPatch;
        final vanillaClass = entry.entity?.summary.itemPatch?.vanillaClass;
        return open == null || vanillaClass == null
            ? null
            : () => open(vanillaClass);
      case Revision3MyModContentKind.dataAsset:
        final open = widget.actions.openDataAsset;
        final stage = entry.dataAssetStage;
        return open == null || stage == null ? null : () => open(stage);
      case Revision3MyModContentKind.quest ||
          Revision3MyModContentKind.npc ||
          Revision3MyModContentKind.dialogLine ||
          Revision3MyModContentKind.localization ||
          Revision3MyModContentKind.voiceSlot ||
          Revision3MyModContentKind.voiceTake ||
          Revision3MyModContentKind.generatedScript:
        final open = widget.actions.openEntity;
        final entity = entry.entity;
        return open == null || entity == null ? null : () => open(entity);
    }
  }

  Future<void> _open() async {
    final action = _action;
    if (_opening || action == null) return;
    setState(() => _opening = true);
    try {
      await Future<void>.sync(action);
    } catch (_) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(widget.copy.changeActionFailedMessage)),
        );
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final entry = widget.entry;
    final action = _action;
    final kindLabel = _kindLabel(entry.kind, widget.copy);
    final title = _friendlyEntryName(entry, kindLabel);
    final problemCount = _recursiveProblemCount(entry);
    final semanticsLabel = <String>[
      title,
      kindLabel,
      if (entry.kind == Revision3MyModContentKind.dataAsset &&
          entry.displayName != title)
        entry.displayName,
      if (entry.selected) widget.copy.selectedVoiceTakeLabel,
      if (problemCount > 0)
        '$problemCount ${widget.copy.unresolvedReferenceCountLabel}',
    ].join('. ');
    final details = <Widget>[
      Text(kindLabel),
      if (entry.kind == Revision3MyModContentKind.dataAsset &&
          entry.displayName != title)
        Text(entry.displayName),
      if (entry.selected) Text(widget.copy.selectedVoiceTakeLabel),
      if (problemCount > 0)
        Chip(
          key: Key(
            'revision3-project-dashboard-change-problems-${entry.stableId}',
          ),
          avatar: const Icon(Icons.warning_amber_rounded, size: 18),
          label: Text(
            '$problemCount ${widget.copy.unresolvedReferenceCountLabel}',
          ),
          visualDensity: VisualDensity.compact,
        ),
    ];
    return Semantics(
      container: true,
      button: action != null,
      enabled: action != null && !_opening,
      label: semanticsLabel,
      hint: action == null ? kindLabel : widget.copy.openChangeLabel,
      excludeSemantics: true,
      child: ListTile(
        key: Key('revision3-project-dashboard-change-${entry.stableId}'),
        enabled: action != null && !_opening,
        onTap: action == null || _opening ? null : _open,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
        leading: Icon(_kindIcon(entry.kind)),
        title: Text(title),
        subtitle: Padding(
          padding: const EdgeInsets.only(top: 4),
          child: Wrap(spacing: 8, runSpacing: 6, children: details),
        ),
        trailing: _opening
            ? const SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : action == null
            ? null
            : const Icon(Icons.chevron_right),
      ),
    );
  }
}

String _groupLabel(_MyModGroup group, Revision3ProjectDashboardCopy copy) =>
    switch (group) {
      _MyModGroup.quests => copy.questDraftCountLabel,
      _MyModGroup.npcs => copy.npcDraftCountLabel,
      _MyModGroup.items => copy.itemPatchLabel,
      _MyModGroup.dataAssets => copy.assetCountLabel,
      _MyModGroup.dialog => copy.dialogLineCountLabel,
      _MyModGroup.text => copy.localizationEntryLabel,
      _MyModGroup.voice => copy.voiceTakeCountLabel,
    };

String _kindLabel(
  Revision3MyModContentKind kind,
  Revision3ProjectDashboardCopy copy,
) => switch (kind) {
  Revision3MyModContentKind.quest => copy.questDraftCountLabel,
  Revision3MyModContentKind.npc => copy.npcDraftCountLabel,
  Revision3MyModContentKind.itemPatch => copy.itemPatchLabel,
  Revision3MyModContentKind.dataAsset => copy.assetCountLabel,
  Revision3MyModContentKind.dialogLine => copy.dialogLineCountLabel,
  Revision3MyModContentKind.localization => copy.localizationEntryLabel,
  Revision3MyModContentKind.voiceSlot => copy.voiceSlotLabel,
  Revision3MyModContentKind.voiceTake => copy.voiceTakeCountLabel,
  Revision3MyModContentKind.generatedScript => copy.generatedScriptLabel,
};

IconData _kindIcon(Revision3MyModContentKind kind) => switch (kind) {
  Revision3MyModContentKind.quest => Icons.assignment_outlined,
  Revision3MyModContentKind.npc => Icons.person_outline,
  Revision3MyModContentKind.itemPatch => Icons.tune_outlined,
  Revision3MyModContentKind.dataAsset => Icons.inventory_2_outlined,
  Revision3MyModContentKind.dialogLine => Icons.chat_bubble_outline,
  Revision3MyModContentKind.localization => Icons.translate_outlined,
  Revision3MyModContentKind.voiceSlot => Icons.mic_none_outlined,
  Revision3MyModContentKind.voiceTake => Icons.graphic_eq_outlined,
  Revision3MyModContentKind.generatedScript => Icons.code_outlined,
};

String _friendlyEntryName(Revision3MyModEntry entry, String fallback) {
  final displayName = entry.displayName.trim();
  if (entry.kind != Revision3MyModContentKind.dataAsset) {
    return displayName.isEmpty ? fallback : displayName;
  }
  final segments = displayName.split('/');
  final leaf = segments.isEmpty ? displayName : segments.last;
  return leaf.isEmpty ? fallback : leaf;
}

int _recursiveProblemCount(Revision3MyModEntry entry) =>
    entry.problemCount +
    entry.children.fold<int>(
      0,
      (sum, child) => sum + _recursiveProblemCount(child),
    );

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
