import 'dart:async';

import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';

typedef Revision3VoiceBuildPlanLoader =
    Future<AuthoringRevision3VoiceBuildPlanResult> Function();

typedef Revision3VoiceBuildLineLocaleAction =
    FutureOr<void> Function({
      required String initialLineId,
      required String initialLocale,
    });

typedef Revision3VoiceBuildReadinessAction = FutureOr<void> Function();

typedef Revision3VoiceBuildReadyCountCopy =
    String Function(int readySlots, int totalSlots);
typedef Revision3VoiceBuildBlockerCountCopy = String Function(int count);
typedef Revision3VoiceBuildRevisionCopy = String Function(int revision);
typedef Revision3VoiceBuildBlockerTitleCopy =
    String Function(AuthoringRevision3VoiceBuildBlockReason reason);

/// All author-facing copy rendered by the Voice build readiness surfaces.
///
/// The managed workspace supplies this from its localization layer. Stable
/// project and line identities remain callback-only and are never interpolated
/// into presentation copy.
@immutable
final class Revision3VoiceBuildReadinessCopy {
  const Revision3VoiceBuildReadinessCopy({
    required this.title,
    required this.refreshTooltip,
    required this.checkingSemanticsLabel,
    required this.loadError,
    required this.retryLabel,
    required this.readyTitle,
    required this.blockedTitle,
    required this.readyCount,
    required this.blockedBoundary,
    required this.buildBundleLabel,
    required this.readyBuildReleaseGuidance,
    required this.readyConfigureGameGuidance,
    required this.hideBlockersLabel,
    required this.showBlockersLabel,
    required this.workflowOpenFailed,
    required this.buildWorkflowOpenFailed,
    required this.exactProjectRevision,
    required this.resolveTargetLabel,
    required this.manageTakesLabel,
    required this.blockerTitle,
  });

  const Revision3VoiceBuildReadinessCopy.english()
    : title = 'Voice readiness',
      refreshTooltip = 'Refresh Voice readiness',
      checkingSemanticsLabel = 'Checking exact Voice readiness',
      loadError =
          'Voice readiness could not be verified for the current project. No build is available from this result.',
      retryLabel = 'Retry',
      readyTitle = 'Voice is ready',
      blockedTitle = 'Voice needs attention',
      readyCount = _englishVoiceReadyCount,
      blockedBoundary =
          'No bundle was created and deployment was not performed.',
      buildBundleLabel = 'Build bundle',
      readyBuildReleaseGuidance =
          'Voice content is ready. Open Build & Release to create the offline bundle.',
      readyConfigureGameGuidance =
          'Voice content is ready. Configure the game installation before creating an offline bundle.',
      hideBlockersLabel = 'Hide blockers',
      showBlockersLabel = _englishVoiceBlockerCount,
      workflowOpenFailed =
          'The selected Voice workflow could not be opened. Refresh and try again.',
      buildWorkflowOpenFailed = 'The Voice build workflow could not be opened.',
      exactProjectRevision = _englishVoiceProjectRevision,
      resolveTargetLabel = 'Resolve target',
      manageTakesLabel = 'Manage takes',
      blockerTitle = _englishVoiceBlockerTitle;

  final String title;
  final String refreshTooltip;
  final String checkingSemanticsLabel;
  final String loadError;
  final String retryLabel;
  final String readyTitle;
  final String blockedTitle;
  final Revision3VoiceBuildReadyCountCopy readyCount;
  final String blockedBoundary;
  final String buildBundleLabel;
  final String readyBuildReleaseGuidance;
  final String readyConfigureGameGuidance;
  final String hideBlockersLabel;
  final Revision3VoiceBuildBlockerCountCopy showBlockersLabel;
  final String workflowOpenFailed;
  final String buildWorkflowOpenFailed;
  final Revision3VoiceBuildRevisionCopy exactProjectRevision;
  final String resolveTargetLabel;
  final String manageTakesLabel;
  final Revision3VoiceBuildBlockerTitleCopy blockerTitle;
}

