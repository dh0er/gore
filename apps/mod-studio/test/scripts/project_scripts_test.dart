import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:path/path.dart' as p;

void main() {
  test('ModProject round-trips scripts and emits build spec', () {
    final p = ModProject(
      name: 'M',
      scripts: const [
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'New',
          relPath: 'New.as',
          asPath: '/a/New.as',
          miniPath: '/a/new.cache',
        ),
        ScriptMod(
          op: ScriptOp.edit,
          moduleName: 'AI.Foo',
          relPath: 'AI/Foo.as',
          asPath: '/a/Foo.as',
          allowNewSymbols: true,
          miniPath: '/a/foo.cache',
        ),
      ],
    );
    final back = ModProject.fromJson(p.toJson());
    expect(back.scripts.length, 2);
    expect(back.scripts.first.moduleName, 'New');
    expect(back.scripts.first.allowNewSymbols, isTrue);
    expect(back.scripts.last.allowNewSymbols, isTrue);

    final spec = p.toBuildSpec();
    final scripts = spec['scripts'] as List;
    expect(scripts.length, 2);
    expect((scripts.first as Map)['op'], 'add');
    expect((scripts.first as Map)['module_name'], 'New');
    expect((scripts.first as Map)['mini_cache'], '/a/new.cache');
  });

  test('saveProject/loadProject embeds and restores script .as + mini', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_scripts_test_');
    final asFile = File(p.join(tmp.path, 'New.as'))
      ..writeAsStringSync('void Foo(){}');
    final miniFile = File(p.join(tmp.path, 'new.cache'))
      ..writeAsBytesSync([1, 2, 3]);
    final project = ModProject(
      name: 'M',
      scripts: [
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'New',
          relPath: 'New.as',
          asPath: asFile.path,
          miniPath: miniFile.path,
        ),
      ],
    );
    final out = p.join(tmp.path, 'm.goremod');
    await saveProject(project, out);
    // Delete the originals so a passing load can only have come from embedded copies.
    asFile.deleteSync();
    miniFile.deleteSync();
    final loaded = await loadProject(out);
    expect(loaded.scripts.length, 1);
    final s = loaded.scripts.single;
    expect(File(s.asPath).readAsStringSync(), 'void Foo(){}');
    expect(File(s.miniPath).readAsBytesSync(), [1, 2, 3]);
  });
}
