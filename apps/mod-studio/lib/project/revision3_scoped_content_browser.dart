import 'dart:async';

import 'package:flutter/material.dart';

/// Honest source scopes exposed by the unified managed-project content host.
enum Revision3ScopedContentScope { thisMod, baseGame, installed, allSources }

/// Programmatic, project-bound handoff into the managed Content browser.
///
/// [openSearchAll] only changes presentation scope. It neither starts a
/// content search nor owns or reuses any source result. One request may be
/// buffered before the matching browser mounts; callers that need that lazy
/// handoff should construct the controller with [projectIdentity]. The first
/// attachment otherwise binds an unbound controller permanently to that
/// project's identity. Project switches, detach, disposal, and late scheduled
/// callbacks resolve outstanding requests to `false`; a controller whose
/// mounted browser changes projects remains invalid for later requests.
final class Revision3ScopedContentBrowserController {
  Revision3ScopedContentBrowserController({Object? projectIdentity})
    : this._(projectIdentity);

  Revision3ScopedContentBrowserController._(this._projectIdentity);

  Object? _attachment;
  Object? _projectIdentity;
  Future<bool> Function()? _openSearchAll;
  VoidCallback? _cancelScheduledActivations;
  _PendingScopedContentActivation? _bufferedActivation;
  final Set<_PendingScopedContentActivation> _forwardedActivations = {};
  bool _projectInvalidated = false;
  bool _disposed = false;

  /// The configured identity or the identity adopted on first attachment.
  Object? get projectIdentity => _projectIdentity;

  /// Opens Search all and, after it mounts, invokes the browser's optional
  /// focus handoff.
  ///
  /// The result is `true` only if the exact bound browser remained alive on
  /// the same project through that post-frame handoff.
  Future<bool> openSearchAll() {
    final activation = _PendingScopedContentActivation();
    if (_disposed || _projectInvalidated) {
      activation.result.complete(false);
      return activation.result.future;
    }
    if (_attachment != null) {
      _cancelScheduledActivations?.call();
      _cancelForwardedActivations();
      return _forward(activation);
    }
    final superseded = _bufferedActivation;
    _bufferedActivation = activation;
    if (superseded != null && !superseded.result.isCompleted) {
      superseded.result.complete(false);
    }
    return activation.result.future;
  }

  /// Permanently releases the controller. It cannot attach again.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _cancelScheduledActivations?.call();
    _attachment = null;
    _openSearchAll = null;
    _cancelScheduledActivations = null;
    _cancelActivations();
  }

  Future<bool> _forward(_PendingScopedContentActivation activation) {
    final open = _openSearchAll;
    final attachment = _attachment;
    final projectIdentity = _projectIdentity;
    if (open == null || attachment == null || projectIdentity == null) {
      activation.result.complete(false);
      return activation.result.future;
    }
    activation
      ..attachment = attachment
      ..projectIdentity = projectIdentity;
    _forwardedActivations.add(activation);
    Future<bool>.sync(open).then(
      (resolved) => _completeForwarded(activation, resolved),
      onError: (_, _) => _completeForwarded(activation, false),
    );
    return activation.result.future;
  }

  void _completeForwarded(
    _PendingScopedContentActivation activation,
    bool resolved,
  ) {
    _forwardedActivations.remove(activation);
    if (!activation.result.isCompleted) {
      activation.result.complete(resolved && _matches(activation));
    }
  }

  bool _matches(_PendingScopedContentActivation activation) =>
      identical(_attachment, activation.attachment) &&
      _projectIdentity == activation.projectIdentity;

  bool _attach(
    Object attachment, {
    required Object projectIdentity,
    required Future<bool> Function() openSearchAll,
    required VoidCallback cancelScheduledActivations,
  }) {
    if (_disposed ||
        _projectInvalidated ||
        (_projectIdentity != null && _projectIdentity != projectIdentity) ||
        (_attachment != null && !identical(_attachment, attachment))) {
      final buffered = _bufferedActivation;
      _bufferedActivation = null;
      if (buffered != null && !buffered.result.isCompleted) {
        buffered.result.complete(false);
      }
      return false;
    }
    assert(
      _attachment == null || identical(_attachment, attachment),
      'A Revision3ScopedContentBrowserController can only be attached to one '
      'scoped content browser at a time.',
    );
    _projectIdentity ??= projectIdentity;
    _attachment = attachment;
    _openSearchAll = openSearchAll;
    _cancelScheduledActivations = cancelScheduledActivations;
    final buffered = _bufferedActivation;
    _bufferedActivation = null;
    if (buffered != null) _forward(buffered);
    return true;
  }

  void _detach(Object attachment) {
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    _openSearchAll = null;
    _cancelScheduledActivations = null;
    _cancelForwardedActivations();
  }

  void _projectChanged(Object attachment, Object projectIdentity) {
    if (!identical(_attachment, attachment) ||
        _projectIdentity == projectIdentity) {
      return;
    }
    _cancelScheduledActivations?.call();
    _detach(attachment);
    _projectInvalidated = true;
    _cancelActivations();
  }

  void _cancelForwardedActivations() {
    final forwarded = _forwardedActivations.toList(growable: false);
    _forwardedActivations.clear();
    for (final activation in forwarded) {
      if (!activation.result.isCompleted) activation.result.complete(false);
    }
  }

  void _cancelActivations() {
    final buffered = _bufferedActivation;
    _bufferedActivation = null;
    if (buffered != null && !buffered.result.isCompleted) {
      buffered.result.complete(false);
    }
    _cancelForwardedActivations();
  }
}

