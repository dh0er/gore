// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Czech (`cs`).
class AppLocalizationsCs extends AppLocalizations {
  AppLocalizationsCs([String locale = 'cs']) : super(locale);

  @override
  String get debugSectionTitle => 'Pokročilé (ladění)';

  @override
  String get debugSectionSubtitle =>
      'Diagnostika a surová data pro hlášení chyb';

  @override
  String get showObjectIdsTitle => 'Zobrazit další technická ID';

  @override
  String get showObjectIdsSubtitle =>
      'V editoru zobrazí technická ID předmětů, dialogových znalostí, úkolů a osiřelých aktérů. ID NPC se zobrazují vždy.';

  @override
  String get storyStateSidebar => 'Stav příběhu';

  @override
  String get storyStateDescription =>
      'Hra si tu sleduje postup v úkolech, dialozích a událostech. „Uloženo“ ukazuje hodnoty z tvého uložení; „Nenastaveno“ ostatní známé položky. Podle položky může číslo znamenat ano/ne, počet nebo stupeň postupu. Časové značky ukazují den a čas ve hře.';

  @override
  String get storyStateReadOnly =>
      'Jen pro čtení, dokud nebude ověřen skriptový význam hodnot a bezpečný zápis mapy. Související text glosáře je kontext, ne přímý překlad technického ID.';

  @override
  String get storyStateStructureReadOnly =>
      'Strukturu StoryPropertyValues v tomto uložení se nepodařilo jednoznačně a bezpečně vyřešit. Hodnoty příběhu zůstávají u tohoto uložení jen pro čtení.';

