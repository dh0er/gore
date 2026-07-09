import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/loc/game_lang.dart';

void main() {
  test('exact language match wins', () {
    expect(deviceLanguageCode([const Locale('de')]), 'de');
    expect(deviceLanguageCode([const Locale('de', 'DE')]), 'de');
    expect(deviceLanguageCode([const Locale('ja')]), 'ja');
  });

  test('region variants collapse to the only shipped variant', () {
    // Only pt-BR and zh-Hans ship; any region/script maps onto them.
    expect(deviceLanguageCode([const Locale('pt', 'PT')]), 'pt-BR');
    expect(deviceLanguageCode([const Locale('zh', 'CN')]), 'zh-Hans');
    expect(
      deviceLanguageCode(
        [const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant')],
      ),
      'zh-Hans',
    );
  });

  test('first supported device locale wins over later ones', () {
    expect(deviceLanguageCode([const Locale('ko'), const Locale('fr')]), 'fr');
  });

  test('falls back to English when nothing is supported', () {
    expect(deviceLanguageCode([const Locale('ko')]), 'en');
    expect(deviceLanguageCode(const []), 'en');
  });
}