final class _PendingScopedContentActivation {
  Object? attachment;
  Object? projectIdentity;
  final Completer<bool> result = Completer<bool>();
}

/// Presentation-only source host for the managed-project Content Library.
///
/// Equality of [projectIdentity] defines the hosted project lifetime. Rebuilds
/// for the same project retain the selected scope and every visited page. A
/// different identity returns to [initialScope] and discards pages mounted for
/// the previous project.
class Revision3ScopedContentBrowser extends StatefulWidget {
  static const searchAllSecondaryRoute = 'search-all';

  const Revision3ScopedContentBrowser({
    required this.projectIdentity,
    required this.thisModLabel,
    required this.baseGameLabel,
    required this.installedLabel,
    required this.allSourcesLabel,
    required this.thisMod,
    required this.baseGame,
    required this.installed,
    required this.allSources,
    this.controller,
    this.onAllSourcesActivated,
    this.initialScope = Revision3ScopedContentScope.thisMod,
    super.key,
  });

  final Object projectIdentity;
  final String thisModLabel;
  final String baseGameLabel;
  final String installedLabel;
  final String allSourcesLabel;
  final Widget thisMod;
  final Widget baseGame;
  final Widget installed;
  final Widget allSources;

  /// Optional programmatic Search-all handoff. Omitting it preserves the
  /// original descendant-only navigation API.
  final Revision3ScopedContentBrowserController? controller;

  /// Invoked only for a successful [Revision3ScopedContentBrowserController]
  /// Search-all request, after that page has mounted. Hosts use this to focus
  /// the existing query field without granting this browser search or cache
  /// authority.
  final VoidCallback? onAllSourcesActivated;

  /// Initial presentation scope for a newly mounted project browser.
  ///
  /// This lets an exact workspace deep link mount Search all directly without
  /// briefly mounting (and loading) This mod first. Later scope changes remain
  /// owned by this browser or its bound controller.
  final Revision3ScopedContentScope initialScope;

  /// Switches the nearest scoped browser to one of its already-authorized
  /// presentation sources. Result identities still have to be reopened by the
  /// destination page; changing the visible scope grants no content authority.
  static void navigate(
    BuildContext context,
    Revision3ScopedContentScope scope,
  ) {
    final state = context
        .findAncestorStateOfType<_Revision3ScopedContentBrowserState>();
    if (state == null) {
      throw FlutterError(
        'Revision3ScopedContentBrowser.navigate requires a descendant context.',
      );
    }
    state._select(scope);
  }

  @override
  State<Revision3ScopedContentBrowser> createState() =>
      _Revision3ScopedContentBrowserState();
}

