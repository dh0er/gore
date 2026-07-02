import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';

void main() {
  test('ScriptModuleInfo.fromJson', () {
    final m = ScriptModuleInfo.fromJson({'name': 'AI.Foo', 'file': 'AI/Foo.as'});
    expect(m.name, 'AI.Foo');
    expect(m.file, 'AI/Foo.as');
  });
}
