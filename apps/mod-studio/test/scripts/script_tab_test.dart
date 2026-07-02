import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/scripts/ui/script_tab.dart';
import 'package:path/path.dart' as p;

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

  // Fix 3: loadProject treats the script relPath as untrusted (defense-in-depth, matching the
  // asPath guard + gore-as compile-side check) and drops mods whose relPath is empty/absolute/'..'.
  test('loadProject drops script mods with an unsafe relPath', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_relpath_test_');
    addTearDown(() => tmp.deleteSync(recursive: true));
    final asFile = File(p.join(tmp.path, 'New.as'))..writeAsStringSync('void Foo(){}');
    final project = ModProject(
      name: 'M',
      scripts: [
        // Safe sibling — must survive the load.
        ScriptMod(op: ScriptOp.add, moduleName: 'Good', relPath: 'AI/Good.as', asPath: asFile.path),
        // Escapes the staged tree — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Esc', relPath: '../evil.as', asPath: asFile.path),
        // Absolute — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Abs', relPath: '/etc/evil.as', asPath: asFile.path),
        // Empty — must be dropped.
        ScriptMod(op: ScriptOp.add, moduleName: 'Empty', relPath: '', asPath: asFile.path),
      ],
    );
    final out = p.join(tmp.path, 'm.goremod');
    await saveProject(project, out);
    final loaded = await loadProject(out);
    expect(loaded.scripts.map((s) => s.moduleName).toList(), ['Good']);
    expect(loaded.scripts.single.relPath, 'AI/Good.as');
  });
}
