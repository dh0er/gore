import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_project_history.dart';

@immutable
final class Revision3ProjectHistoryPageCopy {
  const Revision3ProjectHistoryPageCopy({
    required this.title,
    required this.description,
    required this.projectOnlyBoundary,
    required this.refresh,
    required this.loading,
    required this.loadFailedTitle,
    required this.retry,
    required this.currentVersion,
    required this.previousVersions,
    required this.undoLastChange,
    required this.restoreVersion,
    required this.restoreDialogTitle,
    required this.restoreDialogBody,
    required this.restoreProjectOnlyBoundary,
    required this.cancel,
    required this.restore,
    required this.restoring,
    required this.restoreFailed,
    required this.restoreSucceeded,
    required this.noPreviousVersions,
    required this.recordingStartsAt,
    required this.olderVersionsExpired,
    required this.revisionLabel,
    required this.currentBadge,
  });

  final String title;
  final String description;
  final String projectOnlyBoundary;
  final String refresh;
  final String loading;
  final String loadFailedTitle;
  final String retry;
  final String currentVersion;
  final String previousVersions;
  final String undoLastChange;
  final String restoreVersion;
  final String restoreDialogTitle;
  final String Function(int revision, int nextRevision) restoreDialogBody;
  final String restoreProjectOnlyBoundary;
  final String cancel;
  final String restore;
  final String restoring;
  final String restoreFailed;
  final String Function(int revision) restoreSucceeded;
  final String noPreviousVersions;
  final String Function(int revision) recordingStartsAt;
  final String olderVersionsExpired;
  final String Function(int revision) revisionLabel;
  final String currentBadge;
}

/// Direct, non-technical timeline over the authenticated managed-R3 lineage.
///
/// This surface can only request a restore through [restore]. It cannot publish
/// a head, enumerate Store directories, or affect the game and save files.
class Revision3ProjectHistoryPage extends StatefulWidget {
  const Revision3ProjectHistoryPage({
    required this.checkpointIdentity,
    required this.load,
    required this.restore,
    required this.copy,
    required this.canRestore,
    this.restoreDisabledReason,
    super.key,
  });

  final Object checkpointIdentity;
  final Revision3ProjectHistoryLoader load;
  final Revision3ProjectHistoryRestorer restore;
  final Revision3ProjectHistoryPageCopy copy;
  final bool canRestore;
  final String? restoreDisabledReason;

  @override
  State<Revision3ProjectHistoryPage> createState() =>
      _Revision3ProjectHistoryPageState();
}

