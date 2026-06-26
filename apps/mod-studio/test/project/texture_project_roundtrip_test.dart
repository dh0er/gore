import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  test('ModProject round-trips textures through json', () {
    final p = ModProject(name: 'M', textures: const [
      TextureReplacement(asset: '/Game/UI/T_X', imagePath: 'x.png'),
    ]);
    final back = ModProject.fromJson(p.toJson());
    expect(back.textures.single.asset, '/Game/UI/T_X');
    expect(back.toBuildSpec()['texture'], isA<List>());
  });
}
