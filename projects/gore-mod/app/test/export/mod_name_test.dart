import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/export/domain/mod_name.dart';

void main() {
  group('validateModName', () {
    test('accepts a plain name', () {
      expect(validateModName('MyBalanceMod'), isNull);
      expect(validateModName('my_mod_123'), isNull);
    });

    test('rejects empty / whitespace', () {
      expect(validateModName(''), isNotNull);
      expect(validateModName('   '), isNotNull);
    });

    test('rejects path separators', () {
      expect(validateModName('a/b'), isNotNull);
      expect(validateModName(r'a\b'), isNotNull);
      expect(validateModName('sub/MyMod'), isNotNull);
    });

    test('rejects parent reference', () {
      expect(validateModName('..'), isNotNull);
      expect(validateModName('.'), isNotNull);
    });

    test('rejects control characters', () {
      expect(validateModName('Bad\nMod'), isNotNull);
      expect(validateModName('Bad\tMod'), isNotNull);
    });
  });
}