  @override
  String get storyStateSearch => 'Hledat ve stavu příběhu';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown z $total příběhových hodnot';
  }

  @override
  String get storyStateInteger => 'Celé číslo';

  @override
  String get storyStateTimeMarker => 'Časová značka';

  @override
  String get storyStateChapter => 'Kapitola';

  @override
  String get storyStateUnknown => 'Neznámý typ zdroje';

  @override
  String storyStateShowDormant(int count) {
    return 'Zobrazit nepoužívané ($count)';
  }

  @override
  String get storyStateUnknownDetail =>
      'Toto uložené ID chybí v aktuálním katalogu skriptů (například z modu nebo novější verze hry). Hodnota v uložení je int32, ale její význam se neodvozuje.';

  @override
  String get storyStateStored => 'Uloženo';

  @override
  String get storyStateUnset => 'Nenastaveno';

  @override
  String get storyStateUnsetDetail =>
      'Toto katalogové pole v tomto uložení není serializované; hra proto použije nenastavený nebo výchozí stav.';

  @override
  String get storyStateRawValue => 'Surová hodnota';

  @override
  String storyStateElapsed(String duration) {
    return 'Uplynulo při uložení: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Před časem uložení: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days dnů',
      few: '$days dny',
      one: '1 den',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Související položka glosáře';

  @override
  String get storyStateTechnicalPath => 'Technická cesta';

  @override
  String get storyStateEditingGuidance =>
      'Vyber položku a změň její hodnotu. Změny se projeví až po uložení. Měň jen hodnoty, jejichž účinek znáš: jinak mohou úkoly nebo dialogy přestat fungovat očekávaně. Při uložení se automaticky vytvoří záloha.';

  @override
  String get storyStatePending => 'Čeká';

  @override
  String storyStatePendingValue(String value) {
    return 'Uloží se jako $value';
  }

  @override
  String get storyStatePendingRemoval => 'Odstraní se z uložení';

  @override
  String get storyStateEditValue => 'Upravit hodnotu';

  @override
  String get storyStateSetValue => 'Nastavit hodnotu';

  @override
  String get storyStateRemoveValue => 'Odstranit z uložení';

  @override
  String get storyStateUndoChange => 'Vrátit změnu příběhu';

  @override
  String get storyStateResetChanges => 'Resetovat změny příběhu';

  @override
  String storyStateDialogTitle(String id) {
    return 'Upravit $id';
  }

  @override
  String get storyStateRawInput => 'Znaménková hodnota int32';

  @override
  String get storyStateInvalidInt32 =>
      'Zadej celé číslo od -2147483648 do 2147483647.';

  @override
  String get storyStateQueueChange => 'Zařadit změnu';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Hodnoty doložené v dodaných skriptech: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Návrhy nejsou limity validace; nativní kód, mody nebo novější verze hry mohou používat jiné hodnoty.';

  @override
  String get storyStateUseCurrentTime => 'Použít aktuální čas uložení';

  @override
  String get storyStateStructuredTime => 'Den / čas';

  @override
  String get storyStateRawMode => 'Surové int32';

  @override
  String get storyStateChapterWarning =>
      'Samotná změna kapitoly nesynchronizuje úkoly, NPC, inventář ani stav světa.';

  @override
  String get storyStateDormantWarning =>
      'V mezipaměti dodaných skriptů se pro toto pole nenašel aktivní čtení ani zápis. Může jít o starší, nativně řízené nebo rezervované pole.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Dodané skripty toto pole čtou, ale neobsahují skriptový zápis. Nativní kód ho přesto může spravovat.';

  @override
  String get storyStateUnknownEditWarning =>
      'Toto ID z modu nebo novější verze nemá přiloženou sémantiku zdroje. Upravuj jen jeho surovou hodnotu int32.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Binární příznak',
      'finiteState': 'Vícestavová hodnota',
      'counterOrScore': 'Počitadlo / skóre',
      'calendarDay': 'Kalendářní den',
      'derivedOrOpaqueInteger': 'Odvozené / neprůhledné číslo',
      'readOnlyInSourceInteger': 'V dodaných skriptech jen pro čtení',
      'dormantOrLegacyInteger': 'V dodaných skriptech nepoužívané',
      'other': 'Celé číslo',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Uložená 0 a chybějící položka mapy jsou různé stavy souboru. „Odstranit z uložení“ obnoví konstruktorový/výchozí stav.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo GORE Save Editor';

  @override
  String get zoomTooltip => 'Stiskni Ctrl +/- pro přiblížení/oddálení';

  @override
  String get switchToLightMode => 'Přepnout na světlý režim';

  @override
  String get switchToDarkMode => 'Přepnout na tmavý režim';

  @override
  String get about => 'O aplikaci';

  @override
  String get tabOverview => 'Přehled';

  @override
  String get tabPlayer => 'Hráč';

  @override
  String get tabAttribute => 'Atributy';

  @override
  String get heroGroupSkills => 'Dovednosti';

  @override
  String get skillsNoneBody =>
      'U této postavy nebyly nalezeny žádné dovednosti.';

  @override
  String get skillsUnavailableBody =>
      'Dovednosti v tomto uložení nelze upravit — hrdina nemá data efektů ke změně.';

  @override
  String get skillNotLearned => 'Nenaučeno';

  @override
  String get skillLearn => 'Naučit';

  @override
  String get skillActionLearn => 'naučit';

  @override
  String get skillActionUnlearn => 'odnaučit';

  @override
  String get skillTierUntrained => 'Nevycvičený';

  @override
  String get skillTierBeginner => 'Začátečník';

  @override
  String get skillTierTrained => 'Vycvičený';

  @override
  String get skillTierMaster => 'Mistr';

  @override
  String get skillTierNovice => 'Novic';

  @override
  String get skillTierAmateur => 'Amatér (Kruh 0)';

  @override
  String get skillTierLearned => 'Naučeno';

  @override
  String skillTierCircle(int n) {
    return 'Kruh $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Jednoruční zbraně';

  @override
  String get skillHintBlacksmith2H => 'Obouruční zbraně';

  @override
  String get skillScutesTrained => 'Vycvičený (kostěné štítky)';

  @override
  String get skillScutesMaster => 'Mistr (+ břitové destičky)';

  @override
  String get skillCategoryCombat => 'Boj';

  @override
  String get skillCategoryCrafting => 'Řemeslo';

  @override
  String get skillCategoryHunting => 'Lov';

  @override
  String get skillCategoryLanguage => 'Jazyk';

  @override
  String get skillCategoryMagic => 'Magie';

  @override
  String get skillCategoryMovement => 'Pohyb';

  @override
  String get skillCategoryThievery => 'Zlodějství';

  @override
  String get skillCategoryOther => 'Ostatní';

  @override
  String get skillNameOneHanded => 'Jednoruční';

  @override
  String get skillNameTwoHanded => 'Obouruční';

  @override
  String get skillNameFists => 'Pěsti';

  @override
  String get skillNameBow => 'Luk';

  @override
  String get skillNameCrossbow => 'Kuše';

  @override
  String get skillNameLockpicking => 'Páčení zámků';

  @override
  String get skillNamePickpocketing => 'Kapsářství';

  @override
  String get skillNameTakeOrgans => 'Vyjmout orgán';

  @override
  String get skillNameBreakTeeth => 'Vyjmout zuby';

  @override
  String get skillNameTakeClaws => 'Vyjmout dráp';

  @override
  String get skillNameSkinFur => 'Sejmout kožešinu';

  @override
  String get skillNameSkin => 'Sejmout kůži';

  @override
  String get skillNameTakeFins => 'Sejmout ploutve';

  @override
  String get skillNameTakeStingers => 'Vyjmout žihadla';

  @override
  String get skillNameTakeSecretion => 'Vyjmout sekret';

  @override
  String get skillNameTakeSkullPlates => 'Sejmout lebeční pancíř';

  @override
  String get skillNameSkinSwampshark => 'Sejmout žraločí kůži';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Sejmout destičky';

  @override
  String get skillNameTakeScutes => 'Sejmout štítky';

  @override
  String get skillNameTakeUluMulu => 'Sejmout Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Skřetí zbraně';

  @override
  String get skillNameMining => 'Těžba';

  @override
  String get skillNameDiving => 'Potápění';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Vyjmout kusadla';

  @override
  String get skillNameTakeShadowbeastHorn => 'Sejmout roh (stínová šelma)';

  @override
  String get skillNameTakeSpines => 'Vyjmout páteř';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Vyjmout žraločí zuby';

  @override
  String get skillNameTakeFireTongue => 'Sejmout ohnivý jazyk';

  @override
  String get skillNameTakeTrollHorn => 'Sejmout roh (troll)';

  @override
  String get skillNameAcrobatics => 'Akrobacie';

  @override
  String get skillNameWallClimbing => 'Lezení';

  @override
  String get skillNameRiding => 'Jízda na mrchožroutovi';

  @override
  String get skillNameSneaking => 'Plížení';

  @override
  String get skillNameAlchemy => 'Alchymie';

  @override
  String get skillNameRuneInscription => 'Rytí run';

  @override
  String get skillNameBlacksmithing => 'Kovářství';

  @override
  String get skillNameMagicCircle => 'Magický kruh';

  @override
  String get skillNameOrcish => 'Skřetština';

  @override
  String get tabInventory => 'Inventář';

  @override
  String get tabTrade => 'Obchod';

  @override
  String get traderNotAMerchant => 'Tato postava neobchoduje.';

  @override
  String get traderRetry => 'Zkusit znovu';

  @override
  String get traderAmbiguousName =>
      'Stejné jméno nese víc záznamů obchodníků, takže editor nepozná, který obchod patří této postavě. Úpravy jsou vypnuté, aby se omylem nezměnil ten špatný.';

  @override
  String get traderOre => 'Ruda (kupní síla)';

  @override
  String get traderNoOre => 'žádná ruda';

  @override
  String get traderStockCurrent => 'Zásoba';

  @override
  String get traderStockCurrentTooltip =>
      'Co má tento obchodník právě na prodej. Přidané předměty můžou zase zmizet, až hra obchodníka aktualizuje.';

  @override
  String get traderStockBase => 'Základ pro doplnění';

  @override
  String get traderStockBaseTooltip =>
      'Uložení obsahuje tento seznam, aby hra mohla obchodníka doplnit. Hra ho může přepočítat podle svých pravidel, takže změny tu nevydrží.';

  @override
  String get traderStockBaseHint =>
      'Jen pro čtení: hra tento seznam používá při doplnění, ale může ho přepočítat. Předměty přidané sem by nezůstaly trvale.';

  @override
  String get traderCurrentStockWarning =>
      'Změny v inventáři obchodníka vydrží jen do příštího doplnění.';

  @override
  String get traderRestockTitle => 'Časovač doplnění';

  @override
  String get traderRestockTitleTooltip =>
      'Odhad podle poslední aktivity obchodníka, aktuálního herního času a obtížnosti Zdrojů.';

  @override
  String get traderRestockPending => 'čeká';

  @override
  String get traderRestockRevertTooltip => 'Vrátit čekající změnu času';

  @override
  String get traderRestockNever => 'Nikdy';

  @override
  String get traderRestockUnavailable => 'Nedostupné';

  @override
  String get traderRestockIntervalUnknown => 'Čekání na doplnění neznámé';

  @override
  String get traderRestockNeverStatus =>
      'Zatím nebyla zaznamenána žádná aktivita obchodníka.';

  @override
  String get traderRestockClockAhead =>
      'Uložený čas obchodníka je před aktuálním herním časem.';

  @override
  String traderRestockNotDueYet(String time) {
    return 'Neočekává se dříve než $time.';
  }

  @override
  String get traderRestockPossiblyDue =>
      'Obchodník už možná může být připravený k doplnění.';

  @override
  String get traderRestockEligible =>
      'Obchodník by teď měl být připravený k doplnění.';

  @override
  String get traderRestockNoWorldTime =>
      'Aktuální herní čas není k dispozici, takže editor nepozná, jestli je čas na doplnění.';

  @override
  String get traderRestockLastActivity => 'Poslední aktivita obchodníka';

  @override
  String get traderRestockLastActivityTooltip =>
      'Poslední uložený čas tohoto obchodníka. Může pocházet z obchodu nebo jiné aktualizace, takže nemusí jít o poslední doplnění.';

  @override
  String get traderRestockForecastWindow => 'Očekávané doplnění';

  @override
  String get traderRestockForecastWindowTooltip =>
      'Přesný čas v uložení není. Editor proto ukazuje rozsah od nejdřívějšího po nejpozdější očekávaný čas.';

  @override
  String get traderRestockIntervalLabel => 'Čekání na doplnění';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days dnů · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      'Čekací doba podle obtížnosti Zdrojů: Novic 2, Gothic 3, Těžká 5 herních dnů.';

  @override
  String get traderRestockAutomationLabel => 'Automatické doplnění';

  @override
  String get traderRestockAutomationValue => 'V uložení nelze vypnout';

  @override
  String get traderRestockAutomationTooltip =>
      'Editor uložení nedokáže spolehlivě vypnout automatické doplnění. To vyžaduje herní mod.';

  @override
  String get traderRestockSetNow => 'Nastavit na čas světa';

  @override
  String get traderRestockSetNowTooltip =>
      'Použij aktuální herní čas jako poslední aktivitu obchodníka. Tím se odloží příští očekávané doplnění.';

  @override
  String get traderRestockMakeDue => 'Zpřístupnit teď';

  @override
  String get traderRestockMakeDueTooltip =>
      'Posuň poslední aktivitu obchodníka dost daleko dozadu, aby už mělo být doplnění na řadě.';

  @override
  String get traderRestockCustom => 'Vlastní čas…';

  @override
  String get traderRestockCustomTooltip =>
      'Zvol herní den a čas poslední aktivity obchodníka.';

  @override
  String get traderRestockEditTitle => 'Změnit poslední aktivitu obchodníka';

  @override
  String get traderOreHint =>
      'Číslo ve hře se liší: při načtení hra přičte, co narostlo od jeho posledního obchodu — prodá přebytky a z toho doplní zásoby. Toto číslo je výchozí bod, ne to, co ukazuje obrazovka obchodu.';

  @override
  String get traderOreHintShort =>
      'Výchozí hodnota — částka na obrazovce obchodu se může lišit.';

  @override
  String get traderRestockStatusLabel => 'Stav';

  @override
  String get traderRestockStatusNever => 'Bez aktivity';

  @override
  String get traderRestockStatusWaiting => 'Čeká na doplnění';

  @override
  String get traderRestockStatusReady => 'Připraven k doplnění';

  @override
  String get traderRestockStatusPossiblyReady => 'Možná připraven';

  @override
  String get traderRestockStatusCheckTime => 'Zkontroluj uložený čas';

  @override
  String get traderRestockStatusUnknown => 'Neznámý';

  @override
  String get traderPriceWarning =>
      'Ceny reagují na to, kolik má obchodník na skladě a kolik rudy drží, takže změna těchto čísel může posunout i jeho ceny.';

  @override
  String get traderAddItem => 'Přidat předmět';

  @override
  String get traderRemoveItem => 'Odebrat řádek';

  @override
  String get traderReadOnlyCore => 'Toto jádro umí data obchodníků jen číst.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Tento obchodník má zásoby podle obtížnosti, které editor nemodeluje. Úpravy jsou tu vypnuté, protože změna by vypadala úspěšně, ale tu zásobu by nechala beze změny.';

  @override
  String get traderRecordIncomplete =>
      'Seznamy zásob tohoto obchodníka chybí, nebo mají tvar, který editor nepodporuje a neumí zapsat. Úpravy jsou vypnuté, aby změna neselhala při uložení.';

  @override
  String get traderEmptyStock => 'Nic na skladě.';

  @override
  String get traderUnknownItem => 'není v katalogu předmětů';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Načtení obchodníků selhalo: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count řádků';
  }

  @override
  String get tabWorld => 'Svět';

  @override
  String get tabCharacters => 'Postavy';

  @override
  String get characterNoActorBody =>
      'Tato postava nemá aktéra ve světě, takže nemá atributy, inventář ani události.';

  @override
  String get characterNoEventsBody => 'Žádné události pro tuto postavu.';

  @override
  String get characterOrphanGroup => 'Ostatní';

  @override
  String get tabAllData => 'Všechna data';

  @override
  String get tabBackups => 'Zálohy';

  @override
  String get tabSettings => 'Nastavení';

  @override
  String get reset => 'Resetovat';

  @override
  String get save => 'Uložit';

  @override
  String saveWithCount(int count) {
    return 'Uložit ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Zrušit';

  @override
  String get confirm => 'Potvrdit';

  @override
  String get close => 'Zavřít';

  @override
  String get add => 'Přidat';

  @override
  String get equippedBadge => 'Nasazeno';

  @override
  String get armorUpgradesLabel => 'Vylepšení';

  @override
  String get browse => 'Procházet';

  @override
  String get noSavFilesFound => 'Nebyly nalezeny žádné soubory .sav';

  @override
  String get profile => 'Profil';

  @override
  String get otherSaves => 'Jiná uložení';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count uložení)';
  }

  @override
  String get switchProfile => 'Přepnout profil';

  @override
  String get openSaveFile => 'Otevřít soubor';

  @override
  String get externalSave => 'Externě otevřené uložení';

  @override
  String get saveProfileTitle => 'Profil uložení';

  @override
  String get saveProfileDescription =>
      'Přiřaď toto uložení k jinému hernímu profilu. Uložení a index profilu se zálohují společně.';

  @override
  String get saveProfileExternalHint =>
      'Vyber profil, do kterého se má tento soubor importovat do herní složky uložení a zaregistrovat. Původní soubor zůstane beze změny.';

  @override
  String get saveProfileNoProfiles =>
      'V PersistentDataList.sav nebyly nalezeny žádné upravitelné herní profily.';

  @override
  String get saveProfileSelect => 'Vybrat profil';

  @override
  String get rescanSaveFolder => 'Znovu prohledat složku uložení';

  @override
  String get discardUnsavedChangesTitle => 'Zahodit neuložené změny?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'změn',
      few: 'změny',
      one: 'změnu',
    );
    return 'Opětovné prohledání znovu načte všechna uložení a zahodí tvých $count neuložených $_temp0.';
  }

  @override
  String get discardAndRescan => 'Zahodit a prohledat znovu';

  @override
  String chapterLabel(Object id) {
    return 'Kapitola $id';
  }

  @override
  String get quickSave => 'Rychlé uložení';

  @override
  String get autoSave => 'Automatické uložení';

  @override
  String get manualSave => 'Ruční uložení';

  @override
  String get errorTitle => 'Chyba';

  @override
  String get selectASaveTitle => 'Vyber uložení';

  @override
  String get selectASaveBody => 'Podrobnosti uložení se zobrazí tady.';

  @override
  String bytesValue(String count) {
    return '$count bajtů';
  }

  @override
  String get inspectionJsonTitle => 'Inspekční JSON';

  @override
  String get copy => 'Kopírovat';

  @override
  String get savegameFallbackTitle => 'Uložení hry';

  @override
  String screenshotForSlot(String slot) {
    return 'Snímek obrazovky pro $slot';
  }

  @override
  String get publicSaveName => 'Název';

  @override
  String get gameTimeTitle => 'Herní čas';

  @override
  String get gameTimeDay => 'Den';

  @override
  String get gameTimeHours => 'Hodiny';

  @override
  String get gameTimeMinutes => 'Minuty';

  @override
  String get gameTimeSeconds => 'Sekundy';

  @override
  String gameTimeTotal(int seconds) {
    return '= celkem $seconds s';
  }

  @override
  String get gameTimeInvalid =>
      'Zadej celá čísla — den ≥ 0, hodiny 0–23, minuty a sekundy 0–59.';

  @override
  String get required => 'Povinné';

  @override
  String get playerLockedBody =>
      'Úpravy soukromých dat hráče vyžadují kodek připravený ke kompresi.';

  @override
  String get heroTransform => 'Pozice';

  @override
  String get locationX => 'Pozice X';

  @override
  String get locationY => 'Pozice Y';

  @override
  String get locationZ => 'Pozice Z';

  @override
  String get rotationPitch => 'Natočení pitch';

  @override
  String get rotationYaw => 'Natočení yaw';

  @override
  String get rotationRoll => 'Natočení roll';

  @override
  String get spawnPositionSection => 'Pozice spawnu (reference)';

  @override
  String get resetToSpawnPosition => 'Vrátit na pozici spawnu';

  @override
  String get positionOutOfRange =>
      'Hodnota musí být mezi −10 000 000 a 10 000 000';

  @override
  String get positionNotEditable =>
      'Uloženou pozici této postavy se nepodařilo přečíst, takže ji nelze upravit.';

  @override
  String get positionNeverPlaced =>
      'Tato postava ještě nebyla umístěna do světa (pozice 0, 0, 0) — hra může uloženou pozici ignorovat.';

  @override
  String get npcStayInPlace => 'Vypnout jeho denní rutinu';

  @override
  String get npcStayInPlaceHint => 'Pak zůstane tam, kde je.';

  @override
  String get npcStayInPlaceLocked =>
      'Jeho původní denní rutina není zaznamenaná, takže to už nelze vrátit.';

  @override
  String get npcUndoPlacement => 'Vzít přesun zpět';

  @override
  String get npcUndoPlacementStale =>
      'Uložení už neobsahuje to, co ten přesun zapsal, takže obnovení by zahodilo, co se stalo mezitím.';

  @override
  String get positionNotReadable =>
      'Uloženou pozici této postavy se nepodařilo přečíst.';

  @override
  String get npcPositionReadOnly =>
      'Hra obnovuje pozici NPC z úrovně, ne z uložení, takže tyto hodnoty lze číst, ale ne měnit.';

  @override
  String get pickLocation => 'Zvolit místo…';

  @override
  String get pickLocationDialogTitle => 'Zvol místo';

  @override
  String get applySpotRotation => 'Také použít orientaci místa';

  @override
  String get locationAreaOther => 'Ostatní';

  @override
  String get locationAreaCavalornValley => 'Cavalornovo údolí';

  @override
  String get locationAreaEastForest => 'Východní les';

  @override
  String get locationAreaFogTower => 'Věž mlhy';

  @override
  String get locationAreaIllegalWeedMixers => 'Nelegální míchárna trávy';

  @override
  String get locationAreaOrcArena => 'Skřetí aréna';

  @override
  String get locationAreaOrcGraveyard => 'Skřetí pohřebiště';

  @override
  String get locationAreaShipwreck => 'Vrak lodi';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable => 'Katalog míst se nepodařilo načíst.';

  @override
  String get invalid => 'Neplatné';

  @override
  String get heroAttributes => 'Atributy hrdiny';

  @override
  String attributeBase(String name) {
    return '$name základ';
  }

  @override
  String attributeCurrent(String name) {
    return '$name aktuálně';
  }

  @override
  String get attributeBaseValue => 'Základní hodnota';

  @override
  String get attributeCurrentValue => 'Aktuální hodnota';

  @override
  String get inventoryTitle => 'Inventář';

  @override
  String get inventoryEmpty => 'Tento inventář je prázdný.';

  @override
  String get inventoryNeedsDecoded =>
      'Úpravy inventáře vyžadují dekódovaná soukromá data z kodeku.';

  @override
  String get inventoryNoStacks =>
      'V dekódovaných soukromých datech nebyly nalezeny žádné stohy předmětů.';

  @override
  String get resetInventoryChanges => 'Resetovat změny inventáře';

  @override
  String get addItemTooltipPendingAdd =>
      'Nejdřív ulož čekající změny — jeden nový předmět na uložení';

  @override
  String get addItemTooltipPendingRemove =>
      'Nejdřív ulož čekající odstranění — jedna strukturální změna na uložení';

  @override
  String get addItemTooltipPendingCount =>
      'Nejdřív ulož nebo resetuj čekající změny počtu — strukturální úprava se musí uložit samostatně';

  @override
  String get addItemTooltipDefault => 'Přidat předmět do inventáře';

  @override
  String get addItemButton => 'Přidat předmět';

  @override
  String get resetInventoryButton => 'Resetovat inventář';

  @override
  String get resetInventoryTooltipDefault =>
      'Nahradit tento inventář inventářem z uložení na začátku hry';

  @override
  String get resetInventoryTooltipBlocked =>
      'Nejdřív ulož nebo zruš čekající změny inventáře';

  @override
  String get pendingResetTitle => 'Resetovat na inventář ze začátku hry';

  @override
  String pendingResetSubtitle(String level) {
    return 'Úroveň zdrojů: $level';
  }

  @override
  String get cancelPendingReset => 'Zrušit reset';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — čeká na přidání (ještě neuloženo)';
  }

  @override
  String get cancelPendingAdd => 'Zrušit čekající přidání';

  @override
  String get pendingRemovalSubtitle => 'čeká na odstranění (ještě neuloženo)';

  @override
  String get cancelPendingRemoval => 'Zrušit čekající odstranění';

  @override
  String get filterItems => 'Filtrovat předměty';

  @override
  String noItemsMatchQuery(String query) {
    return 'Žádné předměty neodpovídají „$query“.';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Čekající odstranění skryje všechny předměty — ulož, aby se uplatnilo.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemTooltipIngredientFor => 'Přísada pro';

  @override
  String itemTooltipTeaches(String item) {
    return 'Učí: $item';
  }

  @override
  String get itemTooltipValue => 'Hodnota';

  @override
  String get itemTooltipProtection => 'Ochrana';

  @override
  String get itemTooltipRequirements => 'Požadavky:';

  @override
  String get itemTooltipManaCost => 'Cena many';

  @override
  String get itemTooltipManaUpkeep => 'Cena many za nabití';

  @override
  String get itemCategoryAll => 'Vše';

  @override
  String get itemCategoryMeleeWeapon => 'Zbraně na blízko';

  @override
  String get itemCategoryRangedWeapon => 'Zbraně na dálku';

  @override
  String get itemCategoryMagic => 'Magie';

  @override
  String get itemCategoryWearable => 'Oblečení';

  @override
  String get itemCategoryFood => 'Jídlo';

  @override
  String get itemCategoryPotion => 'Lektvary';

  @override
  String get itemCategoryMaterial => 'Materiály';

  @override
  String get itemCategoryDocument => 'Dokumenty';

  @override
  String get itemCategoryMisc => 'Různé';

  @override
  String get itemCategoryArtefact => 'Artefakty';

  @override
  String get itemCategoryOther => 'Ostatní';

  @override
  String get count => 'Počet';

  @override
  String get min1 => 'Min. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Nelze smazat: tento předmět je pravděpodobně nasazený nebo přiřazený ke slotu zkratky';

  @override
  String get removeBlockedTooltip =>
      'Nejdřív ulož nebo resetuj čekající změny inventáře — přidání nebo odebrání se musí uložit samostatně';

  @override
  String get removeItemFromInventory => 'Odebrat předmět z inventáře';

  @override
  String get progressionLockedBody =>
      'Data postupu vyžadují dekódovaná soukromá data z kodeku.';

  @override
  String get progressionNeedsTyped =>
      'Strukturovaná data postupu vyžadují plně dekódované uložení s ověřeným typovaným rozborem.';

  @override
  String get sectionQuests => 'Úkoly';

  @override
  String get sectionKnowledge => 'Znalosti';

  @override
  String get sectionEvents => 'Události';

  @override
  String get firstPage => 'První stránka';

  @override
  String get previousPage => 'Předchozí stránka';

  @override
  String get nextPage => 'Další stránka';

  @override
  String get lastPage => 'Poslední stránka';

  @override
  String pageOfPages(int page, int total) {
    return 'Stránka $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last z $total';
  }

  @override
  String get perPage => 'Na stránku:';

  @override
  String get resetQuestChanges => 'Resetovat změny úkolů';

  @override
  String get searchQuests => 'Hledat úkoly';

  @override
  String get allGroups => 'Všechny skupiny';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Žádný';

  @override
  String get questStateAvailable => 'Dostupný';

  @override
  String get questStateRunning => 'Probíhá';

  @override
  String get questStateSucceeded => 'Splněný';

  @override
  String get questStateFailed => 'Neúspěšný';

  @override
  String get questStateUnknown => 'neznámý';

  @override
  String get dialogKnowledge => 'Dialogové znalosti';

  @override
  String get resetKnowledgeChanges => 'Resetovat změny znalostí';

  @override
  String get addNpc => 'Přidat NPC';

  @override
  String get searchNpcs => 'Hledat NPC';

  @override
  String get npcStatusRowLabel => 'Stav';

  @override
  String get npcStatusAlive => 'naživu';

  @override
  String get npcStatusDead => 'mrtvý';

  @override
  String get npcRelationshipRowLabel => 'Vztah';

  @override
  String get npcRelationshipUnavailable => 'Stav vztahu není k dispozici';

  @override
  String get npcRelationshipAutomatic => 'Počítá hra';

  @override
  String get npcRelationshipAutomaticHint =>
      'Není uloženo trvalé přepsání. Cech, příběh, oblast a zločiny se vyhodnocují ve hře.';

  @override
  String get npcRelationshipStoredHint =>
      'Uloženo jako trvalé přepsání vztahu NPC–hráč. Cech, příběh, oblast a zločiny můžou ve hře stále změnit skutečný stav.';

  @override
  String get npcRelationshipFriend => 'Přítel';

  @override
  String get npcRelationshipNeutral => 'Neutrální';

  @override
  String get npcRelationshipEnemy => 'Nepřítel';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Při uložení bude $relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Oživit';

  @override
  String get npcReviveQueued => 'Při uložení bude oživen';

  @override
  String entriesForCharacter(String name) {
    return 'Záznamy — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Vyber NPC pro zobrazení záznamů';

  @override
  String get addKnowledgeEntry => 'Přidat záznam znalosti';

  @override
  String get browseCatalog => 'Procházet katalog';

  @override
  String get alreadyExistsForCharacter => 'U této postavy už existuje.';

  @override
  String get alreadyInPendingChanges => 'Už je v čekajících změnách.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Kontrola duplicit selhala — zkus znovu: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Čekající přidání ($count)';
  }

  @override
  String get undoAdd => 'Vrátit přidání';

  @override
  String get undoRemove => 'Vrátit odebrání';

  @override
  String get removeEntry => 'Odebrat záznam';

  @override
  String get selectNpcFromList => 'Vyber NPC ze seznamu';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Paměťové události';

  @override
  String get searchCharacters => 'Hledat postavy';

  @override
  String eventsForCharacter(String name) {
    return 'Události — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Vyber postavu pro zobrazení událostí';

  @override
  String get noTags => '(bez štítků)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Odebrat událost';

  @override
  String get removeMemoryEventTitle => 'Odebrat paměťovou událost?';

  @override
  String get removeMemoryEventBody =>
      'Zařadit tuto paměťovou událost k odebrání? Soubor uložení se změní až po stisku Uložit.';

  @override
  String get memoryEventRemovalQueued =>
      'Odebrání události zařazeno — stiskni Uložit pro uplatnění.';

  @override
  String get duplicateEvent => 'Duplikovat událost';

  @override
  String get duplicateMemoryEventTitle => 'Duplikovat paměťovou událost?';

  @override
  String get duplicateMemoryEventBody =>
      'Zařadit kopii této paměťové události? Soubor uložení se změní až po stisku Uložit.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplikace události zařazena — stiskni Uložit pro uplatnění.';

  @override
  String get selectCharacterFromList => 'Vyber postavu ze seznamu';

  @override
  String get factionsSidebar => 'Frakce';

  @override
  String get factionsForgiveButton => 'Odpustit';

  @override
  String get factionHostile => 'Nepřátelská';

  @override
  String get factionFriendly => 'Přátelská';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count vražd',
      few: '$count vraždy',
      one: '$count vražda',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count napadení',
      few: '$count napadení',
      one: '$count napadení',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count krádeží',
      few: '$count krádeže',
      one: '$count krádež',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count vniknutí',
      few: '$count vniknutí',
      one: '$count vniknutí',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count výhrůžek',
      few: '$count výhrůžky',
      one: '$count výhrůžka',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count jiných zločinů',
      few: '$count jiné zločiny',
      one: '$count jiný zločin',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'odpouští se…';

  @override
  String get factionsEmpty => 'Žádné otevřené zločiny proti frakcím.';

  @override
  String get factionGuildOldCamp => 'Starý tábor';

  @override
  String get factionGuildNewCamp => 'Nový tábor';

  @override
  String get factionGuildSwampCamp => 'Tábor v bažinách';

  @override
  String get factionGuildOther => 'Ostatní / jednotlivci';

  @override
  String get allDataLockedBody =>
      'Úplný prohlížeč zdrojových dat je momentálně k dispozici pro uložené hry ve formátu GSAV.';

  @override
  String get allDataDescription =>
      'Procházej metadata GSAV a každý typovaný uzel PUBLIC/PRIVATE. Bezpečné skalární hodnoty a nativní struktury lze upravovat; kontejnery a nečitelné bajty zůstávají viditelné.';

  @override
  String get allDataEditable => 'Upravitelné';

  @override
  String get allDataReadOnly => 'Jen pro čtení';

  @override
  String get allDataType => 'Typ';

  @override
  String get allDataScalars => 'Skaláry';

  @override
  String get allDataStructs => 'Struktury';

  @override
  String get allDataContainers => 'Kontejnery';

  @override
  String get allDataOpaque => 'Nečitelné bajty';

  @override
  String get allDataNodes => 'Uzly';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count podřízeného uzlu',
      many: '$count podřízených uzlů',
      few: '$count podřízené uzly',
      one: '1 podřízený uzel',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Čekající';

  @override
  String get allDataTagInputHint => 'Tagy oddělené čárkou nebo novým řádkem';

  @override
  String allDataTypedSource(String source) {
    return '$source typované';
  }

  @override
  String get searchPropertiesLabel =>
      'Hledat vlastnosti (prázdné = vypsat vše) — např. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Dekóduji save…';

  @override
  String get decodingSaveBody =>
      'Při prvním hledání se dekóduje celé soukromé payload. U každého save to proběhne jednou, pak je hledání okamžité.';

  @override
  String get searchTheSaveTitle => 'Prohledat save';

  @override
  String get searchTheSaveBody =>
      'Zadej název vlastnosti a stiskni Enter. Nech prázdné, ať se vypíše vše.';

  @override
  String get searchFailedTitle => 'Hledání selhalo';

  @override
  String get noMatchesTitle => 'Žádné shody';

  @override
  String get noMatchesBody =>
      'Žádná cesta vlastnosti neobsahovala všechny tyto termíny.';

  @override
  String get value => 'Hodnota';

  @override
  String get backupsTitle => 'Zálohy';

  @override
  String get refreshBackups => 'Obnovit zálohy';

  @override
  String get noBackupsTitle => 'Žádné zálohy';

  @override
  String get noBackupsBody =>
      'Upravené save soubory vytvoří záložní soubory vedle vybraného slotu.';

  @override
  String get slotBackups => 'Zálohy slotu';

  @override
  String get profileBackups => 'Zálohy profilu';

  @override
  String get backupFactName => 'Název';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Vytvořeno';

  @override
  String get backupFactSize => 'Velikost';

  @override
  String get backupFactStatus => 'Stav';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Obnovit $fileName';
  }

  @override
  String get appearanceTitle => 'Vzhled';

  @override
  String get uiFont => 'Písmo';

  @override
  String get theme => 'Motiv';

  @override
  String get themeLight => 'Světlý';

  @override
  String get themeDark => 'Tmavý';

  @override
  String get themeSystem => 'Systémový';

  @override
  String get uiScale => 'Měřítko rozhraní';

  @override
  String get resetZoomTooltip => 'Resetovat zoom (Ctrl+0)';

  @override
  String get zoomTip => 'Tip: Ctrl + / Ctrl - mění zoom kdekoli v aplikaci.';

  @override
  String get language => 'Rozhraní';

  @override
  String get gameTextLanguage => 'Text hry';

  @override
  String get gameTextLanguageHint =>
      'Volba jazyka rozhraní zároveň nastaví odpovídající text hry. Text hry můžeš potom zvolit jiný.';

  @override
  String get updatesTitle => 'Aktualizace';

  @override
  String get checkForUpdatesAutomatically =>
      'Automaticky kontrolovat aktualizace';

  @override
  String get checkForUpdatesNow => 'Zkontrolovat aktualizace teď';

  @override
  String get updatesPortableNotice =>
      'Přenosná verze otevře stránku ke stažení v prohlížeči. Nahraď stávající soubory novým stažením.';

  @override
  String get updateAvailableTitle => 'Je k dispozici aktualizace';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Je k dispozici verze $version. Máš $current.';
  }

  @override
  String get updateDownload => 'Stáhnout';

  @override
  String updateOpenFailed(String url) {
    return 'Stránku ke stažení se nepodařilo otevřít. Najdeš ji na $url';
  }

  @override
  String get updateLater => 'Později';

  @override
  String get updateUpToDate => 'Používáš nejnovější verzi.';

  @override
  String get updateCheckFailed =>
      'Aktualizace se nepodařilo zkontrolovat. Zkus to později znovu.';

  @override
  String get gameTextTitle => 'Text hry';

  @override
  String get itemImagesTitle => 'Obrázky předmětů';

  @override
  String get gameDataTitle => 'Herní data';

  @override
  String itemImagesReady(int count) {
    return 'Připraveno $count obrázků předmětů.';
  }

  @override
  String get itemImagesUnavailable =>
      'Obrázky předmětů nejsou k dispozici. Místo nich se použijí ikony kategorií.';

  @override
  String get checkRefreshItemImages =>
      'Zkontrolovat / obnovit obrázky předmětů';

  @override
  String get gameDataSourceMissing =>
      'Text hry se nepodařilo připravit automaticky. V Nastavení můžeš vybrat cache lokalizace.';

  @override
  String get loadingTexts => 'Načítám texty…';

  @override
  String get loadingImages => 'Načítám obrázky…';

  @override
  String get preparing => 'Připravuji…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extrahováno: $ids id napříč $languages jazyky.';
  }

  @override
  String get gameTextExtracted => 'Lokalizovaný text hry je extrahovaný.';

  @override
  String get gameTextNotExtracted =>
      'Lokalizovaný text hry ještě není extrahovaný.';

  @override
  String get extracting => 'Extrahuji…';

  @override
  String get extractRefreshLocalizedText =>
      'Extrahovat / obnovit lokalizovaný text';

  @override
  String get extractionComplete => 'Extrakce dokončena';

  @override
  String get extractionFailed => 'Extrakce selhala';

  @override
  String get localizationCacheFileType => 'Cache lokalizace';

  @override
  String get savegameDirectoryTitle => 'Adresář save souborů';

  @override
  String get folder => 'Složka';

  @override
  String get codecTitle => 'Kodek';

  @override
  String get check => 'Zkontrolovat';

  @override
  String get roundtrip => 'Roundtrip';

  @override
  String get noCodecStatus => 'Žádný stav kodeku';

  @override
  String get codecReady => 'Kodek připraven';

  @override
  String get codecReadOnly => 'Kodek jen pro čtení';

  @override
  String get codecUnavailable => 'Kodek není k dispozici';

  @override
  String get details => 'Podrobnosti';

  @override
  String codecStatusLine(String status) {
    return 'Stav: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Dekomprese: $decompress | Komprese: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'ano';

  @override
  String get no => 'ne';

  @override
  String aboutVersion(String version, String sha) {
    return 'Verze $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Licencováno pod MIT License.';

  @override
  String difficultyTitle(String profile) {
    return 'Obtížnost — $profile';
  }

  @override
  String get difficultyNoProfile => 'Žádný profil';

  @override
  String get difficultyNoDifficulty => 'Žádná obtížnost';

  @override
  String get difficultyLabel => 'Obtížnost';

  @override
  String get difficultyTooltipNoProfile => 'Není vybraný profil';

  @override
  String get difficultyTooltipEdit => 'Upravit obtížnost pro tento profil';

  @override
  String get difficultyTooltipNoEditable =>
      'Tento profil nemá upravitelnou obtížnost';

  @override
  String get preset => 'Předvolba';

  @override
  String get presetNovice => 'Nováček';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Těžká';

  @override
  String get presetCustom => 'Vlastní';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Uložená předvolba není rozpoznaná ($preset). Pořád můžeš uložit změny Flow Helper v boji zblízka / Permadeath, nebo výše zvolit předvolbu a přepsat ji.';
  }

  @override
  String get closeCombatFlowHelper => 'Pomocník plynulosti boje zblízka';

  @override
  String get permadeath => 'Trvalá smrt';

  @override
  String get notAvailableOnNovice => 'Na Nováčkovi není k dispozici';

  @override
  String get levelCombat => 'Boj';

  @override
  String get levelResources => 'Zdroje';

  @override
  String get levelProgression => 'Postup';

  @override
  String get difficultyAppliesToAllSaves =>
      'Obtížnost platí pro všechny save soubory v tomto profilu.';

  @override
  String get savingDifficultyFailed => 'Uložení obtížnosti selhalo.';

  @override
  String get addItemDialogTitle => 'Přidat předmět';

  @override
  String get searchItems => 'Hledat předměty';

  @override
  String failedToLoadCatalog(String error) {
    return 'Katalog se nepodařilo načíst: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Žádné předměty k přidání';

  @override
  String get noItemsMatch => 'Žádné shody';

  @override
  String get countMustBeAtLeast1 => 'Musí být ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Musí být ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Přidat NPC';

  @override
  String get noNpcsAvailableToAdd => 'Žádní NPC k přidání';

  @override
  String get noNpcsMatch => 'Žádný NPC neodpovídá filtru';

  @override
  String get categoryAll => 'Vše';

  @override
  String allWithCount(int count) {
    return 'Vše ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Přidat záznam znalostí';

  @override
  String get searchEntries => 'Hledat záznamy';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nejsou k dispozici žádné záznamy znalostí k přidání';

  @override
  String get noEntriesMatch => 'Žádný záznam neodpovídá';

  @override
  String get heroGroupMainStats => 'Hlavní statistiky';

  @override
  String get heroGroupCombatMovement => 'Boj / pohyb';

  @override
  String get heroGroupResistances => 'Odolnosti';

  @override
  String get heroGroupThieving => 'Zlodějina';

  @override
  String get heroGroupAdvanced => 'Pokročilé';

  @override
  String get heroGroupDiving => 'Potápění';

  @override
  String get heroDivingSkillNote =>
      'Jakmile se naučíš potápění, hra při každém načtení uložené hry obnoví dech a regeneraci na hodnoty dané touhle dovedností. Spotřeba vzduchu za sekundu zůstane tak, jak ji nastavíš.';

  @override
  String get heroGroupSleep => 'Spánek';

  @override
  String get heroGroupIntoxication => 'Opilost a omámení';

  @override
  String get heroEntryHeroTransform => 'Pozice';

  @override
  String attributeEmpty(String name) {
    return '$name je prázdné — zadej hodnotu nebo obnov původní před uložením.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Neplatné číslo pro $name: „$text\"';
  }

  @override
  String get loadingEditorData => 'Načítání dat editoru';

  @override
  String savingProgress(int done, int total) {
    return 'Ukládání… $done z $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extrahováno $idCount id v $languageCount jazycích';
  }

  @override
  String get skillSmithing1H => 'Kovářství – jednoruční';

  @override
  String get skillSmithing2H => 'Kovářství – obouruční';

  @override
  String get skillCircleNovice => 'Mág-nováček';

  @override
  String get skillCircle1 => 'První kruh magie';

  @override
  String get skillCircle2 => 'Druhý kruh magie';

  @override
  String get skillCircle3 => 'Třetí kruh magie';

  @override
  String get skillCircle4 => 'Čtvrtý kruh magie';

  @override
  String get skillCircle5 => 'Pátý kruh magie';

  @override
  String get skillCircle6 => 'Šestý kruh magie';

  @override
  String get sectionGlossary => 'Glosář';

  @override
  String get glossarySearch => 'Hledat v glosáři';

  @override
  String get glossaryOldCamp => 'Starý tábor';

  @override
  String get glossaryNewCamp => 'Nový tábor';

  @override
  String get glossarySwampCamp => 'Tábor v bažinách';

  @override
  String get glossaryOutsiders => 'Outsideri';

  @override
  String get glossaryCreatures => 'Stvoření';

  @override
  String get glossaryLocations => 'Místa';

  @override
  String get glossaryFilterLabel => 'Filtr';

  @override
  String get glossaryFilterTraders => 'Obchodníci';

  @override
  String get glossaryFilterTeachers => 'Učitelé';

  @override
  String get roleTrader => 'Obchodník';

  @override
  String get roleDead => 'Mrtvý';

  @override
  String get roleTeacher => 'Učitel';

  @override
  String get roleArmorer => 'Zbrojíř';

  @override
  String get glossaryFilterArmorers => 'Zbrojíři';

  @override
  String get glossaryFilterHostile => 'Nepřátelští';

  @override
  String get glossaryRelationshipFilterNote =>
      'Zobrazuje trvalá nastavení nepřátelství uložená v uložené hře. Dynamické vztahy cechů, příběhu, oblasti a zločinů se počítají jen ve hře.';

  @override
  String get glossaryFilterDead => 'Mrtví';

  @override
  String get glossaryAddEntry => 'Přidat položku glosáře';

  @override
  String get glossaryAddTitle => 'Přidat položku glosáře';

  @override
  String get glossaryResetChanges => 'Resetovat změny glosáře';

  @override
  String get glossaryNoVisibleEntries =>
      'V tomto pohledu nejsou žádné viditelné položky glosáře.';

  @override
  String get glossaryNoHiddenEntries =>
      'Všechny dostupné položky už jsou viditelné.';

  @override
  String get glossaryNoMatch => 'Žádné položky glosáře neodpovídají.';

  @override
  String get glossarySelectEntry =>
      'Vyber položku glosáře a uprav její záznamy.';

  @override
  String glossaryEntryCount(int count) {
    return '$count záznamů';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked z $total záznamů';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portrét odemčen';

  @override
  String get glossaryPortraitSilhouette => 'Silueta — portrét není odemčen';

  @override
  String get glossarySegments => 'Záznamy';

  @override
  String get glossaryPending => 'Neuložená změna';

  @override
  String get glossaryShowFullText => 'Zobrazit celý text záznamu';

  @override
  String get glossarySegmentIntroduction => 'Úvod / portrét';

  @override
  String get glossarySegmentUnlock => 'Objevení';

  @override
  String glossarySegmentEntry(int number) {
    return 'Záznam $number';
  }

  @override
  String get questJournalAll => 'Všechny questy';

  @override
  String get questJournalOldCamp => 'Starý tábor';

  @override
  String get questJournalNewCamp => 'Nový tábor';

  @override
  String get questJournalSwampCamp => 'Tábor v bažinách';

  @override
  String get questJournalColony => 'Kolonie';

  @override
  String get questJournalCompleted => 'Dokončené';

  @override
  String get questJournalHint =>
      'Pohled herního deníku. Vnitřní a ještě nezačaté stavy questů zůstávají v sekci Všechna data.';

  @override
  String get questJournalNoEntries =>
      'Žádné questy v deníku neodpovídají aktuálním filtrům.';

  @override
  String get glossaryTutorials => 'Návody';

  @override
  String get tutorialGateNote =>
      'Tyto řádky řídí uložené odemykání návodů. Jedna brána nemusí odpovídat jedné stránce návodu ve hře.';

  @override
  String get tutorialResetChanges => 'Resetovat změny návodů';

  @override
  String get tutorialNoGates =>
      'V tomto uložení nejsou k dispozici žádné brány návodů.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return 'Odemčeno $unlocked z $total bran návodů';
  }

  @override
  String get tutorialGateCombatBasics => 'Základy boje';

  @override
  String get tutorialGateCrafting => 'Výroba';

  @override
  String get tutorialGateCrime => 'Zločin a následky';

  @override
  String get tutorialGateDrugs => 'Spotřební věci a efekty';

  @override
  String get tutorialGateLockpicking => 'Zamykání';

  @override
  String get tutorialGateMagic => 'Magie';

  @override
  String get tutorialGateMap => 'Mapa';

  @override
  String get tutorialGateMeleeCombat => 'Boj na blízko';

  @override
  String get tutorialGateNavigation => 'Pohyb a orientace';

  @override
  String get tutorialGatePerception => 'Vnímání';

  @override
  String get tutorialGatePlayerProgression => 'Vývoj postavy';

  @override
  String get tutorialGateRanged => 'Střelba';

  @override
  String get tutorialGateRiding => 'Jízda';

  @override
  String get tutorialGateSleep => 'Spánek';

  @override
  String get tutorialGateTrading => 'Obchod';

  @override
  String get windowMinimizeTooltip => 'Minimalizovat';

  @override
  String get windowMaximizeTooltip => 'Maximalizovat';

  @override
  String get windowRestoreTooltip => 'Obnovit';

  @override
  String get fallbackDialogEntry => 'Položka dialogu';

  @override
  String get fallbackDialogChoice => 'Volba dialogu';

  @override
  String get fallbackDialogTopic => 'Téma dialogu';

  @override
  String get fallbackDialogInformation => 'Informace dialogu';

  @override
  String get fallbackQuest => 'Úkol';

  @override
  String get fallbackObjective => 'Cíl';

  @override
  String get fallbackItem => 'Předmět';

  @override
  String get attributeSkillPointsFallback => 'Body dovedností (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Rovnováha',
      'MaxSuperArmor': 'Max. rovnováha',
      'DamageMultiplier': 'Přijímané poškození',
      'SpeedModifier': 'Rychlost pohybu',
      'Oxygen': 'Dech',
      'MaxOxygen': 'Max. dech',
      'OxygenDepletionRate': 'Spotřeba dechu za sekundu',
      'OxygenRecoveryRate': 'Obnova dechu za sekundu',
      'CriticalLevelPercent': 'Varování před nedostatkem dechu',
      'SleepTime': 'Zbývající hodiny odpočinku',
      'MaxSleepTime': 'Max. hodiny odpočinku',
      'SleepTimeRecoveryAmount': 'Obnovené hodiny odpočinku',
      'SleepTimeRecoveryPeriod': 'Interval doplnění',
      'MaxRestTime': 'Max. čas v posteli',
      'Health_RecoveryRatePerHourOfSleep': 'Zdraví za hodinu spánku',
      'Mana_RecoveryRatePerHourOfSleep': 'Mana za hodinu spánku',
      'Alcohol': 'Úroveň alkoholu',
      'MaxAlcohol': 'Max. alkohol',
      'AlcoholDepletionRate': 'Rychlost střízlivění',
      'Swampweed': 'Úroveň bažinné trávy',
      'MaxSwampweed': 'Max. bažinná tráva',
      'SwampweedDepletionRate': 'Rychlost odeznění',
      'XPExecutedBounty': 'XP za dorážku',
      'XPKillOrDefeatBounty': 'XP za poražení',
      'Level': 'Úroveň',
      'LockpickDurability': 'Odolnost paklíče',
      'LockpickPrecision': 'Přesnost paklíče',
      'PickPocketing': 'Kapsářství',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Kolik ran tato postava vydrží, než ji úder srazí.',
      'MaxSuperArmor':
          'Celková zásoba rovnováhy; roste s úrovní postavy a nošenou zbrojí.',
      'DamageMultiplier':
          'Násobitel poškození, které tato postava dostává — 1 je normál, vyšší bolí víc.',
      'SpeedModifier': 'Násobitel rychlosti pohybu této postavy — 1 je normál.',
      'Oxygen':
          'Kolik sekund vzduchu zbývá pod vodou; při nule tato postava utopí.',
      'MaxOxygen':
          'Kolik sekund může tato postava vydržet pod vodou; zvyšuje to dovednost Potápění.',
      'OxygenDepletionRate': 'Kolik vzduchu ubývá každou sekundu pod hladinou.',
      'OxygenRecoveryRate':
          'Kolik vzduchu se vrátí každou sekundu po vynoření.',
      'CriticalLevelPercent':
          'Podíl zbývajícího vzduchu, při kterém hra varuje před utopením.',
      'SleepTime':
          'Hodiny spánku, které ještě něco obnoví; nad tím hra nedá bonus odpočinku.',
      'MaxSleepTime':
          'Největší zásoba odpočinkových hodin, kterou tato postava může mít.',
      'SleepTimeRecoveryAmount':
          'Odpočinkové hodiny, které se vrátí při každém doplnění zásoby.',
      'SleepTimeRecoveryPeriod':
          'Jak dlouho trvá, než se zásoba odpočinkových hodin znovu doplní.',
      'MaxRestTime': 'Nejdelší jednorázový pobyt v posteli, který hra dovolí.',
      'Health_RecoveryRatePerHourOfSleep':
          'Podíl maxima zdraví obnovený za každou prospatou hodinu.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Podíl maxima many obnovený za každou prospatou hodinu.',
      'Alcohol':
          'Jak opilá je tato postava; vyšší stupně mění obratnost a manu za sílu.',
      'MaxAlcohol':
          'Nejvyšší úroveň alkoholu, které tato postava může dosáhnout.',
      'AlcoholDepletionRate':
          'Jak rychle klesá hladina alkoholu zpět k střízlivosti.',
      'Swampweed':
          'Jak omámená je tato postava bažinnou trávou; vyšší stupně přehazují atributy.',
      'MaxSwampweed':
          'Nejvyšší úroveň bažinné trávy, které tato postava může dosáhnout.',
      'SwampweedDepletionRate': 'Jak rychle omámení bažinnou trávou mizí.',
      'XPExecutedBounty':
          'Zkušenost za zabití této postavy, když už leží poražená na zemi.',
      'XPKillOrDefeatBounty':
          'Zkušenost za sražení této postavy, ať už zemře, nebo jen upadne do bezvědomí.',
      'Level': 'Úroveň postavy. Roste se zkušeností a dává body učení.',
      'LockpickDurability':
          'Podle dovednosti Zamykání: 2 nevyškolený, 4 vyškolený, 6 mistr.',
      'LockpickPrecision':
          'Podle dovednosti Zamykání: 0 nevyškolený, 1 vyškolený, 2 mistr.',
      'PickPocketing':
          'Podle dovednosti Kapsářství: -30 nevyškolený, -10 vyškolený, +10 mistr.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Hlasová replika';

  @override
  String get knowledgeTypeOther => 'Jiné';

  @override
  String get armorUpgradeUpper => 'Horní';

  @override
  String get armorUpgradeMiddle => 'Střední';

  @override
  String get armorUpgradeLower => 'Dolní';

  @override
  String get knowledgeCategoryTopic => 'Téma';

  @override
  String get knowledgeCategoryChoice => 'Volba';

  @override
  String get knowledgeCategoryInfo => 'Informace';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Selhalo';

  @override
  String get missingSaveReference => 'Soubor chybí';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'Chybí $slot.sav. Možná byl smazán, přesunut nebo přejmenován; profil na něj stále odkazuje.';
  }

  @override
  String get removeFromProfile => 'Odebrat z profilu';

  @override
  String get deleteSavegame => 'Smazat uloženou hru';

  @override
  String get deleteSavegameTitle => 'Smazat uloženou hru?';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return 'Smazat $save ($fileName)? Odebere se z $profile a ze složky uložených her. GORE nejdřív vytvoří zálohu.';
  }

  @override
  String get removeSaveFromProfileTitle => 'Odebrat uloženou hru z profilu?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Odebrat $save z $profile? Samotný soubor uložené hry zůstane, pokud ještě existuje.';
  }

  @override
  String get unassignedSave => 'Nepřiřazeno k profilu';

  @override
  String get armorUpgradeLight => 'Lehká';

  @override
  String get armorUpgradeMedium => 'Střední';

  @override
  String get armorUpgradeHeavy => 'Těžká';

  @override
  String get knowledgeCaptionForcedConversation => 'Vynucený rozhovor';

  @override
  String get knowledgeCaptionFollowupTopic => 'Navazující téma';

  @override
  String get knowledgeCaptionFallbackTopic => 'Záložní téma';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours hod';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours hod $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Neplatná data profilu';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Chybí metadata vybrané uložené hry';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Neznámé';

  @override
  String editorUnexpectedError(String details) {
    return 'Neočekávaná chyba: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Probíhá jiná operace. Zkus to za chvíli znovu.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Máš neuložené úpravy uložené hry. Ulož je nebo resetuj, než změníš obtížnost profilu.';

  @override
  String get editorNoSaveFolderSelected => 'Není vybrána složka uložených her.';

  @override
  String get editorNoSaveSelected => 'Není vybrána uložená hra.';

  @override
  String get coreUnknownError => 'Neznámá chyba jádra';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Nejdřív ulož nebo resetuj neuložené změny — přepnutí profilu tě odvede od aktuální uložené hry.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Ulož nebo resetuj neuložené změny, než otevřeš jiný soubor.';

  @override
  String get editorSelectSavFile => 'Vyber soubor uložené hry .sav.';

  @override
  String get editorNotGothicGsav =>
      'Vybraný soubor není uložená hra Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Ulož nebo resetuj neuložené změny, než změníš profil uložené hry.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Ulož nebo resetuj neuložené změny, než odebereš uloženou hru z profilu.';

  @override
  String get editorUnsavedBeforeDeleteSave =>
      'Ulož nebo resetuj neuložené změny, než smažeš tuto uloženou hru.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Máš neuložené úpravy uložené hry. Ulož je nebo resetuj, než obnovíš zálohu profilu.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Neuložené úpravy ze dvou záložek míří na stejnou vlastnost ($path). Resetuj nebo vrať jednu z nich a pak znovu ulož.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Změna segmentu glosáře a jiná neuložená úprava v sekci Všechna data míří na pole Hero MemorizedEvents ($path). Změny glosáře do toho pole záznamy přidávají nebo odebírají, takže je nelze uložit najednou — resetuj nebo vrať jednu z nich a pak znovu ulož.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Změna segmentu glosáře a jiná neuložená úprava míří na stejnou vlastnost CurrentState questu ($path). Změna glosáře ten stav sama aktualizuje — resetuj nebo vrať jednu z nich a pak znovu ulož.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Přepsání vztahu a jiná neuložená úprava v sekci Všechna data míří na stejný záznam vztahu NPC ($path). Strukturovaná změna vztahu může v tom záznamu nahradit modifikátory, takže je nelze uložit najednou — resetuj nebo vrať jednu z nich a pak znovu ulož.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Víc než jedna neuložená strukturální úprava míří na stejné pole ($path). Ulož nebo resetuj první změnu, než zařadíš další.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Strukturální změna události a jiná neuložená úprava v sekci Všechna data míří na $path. Ulož nebo resetuj jednu z nich, než budeš pokračovat.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Změna dovedností a úprava v sekci Všechna data stejného efektu postavy (ActiveEffects › EffectSpec › Def) čekají na uložení. Nelze je uložit najednou — resetuj nebo vrať jednu z nich a pak znovu ulož.';

  @override
  String get editorInventoryResetConflict =>
      'Reset inventáře a jiná úprava stejného inventáře čekají na uložení. Reset nahradí celý inventář a zahodil by tu druhou úpravu — resetuj nebo vrať jednu z nich a pak znovu ulož.';

  @override
  String get editorUseFolder => 'Použít složku';

  @override
  String get editorGothicSavegameFileType => 'Uložená hra Gothic';

  @override
  String get editorNoDifficultyChanges => 'Žádné změny obtížnosti k zapsání';

  @override
  String get editorDifficultyWritten =>
      'Obtížnost zapsána do profilu (vytvořena záloha)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count změn uloženo se zálohou',
      many: '$count změn uloženo se zálohou',
      few: '$count změny uloženy se zálohou',
      one: '1 změna uložena se zálohou',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Přesun byl uložen, ale poznámku pro vrácení se nepodařilo zapsat: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profil $profileId nebyl nalezen.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Ve složce uložených her není volný slot (G1R-001 až G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Uložená hra importována a přiřazena k profilu $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Uložená hra přiřazena k profilu $profileId (vytvořeny párové zálohy)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Slot $slot není přiřazen k profilu $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Uložená hra odebrána z profilu';

  @override
  String get editorSaveDeleted => 'Uložená hra smazána; vytvořena záloha';

  @override
  String editorRestoredBackup(String path) {
    return 'Obnovena záloha: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Obnovena záloha: $path (PersistentDataList.sav beze změny — chybí odpovídající párová záloha; metadata slotu se mohou lišit)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Kodek roundtrip OK: chunk $chunkIndex znovu zkomprimován na $bytes bajtů';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Nepodařilo se zapsat obtížnost profilu: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Nepodařilo se přiřadit uloženou hru k profilu: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Nepodařilo se odebrat uloženou hru z profilu: $details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return 'Nepodařilo se smazat uloženou hru: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Nepodařilo se uložit změny: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Nepodařilo se prohledat uložené hry: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Nepodařilo se zkontrolovat uloženou hru: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Nepodařilo se načíst zálohy: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Nepodařilo se obnovit zálohu: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Obnovena záloha: $path, ale znovunačtení uložené hry selhalo: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Kontrola kodeku selhala: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Kodek roundtrip selhal: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Hledání vlastností selhalo: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Výběr uložené hry se změnil během načítání atributů hrdiny.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Načtení dovedností selhalo: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Dotaz na postup selhal: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Načtení seznamu NPC selhalo: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Načtení seznamu postav selhalo: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Načtení atributů NPC selhalo: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Načtení pozice NPC selhalo: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Načtení inventáře NPC selhalo: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Načtení seznamu frakcí selhalo: $details';
  }

  @override
  String get editorNoBackupPath => 'žádná';

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
    return '$prefix: $backupPath; záloha PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Stav lokalizace selhal: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Extrakce selhala: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Načtení glosáře selhalo: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Chyba zálohy: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Úkol',
      'document': 'Dokument',
      'story': 'Příběh',
      'exploration': 'Průzkum',
      'combat': 'Boj',
      'social': 'Společenské',
      'item': 'Předměty',
      'learning': 'Učení',
      'guild': 'Cech',
      'crime': 'Zločin',
      'rest': 'Odpočinek',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Úkol zahájen',
      'questSucceeded': 'Úkol splněn',
      'questFailed': 'Úkol neúspěšný',
      'documentRead': 'Dokument přečten',
      'documentSegmentUnlocked': 'Záznam objeven',
      'documentSegmentViewed': 'Záznam zobrazen',
      'chapterCompleted': 'Kapitola dokončena',
      'areaEntered': 'Vstup do oblasti',
      'areaLeft': 'Opuštění oblasti',
      'characterKilled': 'Postava zabita',
      'characterDefeated': 'Postava poražena',
      'combatDodge': 'Útoku uhnuto',
      'characterDebuffed': 'Debuff aplikován',
      'tradeAvailable': 'Obchod odemčen',
      'itemObtained': 'Předmět získán',
      'itemCrafted': 'Předmět vyroben',
      'skillStateRecorded': 'Stav dovednosti zaznamenán',
      'recipeLearned': 'Recept naučen',
      'guildJoined': 'Připojení k cechu',
      'crimeRecorded': 'Zločin zaznamenán',
      'slept': 'Spáno',
      'storyEvent': 'Příběhová událost',
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
      'gameTime': 'Herní čas',
      'duration': 'Trvání',
      'chapter': 'Kapitola',
      'instigator': 'Iniciátor',
      'affected': 'Postižený',
      'amount': 'Množství',
      'primaryObject': 'Objekt',
      'secondaryObject': 'Kontext',
      'segmentText': 'Text záznamu',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Den $day, $time';
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
  String get memoryEventHero => 'Hrdina';

  @override
  String get memoryEventDetails => 'Podrobnosti';

  @override
  String get memoryEventTags => 'Štítky';

  @override
  String get memoryEventTechnicalData => 'Technická data';

  @override
  String get memoryEventIndex => 'Index';

  @override
  String get memoryEventPosition => 'Pozice';

  @override
  String get memoryEventPayload => 'Datová část';

  @override
  String get memoryEventSubject => 'Subjekt';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Přístup',
      'AccessDenied': 'Přístup odepřen',
      'AccesToTemple': 'Přístup do chrámu',
      'Advice': 'Rada',
      'AfterFight': 'Po boji',
      'AfterFireMages': 'Po mágích ohně',
      'AfterNek': 'Po Nekovi',
      'AfterQuest': 'Po úkolu',
      'Alone': 'Sám',
      'Amulet': 'Amulet',
      'Annoying': 'Otravný',
      'Armor': 'Zbroj',
      'Avoid': 'Vyhnout se',
      'Backstory': 'Zákulisí',
      'BackStory': 'Zákulisí',
      'BasicMagic': 'Základy magie',
      'Beated': 'Poražený',
      'BecomeMercenary': 'Stát se žoldnéřem',
      'Beer': 'Pivo',
      'Bestiary': 'Bestiář',
      'Blessing': 'Požehnání',
      'Boss': 'Bos',
      'Bully': 'Šikana',
      'BullyAdvice': 'Rada ohledně šikany',
      'Camp': 'Tábor',
      'CampDivided': 'Rozdělený tábor',
      'CareOfMessengers': 'Péče o posly',
      'ChangeOpinion': 'Změna názoru',
      'ChargeUriziel': 'Nabití Uriziela',
      'Chosen': 'Vyvolený',
      'Contact': 'Kontakt',
      'Courier': 'Kurýr',
      'CraftBows': 'Výroba luků',
      'Crazy': 'Blázen',
      'DailyMeal': 'Denní jídlo',
      'DailyRation_Trader': 'Obchodník s denními dávkami',
      'DAM': 'DAM',
      'Dead': 'Mrtvý',
      'Deal': 'Dohoda',
      'Dealer': 'Dealer',
      'Deceived': 'Podvedený',
      'Dementia': 'Demence',
      'DenyAccess': 'Odepřít přístup',
      'DifferentOpinion': 'Jiný názor',
      'Discussion': 'Diskuse',
      'DontTalk': 'Nemluv',
      'Duel': 'Souboj',
      'Entrance': 'Vstup',
      'Escape': 'Útěk',
      'Extended': 'Rozšířené',
      'Extra': 'Extra',
      'ExtraInfo': 'Další info',
      'Fanatic': 'Fanatik',
      'Fight': 'Boj',
      'FindUlumulu': 'Najít Ulu-Mulu',
      'FireMages': 'Mágové ohně',
      'FireMagesEscape': 'Útěk mágů ohně',
      'FiskNewDealer': 'Nový pašerák pro Fiska',
      'FiskNewDealerCompleted': 'Nový pašerák pro Fiska — dokončeno',
      'FogTower': 'Mlžná věž',
      'Food': 'Jídlo',
      'Forgave': 'Odpustil',
      'Forgive': 'Odpustit',
      'Forgiven': 'Odpuštěno',
      'FourFriends': 'Čtyři přátelé',
      'FreeHut': 'Volná chýše',
      'FreeMine': 'Volný důl',
      'Fury': 'Zuřivost',
      'GoodTeacher': 'Dobrý učitel',
      'Gossip': 'Drby',
      'GotScavenger': 'Získán mrchožrout',
      'GrantedAccess': 'Přístup povolen',
      'GRDArmor': 'Strážní zbroj',
      'Guide': 'Průvodce',
      'HateMages': 'Nenávist k mágům',
      'HateMagesExplanation': 'Vysvětlení nenávisti k mágům',
      'HateRiceLord': 'Nenávist k rýžovému lordovi',
      'Heal': 'Léčit',
      'Healing': 'Léčení',
      'Help': 'Pomoc',
      'Helper': 'Pomocník',
      'HelpKagan': 'Pomoci Kaganovi',
      'HutStory': 'Příběh chýše',
      'Ignore': 'Ignorovat',
      'Impress': 'Udělat dojem',
      'ImpressAlchemy': 'Udělat dojem — alchymie',
      'ImpressInscription': 'Udělat dojem — nápisy',
      'Info': 'Info',
      'Interested': 'Zajímá se',
      'Introduction': 'Úvod',
      'Introduction_2': 'Úvod 2',
      'Introduction_Armor': 'Úvod — zbroj',
      'Introduction_Teacher': 'Úvod — učitel',
      'Introduction_Trader': 'Úvod — obchodník',
      'Invocation': 'Invokace',
      'JoinSC': 'Připojit se k táboru v bažinách',
      'Joint': 'Společný',
      'KalomCamp': 'Tábor Kaloma',
      'Leader': 'Vůdce',
      'Learning': 'Učení',
      'LearnOrcish': 'Naučit se orkštinu',
      'LeftParty': 'Opustil družinu',
      'Library': 'Knihovna',
      'Lie': 'Lež',
      'Lock': 'Zámek',
      'Lockpick': 'Paklíč',
      'Mad': 'Šílený',
      'Mandibles': 'Klepetá mrchožrouta',
      'MapMaker': 'Tvůrce map',
      'Monastery': 'Klášter',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Nový tábor',
      'NewCamper': 'Nováček v táboře',
      'NewLeader': 'Nový vůdce',
      'NightPatrol': 'Noční hlídka',
      'NotInterested': 'Nezajímá se',
      'OldCamp': 'Starý tábor',
      'OrcEnclaveEntrance': 'Vstup do orkské enklávy',
      'OrcGraveyard': 'Orkský hřbitov',
      'OreArmor': 'Rudná zbroj',
      'Party': 'Družina',
      'Pay': 'Zaplatit',
      'PayMoney': 'Zaplatit peníze',
      'Permission': 'Povolení',
      'Pet': 'Mazlíček',
      'PreparingInvocation': 'Příprava invokace',
      'Quest': 'Úkol',
      'RankUpFireMages': 'Povýšení mága ohně',
      'RankUpGuard': 'Povýšení stráže',
      'RanUpFireMagesCompleted': 'Povýšení mága ohně dokončeno',
      'Realocated': 'Přesunutý',
      'Reason': 'Důvod',
      'Respect': 'Respekt',
      'ReturnToSC': 'Návrat do táboru v bažinách',
      'RicelordForeman': 'Předák rýžového lorda',
      'RideScavenger': 'Jízda na mrchožroutovi',
      'Robe': 'Roucha',
      'Safe': 'Bezpečí',
      'Scraper': 'Škrabák',
      'SecondChance': 'Druhá šance',
      'SecretLocation': 'Tajné místo',
      'SecretPassage': 'Tajná chodba',
      'SecretPath': 'Tajná cesta',
      'SleeperFollower': 'Následovník Spáče',
      'SleeperTemple': 'Chrám Spáče',
      'SmallInfo': 'Malá info',
      'Stonehenge': 'Stonehenge',
      'StopFollowing': 'Přestat následovat',
      'SwampCamp': 'Tábor v bažinách',
      'Talkative': 'Upovídaný',
      'Teach': 'Učit',
      'TeachBow': 'Učit luk',
      'Teacher': 'Učitel',
      'Teacher2': 'Učitel 2',
      'TeacherInscription': 'Učitel nápisů',
      'TeacherMana': 'Učitel many',
      'TeachIchor': 'Učit extrakci ichoru mrchožroutů',
      'TeachMagic': 'Učit magii',
      'TeachOrcish': 'Učit orkštinu',
      'TeachStats': 'Učit statistiky',
      'TeachWeapon': 'Učit zbraně',
      'Teleport': 'Teleport',
      'TheMysteriousOrc': 'Tajemný ork',
      'ThroneRoom': 'Trůnní sál',
      'TradeBow': 'Obchod s luky',
      'Trader': 'Obchodník',
      'TradeSkins_Trader': 'Obchodník se kůžemi',
      'Traitor': 'Zrádce',
      'Trial': 'Zkouška',
      'TrollCanyon': 'Trollí kaňon',
      'Trust': 'Důvěra',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Nezkušený',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Runa Uriziela',
      'Useful': 'Užitečný',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrace',
      'WaitFreeMine': 'Čekat u volného dolu',
      'WaitInTrainingArea': 'Čekat na cvičišti',
      'Warning': 'Varování',
      'WarningTooLate': 'Varování přišlo pozdě',
      'WaterMessenger': 'Posel vodních mágů',
      'Weapon': 'Zbraň',
      'Who': 'Kdo',
      'Women': 'Ženy',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Poškozené sloty inventáře';

  @override
  String slotRepairBody(int count) {
    return 'Tato uložená hra obsahuje $count slotů inventáře, jejichž id už neodpovídá pozici — ve hře by shození takového předmětu odstranilo jiný. Oprava přepíše jen id: žádný předmět se nepřidá, neodebere ani nezmění. Při uložení se jako vždy vytvoří záloha.';
  }

  @override
  String get slotRepairQueued => 'Oprava ve frontě — ulož, aby se aplikovala.';

  @override
  String get slotRepairAction => 'Opravit';

  @override
  String get slotRepairDiscard => 'Zahodit';

  @override
  String get editorInventorySlotEditConflict =>
      'Ve frontě je přímá úprava slotu inventáře spolu se změnou, která zasahuje celé sloty (oprava, přidání nebo odebrání). Druhá by přepsala první — vrať jednu z nich a znovu ulož.';

  @override
  String get editorTraderArrayConflict =>
      'Změna obchodu je ve frontě spolu s přímou úpravou pole obchodníků. Ta úprava přečísluje řádky, na které míří změna obchodu, takže jedna by skončila u špatného obchodníka — vrať jednu z nich a znovu ulož.';

  @override
  String get backupFactFile => 'Soubor';

  @override
  String get renameBackupTooltip => 'Pojmenovat tuto zálohu';

  @override
  String get renameBackupTitle => 'Pojmenovat zálohu';

  @override
  String get renameBackupLabel => 'Název';

  @override
  String renameBackupHelp(String fileName) {
    return 'Zobrazí se místo názvu souboru $fileName. Nech prázdné pro odstranění názvu; soubor se nepřejmenuje.';
  }

  @override
  String get deleteBackupTooltip => 'Smazat tuto zálohu';

  @override
  String get deleteBackupTitle => 'Smazat zálohu?';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Smazat „$name\" ($fileName)? Soubor se odstraní z disku a nejde obnovit.';
  }

  @override
  String get deleteBackupConfirm => 'Smazat';

  @override
  String editorDeletedBackup(String path) {
    return 'Záloha smazána: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Nepodařilo se smazat zálohu: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Nepodařilo se pojmenovat zálohu: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Oprava teď není možná — tuto uloženou hru nelze zapsat.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Záloha smazána: $path — nepodařilo se odstranit její název: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'Oprava není pro tuto uloženou hru k dispozici.';

  @override
  String get statisticsTitle => 'Statistiky';

  @override
  String get statisticsSubtitle =>
      'Stručné shrnutí postavy, questů, světa a postupu uložené hry.';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': 'Čas',
      'character': 'Postava',
      'quests': 'Úkoly',
      'progress': 'Postup',
      'encounters': 'Boj a kontakty',
      'inventory': 'Dovednosti a inventář',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': 'Odehraný čas',
      'worldTime': 'Herní čas',
      'level': 'Úroveň',
      'experience': 'Zkušenost',
      'learningPoints': 'Body učení',
      'guild': 'Cech',
      'health': 'Zdraví',
      'mana': 'Mana',
      'chapter': 'Kapitola',
      'location': 'Místo',
      'kills': 'Zabití NPC',
      'knownCharacters': 'Známé postavy',
      'killedMonsters': 'Zabití monstra',
      'defeatedNpcs': 'Poražení NPC',
      'killedNpcs': 'Zabití NPC',
      'knownNpcs': 'Známí NPC',
      'knownTeachers': 'Známí učitelé',
      'learnedSkills': 'Naučené dovednosti',
      'knowledge': 'Záznamy znalostí',
      'deadCharacters': 'Mrtvé postavy',
      'traders': 'Známí obchodníci',
      'inventoryStacks': 'Stacky předmětů',
      'inventoryItems': 'Předměty',
      'ore': 'Ruda',
      'equipped': 'Vybaveno',
      'hostileFactions': 'Nepřátelské frakce',
      'openCrimes': 'Otevřené zločiny',
      'position': 'Pozice',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': 'Starý tábor · Stín',
      'oldCampGuard': 'Starý tábor · Stráž',
      'oldCampFireMage': 'Starý tábor · Mág ohně',
      'newCampRogue': 'Nový tábor · Bandita',
      'newCampMercenary': 'Nový tábor · Žoldnéř',
      'newCampWaterMage': 'Nový tábor · Mág vody',
      'swampCampNovice': 'Tábor v bažinách · Nováček',
      'swampCampTemplar': 'Tábor v bažinách · Templář',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => 'Není k dispozici';

  @override
  String get statisticsMore => 'Více statistik';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return 'Úroveň $level, $guild, kapitola $chapter. Dokončené questy: $completed, neúspěšné: $failed. Herní čas: $playTime.';
  }

  @override
  String get locksSidebar => 'Zámky';

  @override
  String editorLockListFailed(String details) {
    return 'Načtení seznamu zámků selhalo: $details';
  }

  @override
  String get locksSearchHint => 'Hledat zámek nebo klíč';

  @override
  String get locksAllRegions => 'Všechny regiony';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$shown z $total';
  }

  @override
  String get locksFilterChests => 'Truhly';

  @override
  String get locksFilterDoors => 'Dveře';

  @override
  String get locksFilterUnlocked => 'Odemčené';

  @override
  String get locksFilterLocked => 'Zamčené';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return 'Obtížnost $bars ze 4 (vnitřní stupeň $level ze 7)';
  }

  @override
  String get locksKeyOnly => 'Jen klíč';

  @override
  String locksKeyLabel(String keys) {
    return 'Klíč: $keys';
  }

  @override
  String get locksPermalocked => 'Trvale zapečetěno';

  @override
  String get locksReadOnly => 'Toto uložení nemá editovatelnou sadu zámků.';

  @override
  String get locksUnknownEntry => 'V této verzi hry není';

  @override
  String get locksDoorLeafHint =>
      'Znovuzamknutí dveří také zavře samotné dveře.';

  @override
  String get locksResetPending => 'Zahodit čekající změny zámků';
}