/// Compact exact-current Voice readiness for Validate & Test and build flows.
///
/// This surface owns no build or project-mutation authority. It renders only a
/// caller-supplied native plan, verifies that the result still belongs to the
/// requested project tuple, and carries hidden line identities only through
/// explicit callbacks.
class Revision3VoiceBuildReadinessPanel extends StatefulWidget {
  const Revision3VoiceBuildReadinessPanel({
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.plan,
    this.onResolveVoiceTarget,
    this.onManageVoiceTakes,
    this.onBuild,
    this.gameConfigured = false,
    this.copy = const Revision3VoiceBuildReadinessCopy.english(),
    super.key,
  }) : assert(projectId != ''),
       assert(projectRevision >= 0),
       assert(checkpointIdentity != '');

  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3VoiceBuildPlanLoader plan;
  final Revision3VoiceBuildLineLocaleAction? onResolveVoiceTarget;
  final Revision3VoiceBuildLineLocaleAction? onManageVoiceTakes;
  final Revision3VoiceBuildReadinessAction? onBuild;
  final bool gameConfigured;
  final Revision3VoiceBuildReadinessCopy copy;

  @override
  State<Revision3VoiceBuildReadinessPanel> createState() =>
      _Revision3VoiceBuildReadinessPanelState();
}

class _Revision3VoiceBuildReadinessPanelState
    extends State<Revision3VoiceBuildReadinessPanel> {
  AuthoringRevision3VoiceBuildPlanResult? _result;
  Object? _error;
  bool _loading = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceBuildReadinessPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.checkpointIdentity != widget.checkpointIdentity) {
      unawaited(_load(clearCurrent: true));
    }
  }

  @override
  void dispose() {
    _loadEpoch++;
    super.dispose();
  }

  Future<void> _load({bool clearCurrent = false}) async {
    final epoch = ++_loadEpoch;
    final expectedProjectId = widget.projectId;
    final expectedProjectRevision = widget.projectRevision;
    final expectedCheckpointIdentity = widget.checkpointIdentity;
    setState(() {
      _loading = true;
      _error = null;
      if (clearCurrent) _result = null;
    });
    try {
      final result = await widget.plan();
      if (!mounted || epoch != _loadEpoch) return;
      if (result.projectId != expectedProjectId ||
          result.projectRevision != expectedProjectRevision ||
          result.basisHead.canonicalJson != expectedCheckpointIdentity) {
        throw const FormatException(
          'Voice readiness does not match the current project checkpoint.',
        );
      }
      setState(() {
        _result = result;
        _loading = false;
      });
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() {
        _result = null;
        _error = error;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final result = _result;
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key('revision3-voice-readiness-panel'),
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
            Row(
              children: [
                Icon(Icons.record_voice_over_outlined, color: scheme.primary),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    widget.copy.title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                IconButton(
                  key: const Key('revision3-voice-readiness-refresh'),
                  tooltip: widget.copy.refreshTooltip,
                  onPressed: _loading ? null : _load,
                  icon: const Icon(Icons.refresh),
                ),
              ],
            ),
            if (_loading) ...[
              const SizedBox(height: 10),
              Semantics(
                liveRegion: true,
                label: widget.copy.checkingSemanticsLabel,
                child: const LinearProgressIndicator(
                  key: Key('revision3-voice-readiness-loading'),
                ),
              ),
            ] else if (_error != null) ...[
              const SizedBox(height: 10),
              _VoiceReadinessLoadError(copy: widget.copy, retry: _load),
            ] else if (result != null) ...[
              const SizedBox(height: 8),
              Revision3VoiceBuildReadinessReport(
                key: ValueKey(
                  'revision3-voice-readiness-${result.basisHead.canonicalJson}',
                ),
                projectRevision: result.projectRevision,
                totalSlots: result.totalSlots,
                readySlots: result.readySlots,
                blockers: result.blockers,
                isReady: result.isReady,
                onResolveVoiceTarget: _guardLineLocaleAction(
                  widget.onResolveVoiceTarget,
                  result,
                ),
                onManageVoiceTakes: _guardLineLocaleAction(
                  widget.onManageVoiceTakes,
                  result,
                ),
                onBuild: _guardAction(widget.onBuild, result),
                gameConfigured: widget.gameConfigured,
                compactBlockers: true,
                showReadyBuildGuidance: true,
                copy: widget.copy,
              ),
            ],
          ],
        ),
      ),
    );
  }

  bool _isCurrentResult(AuthoringRevision3VoiceBuildPlanResult result) =>
      mounted &&
      identical(_result, result) &&
      result.projectId == widget.projectId &&
      result.projectRevision == widget.projectRevision &&
      result.basisHead.canonicalJson == widget.checkpointIdentity;

  Revision3VoiceBuildLineLocaleAction? _guardLineLocaleAction(
    Revision3VoiceBuildLineLocaleAction? action,
    AuthoringRevision3VoiceBuildPlanResult result,
  ) {
    if (action == null) return null;
    return ({required initialLineId, required initialLocale}) async {
      if (!_isCurrentResult(result)) return;
      await Future<void>.sync(
        () =>
            action(initialLineId: initialLineId, initialLocale: initialLocale),
      );
      if (_isCurrentResult(result)) {
        await _load(clearCurrent: true);
      }
    };
  }

  Revision3VoiceBuildReadinessAction? _guardAction(
    Revision3VoiceBuildReadinessAction? action,
    AuthoringRevision3VoiceBuildPlanResult result,
  ) {
    if (action == null) return null;
    return () async {
      if (!_isCurrentResult(result)) return;
      await Future<void>.sync(action);
    };
  }
}

