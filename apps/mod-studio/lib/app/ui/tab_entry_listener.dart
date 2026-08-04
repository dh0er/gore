import 'package:flutter/material.dart';

import 'keep_alive_tab.dart';

/// Calls [onTabEntered] whenever the observed [TabController] settles on a
/// different tab — first entries included.
///
/// Companion to [KeepAliveTab]: keep-alive keeps a tab's widget subtree (and
/// therefore its `autoDispose` providers) mounted after the user leaves, so
/// data that previously refreshed on every tab entry — because leaving the
/// tab disposed the provider — would go stale. The caller restores those
/// freshness semantics by invalidating the relevant providers on entry,
/// while the tab's UI state (search text, expansion, selection, scroll)
/// survives untouched.
///
/// Whether a given entry actually needs an invalidate is the CALLER's
/// decision (see `AssetEntryTracker`), not this widget's: a tab's first
/// entry is not necessarily a fresh provider build, because another surface
/// (the Changes tab embeds the same Scripts view) may already be
/// keeping the shared provider alive — with a value from before a deploy,
/// undeploy, or game patch. Only the tab the controller is on at attach
/// never produces a callback by itself: there is no tab CHANGE to react to,
/// and its providers are created by the initial build.
///
/// The callback fires exactly once per settled tab change. The controller
/// notifies more often than that — during a tap's animation
/// (`indexIsChanging` true) and repeatedly with `indexIsChanging` false
/// during swipe gestures — so both the animation phase and repeat
/// notifications for an already-settled index are filtered out.
class TabEntryListener extends StatefulWidget {
  const TabEntryListener({
    super.key,
    this.controller,
    required this.onTabEntered,
    required this.child,
  });

  /// The tab controller to observe. Defaults to [DefaultTabController.of].
  final TabController? controller;

  /// Called with the settled tab index, once per actual tab change.
  final ValueChanged<int> onTabEntered;

  final Widget child;

  @override
  State<TabEntryListener> createState() => _TabEntryListenerState();
}

class _TabEntryListenerState extends State<TabEntryListener> {
  TabController? _controller;
  int _lastSettledIndex = 0;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _updateController();
  }

  @override
  void didUpdateWidget(TabEntryListener oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.controller != oldWidget.controller) _updateController();
  }

  void _updateController() {
    final controller = widget.controller ?? DefaultTabController.of(context);
    if (identical(controller, _controller)) return;
    _controller?.removeListener(_onControllerChanged);
    _controller = controller;
    _lastSettledIndex = controller.index;
    controller.addListener(_onControllerChanged);
  }

  @override
  void dispose() {
    _controller?.removeListener(_onControllerChanged);
    super.dispose();
  }

  void _onControllerChanged() {
    final controller = _controller;
    if (controller == null || controller.indexIsChanging) return;
    final index = controller.index;
    // Swipes (and other paths) notify repeatedly with indexIsChanging false
    // for the same settled index; without this guard one gesture would
    // re-fire the callback several times.
    if (index == _lastSettledIndex) return;
    _lastSettledIndex = index;
    widget.onTabEntered(index);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
