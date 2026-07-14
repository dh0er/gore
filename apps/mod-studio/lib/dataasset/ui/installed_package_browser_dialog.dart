import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../core/mod_ffi.dart';
import '../../project/current_project_controller.dart';
import 'dataasset_lab.dart';
import 'installed_dataasset_semantic_edit_dialog.dart';

typedef Revision3InstalledPackageIndexLoader =
    Future<AuthoringRevision3DataAssetPackageIndexResult> Function({
      required String gameRoot,
    });

typedef Revision3InstalledDataAssetInspector =
    Future<AuthoringRevision3InstalledDataAssetInspectionResult> Function({
      required String gameRoot,
      required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
      required AuthoringRevision3DataAssetPackageCandidate candidate,
    });

/// Search-first browser over one exact installed package snapshot. Candidate
/// paths remain discovery metadata only. An optional typed publisher can
/// promote a proven editable leaf without granting caller-selected paths,
/// offsets, bytes, build, deployment, or runtime authority.
class InstalledPackageBrowserDialog extends StatefulWidget {
  const InstalledPackageBrowserDialog({
    required this.gameRoot,
    required this.load,
    this.inspect,
    this.publish,
    super.key,
  });

  final String gameRoot;
  final Revision3InstalledPackageIndexLoader load;
  final Revision3InstalledDataAssetInspector? inspect;
  final InstalledDataAssetSemanticStagePublisher? publish;

  @override
  State<InstalledPackageBrowserDialog> createState() =>
      _InstalledPackageBrowserDialogState();
}

