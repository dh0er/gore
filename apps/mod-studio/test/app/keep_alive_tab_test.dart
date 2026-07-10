import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/keep_alive_tab.dart';

void main() {
  testWidgets('KeepAliveTab preserves tab state across tab switches', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
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
                KeepAliveTab(child: TextField(key: Key('field-a'))),
                KeepAliveTab(child: Center(child: Text('Tab B content'))),
              ],
            ),
          ),
        ),
      ),
    );

    // Type into the TextField on tab A.
    await tester.enterText(find.byKey(const Key('field-a')), 'hello state');
    await tester.pump();
    expect(find.text('hello state'), findsOneWidget);

    // Switch to tab B.
    await tester.tap(find.text('B'));
    await tester.pumpAndSettle();
    expect(find.text('Tab B content'), findsOneWidget);

    // Switch back to tab A: the entered text must still be there.
    await tester.tap(find.text('A'));
    await tester.pumpAndSettle();
    expect(find.text('hello state'), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byKey(const Key('field-a'))).controller,
      isNull,
      reason: 'field has no external controller; state lives in the tab',
    );
  });
}
