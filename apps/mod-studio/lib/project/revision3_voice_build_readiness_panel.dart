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

/// Exact managed-project identity observed by one mounted Voice plan panel.
@immutable
final class Revision3VoiceBuildReadinessCheckpoint {
  const Revision3VoiceBuildReadinessCheckpoint({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
  }) : assert(projectRoot != ''),
       assert(projectId != ''),
       assert(projectRevision >= 0),
       assert(checkpointIdentity != '');

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;

  @override
  bool operator ==(Object other) =>
      other is Revision3VoiceBuildReadinessCheckpoint &&
      other.projectRoot == projectRoot &&
      other.projectId == projectId &&
      other.projectRevision == projectRevision &&
      other.checkpointIdentity == checkpointIdentity;

  @override
  int get hashCode =>
      Object.hash(projectRoot, projectId, projectRevision, checkpointIdentity);
}

/// Publication state of the read-only Voice bundle plan observation.
enum Revision3VoiceBuildReadinessLoadState {
  detached,
  loading,
  ready,
  unavailable,
}

/// Narrow status output from the exact plan already loaded by the panel.
///
/// It deliberately exposes no blocker targets, build action, game
/// configuration, output, deployment, playback, or runtime evidence.
@immutable
final class Revision3VoiceBuildReadinessSnapshot {
  const Revision3VoiceBuildReadinessSnapshot._({
    required this.state,
    this.checkpoint,
    this.planOutcome,
  }) : assert(
         state == Revision3VoiceBuildReadinessLoadState.detached
             ? checkpoint == null && planOutcome == null
             : checkpoint != null,
       ),
       assert(
         state == Revision3VoiceBuildReadinessLoadState.ready
             ? planOutcome != null
             : planOutcome == null,
       );

  const Revision3VoiceBuildReadinessSnapshot._detached()
    : this._(state: Revision3VoiceBuildReadinessLoadState.detached);

  factory Revision3VoiceBuildReadinessSnapshot._loading(
    Revision3VoiceBuildReadinessCheckpoint checkpoint,
  ) => Revision3VoiceBuildReadinessSnapshot._(
    state: Revision3VoiceBuildReadinessLoadState.loading,
    checkpoint: checkpoint,
  );

  factory Revision3VoiceBuildReadinessSnapshot._unavailable(
    Revision3VoiceBuildReadinessCheckpoint checkpoint,
  ) => Revision3VoiceBuildReadinessSnapshot._(
    state: Revision3VoiceBuildReadinessLoadState.unavailable,
    checkpoint: checkpoint,
  );

  factory Revision3VoiceBuildReadinessSnapshot._ready(
    Revision3VoiceBuildReadinessCheckpoint checkpoint,
    AuthoringRevision3VoiceBuildPlanResult result,
  ) => Revision3VoiceBuildReadinessSnapshot._(
    state: Revision3VoiceBuildReadinessLoadState.ready,
    checkpoint: checkpoint,
    planOutcome: result.outcome,
  );

  final Revision3VoiceBuildReadinessLoadState state;
  final Revision3VoiceBuildReadinessCheckpoint? checkpoint;
  final AuthoringRevision3VoiceBuildPlanOutcome? planOutcome;

  bool belongsTo(Revision3VoiceBuildReadinessCheckpoint expected) =>
      checkpoint == expected;
}

/// Observes one mounted Voice readiness panel without owning or rerunning its
/// native planner.
///
/// Attachment identity prevents a replaced or disposed panel from publishing
/// stale evidence into the current Test & Release row.
final class Revision3VoiceBuildReadinessController extends ChangeNotifier {
  Object? _attachment;
  Revision3VoiceBuildReadinessSnapshot _snapshot =
      const Revision3VoiceBuildReadinessSnapshot._detached();
  int _attachmentGeneration = 0;
  bool _disposed = false;

  Revision3VoiceBuildReadinessSnapshot get snapshot => _snapshot;

  void _attach(Object attachment) {
    if (_disposed) {
      throw StateError('A disposed Voice readiness controller cannot attach.');
    }
    _attachment = attachment;
    _attachmentGeneration++;
    _snapshot = const Revision3VoiceBuildReadinessSnapshot._detached();
  }

  void _publish(
    Object attachment,
    Revision3VoiceBuildReadinessSnapshot snapshot, {
    required bool notify,
  }) {
    if (_disposed || !identical(_attachment, attachment)) return;
    _snapshot = snapshot;
    if (notify) {
      notifyListeners();
    } else {
      _notifyAfterBuild(_attachmentGeneration);
    }
  }

  void _detach(Object attachment) {
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    final generation = ++_attachmentGeneration;
    _snapshot = const Revision3VoiceBuildReadinessSnapshot._detached();
    _notifyAfterBuild(generation);
  }

  void _notifyAfterBuild(int generation) {
    scheduleMicrotask(() {
      if (_disposed || generation != _attachmentGeneration) return;
      notifyListeners();
    });
  }