class _InstalledPackageBrowserDialogState
    extends State<InstalledPackageBrowserDialog> {
  final _search = TextEditingController();
  final _manual = TextEditingController();
  Timer? _debounce;
  String _query = '';
  late Future<AuthoringRevision3DataAssetPackageIndexResult> _snapshot;
  int? _inspectingOrdinal;
  Object? _inspectionError;
  int _inspectionEpoch = 0;

  @override
  void initState() {
    super.initState();
    _snapshot = _load();
    _search.addListener(_searchChanged);
  }

  Future<AuthoringRevision3DataAssetPackageIndexResult> _load() =>
      widget.load(gameRoot: widget.gameRoot);

  void _refresh() {
    final next = _load();
    _inspectionEpoch += 1;
    setState(() {
      _snapshot = next;
      _inspectingOrdinal = null;
      _inspectionError = null;
    });
  }

  void _searchChanged() {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 180), () {
      if (!mounted) return;
      setState(() => _query = _search.text.trim().toLowerCase());
    });
  }

  @override
  void dispose() {
    _inspectionEpoch += 1;
    _debounce?.cancel();
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    _manual.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final viewport = MediaQuery.sizeOf(context);
    final textScale = MediaQuery.textScalerOf(context).scale(1);
    final width = (viewport.width - 96).clamp(280.0, 860.0).toDouble();
    final height = (viewport.height - 180 - (textScale - 1).clamp(0, 4) * 96)
        .clamp(160.0, 660.0)
        .toDouble();
    return AlertDialog(
      key: const Key('installed-package-browser-dialog'),
      title: const Text('Browse installed DataAsset packages'),
      content: SizedBox(
        width: width,
        height: height,
        child: FutureBuilder<AuthoringRevision3DataAssetPackageIndexResult>(
          future: _snapshot,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    CircularProgressIndicator(),
                    SizedBox(height: 16),
                    Text('Reading the exact installed package inventory…'),
                  ],
                ),
              );
            }
            final result = snapshot.data;
            if (result == null) {
              return _InstalledPackageError(
                error: snapshot.error ?? StateError('package audit failed'),
                retry: _refresh,
              );
            }
            return _buildResult(context, result);
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

  Widget _buildResult(
    BuildContext context,
    AuthoringRevision3DataAssetPackageIndexResult result,
  ) {
    final candidates = result.index.candidates;
    final visible = <AuthoringRevision3DataAssetPackageCandidate>[];
    var totalMatches = 0;
    if (_query.isNotEmpty) {
      for (final candidate in candidates) {
        final path = candidate.targetPath.toLowerCase();
        final slash = path.lastIndexOf('/');
        final name = slash < 0 ? path : path.substring(slash + 1);
        if (!path.contains(_query) && !name.contains(_query)) continue;
        totalMatches += 1;
        if (visible.length < 100) visible.add(candidate);
      }
    }
    final complete =
        result.index.status ==
        AuthoringRevision3DataAssetPackageIndexStatus.completeIndex;
    final candidateLabel = candidates.length == 1 ? 'candidate' : 'candidates';

    return CustomScrollView(
      key: const Key('installed-package-browser-result'),
      slivers: [
        SliverToBoxAdapter(
          child: Card(
            color: complete
                ? Colors.green.withValues(alpha: 0.10)
                : Colors.amber.withValues(alpha: 0.14),
            child: ListTile(
              leading: Icon(
                complete
                    ? Icons.verified_outlined
                    : Icons.warning_amber_outlined,
                color: complete ? Colors.green : Colors.amber.shade800,
              ),
              title: Text(
                complete
                    ? '${candidates.length} installed package $candidateLabel indexed'
                    : '${candidates.length} $candidateLabel indexed — partial result',
              ),
              subtitle: Text(
                complete
                    ? 'Directory metadata was read and the installed snapshot stayed exact.'
                    : 'Some package metadata was missing or noncanonical. Search results are useful for discovery but not complete.',
              ),
              trailing: IconButton(
                key: const Key('installed-package-browser-refresh'),
                tooltip: 'Read a fresh exact snapshot',
                onPressed: _refresh,
                icon: const Icon(Icons.refresh),
              ),
            ),
          ),
        ),
        const SliverToBoxAdapter(child: SizedBox(height: 8)),
        const SliverToBoxAdapter(
          child: Text(
            'Each Inspect action is bound to this exact snapshot and extracts only to bounded memory. Copying a path grants no extraction, edit, build, or deployment authority.',
          ),
        ),
        if (_inspectionError != null) ...[
          const SliverToBoxAdapter(child: SizedBox(height: 8)),
          SliverToBoxAdapter(
            child: _InstalledPackageInspectionError(error: _inspectionError!),
          ),
        ],
        const SliverToBoxAdapter(child: SizedBox(height: 10)),
        SliverToBoxAdapter(
          child: TextField(
            key: const Key('installed-package-browser-search'),
            controller: _search,
            autofocus: true,
            decoration: InputDecoration(
              prefixIcon: const Icon(Icons.search),
              hintText: 'Search asset name or /Game path…',
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
        const SliverToBoxAdapter(child: SizedBox(height: 8)),
        if (_query.isEmpty)
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.symmetric(vertical: 40),
              child: _SearchPrompt(),
            ),
          )
        else if (visible.isEmpty)
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.symmetric(vertical: 40),
              child: Center(child: Text('No matching installed package path')),
            ),
          )
        else ...[
          SliverToBoxAdapter(
            child: Text(
              totalMatches > visible.length
                  ? 'Showing the first ${visible.length} of $totalMatches matches'
                  : '$totalMatches match${totalMatches == 1 ? '' : 'es'}',
            ),
          ),
          const SliverToBoxAdapter(child: SizedBox(height: 4)),
          SliverList(
            key: const Key('installed-package-browser-results'),
            delegate: SliverChildBuilderDelegate((context, index) {
              final candidate = visible[index];
              final slash = candidate.targetPath.lastIndexOf('/');
              return ListTile(
                key: ValueKey('installed-package-${candidate.packageIdHex}'),
                leading: const Icon(Icons.data_object_outlined),
                title: Text(candidate.targetPath.substring(slash + 1)),
                subtitle: Text(candidate.targetPath),
                trailing: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (widget.inspect != null)
                      if (_inspectingOrdinal == candidate.ordinal)
                        const SizedBox.square(
                          dimension: 36,
                          child: Padding(
                            padding: EdgeInsets.all(8),
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        )
                      else
                        IconButton.filledTonal(
                          key: ValueKey(
                            'installed-package-inspect-${candidate.ordinal}',
                          ),
                          tooltip: 'Inspect exact installed package',
                          onPressed: _inspectingOrdinal == null
                              ? () => _inspectCandidate(result, candidate)
                              : null,
                          icon: const Icon(Icons.manage_search_outlined),
                        ),
                    IconButton(
                      tooltip: 'Copy /Game path',
                      onPressed: () => _copyPath(candidate.targetPath),
                      icon: const Icon(Icons.copy_outlined),
                    ),
                  ],
                ),
              );
            }, childCount: visible.length),
          ),
        ],
        const SliverToBoxAdapter(child: Divider(height: 20)),
        SliverToBoxAdapter(
          child: ExpansionTile(
            key: const Key('installed-package-browser-manual'),
            tilePadding: EdgeInsets.zero,
            title: const Text('Manual /Game path'),
            subtitle: const Text(
              'Fallback when a package is absent from metadata',
            ),
            children: [
              ValueListenableBuilder<TextEditingValue>(
                valueListenable: _manual,
                builder: (context, value, child) {
                  final manualPath = value.text.trim();
                  final manualValid = _isCanonicalGamePackagePath(manualPath);
                  return Row(
                    children: [
                      Expanded(
                        child: TextField(
                          key: const Key(
                            'installed-package-browser-manual-input',
                          ),
                          controller: _manual,
                          decoration: InputDecoration(
                            hintText: '/Game/Folder/AssetName',
                            errorText: manualPath.isEmpty || manualValid
                                ? null
                                : 'Use a canonical /Game path with letters, numbers, and underscores.',
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      IconButton.filledTonal(
                        key: const Key('installed-package-browser-manual-copy'),
                        tooltip: 'Copy validated path',
                        onPressed: manualValid
                            ? () => _copyPath(manualPath)
                            : null,
                        icon: const Icon(Icons.copy_outlined),
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
        SliverToBoxAdapter(
          child: ExpansionTile(
            key: const Key('installed-package-browser-advanced'),
            tilePadding: EdgeInsets.zero,
            title: const Text('Advanced snapshot evidence'),
            children: [
              _EvidenceValue(
                label: 'Physical chunks',
                value: '${result.index.physicalChunkCount}',
              ),
              _EvidenceValue(
                label: 'Winning packages',
                value: '${result.index.winningExportBundleCount}',
              ),
              _EvidenceValue(
                label: 'Directory indexed',
                value: '${result.index.directoryIndexedExportBundleCount}',
              ),
              _EvidenceValue(
                label: 'Out of scope',
                value: '${result.index.outOfScopeExportBundleCount}',
              ),
              _EvidenceValue(
                label: 'Mount entries',
                value: '${result.mountInventoryEntryCount}',
              ),
              for (final reason in result.index.partialReasons)
                _EvidenceValue(
                  label: _partialReasonLabel(reason.reason),
                  value: '${reason.count}',
                ),
              _EvidenceValue(
                label: 'Executable SHA-256',
                value: result.targetExecutableSeal.sha256,
              ),
              _EvidenceValue(
                label: 'Inventory SHA-256',
                value: result.mountInventorySeal.sha256,
              ),
              _EvidenceValue(
                label: 'Index SHA-256',
                value: result.packageIndexSeal.sha256,
              ),
              _EvidenceValue(
                label: 'Snapshot SHA-256',
                value: result.sourceSnapshotSeal.sha256,
              ),
            ],
          ),
        ),
        const SliverToBoxAdapter(child: SizedBox(height: 8)),
      ],
    );
  }

  Future<void> _copyPath(String path) async {
    await Clipboard.setData(ClipboardData(text: path));
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text('Copied $path')));
  }

  Future<void> _inspectCandidate(
    AuthoringRevision3DataAssetPackageIndexResult snapshot,
    AuthoringRevision3DataAssetPackageCandidate candidate,
  ) async {
    final inspect = widget.inspect;
    if (inspect == null || _inspectingOrdinal != null) return;
    final epoch = ++_inspectionEpoch;
    setState(() {
      _inspectingOrdinal = candidate.ordinal;
      _inspectionError = null;
    });
    try {
      final result = await inspect(
        gameRoot: widget.gameRoot,
        expectedSnapshot: snapshot,
        candidate: candidate,
      );
      if (!mounted || epoch != _inspectionEpoch) return;
      setState(() => _inspectingOrdinal = null);
      final editResult = await _showInspection(snapshot, candidate, result);
      if (!mounted || epoch != _inspectionEpoch || editResult == null) return;
      final messenger = ScaffoldMessenger.maybeOf(context);
      Navigator.of(context).pop();
      final publication = editResult.publication;
      messenger?.showSnackBar(
        SnackBar(
          content: Text(
            publication != null
                ? '${publication.targetPath} staged in project revision ${publication.revision}. Build and runtime remain blocked.'
                : editResult.unavailable!.message,
          ),
        ),
      );
    } catch (error) {
      if (!mounted || epoch != _inspectionEpoch) return;
      if (error
              is Revision3InstalledDataAssetInspectionStaleCheckpointException ||
          error
              is Revision3InstalledDataAssetInspectionRequiresReopenException) {
        final messenger = ScaffoldMessenger.maybeOf(context);
        Navigator.of(context).pop();
        messenger?.showSnackBar(
          SnackBar(
            content: Text(
              error
                      is Revision3InstalledDataAssetInspectionRequiresReopenException
                  ? 'The managed project must be reopened before installed packages can be inspected again.'
                  : 'The project or installed package snapshot changed. Reopen the browser from the current checkpoint.',
            ),
          ),
        );
        return;
      }
      setState(() {
        _inspectingOrdinal = null;
        _inspectionError = error;
      });
    }
  }

  Future<InstalledDataAssetSemanticEditResult?> _showInspection(
    AuthoringRevision3DataAssetPackageIndexResult snapshot,
    AuthoringRevision3DataAssetPackageCandidate candidate,
    AuthoringRevision3InstalledDataAssetInspectionResult result,
  ) {
    final viewport = MediaQuery.sizeOf(context);
    final textScale = MediaQuery.textScalerOf(context).scale(1);
    final width = (viewport.width - 72).clamp(320.0, 1040.0).toDouble();
    final height = (viewport.height - 170 - (textScale - 1).clamp(0, 4) * 112)
        .clamp(180.0, 760.0)
        .toDouble();
    return showDialog<InstalledDataAssetSemanticEditResult>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('installed-dataasset-inspection-dialog'),
        title: Text(result.targetPath),
        content: SizedBox(
          width: width,
          height: height,
          child: DataAssetInspectionReport(
            inspection: result.inspection,
            onEditLeaf: widget.publish == null
                ? null
                : (leaf) async {
                    final editResult =
                        await showDialog<InstalledDataAssetSemanticEditResult>(
                          context: dialogContext,
                          barrierDismissible: false,
                          builder: (context) =>
                              InstalledDataAssetSemanticEditDialog(
                                snapshot: snapshot,
                                candidate: candidate,
                                inspection: result,
                                leaf: leaf,
                                publish: widget.publish!,
                              ),
                        );
                    if (editResult != null && dialogContext.mounted) {
                      Navigator.of(dialogContext).pop(editResult);
                    }
                  },
            header: Card(
              color: Theme.of(dialogContext).colorScheme.secondaryContainer,
              child: const ListTile(
                leading: Icon(Icons.policy_outlined),
                title: Text('Installed package — exact evidence'),
                subtitle: Text(
                  'Extracted to bounded memory from one exact installed snapshot. Proven value leaves can be staged only after a fresh native re-read; build, runtime, and deployment remain unavailable.',
                ),
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('installed-dataasset-inspection-close'),
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}

class _SearchPrompt extends StatelessWidget {
  const _SearchPrompt();

  @override
  Widget build(BuildContext context) => const Center(
    key: Key('installed-package-browser-search-prompt'),
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.manage_search_outlined, size: 42),
        SizedBox(height: 10),
        Text('Type an asset name or /Game path to search'),
      ],
    ),
  );
}

class _InstalledPackageError extends StatelessWidget {
  const _InstalledPackageError({required this.error, required this.retry});

  final Object error;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) {
    final stale =
        error is Revision3DataAssetPackageIndexStaleCheckpointException;
    final requiresReopen =
        error is Revision3DataAssetPackageIndexRequiresReopenException;
    final mustClose = stale || requiresReopen;
    final message = stale
        ? 'The managed project changed while this browser was open. Close it and browse again from the current checkpoint.'
        : requiresReopen
        ? 'The managed project must be reopened before installed packages can be browsed again.'
        : error is ModFfiException
        ? (error as ModFfiException).message
        : 'No project, game, or save files were changed. Check the selected game installation and retry.';
    return Center(
      key: const Key('installed-package-browser-error'),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 540),
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
              'Installed package inventory could not be read',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton.icon(
              key: Key(
                mustClose
                    ? 'installed-package-browser-close-stale'
                    : 'installed-package-browser-retry',
              ),
              onPressed: mustClose ? () => Navigator.of(context).pop() : retry,
              icon: Icon(mustClose ? Icons.close : Icons.refresh),
              label: Text(mustClose ? 'Close browser' : 'Try again'),
            ),
          ],
        ),
      ),
    );
  }
}

class _InstalledPackageInspectionError extends StatelessWidget {
  const _InstalledPackageInspectionError({required this.error});

  final Object error;

  @override
  Widget build(BuildContext context) {
    final stale =
        error is Revision3InstalledDataAssetInspectionStaleCheckpointException;
    final requiresReopen =
        error is Revision3InstalledDataAssetInspectionRequiresReopenException;
    final message = stale
        ? 'The project or installed package snapshot changed. Refresh the browser before inspecting again.'
        : requiresReopen
        ? 'The managed project must be reopened before installed packages can be inspected again.'
        : error is ModFfiException
        ? _installedInspectionNativeMessage(error as ModFfiException)
        : 'Inspection failed without changing game or project files. Refresh the exact package snapshot and retry.';
    return Card(
      key: const Key('installed-package-inspection-error'),
      color: Theme.of(context).colorScheme.errorContainer,
      child: ListTile(
        leading: const Icon(Icons.error_outline),
        title: const Text('Installed package inspection failed'),
        subtitle: Text(message),
      ),
    );
  }
}

String _installedInspectionNativeMessage(
  ModFfiException error,
) => switch (error.code) {
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_MISSING' =>
    'No canonical generated .usmap was found. In UE4SS open Dumpers, run Generate .usmap, then refresh this browser.',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_CHANGED' =>
    'The generated .usmap inventory changed during inspection. Refresh and try the exact snapshot again.',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_GENERATION_MISMATCH' =>
    'This installation no longer matches the executable generation pinned by the project.',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_MISMATCH' ||
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_SOURCE_SNAPSHOT_MISMATCH' =>
    'The installed package inventory changed. Refresh before selecting the asset again.',
  _ => error.message,
};

class _EvidenceValue extends StatelessWidget {
  const _EvidenceValue({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 3),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(width: 170, child: Text(label)),
        Expanded(child: SelectableText(value)),
      ],
    ),
  );
}

String _partialReasonLabel(
  AuthoringRevision3DataAssetPackagePartialReason reason,
) => switch (reason) {
  AuthoringRevision3DataAssetPackagePartialReason
      .noncanonicalExportBundleChunkId =>
    'Noncanonical package chunk IDs',
  AuthoringRevision3DataAssetPackagePartialReason.missingDirectoryIndexPath =>
    'Missing Directory Index paths',
  AuthoringRevision3DataAssetPackagePartialReason
      .noncanonicalGameDirectoryIndexPath =>
    'Noncanonical game paths',
  AuthoringRevision3DataAssetPackagePartialReason.packageIdMismatch =>
    'Package ID mismatches',
};

bool _isCanonicalGamePackagePath(String path) {
  if (path.length > 512 ||
      !path.startsWith('/Game/') ||
      path.contains(r'\') ||
      path.endsWith('/')) {
    return false;
  }
  final segments = path.substring('/Game/'.length).split('/');
  if (segments.isEmpty || segments.length > 32) return false;
  const reserved = <String>{'CON', 'PRN', 'AUX', 'NUL'};
  final segmentPattern = RegExp(r'^[A-Za-z0-9_]+$');
  for (final segment in segments) {
    final upper = segment.toUpperCase();
    final numberedDevice =
        upper.length == 4 &&
        (upper.startsWith('COM') || upper.startsWith('LPT')) &&
        '123456789'.contains(upper.substring(3));
    if (!segmentPattern.hasMatch(segment) ||
        reserved.contains(upper) ||
        numberedDevice) {
      return false;
    }
  }
  return true;
}
