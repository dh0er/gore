import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

void main() {
  test('scriptCompile propagates the new-symbol opt-in', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'script_compile': {
          'ok': true,
          'mini_path': 'mini.cache',
          'module': 'GoreMods.Probe',
        },
      },
    );

    await ModFfi(core).scriptCompile(
      gameDir: r'C:\Game',
      op: 'add',
      moduleName: 'GoreMods.Probe',
      relPath: 'GoreMods/Probe.as',
      asPath: r'C:\Source\Probe.as',
      workDir: r'C:\Temp\compile',
      allowNewSymbols: true,
    );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, 'script_compile');
    expect(core.calls.single.payload['allow_new_symbols'], isTrue);
  });
}