/// Reusable presentation of one already-verified Voice readiness report.
///
/// Stable IDs remain callback-only. Visible content is limited to the friendly
/// dialog-line label, locale, readiness counts, and remediation copy.
class Revision3VoiceBuildReadinessReport extends StatefulWidget {
  Revision3VoiceBuildReadinessReport({
    required this.projectRevision,
    required this.totalSlots,
    required this.readySlots,
    required List<AuthoringRevision3VoiceBuildBlocker> blockers,
    required this.isReady,
    this.onResolveVoiceTarget,
    this.onManageVoiceTakes,
    this.onActionCompleted,
    this.onBuild,
    this.gameConfigured = false,
    this.compactBlockers = false,
    this.showReadyBuildGuidance = false,
    this.blockedBoundary,
    this.copy = const Revision3VoiceBuildReadinessCopy.english(),
    super.key,
  }) : assert(projectRevision >= 0),
       assert(totalSlots >= 0),
       assert(readySlots >= 0 && readySlots <= totalSlots),
       assert(!isReady || (totalSlots > 0 && readySlots == totalSlots)),
       blockers = List.unmodifiable(blockers);

  final int projectRevision;
  final int totalSlots;
  final int readySlots;
  final List<AuthoringRevision3VoiceBuildBlocker> blockers;
  final bool isReady;
  final Revision3VoiceBuildLineLocaleAction? onResolveVoiceTarget;
  final Revision3VoiceBuildLineLocaleAction? onManageVoiceTakes;
  final Revision3VoiceBuildReadinessAction? onActionCompleted;
  final Revision3VoiceBuildReadinessAction? onBuild;
  final bool gameConfigured;
  final bool compactBlockers;
  final bool showReadyBuildGuidance;
  final String? blockedBoundary;
  final Revision3VoiceBuildReadinessCopy copy;

  @override
  State<Revision3VoiceBuildReadinessReport> createState() =>
      _Revision3VoiceBuildReadinessReportState();
}

