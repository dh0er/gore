import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../domain/ui_settings.dart';
import 'keep_alive_tab.dart';

/// Recreates its subtree whenever the configured game executable path changes.
///
/// Companion to [KeepAliveTab] for tabs whose UI state (selected sample or
/// asset, preview, staging tree) is bound to the configured game install:
/// keep-alive preserves that state across tab switches, but after the game
/// path changes in Settings the kept selection would point at the previous
/// install while the data providers reload for the new one — Preview /
/// Export / Replace could then act on a stale selection against the new game
/// path. Keying the subtree by the path drops that state exactly when the
/// install switches; plain tab switching is unaffected.
class GamePathScope extends ConsumerWidget {
  const GamePathScope({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final path = ref.watch(gameExePathProvider);
    // The KeyedSubtree has no siblings (it is this widget's only child), so
    // the same value key in several GamePathScopes cannot collide.
    return KeyedSubtree(key: ValueKey(path ?? ''), child: child);
  }
}
