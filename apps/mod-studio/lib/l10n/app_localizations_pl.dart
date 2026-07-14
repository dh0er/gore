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
  String get tabDialogs => 'Dialogi';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Tekstury';

  @override
  String get tabScripts => 'Skrypty';

  @override
  String get changesAll => 'Wszystkie';

  @override
  String get sectionItemValues => 'Wartości przedmiotów';

  @override
  String get sectionLocalizedText => 'Zlokalizowane teksty';

  @override
  String get audioCatCreatures => 'Stworzenia';

  @override
  String get audioCatObjects => 'Obiekty';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Ruch';

  @override
  String get audioCatWorld => 'Świat';

  @override
  String get audioCatAction => 'Akcje';

  @override
  String get audioCatCombat => 'Walka';

  @override
  String get audioCatPhysics => 'Fizyka';

  @override
  String get audioCatItems => 'Przedmioty';

  @override
  String get audioCatUi => 'Interfejs';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Pod wodą';

  @override
  String get audioCatVision => 'Wizje';

  @override
  String get audioCatDialog => 'Dialog';

  @override
  String get audioCatOther => 'Inne';

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
  String get about => 'O programie';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 współtwórcy GORE';

  @override
  String get aboutLicense => 'Udostępniane na licencji MIT.';

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
  String get searchChanges => 'Szukaj zmian';

  @override
  String get noChangesMatch => 'Brak pasujących zmian';

  @override
  String get clearSection => 'Wyczyść tę grupę';

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

  @override
  String get managerDeployActive =>
      'Aktywny jest loadout mod-managera. Najpierw wykonaj undeploy w gore-manager.';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

  @override
  String get projectManagedRevision3IdentityOnly =>
      'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.';

  @override
  String get projectRoot => 'Root';

  @override
  String get projectId => 'Project ID';

  @override
  String get projectRevision => 'Project revision';

  @override
  String get projectHeadSha256 => 'Head SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot bytes';

  @override
  String get projectNoCurrent => 'No current project';

  @override
  String projectManagedRevision3Opened(String projectId) {
    return 'Opened managed revision-3 project $projectId';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Managed revision-3 project open failed: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Verified revision-3 head $headSha256';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Revision-3 head verification failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      'Nowy projekt jest otwarty, ale nie udało się całkowicie wyczyścić sesji poprzedniego projektu. Czyszczenie nie zostanie ponowione. Uruchom ponownie Mod Studio przed ponownym otwarciem poprzedniego projektu.';

  @override
  String get projectNewManagedRevision3 => 'Nowy zarządzany projekt moda…';

  @override
  String get projectNewLegacy => 'Nowy projekt starszego typu';

  @override
  String get projectCreateGamePathRequired =>
      'Przed utworzeniem projektu moda ustaw ścieżkę do Gothic 1 Remake w Ustawieniach.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Utwórz tutaj zarządzany projekt moda';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Utworzono zarządzany projekt moda $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Nie udało się utworzyć zarządzanego projektu moda: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Utwórz projekt moda';

  @override
  String get projectCreateNameLabel => 'Nazwa projektu';

  @override
  String get projectCreateNameHelper => 'Nazwa wyświetlana w Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Wersja';

  @override
  String get projectCreateVersionHelper =>
      'Wersja początkowa, na przykład 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Twoja nazwa lub nazwa zespołu modderskiego.';

  @override
  String get projectCreateLocalesLabel => 'Języki edycji';

  @override
  String get projectCreateLocalesHelper =>
      'Kanoniczne tagi rozdzielone przecinkami, na przykład: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Tworzy pusty, zarządzany projekt offline. Nie kompiluje, nie wdraża ani nie uruchamia moda oraz nie zmienia plików gry ani zapisów.';

  @override
  String get projectCreateSubmit => 'Utwórz projekt';

  @override
  String projectCreateMetadataRequired(String label) {
    return 'Pole $label jest wymagane.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return 'Pole $label nie może zaczynać się ani kończyć białym znakiem.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return 'Pole $label nie może zawierać znaków sterujących.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return 'Pole $label zawiera nieprawidłowy tekst.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return 'Pole $label przekracza limit UTF-8 wynoszący $maxBytes bajtów.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Wprowadź co najmniej jeden język edycji.';

  @override
  String get projectCreateLocalesEmptyEntry => 'Usuń pusty wpis języka.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Użyj najwyżej $maxLocales języków edycji.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'Ustawienie regionalne „$locale” musi być ograniczonym ciągiem ASCII.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'Ustawienie regionalne „$locale” wymaga języka zapisanego 2–8 małymi literami.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'Ustawienie regionalne „$locale” zawiera nieprawidłowy segment.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'Ustawienie regionalne „$locale” nie jest kanoniczne; użyj „$canonical”.';
  }
}