class _Revision3VoiceBuildReadinessReportState
    extends State<Revision3VoiceBuildReadinessReport> {
  int? _busyBlocker;
  bool _building = false;
  late bool _blockersExpanded;
  String? _actionError;
  int _actionEpoch = 0;

  @override
  void initState() {
    super.initState();
    _blockersExpanded = !widget.compactBlockers;
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceBuildReadinessReport oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectRevision != widget.projectRevision ||
        !listEquals(oldWidget.blockers, widget.blockers) ||
        oldWidget.isReady != widget.isReady) {
      _actionEpoch++;
      _busyBlocker = null;
      _building = false;
      _blockersExpanded = !widget.compactBlockers;
      _actionError = null;
    }
  }

  @override
  void dispose() {
    _actionEpoch++;
    super.dispose();
  }

  Future<void> _runBlockerAction(
    int index,
    AuthoringRevision3VoiceBuildBlocker blocker,
    Revision3VoiceBuildLineLocaleAction action,
  ) async {
    final lineId = blocker.lineId;
    final locale = blocker.locale;
    if (!mounted ||
        _busyBlocker != null ||
        _building ||
        lineId == null ||
        locale == null) {
      return;
    }
    final actionEpoch = ++_actionEpoch;
    setState(() {
      _busyBlocker = index;
      _actionError = null;
    });
    try {
      await Future<void>.sync(
        () => action(initialLineId: lineId, initialLocale: locale),
      );
      if (!mounted || actionEpoch != _actionEpoch) return;
      await Future<void>.sync(() => widget.onActionCompleted?.call());
    } catch (_) {
      if (mounted && actionEpoch == _actionEpoch) {
        setState(() {
          _actionError = widget.copy.workflowOpenFailed;
        });
      }
    } finally {
      if (mounted && actionEpoch == _actionEpoch) {
        setState(() => _busyBlocker = null);
      }
    }
  }

  Future<void> _runBuild() async {
    final action = widget.onBuild;
    if (!mounted ||
        _busyBlocker != null ||
        _building ||
        !widget.isReady ||
        !widget.gameConfigured ||
        action == null) {
      return;
    }
    final actionEpoch = ++_actionEpoch;
    setState(() {
      _building = true;
      _actionError = null;
    });
    try {
      await Future<void>.sync(action);
    } catch (_) {
      if (mounted && actionEpoch == _actionEpoch) {
        setState(() {
          _actionError = widget.copy.buildWorkflowOpenFailed;
        });
      }
    } finally {
      if (mounted && actionEpoch == _actionEpoch) {
        setState(() => _building = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final showBuild =
        widget.isReady && widget.gameConfigured && widget.onBuild != null;
    final blockerList = ListView.separated(
      key: const Key('revision3-voice-readiness-blockers'),
      shrinkWrap: true,
      physics: widget.compactBlockers
          ? null
          : const NeverScrollableScrollPhysics(),
      itemCount: widget.blockers.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final blocker = widget.blockers[index];
        final action = _actionForBlocker(
          blocker,
          resolveTarget: widget.onResolveVoiceTarget,
          manageTakes: widget.onManageVoiceTakes,
          copy: widget.copy,
        );
        return _VoiceReadinessBlockerRow(
          index: index,
          blocker: blocker,
          copy: widget.copy,
          actionLabel: action?.label,
          busy: _busyBlocker == index,
          actionsEnabled: _busyBlocker == null && !_building,
          runAction: action == null
              ? null
              : () => _runBlockerAction(index, blocker, action.callback),
        );
      },
    );
    return Column(
      key: const Key('revision3-voice-readiness-report'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              widget.isReady
                  ? Icons.check_circle_outline
                  : Icons.warning_amber_rounded,
              color: widget.isReady ? scheme.primary : scheme.tertiary,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.isReady
                        ? widget.copy.readyTitle
                        : widget.copy.blockedTitle,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    widget.copy.readyCount(
                      widget.readySlots,
                      widget.totalSlots,
                    ),
                    key: const Key('revision3-voice-readiness-count'),
                  ),
                  if (!widget.isReady) ...[
                    const SizedBox(height: 2),
                    Text(
                      widget.blockedBoundary ?? widget.copy.blockedBoundary,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ],
              ),
            ),
            if (showBuild) ...[
              const SizedBox(width: 10),
              FilledButton.icon(
                key: const Key('revision3-voice-readiness-build'),
                onPressed: _building ? null : _runBuild,
                icon: _building
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.inventory_2_outlined),
                label: Text(widget.copy.buildBundleLabel),
              ),
            ],
          ],
        ),
        if (widget.isReady && widget.showReadyBuildGuidance && !showBuild) ...[
          const SizedBox(height: 8),
          Text(
            widget.gameConfigured
                ? widget.copy.readyBuildReleaseGuidance
                : widget.copy.readyConfigureGameGuidance,
            key: const Key('revision3-voice-readiness-build-guidance'),
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
        if (!widget.isReady && widget.blockers.isNotEmpty) ...[
          const SizedBox(height: 10),
          if (widget.compactBlockers)
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                key: const Key('revision3-voice-readiness-toggle-blockers'),
                onPressed: () =>
                    setState(() => _blockersExpanded = !_blockersExpanded),
                icon: Icon(
                  _blockersExpanded ? Icons.expand_less : Icons.expand_more,
                ),
                label: Text(
                  _blockersExpanded
                      ? widget.copy.hideBlockersLabel
                      : widget.copy.showBlockersLabel(widget.blockers.length),
                ),
              ),
            ),
          if (_blockersExpanded)
            if (widget.compactBlockers)
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 300),
                child: blockerList,
              )
            else
              blockerList,
        ],
        if (_actionError case final error?) ...[
          const SizedBox(height: 8),
          Text(
            error,
            key: const Key('revision3-voice-readiness-action-error'),
            style: TextStyle(color: scheme.error),
          ),
        ],
        const SizedBox(height: 8),
        Text(
          widget.copy.exactProjectRevision(widget.projectRevision),
          style: Theme.of(
            context,
          ).textTheme.labelSmall?.copyWith(color: scheme.onSurfaceVariant),
        ),
      ],
    );
  }
}

