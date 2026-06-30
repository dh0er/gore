import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/scripts/ui/script_tab.dart';

void main() {
  testWidgets('shows staged script mods', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(scriptModsProvider.notifier).setMod(
      const ScriptMod(op: ScriptOp.add, moduleName: 'MyNewModule', relPath: 'MyNewModule.as', asPath: '/x/MyNewModule.as'),
    );
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: ScriptTab())),
    ));
    expect(find.text('MyNewModule'), findsOneWidget);
    // An uncompiled mod surfaces a "not compiled" affordance.
    expect(find.textContaining('compile', findRichText: true), findsWidgets);
  });
}
