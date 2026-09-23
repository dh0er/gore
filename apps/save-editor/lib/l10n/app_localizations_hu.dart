// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Hungarian (`hu`).
class AppLocalizationsHu extends AppLocalizations {
  AppLocalizationsHu([String locale = 'hu']) : super(locale);

  @override
  String get debugSectionTitle => 'Haladó (debug)';

  @override
  String get debugSectionSubtitle =>
      'Diagnosztika és nyers adatok hibajelentésekhez';

  @override
  String get showObjectIdsTitle => 'További technikai azonosítók megjelenítése';

  @override
  String get showObjectIdsSubtitle =>
      'Technikai tárgy-, dialógustudás-, küldetés- és árva aktorazonosítók megjelenítése a szerkesztőben. Az NPC-azonosítók mindig látszanak.';

  @override
  String get storyStateSidebar => 'Történetállapot';

  @override
  String get storyStateDescription =>
      'Itt követi a játék a küldetések, dialógusok és események előrehaladását. A „Tárolt” a mentésedben lévő értékeket mutatja; a „Nincs beállítva” a többi ismert bejegyzést. A számtól függően lehet igen/nem, számláló vagy előrehaladási szint. Az időjelölők a játékbeli napot és időt mutatják.';

  @override
  String get storyStateReadOnly =>
      'Csak olvasható, amíg az értékek szkriptjelentése és a biztonságos map-írás nincs tisztázva. A kapcsolódó glosszárszöveg kontextus, nem a technikai azonosító közvetlen fordítása.';

  @override
  String get storyStateStructureReadOnly =>
      'A StoryPropertyValues struktúra ebben a mentésben nem oldható fel egyértelműen és biztonságosan. A történetértékek ehhez a mentéshez csak olvashatók maradnak.';

