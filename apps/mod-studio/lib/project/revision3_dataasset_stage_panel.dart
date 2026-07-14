import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_dataasset_authoring.dart';

/// Visible management surface for receipt-verified DataAsset edits already
/// supported by the managed revision-3 session.
///
/// This panel imports and removes project registry entries only. It is not a
/// semantic value editor and exposes no build, pack, deploy, or gameplay action.
class Revision3DataAssetStagePanel extends StatefulWidget {
  const Revision3DataAssetStagePanel({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHead,
    required this.load,
    required this.publish,
    required this.remove,
    this.pickPatchReceipt,
    super.key,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead projectHead;
  final Revision3DataAssetStageLoader load;
  final Revision3DataAssetStagePublisher publish;
  final Revision3DataAssetStageRemover remove;
  final Revision3DataAssetPatchReceiptPicker? pickPatchReceipt;

  @override
  State<Revision3DataAssetStagePanel> createState() =>
      _Revision3DataAssetStagePanelState();
}

class _Revision3DataAssetStagePanelState
    extends State<Revision3DataAssetStagePanel> {
  final _search = TextEditingController();
  List<AuthoringRevision3DataAssetStage>? _stages;
  Object? _loadError;
  String? _actionError;
  bool _loading = false;
  bool _picking = false;
  bool _mutating = false;
  bool _confirmationOpen = false;
  bool _locked = false;
  int _loadEpoch = 0;
  int _actionEpoch = 0;

  bool get _busy => _picking || _mutating;
  bool get _registryReady => _stages != null && !_loading && _loadError == null;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    _reload();
  }

  @override
  void didUpdateWidget(covariant Revision3DataAssetStagePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHead.canonicalJson !=
            widget.projectHead.canonicalJson) {
      if (oldWidget.projectRoot != widget.projectRoot ||
          oldWidget.projectId != widget.projectId ||
          oldWidget.projectHead.canonicalJson !=
              widget.projectHead.canonicalJson) {
        _actionEpoch++;
      }
      _search.clear();
      _actionError = null;
      _locked = false;
      _picking = false;
      _mutating = false;
      _confirmationOpen = false;
      _reload(clearCurrent: true);
    }
  }

  @override
  void dispose() {
    _loadEpoch++;
    _actionEpoch++;
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  Future<void> _reload({bool clearCurrent = false}) async {
    final epoch = ++_loadEpoch;
    setState(() {
      _loading = true;
      _loadError = null;
      if (clearCurrent) _stages = null;
    });
    try {
      final stages = await widget.load();
      if (!mounted || epoch != _loadEpoch) return;
      if (stages.any(
        (stage) =>
            stage.projectId != widget.projectId ||
            stage.stagedProjectRevision > widget.projectRevision,
      )) {
        throw const FormatException(
          'DataAsset edits do not match the current project checkpoint.',
        );
      }
      setState(() {
        _stages = List<AuthoringRevision3DataAssetStage>.unmodifiable(stages);
        _loading = false;
      });
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() {
        _loading = false;
        _loadError = error;
        if (_stages != null) {
          _actionError = revision3DataAssetFriendlyError(error);
        }
        if (error is Revision3DataAssetRequiresReopenException) _locked = true;
      });
    }
  }

  Future<void> _addVerifiedEdit() async {
    if (_busy || _locked || !_registryReady) return;
    final projectRoot = widget.projectRoot;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectHeadJson = widget.projectHead.canonicalJson;
    final epoch = ++_actionEpoch;
    setState(() {
      _picking = true;
      _actionError = null;
    });
    String? receiptPath;
    try {
      receiptPath = await (widget.pickPatchReceipt ?? _pickPatchReceipt)();
    } catch (error) {
      if (_actionIsCurrent(
        epoch,
        projectRoot,
        projectId,
        projectRevision,
        projectHeadJson,
      )) {
        setState(() {
          _picking = false;
          _actionError = revision3DataAssetFriendlyError(error);
        });
      }
      return;
    }
    if (!_actionIsCurrent(
      epoch,
      projectRoot,
      projectId,
      projectRevision,
      projectHeadJson,
    )) {
      return;
    }
    if (receiptPath == null) {
      setState(() => _picking = false);
      return;
    }
    setState(() {
      _picking = false;
      _mutating = true;
    });
    try {
      final publication = await widget.publish(patchReceiptPath: receiptPath);
      if (!mounted || epoch != _actionEpoch) return;
      if (publication.projectId != projectId ||
          publication.projectRevision != projectRevision + 1) {
        throw const FormatException(
          'Published DataAsset edit does not advance the current checkpoint.',
        );
      }
      setState(() => _mutating = false);
      _showSuccess(
        'Verified DataAsset edit saved in project revision ${publication.projectRevision}. It is not available to build or test in-game yet.',
      );
    } catch (error) {
      if (!mounted || epoch != _actionEpoch) return;
      if (!_sameCheckpoint(
        projectRoot,
        projectId,
        projectRevision,
        projectHeadJson,
      )) {
        return;
      }
      setState(() {
        _mutating = false;
        _actionError = revision3DataAssetFriendlyError(error);
        if (error is Revision3DataAssetRequiresReopenException) _locked = true;
      });
    }
  }

