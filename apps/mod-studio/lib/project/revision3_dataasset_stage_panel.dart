import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import '../dataasset/ui/dataasset_lab.dart';
import '../dataasset/ui/dataasset_semantic_edit_panel.dart';
import '../dataasset/ui/dataasset_semantic_edit_wizard.dart';
import 'revision3_dataasset_authoring.dart';
import 'revision3_dataasset_build_dialog.dart';

typedef Revision3ReviewedDataAssetStageBuilder =
    Future<AuthoringRevision3ReviewedDataAssetBuildResult> Function({
      required String targetPath,
      required String packName,
      required String output,
    });

/// Visible management surface for receipt-verified DataAsset edits already
/// supported by the managed revision-3 session.
///
/// This panel guides typed value edits, imports/removes verified expert proofs,
/// and can create write-new mod files for an existing reviewed stage. It never
/// deploys files or changes the game installation.
class Revision3DataAssetStagePanel extends StatefulWidget {
  const Revision3DataAssetStagePanel({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHead,
    required this.load,
    required this.publish,
    required this.remove,
    this.requiresReopen = false,
    this.pickPatchReceipt,
    this.publishSemanticEdit,
    this.semanticInspector,
    this.semanticUassetPicker,
    this.semanticUsmapPicker,
    this.semanticExtractReceiptPicker,
    this.semanticExtractReceiptInspector,
    this.browseInstalledPackages,
    this.buildReviewedStage,
    this.pickBuildParentDirectory,
    this.buildUnavailableReason,
    super.key,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead projectHead;
  final bool requiresReopen;
  final Revision3DataAssetStageLoader load;
  final Revision3DataAssetStagePublisher publish;
  final Revision3DataAssetStageRemover remove;
  final Revision3DataAssetPatchReceiptPicker? pickPatchReceipt;
  final DataAssetSemanticStagePublisher? publishSemanticEdit;
  final DataAssetInspector? semanticInspector;
  final DataAssetFilePicker? semanticUassetPicker;
  final DataAssetFilePicker? semanticUsmapPicker;
  final DataAssetExtractReceiptPicker? semanticExtractReceiptPicker;
  final DataAssetExtractReceiptInspector? semanticExtractReceiptInspector;
  final Future<void> Function()? browseInstalledPackages;
  final Revision3ReviewedDataAssetStageBuilder? buildReviewedStage;
  final Revision3DataAssetBuildParentDirectoryPicker? pickBuildParentDirectory;
  final String? buildUnavailableReason;

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
  bool _semanticEditorOpen = false;
  bool _buildDialogOpen = false;
  bool _semanticCheckpointStale = false;
  bool _confirmationOpen = false;
  bool _locked = false;
  int _loadEpoch = 0;
  int _actionEpoch = 0;

  bool get _busy =>
      _picking || _mutating || _semanticEditorOpen || _buildDialogOpen;
  bool get _registryReady => _stages != null && !_loading && _loadError == null;
  bool get _effectivelyLocked => _locked || widget.requiresReopen;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    if (widget.requiresReopen) {
      _loadError = const Revision3DataAssetRequiresReopenException();
    } else {
      _reload();
    }
  }

  @override
  void didUpdateWidget(covariant Revision3DataAssetStagePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final checkpointChanged =
        oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHead.canonicalJson != widget.projectHead.canonicalJson;
    if (checkpointChanged) {
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
      _semanticEditorOpen = false;
      _buildDialogOpen = false;
      _semanticCheckpointStale = false;
      _confirmationOpen = false;
      if (widget.requiresReopen) {
        _loadEpoch++;
        _stages = null;
        _loading = false;
        _loadError = const Revision3DataAssetRequiresReopenException();
      } else {
        _reload(clearCurrent: true);
      }
      return;
    }
    if (oldWidget.requiresReopen != widget.requiresReopen) {
      if (widget.requiresReopen) {
        _loadEpoch++;
        _actionEpoch++;
        _loading = false;
        _picking = false;
        _mutating = false;
        _semanticEditorOpen = false;
        _buildDialogOpen = false;
        _confirmationOpen = false;
        _actionError = null;
        if (_stages == null) {
          _loadError = const Revision3DataAssetRequiresReopenException();
        }
      } else if (!_locked) {
        _reload(clearCurrent: _stages == null);
      }
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
    if (_effectivelyLocked) return;
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
        _semanticCheckpointStale = false;
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
    if (_busy || _effectivelyLocked || !_registryReady) return;
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
        'Verified DataAsset edit saved in project revision ${publication.projectRevision}. Expand it to build new mod files.',
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

  Future<void> _openSemanticEditor() async {
    final publish = widget.publishSemanticEdit;
    final receiptInspector = widget.semanticExtractReceiptInspector;
    if (_busy ||
        _effectivelyLocked ||
        !_registryReady ||
        publish == null ||
        receiptInspector == null) {
      return;
    }
    final projectRoot = widget.projectRoot;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectHeadJson = widget.projectHead.canonicalJson;
    setState(() {
      _semanticEditorOpen = true;
      _actionError = null;
    });
    final result = await showDialog<Object?>(
      context: context,
      builder: (context) => DataAssetSemanticEditWizardDialog(
        publish: publish,
        extractReceiptInspector: receiptInspector,
        inspector: widget.semanticInspector,
        uassetPicker: widget.semanticUassetPicker,
        usmapPicker: widget.semanticUsmapPicker,
        extractReceiptPicker: widget.semanticExtractReceiptPicker,
      ),
    );
    if (!mounted) return;
    setState(() => _semanticEditorOpen = false);
    if (!_sameCheckpoint(
      projectRoot,
      projectId,
      projectRevision,
      projectHeadJson,
    )) {
      return;
    }
    switch (result) {
      case DataAssetSemanticStagePublication publication:
        _showSuccess(
          'Verified value edit saved in project revision ${publication.revision}. Expand it to build new mod files.',
        );
      case DataAssetSemanticStageUnavailableException error:
        setState(() {
          _actionError = error.message;
          if (error.reason ==
              DataAssetSemanticStageUnavailableReason.staleCheckpoint) {
            _semanticCheckpointStale = true;
          }
          if (error.reason ==
              DataAssetSemanticStageUnavailableReason.requiresReopen) {
            _locked = true;
          }
        });
      case null:
        break;
      default:
        throw StateError('unexpected DataAsset value editor result');
    }
  }

  Future<void> _openBuildDialog(AuthoringRevision3DataAssetStage stage) async {
    final build = widget.buildReviewedStage;
    final picker = widget.pickBuildParentDirectory;
    if (_busy ||
        _effectivelyLocked ||
        !_registryReady ||
        build == null ||
        picker == null) {
      return;
    }
    final projectRoot = widget.projectRoot;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectHeadJson = widget.projectHead.canonicalJson;
    final targetPath = stage.targetPath;
    setState(() {
      _buildDialogOpen = true;
      _actionError = null;
    });
    await showDialog<AuthoringRevision3ReviewedDataAssetBuildResult>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3DataAssetBuildDialog(
        targetPath: targetPath,
        pickExistingParentDirectory: picker,
        build: ({required packName, required output}) {
          if (_effectivelyLocked) {
            throw const Revision3DataAssetRequiresReopenException();
          }
          if (!_sameCheckpoint(
            projectRoot,
            projectId,
            projectRevision,
            projectHeadJson,
          )) {
            throw const Revision3DataAssetStaleCheckpointException();
          }
          return build(
            targetPath: targetPath,
            packName: packName,
            output: output,
          );
        },
      ),
    );
    if (!mounted) return;
    setState(() => _buildDialogOpen = false);
  }

  Future<void> _removeStage(AuthoringRevision3DataAssetStage stage) async {
    if (_busy || _effectivelyLocked || !_registryReady || _confirmationOpen) {
      return;
    }
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
        _effectivelyLocked ||
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
      !_effectivelyLocked &&
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
        _DataAssetBoundaryNotice(locked: _effectivelyLocked),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
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
                    onPressed: _busy || _loading || _effectivelyLocked
                        ? null
                        : _reload,
                    icon: const Icon(Icons.refresh),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  alignment: WrapAlignment.end,
                  children: [
                    OutlinedButton.icon(
                      key: const Key('revision3-dataasset-browse-installed'),
                      onPressed: _busy || _effectivelyLocked
                          ? null
                          : widget.browseInstalledPackages,
                      icon: const Icon(Icons.travel_explore_outlined),
                      label: const Text('Browse installed packages...'),
                    ),
                    if (widget.publishSemanticEdit != null)
                      OutlinedButton.icon(
                        key: const Key('revision3-dataasset-semantic-create'),
                        onPressed:
                            _busy ||
                                _effectivelyLocked ||
                                _semanticCheckpointStale ||
                                widget.semanticExtractReceiptInspector ==
                                    null ||
                                !_registryReady
                            ? null
                            : _openSemanticEditor,
                        icon: const Icon(Icons.tune_outlined),
                        label: const Text('Create value edit...'),
                      ),
                    FilledButton.icon(
                      key: const Key('revision3-dataasset-stage-add'),
                      onPressed: _busy || _effectivelyLocked || !_registryReady
                          ? null
                          : _addVerifiedEdit,
                      icon: _picking || _mutating
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.add),
                      label: Text(
                        _picking ? 'Choosing...' : 'Import verified proof...',
                      ),
                    ),
                  ],
                ),
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
              retry: _effectivelyLocked || _loading ? null : _reload,
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
                onBuild:
                    _busy ||
                        _effectivelyLocked ||
                        !_registryReady ||
                        widget.buildReviewedStage == null ||
                        widget.pickBuildParentDirectory == null
                    ? null
                    : () => _openBuildDialog(visible[index]),
                buildUnavailableReason: widget.buildUnavailableReason,
                remove: _busy || _effectivelyLocked || !_registryReady
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
                  : 'These are verified value edits saved in this project. For supported reviewed edits, Build files creates a new mod-file folder without changing the project or game installation. Support is checked before files are created, and the action does not install or test the mod.',
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
  const _DataAssetStageTile({
    required this.stage,
    required this.onBuild,
    required this.buildUnavailableReason,
    required this.remove,
  });

  final AuthoringRevision3DataAssetStage stage;
  final VoidCallback? onBuild;
  final String? buildUnavailableReason;
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
              Chip(label: Text('Gameplay unverified')),
            ],
          ),
        ),
        if (onBuild == null && buildUnavailableReason != null) ...[
          const SizedBox(height: 10),
          Align(
            alignment: Alignment.centerLeft,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(Icons.info_outline, size: 18),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    buildUnavailableReason!,
                    key: ValueKey(
                      'revision3-dataasset-stage-build-unavailable-${stage.targetPath}',
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
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
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.end,
            children: [
              FilledButton.icon(
                key: ValueKey(
                  'revision3-dataasset-stage-build-${stage.targetPath}',
                ),
                onPressed: onBuild,
                icon: const Icon(Icons.inventory_2_outlined),
                label: const Text('Build files...'),
              ),
              OutlinedButton.icon(
                key: ValueKey(
                  'revision3-dataasset-stage-remove-${stage.targetPath}',
                ),
                onPressed: remove,
                icon: const Icon(Icons.remove_circle_outline),
                label: const Text('Remove from project...'),
              ),
            ],
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
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      const padding = 24.0;
      final minimumContentHeight = constraints.maxHeight.isFinite
          ? (constraints.maxHeight > padding * 2
                ? constraints.maxHeight - padding * 2
                : 0.0)
          : 0.0;
      return SingleChildScrollView(
        key: const Key('revision3-dataasset-stage-empty-scroll'),
        padding: const EdgeInsets.all(padding),
        child: ConstrainedBox(
          constraints: BoxConstraints(minHeight: minimumContentHeight),
          child: const Center(
            child: Column(
              key: Key('revision3-dataasset-stage-empty'),
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.data_object_outlined, size: 42),
                SizedBox(height: 12),
                Text('No verified DataAsset edits in this project.'),
                SizedBox(height: 6),
                Text(
                  'Use Create value edit for the guided inspect, preview, and exact ExtractReceipt-v2 workflow. Import verified proof is the expert alternative for an existing guarded PatchReceipt-v2.',
                  textAlign: TextAlign.center,
                ),
              ],
            ),
          ),
        ),
      );
    },
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