  @override
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _attachment = null;
    _attachmentGeneration++;
    _snapshot = const Revision3VoiceBuildReadinessSnapshot._detached();
    super.dispose();
  }
}

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
    : title = 'Voice bundle check',
      refreshTooltip = 'Refresh Voice bundle check',
      checkingSemanticsLabel = 'Checking the exact Voice bundle plan',
      loadError =
          'The exact Voice bundle plan could not be verified for the current project. No bundle-plan evidence is available from this result.',
      retryLabel = 'Retry',
      readyTitle = 'Voice bundle plan checked',
      blockedTitle = 'Voice bundle plan needs attention',
      readyCount = _englishVoiceReadyCount,
      blockedBoundary =
          'No bundle was created and deployment was not performed.',
      buildBundleLabel = 'Build bundle',
      readyBuildReleaseGuidance =
          'This checks only the plan; creating the offline Voice bundle remains a separate action.',
      readyConfigureGameGuidance =
          'The exact Voice bundle plan is checked. A configured game installation is still required before the separate offline bundle action is available.',
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
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.checkpointIdentity,
    required this.plan,
    this.onResolveVoiceTarget,
    this.onManageVoiceTakes,
    this.onBuild,
    this.gameConfigured = false,
    this.requiresReopen = false,
    this.copy = const Revision3VoiceBuildReadinessCopy.english(),
    this.controller,
    super.key,
  }) : assert(projectRoot != ''),
       assert(projectId != ''),
       assert(projectRevision >= 0),
       assert(checkpointIdentity != '');

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String checkpointIdentity;
  final Revision3VoiceBuildPlanLoader plan;
  final Revision3VoiceBuildLineLocaleAction? onResolveVoiceTarget;
  final Revision3VoiceBuildLineLocaleAction? onManageVoiceTakes;
  final Revision3VoiceBuildReadinessAction? onBuild;
  final bool gameConfigured;
  final bool requiresReopen;
  final Revision3VoiceBuildReadinessCopy copy;
  final Revision3VoiceBuildReadinessController? controller;

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

  Revision3VoiceBuildReadinessCheckpoint get _checkpoint =>
      Revision3VoiceBuildReadinessCheckpoint(
        projectRoot: widget.projectRoot,
        projectId: widget.projectId,
        projectRevision: widget.projectRevision,
        checkpointIdentity: widget.checkpointIdentity,
      );

  @override
  void initState() {
    super.initState();
    widget.controller?._attach(this);
    if (widget.requiresReopen) {
      _markUnavailable(notify: false, updateState: false);
    } else {
      unawaited(_load());
    }
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceBuildReadinessPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final controllerChanged = oldWidget.controller != widget.controller;
    if (controllerChanged) {
      oldWidget.controller?._detach(this);
      widget.controller?._attach(this);
    }
    final oldCheckpoint = Revision3VoiceBuildReadinessCheckpoint(
      projectRoot: oldWidget.projectRoot,
      projectId: oldWidget.projectId,
      projectRevision: oldWidget.projectRevision,
      checkpointIdentity: oldWidget.checkpointIdentity,
    );
    final checkpointChanged = oldCheckpoint != _checkpoint;
    if (widget.requiresReopen) {
      if (checkpointChanged || !oldWidget.requiresReopen || controllerChanged) {
        _markUnavailable(notify: false, updateState: false);
      }
    } else if (checkpointChanged || oldWidget.requiresReopen) {
      unawaited(_load());
    } else if (controllerChanged) {
      _publishCurrentSnapshot(notify: false);
    }
  }

  @override
  void dispose() {
    _loadEpoch++;
    widget.controller?._detach(this);
    super.dispose();
  }

  Future<void> _load() async {
    if (widget.requiresReopen) {
      _markUnavailable(notify: true, updateState: true);
      return;
    }
    final epoch = ++_loadEpoch;
    final checkpoint = _checkpoint;
    final plan = widget.plan;
    setState(() {
      _loading = true;
      _error = null;
      _result = null;
    });
    _publishSnapshot(
      Revision3VoiceBuildReadinessSnapshot._loading(checkpoint),
      notify: false,
    );
    try {
      final result = await plan();
      if (!mounted || epoch != _loadEpoch || checkpoint != _checkpoint) return;
      if (result.projectId != checkpoint.projectId ||
          result.projectRevision != checkpoint.projectRevision ||
          result.basisHead.canonicalJson != checkpoint.checkpointIdentity) {
        throw const FormatException(
          'Voice readiness does not match the current project checkpoint.',
        );
      }
      setState(() {
        _result = result;
        _loading = false;
      });
      _publishSnapshot(
        Revision3VoiceBuildReadinessSnapshot._ready(checkpoint, result),
        notify: true,
      );
    } catch (error) {
      if (!mounted || epoch != _loadEpoch || checkpoint != _checkpoint) return;
      setState(() {
        _result = null;
        _error = error;
        _loading = false;
      });
      _publishSnapshot(
        Revision3VoiceBuildReadinessSnapshot._unavailable(checkpoint),
        notify: true,
      );
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
      !widget.requiresReopen &&
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
        await _load();
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

  void _publishCurrentSnapshot({required bool notify}) {
    final result = _result;
    final snapshot = _loading
        ? Revision3VoiceBuildReadinessSnapshot._loading(_checkpoint)
        : _error != null || result == null || !_isCurrentResult(result)
        ? Revision3VoiceBuildReadinessSnapshot._unavailable(_checkpoint)
        : Revision3VoiceBuildReadinessSnapshot._ready(_checkpoint, result);
    _publishSnapshot(snapshot, notify: notify);
  }

  void _publishSnapshot(
    Revision3VoiceBuildReadinessSnapshot snapshot, {
    required bool notify,
  }) => widget.controller?._publish(this, snapshot, notify: notify);

  void _markUnavailable({required bool notify, required bool updateState}) {
    _loadEpoch++;

    void mark() {
      _result = null;
      _loading = false;
      _error = const FormatException(
        'Voice readiness is unavailable until the project is reopened.',
      );
    }

    if (updateState) {
      setState(mark);
    } else {
      mark();
    }
    _publishSnapshot(
      Revision3VoiceBuildReadinessSnapshot._unavailable(_checkpoint),
      notify: notify,
    );
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
    '$readySlots of $totalSlots existing Voice slots pass this bundle plan.';

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
