import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/loc/game_lang.dart';

class _MemoryUiSettingsStore implements UiSettingsStore {
  _MemoryUiSettingsStore(this.settings);

  UiSettings settings;

  @override
  UiSettings read() => settings;

  @override
  void write(UiSettings value) => settings = value;
}

void main() {
  test('exact language match wins', () {
    expect(deviceUiLanguageCode([const Locale('de')]), 'de');
    expect(deviceUiLanguageCode([const Locale('de', 'DE')]), 'de');
    expect(deviceUiLanguageCode([const Locale('ja')]), 'ja');
    expect(deviceUiLanguageCode([const Locale('cs')]), 'cs');
    expect(deviceUiLanguageCode([const Locale('uk')]), 'uk');
    expect(deviceUiLanguageCode([const Locale('hu')]), 'hu');
    expect(deviceUiLanguageCode([const Locale('ro')]), 'ro');
    expect(deviceUiLanguageCode([const Locale('tr')]), 'tr');
  });

  test('Portuguese and Chinese resolve to a shipped variant', () {
    expect(deviceUiLanguageCode([const Locale('pt', 'PT')]), 'pt-BR');
    expect(deviceUiLanguageCode([const Locale('zh', 'CN')]), 'zh-Hans');
    expect(
      deviceUiLanguageCode([
        const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
      ]),
      'zh-Hans',
    );
    expect(
      deviceUiLanguageCode([
        const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
      ]),
      'zh-Hant',
    );
    expect(deviceUiLanguageCode([const Locale('zh', 'TW')]), 'zh-Hant');
    expect(
      deviceUiLanguageCode([
        const Locale.fromSubtags(
          languageCode: 'zh',
          scriptCode: 'Hans',
          countryCode: 'TW',
        ),
      ]),
      'zh-Hans',
    );
    expect(
      deviceUiLanguageCode([
        const Locale.fromSubtags(
          languageCode: 'zh',
          scriptCode: 'Hant',
          countryCode: 'CN',
        ),
      ]),
      'zh-Hant',
    );
    expect(deviceUiLanguageCode([const Locale('zh', 'HK')]), 'zh-Hant');
    expect(deviceUiLanguageCode([const Locale('zh', 'MO')]), 'zh-Hant');
  });

  test('interface languages without game text select a catalog language', () {
    expect(defaultGameTextCode('de'), 'de');
    expect(defaultGameTextCode('zh-Hans'), 'zh-Hans');
    expect(defaultGameTextCode('zh-Hant'), 'zh-Hans');
    expect(defaultGameTextCode('cs'), 'en');
    expect(defaultGameTextCode('uk'), 'en');
    expect(defaultGameTextCode('hu'), 'en');
    expect(defaultGameTextCode('ro'), 'en');
    expect(defaultGameTextCode('tr'), 'en');
  });

  test('first supported device locale wins over later ones', () {
    expect(
      deviceUiLanguageCode([const Locale('ko'), const Locale('fr')]),
      'fr',
    );
    expect(
      deviceUiLanguageCode([const Locale('ar'), const Locale('cs')]),
      'cs',
    );
  });

  test('falls back to English when nothing is supported', () {
    expect(deviceUiLanguageCode([const Locale('ko')]), 'en');
    expect(deviceUiLanguageCode([const Locale('ar')]), 'en');
    expect(deviceUiLanguageCode(const []), 'en');
  });

  test('a saved interface language without game text keeps that pairing', () {
    final store = _MemoryUiSettingsStore(const UiSettings(appLocale: 'de'));
    final container = ProviderContainer(
      overrides: [uiSettingsStoreProvider.overrideWithValue(store)],
    );
    addTearDown(container.dispose);

    expect(container.read(localeProvider), 'de');
    expect(container.read(gameTextLocaleProvider), 'de');
    expect(store.read().toJson().containsKey('gameTextLocale'), isFalse);
  });

  test('game text can diverge until the interface language changes', () {
    final store = _MemoryUiSettingsStore(
      const UiSettings(appLocale: 'de', gameTextLocale: 'de'),
    );
    final container = ProviderContainer(
      overrides: [uiSettingsStoreProvider.overrideWithValue(store)],
    );
    addTearDown(container.dispose);

    container.read(gameTextLocaleProvider.notifier).setGameTextLocale('fr');
    expect(container.read(localeProvider), 'de');
    expect(container.read(gameTextLocaleProvider), 'fr');

    container.read(localeProvider.notifier).setLocale('cs');
    container
        .read(gameTextLocaleProvider.notifier)
        .setGameTextLocale(defaultGameTextCode('cs'));
    expect(container.read(localeProvider), 'cs');
    expect(container.read(gameTextLocaleProvider), 'en');
    expect(store.read().gameTextLocale, 'en');

    container.read(localeProvider.notifier).setLocale('zh-Hant');
    container
        .read(gameTextLocaleProvider.notifier)
        .setGameTextLocale(defaultGameTextCode('zh-Hant'));
    expect(container.read(gameTextLocaleProvider), 'zh-Hans');
  });

  test('Podkova is unavailable for Japanese and Chinese', () {
    expect(
      uiFontFamilySupportedFor(UiFontFamily.podkova, uiLangByCode('en').locale),
      isTrue,
    );
    expect(
      uiFontFamilySupportedFor(UiFontFamily.podkova, uiLangByCode('ru').locale),
      isTrue,
    );
    expect(
      uiFontFamilySupportedFor(UiFontFamily.podkova, uiLangByCode('cs').locale),
      isTrue,
    );
    expect(
      uiFontFamilySupportedFor(UiFontFamily.podkova, uiLangByCode('uk').locale),
      isTrue,
    );
    expect(
      uiFontFamilySupportedFor(UiFontFamily.podkova, uiLangByCode('ja').locale),
      isFalse,
    );
    expect(
      uiFontFamilySupportedFor(
        UiFontFamily.podkova,
        uiLangByCode('zh-Hans').locale,
      ),
      isFalse,
    );
    expect(
      uiFontFamilySupportedFor(
        UiFontFamily.podkova,
        uiLangByCode('zh-Hant').locale,
      ),
      isFalse,
    );
  });
}
