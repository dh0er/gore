import 'package:flutter/widgets.dart';

/// One game-text language. These are exactly the languages the extracted
/// loc_catalog can show. The interface language is a separate list ([kUiLangs]).
class GameLang {
  final String code; // canonical: en de fr it es pl ru ja zh-Hans pt-BR
  final String endonym; // native name shown in the picker
  final Locale locale;
  final List<String> locSets; // loc_catalog set names, highest priority first
  const GameLang(this.code, this.endonym, this.locale, this.locSets);
}

/// One interface language. [gameTextCode] is the game-text language selected
/// automatically when the user picks this interface language.
class UiLang {
  final String code;
  final String endonym;
  final Locale locale;
  final String gameTextCode;
  const UiLang(this.code, this.endonym, this.locale, this.gameTextCode);
}

const List<String> kEnglishLocSets = [
  'english_newer',
  'english_new',
  'english',
];

const List<GameLang> kGameLangs = [
  GameLang('en', 'English', Locale('en'), kEnglishLocSets),
  GameLang('de', 'Deutsch', Locale('de'), ['german_new', 'german']),
  GameLang('fr', 'Français', Locale('fr'), ['french']),
  GameLang('it', 'Italiano', Locale('it'), ['italian']),
  GameLang('es', 'Español', Locale('es'), ['spanish']),
  GameLang('pl', 'Polski', Locale('pl'), ['polish']),
  GameLang('ru', 'Русский', Locale('ru'), ['russian']),
  GameLang('ja', '日本語', Locale('ja'), ['japanese']),
  GameLang(
    'zh-Hans',
    '简体中文',
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
    ['schinese'],
  ),
  GameLang('pt-BR', 'Português (Brasil)', Locale('pt', 'BR'), ['brazilian']),
];

/// Interface languages. Codes shared with [kGameLangs] select that same game
/// text. The extra languages have no catalog of their own and select English,
/// except Traditional Chinese, which selects the simplified Chinese catalog
/// (the only Chinese text the game ships).
///
/// Korean and Arabic are omitted: none of the bundled fonts contain Hangul
/// or Arabic glyphs, so those interfaces would render as missing-glyph boxes.
const List<UiLang> kUiLangs = [
  UiLang('en', 'English', Locale('en'), 'en'),
  UiLang('de', 'Deutsch', Locale('de'), 'de'),
  UiLang('fr', 'Français', Locale('fr'), 'fr'),
  UiLang('it', 'Italiano', Locale('it'), 'it'),
  UiLang('es', 'Español', Locale('es'), 'es'),
  UiLang('pl', 'Polski', Locale('pl'), 'pl'),
  UiLang('ru', 'Русский', Locale('ru'), 'ru'),
  UiLang('ja', '日本語', Locale('ja'), 'ja'),
  UiLang(
    'zh-Hans',
    '简体中文',
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
    'zh-Hans',
  ),
  UiLang(
    'zh-Hant',
    '繁體中文',
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
    'zh-Hans',
  ),
  UiLang('pt-BR', 'Português (Brasil)', Locale('pt', 'BR'), 'pt-BR'),
  UiLang('cs', 'Čeština', Locale('cs'), 'en'),
  UiLang('uk', 'Українська', Locale('uk'), 'en'),
  UiLang('hu', 'Magyar', Locale('hu'), 'en'),
  UiLang('ro', 'Română', Locale('ro'), 'en'),
  UiLang('tr', 'Türkçe', Locale('tr'), 'en'),
];

GameLang gameLangByCode(String? code) => kGameLangs.firstWhere(
  (l) => l.code == code,
  orElse: () => kGameLangs.first,
);

UiLang uiLangByCode(String? code) =>
    kUiLangs.firstWhere((l) => l.code == code, orElse: () => kUiLangs.first);

/// Game-text language selected together with the interface language [uiCode].
String defaultGameTextCode(String? uiCode) => uiLangByCode(uiCode).gameTextCode;

/// Best-supported interface language for the device's preferred
/// [deviceLocales] (highest priority first). Returns `'en'` when none match.
///
/// An explicit Chinese script wins. Region is used only when the script is
/// absent: TW / HK / MO resolve to Traditional Chinese, every other `zh` to
/// Simplified. Any `pt` resolves to `pt-BR`, the only Portuguese variant
/// shipped.
String deviceUiLanguageCode(Iterable<Locale> deviceLocales) {
  for (final device in deviceLocales) {
    final code = _uiCodeForDeviceLocale(device);
    if (code != null) return code;
  }
  return 'en';
}

String? _uiCodeForDeviceLocale(Locale device) {
  switch (device.languageCode) {
    case 'zh':
      if (device.scriptCode == 'Hans') return 'zh-Hans';
      if (device.scriptCode == 'Hant') return 'zh-Hant';
      final traditional = const {
        'TW',
        'HK',
        'MO',
      }.contains(device.countryCode?.toUpperCase());
      return traditional ? 'zh-Hant' : 'zh-Hans';
    case 'pt':
      return 'pt-BR';
  }
  for (final lang in kUiLangs) {
    final locale = lang.locale;
    if (locale.scriptCode != null || locale.countryCode != null) continue;
    if (locale.languageCode == device.languageCode) return lang.code;
  }
  return null;
}

/// Resolve a game-text id to its localized value for [lang], falling back to
/// English, then null. [catalog] is the loaded loc_catalog: id -> {set -> text}.
String? resolveGameText(
  Map<String, Map<String, String>> catalog,
  String locId,
  GameLang lang,
) {
  final entry = catalog[locId.toLowerCase()];
  if (entry == null) return null;
  for (final set in lang.locSets) {
    final v = entry[set];
    if (v != null && v.trim().isNotEmpty) return v;
  }
  for (final set in kEnglishLocSets) {
    final v = entry[set];
    if (v != null && v.trim().isNotEmpty) return v;
  }
  return null;
}

/// loc id for a catalog class id (item `ItFo_Cheese` -> `itfo_cheese`,
/// npc catalog id `OC_STT_Diego` -> `oc_stt_diego`, knowledge `Choice62749` -> `choice62749`).
String locIdForCatalogId(String catalogId) => catalogId.toLowerCase();
