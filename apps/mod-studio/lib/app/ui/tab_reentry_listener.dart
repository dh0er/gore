import 'package:flutter/material.dart';

import 'keep_alive_tab.dart';

/// Calls [onTabReentered] when the observed [TabController] settles on a tab
/// the user has visited before in this controller's lifetime.
///
/// Companion to [KeepAliveTab]: keep-alive keeps a tab's widget subtree (and
/// therefore its `autoDispose` providers) mounted after the user leaves, so
/// data that previously refreshed on every tab entry — because leaving the
/// tab disposed the provider — would go stale. The caller restores those
/// freshness semantics by invalidating the relevant providers on re-entry,
/// while the tab's UI state (search text, expansion, selection, scroll)
/// survives untouched.
///
/// First entry is deliberately excluded: the tab's providers are freshly
/// created by that build anyway, and invalidating would double-fetch.
///
/// The callback fires exactly once per settled tab change. The controller
/// notifies more often than that — during a tap's animation
/// (`indexIsChanging` true) and repeatedly with `indexIsChanging` false
/// during swipe gestures — so both the animation phase and repeat
/// notifications for an already-settled index are filtered out.
class TabReentryListener extends StatefulWidget {
  const TabReentryListener({
    super.key,
    this.controller,
    required this.onTabReentered,
    required this.child,
  });

  /// The tab controller to observe. Defaults to [DefaultTabController.of].
  final TabController? controller;

  /// Called with the settled tab index, only for tabs already visited, once
  /// per actual tab change.
  final ValueChanged<int> onTabReentered;

  final Widget child;

  @override
  State<TabReentryListener> createState() => _TabReentryListenerState();
}

class _TabReentryListenerState extends State<TabReentryListener> {
  TabController? _controller;
  final Set<int> _visited = {};
  int _lastSettledIndex = 0;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _updateController();
  }

  @override
  void didUpdateWidget(TabReentryListener oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.controller != oldWidget.controller) _updateController();
  }

  void _updateController() {
    final controller = widget.controller ?? DefaultTabController.of(context);
    if (identical(controller, _controller)) return;
    _controller?.removeListener(_onControllerChanged);
    _controller = controller;
    _lastSettledIndex = controller.index;
    _visited
      ..clear()
      ..add(controller.index);
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
    // re-fire the callback several times (including on a tab's first entry,
    // right after the visited-set add).
    if (index == _lastSettledIndex) return;
    _lastSettledIndex = index;
    if (!_visited.add(index)) {
      widget.onTabReentered(index);
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
