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

    test('rejects the reserved value workspace name', () {
      expect(validateModName('.value-minis'), isNotNull);
      expect(validateModName('.VALUE-MINIS'), isNotNull);
    });

    test('rejects parent reference', () {
      expect(validateModName('..'), isNotNull);
      expect(validateModName('.'), isNotNull);
    });

    test('rejects control characters', () {
      expect(validateModName('Bad\nMod'), isNotNull);
      expect(validateModName('Bad\tMod'), isNotNull);
      expect(validateModName('Bad\u0085Mod'), isNotNull);
    });

    test('rejects names the core cannot publish portably', () {
      for (final name in [
        'CON',
        'NUL.txt',
        'COM1.lua',
        'COM¹.lua',
        'LPT².bank',
        'LPT³',
        r'CLOCK$',
        r'CONIN$',
        r'CONOUT$',
        'CON .txt',
        'Bad:Name',
        'Bad<Name',
        'Bad>Name',
        'Bad"Name',
        'Bad|Name',
        'Bad?Name',
        'Bad*Name',
        'Name.',
        'Name ',
      ]) {
        expect(validateModName(name), isNotNull, reason: name);
      }
    });

    test('enforces the core UTF-8 byte limit', () {
      expect(validateModName('a' * 198), isNull);
      expect(validateModName('a' * 199), isNotNull);
      expect(validateModName('é' * 99), isNull);
      expect(validateModName('é' * 100), isNotNull);
    });
  });
}
