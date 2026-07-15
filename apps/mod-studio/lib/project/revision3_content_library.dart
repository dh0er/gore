import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_content_index.dart';

typedef Revision3ContentIndexLoader = Future<Revision3ContentIndex> Function();
typedef Revision3QuestOutlineEditor =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity quest,
    );
typedef Revision3QuestContextEditor =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity quest,
    );
typedef Revision3QuestTransitionsEditor =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity quest,
    );
typedef Revision3QuestSourceInspector =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity quest,
    );
typedef Revision3NpcSourceInspector =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity npc,
    );

/// Programmatic navigation into an attached [Revision3ContentLibrary].
///
/// Targets are deliberately accepted only as exact stable identifiers. The
/// library resolves every request against its currently loaded exact project
/// index; requests made while that index is reopening wait for the new index.
/// A missing target, a project switch, or a detached controller resolves to
/// `false` without retaining a stale content object.
class Revision3ContentLibraryController {
  Object? _attachment;
  Future<bool> Function(String entityId)? _openEntityById;
  Future<bool> Function(String sha256)? _openAssetBySha256;

  /// Opens the entity with exactly [entityId].
  Future<bool> openEntityById(String entityId) {
    final open = _openEntityById;
    return open == null ? Future<bool>.value(false) : open(entityId);
  }

  /// Opens the asset with exactly [sha256].
  Future<bool> openAssetBySha256(String sha256) {
    final open = _openAssetBySha256;
    return open == null ? Future<bool>.value(false) : open(sha256);
  }

  void _attach(
    Object attachment, {
    required Future<bool> Function(String entityId) openEntityById,
    required Future<bool> Function(String sha256) openAssetBySha256,
  }) {
    assert(
      _attachment == null || identical(_attachment, attachment),
      'A Revision3ContentLibraryController can only be attached to one '
      'content library at a time.',
    );
    _attachment = attachment;
    _openEntityById = openEntityById;
    _openAssetBySha256 = openAssetBySha256;
  }

  void _detach(Object attachment) {
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    _openEntityById = null;
    _openAssetBySha256 = null;
  }
}

enum _ContentMode { entities, assets }

enum _PendingContentTargetKind { entity, asset }

class _PendingContentNavigation {
  _PendingContentNavigation(this.kind, this.exactId);

  final _PendingContentTargetKind kind;
  final String exactId;
  final Completer<bool> result = Completer<bool>();
}

enum _EntityToolAction {
  questOutline,
  questContext,
  questTransitions,
  questSourceInspection,
  npcProfile,
}

const _stableSlotQuestGeneratorVersion = 4;

