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
    );
    final j = m.toJson();
    final back = ScriptMod.fromJson(j);
    expect(back.op, ScriptOp.edit);
    expect(back.moduleName, 'AI.AIItemScoring');
    expect(back.relPath, 'AI/AIItemScoring.as');
    expect(back.asPath, '/tmp/AIItemScoring.as');
    expect(back.miniPath, '/tmp/mini.cache');
    expect(back.compiled, isTrue);
  });

  test('notifier set/remove/count/clear/load', () {
    final n = ScriptModsNotifier();
    expect(n.state.count, 0);
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M1', relPath: 'M1.as', asPath: 'a'));
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M2', relPath: 'M2.as', asPath: 'b'));
    expect(n.state.count, 2);
    n.remove('M1');
    expect(n.state.count, 1);
    expect(n.state.entries.single.moduleName, 'M2');
    n.loadAll([const ScriptMod(op: ScriptOp.edit, moduleName: 'M3', relPath: 'M3.as', asPath: 'c')]);
    expect(n.state.count, 1);
    expect(n.state.entries.single.op, ScriptOp.edit);
    n.clearAll();
    expect(n.state.count, 0);
  });
}
