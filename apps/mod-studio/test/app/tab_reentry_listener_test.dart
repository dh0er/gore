import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/keep_alive_tab.dart';
import 'package:gore_mod/app/ui/tab_reentry_listener.dart';

/// Exposes [ChangeNotifier.notifyListeners] so tests can simulate the
/// repeated same-settled-index notifications a TabBarView swipe gesture
/// produces (indexIsChanging stays false throughout).
class _RefiringTabController extends TabController {
  _RefiringTabController({required super.length, required super.vsync});

  void refire() => notifyListeners();
}

void main() {
  testWidgets(
    'tab re-entry invalidates provider while KeepAliveTab preserves UI state',
    (tester) async {
      var buildCount = 0;
      final countingProvider = Provider.autoDispose<int>((ref) => ++buildCount);

      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: DefaultTabController(
              length: 2,
              child: Consumer(
                builder: (context, ref, _) => TabReentryListener(
                  onTabReentered: (index) {
                    if (index == 1) ref.invalidate(countingProvider);
                  },
                  child: Scaffold(
                    appBar: const TabBar(
                      tabs: [
                        Tab(text: 'A'),
                        Tab(text: 'B'),
                      ],
                    ),
                    body: TabBarView(
                      children: [
                        const KeepAliveTab(child: Center(child: Text('Tab A'))),
                        KeepAliveTab(
                          child: Column(
                            children: [
                              const TextField(key: Key('field-b')),
                              Consumer(
                                builder: (context, ref, _) => Text(
                                  'build ${ref.watch(countingProvider)}',
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );

      // Provider tab not visited yet: never built.
      expect(buildCount, 0);

      // First entry into tab B: provider builds once, NOT invalidated on top
      // (that would double-fetch).
      await tester.tap(find.text('B'));
      await tester.pumpAndSettle();
      expect(find.text('build 1'), findsOneWidget);
      expect(buildCount, 1);

      // Put UI state into tab B.
      await tester.enterText(find.byKey(const Key('field-b')), 'kept text');
      await tester.pump();

      // Leave for tab A: keep-alive keeps B mounted, so the autoDispose
      // provider stays alive and does not rebuild.
      await tester.tap(find.text('A'));
      await tester.pumpAndSettle();
      expect(buildCount, 1);

      // Re-enter tab B: provider is invalidated and re-evaluates (fresh data),
      // while the TextField's text survives (kept UI state).
      await tester.tap(find.text('B'));
      await tester.pumpAndSettle();
      expect(buildCount, 2);
      expect(find.text('build 2'), findsOneWidget);
      expect(find.text('kept text'), findsOneWidget);
    },
  );

  testWidgets(
    'callback fires exactly once per settled tab change despite repeated '
    'notifications on the same index',
    (tester) async {
      final controller = _RefiringTabController(
        length: 2,
        vsync: const TestVSync(),
      );
      addTearDown(controller.dispose);
      final log = <int>[];

      await tester.pumpWidget(
        MaterialApp(
          home: TabReentryListener(
            controller: controller,
            onTabReentered: log.add,
            child: Scaffold(
              appBar: TabBar(
                controller: controller,
                tabs: const [
                  Tab(text: 'A'),
                  Tab(text: 'B'),
                ],
              ),
              body: TabBarView(
                controller: controller,
                children: const [
                  KeepAliveTab(child: Center(child: Text('Tab A'))),
                  KeepAliveTab(child: Center(child: Text('Tab B'))),
                ],
              ),
            ),
          ),
        ),
      );

      // Repeat notifications on the initial settled tab: no callback.
      controller.refire();
      controller.refire();
      expect(log, isEmpty);

      // First arrival at tab B: no callback (first entry is excluded)...
      controller.index = 1;
      await tester.pumpAndSettle();
      expect(log, isEmpty);

      // ...and swipe-style repeat notifications right after the arrival must
      // not turn the first entry into a phantom re-entry.
      controller.refire();
      controller.refire();
      expect(log, isEmpty);

      // Back to A (visited): exactly one callback per actual change, however
      // many notifications the gesture produces on the settled index.
      controller.index = 0;
      await tester.pumpAndSettle();
      controller.refire();
      controller.refire();
      controller.refire();
      expect(log, [0]);

      // And once more to B (now visited).
      controller.index = 1;
      await tester.pumpAndSettle();
      controller.refire();
      expect(log, [0, 1]);
    },
  );
}