class _Revision3ScopedContentBrowserState
    extends State<Revision3ScopedContentBrowser> {
  late Revision3ScopedContentScope _selected;
  late final Set<Revision3ScopedContentScope> _mounted;
  int _projectEpoch = 0;
  int _activationEpoch = 0;

  @override
  void initState() {
    super.initState();
    _selected = widget.initialScope;
    _mounted = {widget.initialScope};
    _attachController(widget.controller);
  }

  @override
  void didUpdateWidget(covariant Revision3ScopedContentBrowser oldWidget) {
    super.didUpdateWidget(oldWidget);
    final controllerChanged = !identical(
      oldWidget.controller,
      widget.controller,
    );
    final projectChanged = oldWidget.projectIdentity != widget.projectIdentity;
    final activationChanged = !identical(
      oldWidget.onAllSourcesActivated,
      widget.onAllSourcesActivated,
    );
    if (controllerChanged || activationChanged || projectChanged) {
      _invalidateScheduledActivations();
    }
    if (controllerChanged) oldWidget.controller?._detach(this);
    if (projectChanged) {
      if (!controllerChanged) {
        widget.controller?._projectChanged(this, widget.projectIdentity);
      }
      _projectEpoch++;
      _selected = widget.initialScope;
      _mounted
        ..clear()
        ..add(widget.initialScope);
    }
    if (controllerChanged) _attachController(widget.controller);
  }

  @override
  void dispose() {
    _invalidateScheduledActivations();
    widget.controller?._detach(this);
    super.dispose();
  }

  void _attachController(Revision3ScopedContentBrowserController? controller) {
    controller?._attach(
      this,
      projectIdentity: widget.projectIdentity,
      openSearchAll: _openSearchAll,
      cancelScheduledActivations: _invalidateScheduledActivations,
    );
  }

  void _invalidateScheduledActivations() => _activationEpoch++;

  Future<bool> _openSearchAll() async {
    if (!mounted) return false;
    final activationEpoch = _activationEpoch;
    final projectEpoch = _projectEpoch;
    final projectIdentity = widget.projectIdentity;
    final activation = widget.onAllSourcesActivated;
    _select(Revision3ScopedContentScope.allSources);
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        activationEpoch != _activationEpoch ||
        projectEpoch != _projectEpoch ||
        projectIdentity != widget.projectIdentity ||
        !identical(activation, widget.onAllSourcesActivated) ||
        _selected != Revision3ScopedContentScope.allSources ||
        !_mounted.contains(Revision3ScopedContentScope.allSources)) {
      return false;
    }
    try {
      activation?.call();
    } catch (_) {
      return false;
    }
    return mounted &&
        activationEpoch == _activationEpoch &&
        projectEpoch == _projectEpoch &&
        projectIdentity == widget.projectIdentity &&
        _selected == Revision3ScopedContentScope.allSources;
  }

  void _select(Revision3ScopedContentScope scope) {
    if (scope == _selected) return;
    setState(() {
      _selected = scope;
      _mounted.add(scope);
    });
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-scoped-content-browser'),
    container: true,
    explicitChildNodes: true,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Material(
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: SingleChildScrollView(
            key: const Key(
              'revision3-scoped-content-browser-navigation-scroll',
            ),
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
            child: SegmentedButton<Revision3ScopedContentScope>(
              key: const Key('revision3-scoped-content-browser-navigation'),
              showSelectedIcon: false,
              segments: [
                ButtonSegment(
                  value: Revision3ScopedContentScope.thisMod,
                  label: Text(
                    widget.thisModLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-this-mod',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3ScopedContentScope.baseGame,
                  label: Text(
                    widget.baseGameLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-base-game',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3ScopedContentScope.installed,
                  label: Text(
                    widget.installedLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-installed',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3ScopedContentScope.allSources,
                  icon: const Icon(Icons.manage_search_outlined),
                  label: Text(
                    widget.allSourcesLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-all-sources',
                    ),
                  ),
                ),
              ],
              selected: {_selected},
              onSelectionChanged: (selection) => _select(selection.single),
            ),
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: IndexedStack(
            key: const Key('revision3-scoped-content-browser-pages'),
            index: _selected.index,
            sizing: StackFit.expand,
            children: [
              _buildPage(Revision3ScopedContentScope.thisMod, widget.thisMod),
              _buildPage(Revision3ScopedContentScope.baseGame, widget.baseGame),
              _buildPage(
                Revision3ScopedContentScope.installed,
                widget.installed,
              ),
              _buildPage(
                Revision3ScopedContentScope.allSources,
                widget.allSources,
              ),
            ],
          ),
        ),
      ],
    ),
  );

  Widget _buildPage(Revision3ScopedContentScope scope, Widget child) =>
      _mounted.contains(scope)
      ? KeyedSubtree(
          key: ValueKey((_projectEpoch, scope)),
          child: Semantics(
            key: Key(
              'revision3-scoped-content-browser-page-${_scopeKey(scope)}',
            ),
            container: true,
            explicitChildNodes: true,
            child: child,
          ),
        )
      : const SizedBox.shrink();
}

String _scopeKey(Revision3ScopedContentScope scope) => switch (scope) {
  Revision3ScopedContentScope.thisMod => 'this-mod',
  Revision3ScopedContentScope.baseGame => 'base-game',
  Revision3ScopedContentScope.installed => 'installed',
  Revision3ScopedContentScope.allSources => 'all-sources',
};