class _VoiceReadinessLoadError extends StatelessWidget {
  const _VoiceReadinessLoadError({required this.copy, required this.retry});

  final Revision3VoiceBuildReadinessCopy copy;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) => Row(
    key: const Key('revision3-voice-readiness-error'),
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Icon(Icons.error_outline, color: Theme.of(context).colorScheme.error),
      const SizedBox(width: 10),
      Expanded(child: Text(copy.loadError)),
      const SizedBox(width: 8),
      TextButton(
        key: const Key('revision3-voice-readiness-retry'),
        onPressed: retry,
        child: Text(copy.retryLabel),
      ),
    ],
  );
}

class _VoiceReadinessBlockerRow extends StatelessWidget {
  const _VoiceReadinessBlockerRow({
    required this.index,
    required this.blocker,
    required this.copy,
    required this.actionLabel,
    required this.busy,
    required this.actionsEnabled,
    required this.runAction,
  });

  final int index;
  final AuthoringRevision3VoiceBuildBlocker blocker;
  final Revision3VoiceBuildReadinessCopy copy;
  final String? actionLabel;
  final bool busy;
  final bool actionsEnabled;
  final VoidCallback? runAction;

  @override
  Widget build(BuildContext context) {
    final lineLabel = blocker.lineLabel;
    final locale = blocker.locale;
    return ListTile(
      key: ValueKey(
        'revision3-voice-readiness-blocker-$index-${blocker.reason.name}',
      ),
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.block_outlined),
      title: Text(copy.blockerTitle(blocker.reason)),
      subtitle: lineLabel == null || locale == null
          ? null
          : Text('$lineLabel \u2014 $locale'),
      trailing: actionLabel == null
          ? null
          : TextButton(
              key: ValueKey('revision3-voice-readiness-blocker-action-$index'),
              onPressed: actionsEnabled ? runAction : null,
              child: busy
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : Text(actionLabel!),
            ),
    );
  }
}

({String label, Revision3VoiceBuildLineLocaleAction callback})?
_actionForBlocker(
  AuthoringRevision3VoiceBuildBlocker blocker, {
  required Revision3VoiceBuildLineLocaleAction? resolveTarget,
  required Revision3VoiceBuildLineLocaleAction? manageTakes,
  required Revision3VoiceBuildReadinessCopy copy,
}) {
  if (blocker.lineId == null || blocker.locale == null) return null;
  return switch (blocker.reason) {
    AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget ||
    AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget ||
    AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd
        when resolveTarget != null =>
      (label: copy.resolveTargetLabel, callback: resolveTarget),
    AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake ||
    AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved ||
    AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified
        when manageTakes != null =>
      (label: copy.manageTakesLabel, callback: manageTakes),
    _ => null,
  };
}

String _englishVoiceReadyCount(int readySlots, int totalSlots) =>
    '$readySlots of $totalSlots Voice slots are ready.';

String _englishVoiceBlockerCount(int count) =>
    'Show $count ${count == 1 ? 'blocker' : 'blockers'}';

String _englishVoiceProjectRevision(int revision) =>
    'Exact project revision $revision';

String _englishVoiceBlockerTitle(
  AuthoringRevision3VoiceBuildBlockReason reason,
) => switch (reason) {
  AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots =>
    'No Voice setups exist in this project.',
  AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded =>
    'The selected Voice recordings exceed the safe bundle memory budget.',
  AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget =>
    'Resolve this Voice target.',
  AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget =>
    'This Voice target is ambiguous.',
  AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd =>
    'This target is not a sealed existing-member replacement.',
  AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake =>
    'Select an approved Voice take.',
  AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved =>
    'The selected Voice take is not approved.',
  AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified =>
    'The selected Voice take uses an unsupported codec.',
  AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded =>
    'This project exceeds the 1024-slot Voice bundle limit.',
};
