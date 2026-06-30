import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  test('ModProject round-trips scripts and emits build spec', () {
    final p = ModProject(
      name: 'M',
      scripts: const [
        ScriptMod(op: ScriptOp.add, moduleName: 'New', relPath: 'New.as', asPath: '/a/New.as', miniPath: '/a/new.cache'),
        ScriptMod(op: ScriptOp.edit, moduleName: 'AI.Foo', relPath: 'AI/Foo.as', asPath: '/a/Foo.as', miniPath: '/a/foo.cache'),
      ],
    );
    final back = ModProject.fromJson(p.toJson());
    expect(back.scripts.length, 2);
    expect(back.scripts.first.moduleName, 'New');

    final spec = p.toBuildSpec();
    final scripts = spec['scripts'] as List;
    expect(scripts.length, 2);
    expect((scripts.first as Map)['op'], 'add');
    expect((scripts.first as Map)['module_name'], 'New');
    expect((scripts.first as Map)['mini_cache'], '/a/new.cache');
  });
}