/// First real managed-R3 content surface.
///
/// The native index proves exact-current project content and reference shape.
/// Its write entry points are explicitly supplied bounded Quest editors; no
/// build, deployment, or runtime authority is implied.
class Revision3ContentLibrary extends StatefulWidget {
  const Revision3ContentLibrary({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
    required this.load,
    this.editQuestOutline,
    this.editQuestContext,
    this.editQuestTransitions,
    this.inspectQuestSource,
    this.inspectNpcSource,
    this.controller,
    super.key,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Revision3ContentIndexLoader load;
  final Revision3QuestOutlineEditor? editQuestOutline;
  final Revision3QuestContextEditor? editQuestContext;
  final Revision3QuestTransitionsEditor? editQuestTransitions;
  final Revision3QuestSourceInspector? inspectQuestSource;
  final Revision3NpcSourceInspector? inspectNpcSource;
  final Revision3ContentLibraryController? controller;

  @override
  State<Revision3ContentLibrary> createState() =>
      _Revision3ContentLibraryState();
}

class _Revision3ContentLibraryState extends State<Revision3ContentLibrary> {
  final _search = TextEditingController();
  Revision3ContentIndex? _index;
  Object? _error;
  bool _loading = false;
  int _loadGeneration = 0;
  _ContentMode _mode = _ContentMode.entities;
  Revision3ContentEntityKind? _kind;
  String? _selectedEntityId;
  String? _selectedAssetSha256;
  final List<_PendingContentNavigation> _pendingNavigations = [];

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    _attachController(widget.controller);
    _reload();
  }

  @override
  void didUpdateWidget(covariant Revision3ContentLibrary oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.controller, widget.controller)) {
      _cancelPendingNavigations();
      oldWidget.controller?._detach(this);
      _attachController(widget.controller);
    }
    if (oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHeadCanonicalJson != widget.projectHeadCanonicalJson) {
      final changedProject =
          oldWidget.projectRoot != widget.projectRoot ||
          oldWidget.projectId != widget.projectId;
      if (oldWidget.projectRevision != widget.projectRevision) {
        _cancelPendingNavigations();
      }
      if (changedProject) {
        _cancelPendingNavigations();
        _search.clear();
        _mode = _ContentMode.entities;
        _kind = null;
        _selectedEntityId = null;
        _selectedAssetSha256 = null;
      }
      _reload(clearCurrent: true);
    }
  }

  @override
  void dispose() {
    _cancelPendingNavigations();
    widget.controller?._detach(this);
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  void _attachController(Revision3ContentLibraryController? controller) {
    controller?._attach(
      this,
      openEntityById: _openEntityById,
      openAssetBySha256: _openAssetBySha256,
    );
  }

  Future<bool> _openEntityById(String entityId) =>
      _openExactTarget(_PendingContentTargetKind.entity, entityId);

  Future<bool> _openAssetBySha256(String sha256) =>
      _openExactTarget(_PendingContentTargetKind.asset, sha256);

  Future<bool> _openExactTarget(
    _PendingContentTargetKind kind,
    String exactId,
  ) {
    if (!mounted || exactId.isEmpty) return Future<bool>.value(false);
    if (_loading) {
      final pending = _PendingContentNavigation(kind, exactId);
      _pendingNavigations.add(pending);
      return pending.result.future;
    }
    if (_error != null) return Future<bool>.value(false);
    final index = _index;
    if (index == null) return Future<bool>.value(false);
    return Future<bool>.value(_resolveExactTarget(index, kind, exactId));
  }

  bool _resolveExactTarget(
    Revision3ContentIndex index,
    _PendingContentTargetKind kind,
    String exactId,
  ) {
    switch (kind) {
      case _PendingContentTargetKind.entity:
        if (index.entityById(exactId) == null) return false;
        _selectEntity(index, exactId);
        return true;
      case _PendingContentTargetKind.asset:
        if (index.assetBySha256(exactId) == null) return false;
        _selectAsset(index, exactId);
        return true;
    }
  }

  void _resolvePendingNavigations(Revision3ContentIndex index) {
    final pending = List<_PendingContentNavigation>.of(_pendingNavigations);
    _pendingNavigations.clear();
    for (final navigation in pending) {
      final resolved =
          mounted &&
          _resolveExactTarget(index, navigation.kind, navigation.exactId);
      navigation.result.complete(resolved);
    }
  }

  void _cancelPendingNavigations() {
    final pending = List<_PendingContentNavigation>.of(_pendingNavigations);
    _pendingNavigations.clear();
    for (final navigation in pending) {
      navigation.result.complete(false);
    }
  }

  Future<void> _reload({bool clearCurrent = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
      if (clearCurrent) _index = null;
    });
    try {
      final index = await widget.load();
      if (!mounted || generation != _loadGeneration) return;
      if (index.projectId != widget.projectId ||
          index.projectRevision != widget.projectRevision) {
        throw const FormatException(
          'Content index does not match the current project checkpoint.',
        );
      }
      setState(() {
        _index = index;
        _loading = false;
        _selectedEntityId = _retainEntitySelection(index);
        _selectedAssetSha256 = _retainAssetSelection(index);
      });
      _resolvePendingNavigations(index);
    } catch (error) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _error = error;
      });
      _cancelPendingNavigations();
    }
  }

  String? _retainEntitySelection(Revision3ContentIndex index) {
    final selected = _selectedEntityId;
    if (selected != null &&
        index.entities.any((entity) => entity.id == selected)) {
      return selected;
    }
    return index.entities.firstOrNull?.id;
  }

  String? _retainAssetSelection(Revision3ContentIndex index) {
    final selected = _selectedAssetSha256;
    if (selected != null &&
        index.assets.any((asset) => asset.sha256 == selected)) {
      return selected;
    }
    return index.assets.firstOrNull?.sha256;
  }

  void _selectEntity(Revision3ContentIndex index, String entityId) {
    if (index.entityById(entityId) == null) return;
    if (_search.text.isNotEmpty) _search.clear();
    setState(() {
      _mode = _ContentMode.entities;
      _kind = null;
      _selectedEntityId = entityId;
    });
  }

  void _selectAsset(Revision3ContentIndex index, String sha256) {
    if (index.assetBySha256(sha256) == null) return;
    if (_search.text.isNotEmpty) _search.clear();
    setState(() {
      _mode = _ContentMode.assets;
      _selectedAssetSha256 = sha256;
    });
  }

  @override
  Widget build(BuildContext context) {
    final index = _index;
    return Column(
      key: const Key('revision3-content-library'),
      children: [
        _LibraryHeader(
          index: index,
          loading: _loading,
          mode: _mode,
          onModeChanged: (mode) => setState(() => _mode = mode),
          onRefresh: _loading ? null : _reload,
        ),
        if (_error != null)
          Expanded(
            child: _ContentLoadError(error: _error!, retry: _reload),
          )
        else if (index == null)
          Expanded(
            child: Center(
              child: Semantics(
                liveRegion: true,
                label: 'Opening exact current project content',
                child: const CircularProgressIndicator(
                  key: Key('revision3-content-loading'),
                ),
              ),
            ),
          )
        else ...[
          _SearchAndFilters(
            controller: _search,
            mode: _mode,
            selectedKind: _kind,
            onKindChanged: (kind) => setState(() => _kind = kind),
          ),
          Expanded(
            child: switch (_mode) {
              _ContentMode.entities => _buildEntities(index),
              _ContentMode.assets => _buildAssets(index),
            },
          ),
        ],
      ],
    );
  }

  Widget _buildEntities(Revision3ContentIndex index) {
    final query = _search.text.trim().toLowerCase();
    final visible = index.entities
        .where((entity) => _kind == null || entity.kind == _kind)
        .where((entity) => entity.matches(query))
        .toList(growable: false);
    final selected =
        visible.where((entity) => entity.id == _selectedEntityId).firstOrNull ??
        visible.firstOrNull;
    return LayoutBuilder(
      builder: (context, constraints) {
        final list = _EntityList(
          entities: visible,
          selectedId: selected?.id,
          onSelected: (entity) async {
            setState(() => _selectedEntityId = entity.id);
            if (constraints.maxWidth < 900) {
              final editAction = await _showDetailsSheet<_EntityToolAction>(
                context,
                semanticsLabel: '${entity.kind.displayName} details',
                child: _EntityDetails(
                  index: index,
                  entity: entity,
                  onOpenEntity: (entityId) {
                    Navigator.of(context).pop();
                    _selectEntity(index, entityId);
                  },
                  onOpenAsset: (sha256) {
                    Navigator.of(context).pop();
                    _selectAsset(index, sha256);
                  },
                  onEditQuestOutline: widget.editQuestOutline == null
                      ? null
                      : () async => Navigator.of(
                          context,
                        ).pop(_EntityToolAction.questOutline),
                  onEditQuestContext: widget.editQuestContext == null
                      ? null
                      : () async => Navigator.of(
                          context,
                        ).pop(_EntityToolAction.questContext),
                  onEditQuestTransitions: widget.editQuestTransitions == null
                      ? null
                      : () async => Navigator.of(
                          context,
                        ).pop(_EntityToolAction.questTransitions),
                  onInspectQuestSource: widget.inspectQuestSource == null
                      ? null
                      : () async => Navigator.of(
                          context,
                        ).pop(_EntityToolAction.questSourceInspection),
                  onInspectNpcSource: widget.inspectNpcSource == null
                      ? null
                      : () async => Navigator.of(
                          context,
                        ).pop(_EntityToolAction.npcProfile),
                ),
              );
              if (!mounted || editAction == null) return;
              switch (editAction) {
                case _EntityToolAction.questOutline:
                  await widget.editQuestOutline?.call(index, entity);
                case _EntityToolAction.questContext:
                  await widget.editQuestContext?.call(index, entity);
                case _EntityToolAction.questTransitions:
                  await widget.editQuestTransitions?.call(index, entity);
                case _EntityToolAction.questSourceInspection:
                  await widget.inspectQuestSource?.call(index, entity);
                case _EntityToolAction.npcProfile:
                  await widget.inspectNpcSource?.call(index, entity);
              }
            }
          },
        );
        if (constraints.maxWidth < 900) {
          return list;
        }
        return Row(
          children: [
            Expanded(flex: 3, child: list),
            const VerticalDivider(width: 1),
            Expanded(
              flex: 2,
              child: selected == null
                  ? const _EmptyDetails(label: 'Select project content')
                  : _EntityDetails(
                      index: index,
                      entity: selected,
                      onOpenEntity: (entityId) =>
                          _selectEntity(index, entityId),
                      onOpenAsset: (sha256) => _selectAsset(index, sha256),
                      onEditQuestOutline: widget.editQuestOutline == null
                          ? null
                          : () => widget.editQuestOutline!(index, selected),
                      onEditQuestContext: widget.editQuestContext == null
                          ? null
                          : () => widget.editQuestContext!(index, selected),
                      onEditQuestTransitions:
                          widget.editQuestTransitions == null
                          ? null
                          : () => widget.editQuestTransitions!(index, selected),
                      onInspectQuestSource: widget.inspectQuestSource == null
                          ? null
                          : () => widget.inspectQuestSource!(index, selected),
                      onInspectNpcSource: widget.inspectNpcSource == null
                          ? null
                          : () => widget.inspectNpcSource!(index, selected),
                    ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildAssets(Revision3ContentIndex index) {
    final query = _search.text.trim().toLowerCase();
    final visible = index.assets
        .where(
          (asset) =>
              query.isEmpty ||
              asset.sha256.contains(query) ||
              asset.mediaType.toLowerCase().contains(query) ||
              asset.assetClass.displayName.toLowerCase().contains(query),
        )
        .toList(growable: false);
    final selected =
        visible
            .where((asset) => asset.sha256 == _selectedAssetSha256)
            .firstOrNull ??
        visible.firstOrNull;
    return LayoutBuilder(
      builder: (context, constraints) {
        final list = _AssetList(
          assets: visible,
          selectedSha256: selected?.sha256,
          onSelected: (asset) {
            setState(() => _selectedAssetSha256 = asset.sha256);
            if (constraints.maxWidth < 900) {
              unawaited(
                _showDetailsSheet<void>(
                  context,
                  semanticsLabel: '${asset.assetClass.displayName} details',
                  child: _AssetDetails(
                    index: index,
                    asset: asset,
                    onOpenEntity: (entityId) {
                      Navigator.of(context).pop();
                      _selectEntity(index, entityId);
                    },
                  ),
                ),
              );
            }
          },
        );
        if (constraints.maxWidth < 900) return list;
        return Row(
          children: [
            Expanded(flex: 3, child: list),
            const VerticalDivider(width: 1),
            Expanded(
              flex: 2,
              child: selected == null
                  ? const _EmptyDetails(label: 'Select a project asset')
                  : _AssetDetails(
                      index: index,
                      asset: selected,
                      onOpenEntity: (entityId) =>
                          _selectEntity(index, entityId),
                    ),
            ),
          ],
        );
      },
    );
  }

  Future<T?> _showDetailsSheet<T>(
    BuildContext context, {
    required String semanticsLabel,
    required Widget child,
  }) async {
    TransitionRoute<T>? sheetRoute;
    final result = await showModalBottomSheet<T>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (context) {
        sheetRoute ??= ModalRoute.of(context) as TransitionRoute<T>?;
        return SafeArea(
          child: Semantics(
            container: true,
            explicitChildNodes: true,
            label: semanticsLabel,
            child: SizedBox(
              height: MediaQuery.sizeOf(context).height * 0.78,
              child: child,
            ),
          ),
        );
      },
    );
    await sheetRoute?.completed;
    return result;
  }
}

class _LibraryHeader extends StatelessWidget {
  const _LibraryHeader({
    required this.index,
    required this.loading,
    required this.mode,
    required this.onModeChanged,
    required this.onRefresh,
  });

  final Revision3ContentIndex? index;
  final bool loading;
  final _ContentMode mode;
  final ValueChanged<_ContentMode> onModeChanged;
  final VoidCallback? onRefresh;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.surfaceContainerLowest,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 720;
          final identity = Row(
            children: [
              const Icon(Icons.hub_outlined),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Semantics(
                      header: true,
                      child: Text(
                        index?.projectName.isNotEmpty == true
                            ? index!.projectName
                            : 'Managed project content',
                        key: const Key('revision3-content-project-name'),
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                    ),
                    Semantics(
                      liveRegion: loading,
                      child: Text(
                        index == null
                            ? 'Opening the exact current project...'
                            : '${index!.entities.length} entities / ${index!.assets.length} assets / revision ${index!.projectRevision}',
                        key: const Key('revision3-content-project-summary'),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          );
          final controls = Wrap(
            spacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SegmentedButton<_ContentMode>(
                key: const Key('revision3-content-mode'),
                segments: const [
                  ButtonSegment(
                    value: _ContentMode.entities,
                    icon: Icon(Icons.account_tree_outlined),
                    label: Text(
                      'Content',
                      key: Key('revision3-content-mode-entities'),
                    ),
                  ),
                  ButtonSegment(
                    value: _ContentMode.assets,
                    icon: Icon(Icons.inventory_2_outlined),
                    label: Text(
                      'Assets',
                      key: Key('revision3-content-mode-assets'),
                    ),
                  ),
                ],
                selected: {mode},
                onSelectionChanged: (selection) =>
                    onModeChanged(selection.single),
              ),
              IconButton(
                key: const Key('revision3-content-refresh'),
                tooltip: 'Reopen exact current content',
                onPressed: onRefresh,
                icon: loading
                    ? const SizedBox.square(
                        key: Key('revision3-content-refresh-progress'),
                        dimension: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.refresh),
              ),
            ],
          );
          final problemCount = index?.problemCount ?? 0;
          return Padding(
            padding: const EdgeInsets.fromLTRB(20, 14, 12, 14),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (compact) ...[
                  identity,
                  const SizedBox(height: 10),
                  Align(alignment: Alignment.centerRight, child: controls),
                ] else
                  Row(
                    children: [
                      Expanded(child: identity),
                      const SizedBox(width: 12),
                      controls,
                    ],
                  ),
                const SizedBox(height: 10),
                Semantics(
                  container: true,
                  label: 'Read-only content status',
                  child: Container(
                    key: const Key('revision3-content-authority-banner'),
                    width: double.infinity,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    decoration: BoxDecoration(
                      color: scheme.secondaryContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Icon(
                              Icons.visibility_outlined,
                              size: 18,
                              color: scheme.onSecondaryContainer,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'Read-only exact project view. Build readiness has not been evaluated; runtime behavior is unqualified.',
                                style: TextStyle(
                                  color: scheme.onSecondaryContainer,
                                ),
                              ),
                            ),
                          ],
                        ),
                        if (problemCount > 0)
                          Padding(
                            padding: const EdgeInsets.only(left: 26, top: 6),
                            child: Text(
                              '$problemCount unresolved reference${problemCount == 1 ? '' : 's'}',
                              key: const Key('revision3-content-problem-count'),
                              style: TextStyle(
                                color: scheme.onSecondaryContainer,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _SearchAndFilters extends StatelessWidget {
  const _SearchAndFilters({
    required this.controller,
    required this.mode,
    required this.selectedKind,
    required this.onKindChanged,
  });

  final TextEditingController controller;
  final _ContentMode mode;
  final Revision3ContentEntityKind? selectedKind;
  final ValueChanged<Revision3ContentEntityKind?> onKindChanged;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 12, 16, 10),
    child: Column(
      children: [
        TextField(
          key: const Key('revision3-content-search'),
          controller: controller,
          decoration: InputDecoration(
            isDense: true,
            prefixIcon: const Icon(Icons.search),
            hintText: mode == _ContentMode.entities
                ? 'Search names, IDs, speakers, quests, NPCs, modules...'
                : 'Search asset type, media type, or SHA-256...',
            suffixIcon: controller.text.isEmpty
                ? null
                : IconButton(
                    tooltip: 'Clear search',
                    onPressed: controller.clear,
                    icon: const Icon(Icons.close),
                  ),
          ),
        ),
        if (mode == _ContentMode.entities) ...[
          const SizedBox(height: 10),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                ChoiceChip(
                  key: const Key('revision3-content-filter-all'),
                  label: const Text('All'),
                  selected: selectedKind == null,
                  onSelected: (_) => onKindChanged(null),
                ),
                for (final kind in Revision3ContentEntityKind.values) ...[
                  const SizedBox(width: 6),
                  ChoiceChip(
                    key: Key('revision3-content-filter-${kind.wireName}'),
                    label: Text(kind.displayName),
                    selected: selectedKind == kind,
                    onSelected: (_) => onKindChanged(kind),
                  ),
                ],
              ],
            ),
          ),
        ],
      ],
    ),
  );
}

class _EntityList extends StatelessWidget {
  const _EntityList({
    required this.entities,
    required this.selectedId,
    required this.onSelected,
  });

  final List<Revision3ContentEntity> entities;
  final String? selectedId;
  final ValueChanged<Revision3ContentEntity> onSelected;

  @override
  Widget build(BuildContext context) {
    if (entities.isEmpty) {
      return const _EmptyDetails(
        key: Key('revision3-content-entity-empty'),
        label: 'No matching project content',
      );
    }
    return ListView.builder(
      key: const Key('revision3-content-entity-list'),
      itemCount: entities.length,
      itemBuilder: (context, index) {
        final entity = entities[index];
        final title = entity.displayName.isEmpty
            ? entity.summary.primaryIdentity
            : entity.displayName;
        final selected = entity.id == selectedId;
        return Semantics(
          button: true,
          selected: selected,
          label:
              '$title, ${entity.kind.displayName}, ${entity.problemCount} reference problems',
          child: ListTile(
            key: Key('revision3-content-entity-${entity.id}'),
            selected: selected,
            leading: Icon(_kindIcon(entity.kind)),
            title: Text(title),
            subtitle: Text(
              '${entity.kind.displayName} / ${entity.summary.primaryIdentity}',
            ),
            trailing: entity.problemCount == 0
                ? const Icon(Icons.check_circle_outline, size: 18)
                : Badge(
                    label: Text('${entity.problemCount}'),
                    child: const Icon(Icons.warning_amber_rounded),
                  ),
            onTap: () => onSelected(entity),
          ),
        );
      },
    );
  }
}

class _EntityDetails extends StatelessWidget {
  const _EntityDetails({
    required this.index,
    required this.entity,
    required this.onOpenEntity,
    required this.onOpenAsset,
    required this.onEditQuestOutline,
    required this.onEditQuestContext,
    required this.onEditQuestTransitions,
    required this.onInspectQuestSource,
    required this.onInspectNpcSource,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity entity;
  final ValueChanged<String> onOpenEntity;
  final ValueChanged<String> onOpenAsset;
  final Future<void> Function()? onEditQuestOutline;
  final Future<void> Function()? onEditQuestContext;
  final Future<void> Function()? onEditQuestTransitions;
  final Future<void> Function()? onInspectQuestSource;
  final Future<void> Function()? onInspectNpcSource;

  @override
  Widget build(BuildContext context) {
    final backlinks = index.backlinksToEntity(entity.id);
    final outlineRequiresV2 = _questOutlineRequiresV2(index, entity);
    final editQuestOutline = outlineRequiresV2 ? null : onEditQuestOutline;
    return KeyedSubtree(
      key: ValueKey('revision3-content-entity-details-${entity.id}'),
      child: ListView(
        key: const Key('revision3-content-entity-details'),
        padding: const EdgeInsets.all(20),
        children: [
          Icon(_kindIcon(entity.kind), size: 36),
          const SizedBox(height: 12),
          Semantics(
            header: true,
            child: Text(
              _entityTitle(entity),
              style: Theme.of(context).textTheme.titleLarge,
            ),
          ),
          Text(entity.kind.displayName),
          if (entity.kind == Revision3ContentEntityKind.questDraft &&
              entity.summary.questDraft != null) ...[
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: PopupMenuButton<_EntityToolAction>(
                key: Key('revision3-content-edit-quest-${entity.id}'),
                enabled:
                    editQuestOutline != null ||
                    onEditQuestContext != null ||
                    onEditQuestTransitions != null ||
                    onInspectQuestSource != null,
                tooltip: 'Quest tools',
                onSelected: (action) async {
                  switch (action) {
                    case _EntityToolAction.questOutline:
                      await editQuestOutline?.call();
                    case _EntityToolAction.questContext:
                      await onEditQuestContext?.call();
                    case _EntityToolAction.questTransitions:
                      await onEditQuestTransitions?.call();
                    case _EntityToolAction.questSourceInspection:
                      await onInspectQuestSource?.call();
                    case _EntityToolAction.npcProfile:
                  }
                },
                itemBuilder: (context) => [
                  PopupMenuItem(
                    key: Key(
                      'revision3-content-edit-quest-outline-${entity.id}',
                    ),
                    value: _EntityToolAction.questOutline,
                    enabled: editQuestOutline != null,
                    child: ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.format_list_bulleted_outlined),
                      title: const Text('Name & objectives'),
                      subtitle: outlineRequiresV2
                          ? const Text(
                              'Not available yet after adding custom behavior',
                            )
                          : null,
                    ),
                  ),
                  PopupMenuItem(
                    key: Key(
                      'revision3-content-inspect-quest-source-${entity.id}',
                    ),
                    value: _EntityToolAction.questSourceInspection,
                    enabled: onInspectQuestSource != null,
                    child: ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.fact_check_outlined),
                      title: const Text('Source & checks'),
                      subtitle: onInspectQuestSource == null
                          ? const Text(
                              'Configure the game installation to verify source',
                            )
                          : const Text(
                              'Verify the generated script and its source inputs',
                            ),
                    ),
                  ),
                  PopupMenuItem(
                    key: Key(
                      'revision3-content-edit-quest-context-${entity.id}',
                    ),
                    value: _EntityToolAction.questContext,
                    enabled: onEditQuestContext != null,
                    child: ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.account_tree_outlined),
                      title: const Text('Description & connections'),
                      subtitle: onEditQuestContext == null
                          ? const Text('Configure the game installation first')
                          : null,
                    ),
                  ),
                  PopupMenuItem(
                    key: Key(
                      'revision3-content-edit-quest-transitions-${entity.id}',
                    ),
                    value: _EntityToolAction.questTransitions,
                    enabled: onEditQuestTransitions != null,
                    child: const ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: Icon(Icons.schema_outlined),
                      title: Text('States & transitions'),
                      subtitle: Text(
                        'Edit lifecycle triggers, conditions, and effects',
                      ),
                    ),
                  ),
                ],
                child: const Card(
                  margin: EdgeInsets.zero,
                  child: Padding(
                    padding: EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.edit_note_outlined),
                        SizedBox(width: 8),
                        Text('Quest tools'),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
          if (entity.kind == Revision3ContentEntityKind.npcDraft) ...[
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: PopupMenuButton<_EntityToolAction>(
                key: Key('revision3-content-npc-tools-${entity.id}'),
                enabled: onInspectNpcSource != null,
                tooltip: 'NPC tools',
                onSelected: (action) async {
                  if (action == _EntityToolAction.npcProfile) {
                    await onInspectNpcSource?.call();
                  }
                },
                itemBuilder: (context) => [
                  PopupMenuItem(
                    key: Key(
                      'revision3-content-inspect-npc-source-${entity.id}',
                    ),
                    value: _EntityToolAction.npcProfile,
                    enabled: onInspectNpcSource != null,
                    child: const ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: Icon(Icons.fact_check_outlined),
                      title: Text('Profile & checks'),
                      subtitle: Text(
                        'Verify saved source and show remaining blockers',
                      ),
                    ),
                  ),
                ],
                child: const Card(
                  margin: EdgeInsets.zero,
                  child: Padding(
                    padding: EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.person_search_outlined),
                        SizedBox(width: 8),
                        Text('NPC tools'),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
          const Divider(height: 28),
          _Detail(
            label: 'Semantic identity',
            value: entity.summary.primaryIdentity,
          ),
          _Detail(label: 'Details', value: entity.summary.secondaryText),
          _Detail(
            label: 'Origin',
            value: '${entity.origin.type}: ${entity.origin.label}',
          ),
          _Detail(label: 'Entity revision', value: '${entity.revision}'),
          _Detail(label: 'Stable ID', value: entity.id, selectable: true),
          const Divider(height: 28),
          Text('References', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          if (entity.references.isEmpty && entity.assetReferences.isEmpty)
            const Text('No projected references')
          else ...[
            for (final reference in entity.references)
              _ReferenceTile(
                icon:
                    reference.resolution ==
                        Revision3ContentReferenceResolution.resolved
                    ? Icons.link
                    : Icons.link_off,
                title: reference.role.replaceAll('_', ' '),
                subtitle:
                    '${reference.target.expectedKind.displayName} / ${reference.target.entityId}',
                ok:
                    reference.resolution ==
                    Revision3ContentReferenceResolution.resolved,
                onTap:
                    reference.resolution ==
                            Revision3ContentReferenceResolution.resolved &&
                        reference.target.projectId == index.projectId &&
                        index.entityById(reference.target.entityId) != null
                    ? () => onOpenEntity(reference.target.entityId)
                    : null,
              ),
            for (final reference in entity.assetReferences)
              _ReferenceTile(
                icon: Icons.inventory_2_outlined,
                title: reference.role.replaceAll('_', ' '),
                subtitle: reference.logicalName ?? reference.sha256,
                ok:
                    reference.resolution ==
                    Revision3ContentAssetReferenceResolution.resolved,
                onTap:
                    reference.resolution ==
                            Revision3ContentAssetReferenceResolution.resolved &&
                        index.assetBySha256(reference.sha256) != null
                    ? () => onOpenAsset(reference.sha256)
                    : null,
              ),
          ],
          const Divider(height: 28),
          Row(
            children: [
              Expanded(
                child: Text(
                  'Used by',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              Text('${backlinks.length}'),
            ],
          ),
          const SizedBox(height: 8),
          if (backlinks.isEmpty)
            const Text('No incoming project references')
          else
            for (
              var backlinkIndex = 0;
              backlinkIndex < backlinks.length;
              backlinkIndex++
            )
              _ReferenceTile(
                key: Key(
                  'revision3-content-backlink-${entity.id}-${backlinks[backlinkIndex].source.id}-${backlinks[backlinkIndex].reference.role}-$backlinkIndex',
                ),
                icon: _kindIcon(backlinks[backlinkIndex].source.kind),
                title: _entityTitle(backlinks[backlinkIndex].source),
                subtitle:
                    '${backlinks[backlinkIndex].reference.role.replaceAll('_', ' ')} / ${backlinks[backlinkIndex].source.kind.displayName}',
                ok:
                    backlinks[backlinkIndex].reference.resolution ==
                    Revision3ContentReferenceResolution.resolved,
                onTap: () => onOpenEntity(backlinks[backlinkIndex].source.id),
              ),
        ],
      ),
    );
  }
}

class _AssetList extends StatelessWidget {
  const _AssetList({
    required this.assets,
    required this.selectedSha256,
    required this.onSelected,
  });

  final List<Revision3ContentAsset> assets;
  final String? selectedSha256;
  final ValueChanged<Revision3ContentAsset> onSelected;

  @override
  Widget build(BuildContext context) {
    if (assets.isEmpty) {
      return const _EmptyDetails(
        key: Key('revision3-content-asset-empty'),
        label: 'No matching project assets',
      );
    }
    return ListView.builder(
      key: const Key('revision3-content-asset-list'),
      itemCount: assets.length,
      itemBuilder: (context, index) {
        final asset = assets[index];
        final selected = asset.sha256 == selectedSha256;
        return Semantics(
          button: true,
          selected: selected,
          label:
              '${asset.assetClass.displayName}, ${asset.mediaType}, ${_formatBytes(asset.byteLength)}',
          child: ListTile(
            key: Key('revision3-content-asset-${asset.sha256}'),
            selected: selected,
            leading: const Icon(Icons.inventory_2_outlined),
            title: Text(asset.assetClass.displayName),
            subtitle: Text(
              '${asset.mediaType} / ${_formatBytes(asset.byteLength)}',
            ),
            onTap: () => onSelected(asset),
          ),
        );
      },
    );
  }
}

class _AssetDetails extends StatelessWidget {
  const _AssetDetails({
    required this.index,
    required this.asset,
    required this.onOpenEntity,
  });

  final Revision3ContentIndex index;
  final Revision3ContentAsset asset;
  final ValueChanged<String> onOpenEntity;

  @override
  Widget build(BuildContext context) {
    final backlinks = index.backlinksToAsset(asset.sha256);
    return KeyedSubtree(
      key: ValueKey('revision3-content-asset-details-${asset.sha256}'),
      child: ListView(
        key: const Key('revision3-content-asset-details'),
        padding: const EdgeInsets.all(20),
        children: [
          const Icon(Icons.inventory_2_outlined, size: 36),
          const SizedBox(height: 12),
          Semantics(
            header: true,
            child: Text(
              asset.assetClass.displayName,
              style: Theme.of(context).textTheme.titleLarge,
            ),
          ),
          const Divider(height: 28),
          _Detail(label: 'Media type', value: asset.mediaType),
          _Detail(label: 'Size', value: _formatBytes(asset.byteLength)),
          _Detail(label: 'SHA-256', value: asset.sha256, selectable: true),
          const Divider(height: 28),
          Row(
            children: [
              Expanded(
                child: Text(
                  'Used by',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              Text('${backlinks.length}'),
            ],
          ),
          const SizedBox(height: 8),
          if (backlinks.isEmpty)
            const Text('No incoming project references')
          else
            for (
              var backlinkIndex = 0;
              backlinkIndex < backlinks.length;
              backlinkIndex++
            )
              _ReferenceTile(
                key: Key(
                  'revision3-content-asset-backlink-${asset.sha256}-${backlinks[backlinkIndex].source.id}-${backlinks[backlinkIndex].reference.role}-$backlinkIndex',
                ),
                icon: _kindIcon(backlinks[backlinkIndex].source.kind),
                title: _entityTitle(backlinks[backlinkIndex].source),
                subtitle:
                    '${backlinks[backlinkIndex].reference.role.replaceAll('_', ' ')} / ${backlinks[backlinkIndex].source.kind.displayName}',
                ok:
                    backlinks[backlinkIndex].reference.resolution ==
                    Revision3ContentAssetReferenceResolution.resolved,
                onTap: () => onOpenEntity(backlinks[backlinkIndex].source.id),
              ),
        ],
      ),
    );
  }
}

class _ReferenceTile extends StatelessWidget {
  const _ReferenceTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.ok,
    this.onTap,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool ok;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => ListTile(
    dense: true,
    contentPadding: EdgeInsets.zero,
    leading: Icon(icon, color: ok ? null : Theme.of(context).colorScheme.error),
    title: Text(title),
    subtitle: Text(subtitle, maxLines: 2, overflow: TextOverflow.ellipsis),
    trailing: Icon(
      onTap != null
          ? Icons.arrow_forward
          : (ok ? Icons.check : Icons.error_outline),
      size: 18,
    ),
    onTap: onTap,
  );
}

class _Detail extends StatelessWidget {
  const _Detail({
    required this.label,
    required this.value,
    this.selectable = false,
  });

  final String label;
  final String value;
  final bool selectable;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 14),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 3),
        if (selectable) SelectableText(value) else Text(value),
      ],
    ),
  );
}

class _EmptyDetails extends StatelessWidget {
  const _EmptyDetails({required this.label, super.key});
  final String label;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Text(label, textAlign: TextAlign.center),
    ),
  );
}

class _ContentLoadError extends StatelessWidget {
  const _ContentLoadError({required this.error, required this.retry});
  final Object error;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) {
    final message = error is FormatException
        ? (error as FormatException).message.toString()
        : '$error';
    return Center(
      child: Semantics(
        container: true,
        liveRegion: true,
        label: 'Content reopen failed',
        child: ConstrainedBox(
          key: const Key('revision3-content-error'),
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
                Semantics(
                  header: true,
                  child: Text(
                    'Current project content could not be reopened.',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                const SizedBox(height: 8),
                Text(message, textAlign: TextAlign.center),
                const SizedBox(height: 16),
                FilledButton.icon(
                  key: const Key('revision3-content-retry'),
                  onPressed: retry,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Retry exact reopen'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

bool _questOutlineRequiresV2(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  for (final reference in quest.references) {
    if (reference.role != 'draft_script_module' ||
        reference.qualifier != null ||
        reference.resolution != Revision3ContentReferenceResolution.resolved ||
        reference.target.projectId != index.projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.scriptModule) {
      continue;
    }
    final module = index.entityById(reference.target.entityId);
    final generatorVersion = module?.origin.generatorVersion;
    if (module?.kind == Revision3ContentEntityKind.scriptModule &&
        generatorVersion != null &&
        generatorVersion >= _stableSlotQuestGeneratorVersion) {
      return true;
    }
  }
  return false;
}

IconData _kindIcon(Revision3ContentEntityKind kind) => switch (kind) {
  Revision3ContentEntityKind.localizationEntry => Icons.translate,
  Revision3ContentEntityKind.dialogLine => Icons.chat_bubble_outline,
  Revision3ContentEntityKind.voiceSlot => Icons.record_voice_over_outlined,
  Revision3ContentEntityKind.voiceTake => Icons.graphic_eq,
  Revision3ContentEntityKind.npcDraft => Icons.person_outline,
  Revision3ContentEntityKind.questDraft => Icons.assignment_outlined,
  Revision3ContentEntityKind.scriptModule => Icons.code,
};

String _entityTitle(Revision3ContentEntity entity) => entity.displayName.isEmpty
    ? entity.summary.primaryIdentity
    : entity.displayName;

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KiB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MiB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GiB';
}

extension _FirstOrNull<E> on Iterable<E> {
  E? get firstOrNull {
    final iterator = this.iterator;
    return iterator.moveNext() ? iterator.current : null;
  }
}
