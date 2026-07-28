import 'dart:async';

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

typedef Revision3InstalledDataAssetBrowser =
    Future<DataAssetSemanticStagePublication?> Function();

/// Programmatic navigation into the exact DataAsset stage registry shown by a
/// [Revision3DataAssetStagePanel].
///
/// Problems currently use a stage's canonical target path as [stageId]. A
/// request is bound to one exact project ID, revision, and canonical head. One
/// request may be buffered before the matching panel mounts; a newer buffered
/// request supersedes it. Project switches, same-revision head drift, detach,
/// and disposal resolve outstanding requests to `false` rather than following
/// another registry or claiming that a generic DataAsset surface was opened.
class Revision3DataAssetStagePanelController {
  Object? _attachment;
  String? _projectRoot;
  String? _projectId;
  int? _projectRevision;
  String? _projectHeadCanonicalJson;
  Future<bool> Function(
    String stageId, {
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  })?
  _openStageById;
  _PendingDataAssetStageNavigation? _bufferedNavigation;
  final Set<_PendingDataAssetStageNavigation> _forwardedNavigations = {};
  bool _disposed = false;

  /// Opens exactly [stageId] at the supplied managed-project checkpoint.
  ///
  /// [stageId] is the exact target path carried by a DataAsset-stage problem.
  /// The result is `true` only when that stage exists in the exact registry and
  /// the panel selected and scheduled it for expansion.
  Future<bool> openStageByIdAtCheckpoint(
    String stageId, {
    required String projectId,
    required int projectRevision,
    required String projectHeadCanonicalJson,
  }) {
    final navigation = _PendingDataAssetStageNavigation(
      stageId: stageId,
      projectId: projectId,
      projectRevision: projectRevision,
      projectHeadCanonicalJson: projectHeadCanonicalJson,
    );
    if (_disposed || !navigation.isValid) {
      navigation.result.complete(false);
      return navigation.result.future;
    }
    if (_attachment != null) return _forward(navigation);
    final superseded = _bufferedNavigation;
    _bufferedNavigation = navigation;
    if (superseded != null && !superseded.result.isCompleted) {
      superseded.result.complete(false);
    }
    return navigation.result.future;
  }

