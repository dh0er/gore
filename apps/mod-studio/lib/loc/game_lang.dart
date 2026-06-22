import 'package:flutter/widgets.dart';

/// One of the 10 user-selectable languages. Drives BOTH the app UI locale and
/// which game-text strings (from the extracted loc_catalog) are shown.
class GameLang {
  final String code;          // canonical: en de fr it es pl ru ja zh-Hans pt-BR
  final String endonym;       // native name shown in the picker
  final Locale locale;        // Flutter locale for MaterialApp
  final List<String> locSets; // loc_catalog set names, highest priority first
  const GameLang(this.code, this.endonym, this.locale, this.locSets);
}

const List<String> kEnglishLocSets = ['english_newer', 'english_new', 'english'];

const List<GameLang> kGameLangs = [
  GameLang('en', 'English', Locale('en'), kEnglishLocSets),
  GameLang('de', 'Deutsch', Locale('de'), ['german_new', 'german']),
  GameLang('fr', 'Français', Locale('fr'), ['french']),
  GameLang('it', 'Italiano', Locale('it'), ['italian']),
  GameLang('es', 'Español', Locale('es'), ['spanish']),
  GameLang('pl', 'Polski', Locale('pl'), ['polish']),
  GameLang('ru', 'Русский', Locale('ru'), ['russian']),
  GameLang('ja', '日本語', Locale('ja'), ['japanese']),
  GameLang('zh-Hans', '简体中文',
      Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'), ['schinese']),
  GameLang('pt-BR', 'Português (Brasil)', Locale('pt', 'BR'), ['brazilian']),
];

GameLang gameLangByCode(String? code) =>
    kGameLangs.firstWhere((l) => l.code == code, orElse: () => kGameLangs.first);

/// Resolve a game-text id to its localized value for [lang], falling back to
/// English, then null. [catalog] is the loaded loc_catalog: id -> {set -> text}.
String? resolveGameText(
    Map<String, Map<String, String>> catalog, String locId, GameLang lang) {
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

/// loc id for a catalog class id (item `ItFo_Cheese` -> `itfo_cheese`).
String locIdForCatalogId(String catalogId) => catalogId.toLowerCase();