  @override
  String get storyStateSearch => 'Történetállapot keresése';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown / $total történetérték';
  }

  @override
  String get storyStateInteger => 'Egész szám';

  @override
  String get storyStateTimeMarker => 'Időjelölő';

  @override
  String get storyStateChapter => 'Fejezet';

  @override
  String get storyStateUnknown => 'Ismeretlen forrástípus';

  @override
  String storyStateShowDormant(int count) {
    return 'Nem használtak megjelenítése ($count)';
  }

  @override
  String get storyStateUnknownDetail =>
      'Ez a tárolt azonosító hiányzik a jelenlegi szkriptkatalógusból (például modból vagy újabb játékverzióból). A mentésbeli értéke int32, de a jelentése nem következtethető.';

  @override
  String get storyStateStored => 'Tárolt';

  @override
  String get storyStateUnset => 'Nincs beállítva';

  @override
  String get storyStateUnsetDetail =>
      'Ez a katalógusmező nincs serializálva ebben a mentésben; a játék ezért a beállítatlan vagy alapértelmezett állapotát használja.';

  @override
  String get storyStateRawValue => 'Nyers érték';

  @override
  String storyStateElapsed(String duration) {
    return 'A mentés idején eltelt: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'A mentés idején még előrébb: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days nap',
      one: '1 nap',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Kapcsolódó glosszárbejegyzés';

  @override
  String get storyStateTechnicalPath => 'Technikai útvonal';

  @override
  String get storyStateEditingGuidance =>
      'Válassz egy bejegyzést az érték módosításához. A változások mentéskor lépnek életbe. Csak olyan értékeket változtass, amelyek hatását érted: különben a küldetések vagy dialógusok nem a várt módon működhetnek. Mentéskor automatikusan biztonsági mentés készül.';

  @override
  String get storyStatePending => 'Függőben';

  @override
  String storyStatePendingValue(String value) {
    return 'Így lesz tárolva: $value';
  }

  @override
  String get storyStatePendingRemoval => 'El lesz távolítva a mentésből';

  @override
  String get storyStateEditValue => 'Érték szerkesztése';

  @override
  String get storyStateSetValue => 'Érték beállítása';

  @override
  String get storyStateRemoveValue => 'Eltávolítás a mentésből';

  @override
  String get storyStateUndoChange => 'Történetváltozás visszavonása';

  @override
  String get storyStateResetChanges => 'Történetváltozások visszaállítása';

  @override
  String storyStateDialogTitle(String id) {
    return '$id szerkesztése';
  }

  @override
  String get storyStateRawInput => 'Előjeles int32 érték';

  @override
  String get storyStateInvalidInt32 =>
      'Adj meg egy egész számot −2147483648 és 2147483647 között.';

  @override
  String get storyStateQueueChange => 'Változás sorba állítása';

  @override
  String storyStateSuggestedValues(String values) {
    return 'A szállított szkriptekben előforduló értékek: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'A javaslatok nem érvényességi határok; natív kód, modok vagy későbbi játékverziók más értékeket is használhatnak.';

  @override
  String get storyStateUseCurrentTime => 'Jelenlegi mentésidő használata';

  @override
  String get storyStateStructuredTime => 'Nap / idő';

  @override
  String get storyStateRawMode => 'Nyers int32';

  @override
  String get storyStateChapterWarning =>
      'Önmagában a fejezet megváltoztatása nem szinkronizálja a küldetéseket, NPC-ket, leltárt vagy világállapotot.';

  @override
  String get storyStateDormantWarning =>
      'Ehhez a mezőhöz a szállított szkriptgyorsítótárban nem található élő olvasás vagy írás. Lehet örökölt, natívan vezérelt vagy foglalt.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'A szállított szkriptek olvassák ezt a mezőt, de nincs bennük szkriptírás. Natív kód továbbra is kezelheti.';

  @override
  String get storyStateUnknownEditWarning =>
      'Ehhez a modos vagy újabb verziós azonosítóhoz nincs csomagolt forrásjelentés. Csak a nyers int32 értékét szerkeszd.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Bináris jelző',
      'finiteState': 'Többállapotú érték',
      'counterOrScore': 'Számláló / pontszám',
      'calendarDay': 'Naptári nap',
      'derivedOrOpaqueInteger': 'Származtatott / átlátszatlan egész',
      'readOnlyInSourceInteger': 'A szállított szkriptekben csak olvasható',
      'dormantOrLegacyInteger': 'A szállított szkriptekben nem használt',
      'other': 'Egész szám',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'A tárolt 0 és a hiányzó map-bejegyzés különböző fájlállapotok. Az „Eltávolítás a mentésből” a konstruktor-/alapértelmezett állapotot állítja vissza.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor logó';

  @override
  String get zoomTooltip => 'Ctrl +/- a nagyításhoz/kicsinyítéshez';

  @override
  String get switchToLightMode => 'Váltás világos módra';

  @override
  String get switchToDarkMode => 'Váltás sötét módra';

  @override
  String get about => 'Névjegy';

  @override
  String get tabOverview => 'Áttekintés';

  @override
  String get tabPlayer => 'Játékos';

  @override
  String get tabAttribute => 'Tulajdonságok';

  @override
  String get heroGroupSkills => 'Képességek';

  @override
  String get skillsNoneBody => 'Ehhez a karakterhez nem találhatók képességek.';

  @override
  String get skillsUnavailableBody =>
      'A képességek ezen a mentésen nem szerkeszthetők — a hősnek nincs módosítható effektadata.';

  @override
  String get skillNotLearned => 'Nincs megtanulva';

  @override
  String get skillLearn => 'Megtanulás';

  @override
  String get skillActionLearn => 'megtanulás';

  @override
  String get skillActionUnlearn => 'elfelejtés';

  @override
  String get skillTierUntrained => 'Gyakorlatlan';

  @override
  String get skillTierBeginner => 'Kezdő';

  @override
  String get skillTierTrained => 'Kiképzett';

  @override
  String get skillTierMaster => 'Mester';

  @override
  String get skillTierNovice => 'Újonc';

  @override
  String get skillTierAmateur => 'Amatőr (0. kör)';

  @override
  String get skillTierLearned => 'Megtanult';

  @override
  String skillTierCircle(int n) {
    return '$n. kör';
  }

  @override
  String get skillHintBlacksmith1H => '1K fegyverek';

  @override
  String get skillHintBlacksmith2H => '2K fegyverek';

  @override
  String get skillScutesTrained => 'Kiképzett (csontpikkelyek)';

  @override
  String get skillScutesMaster => 'Mester (+ pengelemezek)';

  @override
  String get skillCategoryCombat => 'Harc';

  @override
  String get skillCategoryCrafting => 'Készítés';

  @override
  String get skillCategoryHunting => 'Vadászat';

  @override
  String get skillCategoryLanguage => 'Nyelv';

  @override
  String get skillCategoryMagic => 'Mágia';

  @override
  String get skillCategoryMovement => 'Mozgás';

  @override
  String get skillCategoryThievery => 'Tolvajlás';

  @override
  String get skillCategoryOther => 'Egyéb';

  @override
  String get skillNameOneHanded => 'Egykezes';

  @override
  String get skillNameTwoHanded => 'Kétkezes';

  @override
  String get skillNameFists => 'Ököl';

  @override
  String get skillNameBow => 'Íj';

  @override
  String get skillNameCrossbow => 'Számszeríj';

  @override
  String get skillNameLockpicking => 'Zárfeltörés';

  @override
  String get skillNamePickpocketing => 'Zsebtolvajlás';

  @override
  String get skillNameTakeOrgans => 'Szerv kivétele';

  @override
  String get skillNameBreakTeeth => 'Fogak kivétele';

  @override
  String get skillNameTakeClaws => 'Karom kivétele';

  @override
  String get skillNameSkinFur => 'Szőrme levétele';

  @override
  String get skillNameSkin => 'Bőr levétele';

  @override
  String get skillNameTakeFins => 'Uszonyok levétele';

  @override
  String get skillNameTakeStingers => 'Fullánkok kivétele';

  @override
  String get skillNameTakeSecretion => 'Váladék kivétele';

  @override
  String get skillNameTakeSkullPlates => 'Koponyapáncél levétele';

  @override
  String get skillNameSkinSwampshark => 'Cápa bőr levétele';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Lemezek levétele';

  @override
  String get skillNameTakeScutes => 'Pikkelyek levétele';

  @override
  String get skillNameTakeUluMulu => 'Ulu-Mulu levétele';

  @override
  String get skillNameOrcWeapons => 'Ork fegyverek';

  @override
  String get skillNameMining => 'Bányászat';

  @override
  String get skillNameDiving => 'Búvárkodás';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Állkapcsok kivétele';

  @override
  String get skillNameTakeShadowbeastHorn => 'Szarv levétele (árnyvad)';

  @override
  String get skillNameTakeSpines => 'Gerinc kivétele';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Cápfogak kivétele';

  @override
  String get skillNameTakeFireTongue => 'Tűznyelv levétele';

  @override
  String get skillNameTakeTrollHorn => 'Szarv levétele (troll)';

  @override
  String get skillNameAcrobatics => 'Akrobatika';

  @override
  String get skillNameWallClimbing => 'Mászás';

  @override
  String get skillNameRiding => 'Dögevő lovaglás';

  @override
  String get skillNameSneaking => 'Lopózás';

  @override
  String get skillNameAlchemy => 'Alkímia';

  @override
  String get skillNameRuneInscription => 'Rúnavésés';

  @override
  String get skillNameBlacksmithing => 'Kovácsolás';

  @override
  String get skillNameMagicCircle => 'Mágikus kör';

  @override
  String get skillNameOrcish => 'Ork nyelv';

  @override
  String get tabInventory => 'Leltár';

  @override
  String get tabTrade => 'Kereskedelem';

  @override
  String get traderNotAMerchant => 'Ez a karakter nem kereskedik.';

  @override
  String get traderRetry => 'Újra';

  @override
  String get traderAmbiguousName =>
      'Több kereskedőrekord is ezt a nevet viseli, ezért a szerkesztő nem tudja, melyik bolt tartozik ehhez a karakterhez. A szerkesztés inkább le van tiltva, mint hogy a rosszat változtatnánk.';

  @override
  String get traderOre => 'Érc (vásárlóerő)';

  @override
  String get traderNoOre => 'nincs érc';

  @override
  String get traderStockCurrent => 'Készlet';

  @override
  String get traderStockCurrentTooltip =>
      'Amit ez a kereskedő jelenleg eladásra kínál. A hozzáadott tárgyak eltűnhetnek, amikor a játék frissíti a kereskedőt.';

  @override
  String get traderStockBase => 'Újratöltési alap';

  @override
  String get traderStockBaseTooltip =>
      'A mentés ezt a listát tartalmazza, hogy a játék újratölthesse a kereskedőt. A játék a kereskedői szabályokból újraszámolhatja, ezért az itteni változások nem tartósak.';

  @override
  String get traderStockBaseHint =>
      'Csak olvasható: a játék újratöltéskor használja ezt a listát, de újraszámolhatja. Az ide felvett tárgyak nem maradnának meg tartósan.';

  @override
  String get traderCurrentStockWarning =>
      'A kereskedő leltárának változásai csak a következő újratöltésig maradnak.';

  @override
  String get traderRestockTitle => 'Újratöltési időzítő';

  @override
  String get traderRestockTitleTooltip =>
      'Becslés a kereskedő utolsó aktivitása, a jelenlegi játékidő és az Erőforrások nehézség alapján.';

  @override
  String get traderRestockPending => 'függőben';

  @override
  String get traderRestockRevertTooltip => 'Függő időváltozás visszavonása';

  @override
  String get traderRestockNever => 'Soha';

  @override
  String get traderRestockUnavailable => 'Nem elérhető';

  @override
  String get traderRestockIntervalUnknown => 'Újratöltési várakozás ismeretlen';

  @override
  String get traderRestockNeverStatus =>
      'Még nincs rögzített kereskedői aktivitás.';

  @override
  String get traderRestockClockAhead =>
      'A kereskedő mentett ideje a jelenlegi játékidő előtt jár.';

  @override
  String traderRestockNotDueYet(String time) {
    return 'Nem várható $time előtt.';
  }

  @override
  String get traderRestockPossiblyDue =>
      'A kereskedő már készen állhat az újratöltésre.';

  @override
  String get traderRestockEligible =>
      'A kereskedőnek most készen kell állnia az újratöltésre.';

  @override
  String get traderRestockNoWorldTime =>
      'A jelenlegi játékidő nem elérhető, ezért a szerkesztő nem tudja megmondani, esedékes-e az újratöltés.';

  @override
  String get traderRestockLastActivity => 'Utolsó kereskedői aktivitás';

  @override
  String get traderRestockLastActivityTooltip =>
      'Ehhez a kereskedőhöz mentett utolsó időpont. Kereskedésből vagy más kereskedői frissítésből is jöhet, tehát nem feltétlenül az utolsó újratöltés.';

  @override
  String get traderRestockForecastWindow => 'Várható újratöltés';

  @override
  String get traderRestockForecastWindowTooltip =>
      'A pontos idő nincs a mentésben. A szerkesztő ezért a legkorábbi és a legkésőbbi várható idő közötti tartományt mutatja.';

  @override
  String get traderRestockIntervalLabel => 'Újratöltési várakozás';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days nap · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      'Az Erőforrások nehézség által meghatározott várakozás: Újonc 2, Gothic 3, Nehéz 5 játékbeli nap.';

  @override
  String get traderRestockAutomationLabel => 'Automatikus újratöltés';

  @override
  String get traderRestockAutomationValue => 'A mentésben nem kapcsolható ki';

  @override
  String get traderRestockAutomationTooltip =>
      'A mentésszerkesztő nem tudja megbízhatóan leállítani az automatikus újratöltést. Ehhez játékmod kell.';

  @override
  String get traderRestockSetNow => 'Világidőre állítás';

  @override
  String get traderRestockSetNowTooltip =>
      'A jelenlegi játékidőt használja a kereskedő utolsó aktivitásaként. Ezzel eltolódik a következő várható újratöltés.';

  @override
  String get traderRestockMakeDue => 'Most esedékessé tétel';

  @override
  String get traderRestockMakeDueTooltip =>
      'A kereskedő utolsó aktivitását eléggé visszaviszi, hogy az újratöltés most esedékes legyen.';

  @override
  String get traderRestockCustom => 'Egyéni idő…';

  @override
  String get traderRestockCustomTooltip =>
      'Válaszd ki a kereskedő utolsó aktivitásának játékbeli napját és idejét.';

  @override
  String get traderRestockEditTitle => 'Utolsó kereskedői aktivitás módosítása';

  @override
  String get traderOreHint =>
      'A játékbeli összeg eltér: betöltéskor a játék hozzáadja, ami az utolsó kereskedés óta felgyűlt — a fölös árut eladja, és abból tölt újra. Ez a szám a kiindulópont, nem ami a kereskedelmi képernyőn látszik.';

  @override
  String get traderOreHintShort =>
      'Kiindulási érték — a kereskedelmi képernyőn lévő összeg eltérhet.';

  @override
  String get traderRestockStatusLabel => 'Állapot';

  @override
  String get traderRestockStatusNever => 'Nincs aktivitás';

  @override
  String get traderRestockStatusWaiting => 'Újratöltésre vár';

  @override
  String get traderRestockStatusReady => 'Kész az újratöltésre';

  @override
  String get traderRestockStatusPossiblyReady => 'Lehet, hogy kész';

  @override
  String get traderRestockStatusCheckTime => 'Mentett idő ellenőrzése';

  @override
  String get traderRestockStatusUnknown => 'Ismeretlen';

  @override
  String get traderPriceWarning =>
      'Az árak reagálnak arra, mennyit tart a kereskedő és mennyi ércet birtokol, ezért ezeknek a számoknak a változtatása a felszámított árakat is elmozdíthatja.';

  @override
  String get traderAddItem => 'Tárgy hozzáadása';

  @override
  String get traderRemoveItem => 'Sor eltávolítása';

  @override
  String get traderReadOnlyCore =>
      'Ez a magbuild csak olvasni tudja a kereskedőadatokat.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Ez a kereskedő nehézség szerinti készletet hordoz, amit a szerkesztő nem modellez. A szerkesztés itt le van tiltva, mert a változás sikeresnek tűnne, miközben az extra készlet érintetlen maradna.';

  @override
  String get traderRecordIncomplete =>
      'Ennek a kereskedőnek hiányoznak a készletlistái, vagy olyan formában vannak, amit a szerkesztő nem támogat és nem tud írni. A szerkesztés itt le van tiltva, hogy a változás mentéskor ne bukjon el.';

  @override
  String get traderEmptyStock => 'Nincs készleten semmi.';

  @override
  String get traderUnknownItem => 'nincs a tárgykatalógusban';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Kereskedők betöltése sikertelen: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count sor';
  }

  @override
  String get tabWorld => 'Világ';

  @override
  String get tabCharacters => 'Karakterek';

  @override
  String get characterNoActorBody =>
      'Ennek a karakternek nincs világbeli aktora, ezért nincsenek tulajdonságai, leltára vagy eseményei.';

  @override
  String get characterNoEventsBody =>
      'Nincsenek események ehhez a karakterhez.';

  @override
  String get characterOrphanGroup => 'Egyéb';

  @override
  String get tabAllData => 'Összes adat';

  @override
  String get tabBackups => 'Biztonsági mentések';

  @override
  String get tabSettings => 'Beállítások';

  @override
  String get reset => 'Visszaállítás';

  @override
  String get save => 'Mentés';

  @override
  String saveWithCount(int count) {
    return 'Mentés ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Mégsem';

  @override
  String get confirm => 'Megerősítés';

  @override
  String get close => 'Bezárás';

  @override
  String get add => 'Hozzáadás';

  @override
  String get equippedBadge => 'Felszerelve';

  @override
  String get armorUpgradesLabel => 'Fejlesztések';

  @override
  String get browse => 'Tallózás';

  @override
  String get noSavFilesFound => 'Nem találhatók .sav fájlok';

  @override
  String get profile => 'Profil';

  @override
  String get otherSaves => 'Egyéb mentések';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count mentés)';
  }

  @override
  String get switchProfile => 'Profilváltás';

  @override
  String get openSaveFile => 'Fájl megnyitása';

  @override
  String get externalSave => 'Külsőleg megnyitott mentés';

  @override
  String get saveProfileTitle => 'Mentés profilja';

  @override
  String get saveProfileDescription =>
      'Rendeld ezt a mentést egy másik játékprofilhoz. A mentés és a profilindex együtt kerül biztonsági mentésre.';

  @override
  String get saveProfileExternalHint =>
      'Válassz egy profilt, hogy a fájlt importáld a játék mentésmappájába és ott regisztráld. Az eredeti fájl változatlan marad.';

  @override
  String get saveProfileNoProfiles =>
      'A PersistentDataList.sav-ban nem találhatók szerkeszthető játékprofilok.';

  @override
  String get saveProfileSelect => 'Profil kiválasztása';

  @override
  String get rescanSaveFolder => 'Mentésmappa újraszkennelése';

  @override
  String get discardUnsavedChangesTitle =>
      'Elveted a nem mentett változásokat?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'változást',
      one: 'változást',
    );
    return 'Az újraszkennelés újratölt minden mentést, és elveti a $count nem mentett $_temp0.';
  }

  @override
  String get discardAndRescan => 'Elvetés és újraszkennelés';

  @override
  String chapterLabel(Object id) {
    return '$id. fejezet';
  }

  @override
  String get quickSave => 'Gyorsmentés';

  @override
  String get autoSave => 'Automatikus mentés';

  @override
  String get manualSave => 'Kézi mentés';

  @override
  String get errorTitle => 'Hiba';

  @override
  String get selectASaveTitle => 'Válassz mentést';

  @override
  String get selectASaveBody => 'A mentés részletei itt jelennek meg.';

  @override
  String bytesValue(String count) {
    return '$count bájt';
  }

  @override
  String get inspectionJsonTitle => 'Ellenőrző JSON';

  @override
  String get copy => 'Másolás';

  @override
  String get savegameFallbackTitle => 'Mentés';

  @override
  String screenshotForSlot(String slot) {
    return 'Képernyőkép: $slot';
  }

  @override
  String get publicSaveName => 'Név';

  @override
  String get gameTimeTitle => 'Játékidő';

  @override
  String get gameTimeDay => 'Nap';

  @override
  String get gameTimeHours => 'Óra';

  @override
  String get gameTimeMinutes => 'Perc';

  @override
  String get gameTimeSeconds => 'Másodperc';

  @override
  String gameTimeTotal(int seconds) {
    return '= összesen $seconds s';
  }

  @override
  String get gameTimeInvalid =>
      'Egész számokat adj meg — nap ≥ 0, óra 0–23, perc és másodperc 0–59.';

  @override
  String get required => 'Kötelező';

  @override
  String get playerLockedBody =>
      'A privát játékosszerkesztéshez tömörítésre kész kodek kell.';

  @override
  String get heroTransform => 'Pozíció';

  @override
  String get locationX => 'X hely';

  @override
  String get locationY => 'Y hely';

  @override
  String get locationZ => 'Z hely';

  @override
  String get rotationPitch => 'Dőlés (pitch)';

  @override
  String get rotationYaw => 'Forgás (yaw)';

  @override
  String get rotationRoll => 'Billenés (roll)';

  @override
  String get spawnPositionSection => 'Spawn pozíció (referencia)';

  @override
  String get resetToSpawnPosition => 'Visszaállítás spawn pozícióra';

  @override
  String get positionOutOfRange =>
      'Az értéknek −10 000 000 és 10 000 000 között kell lennie';

  @override
  String get positionNotEditable =>
      'A tárolt pozíció ehhez a karakterhez nem olvasható, ezért nem szerkeszthető.';

  @override
  String get positionNeverPlaced =>
      'Ez a karakter soha nem került a világba (pozíció 0, 0, 0) — a játék figyelmen kívül hagyhatja a tárolt pozíciót.';

  @override
  String get npcStayInPlace => 'Napi rutinjának kikapcsolása';

  @override
  String get npcStayInPlaceHint => 'Így ott marad, ahol van.';

  @override
  String get npcStayInPlaceLocked =>
      'Az eredeti napi rutinját nem rögzítették, ezért ez már nem vonható vissza.';

  @override
  String get npcUndoPlacement => 'Mozgatás visszavonása';

  @override
  String get npcUndoPlacementStale =>
      'A mentés már nem tartalmazza, amit az a mozgatás írt, ezért a visszaállítás elvetné az azóta történteket.';

  @override
  String get positionNotReadable =>
      'A tárolt pozíció ehhez a karakterhez nem olvasható.';

  @override
  String get npcPositionReadOnly =>
      'A játék az NPC pozícióját a pályáról állítja vissza, nem a mentésből, ezért ezek az értékek olvashatók, de nem változtathatók.';

  @override
  String get pickLocation => 'Helyszín választása…';

  @override
  String get pickLocationDialogTitle => 'Helyszín választása';

  @override
  String get applySpotRotation => 'A pont tájolásának alkalmazása is';

  @override
  String get locationAreaOther => 'Egyéb';

  @override
  String get locationAreaCavalornValley => 'Cavalorn völgye';

  @override
  String get locationAreaEastForest => 'Keleti erdő';

  @override
  String get locationAreaFogTower => 'Ködtorony';

  @override
  String get locationAreaIllegalWeedMixers => 'Illegális fűkeverők';

  @override
  String get locationAreaOrcArena => 'Ork aréna';

  @override
  String get locationAreaOrcGraveyard => 'Ork temető';

  @override
  String get locationAreaShipwreck => 'Hajóroncs';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'A helyszínkatalógus nem tölthető be.';

  @override
  String get invalid => 'Érvénytelen';

  @override
  String get heroAttributes => 'Hős tulajdonságai';

  @override
  String attributeBase(String name) {
    return '$name alap';
  }

  @override
  String attributeCurrent(String name) {
    return '$name aktuális';
  }

  @override
  String get attributeBaseValue => 'Alapérték';

  @override
  String get attributeCurrentValue => 'Aktuális érték';

  @override
  String get inventoryTitle => 'Leltár';

  @override
  String get inventoryEmpty => 'Ez a leltár üres.';

  @override
  String get inventoryNeedsDecoded =>
      'A leltárszerkesztéshez dekódolt privát hasznos teher kell a kodekből.';

  @override
  String get inventoryNoStacks =>
      'Nem találhatók tárgycsomagok a dekódolt privát hasznos teherben.';

  @override
  String get resetInventoryChanges => 'Leltárváltozások visszaállítása';

  @override
  String get addItemTooltipPendingAdd =>
      'Előbb mentsd a függő változásokat — mentésenként egy új tárgy';

  @override
  String get addItemTooltipPendingRemove =>
      'Előbb mentsd a függő eltávolítást — mentésenként egy strukturális változás';

  @override
  String get addItemTooltipPendingCount =>
      'Előbb mentsd vagy állítsd vissza a függő darabszám-változásokat — a strukturális szerkesztést önmagában kell menteni';

  @override
  String get addItemTooltipDefault => 'Tárgy hozzáadása a leltárhoz';

  @override
  String get addItemButton => 'Tárgy hozzáadása';

  @override
  String get resetInventoryButton => 'Leltár visszaállítása';

  @override
  String get resetInventoryTooltipDefault =>
      'A leltár cseréje a játékkezdő mentés leltárára';

  @override
  String get resetInventoryTooltipBlocked =>
      'Előbb mentsd vagy töröld a függő leltárváltozásokat';

  @override
  String get pendingResetTitle => 'Visszaállítás a játékkezdő leltárra';

  @override
  String pendingResetSubtitle(String level) {
    return 'Erőforrások szintje: $level';
  }

  @override
  String get cancelPendingReset => 'Visszaállítás megszakítása';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — függő hozzáadás (még nincs mentve)';
  }

  @override
  String get cancelPendingAdd => 'Függő hozzáadás megszakítása';

  @override
  String get pendingRemovalSubtitle => 'függő eltávolítás (még nincs mentve)';

  @override
  String get cancelPendingRemoval => 'Függő eltávolítás megszakítása';

  @override
  String get filterItems => 'Tárgyak szűrése';

  @override
  String noItemsMatchQuery(String query) {
    return 'Nincs a(z) „$query” lekérdezésre illő tárgy.';
  }

  @override
  String get pendingRemovalHidesAll =>
      'A függő eltávolítás minden tárgyat elrejt — mentsd a végrehajtáshoz.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemTooltipIngredientFor => 'Hozzávaló ehhez';

  @override
  String itemTooltipTeaches(String item) {
    return 'Megtanítja: $item';
  }

  @override
  String get itemTooltipValue => 'Érték';

  @override
  String get itemTooltipProtection => 'Védelem';

  @override
  String get itemTooltipRequirements => 'Követelmények:';

  @override
  String get itemTooltipManaCost => 'Mana költség';

  @override
  String get itemTooltipManaUpkeep => 'Töltés mana költsége';

  @override
  String get itemCategoryAll => 'Összes';

  @override
  String get itemCategoryMeleeWeapon => 'Közelharci fegyverek';

  @override
  String get itemCategoryRangedWeapon => 'Távfegyverek';

  @override
  String get itemCategoryMagic => 'Mágia';

  @override
  String get itemCategoryWearable => 'Viselhető';

  @override
  String get itemCategoryFood => 'Étel';

  @override
  String get itemCategoryPotion => 'Bájitalok';

  @override
  String get itemCategoryMaterial => 'Anyagok';

  @override
  String get itemCategoryDocument => 'Dokumentumok';

  @override
  String get itemCategoryMisc => 'Egyéb';

  @override
  String get itemCategoryArtefact => 'Artefaktumok';

  @override
  String get itemCategoryOther => 'Más';

  @override
  String get count => 'Darabszám';

  @override
  String get min1 => 'Min. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Nem törölhető: ez a tárgy valószínűleg fel van szerelve vagy gyorsbillentyű-helyhez van rendelve';

  @override
  String get removeBlockedTooltip =>
      'Előbb mentsd vagy állítsd vissza a függő leltárváltozásokat — a hozzáadást vagy eltávolítást önmagában kell menteni';

  @override
  String get removeItemFromInventory => 'Tárgy eltávolítása a leltárból';

  @override
  String get progressionLockedBody =>
      'A fejlődési adatokhoz dekódolt privát hasznos teher kell a kodekből.';

  @override
  String get progressionNeedsTyped =>
      'A strukturált fejlődési adatokhoz teljesen dekódolt mentés kell, ellenőrzött típusos elemzéssel.';

  @override
  String get sectionQuests => 'Küldetések';

  @override
  String get sectionKnowledge => 'Tudás';

  @override
  String get sectionEvents => 'Események';

  @override
  String get firstPage => 'Első oldal';

  @override
  String get previousPage => 'Előző oldal';

  @override
  String get nextPage => 'Következő oldal';

  @override
  String get lastPage => 'Utolsó oldal';

  @override
  String pageOfPages(int page, int total) {
    return '$page. / $total. oldal';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last / $total';
  }

  @override
  String get perPage => 'Oldalanként:';

  @override
  String get resetQuestChanges => 'Küldetésváltozások visszaállítása';

  @override
  String get searchQuests => 'Küldetések keresése';

  @override
  String get allGroups => 'Összes csoport';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Nincs';

  @override
  String get questStateAvailable => 'Elérhető';

  @override
  String get questStateRunning => 'Folyamatban';

  @override
  String get questStateSucceeded => 'Sikeres';

  @override
  String get questStateFailed => 'Sikertelen';

  @override
  String get questStateUnknown => 'ismeretlen';

  @override
  String get dialogKnowledge => 'Dialógustudás';

  @override
  String get resetKnowledgeChanges => 'Tudásváltozások visszaállítása';

  @override
  String get addNpc => 'NPC hozzáadása';

  @override
  String get searchNpcs => 'NPC-k keresése';

  @override
  String get npcStatusRowLabel => 'Állapot';

  @override
  String get npcStatusAlive => 'él';

  @override
  String get npcStatusDead => 'halott';

  @override
  String get npcRelationshipRowLabel => 'Kapcsolat';

  @override
  String get npcRelationshipUnavailable => 'Kapcsolati állapot nem elérhető';

  @override
  String get npcRelationshipAutomatic => 'A játék számítja';

  @override
  String get npcRelationshipAutomaticHint =>
      'Nincs tartós felülírás tárolva. A céh-, történet-, terület- és bűnszabályokat a játék értékeli.';

  @override
  String get npcRelationshipStoredHint =>
      'Tartós NPC–játékos felülírásként tárolva. A céh-, történet-, terület- és bűnszabályok a játékban továbbra is módosíthatják a tényleges állapotot.';

  @override
  String get npcRelationshipFriend => 'Barát';

  @override
  String get npcRelationshipNeutral => 'Semleges';

  @override
  String get npcRelationshipEnemy => 'Ellenség';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Mentéskor $relationship lesz';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'ÉP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Felélesztés';

  @override
  String get npcReviveQueued => 'Mentéskor fel lesz élesztve';

  @override
  String entriesForCharacter(String name) {
    return 'Bejegyzések — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Válassz NPC-t a bejegyzésekhez';

  @override
  String get addKnowledgeEntry => 'Tudásbejegyzés hozzáadása';

  @override
  String get browseCatalog => 'Katalógus tallózása';

  @override
  String get alreadyExistsForCharacter => 'Ehhez a karakterhez már létezik.';

  @override
  String get alreadyInPendingChanges => 'Már a függő változások között van.';

  @override
  String duplicateCheckFailed(String error) {
    return 'A duplikátumellenőrzés sikertelen — próbáld újra: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Függő hozzáadások ($count)';
  }

  @override
  String get undoAdd => 'Hozzáadás visszavonása';

  @override
  String get undoRemove => 'Eltávolítás visszavonása';

  @override
  String get removeEntry => 'Bejegyzés eltávolítása';

  @override
  String get selectNpcFromList => 'Válassz NPC-t a listából';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Emlékezetesemények';

  @override
  String get searchCharacters => 'Karakterek keresése';

  @override
  String eventsForCharacter(String name) {
    return 'Események — $name';
  }

  @override
  String get selectCharacterToSeeEvents => 'Válassz karaktert az eseményekhez';

  @override
  String get noTags => '(nincs címke)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Esemény eltávolítása';

  @override
  String get removeMemoryEventTitle => 'Emlékezetesemény eltávolítása?';

  @override
  String get removeMemoryEventBody =>
      'Sorba állítod ezt az emlékezeteseményt eltávolításra? A mentésfájl csak Mentéskor változik.';

  @override
  String get memoryEventRemovalQueued =>
      'Eseményeltávolítás sorba állítva — nyomj Mentést a végrehajtáshoz.';

  @override
  String get duplicateEvent => 'Esemény másolása';

  @override
  String get duplicateMemoryEventTitle => 'Emlékezetesemény másolása?';

  @override
  String get duplicateMemoryEventBody =>
      'Sorba állítod ennek az emlékezeteseménynek a másolatát? A mentésfájl csak Mentéskor változik.';

  @override
  String get memoryEventDuplicationQueued =>
      'Eseménymásolás sorba állítva — nyomj Mentést a végrehajtáshoz.';

  @override
  String get selectCharacterFromList => 'Válassz karaktert a listából';

  @override
  String get factionsSidebar => 'Frakciók';

  @override
  String get factionsForgiveButton => 'Megbocsátás';

  @override
  String get factionHostile => 'Ellenséges';

  @override
  String get factionFriendly => 'Barátságos';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count gyilkosság',
      one: '$count gyilkosság',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count támadás',
      one: '$count támadás',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count lopás',
      one: '$count lopás',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count behatolás',
      one: '$count behatolás',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count fenyegetés',
      one: '$count fenyegetés',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count egyéb bűncselekmény',
      one: '$count egyéb bűncselekmény',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'megbocsátás folyamatban…';

  @override
  String get factionsEmpty =>
      'Nincsenek nyitott bűncselekmények a frakciók ellen.';

  @override
  String get factionGuildOldCamp => 'Régi Tábor';

  @override
  String get factionGuildNewCamp => 'Új Tábor';

  @override
  String get factionGuildSwampCamp => 'Mocsári Tábor';

  @override
  String get factionGuildOther => 'Egyebek / egyének';

  @override
  String get allDataLockedBody =>
      'A teljes forrásböngésző jelenleg GSAV mentésfájlokhoz érhető el.';

  @override
  String get allDataDescription =>
      'Böngészd a GSAV metaadatokat és minden típusos PUBLIC/PRIVATE csomópontot. A biztonságos skalár- és natív struktúraértékek szerkeszthetők; a tárolók és az átlátszatlan bájtok továbbra is láthatók.';

  @override
  String get allDataEditable => 'Szerkeszthető';

  @override
  String get allDataReadOnly => 'Csak olvasható';

  @override
  String get allDataType => 'Típus';

  @override
  String get allDataScalars => 'Skalárok';

  @override
  String get allDataStructs => 'Struktúrák';

  @override
  String get allDataContainers => 'Tárolók';

  @override
  String get allDataOpaque => 'Átlátszatlan';

  @override
  String get allDataNodes => 'Csomópontok';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count gyermek',
      one: '1 gyermek',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Függőben';

  @override
  String get allDataTagInputHint =>
      'Vesszővel vagy soronként elválasztott címkék';

  @override
  String allDataTypedSource(String source) {
    return '$source típusos';
  }

  @override
  String get searchPropertiesLabel =>
      'Tulajdonságok keresése (üres = mindent listáz) — pl. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Mentés dekódolása…';

  @override
  String get decodingSaveBody =>
      'A teljes privát hasznos teher dekódolása az első kereséshez. Mentésenként egyszer fut, utána a keresések azonnaliak.';

  @override
  String get searchTheSaveTitle => 'Keresés a mentésben';

  @override
  String get searchTheSaveBody =>
      'Írj be egy tulajdonságnevet, és nyomj Entert. Hagyd üresen az összes listázásához.';

  @override
  String get searchFailedTitle => 'A keresés sikertelen';

  @override
  String get noMatchesTitle => 'Nincs találat';

  @override
  String get noMatchesBody =>
      'Egyetlen tulajdonságútvonal sem tartalmazta az összes kifejezést.';

  @override
  String get value => 'Érték';

  @override
  String get backupsTitle => 'Biztonsági mentések';

  @override
  String get refreshBackups => 'Biztonsági mentések frissítése';

  @override
  String get noBackupsTitle => 'Nincsenek biztonsági mentések';

  @override
  String get noBackupsBody =>
      'A szerkesztett mentések a kiválasztott slot mellé készítenek biztonsági mentésfájlokat.';

  @override
  String get slotBackups => 'Slot biztonsági mentések';

  @override
  String get profileBackups => 'Profil biztonsági mentések';

  @override
  String get backupFactName => 'Név';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Létrehozva';

  @override
  String get backupFactSize => 'Méret';

  @override
  String get backupFactStatus => 'Állapot';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '$fileName visszaállítása';
  }

  @override
  String get appearanceTitle => 'Megjelenés';

  @override
  String get uiFont => 'Betűtípus';

  @override
  String get theme => 'Téma';

  @override
  String get themeLight => 'Világos';

  @override
  String get themeDark => 'Sötét';

  @override
  String get themeSystem => 'Rendszer';

  @override
  String get uiScale => 'UI méretarány';

  @override
  String get resetZoomTooltip => 'Nagyítás visszaállítása (Ctrl+0)';

  @override
  String get zoomTip =>
      'Tipp: a Ctrl + / Ctrl - bárhol a nagyítást állítja az alkalmazásban.';

  @override
  String get language => 'Felület';

  @override
  String get gameTextLanguage => 'Játékszöveg';

  @override
  String get gameTextLanguageHint =>
      'A felület nyelvének kiválasztása a megfelelő játékszöveget is beállítja. Utána a játékszöveget külön is megváltoztathatod.';

  @override
  String get updatesTitle => 'Frissítések';

  @override
  String get checkForUpdatesAutomatically => 'Frissítések automatikus keresése';

  @override
  String get checkForUpdatesNow => 'Frissítések keresése most';

  @override
  String get updatesPortableNotice =>
      'A hordozható verzió a letöltési oldalt nyitja meg a böngészőben. Cseréld le a meglévő fájlokat az új letöltésre.';

  @override
  String get updateAvailableTitle => 'Frissítés elérhető';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'A(z) $version verzió elérhető. Neked a(z) $current van.';
  }

  @override
  String get updateDownload => 'Letöltés';

  @override
  String updateOpenFailed(String url) {
    return 'Nem sikerült megnyitni a letöltési oldalt. Eléred itt: $url';
  }

  @override
  String get updateLater => 'Később';

  @override
  String get updateUpToDate => 'A legújabb verziót használod.';

  @override
  String get updateCheckFailed =>
      'Nem sikerült ellenőrizni a frissítéseket. Próbáld újra később.';

  @override
  String get gameTextTitle => 'Játékszöveg';

  @override
  String get itemImagesTitle => 'Tárgyképek';

  @override
  String get gameDataTitle => 'Játékadatok';

  @override
  String itemImagesReady(int count) {
    return '$count tárgykép készen áll.';
  }

  @override
  String get itemImagesUnavailable =>
      'A tárgyképek nem elérhetők. Helyettük kategóriaikonok lesznek használva.';

  @override
  String get checkRefreshItemImages => 'Tárgyképek ellenőrzése / frissítése';

  @override
  String get gameDataSourceMissing =>
      'A játékszöveg nem készíthető elő automatikusan. A lokalizációs gyorsítótárat a Beállításokban választhatod ki.';

  @override
  String get loadingTexts => 'Szövegek betöltése…';

  @override
  String get loadingImages => 'Képek betöltése…';

  @override
  String get preparing => 'Előkészítés…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Kibontva: $ids azonosító $languages nyelven.';
  }

  @override
  String get gameTextExtracted => 'A lokalizált játékszöveg kibontva.';

  @override
  String get gameTextNotExtracted =>
      'A lokalizált játékszöveg még nincs kibontva.';

  @override
  String get extracting => 'Kibontás…';

  @override
  String get extractRefreshLocalizedText =>
      'Lokalizált szöveg kibontása / frissítése';

  @override
  String get extractionComplete => 'Kibontás kész';

  @override
  String get extractionFailed => 'Kibontás sikertelen';

  @override
  String get localizationCacheFileType => 'Lokalizációs gyorsítótár';

  @override
  String get savegameDirectoryTitle => 'Mentéskönyvtár';

  @override
  String get folder => 'Mappa';

  @override
  String get codecTitle => 'Kodek';

  @override
  String get check => 'Ellenőrzés';

  @override
  String get roundtrip => 'Oda-vissza';

  @override
  String get noCodecStatus => 'Nincs kodekállapot';

  @override
  String get codecReady => 'Kodek kész';

  @override
  String get codecReadOnly => 'Kodek csak olvasható';

  @override
  String get codecUnavailable => 'Kodek nem elérhető';

  @override
  String get details => 'Részletek';

  @override
  String codecStatusLine(String status) {
    return 'Állapot: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Kibontás: $decompress | Tömörítés: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Háttér: $backend';
  }

  @override
  String get yes => 'igen';

  @override
  String get no => 'nem';

  @override
  String aboutVersion(String version, String sha) {
    return 'Verzió $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'MIT licenc alatt.';

  @override
  String difficultyTitle(String profile) {
    return 'Nehézség — $profile';
  }

  @override
  String get difficultyNoProfile => 'Nincs profil';

  @override
  String get difficultyNoDifficulty => 'Nincs nehézség';

  @override
  String get difficultyLabel => 'Nehézség';

  @override
  String get difficultyTooltipNoProfile => 'Nincs kiválasztott profil';

  @override
  String get difficultyTooltipEdit => 'Nehézség szerkesztése ehhez a profilhoz';

  @override
  String get difficultyTooltipNoEditable =>
      'Ennek a profilnak nincs szerkeszthető nehézsége';

  @override
  String get preset => 'Előbeállítás';

  @override
  String get presetNovice => 'Újonc';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Nehéz';

  @override
  String get presetCustom => 'Egyéni';

  @override
  String unrecognisedPreset(Object preset) {
    return 'A tárolt előbeállítás ismeretlen ($preset). A Flow Helper / Permadeath változásokat így is mentheted, vagy válassz fent egy előbeállítást a felülíráshoz.';
  }

  @override
  String get closeCombatFlowHelper => 'Közelharci Flow Helper';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Újonc nehézségen nem elérhető';

  @override
  String get levelCombat => 'Harc';

  @override
  String get levelResources => 'Erőforrások';

  @override
  String get levelProgression => 'Fejlődés';

  @override
  String get difficultyAppliesToAllSaves =>
      'A nehézség a profil összes mentésére vonatkozik.';

  @override
  String get savingDifficultyFailed => 'A nehézség mentése sikertelen.';

  @override
  String get addItemDialogTitle => 'Tárgy hozzáadása';

  @override
  String get searchItems => 'Tárgyak keresése';

  @override
  String failedToLoadCatalog(String error) {
    return 'A katalógus betöltése sikertelen: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Nincs hozzáadható tárgy';

  @override
  String get noItemsMatch => 'Nincs illő tárgy';

  @override
  String get countMustBeAtLeast1 => 'Legalább 1 legyen';

  @override
  String countMustBeAtMost(int max) {
    return 'Legfeljebb $max legyen';
  }

  @override
  String get addNpcDialogTitle => 'NPC hozzáadása';

  @override
  String get noNpcsAvailableToAdd => 'Nincs hozzáadható NPC';

  @override
  String get noNpcsMatch => 'Nincs illő NPC';

  @override
  String get categoryAll => 'Összes';

  @override
  String allWithCount(int count) {
    return 'Összes ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Tudásbejegyzés hozzáadása';

  @override
  String get searchEntries => 'Bejegyzések keresése';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nincs hozzáadható tudásbejegyzés';

  @override
  String get noEntriesMatch => 'Nincs illő bejegyzés';

  @override
  String get heroGroupMainStats => 'Fő értékek';

  @override
  String get heroGroupCombatMovement => 'Harc / mozgás';

  @override
  String get heroGroupResistances => 'Ellenállások';

  @override
  String get heroGroupThieving => 'Tolvajlás';

  @override
  String get heroGroupAdvanced => 'Haladó';

  @override
  String get heroGroupDiving => 'Búvárkodás';

  @override
  String get heroDivingSkillNote =>
      'Ha a Búvárkodás meg van tanulva, a játék mentés betöltésekor mindig a képesség saját értékeire állítja vissza a lélegzetet és a regenerációt. A másodpercenként elhasznált levegő marad, ahogy beállítottad.';

  @override
  String get heroGroupSleep => 'Alvás';

  @override
  String get heroGroupIntoxication => 'Mámor';

  @override
  String get heroEntryHeroTransform => 'Pozíció';

  @override
  String attributeEmpty(String name) {
    return 'A(z) $name üres — ments előtt adj meg értéket, vagy állítsd vissza az eredetit.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Érvénytelen szám a(z) $name mezőhöz: „$text”';
  }

  @override
  String get loadingEditorData => 'Szerkesztőadatok betöltése';

  @override
  String savingProgress(int done, int total) {
    return 'Mentés… $done / $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount azonosító kibontva $languageCount nyelven';
  }

  @override
  String get skillSmithing1H => 'Egykezes kovácsolás';

  @override
  String get skillSmithing2H => 'Kétkezes kovácsolás';

  @override
  String get skillCircleNovice => 'Újonc mágus';

  @override
  String get skillCircle1 => 'A mágia első köre';

  @override
  String get skillCircle2 => 'A mágia második köre';

  @override
  String get skillCircle3 => 'A mágia harmadik köre';

  @override
  String get skillCircle4 => 'A mágia negyedik köre';

  @override
  String get skillCircle5 => 'A mágia ötödik köre';

  @override
  String get skillCircle6 => 'A mágia hatodik köre';

  @override
  String get sectionGlossary => 'Glosszár';

  @override
  String get glossarySearch => 'Glosszár keresése';

  @override
  String get glossaryOldCamp => 'Régi Tábor';

  @override
  String get glossaryNewCamp => 'Új Tábor';

  @override
  String get glossarySwampCamp => 'Mocsári Tábor';

  @override
  String get glossaryOutsiders => 'Kívülállók';

  @override
  String get glossaryCreatures => 'Lények';

  @override
  String get glossaryLocations => 'Helyszínek';

  @override
  String get glossaryFilterLabel => 'Szűrő';

  @override
  String get glossaryFilterTraders => 'Kereskedők';

  @override
  String get glossaryFilterTeachers => 'Tanítók';

  @override
  String get roleTrader => 'Kereskedő';

  @override
  String get roleDead => 'Halott';

  @override
  String get roleTeacher => 'Tanító';

  @override
  String get roleArmorer => 'Páncélkészítő';

  @override
  String get glossaryFilterArmorers => 'Páncélkészítők';

  @override
  String get glossaryFilterHostile => 'Ellenséges';

  @override
  String get glossaryRelationshipFilterNote =>
      'A mentésben tárolt tartós ellenségfelülírásokat mutatja. A dinamikus céh-, történet-, terület- és bűnkapcsolatokat csak a játék számítja.';

  @override
  String get glossaryFilterDead => 'Halott';

  @override
  String get glossaryAddEntry => 'Glosszárbejegyzés hozzáadása';

  @override
  String get glossaryAddTitle => 'Glosszárbejegyzés hozzáadása';

  @override
  String get glossaryResetChanges => 'Glosszárváltozások visszaállítása';

  @override
  String get glossaryNoVisibleEntries =>
      'Ehhez a nézethez nincs látható glosszárbejegyzés.';

  @override
  String get glossaryNoHiddenEntries =>
      'Minden elérhető bejegyzés már látható.';

  @override
  String get glossaryNoMatch => 'Nincs illő glosszárbejegyzés.';

  @override
  String get glossarySelectEntry =>
      'Válassz glosszárbejegyzést a bejegyzéseinek szerkesztéséhez.';

  @override
  String glossaryEntryCount(int count) {
    return '$count bejegyzés';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked / $total bejegyzés';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portré feloldva';

  @override
  String get glossaryPortraitSilhouette => 'Sziluett — portré nincs feloldva';

  @override
  String get glossarySegments => 'Bejegyzések';

  @override
  String get glossaryPending => 'Nem mentett változás';

  @override
  String get glossaryShowFullText => 'Teljes bejegyzésszöveg megjelenítése';

  @override
  String get glossarySegmentIntroduction => 'Bevezető / portré';

  @override
  String get glossarySegmentUnlock => 'Felfedezés';

  @override
  String glossarySegmentEntry(int number) {
    return '$number. bejegyzés';
  }

  @override
  String get questJournalAll => 'Összes küldetés';

  @override
  String get questJournalOldCamp => 'Régi Tábor';

  @override
  String get questJournalNewCamp => 'Új Tábor';

  @override
  String get questJournalSwampCamp => 'Mocsári Tábor';

  @override
  String get questJournalColony => 'A Kolónia';

  @override
  String get questJournalCompleted => 'Teljesítve';

  @override
  String get questJournalHint =>
      'Játékbeli naplónézet. A belső és még el nem indított küldetésállapotok az Összes adat alatt továbbra is elérhetők.';

  @override
  String get questJournalNoEntries =>
      'Nincs a jelenlegi szűrőkhöz illő naplóküldetés.';

  @override
  String get glossaryTutorials => 'Oktatóanyagok';

  @override
  String get tutorialGateNote =>
      'Ezek a sorok a mentett oktatóanyag-feloldási kapukat vezérlik. Egy kapu nem feltétlenül felel meg egy az egyben egy játékbeli oktatóoldalnak.';

  @override
  String get tutorialResetChanges => 'Oktatóanyag-változások visszaállítása';

  @override
  String get tutorialNoGates =>
      'Ebben a mentésben nincsenek elérhető oktatóanyag-feloldási kapuk.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked / $total oktatókapu feloldva';
  }

  @override
  String get tutorialGateCombatBasics => 'Harc alapjai';

  @override
  String get tutorialGateCrafting => 'Készítés';

  @override
  String get tutorialGateCrime => 'Bűn és következmények';

  @override
  String get tutorialGateDrugs => 'Fogyasztható anyagok és hatások';

  @override
  String get tutorialGateLockpicking => 'Zárfeltörés';

  @override
  String get tutorialGateMagic => 'Mágia';

  @override
  String get tutorialGateMap => 'Térkép';

  @override
  String get tutorialGateMeleeCombat => 'Közelharc';

  @override
  String get tutorialGateNavigation => 'Mozgás és tájékozódás';

  @override
  String get tutorialGatePerception => 'Észlelés';

  @override
  String get tutorialGatePlayerProgression => 'Karakterfejlődés';

  @override
  String get tutorialGateRanged => 'Távharc';

  @override
  String get tutorialGateRiding => 'Lovaglás';

  @override
  String get tutorialGateSleep => 'Alvás';

  @override
  String get tutorialGateTrading => 'Kereskedés';

  @override
  String get windowMinimizeTooltip => 'Kis méret';

  @override
  String get windowMaximizeTooltip => 'Teljes méret';

  @override
  String get windowRestoreTooltip => 'Visszaállítás';

  @override
  String get fallbackDialogEntry => 'Dialógusbejegyzés';

  @override
  String get fallbackDialogChoice => 'Dialógusválasztás';

  @override
  String get fallbackDialogTopic => 'Dialógustéma';

  @override
  String get fallbackDialogInformation => 'Dialógusinformáció';

  @override
  String get fallbackQuest => 'Küldetés';

  @override
  String get fallbackObjective => 'Célkitűzés';

  @override
  String get fallbackItem => 'Tárgy';

  @override
  String get attributeSkillPointsFallback => 'Tanulópontok (TP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Állóképesség',
      'MaxSuperArmor': 'Maximális állóképesség',
      'DamageMultiplier': 'Kapott sebzés',
      'SpeedModifier': 'Mozgási sebesség',
      'Oxygen': 'Lélegzet',
      'MaxOxygen': 'Maximális lélegzet',
      'OxygenDepletionRate': 'Lélegzet/másodperc',
      'OxygenRecoveryRate': 'Lélegzet-visszatöltés/másodperc',
      'CriticalLevelPercent': 'Alacsony lélegzet figyelmeztetés',
      'SleepTime': 'Hátralévő pihentető órák',
      'MaxSleepTime': 'Maximális pihentető órák',
      'SleepTimeRecoveryAmount': 'Visszatöltött pihentető órák',
      'SleepTimeRecoveryPeriod': 'Újratöltési időköz',
      'MaxRestTime': 'Maximális ágyban töltött idő',
      'Health_RecoveryRatePerHourOfSleep': 'Életerő alvásóránként',
      'Mana_RecoveryRatePerHourOfSleep': 'Mana alvásóránként',
      'Alcohol': 'Alkoholszint',
      'MaxAlcohol': 'Maximális alkohol',
      'AlcoholDepletionRate': 'Kijózanodás sebessége',
      'Swampweed': 'Mocsári fű szint',
      'MaxSwampweed': 'Maximális mocsári fű',
      'SwampweedDepletionRate': 'Hatás elmúlásának sebessége',
      'XPExecutedBounty': 'TP a végső csapásért',
      'XPKillOrDefeatBounty': 'TP a legyőzésért',
      'Level': 'Szint',
      'LockpickDurability': 'Zárfeltörő tartósság',
      'LockpickPrecision': 'Zárfeltörő pontosság',
      'PickPocketing': 'Zsebtolvajlás',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor':
          'Mennyi ütést nyel el ez a karakter, mielőtt egy találat megtántorítaná.',
      'MaxSuperArmor':
          'A teljes állóképesség-készlet; a karakterszinttel és a viselt páncéllal nő.',
      'DamageMultiplier':
          'A karakter által kapott sebzésre alkalmazott szorzó — 1 a normál, magasabb többet sebez.',
      'SpeedModifier': 'A karakter mozgási sebességének szorzója — 1 a normál.',
      'Oxygen': 'Másodpercnyi levegő víz alatt; nullánál a karakter megfullad.',
      'MaxOxygen':
          'Hány másodpercig maradhat víz alatt; a Búvárkodás képesség növeli.',
      'OxygenDepletionRate':
          'Másodpercenként elhasznált levegő merülés közben.',
      'OxygenRecoveryRate':
          'Másodpercenként visszatérő levegő felszínre érkezés után.',
      'CriticalLevelPercent':
          'A fennmaradó levegő azon hányada, ahol a játék fulladásveszélyre figyelmeztet.',
      'SleepTime':
          'Azok az alvásórák, amelyek még visszaállítanak valamit; ezeken túl a játék nem ad pihenési bónuszt.',
      'MaxSleepTime':
          'A legnagyobb pihentetőóra-keret, amit ez a karakter tarthat.',
      'SleepTimeRecoveryAmount':
          'Visszatöltött pihentető órák minden keretfeltöltéskor.',
      'SleepTimeRecoveryPeriod':
          'Mennyi idő után töltődik újra a pihentetőóra-keret.',
      'MaxRestTime':
          'A leghosszabb egybefüggő ágyban maradás, amit a játék enged.',
      'Health_RecoveryRatePerHourOfSleep':
          'A maximális életerő alvásonkénti órára jutó része.',
      'Mana_RecoveryRatePerHourOfSleep':
          'A maximális mana alvásonkénti órára jutó része.',
      'Alcohol':
          'Mennyire részeg ez a karakter; a magasabb szintek ügyességet és manát erőre cserélnek.',
      'MaxAlcohol': 'A legmagasabb alkoholszint, amit ez a karakter elérhet.',
      'AlcoholDepletionRate':
          'Milyen gyorsan csökken az alkoholszint a józan állapot felé.',
      'Swampweed':
          'Mennyire bódult ez a karakter; a magasabb szintek átrendezik a tulajdonságait.',
      'MaxSwampweed':
          'A legmagasabb mocsárifű-szint, amit ez a karakter elérhet.',
      'SwampweedDepletionRate': 'Milyen gyorsan múlik el a mocsári fű hatása.',
      'XPExecutedBounty':
          'Tapasztalat ezért a karakterért, ha már legyőzve fekszik a földön, és megölik.',
      'XPKillOrDefeatBounty':
          'Tapasztalat ezért a karakterért, akár meghal, akár csak eszméletlenül verik le.',
      'Level': 'A karakterszint. Tapasztalattal nő, és tanulópontokat ad.',
      'LockpickDurability':
          'A Zárfeltörés képesség állítja: 2 gyakorlatlan, 4 kiképzett, 6 mester.',
      'LockpickPrecision':
          'A Zárfeltörés képesség állítja: 0 gyakorlatlan, 1 kiképzett, 2 mester.',
      'PickPocketing':
          'A Zsebtolvajlás képesség állítja: −30 gyakorlatlan, −10 kiképzett, +10 mester.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Hangsor';

  @override
  String get knowledgeTypeOther => 'Egyéb';

  @override
  String get armorUpgradeUpper => 'Felső';

  @override
  String get armorUpgradeMiddle => 'Középső';

  @override
  String get armorUpgradeLower => 'Alsó';

  @override
  String get knowledgeCategoryTopic => 'Téma';

  @override
  String get knowledgeCategoryChoice => 'Választás';

  @override
  String get knowledgeCategoryInfo => 'Információ';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Sikertelen';

  @override
  String get missingSaveReference => 'Hiányzó fájl';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'A(z) $slot.sav hiányzik. Lehet, hogy törölték, áthelyezték vagy átnevezték; a profil mégis hivatkozik rá.';
  }

  @override
  String get removeFromProfile => 'Eltávolítás a profilból';

  @override
  String get deleteSavegame => 'Mentés törlése';

  @override
  String get deleteSavegameTitle => 'Mentés törlése?';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return 'Törlöd a(z) $save ($fileName) mentést? Eltávolításra kerül a(z) $profile profilból, és törlődik a mentésmappából. A GORE előbb biztonsági mentést készít.';
  }

  @override
  String get removeSaveFromProfileTitle => 'Mentés eltávolítása a profilból?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Eltávolítod a(z) $save mentést a(z) $profile profilból? Maga a mentésfájl megmarad, ha még létezik.';
  }

  @override
  String get unassignedSave => 'Nincs profilhoz rendelve';

  @override
  String get armorUpgradeLight => 'Könnyű';

  @override
  String get armorUpgradeMedium => 'Közepes';

  @override
  String get armorUpgradeHeavy => 'Nehéz';

  @override
  String get knowledgeCaptionForcedConversation => 'Kényszerített beszélgetés';

  @override
  String get knowledgeCaptionFollowupTopic => 'Folytató téma';

  @override
  String get knowledgeCaptionFallbackTopic => 'Tartalék téma';

  @override
  String durationMinutes(int minutes) {
    return '$minutes perc';
  }

  @override
  String durationHours(int hours) {
    return '$hours óra';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours óra $minutes perc';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Érvénytelen profiladat';

  @override
  String get backupStatusSlotMetadataMissing =>
      'A kiválasztott mentés metaadata hiányzik';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Ismeretlen';

  @override
  String editorUnexpectedError(String details) {
    return 'Váratlan hiba: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Másik művelet folyamatban van. Próbáld újra egy pillanat múlva.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Nem mentett mentésszerkesztéseid vannak. Mentsd vagy állítsd vissza őket, mielőtt a profil nehézségét változtatnád.';

  @override
  String get editorNoSaveFolderSelected => 'Nincs kiválasztott mentésmappa.';

  @override
  String get editorNoSaveSelected => 'Nincs kiválasztott mentés.';

  @override
  String get coreUnknownError => 'Ismeretlen maghiba';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Előbb mentsd vagy állítsd vissza a nem mentett változásaidat — a profilváltás elmozdítana a jelenlegi mentéstől.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Mentsd vagy állítsd vissza a nem mentett változásaidat, mielőtt másik fájlt nyitnál.';

  @override
  String get editorSelectSavFile => 'Válassz egy .sav mentésfájlt.';

  @override
  String get editorNotGothicGsav =>
      'A kiválasztott fájl nem Gothic GSAV mentés.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Mentsd vagy állítsd vissza a nem mentett változásaidat, mielőtt a mentés profilját változtatnád.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Mentsd vagy állítsd vissza a nem mentett változásaidat, mielőtt mentést távolítanál el a profiljából.';

  @override
  String get editorUnsavedBeforeDeleteSave =>
      'Mentsd vagy állítsd vissza a nem mentett változásaidat, mielőtt ezt a mentést törölnéd.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Nem mentett mentésszerkesztéseid vannak. Mentsd vagy állítsd vissza őket, mielőtt profil biztonsági mentést állítanál vissza.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Ütköző, nem mentett szerkesztések ugyanarra a tulajdonságra ($path) mutatnak két lapról. Állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Egy glosszárszegmens-változás és egy másik, nem mentett Összes adat szerkesztés is a Hős MemorizedEvents tömbjére ($path) mutat. A glosszárváltozások bejegyzéseket adnak hozzá vagy távolítanak el abból a tömbből, ezért együtt nem menthetők — állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Egy glosszárszegmens-változás és egy másik, nem mentett szerkesztés ugyanarra a küldetés CurrentState tulajdonságára ($path) mutat. Maga a glosszárváltozás frissíti azt az állapotot — állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Egy kapcsolati felülírás és egy másik, nem mentett Összes adat szerkesztés ugyanarra az NPC kapcsolati bejegyzésre ($path) mutat. A strukturált kapcsolati változás felülírhatja a módosítókat abban a bejegyzésben, ezért együtt nem menthetők — állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Több mint egy nem mentett strukturális szerkesztés ugyanarra a tömbre ($path) mutat. Mentsd vagy állítsd vissza az első változást, mielőtt másikat állítanál sorba.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Egy strukturális eseményváltozás és egy másik, nem mentett Összes adat szerkesztés is a(z) $path útvonalra mutat. Mentsd vagy állítsd vissza az egyiket a folytatás előtt.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Egy Képességek változás és egy Összes adat szerkesztés ugyanarra az aktor effektjére (ActiveEffects › EffectSpec › Def) is sorban áll. Együtt nem menthetők — állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';

  @override
  String get editorInventoryResetConflict =>
      'Egy leltár-visszaállítás és egy másik, ugyanarra a leltárra vonatkozó szerkesztés is sorban áll. A visszaállítás az egész leltárt cseréli, és elvetné a másik szerkesztést — állítsd vissza vagy vedd vissza az egyiket, majd ments újra.';

  @override
  String get editorUseFolder => 'Mappa használata';

  @override
  String get editorGothicSavegameFileType => 'Gothic mentés';

  @override
  String get editorNoDifficultyChanges => 'Nincs írandó nehézségváltozás';

  @override
  String get editorDifficultyWritten =>
      'Nehézség a profilba írva (biztonsági mentés készült)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count változás mentve biztonsági mentéssel',
      one: '1 változás mentve biztonsági mentéssel',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'A mozgatás mentve, de a visszavonási jegyzete nem írható: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'A(z) $profileId profil nem található.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Nincs szabad mentslot a játék mentésmappájában (G1R-001–G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Mentés importálva és a(z) $profileId profilhoz rendelve';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Mentés a(z) $profileId profilhoz rendelve (páros biztonsági mentések készültek)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'A(z) $slot mentslot nincs a(z) $profileId profilhoz rendelve.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Mentés eltávolítva a profilból';

  @override
  String get editorSaveDeleted => 'Mentés törölve; biztonsági mentés készült';

  @override
  String editorRestoredBackup(String path) {
    return 'Biztonsági mentés visszaállítva: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Biztonsági mentés visszaállítva: $path (PersistentDataList.sav változatlan — nincs illő társbiztonsági mentés; a slot metaadata eltérhet)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Kodek oda-vissza sikeres: a(z) $chunkIndex. darab újratömörítve $bytes bájtra';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Nem sikerült a profil nehézségét írni: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Nem sikerült a mentést a profilhoz rendelni: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Nem sikerült a mentést eltávolítani a profilból: $details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return 'Nem sikerült törölni a mentést: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Nem sikerült menteni a változásokat: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'A mentések szkennelése sikertelen: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'A mentés vizsgálata sikertelen: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'A biztonsági mentések betöltése sikertelen: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Nem sikerült visszaállítani a biztonsági mentést: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Biztonsági mentés visszaállítva: $path, de a mentés újratöltése sikertelen: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Kodekellenőrzés sikertelen: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Kodek oda-vissza sikertelen: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Tulajdonságkeresés sikertelen: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'A mentés kiválasztása megváltozott a hőstulajdonságok betöltése közben.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Képességek betöltése sikertelen: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Fejlődési lekérdezés sikertelen: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'NPC-lista sikertelen: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Karakterlista sikertelen: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'NPC tulajdonságok sikertelenek: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Az NPC pozíciójának betöltése sikertelen: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'NPC leltár sikertelen: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Frakciólista sikertelen: $details';
  }

  @override
  String get editorNoBackupPath => 'nincs';

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
    return '$prefix: $backupPath; PersistentDataList biztonsági mentés: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Lokalizációs állapot sikertelen: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Kibontás sikertelen: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Glosszár betöltése sikertelen: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Biztonsági mentés hiba: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Küldetés',
      'document': 'Dokumentum',
      'story': 'Történet',
      'exploration': 'Felfedezés',
      'combat': 'Harc',
      'social': 'Társas',
      'item': 'Tárgyak',
      'learning': 'Tanulás',
      'guild': 'Céh',
      'crime': 'Bűn',
      'rest': 'Pihenés',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Küldetés elindítva',
      'questSucceeded': 'Küldetés teljesítve',
      'questFailed': 'Küldetés megbukott',
      'documentRead': 'Dokumentum elolvasva',
      'documentSegmentUnlocked': 'Bejegyzés felfedezve',
      'documentSegmentViewed': 'Bejegyzés megtekintve',
      'chapterCompleted': 'Fejezet befejezve',
      'areaEntered': 'Területre belépve',
      'areaLeft': 'Területről kilépve',
      'characterKilled': 'Karakter megölve',
      'characterDefeated': 'Karakter legyőzve',
      'combatDodge': 'Támadás kitérve',
      'characterDebuffed': 'Gyengítés alkalmazva',
      'tradeAvailable': 'Kereskedés feloldva',
      'itemObtained': 'Tárgy megszerezve',
      'itemCrafted': 'Tárgy elkészítve',
      'skillStateRecorded': 'Képességállapot rögzítve',
      'recipeLearned': 'Recept megtanulva',
      'guildJoined': 'Céhhez csatlakozva',
      'crimeRecorded': 'Bűncselekmény rögzítve',
      'slept': 'Alvás',
      'storyEvent': 'Történeti esemény',
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
      'gameTime': 'Játékidő',
      'duration': 'Időtartam',
      'chapter': 'Fejezet',
      'instigator': 'Kezdeményező',
      'affected': 'Érintett',
      'amount': 'Mennyiség',
      'primaryObject': 'Objektum',
      'secondaryObject': 'Kontextus',
      'segmentText': 'Bejegyzésszöveg',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return '$day. nap, $time';
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
  String get memoryEventHero => 'Hős';

  @override
  String get memoryEventDetails => 'Részletek';

  @override
  String get memoryEventTags => 'Címkék';

  @override
  String get memoryEventTechnicalData => 'Technikai adatok';

  @override
  String get memoryEventIndex => 'Index';

  @override
  String get memoryEventPosition => 'Pozíció';

  @override
  String get memoryEventPayload => 'Hasznos teher';

  @override
  String get memoryEventSubject => 'Alany';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Hozzáférés',
      'AccessDenied': 'Hozzáférés megtagadva',
      'AccesToTemple': 'Hozzáférés a templomhoz',
      'Advice': 'Tanács',
      'AfterFight': 'Harc után',
      'AfterFireMages': 'Tűzmágusok után',
      'AfterNek': 'Nek után',
      'AfterQuest': 'Küldetés után',
      'Alone': 'Egyedül',
      'Amulet': 'Amulett',
      'Annoying': 'Bosszantó',
      'Armor': 'Páncél',
      'Avoid': 'Kerülés',
      'Backstory': 'Háttértörténet',
      'BackStory': 'Háttértörténet',
      'BasicMagic': 'Alapmágia',
      'Beated': 'Megverve',
      'BecomeMercenary': 'Zsoldossá válás',
      'Beer': 'Sör',
      'Bestiary': 'Bestiárium',
      'Blessing': 'Áldás',
      'Boss': 'Főnök',
      'Bully': 'Zsaroló',
      'BullyAdvice': 'Zsaroló tanács',
      'Camp': 'Tábor',
      'CampDivided': 'Megosztott tábor',
      'CareOfMessengers': 'Hírnökök gondozása',
      'ChangeOpinion': 'Véleményváltozás',
      'ChargeUriziel': 'Uriziel feltöltése',
      'Chosen': 'Kiválasztott',
      'Contact': 'Kapcsolat',
      'Courier': 'Futár',
      'CraftBows': 'Íjak készítése',
      'Crazy': 'Őrült',
      'DailyMeal': 'Napi étkezés',
      'DailyRation_Trader': 'Napi adag kereskedő',
      'DAM': 'Gát',
      'Dead': 'Halott',
      'Deal': 'Üzlet',
      'Dealer': 'Kereskedő',
      'Deceived': 'Megtévesztve',
      'Dementia': 'Demencia',
      'DenyAccess': 'Hozzáférés megtagadása',
      'DifferentOpinion': 'Eltérő vélemény',
      'Discussion': 'Beszélgetés',
      'DontTalk': 'Ne beszélj',
      'Duel': 'Párbaj',
      'Entrance': 'Bejárat',
      'Escape': 'Menekülés',
      'Extended': 'Kibővített',
      'Extra': 'Extra',
      'ExtraInfo': 'Extra info',
      'Fanatic': 'Fanatikus',
      'Fight': 'Harc',
      'FindUlumulu': 'Ulu-Mulu megtalálása',
      'FireMages': 'Tűzmágusok',
      'FireMagesEscape': 'Tűzmágusok menekülése',
      'FiskNewDealer': 'Új orgazda Fisknek',
      'FiskNewDealerCompleted': 'Új orgazda Fisknek — kész',
      'FogTower': 'Ködtorony',
      'Food': 'Étel',
      'Forgave': 'Megbocsátott',
      'Forgive': 'Megbocsátás',
      'Forgiven': 'Megbocsátva',
      'FourFriends': 'Négy barát',
      'FreeHut': 'Szabad kunyhó',
      'FreeMine': 'Szabadbánya',
      'Fury': 'Düh',
      'GoodTeacher': 'Jó tanító',
      'Gossip': 'Pletyka',
      'GotScavenger': 'Dögevő megszerezve',
      'GrantedAccess': 'Hozzáférés megadva',
      'GRDArmor': 'Őrpáncél',
      'Guide': 'Útmutató',
      'HateMages': 'Gyűlöli a mágusokat',
      'HateMagesExplanation': 'Mágusgyűlölet magyarázata',
      'HateRiceLord': 'Gyűlöli a Rizsurat',
      'Heal': 'Gyógyítás',
      'Healing': 'Gyógyulás',
      'Help': 'Segítség',
      'Helper': 'Segítő',
      'HelpKagan': 'Kagan segítése',
      'HutStory': 'Kunyhó történet',
      'Ignore': 'Figyelmen kívül hagyás',
      'Impress': 'Lenyűgözés',
      'ImpressAlchemy': 'Alkímia lenyűgözése',
      'ImpressInscription': 'Rúnavésés lenyűgözése',
      'Info': 'Info',
      'Interested': 'Érdeklődő',
      'Introduction': 'Bevezető',
      'Introduction_2': 'Bevezető 2',
      'Introduction_Armor': 'Bevezető – páncél',
      'Introduction_Teacher': 'Bevezető – tanító',
      'Introduction_Trader': 'Bevezető – kereskedő',
      'Invocation': 'Idézés',
      'JoinSC': 'Csatlakozás a Mocsári Táborhoz',
      'Joint': 'Cigi',
      'KalomCamp': 'Kalom tábor',
      'Leader': 'Vezető',
      'Learning': 'Tanulás',
      'LearnOrcish': 'Ork nyelv tanulása',
      'LeftParty': 'Elhagyta a csapatot',
      'Library': 'Könyvtár',
      'Lie': 'Hazugság',
      'Lock': 'Zár',
      'Lockpick': 'Zárfeltörő',
      'Mad': 'Őrült',
      'Mandibles': 'Bányakúszó állkapcsok',
      'MapMaker': 'Térképkészítő',
      'Monastery': 'Kolostor',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Új Tábor',
      'NewCamper': 'Új táborlakó',
      'NewLeader': 'Új vezető',
      'NightPatrol': 'Éjszakai járőr',
      'NotInterested': 'Nem érdekli',
      'OldCamp': 'Régi Tábor',
      'OrcEnclaveEntrance': 'Ork enklávé bejárata',
      'OrcGraveyard': 'Ork temető',
      'OreArmor': 'Ércpáncél',
      'Party': 'Csapat',
      'Pay': 'Fizetés',
      'PayMoney': 'Pénzfizetés',
      'Permission': 'Engedély',
      'Pet': 'Háziállat',
      'PreparingInvocation': 'Idézés előkészítése',
      'Quest': 'Küldetés',
      'RankUpFireMages': 'Tűzmágus előléptetés',
      'RankUpGuard': 'Őr előléptetés',
      'RanUpFireMagesCompleted': 'Tűzmágus előléptetés kész',
      'Realocated': 'Áthelyezve',
      'Reason': 'Ok',
      'Respect': 'Tisztelet',
      'ReturnToSC': 'Visszatérés a Mocsári Táborba',
      'RicelordForeman': 'A Rizsur elöljárója',
      'RideScavenger': 'Dögevő lovaglás',
      'Robe': 'Köpeny',
      'Safe': 'Biztonságos',
      'Scraper': 'Bányász',
      'SecondChance': 'Második esély',
      'SecretLocation': 'Titkos hely',
      'SecretPassage': 'Titkos átjáró',
      'SecretPath': 'Titkos ösvény',
      'SleeperFollower': 'Az Alvó követője',
      'SleeperTemple': 'Az Alvó temploma',
      'SmallInfo': 'Kis info',
      'Stonehenge': 'Kőkör',
      'StopFollowing': 'Követés abbahagyása',
      'SwampCamp': 'Mocsári Tábor',
      'Talkative': 'Beszédes',
      'Teach': 'Tanítás',
      'TeachBow': 'Íj tanítása',
      'Teacher': 'Tanító',
      'Teacher2': 'Tanító 2',
      'TeacherInscription': 'Rúnavésés tanító',
      'TeacherMana': 'Mana tanító',
      'TeachIchor': 'Bányakúszó nedv kivonásának tanítása',
      'TeachMagic': 'Mágia tanítása',
      'TeachOrcish': 'Ork nyelv tanítása',
      'TeachStats': 'Tulajdonságok tanítása',
      'TeachWeapon': 'Fegyver tanítása',
      'Teleport': 'Teleport',
      'TheMysteriousOrc': 'A titokzatos ork',
      'ThroneRoom': 'Trónterem',
      'TradeBow': 'Íjkereskedelem',
      'Trader': 'Kereskedő',
      'TradeSkins_Trader': 'Bőrkereskedő',
      'Traitor': 'Áruló',
      'Trial': 'Próba',
      'TrollCanyon': 'Troll-kanyon',
      'Trust': 'Bizalom',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Tapasztalatlan',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Uriziel rúna',
      'Useful': 'Hasznos',
      'Velaya': 'Velaya',
      'Vibrations': 'Rezgések',
      'WaitFreeMine': 'Várakozás a Szabadbányánál',
      'WaitInTrainingArea': 'Várakozás a gyakorlótéren',
      'Warning': 'Figyelmeztetés',
      'WarningTooLate': 'A figyelmeztetés túl későn jött',
      'WaterMessenger': 'Hírnök a Vízmágusoknak',
      'Weapon': 'Fegyver',
      'Who': 'Ki',
      'Women': 'Nők',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Sérült leltárhelyek';

  @override
  String slotRepairBody(int count) {
    return 'Ez a mentés $count olyan leltárhelyet tartalmaz, amelyek azonosítója már nem egyezik a pozíciójukkal — a játékban az ilyen tárgy ledobása egy másikat távolít el. A javítás csak az azonosítókat írja át: semmi sem kerül hozzáadásra, eltávolításra vagy módosításra. Mentéskor mindig készül biztonsági mentés.';
  }

  @override
  String get slotRepairQueued =>
      'Javítás sorba állítva — mentsd a végrehajtáshoz.';

  @override
  String get slotRepairAction => 'Javítás';

  @override
  String get slotRepairDiscard => 'Elvetés';

  @override
  String get editorInventorySlotEditConflict =>
      'Egy leltárhely közvetlen szerkesztése együtt áll sorban egy egész helyeket érintő változással (javítás, hozzáadás vagy eltávolítás). A második felülírná az elsőt — vedd vissza az egyiket, majd ments újra.';

  @override
  String get editorTraderArrayConflict =>
      'Egy kereskedelmi változás együtt áll sorban a kereskedőtömb közvetlen szerkesztésével. Az a szerkesztés átszámozza azokat a sorokat, amelyekre a kereskedelmi változás hivatkozik, ezért az egyik rossz kereskedőre esne — vedd vissza az egyiket, majd ments újra.';

  @override
  String get backupFactFile => 'Fájl';

  @override
  String get renameBackupTooltip => 'Biztonsági mentés elnevezése';

  @override
  String get renameBackupTitle => 'Biztonsági mentés elnevezése';

  @override
  String get renameBackupLabel => 'Név';

  @override
  String renameBackupHelp(String fileName) {
    return 'A(z) $fileName fájlnév helyett jelenik meg. Hagyd üresen a név eltávolításához; magát a fájlt nem nevezi át.';
  }

  @override
  String get deleteBackupTooltip => 'Biztonsági mentés törlése';

  @override
  String get deleteBackupTitle => 'Biztonsági mentés törlése';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Törlöd a(z) „$name” ($fileName) fájlt? A fájl lekerül a lemezről, és nem hozható vissza.';
  }

  @override
  String get deleteBackupConfirm => 'Törlés';

  @override
  String editorDeletedBackup(String path) {
    return 'Biztonsági mentés törölve: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Nem sikerült törölni a biztonsági mentést: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Nem sikerült elnevezni a biztonsági mentést: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'A javítás most nem lehetséges — ez a mentés nem írható.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Biztonsági mentés törölve: $path — a neve nem távolítható el: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'A javítás ehhez a mentéshez nem érhető el.';

  @override
  String get statisticsTitle => 'Statisztikák';

  @override
  String get statisticsSubtitle =>
      'Kompakt összefoglaló a karakter, a küldetések, a világ és a mentés előrehaladásáról.';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': 'Idő',
      'character': 'Karakter',
      'quests': 'Küldetések',
      'progress': 'Haladás',
      'encounters': 'Harc és kapcsolatok',
      'inventory': 'Képességek és leltár',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': 'Játszott',
      'worldTime': 'Világidő',
      'level': 'Szint',
      'experience': 'Tapasztalat',
      'learningPoints': 'Tanulópontok',
      'guild': 'Céh',
      'health': 'Életerő',
      'mana': 'Mana',
      'chapter': 'Fejezet',
      'location': 'Helyszín',
      'kills': 'NPC-ölések',
      'knownCharacters': 'Ismert karakterek',
      'killedMonsters': 'Megölt szörnyek',
      'defeatedNpcs': 'Legyőzött NPC-k',
      'killedNpcs': 'Megölt NPC-k',
      'knownNpcs': 'Ismert NPC-k',
      'knownTeachers': 'Ismert tanítók',
      'learnedSkills': 'Megtanult képességek',
      'knowledge': 'Tudásbejegyzések',
      'deadCharacters': 'Halott karakterek',
      'traders': 'Ismert kereskedők',
      'inventoryStacks': 'Tárgycsomagok',
      'inventoryItems': 'Tárgyak',
      'ore': 'Érc',
      'equipped': 'Felszerelve',
      'hostileFactions': 'Ellenséges frakciók',
      'openCrimes': 'Nyitott bűncselekmények',
      'position': 'Pozíció',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': 'Régi Tábor · Árnyék',
      'oldCampGuard': 'Régi Tábor · Őr',
      'oldCampFireMage': 'Régi Tábor · Tűzmágus',
      'newCampRogue': 'Új Tábor · Bandita',
      'newCampMercenary': 'Új Tábor · Zsoldos',
      'newCampWaterMage': 'Új Tábor · Vízmágus',
      'swampCampNovice': 'Mocsári Tábor · Újonc',
      'swampCampTemplar': 'Mocsári Tábor · Templomos',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => 'Nem elérhető';

  @override
  String get statisticsMore => 'További statisztikák';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return '$level. szint, $guild, $chapter. fejezet. $completed küldetés teljesítve, $failed megbukott. Játékidő: $playTime.';
  }

  @override
  String get locksSidebar => 'Zárak';

  @override
  String editorLockListFailed(String details) {
    return 'Zárlista sikertelen: $details';
  }

  @override
  String get locksSearchHint => 'Zár vagy kulcs keresése';

  @override
  String get locksAllRegions => 'Összes régió';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$shown / $total';
  }

  @override
  String get locksFilterChests => 'Ládák';

  @override
  String get locksFilterDoors => 'Ajtók';

  @override
  String get locksFilterUnlocked => 'Feloldva';

  @override
  String get locksFilterLocked => 'Lezárva';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return 'Nehézség $bars / 4 (belső szint $level / 7)';
  }

  @override
  String get locksKeyOnly => 'Csak kulcs';

  @override
  String locksKeyLabel(String keys) {
    return 'Kulcs: $keys';
  }

  @override
  String get locksPermalocked => 'Véglegesen lezárva';

  @override
  String get locksReadOnly =>
      'Ennek a mentésnek nincs szerkeszthető zárkészlete.';

  @override
  String get locksUnknownEntry => 'Nincs ebben a játékverzióban';

  @override
  String get locksDoorLeafHint =>
      'Egy ajtó újrazárása magát az ajtót is becsukja.';

  @override
  String get locksResetPending => 'Sorba állított zárváltozások elvetése';
}