  /// Permanently releases this controller and resolves outstanding requests
  /// to `false`. A disposed controller cannot attach again.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _attachment = null;
    _projectRoot = null;
    _projectId = null;
    _projectRevision = null;
    _projectHeadCanonicalJson = null;
    _openStageById = null;
    _cancelNavigations();
  }

  Future<bool> _forward(_PendingDataAssetStageNavigation navigation) {
    final open = _openStageById;
    if (open == null || !_matches(navigation)) {
      navigation.result.complete(false);
      return navigation.result.future;
    }
    _forwardedNavigations.add(navigation);
    Future<bool>.sync(
      () => open(
        navigation.stageId,
        expectedProjectId: navigation.projectId,
        expectedProjectRevision: navigation.projectRevision,
        expectedProjectHeadCanonicalJson: navigation.projectHeadCanonicalJson,
      ),
    ).then(
      (resolved) => _completeForwarded(navigation, resolved),
      onError: (_, _) => _completeForwarded(navigation, false),
    );
    return navigation.result.future;
  }

  void _completeForwarded(
    _PendingDataAssetStageNavigation navigation,
    bool resolved,
  ) {
    _forwardedNavigations.remove(navigation);
    if (!navigation.result.isCompleted) {
      navigation.result.complete(resolved && _matches(navigation));
    }
  }

  bool _matches(_PendingDataAssetStageNavigation navigation) =>
      navigation.projectId == _projectId &&
      navigation.projectRevision == _projectRevision &&
      navigation.projectHeadCanonicalJson == _projectHeadCanonicalJson;

  bool _attach(
    Object attachment, {
    required String projectRoot,
    required String projectId,
    required int projectRevision,
    required String projectHeadCanonicalJson,
    required Future<bool> Function(
      String stageId, {
      required String expectedProjectId,
      required int expectedProjectRevision,
      required String expectedProjectHeadCanonicalJson,
    })
    openStageById,
  }) {
    if (_disposed ||
        (_attachment != null && !identical(_attachment, attachment))) {
      final buffered = _bufferedNavigation;
      _bufferedNavigation = null;
      if (buffered != null && !buffered.result.isCompleted) {
        buffered.result.complete(false);
      }
      return false;
    }
    _attachment = attachment;
    _projectRoot = projectRoot;
    _projectId = projectId;
    _projectRevision = projectRevision;
    _projectHeadCanonicalJson = projectHeadCanonicalJson;
    _openStageById = openStageById;
    final buffered = _bufferedNavigation;
    _bufferedNavigation = null;
    if (buffered != null) _forward(buffered);
    return true;
  }

  void _detach(Object attachment) {
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    _projectRoot = null;
    _projectId = null;
    _projectRevision = null;
    _projectHeadCanonicalJson = null;
    _openStageById = null;
    _cancelForwardedNavigations();
  }

  void _bindingChanged(
    Object attachment, {
    required String projectRoot,
    required String projectId,
    required int projectRevision,
    required String projectHeadCanonicalJson,
  }) {
    if (!identical(_attachment, attachment)) return;
    final rootChanged = _projectRoot != projectRoot;
    _projectRoot = projectRoot;
    _projectId = projectId;
    _projectRevision = projectRevision;
    _projectHeadCanonicalJson = projectHeadCanonicalJson;
    if (rootChanged) {
      _cancelForwardedNavigations();
      return;
    }
    final stale = _forwardedNavigations
        .where((navigation) => !_matches(navigation))
        .toList(growable: false);
    _forwardedNavigations.removeAll(stale);
    for (final navigation in stale) {
      if (!navigation.result.isCompleted) navigation.result.complete(false);
    }
  }

  void _cancelForwardedNavigations() {
    final forwarded = _forwardedNavigations.toList(growable: false);
    _forwardedNavigations.clear();
    for (final navigation in forwarded) {
      if (!navigation.result.isCompleted) navigation.result.complete(false);
    }
  }

  void _cancelNavigations() {
    final buffered = _bufferedNavigation;
    _bufferedNavigation = null;
    if (buffered != null && !buffered.result.isCompleted) {
      buffered.result.complete(false);
    }
    _cancelForwardedNavigations();
  }
}

class _PendingDataAssetStageNavigation {
  _PendingDataAssetStageNavigation({
    required this.stageId,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
  });

  final String stageId;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Completer<bool> result = Completer<bool>();

  bool get isValid =>
      stageId.isNotEmpty &&
      projectId.isNotEmpty &&
      projectRevision >= 0 &&
      projectHeadCanonicalJson.isNotEmpty;
}

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
    this.mutationsEnabled = true,
    this.mutationDisabledReason,
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
    this.controller,
    super.key,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead projectHead;
  final bool requiresReopen;
  final bool mutationsEnabled;
  final String? mutationDisabledReason;
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
  final Revision3InstalledDataAssetBrowser? browseInstalledPackages;
  final Revision3ReviewedDataAssetStageBuilder? buildReviewedStage;
  final Revision3DataAssetBuildParentDirectoryPicker? pickBuildParentDirectory;
  final String? buildUnavailableReason;
  final Revision3DataAssetStagePanelController? controller;

  @override
  State<Revision3DataAssetStagePanel> createState() =>
      _Revision3DataAssetStagePanelState();
}

