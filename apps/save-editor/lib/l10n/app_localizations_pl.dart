// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class AppLocalizationsPl extends AppLocalizations {
  AppLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get appTitle => 'Edytor zapisów gry Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'Logo goresave';

  @override
  String get zoomTooltip => 'Naciśnij Ctrl +/-, aby przybliżyć lub oddalić';

  @override
  String get switchToLightMode => 'Przełącz na tryb jasny';

  @override
  String get switchToDarkMode => 'Przełącz na tryb ciemny';

  @override
  String get about => 'O programie';

  @override
  String get tabOverview => 'Przegląd';

  @override
  String get tabPlayer => 'Postać';

  @override
  String get tabAttribute => 'Atrybuty';

  @override
  String get tabInventory => 'Ekwipunek';

  @override
  String get tabWorld => 'Świat';

  @override
  String get tabCharacters => 'Postacie';

  @override
  String get characterNoActorBody =>
      'Ta postać nie ma aktora w świecie, więc nie ma atrybutów, ekwipunku ani zdarzeń.';

  @override
  String get characterNoEventsBody => 'Brak zdarzeń dla tej postaci.';

  @override
  String get characterOrphanGroup => 'Inne';

  @override
  String get tabAllData => 'Wszystkie dane';

  @override
  String get tabBackups => 'Kopie zapasowe';

  @override
  String get tabSettings => 'Ustawienia';

  @override
  String get reset => 'Resetuj';

  @override
  String get save => 'Zapisz';

  @override
  String saveWithCount(int count) {
    return 'Zapisz ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Anuluj';

  @override
  String get confirm => 'Potwierdź';

  @override
  String get close => 'Zamknij';

  @override
  String get add => 'Dodaj';

  @override
  String get equippedBadge => 'Założone';

  @override
  String get armorUpgradesLabel => 'Ulepszenia';

  @override
  String get browse => 'Przeglądaj';

  @override
  String get noSavFilesFound => 'Nie znaleziono plików .sav';

  @override
  String get profile => 'Profil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name (zapisy: $count)';
  }

  @override
  String get switchProfile => 'Zmień profil';

  @override
  String get rescanSaveFolder => 'Przeskanuj folder zapisów ponownie';

  @override
  String get discardUnsavedChangesTitle => 'Odrzucić niezapisane zmiany?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Twoje niezapisane zmiany ($count)',
      one: 'Twoją $count niezapisaną zmianę',
    );
    return 'Ponowne skanowanie przeładuje wszystkie zapisy i odrzuci $_temp0.';
  }

  @override
  String get discardAndRescan => 'Odrzuć i przeskanuj ponownie';

  @override
  String chapterLabel(Object id) {
    return 'Rozdział $id';
  }

  @override
  String get quickSave => 'Szybki zapis';

  @override
  String get autoSave => 'Autozapis';

  @override
  String get manualSave => 'Zapis ręczny';

  @override
  String get errorTitle => 'Błąd';

  @override
  String get selectASaveTitle => 'Wybierz zapis';

  @override
  String get selectASaveBody => 'Szczegóły zapisu pojawią się tutaj.';

  @override
  String get diagnosticsTitle => 'Diagnostyka i szczegóły';

  @override
  String get diagnosticsSubtitle => 'Podgląd formatu (tylko do odczytu)';

  @override
  String get metricFormat => 'Format';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Rozdział';

  @override
  String get metricTimePlayed => 'Czas gry';

  @override
  String get metricSaveKind => 'Rodzaj zapisu';

  @override
  String get metricFileSize => 'Rozmiar pliku';

  @override
  String get metricCompression => 'Kompresja';

  @override
  String get metricChunks => 'Fragmenty';

  @override
  String get metricUncompressed => 'Nieskompresowane';

  @override
  String get metricPrivate => 'Prywatne';

  @override
  String get metricSlotName => 'Nazwa slotu';

  @override
  String get metricTrailer => 'Stopka';

  @override
  String get metricDecodedPrivate => 'Odkodowane prywatne';

  @override
  String get metricPrivateStrings => 'Ciągi prywatne';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count B';
  }

  @override
  String get inspectionJsonTitle => 'JSON inspekcji';

  @override
  String get inspectionJsonSubtitle => 'Surowe dane inspekcji zapisu';

  @override
  String get copy => 'Kopiuj';

  @override
  String get savegameFallbackTitle => 'Zapis gry';

  @override
  String screenshotForSlot(String slot) {
    return 'Zrzut ekranu dla $slot';
  }

  @override
  String get publicSaveName => 'Publiczna nazwa zapisu';

  @override
  String get gameTimeTitle => 'Game time';

  @override
  String get gameTimeDay => 'Day';

  @override
  String get gameTimeHours => 'Hours';

  @override
  String get gameTimeMinutes => 'Minutes';

  @override
  String get gameTimeSeconds => 'Seconds';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s total';
  }

  @override
  String get gameTimeInvalid =>
      'Enter whole numbers — day ≥ 0, hours 0–23, minutes and seconds 0–59.';

  @override
  String get required => 'Wymagane';

  @override
  String get playerLockedBody =>
      'Edycja prywatnych danych postaci wymaga kodeka obsługującego kompresję.';

  @override
  String get heroTransform => 'Pozycja bohatera';

  @override
  String get locationX => 'Pozycja X';

  @override
  String get locationY => 'Pozycja Y';

  @override
  String get locationZ => 'Pozycja Z';

  @override
  String get rotationPitch => 'Pochylenie';

  @override
  String get rotationYaw => 'Odchylenie';

  @override
  String get rotationRoll => 'Przechylenie';

  @override
  String get invalid => 'Nieprawidłowe';

  @override
  String get heroAttributes => 'Atrybuty bohatera';

  @override
  String attributeBase(String name) {
    return '$name – bazowa';
  }

  @override
  String attributeCurrent(String name) {
    return '$name – bieżąca';
  }

  @override
  String get inventoryTitle => 'Ekwipunek';

  @override
  String get inventoryEmpty => 'Ten ekwipunek jest pusty.';

  @override
  String get inventoryNeedsDecoded =>
      'Edycja ekwipunku wymaga odkodowanych prywatnych danych z kodeka.';

  @override
  String get inventoryNoStacks =>
      'Nie znaleziono stosów przedmiotów w odkodowanych danych prywatnych.';

  @override
  String get resetInventoryChanges => 'Resetuj zmiany w ekwipunku';

  @override
  String get addItemTooltipPendingAdd =>
      'Najpierw zapisz oczekujące zmiany — jeden nowy przedmiot na zapis';

  @override
  String get addItemTooltipPendingRemove =>
      'Najpierw zapisz oczekujące usunięcie — jedna zmiana strukturalna na zapis';

  @override
  String get addItemTooltipPendingCount =>
      'Najpierw zapisz lub zresetuj oczekujące zmiany liczby — zmianę strukturalną trzeba zapisać osobno';

  @override
  String get addItemTooltipDefault => 'Dodaj przedmiot do ekwipunku';

  @override
  String get addItemButton => 'Dodaj przedmiot';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — oczekujące dodanie (jeszcze niezapisane)';
  }

  @override
  String get cancelPendingAdd => 'Anuluj oczekujące dodanie';

  @override
  String get pendingRemovalSubtitle =>
      'oczekujące usunięcie (jeszcze niezapisane)';

  @override
  String get cancelPendingRemoval => 'Anuluj oczekujące usunięcie';

  @override
  String get filterItems => 'Filtruj przedmioty';

  @override
  String noItemsMatchQuery(String query) {
    return 'Żaden przedmiot nie pasuje do „$query”.';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Oczekujące usunięcie ukrywa wszystkie przedmioty — zapisz, aby je zastosować.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Broń biała';

  @override
  String get itemCategoryRangedWeapon => 'Broń dystansowa';

  @override
  String get itemCategoryAmmunition => 'Amunicja';

  @override
  String get itemCategoryArmor => 'Zbroje';

  @override
  String get itemCategoryRune => 'Runy';

  @override
  String get itemCategoryScroll => 'Zwoje zaklęć';

  @override
  String get itemCategoryFood => 'Jedzenie i mikstury';

  @override
  String get itemCategoryMisc => 'Różne';

  @override
  String get itemCategoryAmulet => 'Amulety';

  @override
  String get itemCategoryRing => 'Pierścienie';

  @override
  String get itemCategoryTrophy => 'Trofea ze zwierząt';

  @override
  String get itemCategoryWriting => 'Pisma';

  @override
  String get itemCategoryMission => 'Przedmioty zadań';

  @override
  String get itemCategoryKey => 'Klucze';

  @override
  String get itemCategoryOther => 'Inne';

  @override
  String get count => 'Liczba';

  @override
  String get min1 => 'Min. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Nie można usunąć: ten przedmiot jest prawdopodobnie założony lub przypisany do slotu skrótu';

  @override
  String get removeBlockedTooltip =>
      'Najpierw zapisz lub zresetuj oczekujące zmiany w ekwipunku — dodanie lub usunięcie trzeba zapisać osobno';

  @override
  String get removeItemFromInventory => 'Usuń przedmiot z ekwipunku';

  @override
  String get progressionLockedBody =>
      'Dane postępów wymagają odkodowanych prywatnych danych z kodeka.';

  @override
  String get progressionNeedsTyped =>
      'Uporządkowane dane postępów wymagają w pełni odkodowanego zapisu ze zweryfikowaną analizą typowaną.';

  @override
  String get sectionQuests => 'Zadania';

  @override
  String get sectionKnowledge => 'Wiedza';

  @override
  String get sectionEvents => 'Zdarzenia';

  @override
  String get firstPage => 'Pierwsza strona';

  @override
  String get previousPage => 'Poprzednia strona';

  @override
  String get nextPage => 'Następna strona';

  @override
  String get lastPage => 'Ostatnia strona';

  @override
  String pageOfPages(int page, int total) {
    return 'Strona $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last z $total';
  }

  @override
  String get perPage => 'Na stronę:';

  @override
  String get resetQuestChanges => 'Resetuj zmiany w zadaniach';

  @override
  String get searchQuests => 'Szukaj zadań';

  @override
  String get allGroups => 'Wszystkie grupy';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Brak';

  @override
  String get questStateAvailable => 'Dostępne';

  @override
  String get questStateRunning => 'W toku';

  @override
  String get questStateSucceeded => 'Ukończone';

  @override
  String get questStateFailed => 'Nieudane';

  @override
  String get questStateUnknown => 'nieznany';

  @override
  String get dialogKnowledge => 'Wiedza z dialogów';

  @override
  String get resetKnowledgeChanges => 'Resetuj zmiany w wiedzy';

  @override
  String get addNpc => 'Dodaj NPC';

  @override
  String get searchNpcs => 'Szukaj NPC';

  @override
  String get npcStatusRowLabel => 'Stan';

  @override
  String get npcStatusAlive => 'żywy';

  @override
  String get npcStatusDead => 'martwy';

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PŻ $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Wskrześ';

  @override
  String get npcReviveQueued => 'Zostanie wskrzeszony przy zapisie';

  @override
  String entriesForCharacter(String name) {
    return 'Wpisy — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Wybierz NPC, aby zobaczyć wpisy';

  @override
  String get addKnowledgeEntry => 'Dodaj wpis wiedzy';

  @override
  String get browseCatalog => 'Przeglądaj katalog';

  @override
  String get alreadyExistsForCharacter => 'Już istnieje dla tej postaci.';

  @override
  String get alreadyInPendingChanges =>
      'Już znajduje się w oczekujących zmianach.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Sprawdzenie duplikatów nie powiodło się — spróbuj ponownie: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Oczekujące dodania ($count)';
  }

  @override
  String get undoAdd => 'Cofnij dodanie';

  @override
  String get undoRemove => 'Cofnij usunięcie';

  @override
  String get removeEntry => 'Usuń wpis';

  @override
  String get selectNpcFromList => 'Wybierz NPC z listy';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Zdarzenia z pamięci';

  @override
  String get searchCharacters => 'Szukaj postaci';

  @override
  String eventsForCharacter(String name) {
    return 'Zdarzenia — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Wybierz postać, aby zobaczyć zdarzenia';

  @override
  String get noTags => '(brak tagów)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Usuń zdarzenie';

  @override
  String get removeMemoryEventTitle => 'Usunąć zdarzenie z pamięci?';

  @override
  String get removeMemoryEventBody =>
      'Usunąć to zdarzenie z pamięci? Najpierw zostanie utworzona kopia zapasowa.';

  @override
  String get duplicateEvent => 'Powiel zdarzenie';

  @override
  String get duplicateMemoryEventTitle => 'Powielić zdarzenie z pamięci?';

  @override
  String get duplicateMemoryEventBody =>
      'Powielić to zdarzenie z pamięci? Najpierw zostanie utworzona kopia zapasowa.';

  @override
  String get selectCharacterFromList => 'Wybierz postać z listy';

  @override
  String get factionsSidebar => 'Frakcje';

  @override
  String get factionsForgiveButton => 'Ułaskaw';

  @override
  String get factionHostile => 'Wrogo';

  @override
  String get factionFriendly => 'Przyjaźnie';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count morderstwa',
      many: '$count morderstw',
      few: '$count morderstwa',
      one: '$count morderstwo',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count napaści',
      many: '$count napaści',
      few: '$count napaści',
      one: '$count napaść',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count kradzieży',
      many: '$count kradzieży',
      few: '$count kradzieże',
      one: '$count kradzież',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count wtargnięcia',
      many: '$count wtargnięć',
      few: '$count wtargnięcia',
      one: '$count wtargnięcie',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count groźby',
      many: '$count gróźb',
      few: '$count groźby',
      one: '$count groźba',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count innego przestępstwa',
      many: '$count innych przestępstw',
      few: '$count inne przestępstwa',
      one: '$count inne przestępstwo',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'ułaskawianie…';

  @override
  String get factionsEmpty => 'Brak otwartych przestępstw przeciwko frakcjom.';

  @override
  String get factionGuildOldCamp => 'Stary Obóz';

  @override
  String get factionGuildNewCamp => 'Nowy Obóz';

  @override
  String get factionGuildSwampCamp => 'Obóz na Bagnach';

  @override
  String get factionGuildOther => 'Inni/jednostki';

  @override
  String get allDataLockedBody =>
      'Pełna przeglądarka właściwości wymaga odkodowanych prywatnych danych z kodeka.';

  @override
  String get allDataDescription =>
      'Przeszukuj wszystkie typowane właściwości według nazwy lub ścieżki. Wartości liczbowe, ciągi, wyliczenia i ścieżki obiektów można edytować; struktury są na razie wyświetlane tylko do odczytu.';

  @override
  String get searchPropertiesLabel =>
      'Szukaj właściwości (puste = pokaż wszystko) — np. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Dekodowanie zapisu…';

  @override
  String get decodingSaveBody =>
      'Dekodowanie pełnych prywatnych danych dla pierwszego wyszukiwania. Odbywa się to raz na zapis, a potem wyszukiwania są natychmiastowe.';

  @override
  String get searchTheSaveTitle => 'Przeszukaj zapis';

  @override
  String get searchTheSaveBody =>
      'Wpisz nazwę właściwości i naciśnij Enter. Pozostaw puste, aby pokazać wszystko.';

  @override
  String get searchFailedTitle => 'Wyszukiwanie nie powiodło się';

  @override
  String get noMatchesTitle => 'Brak wyników';

  @override
  String get noMatchesBody =>
      'Żadna ścieżka właściwości nie zawierała wszystkich tych terminów.';

  @override
  String get value => 'Wartość';

  @override
  String get backupsTitle => 'Kopie zapasowe';

  @override
  String get refreshBackups => 'Odśwież kopie zapasowe';

  @override
  String get noBackupsTitle => 'Brak kopii zapasowych';

  @override
  String get noBackupsBody =>
      'Edytowane zapisy tworzą pliki kopii zapasowych obok wybranego slotu.';

  @override
  String get slotBackups => 'Kopie slotu';

  @override
  String get profileBackups => 'Kopie profilu';

  @override
  String get backupFactName => 'Nazwa';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Utworzono';

  @override
  String get backupFactSize => 'Rozmiar';

  @override
  String get backupFactStatus => 'Stan';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Przywróć $fileName';
  }

  @override
  String get appearanceTitle => 'Wygląd';

  @override
  String get theme => 'Motyw';

  @override
  String get themeLight => 'Jasny';

  @override
  String get themeDark => 'Ciemny';

  @override
  String get themeSystem => 'Systemowy';

  @override
  String get uiScale => 'Skala interfejsu';

  @override
  String get resetZoomTooltip => 'Resetuj powiększenie (Ctrl+0)';

  @override
  String get zoomTip =>
      'Wskazówka: Ctrl + / Ctrl - zmienia powiększenie w dowolnym miejscu aplikacji.';

  @override
  String get language => 'Język';

  @override
  String get updatesTitle => 'Aktualizacje';

  @override
  String get checkForUpdatesAutomatically =>
      'Sprawdzaj aktualizacje automatycznie';

  @override
  String get checkForUpdatesNow => 'Sprawdź aktualizacje teraz';

  @override
  String get updatesPortableNotice =>
      'Wersja przenośna otwiera stronę pobierania w przeglądarce. Zastąp istniejące pliki nowym pobraniem.';

  @override
  String get updateAvailableTitle => 'Dostępna aktualizacja';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Wersja $version jest dostępna. Masz $current.';
  }

  @override
  String get updateDownload => 'Pobierz';

  @override
  String get updateLater => 'Później';

  @override
  String get updateUpToDate => 'Używasz najnowszej wersji.';

  @override
  String get updateCheckFailed =>
      'Nie udało się sprawdzić aktualizacji. Spróbuj ponownie później.';

  @override
  String get gameTextTitle => 'Tekst gry';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Wyodrębniono: $ids identyfikatorów w $languages językach.';
  }

  @override
  String get gameTextExtracted =>
      'Zlokalizowany tekst gry został wyodrębniony.';

  @override
  String get gameTextNotExtracted =>
      'Zlokalizowany tekst gry nie został jeszcze wyodrębniony.';

  @override
  String get extracting => 'Wyodrębnianie…';

  @override
  String get extractRefreshLocalizedText =>
      'Wyodrębnij / odśwież zlokalizowany tekst';

  @override
  String get extractLocalizedTextTitle => 'Wyodrębnić zlokalizowany tekst gry?';

  @override
  String get extractLocalizedTextBody =>
      'Zlokalizowany tekst gry nie został jeszcze wyodrębniony. Wyodrębnić go teraz z Twojej instalacji gry? (opcjonalnie)';

  @override
  String get notNow => 'Nie teraz';

  @override
  String get extract => 'Wyodrębnij';

  @override
  String get extractionComplete => 'Wyodrębnianie zakończone';

  @override
  String get extractionFailed => 'Wyodrębnianie nie powiodło się';

  @override
  String get localizationCacheFileType => 'Pamięć podręczna lokalizacji';

  @override
  String get savegameDirectoryTitle => 'Folder zapisów gry';

  @override
  String get folder => 'Folder';

  @override
  String get codecTitle => 'Kodek';

  @override
  String get check => 'Sprawdź';

  @override
  String get roundtrip => 'Test obiegu';

  @override
  String get noCodecStatus => 'Brak stanu kodeka';

  @override
  String get codecReady => 'Kodek gotowy';

  @override
  String get codecReadOnly => 'Kodek tylko do odczytu';

  @override
  String get codecUnavailable => 'Kodek niedostępny';

  @override
  String get details => 'Szczegóły';

  @override
  String codecStatusLine(String status) {
    return 'Stan: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Dekompresja: $decompress | Kompresja: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'tak';

  @override
  String get no => 'nie';

  @override
  String get aboutSubtitle => 'Edytor zapisów gry Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Wersja $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 współtwórcy goresave';

  @override
  String get aboutLicense => 'Udostępniane na licencji MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Poziom trudności — $profile';
  }

  @override
  String get difficultyNoProfile => 'Brak profilu';

  @override
  String get difficultyNoDifficulty => 'Brak poziomu trudności';

  @override
  String get difficultyLabel => 'Poziom trudności';

  @override
  String get difficultyTooltipNoProfile => 'Nie wybrano profilu';

  @override
  String get difficultyTooltipEdit =>
      'Edytuj poziom trudności dla tego profilu';

  @override
  String get difficultyTooltipNoEditable =>
      'Ten profil nie ma edytowalnego poziomu trudności';

  @override
  String get preset => 'Ustawienie';

  @override
  String get presetNovice => 'Nowicjusz';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Trudny';

  @override
  String get presetCustom => 'Niestandardowy';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Zapisane ustawienie jest nierozpoznane ($preset). Nadal możesz zapisać zmiany Asystenta walki / Trwałej śmierci lub wybrać ustawienie powyżej, aby je nadpisać.';
  }

  @override
  String get closeCombatFlowHelper => 'Asystent walki wręcz';

  @override
  String get permadeath => 'Trwała śmierć';

  @override
  String get notAvailableOnNovice => 'Niedostępne na poziomie Nowicjusz';

  @override
  String get levelCombat => 'Walka';

  @override
  String get levelResources => 'Zasoby';

  @override
  String get levelProgression => 'Postępy';

  @override
  String get difficultyAppliesToAllSaves =>
      'Poziom trudności dotyczy wszystkich zapisów w tym profilu.';

  @override
  String get savingDifficultyFailed =>
      'Zapisanie poziomu trudności nie powiodło się.';

  @override
  String get addItemDialogTitle => 'Dodaj przedmiot';

  @override
  String get searchItems => 'Szukaj przedmiotów';

  @override
  String failedToLoadCatalog(String error) {
    return 'Nie udało się wczytać katalogu: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Brak przedmiotów do dodania';

  @override
  String get noItemsMatch => 'Żaden przedmiot nie pasuje';

  @override
  String get countMustBeAtLeast1 => 'Musi być ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Musi być ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Dodaj NPC';

  @override
  String get noNpcsAvailableToAdd => 'Brak NPC do dodania';

  @override
  String get noNpcsMatch => 'Żaden NPC nie pasuje';

  @override
  String get categoryAll => 'Wszystkie';

  @override
  String allWithCount(int count) {
    return 'Wszystkie ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Dodaj wpis wiedzy';

  @override
  String get searchEntries => 'Szukaj wpisów';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Brak wpisów wiedzy do dodania';

  @override
  String get noEntriesMatch => 'Żaden wpis nie pasuje';

  @override
  String get heroGroupMainStats => 'Główne statystyki';

  @override
  String get heroGroupCombatSkills => 'Umiejętności bojowe';

  @override
  String get heroGroupResistances => 'Odporności';

  @override
  String get heroGroupThieving => 'Złodziejstwo';

  @override
  String get heroGroupAdvanced => 'Zaawansowane';

  @override
  String get heroEntryHeroTransform => 'Pozycja bohatera';

  @override
  String attributeEmpty(String name) {
    return '$name jest puste — wpisz wartość lub przywróć oryginał przed zapisem.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Nieprawidłowa liczba dla $name: „$text”';
  }

  @override
  String get loadingEditorData => 'Wczytywanie danych edytora';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Wyodrębniono $idCount identyfikatorów w $languageCount językach';
  }
}
