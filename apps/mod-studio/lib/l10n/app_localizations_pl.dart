// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class AppLocalizationsPl extends AppLocalizations {
  AppLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get gameExecutable => 'Game executable';

  @override
  String get gameExecutableSubtitle =>
      'Path to the game\'s .exe. Used to auto-detect localized text and the game install.';

  @override
  String get gameExecutableNotSet => 'Not set';

  @override
  String get chooseGameExecutable => 'Choose…';

  @override
  String get settingsDataSourceSection => 'Game data';

  @override
  String get settingsLocalizationSection => 'Localized text';

  @override
  String get extractLocalizedText => 'Wyodrębnij zlokalizowane teksty';

  @override
  String get lightMode => 'Tryb jasny';

  @override
  String get darkMode => 'Tryb ciemny';

  @override
  String get language => 'Język';

  @override
  String get exportMod => 'Eksportuj mod';

  @override
  String exportModWithCount(int count) {
    return 'Eksportuj mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Wybierz przedmiot, aby edytować jego pola.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dane gry: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dane gry: dołączone';

  @override
  String get loadGameDataDump => 'Wczytaj zrzut danych gry…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json z moda gore-dump';

  @override
  String get useBundledData => 'Użyj dołączonych danych';

  @override
  String get alreadyBundled => 'już dołączone';

  @override
  String get gameDataFileGroupLabel => 'dane gry';

  @override
  String get minimize => 'Minimalizuj';

  @override
  String get restore => 'Przywróć';

  @override
  String get maximize => 'Maksymalizuj';

  @override
  String get close => 'Zamknij';

  @override
  String get categoryMeleeWeapons => 'Broń biała';

  @override
  String get categoryRangedWeapons => 'Broń dystansowa';

  @override
  String get categoryAmmunition => 'Amunicja';

  @override
  String get categoryRunes => 'Runy';

  @override
  String get categorySpellScrolls => 'Zwoje zaklęć';

  @override
  String get categoryFoodAndPotions => 'Jedzenie i mikstury';

  @override
  String get categoryMiscellaneous => 'Różne';

  @override
  String get categoryAmulets => 'Amulety';

  @override
  String get categoryRings => 'Pierścienie';

  @override
  String get categoryAnimalTrophies => 'Trofea ze zwierząt';

  @override
  String get categoryWritings => 'Pisma';

  @override
  String get categoryMissionItems => 'Przedmioty zadań';

  @override
  String get categoryKeys => 'Klucze';

  @override
  String get categoryOther => 'Inne';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Szukaj przedmiotów';

  @override
  String get noItemsMatch => 'Brak pasujących przedmiotów';

  @override
  String failedToLoadCatalog(String error) {
    return 'Nie udało się wczytać katalogu: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Oczekujące zmiany ($count)';
  }

  @override
  String get clearAll => 'Wyczyść wszystko';

  @override
  String get noPendingOverrides =>
      'Brak oczekujących zmian.\nEdytuj pola przedmiotów, aby dodać.';

  @override
  String get removeOverride => 'Usuń zmianę';

  @override
  String get modName => 'Nazwa moda';

  @override
  String get loadDelayLabel => 'Opóźnienie wczytywania (ms, 0 = natychmiast)';

  @override
  String get noFolderSelected => 'Nie wybrano folderu';

  @override
  String get chooseFolder => 'Wybierz folder';

  @override
  String get packageAsZip => 'Spakuj jako .zip';

  @override
  String get cancel => 'Anuluj';

  @override
  String get export => 'Eksportuj';

  @override
  String get exportHere => 'Eksportuj tutaj';

  @override
  String get mustBeNonNegativeInteger => 'Musi być nieujemną liczbą całkowitą';

  @override
  String get extractingLocalizedText =>
      'Wyodrębnianie zlokalizowanych tekstów gry…';

  @override
  String get localizedTextExtractionCancelled =>
      'Anulowano wyodrębnianie zlokalizowanych tekstów.';

  @override
  String get localizedTextExtracted => 'Wyodrębniono zlokalizowane teksty.';

  @override
  String get extractionFailed => 'Wyodrębnianie nie powiodło się.';

  @override
  String get localizationCacheFileGroupLabel => 'pamięć podręczna lokalizacji';

  @override
  String get extractLocalizedTextQuestion =>
      'Wyodrębnić zlokalizowane teksty gry?';

  @override
  String get extractLocalizedTextBody =>
      'Zlokalizowane teksty gry nie zostały jeszcze wyodrębnione. Wyodrębnić je teraz z Twojej instalacji gry? (opcjonalnie)';

  @override
  String get notNow => 'Nie teraz';

  @override
  String get extract => 'Wyodrębnij';

  @override
  String get validationRequired => 'Wymagane';

  @override
  String get validationMustBeWholeNumber => 'Musi być liczbą całkowitą';

  @override
  String get validationMustBeNumber => 'Musi być liczbą';

  @override
  String get validationMustBeFinite => 'Musi być liczbą skończoną';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Musi być ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Musi być ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Musi być true lub false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Musi być jednym z: $options';
  }

  @override
  String get modNameRequired => 'Wymagane';

  @override
  String get modNameControlCharacters => 'Nie może zawierać znaków sterujących';

  @override
  String get modNamePathSeparators => 'Nie może zawierać separatorów ścieżki';

  @override
  String get modNameNotAFolderName => 'Nieprawidłowa nazwa folderu';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Wyodrębniono $idCount identyfikatorów w $languageCount językach';
  }
}