class _Revision3DataAssetStagePanelState
    extends State<Revision3DataAssetStagePanel> {
  final _search = TextEditingController();
  final _headerScroll = ScrollController();
  List<AuthoringRevision3DataAssetStage>? _stages;
  Object? _loadError;
  String? _actionError;
  bool _loading = false;
  bool _picking = false;
  bool _mutating = false;
  bool _installedBrowserOpen = false;
  bool _semanticEditorOpen = false;
  bool _buildDialogOpen = false;
  bool _semanticCheckpointStale = false;
  bool _confirmationOpen = false;
  bool _locked = false;
  int _loadEpoch = 0;
  int _actionEpoch = 0;
  int _installedBrowserEpoch = 0;
  String? _focusedTargetPath;
  int? _focusedProjectRevision;
  bool _focusedStageRequiresPublishedRevision = false;
  String? _stageRevealMessage;
  final Map<String, ExpansibleController> _stageExpansionControllers = {};
  final Map<String, GlobalKey> _stageFocusKeys = {};
  final List<_PendingDataAssetStageNavigation> _pendingStageNavigations = [];

  bool get _busy =>
      _picking ||
      _mutating ||
      _installedBrowserOpen ||
      _semanticEditorOpen ||
      _buildDialogOpen;
  bool get _registryReady => _stages != null && !_loading && _loadError == null;
  bool get _effectivelyLocked => _locked || widget.requiresReopen;
  bool get _mutationsLocked => _effectivelyLocked || !widget.mutationsEnabled;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    if (widget.requiresReopen) {
      _loadError = const Revision3DataAssetRequiresReopenException();
    }
    _attachController(widget.controller);
    if (!widget.requiresReopen) _reload();
  }

  @override
  void didUpdateWidget(covariant Revision3DataAssetStagePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final controllerChanged = !identical(
      oldWidget.controller,
      widget.controller,
    );
    final checkpointChanged =
        oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHead.canonicalJson != widget.projectHead.canonicalJson;
    if (controllerChanged) {
      _cancelPendingStageNavigations();
      oldWidget.controller?._detach(this);
    } else if (checkpointChanged) {
      widget.controller?._bindingChanged(
        this,
        projectRoot: widget.projectRoot,
        projectId: widget.projectId,
        projectRevision: widget.projectRevision,
        projectHeadCanonicalJson: widget.projectHead.canonicalJson,
      );
    }
    if (checkpointChanged) {
      _cancelPendingStageNavigations();
      final projectIdentityChanged =
          oldWidget.projectRoot != widget.projectRoot ||
          oldWidget.projectId != widget.projectId;
      if (projectIdentityChanged ||
          oldWidget.projectHead.canonicalJson !=
              widget.projectHead.canonicalJson) {
        _actionEpoch++;
      }
      if (projectIdentityChanged) {
        _installedBrowserEpoch++;
        _focusedTargetPath = null;
        _focusedProjectRevision = null;
        _focusedStageRequiresPublishedRevision = false;
        _stageRevealMessage = null;
        _search.clear();
      } else if (_focusedProjectRevision != widget.projectRevision ||
          (oldWidget.projectRevision == widget.projectRevision &&
              oldWidget.projectHead.canonicalJson !=
                  widget.projectHead.canonicalJson)) {
        _focusedTargetPath = null;
        _focusedProjectRevision = null;
        _focusedStageRequiresPublishedRevision = false;
        _stageRevealMessage = null;
        _search.clear();
      }
      _actionError = null;
      _locked = false;
      _picking = false;
      _mutating = false;
      _installedBrowserOpen = false;
      _semanticEditorOpen = false;
      _buildDialogOpen = false;
      _semanticCheckpointStale = false;
      _confirmationOpen = false;
      if (widget.requiresReopen) {
        _loadEpoch++;
        _installedBrowserEpoch++;
        _stages = null;
        _loading = false;
        _loadError = const Revision3DataAssetRequiresReopenException();
      } else {
        _reload(clearCurrent: true);
      }
      if (controllerChanged) _attachController(widget.controller);
      return;
    }
    if (controllerChanged) _attachController(widget.controller);
    if (oldWidget.requiresReopen != widget.requiresReopen) {
      if (widget.requiresReopen) {
        _cancelPendingStageNavigations();
        _loadEpoch++;
        _actionEpoch++;
        _installedBrowserEpoch++;
        _loading = false;
        _picking = false;
        _mutating = false;
        _installedBrowserOpen = false;
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
    _installedBrowserEpoch++;
    _cancelPendingStageNavigations();
    widget.controller?._detach(this);
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    _headerScroll.dispose();
    for (final controller in _stageExpansionControllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  void _attachController(Revision3DataAssetStagePanelController? controller) {
    controller?._attach(
      this,
      projectRoot: widget.projectRoot,
      projectId: widget.projectId,
      projectRevision: widget.projectRevision,
      projectHeadCanonicalJson: widget.projectHead.canonicalJson,
      openStageById: _openStageByIdAtCheckpoint,
    );
  }

  Future<bool> _openStageByIdAtCheckpoint(
    String stageId, {
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) {
    if (!mounted ||
        _effectivelyLocked ||
        !_matchesStageCheckpoint(
          expectedProjectId,
          expectedProjectRevision,
          expectedProjectHeadCanonicalJson,
        )) {
      return Future<bool>.value(false);
    }
    if (_loading || (_stages == null && _loadError == null)) {
      final pending = _PendingDataAssetStageNavigation(
        stageId: stageId,
        projectId: expectedProjectId,
        projectRevision: expectedProjectRevision,
        projectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
      );
      _pendingStageNavigations.add(pending);
      return pending.result.future;
    }
    final stages = _stages;
    if (_loadError != null || stages == null) {
      return Future<bool>.value(false);
    }
    return Future<bool>.value(
      _resolveExactStage(
        stages,
        stageId,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        expectedProjectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
      ),
    );
  }

  bool _resolveExactStage(
    List<AuthoringRevision3DataAssetStage> stages,
    String stageId, {
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) {
    if (!_matchesStageCheckpoint(
      expectedProjectId,
      expectedProjectRevision,
      expectedProjectHeadCanonicalJson,
    )) {
      return false;
    }
    AuthoringRevision3DataAssetStage? exactStage;
    for (final stage in stages) {
      if (stage.targetPath == stageId) {
        exactStage = stage;
        break;
      }
    }
    if (exactStage == null || exactStage.projectId != expectedProjectId) {
      return false;
    }
    _focusedTargetPath = exactStage.targetPath;
    _focusedProjectRevision = expectedProjectRevision;
    _focusedStageRequiresPublishedRevision = false;
    _stageRevealMessage = null;
    _search.text = exactStage.targetPath;
    _scheduleFocusedStageReveal();
    return true;
  }

  bool _matchesStageCheckpoint(
    String projectId,
    int projectRevision,
    String projectHeadCanonicalJson,
  ) =>
      widget.projectId == projectId &&
      widget.projectRevision == projectRevision &&
      widget.projectHead.canonicalJson == projectHeadCanonicalJson;

  void _resolvePendingStageNavigations(
    List<AuthoringRevision3DataAssetStage> stages,
  ) {
    final pending = List<_PendingDataAssetStageNavigation>.of(
      _pendingStageNavigations,
    );
    _pendingStageNavigations.clear();
    for (final navigation in pending) {
      final resolved =
          mounted &&
          _resolveExactStage(
            stages,
            navigation.stageId,
            expectedProjectId: navigation.projectId,
            expectedProjectRevision: navigation.projectRevision,
            expectedProjectHeadCanonicalJson:
                navigation.projectHeadCanonicalJson,
          );
      if (!navigation.result.isCompleted) {
        navigation.result.complete(resolved);
      }
    }
  }

  void _cancelPendingStageNavigations() {
    final pending = List<_PendingDataAssetStageNavigation>.of(
      _pendingStageNavigations,
    );
    _pendingStageNavigations.clear();
    for (final navigation in pending) {
      if (!navigation.result.isCompleted) navigation.result.complete(false);
    }
  }

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
      final focusTarget = _focusedTargetPath;
      final focusRevision = _focusedProjectRevision;
      final focusRequiresPublishedRevision =
          _focusedStageRequiresPublishedRevision;
      AuthoringRevision3DataAssetStage? focusedStage;
      if (focusTarget != null && focusRevision == widget.projectRevision) {
        for (final stage in stages) {
          if (stage.targetPath == focusTarget &&
              (!focusRequiresPublishedRevision ||
                  stage.stagedProjectRevision == focusRevision)) {
            focusedStage = stage;
            break;
          }
        }
      }
      final focusMissing =
          focusTarget != null &&
          focusRevision == widget.projectRevision &&
          focusedStage == null;
      setState(() {
        _stages = List<AuthoringRevision3DataAssetStage>.unmodifiable(stages);
        _loading = false;
        _semanticCheckpointStale = false;
        if (focusedStage != null) {
          _stageRevealMessage =
              '${_assetName(focusedStage.targetPath)} is saved and opened below. Review it, then use Build files if support for this exact edit is confirmed.';
        } else if (focusMissing) {
          _focusedTargetPath = null;
          _focusedProjectRevision = null;
          _focusedStageRequiresPublishedRevision = false;
          _stageRevealMessage = null;
          _actionError =
              'The newly saved DataAsset edit was not present at its published project revision. Refresh the exact project list before continuing.';
        }
      });
      _resolvePendingStageNavigations(stages);
      if (focusMissing) {
        _search.clear();
      } else {
        _scheduleFocusedStageReveal();
      }
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      _cancelPendingStageNavigations();
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
    if (_busy || _mutationsLocked || !_registryReady) return;
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
        'Verified DataAsset edit saved in project revision ${publication.projectRevision}. Review it below; offline build is available only for supported reviewed edits.',
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

  Future<void> _browseInstalledPackages() async {
    final browse = widget.browseInstalledPackages;
    if (_busy || _effectivelyLocked || browse == null) return;
    final projectRoot = widget.projectRoot;
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectHeadJson = widget.projectHead.canonicalJson;
    final epoch = ++_installedBrowserEpoch;
    setState(() {
      _installedBrowserOpen = true;
      _actionError = null;
    });
    try {
      final publication = await browse();
      if (!mounted || epoch != _installedBrowserEpoch) return;
      setState(() => _installedBrowserOpen = false);
      if (publication == null) return;
      if (publication.targetPath.isEmpty ||
          publication.revision != projectRevision + 1) {
        throw const FormatException(
          'Installed DataAsset publication does not advance the opened project checkpoint.',
        );
      }
      final stillAtOpeningCheckpoint =
          widget.projectRevision == projectRevision &&
          widget.projectHead.canonicalJson == projectHeadJson;
      final advancedToPublication =
          widget.projectRevision == publication.revision;
      if (widget.projectRoot != projectRoot ||
          widget.projectId != projectId ||
          (!stillAtOpeningCheckpoint && !advancedToPublication)) {
        return;
      }
      _focusPublishedStage(publication);
      if (widget.projectRevision == publication.revision &&
          !_effectivelyLocked) {
        await _reload(clearCurrent: true);
      }
    } catch (error) {
      if (!mounted || epoch != _installedBrowserEpoch) return;
      setState(() {
        _installedBrowserOpen = false;
        _actionError = revision3DataAssetFriendlyError(error);
        if (error is Revision3DataAssetRequiresReopenException) _locked = true;
      });
    }
  }

  void _focusPublishedStage(DataAssetSemanticStagePublication publication) {
    _focusedTargetPath = publication.targetPath;
    _focusedProjectRevision = publication.revision;
    _focusedStageRequiresPublishedRevision = true;
    _search.text = publication.targetPath;
  }

  ExpansibleController _stageExpansionController(String targetPath) =>
      _stageExpansionControllers.putIfAbsent(
        targetPath.toLowerCase(),
        ExpansibleController.new,
      );

  GlobalKey _stageFocusKey(String targetPath) =>
      _stageFocusKeys.putIfAbsent(targetPath.toLowerCase(), GlobalKey.new);

  void _scheduleFocusedStageReveal() {
    final targetPath = _focusedTargetPath;
    final focusedRevision = _focusedProjectRevision;
    if (targetPath == null || focusedRevision != widget.projectRevision) {
      return;
    }
    final stages = _stages;
    if (stages == null ||
        !stages.any(
          (stage) =>
              stage.targetPath == targetPath &&
              (!_focusedStageRequiresPublishedRevision ||
                  stage.stagedProjectRevision == focusedRevision),
        )) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          _focusedProjectRevision != widget.projectRevision ||
          _focusedTargetPath != targetPath) {
        return;
      }
      final folded = targetPath.toLowerCase();
      final controller = _stageExpansionControllers[folded];
      if (controller != null && !controller.isExpanded) controller.expand();
      final focusContext = _stageFocusKeys[folded]?.currentContext;
      if (focusContext != null) {
        Scrollable.ensureVisible(
          focusContext,
          alignment: 0.08,
          duration: const Duration(milliseconds: 180),
        );
      }
      setState(() {
        _focusedTargetPath = null;
        _focusedProjectRevision = null;
        _focusedStageRequiresPublishedRevision = false;
      });
    });
  }

  Future<void> _openSemanticEditor() async {
    final publish = widget.publishSemanticEdit;
    final receiptInspector = widget.semanticExtractReceiptInspector;
    if (_busy ||
        _mutationsLocked ||
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
          'Verified value edit saved in project revision ${publication.revision}. Review it below; offline build is available only for supported reviewed edits.',
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
        _mutationsLocked ||
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
          if (_mutationsLocked) {
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
    if (_busy || _mutationsLocked || !_registryReady || _confirmationOpen) {
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
        _mutationsLocked ||
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
      !_mutationsLocked &&
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
    return LayoutBuilder(
      builder: (context, constraints) {
        final headerMaxHeight = constraints.maxHeight.isFinite
            ? (constraints.maxHeight * 0.6).clamp(160.0, 460.0).toDouble()
            : 460.0;
        return Column(
          key: const Key('revision3-dataasset-stage-panel'),
          children: [
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: headerMaxHeight),
              child: Scrollbar(
                key: const Key('revision3-dataasset-stage-header-scrollbar'),
                controller: _headerScroll,
                thumbVisibility: true,
                child: SingleChildScrollView(
                  key: const Key('revision3-dataasset-stage-header-scroll'),
                  controller: _headerScroll,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _DataAssetBoundaryNotice(
                        locked: _mutationsLocked,
                        lockedReason: !widget.mutationsEnabled
                            ? widget.mutationDisabledReason
                            : null,
                      ),
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
                                    style: Theme.of(
                                      context,
                                    ).textTheme.titleLarge,
                                  ),
                                ),
                                IconButton(
                                  key: const Key(
                                    'revision3-dataasset-stage-refresh',
                                  ),
                                  tooltip: 'Refresh exact project list',
                                  onPressed:
                                      _busy || _loading || _effectivelyLocked
                                      ? null
                                      : _reload,
                                  icon: const Icon(Icons.refresh),
                                ),
                              ],
                            ),
                            const SizedBox(height: 10),
                            _ReviewedDataAssetQuickStart(
                              browseAvailable:
                                  widget.browseInstalledPackages != null,
                              onBrowse:
                                  _busy ||
                                      _mutationsLocked ||
                                      widget.browseInstalledPackages == null
                                  ? null
                                  : _browseInstalledPackages,
                              busy: _installedBrowserOpen,
                            ),
                            const SizedBox(height: 8),
                            _DataAssetExpertTools(
                              semanticEditAvailable:
                                  widget.publishSemanticEdit != null,
                              onCreateSemanticEdit:
                                  _busy ||
                                      _mutationsLocked ||
                                      _semanticCheckpointStale ||
                                      widget.semanticExtractReceiptInspector ==
                                          null ||
                                      !_registryReady
                                  ? null
                                  : _openSemanticEditor,
                              onImportProof:
                                  _busy || _mutationsLocked || !_registryReady
                                  ? null
                                  : _addVerifiedEdit,
                              picking: _picking,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
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
            if (_stageRevealMessage != null)
              _DataAssetStageRevealNotice(
                message: _stageRevealMessage!,
                onDismiss: () => setState(() => _stageRevealMessage = null),
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
                  _DataAssetEmptyState(
                    browseAvailable: widget.browseInstalledPackages != null,
                    onBrowse:
                        _busy ||
                            _effectivelyLocked ||
                            widget.browseInstalledPackages == null
                        ? null
                        : _browseInstalledPackages,
                  ),
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
                    expansionController: _stageExpansionController(
                      visible[index].targetPath,
                    ),
                    focusKey: _stageFocusKey(visible[index].targetPath),
                    onBuild:
                        _busy ||
                            _mutationsLocked ||
                            !_registryReady ||
                            widget.buildReviewedStage == null ||
                            widget.pickBuildParentDirectory == null
                        ? null
                        : () => _openBuildDialog(visible[index]),
                    buildUnavailableReason: widget.buildUnavailableReason,
                    remove: _busy || _mutationsLocked || !_registryReady
                        ? null
                        : () => _removeStage(visible[index]),
                  ),
                ),
              },
            ),
          ],
        );
      },
    );
  }
}

