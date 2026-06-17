import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/utils/default_paths.dart';

void main() {
  test('default save root points at the G1R SaveGames folder', () {
    final root = defaultSaveRoot();

    // Regardless of which environment variable resolves it, the default always
    // ends at the game's SaveGames directory.
    expect(root, contains('G1R'));
    expect(root, endsWith('SaveGames'));
  });
}