class _Revision3ProjectHistoryPageState
    extends State<Revision3ProjectHistoryPage> {
  Revision3ProjectHistorySnapshot? _history;
  Object? _loadError;
  bool _loading = false;
  bool _restoring = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  @override
  void didUpdateWidget(covariant Revision3ProjectHistoryPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.checkpointIdentity != widget.checkpointIdentity) {
      // Supersede an in-flight read instead of letting its old exact-head
      // result populate the newly rendered checkpoint.
      _loadEpoch++;
      _loading = false;
      _history = null;
      _loadError = null;
      unawaited(_load());
    }
  }

  Future<void> _load() async {
    if (_loading) return;
    final epoch = ++_loadEpoch;
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final history = await widget.load();
      if (!mounted || epoch != _loadEpoch) return;
      setState(() => _history = history);
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() => _loadError = error);
    } finally {
      if (mounted && epoch == _loadEpoch) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _confirmRestore(Revision3ProjectHistoryEntry target) async {
    final history = _history;
    if (history == null ||
        _restoring ||
        !widget.canRestore ||
        target.isCurrent) {
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(widget.copy.restoreDialogTitle),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              widget.copy.restoreDialogBody(
                target.projectRevision,
                history.currentRevision + 1,
              ),
            ),
            const SizedBox(height: 12),
            Text(widget.copy.restoreProjectOnlyBoundary),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(widget.copy.cancel),
          ),
          FilledButton(
            key: const Key('revision3-history-confirm-restore'),
            onPressed: () => Navigator.pop(context, true),
            child: Text(widget.copy.restore),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;

    setState(() => _restoring = true);
    try {
      final publication = await widget.restore(history, target);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            widget.copy.restoreSucceeded(publication.restoredFromRevision),
          ),
        ),
      );
    } catch (_) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(widget.copy.restoreFailed)));
    } finally {
      if (mounted) setState(() => _restoring = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_restoring,
    child: Stack(
      children: [
        RefreshIndicator(
          onRefresh: _load,
          child: CustomScrollView(
            key: const Key('revision3-project-history-page'),
            physics: const AlwaysScrollableScrollPhysics(),
            slivers: [
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(24, 24, 24, 8),
                sliver: SliverToBoxAdapter(child: _buildHeader(context)),
              ),
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(24, 8, 24, 32),
                sliver: _buildBody(context),
              ),
            ],
          ),
        ),
        if (_restoring)
          ColoredBox(
            key: const Key('revision3-history-restoring-barrier'),
            color: Theme.of(context).colorScheme.scrim.withValues(alpha: 0.18),
            child: Center(
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(20),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const SizedBox.square(
                        dimension: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      const SizedBox(width: 12),
                      Text(widget.copy.restoring),
                    ],
                  ),
                ),
              ),
            ),
          ),
      ],
    ),
  );

  Widget _buildHeader(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Row(
        children: [
          Expanded(
            child: Text(
              widget.copy.title,
              style: Theme.of(context).textTheme.headlineSmall,
            ),
          ),
          IconButton(
            key: const Key('revision3-history-refresh'),
            tooltip: widget.copy.refresh,
            onPressed: _loading || _restoring ? null : () => unawaited(_load()),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      const SizedBox(height: 6),
      Text(widget.copy.description),
      const SizedBox(height: 12),
      Container(
        width: double.infinity,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.secondaryContainer,
          borderRadius: BorderRadius.circular(10),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.shield_outlined, size: 20),
            const SizedBox(width: 10),
            Expanded(child: Text(widget.copy.projectOnlyBoundary)),
          ],
        ),
      ),
      if (widget.restoreDisabledReason case final reason?) ...[
        const SizedBox(height: 8),
        Text(
          reason,
          key: const Key('revision3-history-restore-disabled-reason'),
          style: TextStyle(color: Theme.of(context).colorScheme.error),
        ),
      ],
    ],
  );

  Widget _buildBody(BuildContext context) {
    final history = _history;
    if (_loading && history == null) {
      return SliverFillRemaining(
        hasScrollBody: false,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 12),
              Text(widget.copy.loading),
            ],
          ),
        ),
      );
    }
    if (_loadError != null && history == null) {
      return SliverFillRemaining(
        hasScrollBody: false,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline, size: 36),
              const SizedBox(height: 10),
              Text(widget.copy.loadFailedTitle),
              const SizedBox(height: 12),
              OutlinedButton.icon(
                onPressed: _loading ? null : () => unawaited(_load()),
                icon: const Icon(Icons.refresh),
                label: Text(widget.copy.retry),
              ),
            ],
          ),
        ),
      );
    }
    if (history == null) return const SliverToBoxAdapter(child: SizedBox());

    final previous = history.entries.skip(1).toList(growable: false);
    return SliverList.list(
      children: [
        Text(
          widget.copy.currentVersion,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        _HistoryEntryCard(
          entry: history.current,
          copy: widget.copy,
          action: null,
        ),
        const SizedBox(height: 20),
        Row(
          children: [
            Expanded(
              child: Text(
                widget.copy.previousVersions,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            if (history.immediatePrevious case final target?)
              FilledButton.tonalIcon(
                key: const Key('revision3-history-undo-last'),
                onPressed: widget.canRestore && !_restoring
                    ? () => unawaited(_confirmRestore(target))
                    : null,
                icon: const Icon(Icons.undo),
                label: Text(widget.copy.undoLastChange),
              ),
          ],
        ),
        const SizedBox(height: 8),
        if (previous.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 24),
            child: Text(widget.copy.noPreviousVersions),
          )
        else
          for (final entry in previous) ...[
            _HistoryEntryCard(
              entry: entry,
              copy: widget.copy,
              action: OutlinedButton(
                key: Key('revision3-history-restore-${entry.projectRevision}'),
                onPressed: widget.canRestore && !_restoring
                    ? () => unawaited(_confirmRestore(entry))
                    : null,
                child: Text(widget.copy.restoreVersion),
              ),
            ),
            const SizedBox(height: 8),
          ],
        if (history.earliestVisibleRevision > 0 &&
            !history.historyTruncated) ...[
          const SizedBox(height: 8),
          Text(
            widget.copy.recordingStartsAt(history.earliestVisibleRevision),
            key: const Key('revision3-history-recording-start'),
          ),
        ],
        if (history.historyTruncated) ...[
          const SizedBox(height: 8),
          Text(
            widget.copy.olderVersionsExpired,
            key: const Key('revision3-history-truncated'),
          ),
        ],
      ],
    );
  }
}

class _HistoryEntryCard extends StatelessWidget {
  const _HistoryEntryCard({
    required this.entry,
    required this.copy,
    required this.action,
  });

  final Revision3ProjectHistoryEntry entry;
  final Revision3ProjectHistoryPageCopy copy;
  final Widget? action;

  @override
  Widget build(BuildContext context) => Card.outlined(
    key: Key('revision3-history-entry-${entry.projectRevision}'),
    margin: EdgeInsets.zero,
    child: ListTile(
      leading: CircleAvatar(
        child: Icon(entry.isCurrent ? Icons.edit_note : Icons.history),
      ),
      title: Text(copy.revisionLabel(entry.projectRevision)),
      subtitle: entry.isCurrent ? Text(copy.currentBadge) : null,
      trailing: action,
    ),
  );
}