class _DataAssetBoundaryNotice extends StatelessWidget {
  const _DataAssetBoundaryNotice({
    required this.locked,
    required this.lockedReason,
  });

  final bool locked;
  final String? lockedReason;

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
                  ? lockedReason ??
                        'Exact verification is unavailable. Reopen the managed project before continuing.'
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

class _ReviewedDataAssetQuickStart extends StatelessWidget {
  const _ReviewedDataAssetQuickStart({
    required this.browseAvailable,
    required this.onBrowse,
    required this.busy,
  });

  final bool browseAvailable;
  final VoidCallback? onBrowse;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Card(
      key: const Key('revision3-dataasset-reviewed-quick-start'),
      color: scheme.primaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  Icons.auto_awesome_outlined,
                  color: scheme.onPrimaryContainer,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Start with a reviewed preset',
                        style: Theme.of(context).textTheme.titleMedium
                            ?.copyWith(color: scheme.onPrimaryContainer),
                      ),
                      const SizedBox(height: 3),
                      Text(
                        'Choose an installed Human, Scavenger, or Wolf footstep preset, inspect its exact game data, then preview and save a reviewed, bounded X/Y texture-size project edit.',
                        style: TextStyle(color: scheme.onPrimaryContainer),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              key: const Key('revision3-dataasset-reviewed-presets'),
              spacing: 8,
              runSpacing: 6,
              children: [
                for (final target in footstepPresetReviewedSchema.targets)
                  Chip(
                    key: ValueKey(
                      'revision3-dataasset-reviewed-preset-${target.assetName}',
                    ),
                    avatar: const Icon(Icons.pets_outlined, size: 18),
                    label: Text(target.friendlyName),
                  ),
              ],
            ),
            const SizedBox(height: 10),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.icon(
                key: const Key('revision3-dataasset-browse-installed'),
                onPressed: onBrowse,
                icon: busy
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.travel_explore_outlined),
                label: Text(
                  busy
                      ? 'Opening installed data...'
                      : 'Edit installed preset...',
                ),
              ),
            ),
            if (!browseAvailable) ...[
              const SizedBox(height: 8),
              Text(
                'Choose the Gothic 1 Remake installation in Settings to inspect reviewed presets.',
                key: const Key(
                  'revision3-dataasset-reviewed-browser-unavailable',
                ),
                style: TextStyle(color: scheme.onPrimaryContainer),
              ),
            ],
            const SizedBox(height: 8),
            Text(
              'Saving changes only this project. Supported reviewed edits can later create offline mod files; installation, deployment, and gameplay remain separate and unverified.',
              style: TextStyle(color: scheme.onPrimaryContainer),
            ),
          ],
        ),
      ),
    );
  }
}