  Future<void> _removeStage(AuthoringRevision3DataAssetStage stage) async {
    if (_busy || _locked || !_registryReady || _confirmationOpen) return;
    final confirmationProjectRoot = widget.projectRoot;
    final confirmationProjectId = widget.projectId;
    final confirmationProjectRevision = widget.projectRevision;
    final confirmationProjectHeadJson = widget.projectHead.canonicalJson;
    _confirmationOpen = true;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('revision3-dataasset-remove-dialog'),
        title: const Text('Remove this DataAsset edit?'),
        content: Text(
          '${_assetName(stage.targetPath)} will be removed from the project registry. '
          'Its source files and the game installation will not be changed.',
        ),
        actions: [
          TextButton(
            key: const Key('revision3-dataasset-remove-cancel'),
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('revision3-dataasset-remove-confirm'),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: const Text('Remove from project'),
          ),
        ],
      ),
    );
    _confirmationOpen = false;
    if (!mounted ||
        confirmed != true ||
        _busy ||
        _locked ||
        !_sameCheckpoint(
          confirmationProjectRoot,
          confirmationProjectId,
          confirmationProjectRevision,
          confirmationProjectHeadJson,
        )) {
      return;
    }

    final projectRoot = widget.projectRoot;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectHeadJson = widget.projectHead.canonicalJson;
    final epoch = ++_actionEpoch;
    setState(() {
      _mutating = true;
      _actionError = null;
    });
    try {
      final publication = await widget.remove(targetPath: stage.targetPath);
      if (!mounted || epoch != _actionEpoch) return;
      if (publication.projectId != projectId ||
          publication.projectRevision != projectRevision + 1 ||
          publication.removed.targetPath.toLowerCase() !=
              stage.targetPath.toLowerCase()) {
        throw const FormatException(
          'Removed DataAsset edit does not advance the current checkpoint.',
        );
      }
      setState(() => _mutating = false);
      _showSuccess(
        'DataAsset edit removed from project revision ${publication.projectRevision}. No game files were changed.',
      );
    } catch (error) {
      if (!mounted || epoch != _actionEpoch) return;
      if (!_sameCheckpoint(
        projectRoot,
        projectId,
        projectRevision,
        projectHeadJson,
      )) {
        return;
      }
      setState(() {
        _mutating = false;
        _actionError = revision3DataAssetFriendlyError(error);
        if (error is Revision3DataAssetRequiresReopenException) _locked = true;
      });
    }
  }

  bool _actionIsCurrent(
    int epoch,
    String projectRoot,
    String projectId,
    int projectRevision,
    String projectHeadJson,
  ) =>
      mounted &&
      epoch == _actionEpoch &&
      _sameCheckpoint(projectRoot, projectId, projectRevision, projectHeadJson);

  bool _sameCheckpoint(
    String projectRoot,
    String projectId,
    int projectRevision,
    String projectHeadJson,
  ) =>
      widget.projectRoot == projectRoot &&
      widget.projectId == projectId &&
      widget.projectRevision == projectRevision &&
      widget.projectHead.canonicalJson == projectHeadJson;

  void _showSuccess(String message) {
    ScaffoldMessenger.maybeOf(
      context,
    )?.showSnackBar(SnackBar(content: Text(message)));
  }

  List<AuthoringRevision3DataAssetStage> _visibleStages(
    List<AuthoringRevision3DataAssetStage> stages,
  ) {
    final query = _search.text.trim().toLowerCase();
    if (query.isEmpty) return stages;
    return stages
        .where(
          (stage) =>
              stage.targetPath.toLowerCase().contains(query) ||
              _dataKindLabel(stage.selectorKind).toLowerCase().contains(query),
        )
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    final stages = _stages;
    final visible = stages == null
        ? const <AuthoringRevision3DataAssetStage>[]
        : _visibleStages(stages);
    return Column(
      key: const Key('revision3-dataasset-stage-panel'),
      children: [
        _DataAssetBoundaryNotice(locked: _locked),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  stages == null
                      ? 'Verified DataAsset edits'
                      : 'Verified DataAsset edits (${stages.length})',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
              ),
              IconButton(
                key: const Key('revision3-dataasset-stage-refresh'),
                tooltip: 'Refresh exact project list',
                onPressed: _busy || _loading || _locked ? null : _reload,
                icon: const Icon(Icons.refresh),
              ),
              const SizedBox(width: 8),
              FilledButton.icon(
                key: const Key('revision3-dataasset-stage-add'),
                onPressed: _busy || _locked || !_registryReady
                    ? null
                    : _addVerifiedEdit,
                icon: _picking || _mutating
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.add),
                label: Text(_picking ? 'Choosing...' : 'Add verified edit...'),
              ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
          child: TextField(
            key: const Key('revision3-dataasset-stage-search'),
            controller: _search,
            enabled: stages != null,
            decoration: InputDecoration(
              isDense: true,
              prefixIcon: const Icon(Icons.search),
              hintText: 'Search DataAsset name or /Game path...',
              suffixIcon: _search.text.isEmpty
                  ? null
                  : IconButton(
                      tooltip: 'Clear search',
                      onPressed: _search.clear,
                      icon: const Icon(Icons.close),
                    ),
            ),
          ),
        ),
        if (_loading) const LinearProgressIndicator(minHeight: 2),
        if (_actionError != null)
          _DataAssetActionError(
            message: _actionError!,
            onDismiss: () => setState(() => _actionError = null),
          ),
        Expanded(
          child: switch ((stages, _loadError)) {
            (null, final Object error) => _DataAssetLoadError(
              error: error,
              retry: _locked || _loading ? null : _reload,
            ),
            (null, null) => Center(
              child: Semantics(
                liveRegion: true,
                label: 'Loading exact DataAsset edit list',
                child: const CircularProgressIndicator(
                  key: Key('revision3-dataasset-stage-loading'),
                ),
              ),
            ),
            (final List<AuthoringRevision3DataAssetStage> loaded, _)
                when loaded.isEmpty =>
              const _DataAssetEmptyState(),
            (_, _) when visible.isEmpty => const Center(
              child: Text(
                'No matching DataAsset edits.',
                key: Key('revision3-dataasset-stage-no-matches'),
              ),
            ),
            _ => ListView.builder(
              key: const Key('revision3-dataasset-stage-list'),
              padding: const EdgeInsets.fromLTRB(12, 4, 12, 16),
              itemCount: visible.length,
              itemBuilder: (context, index) => _DataAssetStageTile(
                stage: visible[index],
                remove: _busy || _locked || !_registryReady
                    ? null
                    : () => _removeStage(visible[index]),
              ),
            ),
          },
        ),
      ],
    );
  }
}

