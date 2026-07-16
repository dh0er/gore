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
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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

  @override
  String get managedWorkspaceOverviewLabel => 'Przegląd';

  @override
  String get managedWorkspaceContentLabel => 'Zawartość';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Ten mod';

  @override
  String get managedWorkspaceHomeLabel => 'Strona główna';

  @override
  String get managedWorkspaceStoryLabel => 'Fabuła';

  @override
  String get managedWorkspaceWorldLabel => 'Świat';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Lokalizacja i głosy';

  @override
  String get managedWorkspaceValidateTestLabel => 'Walidacja i testy';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Kompilacja i wydanie';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Ustawienia i tryb ekspercki';

  @override
  String get managedSectionStoryDescription => 'NPC, zadania i dialogi.';

  @override
  String get managedStoryWorkspaceLoading =>
      'Opening the current Story drafts…';

  @override
  String get managedStoryWorkspaceAuthorityNotice =>
      'Project-only NPC and Quest drafts. Build readiness has not been evaluated; runtime behavior remains unqualified.';

  @override
  String get managedStoryWorkspaceSearchHint =>
      'Search NPC and Quest names, objectives, speakers, or IDs';

  @override
  String get managedStoryWorkspaceCreatingNpc => 'Creating NPC draft…';

  @override
  String get managedStoryWorkspaceCreatingQuest => 'Creating Quest draft…';

  @override
  String get managedStoryWorkspaceEmpty => 'No NPC or Quest drafts yet';

  @override
  String get managedStoryWorkspaceNoMatches =>
      'No NPC or Quest drafts match this search';

  @override
  String get managedStoryWorkspaceSelectDraft =>
      'Select an NPC or Quest draft to continue';

  @override
  String get managedStoryWorkspaceLoadErrorTitle =>
      'Story drafts could not be opened';

  @override
  String get managedStoryWorkspaceCheckpointMismatch =>
      'The project changed while Story was loading. Refresh the exact current checkpoint and try again.';

  @override
  String get managedStoryWorkspacePublishedSelectionStale =>
      'The saved Story draft could not be selected at its exact project revision. Check the current Story list before continuing.';

  @override
  String managedStoryWorkspaceCheckpointSummary(int count, int revision) {
    return 'NPC and Quest drafts: $count · project revision $revision';
  }

  @override
  String managedStoryWorkspaceLoadErrorDetails(String error) {
    return 'The exact current Story view could not be read: $error';
  }

  @override
  String managedStoryWorkspaceCreateErrorDetails(String error) {
    return 'The Story draft could not be created: $error';
  }

  @override
  String managedStoryWorkspaceDetailsSheetLabel(String entityName) {
    return '$entityName Story details';
  }

  @override
  String get managedStoryWorkspaceRemovePairUnavailable =>
      'This draft is not an exact removable draft and generated-script pair.';

  @override
  String get managedStoryWorkspaceRemoveBusy =>
      'Another Story action is still in progress.';

  @override
  String get managedStoryWorkspaceRemoveRequiresReopen =>
      'Reopen this managed project before removing a draft.';

  @override
  String managedStoryWorkspaceRemoveBlocked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count incoming project references must be removed first.',
      one: '1 incoming project reference must be removed first.',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkspaceRemoveDialogTitle =>
      'Remove draft from project?';

  @override
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  ) {
    return 'This removes the draft \'$draftName\' together with its uniquely owned generated script \'$scriptName\'.';
  }

  @override
  String get managedStoryWorkspaceRemoveNoUndo =>
      'This removal cannot be undone in version 1.';

  @override
  String get managedStoryWorkspaceRemoveBoundary =>
      'Only the current project registry is changed. The game installation and save games stay unchanged.';

  @override
  String get managedStoryWorkspaceRemoveCancel => 'Cancel';

  @override
  String get managedStoryWorkspaceRemoveConfirm => 'Remove draft';

  @override
  String get managedStoryWorkspaceRemoveBlockedTitle =>
      'Draft is still referenced';

  @override
  String get managedStoryWorkspaceRemoveBlockedDescription =>
      'Open every source below and remove its project reference before trying again.';

  @override
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  ) {
    return '$sourceName · $role';
  }

  @override
  String get managedStoryWorkspaceRemoveOpenBlocker =>
      'Open referencing source';

  @override
  String get managedStoryWorkspaceRemoveBlockedClose => 'Close';

  @override
  String managedStoryWorkspaceRemoveSucceeded(String draftName) {
    return 'Removed \'$draftName\' and its generated script from the project. Game files and save games were not changed.';
  }

  @override
  String managedStoryWorkspaceRemoveError(String error) {
    return 'The draft was not removed. The Story view was refreshed without retrying automatically: $error';
  }

  @override
  String get managedSectionWorldDescription =>
      'Rozmieszczanie w świecie i powiązane procesy są planowane.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Pisz i tłumacz dialogi projektu w jednym miejscu, a potem przejdź do pracy nad głosami.';

  @override
  String get managedLocalizationProjectTextsLabel => 'Project texts';

  @override
  String get managedLocalizationSearchLabel => 'Search project texts';

  @override
  String get managedLocalizationRefresh => 'Refresh';

  @override
  String get managedLocalizationEmptyTitle => 'No project text yet';

  @override
  String get managedLocalizationEmptyDescription =>
      'Create a dialog line to start writing and translating text.';

  @override
  String get managedLocalizationLoadFailed =>
      'Project texts could not be opened';

  @override
  String get managedLocalizationSelectText => 'Select a project text to edit';

  @override
  String get managedLocalizationLanguagesLabel => 'Languages';

  @override
  String get managedLocalizationUsedByLines => 'Used by dialog lines';

  @override
  String get managedLocalizationNoLine => 'Not used by a dialog line yet';

  @override
  String get managedLocalizationSpeakerLabel => 'Speaker label';

  @override
  String get managedLocalizationAddLanguage => 'Add language';

  @override
  String get managedLocalizationRemoveLanguage => 'Remove language';

  @override
  String get managedLocalizationLanguageHint => 'For example de, en, or pt-BR';

  @override
  String get managedLocalizationLanguageExists =>
      'This language is already present.';

  @override
  String get managedLocalizationAdd => 'Add';

  @override
  String get managedLocalizationSaved => 'Project text saved';

  @override
  String get managedLocalizationVoiceLocked =>
      'This text has recorded voice takes, so its transcript is locked in this editor.';

  @override
  String get managedLocalizationVoiceSlotRemovalLocked =>
      'This language is connected to a Voice slot and cannot be removed here.';

  @override
  String get managedLocalizationMinimumLanguageLocked =>
      'Keep at least one language for this project text.';

  @override
  String get managedLocalizationSharedNotice =>
      'This project text is shared. Saving changes updates every listed dialog line.';

  @override
  String get managedLocalizationOfflineNotice =>
      'Changes are saved only to this managed project. Build and in-game behavior remain separate.';

  @override
  String get managedLocalizationUnsavedTitle => 'Discard unsaved changes?';

  @override
  String get managedLocalizationUnsavedDescription =>
      'You changed this project text. Switching now would discard those edits.';

  @override
  String get managedLocalizationDiscard => 'Discard changes';

  @override
  String get managedLocalizationKeepEditing => 'Keep editing';

  @override
  String get managedLocalizationStale =>
      'The project changed while this text was open. Refresh and try again.';

  @override
  String get managedLocalizationReopen =>
      'The project must be reopened before text editing can continue.';

  @override
  String get managedLocalizationInvalid =>
      'Check that every language and dialog text is valid and not empty.';

  @override
  String get managedLocalizationSaveFailed =>
      'The project text could not be saved.';

  @override
  String get managedSectionValidateTestDescription =>
      'Sprawdza dokładną integralność projektu i punkty kontrolne; nie potwierdza testu w czasie działania.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Pakiety głosowe są dostępne; pełne grywalne kompilacje i wdrażanie nie są dostępne.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Ustawienia są dostępne; narzędzia eksperckie nie są jeszcze zintegrowane.';

  @override
  String get managedSectionStatusHeading => 'Stan';

  @override
  String get managedSectionActionsHeading => 'Działania';

  @override
  String get managedCapabilityAvailable => 'Dostępne';

  @override
  String get managedCapabilityPartial => 'Częściowe';

  @override
  String get managedCapabilityPlanned => 'Planowane';

  @override
  String get managedCapabilityUnavailable => 'Niedostępne';

  @override
  String get managedProjectSubtitle =>
      'Obszar tworzenia offline zgodny dokładnie z bieżącą wersją';

  @override
  String get managedProjectLandingTitle =>
      'Obszar roboczy zarządzanego projektu';

  @override
  String get managedProjectLandingDescription =>
      'Korzystaj z nowych sekcji: Strona główna, Zawartość, Fabuła, Głos, Walidacja i Wydanie — wszystko w jednym zarządzanym projekcie.';

  @override
  String get legacyCompatibilityToolsTitle => 'Starsze narzędzia zgodności';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Karty poniżej zawierają starsze narzędzia do bezpośredniego zastępowania. Pozostaną dostępne podczas rozbudowy obszaru zarządzanego projektu.';

  @override
  String get managedProjectTechnicalDetails => 'Szczegóły techniczne projektu';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Przed odczytaniem zawartości ponownie otwórz zarządzany projekt.';

  @override
  String get managedProjectRecoveryDescription =>
      'Mod Studio will safely reopen this project while keeping its lock. This does not change the game or any save.';

  @override
  String get managedProjectRecoveryTry => 'Try recovery';

  @override
  String get managedProjectRecoveryTrying => 'Trying recovery…';

  @override
  String get managedProjectRecoveryAlternative =>
      'If recovery does not work, close and open the project again.';

  @override
  String get managedProjectRecoverySucceeded =>
      'Project recovery completed. You can continue working.';

  @override
  String get managedProjectRecoveryFailed =>
      'Recovery did not complete. Try again, or close and open the project again.';

  @override
  String get managedProjectRecoveryUnavailable =>
      'Recovery is not available for this project. Close and open the project again.';

  @override
  String get managedDashboardUntitledProject => 'Projekt bez tytułu';

  @override
  String get managedDashboardDraftStatus => 'Szkic';

  @override
  String get managedDashboardProjectVersion => 'Wersja';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'Nie podano';

  @override
  String get managedDashboardContentCounts => 'Zawartość projektu';

  @override
  String get managedDashboardNpcDrafts => 'Szkice NPC';

  @override
  String get managedDashboardQuestDrafts => 'Szkice zadań';

  @override
  String get managedDashboardDialogLines => 'Kwestie dialogowe';

  @override
  String get managedDashboardVoiceTakes => 'Nagrania głosowe';

  @override
  String get managedDashboardAssets => 'Zasoby';

  @override
  String get managedDashboardUnresolvedReferences => 'Nierozwiązane odwołania';

  @override
  String get managedDashboardReadiness => 'Co już działa';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Tworzenie offline jest dostępne';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Twórz i edytuj obsługiwaną zawartość projektu bez zmieniania instalacji gry ani plików zapisu.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Ogólne budowanie moda jest niedostępne';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Można budować tylko zapieczętowane pakiety Voice offline; pełnego grywalnego moda nie można jeszcze zbudować.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Działanie w grze nie zostało jeszcze zweryfikowane';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio nie potwierdziło jeszcze działania tej zawartości projektu w uruchomionej grze.';

  @override
  String get managedDashboardReferenceIntegrityTitle => 'Spójność odwołań';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Ta liczba sprawdza tylko odwołania projektu; nie oznacza gotowości do budowania ani uruchomienia.';

  @override
  String get managedDashboardMissingGameTitle => 'Wymagana konfiguracja gry';

  @override
  String get managedDashboardMissingGameDescription =>
      'Skonfiguruj instalację Gothic 1 Remake w Ustawieniach przed użyciem działań wymagających zweryfikowanych danych z zainstalowanej gry.';

  @override
  String get managedDashboardCreateHeading => 'Utwórz';

  @override
  String get managedDashboardToolsHeading => 'Narzędzia projektu';

  @override
  String get managedDashboardLoading => 'Wczytywanie przeglądu projektu';

  @override
  String get managedDashboardLoadError => 'Przegląd projektu jest niedostępny';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Nie udało się wczytać zweryfikowanego przeglądu projektu. Zawartość projektu nie została zmieniona.';

  @override
  String get managedDashboardRetry => 'Spróbuj ponownie';

  @override
  String get managedActionNewNpcTitle => 'Nowy NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Utwórz ograniczony szkic NPC offline na podstawie zweryfikowanych danych z zainstalowanej gry.';

  @override
  String get managedActionNewQuestTitle => 'Nowe zadanie';

  @override
  String get managedActionNewQuestDescription =>
      'Utwórz szkic zadania offline z celami i zweryfikowanymi tożsamościami nadrzędnymi.';

  @override
  String get managedActionNewDialogLineTitle => 'Dodaj kwestię dialogową';

  @override
  String get managedActionNewDialogLineDescription =>
      'Napisz zlokalizowany tekst projektu lub połącz nieużywany tekst, który już znajduje się w tym projekcie. Nie tworzy to grywalnego tematu dialogowego.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Kwestię dialogową zapisano w rewizji projektu $projectRevision. Gra ani pliki zapisu nie zostały zmienione.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Napisz nową zlokalizowaną kwestię dialogową lub połącz tekst należący już do tego projektu.';

  @override
  String get managedDialogLineBoundary =>
      'Zmieniane są tylko pliki projektu. Nie tworzy to tematu AngelScript ani grywalnego dialogu i nigdy nie zmienia instalacji gry ani plików zapisu. Pole mówcy jest tylko etykietą i nie łączy z żadnym NPC.';

  @override
  String get managedDialogLineCreateMode => 'Napisz nowy tekst';

  @override
  String get managedDialogLineReuseMode => 'Użyj tekstu projektu';

  @override
  String get managedDialogLineNameLabel => 'Nazwa kwestii';

  @override
  String get managedDialogLineNameHint => 'Powitanie przy wejściu do kopalni';

  @override
  String get managedDialogLineSpeakerLabel => 'Etykieta mówcy (opcjonalnie)';

  @override
  String get managedDialogLineSpeakerHint => 'Na przykład Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Język';

  @override
  String get managedDialogLineTextLabel => 'Tekst dialogu';

  @override
  String get managedDialogLineReuseSearch =>
      'Szukaj nieużywanego tekstu projektu';

  @override
  String get managedDialogLineNoReusableText =>
      'Brak nieużywanego, strukturalnie poprawnego tekstu projektu, który można połączyć. Zamiast tego napisz nowy tekst.';

  @override
  String get managedDialogLineCreateSlotLabel => 'Przygotuj ten język do Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Tworzy pusty, nierozwiązany slot Voice w projekcie. Nie dodaje ani nie wdraża nagrania.';

  @override
  String get managedDialogLineCancel => 'Anuluj';

  @override
  String get managedDialogLineSave => 'Zapisz w projekcie';

  @override
  String get managedDialogLineSaving => 'Zapisywanie…';

  @override
  String get managedDialogLineLoading =>
      'Odczytywanie dokładnej zawartości projektu…';

  @override
  String get managedDialogLineLoadFailed =>
      'Nie udało się odczytać dokładnej bieżącej zawartości projektu. Niczego nie zmieniono.';

  @override
  String get managedDialogLineRetry => 'Spróbuj ponownie';

  @override
  String get managedDialogLineStale =>
      'Projekt zmienił się, gdy to okno było otwarte. Zamknij je i spróbuj ponownie z bieżącego projektu.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Nie można już bezpiecznie zweryfikować bieżącego projektu. Zamknij to okno i ponownie otwórz zarządzany projekt.';

  @override
  String get managedDialogLineInvalidInput =>
      'Sprawdź wyróżnione dane projektu i wybierz dokładną bieżącą opcję.';

  @override
  String get managedDialogLineSaveFailed =>
      'Nie udało się bezpiecznie zapisać kwestii dialogowej. Gra ani pliki zapisu nie zostały zmienione.';

  @override
  String get managedDialogLineDone => 'Gotowe';

  @override
  String get managedDialogLineAddRecording => 'Dodaj nagranie';

  @override
  String get managedActionAddVoiceTakeTitle => 'Dodaj nagranie głosowe';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Zaimportuj nagranie Ogg Vorbis do tego projektu bez jego wdrażania.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle =>
      'Zarządzaj nagraniami głosowymi';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Przejrzyj nagrania i wybierz zatwierdzone wersje dla miejsc Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Ustal cel Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Dopasuj miejsca Voice projektu do dokładnych elementów zainstalowanych archiwów bez zmieniania gry.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Zbuduj pakiet Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Zbuduj zapieczętowany pakiet offline z istniejących elementów; wdrażanie nie zostanie wykonane.';

  @override
  String get managedActionDataAssetsTitle => 'Edycja DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Sprawdź zainstalowane pakiety i przygotuj w projekcie zweryfikowane zmiany wartości o stałej szerokości.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Przeglądaj dokładną zawartość projektu oraz powiązane z nią rozpoznane i nierozpoznane odwołania.';

  @override
  String get managedActionSettingsTitle => 'Ustawienia';

  @override
  String get managedActionSettingsDescription =>
      'Skonfiguruj instalację Gothic 1 Remake i preferencje Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Projekt $projectId został bezpiecznie utworzony, ale konfiguracja startowa nie otworzyła się. Prawidłowy pusty projekt pozostaje aktywny.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Projekt $projectId został utworzony, ale Mod Studio nie może potwierdzić wyniku konfiguracji startowej. Przed kontynuowaniem otwórz ponownie zarządzany projekt; gra i zapisy nie zostały zmienione.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Projekt $projectId został utworzony. Start NPC nie został dodany, więc prawidłowy pusty projekt pozostaje aktywny.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Start NPC zapisano w rewizji $projectRevision. Nadal jest zablokowany dla kompilacji, niezweryfikowany w czasie działania i nie zostaje utworzony w świecie.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Projekt $projectId został utworzony. Start zadania nie został dodany, więc prawidłowy pusty projekt pozostaje aktywny.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Start zadania zapisano w rewizji $projectRevision. Nadal jest zablokowany dla kompilacji i niezweryfikowany w czasie działania.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Początek projektu';

  @override
  String get projectStarterPrompt => 'Jak chcesz zacząć?';

  @override
  String get projectStarterWriteBoundary =>
      'Wybór wariantu startowego niczego nie zapisuje. Projekt powstaje dopiero po wysłaniu formularza i wybraniu pustego folderu.';

  @override
  String get projectStarterEmptyTitle => 'Pusty projekt';

  @override
  String get projectStarterEmptyDescription =>
      'Utwórz tylko zarządzany projekt. Zawartość możesz dodać później.';

  @override
  String get projectStarterNpcDraftTitle => 'Szkic NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Najpierw utwórz pusty projekt, a następnie otwórz prowadzoną konfigurację szkicu NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Szkic zadania';

  @override
  String get projectStarterQuestDraftDescription =>
      'Najpierw utwórz pusty projekt, a następnie otwórz prowadzoną konfigurację szkicu zadania.';

  @override
  String get projectStarterPartialOutcome =>
      'Anulowanie prowadzonej konfiguracji NPC lub zadania albo błąd szkicu pozostawia prawidłowy pusty projekt. Wybór nie zapisuje niczego w grze ani w zapisie.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Przeglądaj';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Zweryfikowane zmiany';

  @override
  String get managedContentScopeBaseGameLabel => 'Gra podstawowa';

  @override
  String get managedContentScopeInstalledLabel => 'Zainstalowane';

  @override
  String get managedBaseGameBrowserTitle =>
      'Obsługiwane punkty startowe gry podstawowej';

  @override
  String get managedBaseGameBrowserDescription =>
      'Przeglądaj dokładne dane zainstalowanej gry, które Mod Studio może sprawdzić lub użyć jako bezpiecznego początku szkicu. Nie jest to pełny katalog oryginalnej zawartości.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Odczytywanie dokładnych danych gry podstawowej…';

  @override
  String get managedBaseGameBrowserRefresh => 'Odczytaj nowy dokładny katalog';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Szukaj obsługiwanej zawartości gry podstawowej';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Zadania';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'Punkty startowe NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => 'Punkty startowe zadań';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Archetypy NPC tylko do inspekcji';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Wyszukaj, aby uwzględnić szersze statycznie powiązane dane NPC. Te wiersze nie umożliwiają utworzenia szkicu.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Brak obsługiwanego wyniku gry podstawowej pasującego do wyszukiwania.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Dane gry podstawowej są niedostępne';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Nie udało się odczytać dokładnego obsługiwanego katalogu. Nie zmieniono plików projektu, gry ani zapisów.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Obsługa szkicu offline';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Tylko inspekcja';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'Użyj jako początku NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Użyj jako początku zadania';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Definicja tworzenia';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint aktora';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Wyświetlono pierwsze 100 wyników tylko do inspekcji. Zawęź wyszukiwanie, aby uzyskać dokładniejsze wyniki.';

  @override
  String get managedInstalledBrowserLoading =>
      'Odczytywanie dokładnego spisu zainstalowanych pakietów…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return 'Zainstalowane pakiety kandydujące: $count';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return 'Zainstalowane pakiety kandydujące: $count — wynik częściowy';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Odczytano metadane katalogu, a zainstalowana migawka pozostała dokładna.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Brakowało części metadanych pakietów lub nie były kanoniczne; wyniki pomagają w odkrywaniu, ale nie są kompletne.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Ten zakres pokazuje wyłącznie metadane zainstalowanych pakietów DataAsset. Inspekcja lub kopiowanie ścieżki nie daje uprawnień do kompilacji, wdrażania, działania ani zapisu w grze.';

  @override
  String get managedInstalledBrowserRefresh => 'Odczytaj nową dokładną migawkę';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Szukaj zainstalowanych DataAssets';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nazwa zasobu lub ścieżka /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Wpisz nazwę zasobu lub ścieżkę /Game do wyszukania.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Brak pasującego zainstalowanego DataAsset';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Spróbuj innej nazwy zasobu lub szerszej ścieżki /Game.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Wyświetlono pierwsze 100 wyników. Zawęź wyszukiwanie, aby ograniczyć dokładną migawkę.';

  @override
  String get managedInstalledBrowserKindBadge => 'Pakiet DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Tylko metadane';

  @override
  String get managedInstalledBrowserOpenInspector => 'Sprawdź dokładny pakiet';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Spis zainstalowanych pakietów jest niedostępny';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Nie udało się odczytać dokładnej zainstalowanej migawki. Nie zmieniono plików projektu, gry ani zapisów.';

  @override
  String get managedGlobalSearchScopeLabel => 'Przeszukaj wszystko';

  @override
  String get managedGlobalSearchTitle => 'Przeszukaj całą zawartość';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, zadanie, kwestia, zasób, ID lub ścieżka /Game';

  @override
  String get managedGlobalSearchAction => 'Szukaj';

  @override
  String get managedGlobalSearchClear => 'Wyczyść';

  @override
  String get managedGlobalSearchPrompt =>
      'Wpisz zapytanie, aby niezależnie odczytać trzy źródła.';

  @override
  String get managedGlobalSearchNoResults => 'Brak wyników w tym źródle.';

  @override
  String get managedGlobalSearchLoading => 'Odczytywanie dokładnego źródła…';

  @override
  String get managedGlobalSearchFailed => 'Nie udało się odczytać tego źródła.';

  @override
  String get managedGlobalSearchComplete => 'Kompletne';

  @override
  String get managedGlobalSearchPartial => 'Częściowe';

  @override
  String get managedGlobalSearchTruncated =>
      'Wyświetlono pierwsze 100 wyników. Zawęź wyszukiwanie.';

  @override
  String get managedGlobalSearchOpen => 'Otwórz';

  @override
  String get managedGlobalSearchCreateDraft => 'Utwórz szkic';

  @override
  String get managedGlobalSearchInspect => 'Sprawdź';

  @override
  String get managedGlobalSearchKindModEntity => 'Zawartość moda';

  @override
  String get managedGlobalSearchKindModAsset => 'Zasób moda';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Punkt wyjścia NPC';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Punkt wyjścia zadania';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Dowód dotyczący NPC';

  @override
  String get managedGlobalSearchReadinessExact => 'Dokładny bieżący projekt';

  @override
  String get managedGlobalSearchReadinessProblems => 'Dokładne, z problemami';

  @override
  String get managedGlobalSearchResultStale =>
      'Tego wyniku nie ma już w bieżącym projekcie. Wyszukaj ponownie.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Tylko wersja robocza';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Kompilacja zablokowana';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Uruchomienie niezweryfikowane';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Przegląd';

  @override
  String get managedStoryWorkbenchProfileTab => 'Profil';

  @override
  String get managedStoryWorkbenchStoryTab => 'Fabuła';

  @override
  String get managedStoryWorkbenchLogicTab => 'Logika';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Rutyna';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Ekwipunek';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Dialogi i głosy';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Odwołania';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => 'Problemy i kontrole';

  @override
  String get managedStoryWorkbenchEditOverview => 'Edytuj nazwę i cele';

  @override
  String get managedStoryWorkbenchEditStory => 'Edytuj opis i powiązania';

  @override
  String get managedStoryWorkbenchEditLogic => 'Edytuj stany i przejścia';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Otwórz kod źródłowy i kontrole kompilatora';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Otwórz profil i kontrole kompilatora';

  @override
  String get managedStoryWorkbenchMoreActions => 'More actions';

  @override
  String get managedStoryWorkbenchRemoveDraft => 'Remove draft…';

  @override
  String get managedStoryWorkbenchRemovingDraft => 'Removing draft…';

  @override
  String get managedStoryWorkbenchReviewRemovalBlockers =>
      'Review removal blockers';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable =>
      'Jeszcze nie zamodelowano';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Relacje z zadaniami i fabułą nie są jeszcze zamodelowane dla wersji roboczych NPC.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'Rutyny i rozmieszczenie w świecie nie są jeszcze zamodelowane.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'Ekwipunek, wyposażenie i handel nie są jeszcze zamodelowane.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Relacje z dialogami, lokalizacją i głosami nie są jeszcze zamodelowane dla wersji roboczych NPC.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Relacje z dialogami, lokalizacją i głosami nie są jeszcze zamodelowane dla wersji roboczych zadań.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Brak nierozwiązanych odwołań w projekcie';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count nierozwiązanego odwołania w projekcie',
      many: '$count nierozwiązanych odwołań w projekcie',
      few: '$count nierozwiązane odwołania w projekcie',
      one: '1 nierozwiązane odwołanie w projekcie',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'To tylko stan odwołań; nie oznacza gotowości do kompilacji ani uruchomienia.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Szczegóły techniczne';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Wersja robocza zadania';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Wersja robocza NPC';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Tytuł zadania';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel =>
      'Identyfikator techniczny';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Cele';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Unikatowa nazwa';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Przestrzeń nazw modułu';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Zleceniodawca zadania';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Klasa bazowa w czasie działania';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Stany cyklu życia zadania, wyzwalacze, warunki i efekty są edytowane jako jedna atomowa operacja na dokładnym bieżącym stanie.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Wychodzące';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Brak przewidywanych odwołań';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Przychodzące';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Brak przychodzących odwołań w projekcie';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Tożsamość semantyczna';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Pochodzenie';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Rewizja encji';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'Stabilny identyfikator';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Odwołanie rozwiązane';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Odwołanie nierozwiązane';

  @override
  String get managedProblemsTitle => 'Problems & readiness';

  @override
  String get managedProblemsDescription =>
      'See what needs attention and open the exact affected project content.';

  @override
  String get managedProblemsScopeNotice =>
      'Every status covers only its named scope. A clear reference check does not mean the mod can be built or tested in-game.';

  @override
  String get managedProblemsRefresh => 'Refresh problems';

  @override
  String get managedProblemsPartialTitle => 'Some checks are unavailable';

  @override
  String get managedProblemsDataAssetsUnavailable =>
      'DataAsset edits could not be checked. Other exact project findings are still shown.';

  @override
  String get managedProblemsOverviewHeading => 'Readiness by area';

  @override
  String get managedProblemsSearchLabel => 'Search problems';

  @override
  String get managedProblemsClearSearch => 'Clear problem search';

  @override
  String get managedProblemsListHeading => 'Problems';

  @override
  String get managedProblemsEmptyTitle =>
      'No modeled structural problems found';

  @override
  String get managedProblemsEmptyDescription =>
      'The exact checks currently modeled by Mod Studio found nothing to repair.';

  @override
  String get managedProblemsEmptyBoundary =>
      'Compiler evidence was not evaluated, the full managed build is unavailable, and runtime behavior remains unqualified.';

  @override
  String get managedProblemsFilteredEmptyTitle => 'No matching problems';

  @override
  String get managedProblemsFilteredEmptyDescription =>
      'Change the search or category filter to see other findings.';

  @override
  String get managedProblemsSelectTitle => 'Select a problem';

  @override
  String get managedProblemsSelectDescription =>
      'Choose a finding to see what it means and the safest available next action.';

  @override
  String get managedProblemsDetailHeading => 'Problem details';

  @override
  String get managedProblemsCloseDetail => 'Close problem details';

  @override
  String get managedProblemsCategoryLabel => 'Area';

  @override
  String get managedProblemsSeverityLabel => 'Attention';

  @override
  String get managedProblemsSourceLabel => 'Evidence';

  @override
  String get managedProblemsOpenSourceEntity => 'Open source content';

  @override
  String get managedProblemsOpenReferencedAsset => 'Open referenced asset';

  @override
  String get managedProblemsOpenDataAssetEdits => 'Open DataAsset edits';

  @override
  String get managedProblemsActionFailed =>
      'The exact target could not be opened. Refresh the project problems and try again.';

  @override
  String get managedProblemsActionProgress =>
      'Opening the exact project target';

  @override
  String get managedProblemsCategoryReferences => 'References';

  @override
  String get managedProblemsCategorySetup => 'Setup';

  @override
  String get managedProblemsCategoryDataAssets => 'DataAssets';

  @override
  String get managedProblemsSeverityInformation => 'Information';

  @override
  String get managedProblemsSeverityWarning => 'Needs attention';

  @override
  String get managedProblemsSeverityBlocking => 'Blocks this scope';

  @override
  String get managedProblemsScopeReferencesTitle => 'Reference integrity';

  @override
  String get managedProblemsScopeReferencesDescription =>
      'Checks exact links between current project content and assets.';

  @override
  String get managedProblemsScopeDataAssetsTitle => 'DataAsset edit registry';

  @override
  String get managedProblemsScopeDataAssetsDescription =>
      'Checks whether the exact current list of saved DataAsset edits could be read.';

  @override
  String get managedProblemsScopeGameTitle => 'Game setup';

  @override
  String get managedProblemsScopeGameDescription =>
      'Shows whether a game installation is configured for bounded read-only tools.';

  @override
  String get managedProblemsScopeCompilerTitle => 'Source & compiler evidence';

  @override
  String get managedProblemsScopeCompilerDescription =>
      'Compiler checks run only when you explicitly open and start them for one exact entity.';

  @override
  String get managedProblemsScopeBuildTitle => 'Managed project build';

  @override
  String get managedProblemsScopeBuildDescription =>
      'A complete build path for managed NPC, Quest, dialog, and DataAsset edits is not available yet.';

  @override
  String get managedProblemsScopeRuntimeTitle => 'In-game behavior';

  @override
  String get managedProblemsScopeRuntimeDescription =>
      'No general runtime, save, deployment, or cleanup qualification is claimed.';

  @override
  String get managedProblemsReadinessClear => 'Checked within this scope';

  @override
  String get managedProblemsReadinessIssues => 'Needs attention';

  @override
  String get managedProblemsReadinessUnavailable => 'Check unavailable';

  @override
  String get managedProblemsReadinessNotEvaluated => 'Not evaluated';

  @override
  String get managedProblemsReadinessBlocked => 'Build path unavailable';

  @override
  String get managedProblemsReadinessUnqualified => 'Runtime unqualified';

  @override
  String get managedProblemsEvidenceContent => 'Exact current project content';

  @override
  String get managedProblemsEvidenceDataAssets =>
      'Exact current DataAsset registry';

  @override
  String get managedProblemsEvidenceConfiguration =>
      'Current app configuration';

  @override
  String get managedProblemsEvidenceUnavailable =>
      'Evidence source unavailable';

  @override
  String get managedProblemsEvidenceBoundary => 'Known capability boundary';

  @override
  String get managedProblemsForeignReferenceTitle =>
      'Reference points to another project';

  @override
  String get managedProblemsMissingEntityTitle =>
      'Linked project content is missing';

  @override
  String get managedProblemsEntityKindTitle =>
      'Linked project content has the wrong type';

  @override
  String get managedProblemsMissingAssetTitle =>
      'Linked project file is missing';

  @override
  String get managedProblemsAssetLengthTitle =>
      'Linked project file has an unexpected size';

  @override
  String get managedProblemsAssetTypeTitle =>
      'Linked project file has an unexpected type';

  @override
  String get managedProblemsGameSetupTitle =>
      'Game installation is not configured';

  @override
  String get managedProblemsDataAssetRegistryTitle =>
      'DataAsset edits could not be checked';

  @override
  String get managedProblemsDataAssetOfflineTitle =>
      'DataAsset edit is draft-only';

  @override
  String managedProblemsEntityReferenceDescription(String source) {
    return 'Open $source and repair this exact project-content link.';
  }

  @override
  String managedProblemsAssetReferenceDescription(String source) {
    return 'Open $source and repair this exact project-file link.';
  }

  @override
  String get managedProblemsDataAssetRegistryDescription =>
      'Refresh the exact current project. No conclusion is drawn about saved DataAsset edits until this source is available.';

  @override
  String managedProblemsDataAssetOfflineDescription(String targetPath) {
    return 'The saved edit for $targetPath can be reviewed in DataAsset edits, but it cannot be emitted by a managed project build or claimed as working in-game yet.';
  }

  @override
  String get projectExportActionTitle => 'Export project copy…';

  @override
  String get projectExportActionDescription =>
      'Write an exact portable copy of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before exporting a project copy.';

  @override
  String get projectExportDialogTitle => 'Export project copy';

  @override
  String get projectExportPortableCopyTitle => 'Portable project copy';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. The open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This copy is not a playable mod, build, deployment, or restorable backup. It does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'Importing this managed copy is not available yet. Keep the original project folder.';

  @override
  String get projectExportFileNameLabel => 'New project-copy file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new portable file name ending in .goremod.';

  @override
  String get projectExportChooseDestination => 'Choose destination folder';

  @override
  String get projectExportNoDestination => 'No destination folder selected';

  @override
  String get projectExportNewFile => 'New file';

  @override
  String get projectExportCancel => 'Cancel';

  @override
  String get projectExportClose => 'Close';

  @override
  String get projectExportSubmit => 'Export copy';

  @override
  String get projectExportExporting => 'Exporting…';

  @override
  String get projectExportParentRequired =>
      'Choose an existing destination folder.';

  @override
  String get projectExportParentAbsolute =>
      'Choose an absolute existing destination folder.';

  @override
  String get projectExportParentLink =>
      'The selected destination is a link. Choose a real existing folder.';

  @override
  String get projectExportParentInspectFailed =>
      'The destination folder could not be inspected safely. Nothing was created.';

  @override
  String get projectExportFileNameRequired =>
      'Enter a new project-copy file name.';

  @override
  String get projectExportFileNameTooLong =>
      'The file name must be at most 128 ASCII characters.';

  @override
  String get projectExportFileNameInvalid =>
      'Start with a letter or digit, use only ASCII letters, digits, dots, underscores, or hyphens, and end with .goremod.';

  @override
  String get projectExportFileNameReserved =>
      'That file name is reserved by Windows.';

  @override
  String get projectExportOutputExists =>
      'That file already exists. Choose a new file name; existing files are never overwritten.';

  @override
  String get projectExportOutputLink =>
      'The new file path is a link. Choose a different file name.';

  @override
  String get projectExportOutputRejected =>
      'The destination was rejected before the new local file was created. Nothing was created. Choose a different file name or destination folder.';

  @override
  String get projectExportStale =>
      'The project changed before export started. No output was created. Close this window and open Export project copy again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot export exact portable copies. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project copy could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Export stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'The export did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed export does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact portable project copy was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact project copy was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

  @override
  String projectExportPublicationUncertain(String output) {
    return 'The local file may have been created. Do not retry. Check whether this destination exists: $output';
  }

  @override
  String get projectExportArchiveBytes => 'Archive bytes';

  @override
  String get projectExportArchiveSha256 => 'Archive SHA-256';

  @override
  String get projectExportCurrentProjectUnchanged =>
      'The current project remains open and unchanged. The game and saves were not touched.';
}