class _DataAssetExpertTools extends StatelessWidget {
  const _DataAssetExpertTools({
    required this.semanticEditAvailable,
    required this.onCreateSemanticEdit,
    required this.onImportProof,
    required this.picking,
  });

  final bool semanticEditAvailable;
  final VoidCallback? onCreateSemanticEdit;
  final VoidCallback? onImportProof;
  final bool picking;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    clipBehavior: Clip.antiAlias,
    child: ExpansionTile(
      key: const Key('revision3-dataasset-expert-tools'),
      leading: const Icon(Icons.construction_outlined),
      title: const Text('Expert tools'),
      subtitle: const Text('Advanced import and generic value editing'),
      childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 14),
      children: [
        const Divider(),
        const Align(
          alignment: Alignment.centerLeft,
          child: Text(
            'Use these only when you already have exact extraction or patch proof. They do not grant structural editing, deployment, or runtime authority.',
          ),
        ),
        const SizedBox(height: 10),
        Align(
          alignment: Alignment.centerLeft,
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              if (semanticEditAvailable)
                OutlinedButton.icon(
                  key: const Key('revision3-dataasset-semantic-create'),
                  onPressed: onCreateSemanticEdit,
                  icon: const Icon(Icons.tune_outlined),
                  label: const Text('Create generic value edit...'),
                ),
              FilledButton.tonalIcon(
                key: const Key('revision3-dataasset-stage-add'),
                onPressed: onImportProof,
                icon: picking
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.receipt_long_outlined),
                label: Text(
                  picking ? 'Choosing...' : 'Import verified proof...',
                ),
              ),
            ],
          ),
        ),
      ],
    ),
  );
}

