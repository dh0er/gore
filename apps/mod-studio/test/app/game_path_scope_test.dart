import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/app/ui/game_path_scope.dart';
import 'package:gore_mod/app/ui/keep_alive_tab.dart';

void main() {
  testWidgets(
    'game path change resets scoped tab state; tab switching alone keeps it',
    (tester) async {
      // gameExePathProvider is backed by NoopUiSettingsStore under
      // FLUTTER_TEST, so driving the real notifier is side-effect free.
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: DefaultTabController(
              length: 2,
              child: Scaffold(
                appBar: TabBar(
                  tabs: [
                    Tab(text: 'A'),
                    Tab(text: 'B'),
                  ],
                ),
                body: TabBarView(
                  children: [
                    // Source-bound tab: state must reset on install switch.
                    KeepAliveTab(
                      child: GamePathScope(
                        child: TextField(key: Key('field-a')),
                      ),
                    ),
                    KeepAliveTab(child: Center(child: Text('Tab B content'))),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(MaterialApp)),
      );

      // State survives plain tab switching (keep-alive, path unchanged).
      await tester.enterText(
        find.byKey(const Key('field-a')),
        'old install selection',
      );
      await tester.pump();
      await tester.tap(find.text('B'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('A'));
      await tester.pumpAndSettle();
      expect(find.text('old install selection'), findsOneWidget);

      // Change the game path while the scoped tab is offstage (the realistic
      // flow: user is in Settings). On return the subtree was recreated, so
      // the stale install-bound state is gone.
      await tester.tap(find.text('B'));
      await tester.pumpAndSettle();
      container
          .read(gameExePathProvider.notifier)
          .set(r'C:\OtherGame\gothic.exe');
      await tester.pumpAndSettle();
      await tester.tap(find.text('A'));
      await tester.pumpAndSettle();
      expect(find.text('old install selection'), findsNothing);
      expect(find.byKey(const Key('field-a')), findsOneWidget);

      // The fresh subtree keeps state again until the next path change.
      await tester.enterText(
        find.byKey(const Key('field-a')),
        'new install selection',
      );
      await tester.pump();
      container.read(gameExePathProvider.notifier).clear();
      await tester.pumpAndSettle();
      expect(find.text('new install selection'), findsNothing);
    },
  );
}