class _DataAssetBoundaryNotice extends StatelessWidget {
  const _DataAssetBoundaryNotice({required this.locked});

  final bool locked;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-dataasset-boundary-notice'),
      width: double.infinity,
      margin: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: locked ? scheme.errorContainer : scheme.secondaryContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            locked ? Icons.lock_reset_outlined : Icons.science_outlined,
            color: locked
                ? scheme.onErrorContainer
                : scheme.onSecondaryContainer,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              locked
                  ? 'Exact verification is unavailable. Reopen the managed project before continuing.'
                  : 'This list contains receipt-verified fixed-size edits saved in the project. They are not yet included in builds, deployable, or qualified for gameplay. Adding or removing an entry does not write to the game installation.',
              style: TextStyle(
                color: locked
                    ? scheme.onErrorContainer
                    : scheme.onSecondaryContainer,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _DataAssetStageTile extends StatelessWidget {
  const _DataAssetStageTile({required this.stage, required this.remove});

  final AuthoringRevision3DataAssetStage stage;
  final VoidCallback? remove;

  @override
  Widget build(BuildContext context) => Card(
    child: ExpansionTile(
      key: ValueKey('revision3-dataasset-stage-${stage.targetPath}'),
      leading: const Icon(Icons.data_object_outlined),
      title: Text(_assetName(stage.targetPath)),
      subtitle: Text(
        '${stage.targetPath}\n${_dataKindLabel(stage.selectorKind)} - saved in project revision ${stage.stagedProjectRevision}',
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
      ),
      childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
      children: [
        const Divider(),
        const Align(
          alignment: Alignment.centerLeft,
          child: Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              Chip(label: Text('Saved project edit')),
              Chip(label: Text('Build unavailable')),
              Chip(label: Text('Gameplay unverified')),
            ],
          ),
        ),
        const SizedBox(height: 12),
        _DataAssetFact(
          label: 'Verified value shape',
          value: _dataKindLabel(stage.selectorKind),
        ),
        _DataAssetFact(
          label: 'Replacement width',
          value:
              '${stage.replacementByteLength} byte${stage.replacementByteLength == 1 ? '' : 's'}',
        ),
        _DataAssetFact(
          label: 'Selector depth',
          value: '${stage.selectorPathDepth}',
        ),
        _DataAssetFact(
          label: 'Verified package inputs',
          value:
              '${stage.generationContainerCount} containers, ${stage.generationChunkCount} chunks, ${stage.sidecars.length} sidecars',
        ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerRight,
          child: OutlinedButton.icon(
            key: ValueKey(
              'revision3-dataasset-stage-remove-${stage.targetPath}',
            ),
            onPressed: remove,
            icon: const Icon(Icons.remove_circle_outline),
            label: const Text('Remove from project...'),
          ),
        ),
      ],
    ),
  );
}

