import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  test('set/remove/clear/loadAll', () {
    final n = TextureReplacementsNotifier();
    n.setReplacement(const TextureReplacement(asset: '/Game/T_A', imagePath: 'a.png'));
    n.setReplacement(const TextureReplacement(asset: '/Game/T_B', imagePath: 'b.png'));
    expect(n.state.count, 2);
    n.remove('/Game/T_A');
    expect(n.state.count, 1);
    n.loadAll([const TextureReplacement(asset: '/Game/T_C', imagePath: 'c.png')]);
    expect(n.state.items.keys.single, '/Game/T_C');
    n.clearAll();
    expect(n.state.count, 0);
  });
}