class _DataAssetStageTile extends StatelessWidget {
  const _DataAssetStageTile({
    required this.stage,
    required this.expansionController,
    required this.focusKey,
    required this.onBuild,
    required this.buildUnavailableReason,
    required this.remove,
  });

  final AuthoringRevision3DataAssetStage stage;
  final ExpansibleController expansionController;
  final GlobalKey focusKey;
  final VoidCallback? onBuild;
  final String? buildUnavailableReason;
  final VoidCallback? remove;

  @override
  Widget build(BuildContext context) => Container(
    key: focusKey,
    child: Card(
      child: ExpansionTile(
        key: ValueKey('revision3-dataasset-stage-${stage.targetPath}'),
        controller: expansionController,
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
  const _DataAssetEmptyState({
    required this.browseAvailable,
    required this.onBrowse,
  });

  final bool browseAvailable;
  final VoidCallback? onBrowse;

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
          child: Center(
            child: Column(
              key: const Key('revision3-dataasset-stage-empty'),
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.data_object_outlined, size: 42),
                const SizedBox(height: 12),
                const Text('No verified DataAsset edits in this project.'),
                const SizedBox(height: 6),
                Text(
                  browseAvailable
                      ? 'Start with an installed reviewed preset. You can inspect its exact values, preview the change, and save it without changing the game installation.'
                      : 'Choose the Gothic 1 Remake installation in Settings, then return here to inspect a reviewed preset.',
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 14),
                FilledButton.icon(
                  key: const Key('revision3-dataasset-empty-browse-installed'),
                  onPressed: onBrowse,
                  icon: const Icon(Icons.travel_explore_outlined),
                  label: const Text('Edit installed preset...'),
                ),
                const SizedBox(height: 10),
                const Text(
                  'Advanced receipt workflows remain available under Expert tools.',
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
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SingleChildScrollView(
      key: const Key('revision3-dataasset-stage-error-scroll'),
      child: ConstrainedBox(
        constraints: BoxConstraints(minHeight: constraints.maxHeight),
        child: Center(
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
        ),
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

class _DataAssetStageRevealNotice extends StatelessWidget {
  const _DataAssetStageRevealNotice({
    required this.message,
    required this.onDismiss,
  });

  final String message;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) => Semantics(
    container: true,
    liveRegion: true,
    label: message,
    child: Container(
      key: const Key('revision3-dataasset-stage-reveal-notice'),
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
      color: Theme.of(context).colorScheme.primaryContainer,
      child: Row(
        children: [
          const Icon(Icons.check_circle_outline),
          const SizedBox(width: 8),
          Expanded(child: ExcludeSemantics(child: Text(message))),
          IconButton(
            key: const Key('revision3-dataasset-stage-reveal-dismiss'),
            tooltip: 'Dismiss saved edit notice',
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