class _DataAssetFact extends StatelessWidget {
  const _DataAssetFact({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 8),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 180,
          child: Text(label, style: Theme.of(context).textTheme.labelLarge),
        ),
        Expanded(child: Text(value)),
      ],
    ),
  );
}

class _DataAssetEmptyState extends StatelessWidget {
  const _DataAssetEmptyState();

  @override
  Widget build(BuildContext context) => const Center(
    child: Padding(
      padding: EdgeInsets.all(24),
      child: Column(
        key: Key('revision3-dataasset-stage-empty'),
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.data_object_outlined, size: 42),
          SizedBox(height: 12),
          Text('No verified DataAsset edits in this project.'),
          SizedBox(height: 6),
          Text(
            'Use Add verified edit to choose a receipt created by the guarded gore asset patch-fixed workflow.',
            textAlign: TextAlign.center,
          ),
        ],
      ),
    ),
  );
}

class _DataAssetLoadError extends StatelessWidget {
  const _DataAssetLoadError({required this.error, required this.retry});

  final Object error;
  final VoidCallback? retry;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      key: const Key('revision3-dataasset-stage-error'),
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
            revision3DataAssetFriendlyError(error),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 16),
          FilledButton.icon(
            key: const Key('revision3-dataasset-stage-retry'),
            onPressed: retry,
            icon: const Icon(Icons.refresh),
            label: const Text('Retry exact read'),
          ),
        ],
      ),
    ),
  );
}

class _DataAssetActionError extends StatelessWidget {
  const _DataAssetActionError({required this.message, required this.onDismiss});

  final String message;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: Container(
      key: const Key('revision3-dataasset-stage-action-error'),
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
      color: Theme.of(context).colorScheme.errorContainer,
      child: Row(
        children: [
          const Icon(Icons.error_outline),
          const SizedBox(width: 8),
          Expanded(child: Text(message)),
          IconButton(
            tooltip: 'Dismiss error',
            onPressed: onDismiss,
            icon: const Icon(Icons.close),
          ),
        ],
      ),
    ),
  );
}

Future<String?> _pickPatchReceipt() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(
        label: 'Verified gore DataAsset edit receipt',
        extensions: ['json'],
      ),
    ],
  );
  return file?.path;
}

String _assetName(String targetPath) {
  final segments = targetPath.split('/');
  return segments.isEmpty || segments.last.isEmpty ? targetPath : segments.last;
}

String _dataKindLabel(String kind) => switch (kind) {
  'bool' => 'Boolean value',
  'linear_color_f32x4' => 'Color value',
  'vector4_f64x4' => 'Vector value',
  'byte' ||
  'int8' ||
  'uint16' ||
  'int16' ||
  'int32' ||
  'uint32' ||
  'float32' ||
  'float64' ||
  'uint64' ||
  'int64' => 'Numeric value',
  _ => 'Verified fixed-size value',
};
