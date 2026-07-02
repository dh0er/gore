import 'package:flutter/material.dart';

import 'keep_alive_tab.dart';

/// Calls [onTabReentered] when the enclosing [DefaultTabController] settles
/// on a tab the user has visited before in this controller's lifetime.
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
class TabReentryListener extends StatefulWidget {
  const TabReentryListener({
    super.key,
    required this.onTabReentered,
    required this.child,
  });

  /// Called with the settled tab index, only for tabs already visited.
  final ValueChanged<int> onTabReentered;

  final Widget child;

  @override
  State<TabReentryListener> createState() => _TabReentryListenerState();
}

class _TabReentryListenerState extends State<TabReentryListener> {
  TabController? _controller;
  final Set<int> _visited = {};

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final controller = DefaultTabController.of(context);
    if (identical(controller, _controller)) return;
    _controller?.removeListener(_onControllerChanged);
    _controller = controller;
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
    // Fire once per switch: taps notify while animating (indexIsChanging
    // true) and again on settle; only the settled notification counts.
    if (controller == null || controller.indexIsChanging) return;
    final index = controller.index;
    if (!_visited.add(index)) {
      widget.onTabReentered(index);
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
