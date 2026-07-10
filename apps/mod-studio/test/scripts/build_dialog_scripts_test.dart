import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/export/ui/build_deploy_dialog.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  testWidgets('build dialog counts script mods', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(scriptModsProvider.notifier).setMod(
      const ScriptMod(op: ScriptOp.add, moduleName: 'M', relPath: 'M.as', asPath: 'a', miniPath: 'm'),
    );
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
    ));
    expect(find.textContaining('1 script'), findsOneWidget);
  });
}
