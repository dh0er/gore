import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  test('ScriptMod json round-trips', () {
    const m = ScriptMod(
      op: ScriptOp.edit,
      moduleName: 'AI.AIItemScoring',
      relPath: 'AI/AIItemScoring.as',
      asPath: '/tmp/AIItemScoring.as',
      miniPath: '/tmp/mini.cache',
      compiledHash: 'deadbeefdeadbeef',
    );
    final j = m.toJson();
    expect(j['compiled_hash'], 'deadbeefdeadbeef');
    final back = ScriptMod.fromJson(j);
    expect(back.op, ScriptOp.edit);
    expect(back.moduleName, 'AI.AIItemScoring');
    expect(back.relPath, 'AI/AIItemScoring.as');
    expect(back.asPath, '/tmp/AIItemScoring.as');
    expect(back.miniPath, '/tmp/mini.cache');
    expect(back.compiledHash, 'deadbeefdeadbeef');
    expect(back.compiled, isTrue);
  });

  test('compiled_hash defaults to empty when absent from json', () {
    final back = ScriptMod.fromJson({
      'op': 'add',
      'module': 'M',
      'rel_path': 'M.as',
      'as_path': '/tmp/M.as',
      'mini_path': '/tmp/M.cache',
    });
    expect(back.compiledHash, '');
  });

  test('scriptCompileFresh tracks the compiled .as content', () {
    final dir = Directory.systemTemp.createTempSync('goremod_fresh_test_');
    addTearDown(() {
      try {
        dir.deleteSync(recursive: true);
      } catch (_) {}
    });
    final asFile = File('${dir.path}/Mod.as')..writeAsStringSync('void main() {}');
    final mini = File('${dir.path}/Mod.cache')..writeAsStringSync('compiled-bytes');
    final hash = fnv1aHex(asFile.readAsBytesSync());

    // Compiled mini + matching content hash => fresh.
    final fresh = ScriptMod(
      op: ScriptOp.add,
      moduleName: 'Mod',
      relPath: 'Mod.as',
      asPath: asFile.path,
      miniPath: mini.path,
      compiledHash: hash,
    );
    expect(scriptCompileFresh(fresh), isTrue);

    // Editing the source on disk (same path) => no longer fresh.
    asFile.writeAsStringSync('void main() { Print("edited"); }');
    expect(scriptCompileFresh(fresh), isFalse);

    // A mod with no compiled mini is never fresh, even with a (stale) hash.
    final notCompiled = ScriptMod(
      op: ScriptOp.add,
      moduleName: 'Mod',
      relPath: 'Mod.as',
      asPath: asFile.path,
      compiledHash: hash,
    );
    expect(scriptCompileFresh(notCompiled), isFalse);

    // withSource clears the compile, so freshness drops even before any edit.
    final reSourced = fresh.withSource(asFile.path);
    expect(reSourced.miniPath, isEmpty);
    expect(reSourced.compiledHash, isEmpty);
    expect(scriptCompileFresh(reSourced), isFalse);
  });

  test('notifier set/remove/count/clear/load', () {
    final n = ScriptModsNotifier();
    expect(n.state.count, 0);
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M1', relPath: 'M1.as', asPath: 'a'));
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M2', relPath: 'M2.as', asPath: 'b'));
    expect(n.state.count, 2);
    // The staging key is relPath, not moduleName — remove by the relPath key.
    n.remove('M1.as');
    expect(n.state.count, 1);
    expect(n.state.entries.single.moduleName, 'M2');
    n.loadAll([const ScriptMod(op: ScriptOp.edit, moduleName: 'M3', relPath: 'M3.as', asPath: 'c')]);
    expect(n.state.count, 1);
    expect(n.state.entries.single.op, ScriptOp.edit);
    n.clearAll();
    expect(n.state.count, 0);
  });

  test('mods sharing a moduleName but differing in relPath coexist (keyed by relPath)', () {
    final n = ScriptModsNotifier();
    // Two distinct game-relative paths that flatten to the SAME module basename (`Foo`). Keying by
    // moduleName would overwrite one with the other; keying by relPath keeps both.
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'Foo', relPath: 'AI/Foo.as', asPath: 'a'));
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'Foo', relPath: 'Quest/Foo.as', asPath: 'b'));
    expect(n.state.count, 2);
    expect(n.state.items.keys.toSet(), {'AI/Foo.as', 'Quest/Foo.as'});
    // Removing one relPath leaves the other intact.
    n.remove('AI/Foo.as');
    expect(n.state.count, 1);
    expect(n.state.entries.single.relPath, 'Quest/Foo.as');
    // Re-staging the SAME relPath overwrites in place (same target — correct), no duplicate.
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'Bar', relPath: 'Quest/Foo.as', asPath: 'c'));
    expect(n.state.count, 1);
    expect(n.state.entries.single.moduleName, 'Bar');
  });
}
