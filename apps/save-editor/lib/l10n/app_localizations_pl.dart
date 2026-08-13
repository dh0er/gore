// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class AppLocalizationsPl extends AppLocalizations {
  AppLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get debugSectionTitle => 'Zaawansowane (debugowanie)';

  @override
  String get debugSectionSubtitle =>
      'Diagnostyka i surowe dane do zgłoszeń błędów';

  @override
  String get showObjectIdsTitle => 'Pokaż dodatkowe identyfikatory techniczne';

  @override
  String get showObjectIdsSubtitle =>
      'Pokazuje techniczne identyfikatory przedmiotów, wiedzy dialogowej, zadań i osieroconych postaci. Identyfikatory NPC są zawsze widoczne.';

  @override
  String get storyStateSidebar => 'Stan fabuły';

  @override
  String get storyStateDescription =>
      'Autorytatywny katalog trwałych stanów fabuły zadeklarowanych w skryptach dostarczonych z grą. Zapisane wpisy pokazują surową wartość; pola katalogu nieobecne w tym zapisie są oznaczone jako nieustawione. Znaczniki czasu zadeklarowane w kodzie są przedstawiane jako czas gry, a pozostałe liczby całkowite mogą być wartościami logicznymi, licznikami albo stanami wielopoziomowymi.';

  @override
  String get storyStateReadOnly =>
      'Tylko do odczytu, dopóki nie zostanie potwierdzone znaczenie wartości w skryptach i bezpieczny zapis mapy. Powiązany tekst glosariusza jest kontekstem, a nie bezpośrednim tłumaczeniem technicznego ID.';

  @override
  String get storyStateStructureReadOnly =>
      'Nie udało się jednoznacznie i bezpiecznie ustalić struktury StoryPropertyValues w tym zapisie. Wartości fabuły pozostają w tym zapisie tylko do odczytu.';

  @override
  String get storyStateSearch => 'Szukaj w stanie fabuły';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown z $total wartości fabularnych';
  }

  @override
  String get storyStateInteger => 'Liczba całkowita';

  @override
  String get storyStateTimeMarker => 'Znacznik czasu';

  @override
  String get storyStateChapter => 'Rozdział';

  @override
  String get storyStateUnknown => 'Nieznany typ źródłowy';

  @override
  String get storyStateUnknownDetail =>
      'Tego zapisanego ID nie ma w bieżącym katalogu skryptów (np. pochodzi z moda lub nowszej wersji gry). Wartość w zapisie ma format int32, ale jej znaczenie nie jest zgadywane.';

  @override
  String get storyStateStored => 'Zapisane';

  @override
  String get storyStateUnset => 'Nieustawione';

  @override
  String get storyStateUnsetDetail =>
      'To pole katalogu nie jest zserializowane w tym zapisie; gra używa więc stanu nieustawionego lub domyślnego.';

  @override
  String get storyStateRawValue => 'Wartość surowa';

  @override
  String storyStateElapsed(String duration) {
    return 'Czas, który upłynął przy zapisie: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Czas przyszły przy zapisie: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days dnia',
      many: '$days dni',
      few: '$days dni',
      one: '1 dzień',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Powiązany wpis glosariusza';

  @override
  String get storyStateTechnicalPath => 'Ścieżka techniczna';

  @override
  String get storyStateEditingGuidance =>
      'Każdy wpis można edytować w pełnym zakresie wartości int32 ze znakiem. Flagi i sugerowane wartości oparte na skryptach są jedynie wskazówkami; zawsze można wprowadzić surową wartość. Zmiany stanu fabuły mogą pominąć przejścia dialogów, zadań lub świata, dlatego zapisuj je rozważnie. Kopia zapasowa zostanie utworzona automatycznie.';

  @override
  String get storyStatePending => 'Oczekuje';

  @override
  String storyStatePendingValue(String value) {
    return 'Zostanie zapisane jako $value';
  }

  @override
  String get storyStatePendingRemoval => 'Zostanie usunięte z zapisu';

  @override
  String get storyStateEditValue => 'Edytuj wartość';

  @override
  String get storyStateSetValue => 'Ustaw wartość';

  @override
  String get storyStateRemoveValue => 'Usuń z zapisu';

  @override
  String get storyStateUndoChange => 'Cofnij zmianę fabuły';

  @override
  String get storyStateResetChanges => 'Zresetuj zmiany fabuły';

  @override
  String storyStateDialogTitle(String id) {
    return 'Edytuj $id';
  }

  @override
  String get storyStateRawInput => 'Wartość int32 ze znakiem';

  @override
  String get storyStateInvalidInt32 =>
      'Wprowadź liczbę całkowitą od -2147483648 do 2147483647.';

  @override
  String get storyStateQueueChange => 'Dodaj zmianę do kolejki';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Wartości potwierdzone w dostarczonych skryptach: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Sugestie nie są ograniczeniami walidacji; kod natywny, mody lub nowsze wersje gry mogą używać innych wartości.';

  @override
  String get storyStateUseCurrentTime => 'Użyj bieżącego czasu zapisu';

  @override
  String get storyStateStructuredTime => 'Dzień / czas';

  @override
  String get storyStateRawMode => 'Surowe int32';

  @override
  String get storyStateChapterWarning =>
      'Zmiana samego rozdziału nie synchronizuje zadań, postaci niezależnych, ekwipunku ani stanu świata.';

  @override
  String get storyStateDormantWarning =>
      'W pamięci podręcznej dostarczonych skryptów nie znaleziono aktywnego odczytu ani zapisu tego pola. Może być przestarzałe, sterowane przez kod natywny lub zarezerwowane.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Dostarczone skrypty odczytują to pole, ale nie zawierają zapisu skryptowego. Nadal może być ono obsługiwane przez kod natywny.';

  @override
  String get storyStateUnknownEditWarning =>
      'Ten identyfikator z moda lub nowszej wersji nie ma dołączonej semantyki źródłowej. Edytuj wyłącznie jego surową wartość int32.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Flaga binarna',
      'finiteState': 'Wartość wielostanowa',
      'counterOrScore': 'Licznik / wynik',
      'calendarDay': 'Dzień kalendarzowy',
      'derivedOrOpaqueInteger': 'Liczba pochodna / niejawna',
      'readOnlyInSourceInteger': 'Tylko do odczytu w dostarczonych skryptach',
      'dormantOrLegacyInteger': 'Nieużywane w dostarczonych skryptach',
      'other': 'Liczba całkowita',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Zapisane 0 i brak wpisu w mapie to dwa różne stany pliku. Opcja „Usuń z zapisu” przywraca stan konstruktora lub stan domyślny.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo GORE Save Editor';

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
  String get heroGroupSkills => 'Umiejętności';

  @override
  String get skillsNoneBody => 'Nie znaleziono umiejętności dla tej postaci.';

  @override
  String get skillsUnavailableBody =>
      'Umiejętności nie można edytować w tym zapisie — bohater nie ma danych efektów do zmiany.';

  @override
  String get skillNotLearned => 'Nie nauczono';

  @override
  String get skillLearn => 'Naucz się';

  @override
  String get skillActionLearn => 'naucz się';

  @override
  String get skillActionUnlearn => 'zapomnij';

  @override
  String get skillTierUntrained => 'Niewyszkolony';

  @override
  String get skillTierBeginner => 'Początkujący';

  @override
  String get skillTierTrained => 'Wyszkolony';

  @override
  String get skillTierMaster => 'Mistrz';

  @override
  String get skillTierNovice => 'Nowicjusz';

  @override
  String get skillTierAmateur => 'Amator (Krąg 0)';

  @override
  String get skillTierLearned => 'Nauczono';

  @override
  String skillTierCircle(int n) {
    return 'Krąg $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Broń jednoręczna';

  @override
  String get skillHintBlacksmith2H => 'Broń dwuręczna';

  @override
  String get skillCategoryCombat => 'Walka';

  @override
  String get skillCategoryCrafting => 'Rzemiosło';

  @override
  String get skillCategoryHunting => 'Łowiectwo';

  @override
  String get skillCategoryLanguage => 'Język';

  @override
  String get skillCategoryMagic => 'Magia';

  @override
  String get skillCategoryMovement => 'Ruch';

  @override
  String get skillCategoryThievery => 'Złodziejstwo';

  @override
  String get skillNameOneHanded => 'Broń jednoręczna';

  @override
  String get skillNameTwoHanded => 'Broń dwuręczna';

  @override
  String get skillNameFists => 'Gołe pięści';

  @override
  String get skillNameBow => 'Łuk';

  @override
  String get skillNameCrossbow => 'Kusza';

  @override
  String get skillNameLockpicking => 'Otwieranie zamków';

  @override
  String get skillNamePickpocketing => 'Kradzież kieszonkowa';

  @override
  String get skillNameTakeOrgans => 'Wyjęcie organu';

  @override
  String get skillNameBreakTeeth => 'Usuwanie kłów';

  @override
  String get skillNameTakeClaws => 'Usuwanie pazurów';

  @override
  String get skillNameSkinFur => 'Pozyskiwanie futra';

  @override
  String get skillNameSkin => 'Pozyskiwanie skór';

  @override
  String get skillNameTakeFins => 'Pozyskiwanie płetw';

  @override
  String get skillNameTakeStingers => 'Wyjęcie żądła';

  @override
  String get skillNameTakeSecretion => 'Pozyskiwanie wydzieliny';

  @override
  String get skillNameTakeSkullPlates => 'Pozyskiwanie płytek czaszki';

  @override
  String get skillNameSkinSwampshark => 'Skórowanie błotnego węża';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Płytki pełzacza';

  @override
  String get skillNameTakeScutes => 'Pozyskiwanie łusek';

  @override
  String get skillNameTakeUluMulu => 'Pozyskiwanie Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Broń orków';

  @override
  String get skillNameMining => 'Górnictwo';

  @override
  String get skillNameDiving => 'Nurkowanie';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Wyrwanie żuwaczek';

  @override
  String get skillNameTakeShadowbeastHorn => 'Róg cieniostwora (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Wyjęcie kręgosłupa';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Kły błotnego węża';

  @override
  String get skillNameTakeFireTongue => 'Język ognia';

  @override
  String get skillNameTakeTrollHorn => 'Pozyskiwanie rogów (Troll)';

  @override
  String get skillNameAcrobatics => 'Akrobatyka';

  @override
  String get skillNameWallClimbing => 'Wspinaczka';

  @override
  String get skillNameRiding => 'Ujeżdżanie ścierwojadów';

  @override
  String get skillNameSneaking => 'Skradanie';

  @override
  String get skillNameAlchemy => 'Alchemia';

  @override
  String get skillNameRuneInscription => 'Inskrypcja';

  @override
  String get skillNameBlacksmithing => 'Kowalstwo';

  @override
  String get skillNameMagicCircle => 'Krąg magiczny';

  @override
  String get skillNameOrcish => 'Język orkowy';

  @override
  String get tabInventory => 'Ekwipunek';

  @override
  String get tabTrade => 'Handel';

  @override
  String get traderNotAMerchant => 'Ta postać nie handluje.';

  @override
  String get traderRetry => 'Spróbuj ponownie';

  @override
  String get traderAmbiguousName =>
      'Kilka rekordów kupca nosi tę nazwę, więc nie da się ustalić, który sklep należy do tej postaci. Edycja jest wyłączona, zamiast ryzykować zmianę niewłaściwego.';

  @override
  String get traderOre => 'Ruda (siła nabywcza)';

  @override
  String get traderNoOre => 'brak rudy';

  @override
  String get traderStockCurrent => 'Zapas';

  @override
  String get traderStockBase => 'Baza uzupełniania';

  @override
  String get traderStockBaseHint =>
      'To, do czego kupiec uzupełnia zapasy. Rośnie wraz z fabułą, więc nie jest stanem pierwotnym.';

  @override
  String get traderOreHint =>
      'Wartość w grze się różni: przy wczytaniu gra dolicza to, co narosło od jego ostatniego handlu — sprzedaje nadwyżki i z tego uzupełnia zapasy. Ta liczba to punkt wyjścia, a nie kwota z ekranu handlu.';

  @override
  String get traderPriceWarning =>
      'Ceny reagują na to, ile kupiec ma na stanie i ile ma rudy, więc zmiana tych liczb może też zmienić jego stawki.';

  @override
  String get traderAddItem => 'Dodaj przedmiot';

  @override
  String get traderRemoveItem => 'Usuń pozycję';

  @override
  String get traderReadOnlyCore =>
      'Ta wersja rdzenia może tylko odczytywać dane kupców.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Ten kupiec ma zapasy zależne od poziomu trudności, których edytor nie odwzorowuje. Edycja jest tu wyłączona, bo zmiana wyglądałaby na udaną, zostawiając te dodatkowe zapasy nietknięte.';

  @override
  String get traderRecordIncomplete =>
      'Listy zapasów tego kupca nie istnieją albo mają postać, której edytor nie obsługuje i nie potrafi zapisać. Edycja jest wyłączona, aby zmiana nie zawiodła przy zapisie.';

  @override
  String get traderEmptyStock => 'Brak zapasów.';

  @override
  String get traderUnknownItem => 'brak w katalogu przedmiotów';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Nie udało się wczytać kupców: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count pozycji';
  }

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
  String get otherSaves => 'Inne zapisy';

  @override
  String profileWithSaves(String name, int count) {
    return '$name (zapisy: $count)';
  }

  @override
  String get switchProfile => 'Zmień profil';

  @override
  String get openSaveFile => 'Otwórz plik';

  @override
  String get externalSave => 'Zapis otwarty zewnętrznie';

  @override
  String get saveProfileTitle => 'Profil zapisu';

  @override
  String get saveProfileDescription =>
      'Przypisz ten zapis do innego profilu gry. Kopia zapisu i indeksu profili zostanie utworzona razem.';

  @override
  String get saveProfileExternalHint =>
      'Wybierz profil, aby zaimportować ten plik do folderu zapisów gry i go tam zarejestrować. Oryginalny plik pozostanie bez zmian.';

  @override
  String get saveProfileNoProfiles =>
      'Nie znaleziono edytowalnych profili gry w pliku PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Wybierz profil';

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
  String bytesValue(String count) {
    return '$count B';
  }

  @override
  String get inspectionJsonTitle => 'JSON inspekcji';

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
  String get gameTimeTitle => 'Czas gry';

  @override
  String get gameTimeDay => 'Dzień';

  @override
  String get gameTimeHours => 'Godziny';

  @override
  String get gameTimeMinutes => 'Minuty';

  @override
  String get gameTimeSeconds => 'Sekundy';

  @override
  String gameTimeTotal(int seconds) {
    return '= łącznie $seconds s';
  }

  @override
  String get gameTimeInvalid =>
      'Wprowadź liczby całkowite: dzień ≥ 0, godziny 0–23, minuty i sekundy 0–59.';

  @override
  String get required => 'Wymagane';

  @override
  String get playerLockedBody =>
      'Edycja prywatnych danych postaci wymaga kodeka obsługującego kompresję.';

  @override
  String get heroTransform => 'Pozycja';

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
  String get spawnPositionSection => 'Pozycja odrodzenia (odniesienie)';

  @override
  String get resetToSpawnPosition => 'Przywróć pozycję odrodzenia';

  @override
  String get positionOutOfRange =>
      'Wartość musi mieścić się w zakresie od −10 000 000 do 10 000 000';

  @override
  String get positionNotEditable =>
      'Nie udało się odczytać zapisanej pozycji tej postaci, więc nie można jej edytować.';

  @override
  String get positionNeverPlaced =>
      'Ta postać nigdy nie została umieszczona w świecie (pozycja 0, 0, 0) — gra może zignorować zapisaną pozycję.';

  @override
  String get npcStayInPlace => 'Wyłącz jego plan dnia';

  @override
  String get npcStayInPlaceHint => 'Zostanie wtedy tam, gdzie jest.';

  @override
  String get npcStayInPlaceLocked =>
      'Jego pierwotny plan dnia nie został zapisany, więc nie można już tego cofnąć.';

  @override
  String get npcUndoPlacement => 'Cofnij przeniesienie';

  @override
  String get npcUndoPlacementStale =>
      'Zapis nie zawiera już tego, co zapisało to przeniesienie, więc przywrócenie odrzuciłoby to, co zdarzyło się później.';

  @override
  String get positionNotReadable =>
      'Nie udało się odczytać zapisanej pozycji tej postaci.';

  @override
  String get npcPositionReadOnly =>
      'Gra odtwarza pozycję NPC z poziomu, a nie z zapisu gry, więc te wartości można odczytać, ale nie zmienić.';

  @override
  String get pickLocation => 'Wybierz miejsce…';

  @override
  String get pickLocationDialogTitle => 'Wybierz miejsce';

  @override
  String get applySpotRotation => 'Zastosuj też orientację miejsca';

  @override
  String get locationAreaOther => 'Inne';

  @override
  String get locationAreaCavalornValley => 'Dolina Cavalorna';

  @override
  String get locationAreaEastForest => 'Wschodni Las';

  @override
  String get locationAreaFogTower => 'Mglista Wieża';

  @override
  String get locationAreaIllegalWeedMixers => 'Nielegalni mieszacze ziela';

  @override
  String get locationAreaOrcArena => 'Arena Orków';

  @override
  String get locationAreaOrcGraveyard => 'Cmentarzysko Orków';

  @override
  String get locationAreaShipwreck => 'Wrak statku';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'Nie udało się wczytać katalogu miejsc.';

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
  String get attributeBaseValue => 'Wartość bazowa';

  @override
  String get attributeCurrentValue => 'Wartość bieżąca';

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
  String get resetInventoryButton => 'Resetuj ekwipunek';

  @override
  String get resetInventoryTooltipDefault =>
      'Zastąp ten ekwipunek ekwipunkiem z początku gry';

  @override
  String get resetInventoryTooltipBlocked =>
      'Najpierw zapisz lub anuluj oczekujące zmiany ekwipunku';

  @override
  String get pendingResetTitle => 'Przywróć ekwipunek z początku gry';

  @override
  String pendingResetSubtitle(String level) {
    return 'Poziom zasobów: $level';
  }

  @override
  String get cancelPendingReset => 'Anuluj resetowanie';

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
  String get npcRelationshipRowLabel => 'Relacja';

  @override
  String get npcRelationshipUnavailable => 'Stan relacji jest niedostępny';

  @override
  String get npcRelationshipAutomatic => 'Obliczana przez grę';

  @override
  String get npcRelationshipAutomaticHint =>
      'Nie zapisano stałego nadpisania. Gra ocenia reguły gildii, fabuły, obszaru i przestępstw.';

  @override
  String get npcRelationshipStoredHint =>
      'Zapisana jako stałe nadpisanie relacji NPC z graczem. Reguły gildii, fabuły, obszaru i przestępstw mogą nadal zmienić faktyczną relację w grze.';

  @override
  String get npcRelationshipFriend => 'Przyjaciel';

  @override
  String get npcRelationshipNeutral => 'Neutralny';

  @override
  String get npcRelationshipEnemy => 'Wróg';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Po zapisaniu: $relationship';
  }

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
  String get memoryEventRemovalQueued =>
      'Usunięcie zdarzenia oczekuje — naciśnij Zapisz, aby je zastosować.';

  @override
  String get duplicateEvent => 'Powiel zdarzenie';

  @override
  String get duplicateMemoryEventTitle => 'Powielić zdarzenie z pamięci?';

  @override
  String get duplicateMemoryEventBody =>
      'Powielić to zdarzenie z pamięci? Najpierw zostanie utworzona kopia zapasowa.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplikowanie zdarzenia oczekuje — naciśnij Zapisz, aby je zastosować.';

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
  String get factionGuildSwampCamp => 'Obóz Bractwa';

  @override
  String get factionGuildOther => 'Inni/jednostki';

  @override
  String get allDataLockedBody =>
      'Pełna przeglądarka danych jest obecnie dostępna dla zapisów w formacie GSAV.';

  @override
  String get allDataDescription =>
      'Przeglądaj metadane GSAV oraz wszystkie typowane węzły sekcji PUBLIC i PRIVATE. Bezpieczne wartości skalarne i struktury natywne można edytować; kontenery i nieprzetworzone bajty pozostają widoczne.';

  @override
  String get allDataEditable => 'Edytowalne';

  @override
  String get allDataReadOnly => 'Tylko do odczytu';

  @override
  String get allDataType => 'Typ';

  @override
  String get allDataScalars => 'Wartości skalarne';

  @override
  String get allDataStructs => 'Struktury';

  @override
  String get allDataContainers => 'Kontenery';

  @override
  String get allDataOpaque => 'Nieprzetworzone bajty';

  @override
  String get allDataNodes => 'Węzły';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count węzła podrzędnego',
      many: '$count węzłów podrzędnych',
      few: '$count węzły podrzędne',
      one: '$count węzeł podrzędny',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Do zapisania';

  @override
  String get allDataTagInputHint =>
      'Tagi rozdzielone przecinkami lub podziałami wiersza';

  @override
  String allDataTypedSource(String source) {
    return 'Dane typowane: $source';
  }

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
  String aboutVersion(String version, String sha) {
    return 'Wersja $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 współtwórcy GORE';

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
  String get presetNovice => 'Łatwy';

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
  String get closeCombatFlowHelper => 'Pomoc w płynności walki';

  @override
  String get permadeath => 'Permanentna śmierć';

  @override
  String get notAvailableOnNovice => 'Niedostępne na poziomie Nowicjusz';

  @override
  String get levelCombat => 'Walka';

  @override
  String get levelResources => 'Zasoby';

  @override
  String get levelProgression => 'Postęp';

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
  String get heroEntryHeroTransform => 'Pozycja';

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
  String savingProgress(int done, int total) {
    return 'Zapisywanie… $done z $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Wyodrębniono $idCount identyfikatorów w $languageCount językach';
  }

  @override
  String get skillSmithing1H => 'Kowalstwo – broń jednoręczna';

  @override
  String get skillSmithing2H => 'Kowalstwo – broń dwuręczna';

  @override
  String get skillCircleNovice => 'Nowicjusz Mag';

  @override
  String get skillCircle1 => 'Pierwszy Krąg Magii';

  @override
  String get skillCircle2 => 'Drugi Krąg Magii';

  @override
  String get skillCircle3 => 'Trzeci Krąg Magii';

  @override
  String get skillCircle4 => 'Czwarty Krąg Magii';

  @override
  String get skillCircle5 => 'Piąty Krąg Magii';

  @override
  String get skillCircle6 => 'Szósty Krąg Magii';

  @override
  String get sectionGlossary => 'Glosariusz';

  @override
  String get glossarySearch => 'Szukaj w glosariuszu';

  @override
  String get glossaryOldCamp => 'Stary Obóz';

  @override
  String get glossaryNewCamp => 'Nowy Obóz';

  @override
  String get glossarySwampCamp => 'Obóz Bractwa';

  @override
  String get glossaryOutsiders => 'Obcy';

  @override
  String get glossaryCreatures => 'Stworzenia';

  @override
  String get glossaryLocations => 'Lokacje';

  @override
  String get glossaryFilterLabel => 'Filtr';

  @override
  String get glossaryFilterTraders => 'Handlarze';

  @override
  String get glossaryFilterTeachers => 'Nauczyciele';

  @override
  String get glossaryFilterArmorers => 'Płatnerze';

  @override
  String get glossaryFilterHostile => 'Wrodzy';

  @override
  String get glossaryRelationshipFilterNote =>
      'Pokazuje stałe ustawienia wrogości zapisane w pliku. Dynamiczne relacje gildii, fabuły, obszaru i przestępstw są obliczane wyłącznie w grze.';

  @override
  String get glossaryFilterDead => 'Martwi';

  @override
  String get glossaryAddEntry => 'Dodaj wpis do glosariusza';

  @override
  String get glossaryAddTitle => 'Dodaj wpis do glosariusza';

  @override
  String get glossaryResetChanges => 'Resetuj zmiany glosariusza';

  @override
  String get glossaryNoVisibleEntries =>
      'Brak widocznych wpisów glosariusza pasujących do tego widoku.';

  @override
  String get glossaryNoHiddenEntries =>
      'Wszystkie dostępne wpisy są już widoczne.';

  @override
  String get glossaryNoMatch => 'Brak pasujących wpisów glosariusza.';

  @override
  String get glossarySelectEntry =>
      'Wybierz wpis glosariusza, aby edytować jego sekcje.';

  @override
  String glossaryEntryCount(int count) {
    return 'Liczba wpisów: $count';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return 'Odblokowano $unlocked z $total wpisów';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portret odblokowany';

  @override
  String get glossaryPortraitSilhouette => 'Sylwetka — portret nieodblokowany';

  @override
  String get glossarySegments => 'Wpisy';

  @override
  String get glossaryPending => 'Niezapisana zmiana';

  @override
  String get glossaryShowFullText => 'Pokaż pełny tekst wpisu';

  @override
  String get glossarySegmentIntroduction => 'Wprowadzenie / portret';

  @override
  String get glossarySegmentUnlock => 'Odkrycie';

  @override
  String glossarySegmentEntry(int number) {
    return 'Wpis $number';
  }

  @override
  String get questJournalAll => 'Wszystkie zadania';

  @override
  String get questJournalOldCamp => 'Stary Obóz';

  @override
  String get questJournalNewCamp => 'Nowy Obóz';

  @override
  String get questJournalSwampCamp => 'Obóz Bractwa';

  @override
  String get questJournalColony => 'Kolonia';

  @override
  String get questJournalCompleted => 'Ukończone';

  @override
  String get questJournalHint =>
      'Widok dziennika w grze. Stany wewnętrzne i jeszcze nierozpoczęte zadania pozostają dostępne w sekcji Wszystkie dane.';

  @override
  String get questJournalNoEntries =>
      'Żadne zadania w dzienniku nie pasują do bieżących filtrów.';

  @override
  String get glossaryTutorials => 'Samouczki';

  @override
  String get tutorialGateNote =>
      'Te wiersze sterują zapisanymi odblokowaniami samouczków. Jedno odblokowanie nie musi odpowiadać jednej stronie samouczka w grze.';

  @override
  String get tutorialResetChanges => 'Resetuj zmiany samouczków';

  @override
  String get tutorialNoGates =>
      'W tym zapisie nie ma dostępnych odblokowań samouczków.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return 'Odblokowano $unlocked z $total samouczków';
  }

  @override
  String get tutorialGateCombatBasics => 'Podstawy walki';

  @override
  String get tutorialGateCrafting => 'Rzemiosło';

  @override
  String get tutorialGateCrime => 'Przestępstwa i konsekwencje';

  @override
  String get tutorialGateDrugs => 'Przedmioty zużywalne i efekty';

  @override
  String get tutorialGateLockpicking => 'Otwieranie zamków';

  @override
  String get tutorialGateMagic => 'Magia';

  @override
  String get tutorialGateMap => 'Mapa';

  @override
  String get tutorialGateMeleeCombat => 'Walka wręcz';

  @override
  String get tutorialGateNavigation => 'Ruch i nawigacja';

  @override
  String get tutorialGatePerception => 'Percepcja';

  @override
  String get tutorialGatePlayerProgression => 'Rozwój postaci';

  @override
  String get tutorialGateRanged => 'Walka dystansowa';

  @override
  String get tutorialGateRiding => 'Jazda wierzchem';

  @override
  String get tutorialGateSleep => 'Sen';

  @override
  String get tutorialGateTrading => 'Handel';

  @override
  String get windowMinimizeTooltip => 'Minimalizuj';

  @override
  String get windowMaximizeTooltip => 'Maksymalizuj';

  @override
  String get windowRestoreTooltip => 'Przywróć';

  @override
  String get fallbackDialogEntry => 'Wpis dialogowy';

  @override
  String get fallbackDialogChoice => 'Wybór dialogowy';

  @override
  String get fallbackDialogTopic => 'Temat dialogu';

  @override
  String get fallbackDialogInformation => 'Informacja dialogowa';

  @override
  String get fallbackQuest => 'Zadanie';

  @override
  String get fallbackObjective => 'Cel';

  @override
  String get fallbackItem => 'Przedmiot';

  @override
  String get attributeSkillPointsFallback => 'Punkty nauki (PN)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Alkohol',
      'AlcoholDepletionRate': 'Tempo spadku poziomu alkoholu',
      'MaxAlcohol': 'Maksymalny poziom alkoholu',
      'MaxSuperArmor': 'Maksymalny superpancerz',
      'SuperArmor': 'Superpancerz',
      'Fatigue': 'Zmęczenie',
      'FillRatio': 'Stopień napełnienia',
      'FillRatioPeriod': 'Okres napełniania',
      'MaxFatigue': 'Maksymalne zmęczenie',
      'MaxThresholdIndex': 'Maksymalny indeks progu',
      'RecoveryRatePerHourOfSleep': 'Regeneracja na godzinę snu',
      'DamageMultiplier': 'Mnożnik obrażeń',
      'Toughness': 'Wytrzymałość',
      'ToughnessA': 'Wytrzymałość A',
      'ToughnessB': 'Wytrzymałość B',
      'ToughnessC': 'Wytrzymałość C',
      'XPExecutedBounty': 'Nagroda PD za egzekucję',
      'XPKillOrDefeatBounty': 'Nagroda PD za zabicie lub pokonanie',
      'SpeedModifier': 'Modyfikator prędkości',
      'CriticalLevelPercent': 'Poziom krytyczny (%)',
      'MaxOxygen': 'Maksymalny poziom tlenu',
      'Oxygen': 'Tlen',
      'OxygenDepletionRate': 'Tempo zużycia tlenu',
      'OxygenRecoveryRate': 'Tempo regeneracji tlenu',
      'MaxRestTime': 'Maksymalny czas odpoczynku',
      'MaxSleepTime': 'Maksymalny czas snu',
      'SleepTime': 'Czas snu',
      'SleepTimeRecoveryAmount': 'Wartość regeneracji podczas snu',
      'SleepTimeRecoveryPeriod': 'Okres regeneracji podczas snu',
      'MaxSwampweed': 'Maksymalny poziom bagiennego ziela',
      'Swampweed': 'Bagienne ziele',
      'SwampweedDepletionRate': 'Tempo zużycia bagiennego ziela',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Kwestia głosowa';

  @override
  String get knowledgeTypeOther => 'Inne';

  @override
  String get armorUpgradeUpper => 'Góra';

  @override
  String get armorUpgradeMiddle => 'Środek';

  @override
  String get armorUpgradeLower => 'Dół';

  @override
  String get knowledgeCategoryTopic => 'Temat';

  @override
  String get knowledgeCategoryChoice => 'Wybór';

  @override
  String get knowledgeCategoryInfo => 'Informacja';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Niepowodzenie';

  @override
  String get missingSaveReference => 'Brak pliku';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'Brakuje pliku $slot.sav. Mógł zostać usunięty, przeniesiony lub przemianowany; profil nadal się do niego odwołuje.';
  }

  @override
  String get removeFromProfile => 'Usuń z profilu';

  @override
  String get removeSaveFromProfileTitle => 'Usunąć zapis z profilu?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Usunąć $save z profilu $profile? Sam plik zapisu zostanie zachowany, jeśli nadal istnieje.';
  }

  @override
  String get unassignedSave => 'Nieprzypisany do profilu';

  @override
  String get armorUpgradeLight => 'Lekki';

  @override
  String get armorUpgradeMedium => 'Średni';

  @override
  String get armorUpgradeHeavy => 'Ciężki';

  @override
  String get knowledgeCaptionForcedConversation => 'Wymuszona rozmowa';

  @override
  String get knowledgeCaptionFollowupTopic => 'Temat uzupełniający';

  @override
  String get knowledgeCaptionFallbackTopic => 'Temat zastępczy';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours godz.';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours godz. $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure =>
      'Nieprawidłowe dane profilu';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Brak metadanych wybranego zapisu';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Nieznany';

  @override
  String editorUnexpectedError(String details) {
    return 'Nieoczekiwany błąd: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Trwa inna operacja. Spróbuj ponownie za chwilę.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Zapis zawiera niezapisane zmiany. Zapisz je lub zresetuj przed zmianą poziomu trudności profilu.';

  @override
  String get editorNoSaveFolderSelected => 'Nie wybrano folderu zapisów.';

  @override
  String get editorNoSaveSelected => 'Nie wybrano zapisu.';

  @override
  String get coreUnknownError => 'Nieznany błąd rdzenia';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Najpierw zapisz lub zresetuj niezapisane zmiany — zmiana profilu spowoduje opuszczenie bieżącego zapisu.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Zapisz lub zresetuj niezapisane zmiany przed otwarciem innego pliku.';

  @override
  String get editorSelectSavFile => 'Wybierz plik zapisu .sav.';

  @override
  String get editorNotGothicGsav =>
      'Wybrany plik nie jest zapisem Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Zapisz lub zresetuj niezapisane zmiany przed zmianą profilu zapisu.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Zapisz lub zresetuj niezapisane zmiany przed usunięciem zapisu z profilu.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Zapis zawiera niezapisane zmiany. Zapisz je lub zresetuj przed przywróceniem kopii zapasowej profilu.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Niezapisane zmiany z dwóch kart dotyczą tej samej właściwości ($path). Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Zmiana segmentu Glosariusza i inna niezapisana zmiana w sekcji „Wszystkie dane” dotyczą tablicy Hero MemorizedEvents ($path). Zmiany Glosariusza dodają lub usuwają wpisy z tej tablicy, dlatego nie można ich zapisać razem. Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Zmiana segmentu Glosariusza i inna niezapisana zmiana dotyczą tej samej właściwości CurrentState zadania ($path). Zmiana Glosariusza sama aktualizuje ten stan. Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Zmiana relacji i inna niezapisana zmiana w sekcji „Wszystkie dane” dotyczą tego samego wpisu relacji NPC ($path). Strukturalna zmiana relacji może zastąpić modyfikatory w tym wpisie, dlatego nie można zapisać obu zmian razem. Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Więcej niż jedna niezapisana zmiana struktury dotyczy tej samej tablicy ($path). Zapisz lub zresetuj pierwszą zmianę przed dodaniem kolejnej.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Strukturalna zmiana zdarzenia i inna niezapisana zmiana w sekcji „Wszystkie dane” dotyczą $path. Zapisz lub zresetuj jedną z nich przed kontynuowaniem.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Zmiana w sekcji „Umiejętności” i zmiana w sekcji „Wszystkie dane” dotycząca tego samego efektu postaci (ActiveEffects › EffectSpec › Def) oczekują na zapis. Nie można ich zapisać razem. Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';

  @override
  String get editorInventoryResetConflict =>
      'Reset ekwipunku i inna zmiana tego samego ekwipunku oczekują na zapis. Reset zastępuje cały ekwipunek i odrzuciłby drugą zmianę. Zresetuj lub cofnij jedną z nich, a następnie ponownie zapisz.';

  @override
  String get editorUseFolder => 'Użyj folderu';

  @override
  String get editorGothicSavegameFileType => 'Zapis gry Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Brak zmian poziomu trudności do zapisania';

  @override
  String get editorDifficultyWritten =>
      'Poziom trudności zapisano w profilu (utworzono kopię zapasową)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Zapisano $count zmian i utworzono kopię zapasową',
      many: 'Zapisano $count zmian i utworzono kopię zapasową',
      few: 'Zapisano $count zmiany i utworzono kopię zapasową',
      one: 'Zapisano 1 zmianę i utworzono kopię zapasową',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Przeniesienie zostało zapisane, ale nie udało się zapisać notatki do cofnięcia: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Nie znaleziono profilu $profileId.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'W folderze zapisów gry nie ma wolnego slotu zapisu (od G1R-001 do G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Zapis zaimportowano i przypisano do profilu $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Zapis przypisano do profilu $profileId (utworzono powiązane kopie zapasowe)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Slot zapisu $slot nie jest przypisany do profilu $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Zapis usunięto z profilu';

  @override
  String editorRestoredBackup(String path) {
    return 'Przywrócono kopię zapasową: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Przywrócono kopię zapasową: $path (PersistentDataList.sav pozostawiono bez zmian, ponieważ nie ma pasującej powiązanej kopii; metadane slotu mogą się różnić)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Test pełnego cyklu kodeka zakończony powodzeniem: fragment $chunkIndex ponownie skompresowano do $bytes bajtów';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Nie udało się zapisać poziomu trudności profilu: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Nie udało się przypisać zapisu do profilu: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Nie udało się usunąć zapisu z profilu: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Nie udało się zapisać zmian: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Nie udało się przeskanować zapisów: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Nie udało się sprawdzić zapisu: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Nie udało się załadować kopii zapasowych: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Nie udało się przywrócić kopii zapasowej: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Przywrócono kopię zapasową: $path, ale nie udało się ponownie wczytać zapisu: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Sprawdzanie kodeka nie powiodło się: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Test pełnego cyklu kodeka nie powiódł się: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Wyszukiwanie właściwości nie powiodło się: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Wybrany zapis zmienił się podczas wczytywania atrybutów bohatera.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Nie udało się wczytać umiejętności: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Zapytanie o postęp nie powiodło się: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Nie udało się wczytać listy NPC: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Nie udało się wczytać listy postaci: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Nie udało się wczytać atrybutów NPC: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Nie udało się wczytać pozycji NPC: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Nie udało się wczytać ekwipunku NPC: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Nie udało się wczytać listy frakcji: $details';
  }

  @override
  String get editorNoBackupPath => 'brak';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix: $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix: $backupPath; kopia zapasowa PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Nie udało się pobrać stanu lokalizacji: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Nie udało się wyodrębnić danych: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Nie udało się wczytać Glosariusza: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Błąd kopii zapasowej: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Zadanie',
      'document': 'Dokument',
      'story': 'Fabuła',
      'exploration': 'Eksploracja',
      'combat': 'Walka',
      'social': 'Relacje',
      'item': 'Przedmioty',
      'learning': 'Nauka',
      'guild': 'Gildia',
      'crime': 'Przestępstwo',
      'rest': 'Odpoczynek',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Rozpoczęto zadanie',
      'questSucceeded': 'Ukończono zadanie',
      'questFailed': 'Zadanie nieudane',
      'documentRead': 'Przeczytano dokument',
      'documentSegmentUnlocked': 'Odkryto wpis',
      'documentSegmentViewed': 'Wyświetlono wpis',
      'chapterCompleted': 'Ukończono rozdział',
      'areaEntered': 'Wejście do obszaru',
      'areaLeft': 'Opuszczenie obszaru',
      'characterKilled': 'Zabito postać',
      'characterDefeated': 'Pokonano postać',
      'combatDodge': 'Uniknięto ataku',
      'characterDebuffed': 'Nałożono osłabienie',
      'tradeAvailable': 'Odblokowano handel',
      'itemObtained': 'Zdobyto przedmiot',
      'itemCrafted': 'Wytworzono przedmiot',
      'skillStateRecorded': 'Zapisano stan umiejętności',
      'recipeLearned': 'Nauczono się receptury',
      'guildJoined': 'Dołączono do gildii',
      'crimeRecorded': 'Zarejestrowano przestępstwo',
      'slept': 'Sen',
      'storyEvent': 'Wydarzenie fabularne',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action: $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'Czas gry',
      'duration': 'Czas trwania',
      'chapter': 'Rozdział',
      'instigator': 'Sprawca',
      'affected': 'Dotyczy',
      'amount': 'Ilość',
      'primaryObject': 'Obiekt',
      'secondaryObject': 'Kontekst',
      'segmentText': 'Tekst wpisu',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Dzień $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value s';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Bohater';

  @override
  String get memoryEventDetails => 'Szczegóły';

  @override
  String get memoryEventTags => 'Tagi';

  @override
  String get memoryEventTechnicalData => 'Dane techniczne';

  @override
  String get memoryEventIndex => 'Indeks';

  @override
  String get memoryEventPosition => 'Pozycja';

  @override
  String get memoryEventPayload => 'Dane zdarzenia';

  @override
  String get memoryEventSubject => 'Obiekt zdarzenia';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Dostęp',
      'AccessDenied': 'Dostęp zabroniony',
      'AccesToTemple': 'Dostęp do świątyni',
      'Advice': 'Rada',
      'AfterFight': 'Po walce',
      'AfterFireMages': 'Po Magach Ognia',
      'AfterNek': 'Po Neku',
      'AfterQuest': 'Po zadaniu',
      'Alone': 'Samotny',
      'Amulet': 'Amulet',
      'Annoying': 'Irytujący',
      'Armor': 'Pancerz',
      'Avoid': 'Unikanie',
      'Backstory': 'Historia',
      'BackStory': 'Historia',
      'BasicMagic': 'Podstawy magii',
      'Beated': 'Pobity',
      'BecomeMercenary': 'Dołączenie do najemników',
      'Beer': 'Piwo',
      'Bestiary': 'Bestiariusz',
      'Blessing': 'Błogosławieństwo',
      'Boss': 'Przywódca',
      'Bully': 'Dręczyciel',
      'BullyAdvice': 'Rada w sprawie dręczyciela',
      'Camp': 'Obóz',
      'CampDivided': 'Podzielony obóz',
      'CareOfMessengers': 'Opieka nad posłańcami',
      'ChangeOpinion': 'Zmiana zdania',
      'ChargeUriziel': 'Naładowanie Uriziela',
      'Chosen': 'Wybrany',
      'Contact': 'Kontakt',
      'Courier': 'Kurier',
      'CraftBows': 'Wytwarzanie łuków',
      'Crazy': 'Szalony',
      'DailyMeal': 'Codzienny posiłek',
      'DailyRation_Trader': 'Handlarz dziennymi racjami',
      'DAM': 'Tama',
      'Dead': 'Martwy',
      'Deal': 'Umowa',
      'Dealer': 'Kupiec',
      'Deceived': 'Oszukany',
      'Dementia': 'Demencja',
      'DenyAccess': 'Odmowa dostępu',
      'DifferentOpinion': 'Odmienne zdanie',
      'Discussion': 'Dyskusja',
      'DontTalk': 'Nie rozmawiaj',
      'Duel': 'Pojedynek',
      'Entrance': 'Wejście',
      'Escape': 'Ucieczka',
      'Extended': 'Rozszerzone',
      'Extra': 'Dodatkowe',
      'ExtraInfo': 'Dodatkowa informacja',
      'Fanatic': 'Fanatyk',
      'Fight': 'Walka',
      'FindUlumulu': 'Odnalezienie Ulu-Mulu',
      'FireMages': 'Magowie Ognia',
      'FireMagesEscape': 'Ucieczka Magów Ognia',
      'FiskNewDealer': 'Nowy paser dla Fiska',
      'FiskNewDealerCompleted': 'Nowy paser dla Fiska — ukończone',
      'FogTower': 'Wieża Mgieł',
      'Food': 'Żywność',
      'Forgave': 'Wybaczył',
      'Forgive': 'Wybaczyć',
      'Forgiven': 'Wybaczono',
      'FourFriends': 'Czterej przyjaciele',
      'FreeHut': 'Wolna chata',
      'FreeMine': 'Wolna Kopalnia',
      'Fury': 'Furia',
      'GoodTeacher': 'Dobry nauczyciel',
      'Gossip': 'Plotki',
      'GotScavenger': 'Zdobyty ścierwojad',
      'GrantedAccess': 'Dostęp przyznany',
      'GRDArmor': 'Zbroja strażnika',
      'Guide': 'Przewodnik',
      'HateMages': 'Nienawiść do magów',
      'HateMagesExplanation': 'Powód nienawiści do magów',
      'HateRiceLord': 'Nienawiść do Ryżowego Księcia',
      'Heal': 'Leczenie',
      'Healing': 'Uzdrowienie',
      'Help': 'Pomoc',
      'Helper': 'Pomocnik',
      'HelpKagan': 'Pomoc Kaganowi',
      'HutStory': 'Historia chaty',
      'Ignore': 'Ignorowanie',
      'Impress': 'Zaimponować',
      'ImpressAlchemy': 'Imponowanie — alchemia',
      'ImpressInscription': 'Imponowanie — inskrypcje',
      'Info': 'Informacja',
      'Interested': 'Zainteresowany',
      'Introduction': 'Przedstawienie',
      'Introduction_2': 'Przedstawienie 2',
      'Introduction_Armor': 'Prezentacja zbroi',
      'Introduction_Teacher': 'Przedstawienie — nauczyciel',
      'Introduction_Trader': 'Przedstawienie — handlarz',
      'Invocation': 'Przywołanie',
      'JoinSC': 'Dołączenie do Obozu Bractwa',
      'Joint': 'Skręt z bagiennego ziela',
      'KalomCamp': 'Obóz Cor Kaloma',
      'Leader': 'Przywódca',
      'Learning': 'Nauka',
      'LearnOrcish': 'Nauka języka orków',
      'LeftParty': 'Opuścił drużynę',
      'Library': 'Biblioteka',
      'Lie': 'Kłamstwo',
      'Lock': 'Zamek',
      'Lockpick': 'Wytrych',
      'Mad': 'Obłąkany',
      'Mandibles': 'Żuwaczki',
      'MapMaker': 'Kartograf',
      'Monastery': 'Klasztor',
      'MordragKO': 'Mordrag znokautowany',
      'Nek': 'Nek',
      'NewCamp': 'Nowy Obóz',
      'NewCamper': 'Nowy obozowicz',
      'NewLeader': 'Nowy przywódca',
      'NightPatrol': 'Nocny patrol',
      'NotInterested': 'Brak zainteresowania',
      'OldCamp': 'Stary Obóz',
      'OrcEnclaveEntrance': 'Wejście do enklawy orków',
      'OrcGraveyard': 'Cmentarzysko Orków',
      'OreArmor': 'Zbroja z magicznej rudy',
      'Party': 'Drużyna',
      'Pay': 'Zapłata',
      'PayMoney': 'Zapłata',
      'Permission': 'Pozwolenie',
      'Pet': 'Zwierzak',
      'PreparingInvocation': 'Przygotowanie przywołania',
      'Quest': 'Zadanie',
      'RankUpFireMages': 'Awans na Maga Ognia',
      'RankUpGuard': 'Awans na strażnika',
      'RanUpFireMagesCompleted': 'Awans na Maga Ognia ukończony',
      'Realocated': 'Przeniesiony',
      'Reason': 'Powód',
      'Respect': 'Szacunek',
      'ReturnToSC': 'Powrót do Obozu Bractwa',
      'RicelordForeman': 'Nadzorca Ryżowego Księcia',
      'RideScavenger': 'Jazda na ścierwojadzie',
      'Robe': 'Szata',
      'Safe': 'W bezpiecznym miejscu',
      'Scraper': 'Kret',
      'SecondChance': 'Druga szansa',
      'SecretLocation': 'Sekretne miejsce',
      'SecretPassage': 'Tajne przejście',
      'SecretPath': 'Tajna ścieżka',
      'SleeperFollower': 'Wyznawca Śniącego',
      'SleeperTemple': 'Świątynia Śniącego',
      'SmallInfo': 'Drobna informacja',
      'Stonehenge': 'Kamienny krąg',
      'StopFollowing': 'Przestań podążać',
      'SwampCamp': 'Obóz Bractwa',
      'Talkative': 'Gadatliwy',
      'Teach': 'Nauczanie',
      'TeachBow': 'Nauka łucznictwa',
      'Teacher': 'Nauczyciel',
      'Teacher2': 'Nauczyciel 2',
      'TeacherInscription': 'Nauczyciel inskrypcji',
      'TeacherMana': 'Nauczyciel many',
      'TeachIchor': 'Nauka pozyskiwania juchy pełzaczy',
      'TeachMagic': 'Nauka magii',
      'TeachOrcish': 'Nauczanie języka orków',
      'TeachStats': 'Trening atrybutów',
      'TeachWeapon': 'Trening broni',
      'Teleport': 'Teleportacja',
      'TheMysteriousOrc': 'Tajemniczy ork',
      'ThroneRoom': 'Sala tronowa',
      'TradeBow': 'Handel łukami',
      'Trader': 'Handlarz',
      'TradeSkins_Trader': 'Handlarz skór',
      'Traitor': 'Zdrajca',
      'Trial': 'Próba',
      'TrollCanyon': 'Kanion trolli',
      'Trust': 'Zaufanie',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Niedoświadczony',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Runa Uriziela',
      'Useful': 'Przydatny',
      'Velaya': 'Velaya',
      'Vibrations': 'Wibracje',
      'WaitFreeMine': 'Oczekiwanie w Wolnej Kopalni',
      'WaitInTrainingArea': 'Oczekiwanie na placu treningowym',
      'Warning': 'Ostrzeżenie',
      'WarningTooLate': 'Spóźnione ostrzeżenie',
      'WaterMessenger': 'Posłaniec Magów Wody',
      'Weapon': 'Broń',
      'Who': 'Kim jest',
      'Women': 'Kobiety',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Uszkodzone sloty ekwipunku';

  @override
  String slotRepairBody(int count) {
    return 'Ten zapis gry zawiera $count slotów ekwipunku, których identyfikator nie odpowiada już ich pozycji — w grze wyrzucenie takiego przedmiotu usuwa inny. Naprawa zmienia wyłącznie identyfikatory: żaden przedmiot nie zostanie dodany, usunięty ani zmieniony. Przy zapisie tworzona jest kopia zapasowa, jak zawsze.';
  }

  @override
  String get slotRepairQueued =>
      'Naprawa w kolejce — zapisz, aby ją zastosować.';

  @override
  String get slotRepairAction => 'Napraw';

  @override
  String get slotRepairDiscard => 'Odrzuć';

  @override
  String get editorInventorySlotEditConflict =>
      'W kolejce jest bezpośrednia zmiana slotu ekwipunku razem z operacją zajmującą całe sloty (naprawa, dodanie lub usunięcie). Druga nadpisałaby pierwszą — cofnij jedną z nich i zapisz ponownie.';

  @override
  String get editorTraderArrayConflict =>
      'Zmiana handlu jest w kolejce razem z bezpośrednią edycją tablicy kupców. Ta edycja przenumerowuje wiersze, po których adresowana jest zmiana handlu, więc jedna z nich trafiłaby w niewłaściwego kupca — cofnij jedną i zapisz ponownie.';

  @override
  String get backupFactFile => 'Plik';

  @override
  String get renameBackupTooltip => 'Nazwij tę kopię';

  @override
  String get renameBackupTitle => 'Nazwa kopii zapasowej';

  @override
  String get renameBackupLabel => 'Nazwa';

  @override
  String renameBackupHelp(String fileName) {
    return 'Wyświetlana zamiast nazwy pliku $fileName. Puste pole usuwa nazwę; sam plik nie jest zmieniany.';
  }

  @override
  String get deleteBackupTooltip => 'Usuń tę kopię zapasową';

  @override
  String get deleteBackupTitle => 'Usuń kopię zapasową';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Usunąć „$name” ($fileName)? Plik zostanie skasowany z dysku i nie da się go przywrócić.';
  }

  @override
  String get deleteBackupConfirm => 'Usuń';

  @override
  String editorDeletedBackup(String path) {
    return 'Usunięto kopię zapasową: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Nie udało się usunąć kopii zapasowej: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Nie udało się nazwać kopii zapasowej: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Naprawa nie jest teraz możliwa — tego zapisu gry nie da się zapisać.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Usunięto kopię zapasową: $path — nie udało się usunąć jej nazwy: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'Naprawa nie jest dostępna dla tego zapisu gry.';
}
