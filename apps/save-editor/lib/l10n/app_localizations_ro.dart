// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Romanian Moldavian Moldovan (`ro`).
class AppLocalizationsRo extends AppLocalizations {
  AppLocalizationsRo([String locale = 'ro']) : super(locale);

  @override
  String get debugSectionTitle => 'Avansat (debug)';

  @override
  String get debugSectionSubtitle =>
      'Diagnostic și date brute pentru rapoarte de erori';

  @override
  String get showObjectIdsTitle => 'Afișează ID-uri tehnice suplimentare';

  @override
  String get showObjectIdsSubtitle =>
      'Afișează în editor ID-urile tehnice ale obiectelor, cunoștințelor din dialog, misiunilor și actorilor orfani. ID-urile NPC sunt mereu afișate.';

  @override
  String get storyStateSidebar => 'Stare poveste';

  @override
  String get storyStateDescription =>
      'Jocul ține aici evidența progresului la misiuni, dialoguri și evenimente. „Stocat” arată valorile din salvarea ta; „Nesetat” arată celelalte înregistrări cunoscute. În funcție de înregistrare, un număr poate însemna da/nu, un contor sau o etapă de progres. Marcajele de timp arată o zi și o oră în joc.';

  @override
  String get storyStateReadOnly =>
      'Doar citire până când sensul din script al valorilor și scrierile sigure în map sunt stabilite. Textul de glosar asociat e context, nu o traducere directă a ID-ului tehnic.';

  @override
  String get storyStateStructureReadOnly =>
      'Structura StoryPropertyValues din această salvare nu a putut fi rezolvată unic și în siguranță. Valorile de poveste rămân doar pentru citire în această salvare.';

  @override
  String get storyStateSearch => 'Caută stare poveste';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown din $total valori de poveste';
  }

  @override
  String get storyStateInteger => 'Întreg';

  @override
  String get storyStateTimeMarker => 'Marcaj de timp';

  @override
  String get storyStateChapter => 'Capitol';

  @override
  String get storyStateUnknown => 'Tip de sursă necunoscut';

  @override
  String storyStateShowDormant(int count) {
    return 'Arată nefolosite ($count)';
  }

  @override
  String get storyStateUnknownDetail =>
      'Acest ID stocat lipsește din catalogul actual de scripturi (de exemplu dintr-un mod sau o versiune mai nouă a jocului). Valoarea din salvare e int32, dar sensul ei nu e dedus.';

  @override
  String get storyStateStored => 'Stocat';

  @override
  String get storyStateUnset => 'Nesetat';

  @override
  String get storyStateUnsetDetail =>
      'Acest câmp din catalog nu e serializat în această salvare; jocul folosește deci starea nesetată sau implicită.';

  @override
  String get storyStateRawValue => 'Valoare brută';

  @override
  String storyStateElapsed(String duration) {
    return 'Trecut la momentul salvării: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Înaintea timpului salvării: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days zile',
      one: '1 zi',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Intrare de glosar asociată';

  @override
  String get storyStateTechnicalPath => 'Cale tehnică';

  @override
  String get storyStateEditingGuidance =>
      'Selectează o înregistrare ca să-i schimbi valoarea. Modificările au efect când salvezi. Schimbă doar valori ale căror efecte le înțelegi: altfel misiunile sau dialogurile pot nu mai funcționa cum te aștepți. La salvare se creează automat o copie de rezervă.';

  @override
  String get storyStatePending => 'În așteptare';

  @override
  String storyStatePendingValue(String value) {
    return 'Va fi stocat ca $value';
  }

  @override
  String get storyStatePendingRemoval => 'Va fi eliminat din salvare';

  @override
  String get storyStateEditValue => 'Editează valoarea';

  @override
  String get storyStateSetValue => 'Setează valoarea';

  @override
  String get storyStateRemoveValue => 'Elimină din salvare';

  @override
  String get storyStateUndoChange => 'Anulează modificarea de poveste';

  @override
  String get storyStateResetChanges => 'Resetează modificările de poveste';

  @override
  String storyStateDialogTitle(String id) {
    return 'Editează $id';
  }

  @override
  String get storyStateRawInput => 'Valoare int32 cu semn';

  @override
  String get storyStateInvalidInt32 =>
      'Introdu un număr întreg de la -2147483648 la 2147483647.';

  @override
  String get storyStateQueueChange => 'Pune modificarea în coadă';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valori atestate în scripturile livrate: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Sugestiile nu sunt limite de validare; codul nativ, modurile sau versiunile ulterioare pot folosi alte valori.';

  @override
  String get storyStateUseCurrentTime => 'Folosește timpul actual al salvării';

  @override
  String get storyStateStructuredTime => 'Zi / oră';

  @override
  String get storyStateRawMode => 'int32 brut';

  @override
  String get storyStateChapterWarning =>
      'Schimbarea doar a capitolului nu sincronizează misiunile, NPC-urile, inventarul sau starea lumii.';

  @override
  String get storyStateDormantWarning =>
      'Nu s-a găsit nicio citire sau scriere activă pentru acest câmp în cache-ul de scripturi livrat. Poate fi vechi, controlat nativ sau rezervat.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Scripturile livrate citesc acest câmp, dar nu conțin nicio scriere din script. Codul nativ îl poate totuși gestiona.';

  @override
  String get storyStateUnknownEditWarning =>
      'Acest ID din mod sau dintr-o versiune mai nouă nu are semantică de sursă inclusă. Editează doar valoarea int32 brută.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Flag binar',
      'finiteState': 'Valoare multi-stare',
      'counterOrScore': 'Contor / scor',
      'calendarDay': 'Zi calendaristică',
      'derivedOrOpaqueInteger': 'Întreg derivat / opac',
      'readOnlyInSourceInteger': 'Doar citire în scripturile livrate',
      'dormantOrLegacyInteger': 'Nefolosit în scripturile livrate',
      'other': 'Întreg',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Un 0 stocat și o înregistrare lipsă din map sunt stări de fișier distincte. „Elimină din salvare” restabilește starea din constructor/implicită.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo GORE Save Editor';

  @override
  String get zoomTooltip => 'Apasă Ctrl +/- pentru a mări/micșora';

  @override
  String get switchToLightMode => 'Treci la modul luminos';

  @override
  String get switchToDarkMode => 'Treci la modul întunecat';

  @override
  String get about => 'Despre';

  @override
  String get tabOverview => 'Prezentare';

  @override
  String get tabPlayer => 'Jucător';

  @override
  String get tabAttribute => 'Atribute';

  @override
  String get heroGroupSkills => 'Abilități';

  @override
  String get skillsNoneBody => 'Nu s-au găsit abilități pentru acest personaj.';

  @override
  String get skillsUnavailableBody =>
      'Abilitățile nu pot fi editate în această salvare — eroul nu are date de efect de modificat.';

  @override
  String get skillNotLearned => 'Neînvățat';

  @override
  String get skillLearn => 'Învață';

  @override
  String get skillActionLearn => 'învață';

  @override
  String get skillActionUnlearn => 'uită';

  @override
  String get skillTierUntrained => 'Neantrenat';

  @override
  String get skillTierBeginner => 'Începător';

  @override
  String get skillTierTrained => 'Antrenat';

  @override
  String get skillTierMaster => 'Maestru';

  @override
  String get skillTierNovice => 'Novice';

  @override
  String get skillTierAmateur => 'Amator (Cercul 0)';

  @override
  String get skillTierLearned => 'Învățat';

  @override
  String skillTierCircle(int n) {
    return 'Cercul $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Arme 1H';

  @override
  String get skillHintBlacksmith2H => 'Arme 2H';

  @override
  String get skillScutesTrained => 'Antrenat (plăci osoase)';

  @override
  String get skillScutesMaster => 'Maestru (+ plăci ascuțite)';

  @override
  String get skillCategoryCombat => 'Luptă';

  @override
  String get skillCategoryCrafting => 'Meșteșug';

  @override
  String get skillCategoryHunting => 'Vânătoare';

  @override
  String get skillCategoryLanguage => 'Limbă';

  @override
  String get skillCategoryMagic => 'Magie';

  @override
  String get skillCategoryMovement => 'Mișcare';

  @override
  String get skillCategoryThievery => 'Hoție';

  @override
  String get skillCategoryOther => 'Altele';

  @override
  String get skillNameOneHanded => 'O mână';

  @override
  String get skillNameTwoHanded => 'Două mâini';

  @override
  String get skillNameFists => 'Pumni';

  @override
  String get skillNameBow => 'Arc';

  @override
  String get skillNameCrossbow => 'Arbaletă';

  @override
  String get skillNameLockpicking => 'Spargerea lacătului';

  @override
  String get skillNamePickpocketing => 'Furatul din buzunare';

  @override
  String get skillNameTakeOrgans => 'Extrage organ';

  @override
  String get skillNameBreakTeeth => 'Extrage dinți';

  @override
  String get skillNameTakeClaws => 'Extrage gheară';

  @override
  String get skillNameSkinFur => 'Ia blană';

  @override
  String get skillNameSkin => 'Ia piele';

  @override
  String get skillNameTakeFins => 'Ia înotătoare';

  @override
  String get skillNameTakeStingers => 'Extrage țepi';

  @override
  String get skillNameTakeSecretion => 'Extrage secreție';

  @override
  String get skillNameTakeSkullPlates => 'Ia armură de craniu';

  @override
  String get skillNameSkinSwampshark => 'Ia piele de rechin';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Ia plăci';

  @override
  String get skillNameTakeScutes => 'Ia plăci osoase';

  @override
  String get skillNameTakeUluMulu => 'Ia Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Arme orcești';

  @override
  String get skillNameMining => 'Minerit';

  @override
  String get skillNameDiving => 'Scufundare';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Extrage mandibule';

  @override
  String get skillNameTakeShadowbeastHorn => 'Ia corn (bestie din umbră)';

  @override
  String get skillNameTakeSpines => 'Extrage coloană';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extrage dinți de rechin';

  @override
  String get skillNameTakeFireTongue => 'Ia limba focului';

  @override
  String get skillNameTakeTrollHorn => 'Ia corn (troll)';

  @override
  String get skillNameAcrobatics => 'Acrobație';

  @override
  String get skillNameWallClimbing => 'Cățărare';

  @override
  String get skillNameRiding => 'Călărit scavenger';

  @override
  String get skillNameSneaking => 'Furișare';

  @override
  String get skillNameAlchemy => 'Alchimie';

  @override
  String get skillNameRuneInscription => 'Inscripție';

  @override
  String get skillNameBlacksmithing => 'Fierărie';

  @override
  String get skillNameMagicCircle => 'Cerc magic';

  @override
  String get skillNameOrcish => 'Orcește';

  @override
  String get tabInventory => 'Inventar';

  @override
  String get tabTrade => 'Negustorie';

  @override
  String get traderNotAMerchant => 'Acest personaj nu face negoț.';

  @override
  String get traderRetry => 'Încearcă din nou';

  @override
  String get traderAmbiguousName =>
      'Mai multe înregistrări de negustor poartă acest nume, deci editorul nu poate ști care magazin aparține acestui personaj. Editarea e dezactivată ca să nu riști să schimbi pe greșit.';

  @override
  String get traderOre => 'Minereu (putere de cumpărare)';

  @override
  String get traderNoOre => 'fără minereu';

  @override
  String get traderStockCurrent => 'Stoc';

  @override
  String get traderStockCurrentTooltip =>
      'Ce are acest negustor de vânzare acum. Obiectele adăugate pot dispărea din nou când jocul actualizează negustorul.';

  @override
  String get traderStockBase => 'Bază de reaprovizionare';

  @override
  String get traderStockBaseTooltip =>
      'Salvarea conține această listă ca să ajute jocul să reaprovizioneze negustorul. Jocul o poate recalcula după regulile sale de negustor, deci modificările de aici nu ar ține.';

  @override
  String get traderStockBaseHint =>
      'Doar citire: jocul folosește această listă la reaprovizionare, dar o poate recalcula. Obiectele adăugate aici nu ar rămâne permanent.';

  @override
  String get traderCurrentStockWarning =>
      'Modificările la inventarul negustorului țin doar până la următoarea reaprovizionare.';

  @override
  String get traderRestockTitle => 'Temporizator reaprovizionare';

  @override
  String get traderRestockTitleTooltip =>
      'O estimare pe baza ultimei activități a negustorului, a timpului actual din joc și a dificultății Resurse.';

  @override
  String get traderRestockPending => 'în așteptare';

  @override
  String get traderRestockRevertTooltip =>
      'Anulează modificarea de timp în așteptare';

  @override
  String get traderRestockNever => 'Niciodată';

  @override
  String get traderRestockUnavailable => 'Indisponibil';

  @override
  String get traderRestockIntervalUnknown =>
      'Așteptarea de reaprovizionare e necunoscută';

  @override
  String get traderRestockNeverStatus =>
      'Nu a fost înregistrată încă nicio activitate de negustor.';

  @override
  String get traderRestockClockAhead =>
      'Timpul salvat al negustorului e înaintea timpului actual din joc.';

  @override
  String traderRestockNotDueYet(String time) {
    return 'Nu e de așteptat înainte de $time.';
  }

  @override
  String get traderRestockPossiblyDue =>
      'Negustorul ar putea fi deja gata de reaprovizionare.';

  @override
  String get traderRestockEligible =>
      'Negustorul ar trebui să fie acum gata de reaprovizionare.';

  @override
  String get traderRestockNoWorldTime =>
      'Timpul actual din joc e indisponibil, deci editorul nu poate spune dacă reaprovizionarea e gata.';

  @override
  String get traderRestockLastActivity => 'Ultima activitate a negustorului';

  @override
  String get traderRestockLastActivityTooltip =>
      'Ultimul timp salvat pentru acest negustor. Poate veni din negoț sau din altă actualizare de negustor, deci nu e neapărat ultima reaprovizionare.';

  @override
  String get traderRestockForecastWindow => 'Reaprovizionare așteptată';

  @override
  String get traderRestockForecastWindowTooltip =>
      'Ora exactă nu e stocată în salvare. Editorul arată deci un interval de la cel mai devreme la cel mai târziu timp așteptat.';

  @override
  String get traderRestockIntervalLabel => 'Așteptare reaprovizionare';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days zile · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      'Timp de așteptare setat de dificultatea Resurse: Novice 2, Gothic 3, Greu 5 zile din joc.';

  @override
  String get traderRestockAutomationLabel => 'Reaprovizionare automată';

  @override
  String get traderRestockAutomationValue =>
      'Nu poate fi dezactivată în salvare';

  @override
  String get traderRestockAutomationTooltip =>
      'Editorul de salvare nu poate opri în mod fiabil reaprovizionarea automată. E nevoie de un mod de joc.';

  @override
  String get traderRestockSetNow => 'Setează la timpul lumii';

  @override
  String get traderRestockSetNowTooltip =>
      'Folosește timpul actual din joc ca ultima activitate a negustorului. Asta amână următoarea reaprovizionare așteptată.';

  @override
  String get traderRestockMakeDue => 'Fă-o gata acum';

  @override
  String get traderRestockMakeDueTooltip =>
      'Mută ultima activitate a negustorului suficient de înapoi încât reaprovizionarea să fie gata acum.';

  @override
  String get traderRestockCustom => 'Oră personalizată…';

  @override
  String get traderRestockCustomTooltip =>
      'Alege ziua și ora din joc ale ultimei activități a negustorului.';

  @override
  String get traderRestockEditTitle =>
      'Modifică ultima activitate a negustorului';

  @override
  String get traderOreHint =>
      'Cifra din joc e diferită: la încărcare jocul adaugă ce s-a acumulat de la ultimul său negoț — vinde surplusul și se reaprovizionează din el. Acest număr e punctul de plecare, nu ce arată ecranul de negoț.';

  @override
  String get traderOreHintShort =>
      'Valoare de pornire — suma din ecranul de negoț poate diferi.';

  @override
  String get traderRestockStatusLabel => 'Stare';

  @override
  String get traderRestockStatusNever => 'Fără activitate';

  @override
  String get traderRestockStatusWaiting => 'Așteaptă reaprovizionarea';

  @override
  String get traderRestockStatusReady => 'Gata de reaprovizionare';

  @override
  String get traderRestockStatusPossiblyReady => 'Posibil gata';

  @override
  String get traderRestockStatusCheckTime => 'Verifică timpul salvat';

  @override
  String get traderRestockStatusUnknown => 'Necunoscut';

  @override
  String get traderPriceWarning =>
      'Prețurile reacționează la cât stochează un negustor și cât minereu ține, deci schimbarea acestor numere poate muta și ce cere.';

  @override
  String get traderAddItem => 'Adaugă obiect';

  @override
  String get traderRemoveItem => 'Elimină linia';

  @override
  String get traderReadOnlyCore =>
      'Acest nucleu poate doar citi datele negustorului.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Acest negustor are stoc pe dificultate, pe care editorul nu îl modelează. Editarea e dezactivată aici, pentru că o modificare ar părea reușită lăsând acel stoc extra neatins.';

  @override
  String get traderRecordIncomplete =>
      'Listele de stoc ale acestui negustor lipsesc sau au o formă pe care editorul nu o suportă și nu o poate scrie. Editarea e dezactivată aici ca o modificare să nu eșueze la salvare.';

  @override
  String get traderEmptyStock => 'Nimic în stoc.';

  @override
  String get traderUnknownItem => 'nu e în catalogul de obiecte';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Încărcarea negustorilor a eșuat: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count linii';
  }

  @override
  String get tabWorld => 'Lume';

  @override
  String get tabCharacters => 'Personaje';

  @override
  String get characterNoActorBody =>
      'Acest personaj nu are actor în lume, deci nu are atribute, inventar sau evenimente.';

  @override
  String get characterNoEventsBody => 'Niciun eveniment pentru acest personaj.';

  @override
  String get characterOrphanGroup => 'Altele';

  @override
  String get tabAllData => 'Toate datele';

  @override
  String get tabBackups => 'Copii de rezervă';

  @override
  String get tabSettings => 'Setări';

  @override
  String get reset => 'Resetează';

  @override
  String get save => 'Salvează';

  @override
  String saveWithCount(int count) {
    return 'Salvează ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Anulează';

  @override
  String get confirm => 'Confirmă';

  @override
  String get close => 'Închide';

  @override
  String get add => 'Adaugă';

  @override
  String get equippedBadge => 'Echipat';

  @override
  String get armorUpgradesLabel => 'Îmbunătățiri';

  @override
  String get browse => 'Răsfoiește';

  @override
  String get noSavFilesFound => 'Nu s-au găsit fișiere .sav';

  @override
  String get profile => 'Profil';

  @override
  String get otherSaves => 'Alte salvări';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count salvări)';
  }

  @override
  String get switchProfile => 'Schimbă profilul';

  @override
  String get openSaveFile => 'Deschide fișier';

  @override
  String get externalSave => 'Salvare deschisă din exterior';

  @override
  String get saveProfileTitle => 'Profil de salvare';

  @override
  String get saveProfileDescription =>
      'Atribuie această salvare unui alt profil de joc. Salvarea și indexul profilului primesc împreună o copie de rezervă.';

  @override
  String get saveProfileExternalHint =>
      'Selectează un profil ca să imporți acest fișier în folderul de salvări al jocului și să-l înregistrezi acolo. Fișierul original rămâne neschimbat.';

  @override
  String get saveProfileNoProfiles =>
      'Nu s-au găsit profile de joc editabile în PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Selectează profilul';

  @override
  String get rescanSaveFolder => 'Rescanează folderul de salvări';

  @override
  String get discardUnsavedChangesTitle => 'Renunți la modificările nesalvate?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'modificări nesalvate',
      one: 'modificare nesalvată',
    );
    return 'Rescanarea reîncarcă fiecare salvare și renunță la cele $count $_temp0.';
  }

  @override
  String get discardAndRescan => 'Renunță și rescanază';

  @override
  String chapterLabel(Object id) {
    return 'Capitolul $id';
  }

  @override
  String get quickSave => 'Salvare rapidă';

  @override
  String get autoSave => 'Salvare automată';

  @override
  String get manualSave => 'Salvare manuală';

  @override
  String get errorTitle => 'Eroare';

  @override
  String get selectASaveTitle => 'Selectează o salvare';

  @override
  String get selectASaveBody => 'Detaliile salvării vor apărea aici.';

  @override
  String bytesValue(String count) {
    return '$count octeți';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspecție';

  @override
  String get copy => 'Copiază';

  @override
  String get savegameFallbackTitle => 'Salvare';

  @override
  String screenshotForSlot(String slot) {
    return 'Captură pentru $slot';
  }

  @override
  String get publicSaveName => 'Nume';

  @override
  String get gameTimeTitle => 'Timp de joc';

  @override
  String get gameTimeDay => 'Zi';

  @override
  String get gameTimeHours => 'Ore';

  @override
  String get gameTimeMinutes => 'Minute';

  @override
  String get gameTimeSeconds => 'Secunde';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s în total';
  }

  @override
  String get gameTimeInvalid =>
      'Introdu numere întregi — zi ≥ 0, ore 0–23, minute și secunde 0–59.';

  @override
  String get required => 'Obligatoriu';

  @override
  String get playerLockedBody =>
      'Editările private ale jucătorului au nevoie de un codec pregătit pentru comprimare.';

  @override
  String get heroTransform => 'Poziție';

  @override
  String get locationX => 'Locație X';

  @override
  String get locationY => 'Locație Y';

  @override
  String get locationZ => 'Locație Z';

  @override
  String get rotationPitch => 'Rotație pitch';

  @override
  String get rotationYaw => 'Rotație yaw';

  @override
  String get rotationRoll => 'Rotație roll';

  @override
  String get spawnPositionSection => 'Poziție de spawn (referință)';

  @override
  String get resetToSpawnPosition => 'Resetează la poziția de spawn';

  @override
  String get positionOutOfRange =>
      'Valoarea trebuie să fie între −10.000.000 și 10.000.000';

  @override
  String get positionNotEditable =>
      'Poziția stocată nu a putut fi citită pentru acest personaj, deci nu poate fi editată.';

  @override
  String get positionNeverPlaced =>
      'Acest personaj nu a fost niciodată plasat în lume (poziție 0, 0, 0) — jocul poate ignora poziția stocată.';

  @override
  String get npcStayInPlace => 'Dezactivează-i rutina zilnică';

  @override
  String get npcStayInPlaceHint => 'Rămâne atunci unde e.';

  @override
  String get npcStayInPlaceLocked =>
      'Rutina lui zilnică originală nu e înregistrată, deci asta nu mai poate fi anulată.';

  @override
  String get npcUndoPlacement => 'Anulează mutarea';

  @override
  String get npcUndoPlacementStale =>
      'Salvarea nu mai ține ce a scris acea mutare, deci restaurarea ar renunța la ce s-a întâmplat de atunci.';

  @override
  String get positionNotReadable =>
      'Poziția stocată nu a putut fi citită pentru acest personaj.';

  @override
  String get npcPositionReadOnly =>
      'Jocul restaurează poziția unui NPC din nivel, nu din salvare, deci aceste valori pot fi citite, dar nu schimbate.';

  @override
  String get pickLocation => 'Alege locația…';

  @override
  String get pickLocationDialogTitle => 'Alege o locație';

  @override
  String get applySpotRotation => 'Aplică și orientarea punctului';

  @override
  String get locationAreaOther => 'Altele';

  @override
  String get locationAreaCavalornValley => 'Valea lui Cavalorn';

  @override
  String get locationAreaEastForest => 'Pădurea de Est';

  @override
  String get locationAreaFogTower => 'Turnul de Ceață';

  @override
  String get locationAreaIllegalWeedMixers => 'Amestecătorii ilegali de iarbă';

  @override
  String get locationAreaOrcArena => 'Arena orcilor';

  @override
  String get locationAreaOrcGraveyard => 'Cimitirul orcilor';

  @override
  String get locationAreaShipwreck => 'Epavă';

  @override
  String get locationAreaTundra => 'Tundră';

  @override
  String get locationCatalogUnavailable =>
      'Catalogul de locații nu a putut fi încărcat.';

  @override
  String get invalid => 'Nevalid';

  @override
  String get heroAttributes => 'Atribute erou';

  @override
  String attributeBase(String name) {
    return '$name bază';
  }

  @override
  String attributeCurrent(String name) {
    return '$name curent';
  }

  @override
  String get attributeBaseValue => 'Valoare de bază';

  @override
  String get attributeCurrentValue => 'Valoare curentă';

  @override
  String get inventoryTitle => 'Inventar';

  @override
  String get inventoryEmpty => 'Acest inventar e gol.';

  @override
  String get inventoryNeedsDecoded =>
      'Editarea inventarului are nevoie de date private decodate din codec.';

  @override
  String get inventoryNoStacks =>
      'Nu s-au găsit stive de obiecte în payload-ul privat decodat.';

  @override
  String get resetInventoryChanges => 'Resetează modificările de inventar';

  @override
  String get addItemTooltipPendingAdd =>
      'Salvează mai întâi modificările în așteptare — un obiect nou pe salvare';

  @override
  String get addItemTooltipPendingRemove =>
      'Salvează mai întâi eliminarea în așteptare — o modificare structurală pe salvare';

  @override
  String get addItemTooltipPendingCount =>
      'Salvează sau resetează mai întâi modificările de număr în așteptare — o editare structurală trebuie salvată separat';

  @override
  String get addItemTooltipDefault => 'Adaugă obiect în inventar';

  @override
  String get addItemButton => 'Adaugă obiect';

  @override
  String get resetInventoryButton => 'Resetează inventarul';

  @override
  String get resetInventoryTooltipDefault =>
      'Înlocuiește acest inventar cu inventarul salvării de la începutul jocului';

  @override
  String get resetInventoryTooltipBlocked =>
      'Salvează sau anulează mai întâi modificările de inventar în așteptare';

  @override
  String get pendingResetTitle =>
      'Resetează la inventarul de la începutul jocului';

  @override
  String pendingResetSubtitle(String level) {
    return 'Nivel resurse: $level';
  }

  @override
  String get cancelPendingReset => 'Anulează resetarea';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — adăugare în așteptare (încă nesalvată)';
  }

  @override
  String get cancelPendingAdd => 'Anulează adăugarea în așteptare';

  @override
  String get pendingRemovalSubtitle =>
      'eliminare în așteptare (încă nesalvată)';

  @override
  String get cancelPendingRemoval => 'Anulează eliminarea în așteptare';

  @override
  String get filterItems => 'Filtrează obiecte';

  @override
  String noItemsMatchQuery(String query) {
    return 'Niciun obiect nu corespunde cu \"$query\".';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Eliminarea în așteptare ascunde fiecare obiect — salvează ca să o aplici.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemTooltipIngredientFor => 'Ingredient pentru';

  @override
  String itemTooltipTeaches(String item) {
    return 'Predă: $item';
  }

  @override
  String get itemTooltipValue => 'Valoare';

  @override
  String get itemTooltipProtection => 'Protecție';

  @override
  String get itemTooltipRequirements => 'Cerințe:';

  @override
  String get itemTooltipManaCost => 'Cost mana';

  @override
  String get itemTooltipManaUpkeep => 'Cost mana de încărcare';

  @override
  String get itemCategoryAll => 'Toate';

  @override
  String get itemCategoryMeleeWeapon => 'Arme corp la corp';

  @override
  String get itemCategoryRangedWeapon => 'Arme la distanță';

  @override
  String get itemCategoryMagic => 'Magie';

  @override
  String get itemCategoryWearable => 'Echipamente';

  @override
  String get itemCategoryFood => 'Mâncare';

  @override
  String get itemCategoryPotion => 'Poțiuni';

  @override
  String get itemCategoryMaterial => 'Materiale';

  @override
  String get itemCategoryDocument => 'Documente';

  @override
  String get itemCategoryMisc => 'Diverse';

  @override
  String get itemCategoryArtefact => 'Artefacte';

  @override
  String get itemCategoryOther => 'Altele';

  @override
  String get count => 'Număr';

  @override
  String get min1 => 'Min. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Nu se poate șterge: acest obiect e probabil echipat sau atribuit unui slot de tastă rapidă';

  @override
  String get removeBlockedTooltip =>
      'Salvează sau resetează mai întâi modificările de inventar în așteptare — o adăugare sau eliminare trebuie salvată separat';

  @override
  String get removeItemFromInventory => 'Elimină obiectul din inventar';

  @override
  String get progressionLockedBody =>
      'Datele de progres au nevoie de payload privat decodat din codec.';

  @override
  String get progressionNeedsTyped =>
      'Datele de progres structurate au nevoie de o salvare complet decodată cu o analiză tipizată verificată.';

  @override
  String get sectionQuests => 'Misiuni';

  @override
  String get sectionKnowledge => 'Cunoștințe';

  @override
  String get sectionEvents => 'Evenimente';

  @override
  String get firstPage => 'Prima pagină';

  @override
  String get previousPage => 'Pagina anterioară';

  @override
  String get nextPage => 'Pagina următoare';

  @override
  String get lastPage => 'Ultima pagină';

  @override
  String pageOfPages(int page, int total) {
    return 'Pagina $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last din $total';
  }

  @override
  String get perPage => 'Pe pagină:';

  @override
  String get resetQuestChanges => 'Resetează modificările de misiuni';

  @override
  String get searchQuests => 'Caută misiuni';

  @override
  String get allGroups => 'Toate grupurile';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Niciuna';

  @override
  String get questStateAvailable => 'Disponibilă';

  @override
  String get questStateRunning => 'În curs';

  @override
  String get questStateSucceeded => 'Reușită';

  @override
  String get questStateFailed => 'Eșuată';

  @override
  String get questStateUnknown => 'necunoscut';

  @override
  String get dialogKnowledge => 'Cunoștințe din dialog';

  @override
  String get resetKnowledgeChanges => 'Resetează modificările de cunoștințe';

  @override
  String get addNpc => 'Adaugă NPC';

  @override
  String get searchNpcs => 'Caută NPC-uri';

  @override
  String get npcStatusRowLabel => 'Stare';

  @override
  String get npcStatusAlive => 'în viață';

  @override
  String get npcStatusDead => 'mort';

  @override
  String get npcRelationshipRowLabel => 'Relație';

  @override
  String get npcRelationshipUnavailable => 'Starea relației e indisponibilă';

  @override
  String get npcRelationshipAutomatic => 'Calculată de joc';

  @override
  String get npcRelationshipAutomaticHint =>
      'Nu e stocată nicio suprascriere permanentă. Regulile de gildă, poveste, zonă și infracțiuni sunt evaluate în joc.';

  @override
  String get npcRelationshipStoredHint =>
      'Stocată ca suprascriere permanentă NPC–jucător. Regulile de gildă, poveste, zonă și infracțiuni pot totuși schimba starea efectivă în joc.';

  @override
  String get npcRelationshipFriend => 'Prieten';

  @override
  String get npcRelationshipNeutral => 'Neutru';

  @override
  String get npcRelationshipEnemy => 'Inamic';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Va fi $relationship la salvare';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Reînvie';

  @override
  String get npcReviveQueued => 'Va fi reînviat la salvare';

  @override
  String entriesForCharacter(String name) {
    return 'Intrări — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Selectează un NPC ca să vezi intrările';

  @override
  String get addKnowledgeEntry => 'Adaugă intrare de cunoștințe';

  @override
  String get browseCatalog => 'Răsfoiește catalogul';

  @override
  String get alreadyExistsForCharacter => 'Există deja pentru acest personaj.';

  @override
  String get alreadyInPendingChanges => 'E deja în modificările în așteptare.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Verificarea de duplicate a eșuat — încearcă din nou: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Adăugări în așteptare ($count)';
  }

  @override
  String get undoAdd => 'Anulează adăugarea';

  @override
  String get undoRemove => 'Anulează eliminarea';

  @override
  String get removeEntry => 'Elimină intrarea';

  @override
  String get selectNpcFromList => 'Selectează un NPC din listă';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Evenimente din memorie';

  @override
  String get searchCharacters => 'Caută personaje';

  @override
  String eventsForCharacter(String name) {
    return 'Evenimente — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Selectează un personaj ca să vezi evenimentele';

  @override
  String get noTags => '(fără etichete)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Elimină evenimentul';

  @override
  String get removeMemoryEventTitle => 'Elimini evenimentul din memorie?';

  @override
  String get removeMemoryEventBody =>
      'Pui acest eveniment din memorie în coadă pentru eliminare? Fișierul de salvare se schimbă doar când apeși Salvează.';

  @override
  String get memoryEventRemovalQueued =>
      'Eliminarea evenimentului e în coadă — apasă Salvează ca să o aplici.';

  @override
  String get duplicateEvent => 'Duplică evenimentul';

  @override
  String get duplicateMemoryEventTitle => 'Duplici evenimentul din memorie?';

  @override
  String get duplicateMemoryEventBody =>
      'Pui un duplicat al acestui eveniment din memorie în coadă? Fișierul de salvare se schimbă doar când apeși Salvează.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplicarea evenimentului e în coadă — apasă Salvează ca să o aplici.';

  @override
  String get selectCharacterFromList => 'Selectează un personaj din listă';

  @override
  String get factionsSidebar => 'Facțiuni';

  @override
  String get factionsForgiveButton => 'Iartă';

  @override
  String get factionHostile => 'Ostil';

  @override
  String get factionFriendly => 'Prietenos';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count crime',
      one: '$count crimă',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count agresiuni',
      one: '$count agresiune',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count furturi',
      one: '$count furt',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count intruziuni',
      one: '$count intruziune',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count amenințări',
      one: '$count amenințare',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count alte infracțiuni',
      one: '$count altă infracțiune',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'în curs de iertare…';

  @override
  String get factionsEmpty =>
      'Nicio infracțiune deschisă împotriva facțiunilor.';

  @override
  String get factionGuildOldCamp => 'Lagărul Vechi';

  @override
  String get factionGuildNewCamp => 'Lagărul Nou';

  @override
  String get factionGuildSwampCamp => 'Lagărul din Mlaștină';

  @override
  String get factionGuildOther => 'Alții / indivizi';

  @override
  String get allDataLockedBody =>
      'Browserul exhaustiv al surselor e disponibil momentan pentru fișierele de salvare GSAV.';

  @override
  String get allDataDescription =>
      'Răsfoiește metadatele GSAV și fiecare nod tipizat PUBLIC/PRIVATE. Valorile scalare și native-struct sigure sunt editabile; containerele și octeții opaci rămân vizibili.';

  @override
  String get allDataEditable => 'Editabil';

  @override
  String get allDataReadOnly => 'Doar citire';

  @override
  String get allDataType => 'Tip';

  @override
  String get allDataScalars => 'Scalare';

  @override
  String get allDataStructs => 'Structuri';

  @override
  String get allDataContainers => 'Containere';

  @override
  String get allDataOpaque => 'Opac';

  @override
  String get allDataNodes => 'Noduri';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count elemente fii',
      one: '1 element fiu',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'În așteptare';

  @override
  String get allDataTagInputHint =>
      'Etichete separate prin virgulă sau pe linii';

  @override
  String allDataTypedSource(String source) {
    return '$source tipizat';
  }

  @override
  String get searchPropertiesLabel =>
      'Caută proprietăți (gol = listează totul) — ex. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Se decodează salvarea…';

  @override
  String get decodingSaveBody =>
      'Se decodează întregul payload privat pentru prima căutare. Rulează o dată pe salvare, apoi căutările sunt instantanee.';

  @override
  String get searchTheSaveTitle => 'Caută în salvare';

  @override
  String get searchTheSaveBody =>
      'Tastează un nume de proprietate și apasă Enter. Lasă gol ca să listezi totul.';

  @override
  String get searchFailedTitle => 'Căutarea a eșuat';

  @override
  String get noMatchesTitle => 'Nicio potrivire';

  @override
  String get noMatchesBody =>
      'Nicio cale de proprietate nu conținea toți termenii aceia.';

  @override
  String get value => 'Valoare';

  @override
  String get backupsTitle => 'Copii de rezervă';

  @override
  String get refreshBackups => 'Reîmprospătează copiile de rezervă';

  @override
  String get noBackupsTitle => 'Fără copii de rezervă';

  @override
  String get noBackupsBody =>
      'Salvările editate creează fișiere de rezervă lângă slotul selectat.';

  @override
  String get slotBackups => 'Copii pe slot';

  @override
  String get profileBackups => 'Copii pe profil';

  @override
  String get backupFactName => 'Nume';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Creat';

  @override
  String get backupFactSize => 'Dimensiune';

  @override
  String get backupFactStatus => 'Stare';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restaurează $fileName';
  }

  @override
  String get appearanceTitle => 'Aspect';

  @override
  String get uiFont => 'Font';

  @override
  String get theme => 'Temă';

  @override
  String get themeLight => 'Luminos';

  @override
  String get themeDark => 'Întunecat';

  @override
  String get themeSystem => 'Sistem';

  @override
  String get uiScale => 'Scară UI';

  @override
  String get resetZoomTooltip => 'Resetează zoomul (Ctrl+0)';

  @override
  String get zoomTip =>
      'Sfat: Ctrl + / Ctrl - schimbă zoomul oriunde în aplicație.';

  @override
  String get language => 'Interfață';

  @override
  String get gameTextLanguage => 'Text de joc';

  @override
  String get gameTextLanguageHint =>
      'Alegerea limbii interfeței selectează și textul de joc corespunzător. Poți alege ulterior un alt text de joc.';

  @override
  String get updatesTitle => 'Actualizări';

  @override
  String get checkForUpdatesAutomatically => 'Verifică actualizările automat';

  @override
  String get checkForUpdatesNow => 'Verifică actualizările acum';

  @override
  String get updatesPortableNotice =>
      'Versiunea portabilă deschide pagina de descărcare în browser. Înlocuiește fișierele existente cu noua descărcare.';

  @override
  String get updateAvailableTitle => 'Actualizare disponibilă';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Versiunea $version e disponibilă. Tu ai $current.';
  }

  @override
  String get updateDownload => 'Descarcă';

  @override
  String updateOpenFailed(String url) {
    return 'Nu s-a putut deschide pagina de descărcare. O poți găsi la $url';
  }

  @override
  String get updateLater => 'Mai târziu';

  @override
  String get updateUpToDate => 'Folosești cea mai recentă versiune.';

  @override
  String get updateCheckFailed =>
      'Nu s-au putut verifica actualizările. Încearcă din nou mai târziu.';

  @override
  String get gameTextTitle => 'Text de joc';

  @override
  String get itemImagesTitle => 'Imagini obiecte';

  @override
  String get gameDataTitle => 'Date de joc';

  @override
  String itemImagesReady(int count) {
    return '$count imagini de obiecte sunt gata.';
  }

  @override
  String get itemImagesUnavailable =>
      'Imaginile obiectelor nu sunt disponibile. Se vor folosi în schimb iconițele de categorie.';

  @override
  String get checkRefreshItemImages =>
      'Verifică / reîmprospătează imaginile obiectelor';

  @override
  String get gameDataSourceMissing =>
      'Textul de joc nu a putut fi pregătit automat. Poți selecta cache-ul de localizare din Setări.';

  @override
  String get loadingTexts => 'Se încarcă textele…';

  @override
  String get loadingImages => 'Se încarcă imaginile…';

  @override
  String get preparing => 'Se pregătește…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extras: $ids id-uri în $languages limbi.';
  }

  @override
  String get gameTextExtracted => 'Textul de joc localizat e extras.';

  @override
  String get gameTextNotExtracted =>
      'Textul de joc localizat nu e încă extras.';

  @override
  String get extracting => 'Se extrage…';

  @override
  String get extractRefreshLocalizedText =>
      'Extrage / reîmprospătează textul localizat';

  @override
  String get extractionComplete => 'Extragere finalizată';

  @override
  String get extractionFailed => 'Extragerea a eșuat';

  @override
  String get localizationCacheFileType => 'Cache de localizare';

  @override
  String get savegameDirectoryTitle => 'Director de salvări';

  @override
  String get folder => 'Dosar';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Verifică';

  @override
  String get roundtrip => 'Roundtrip';

  @override
  String get noCodecStatus => 'Fără stare codec';

  @override
  String get codecReady => 'Codec gata';

  @override
  String get codecReadOnly => 'Codec doar citire';

  @override
  String get codecUnavailable => 'Codec indisponibil';

  @override
  String get details => 'Detalii';

  @override
  String codecStatusLine(String status) {
    return 'Stare: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Decomprimare: $decompress | Comprimare: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'da';

  @override
  String get no => 'nu';

  @override
  String aboutVersion(String version, String sha) {
    return 'Versiunea $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Licențiat sub Licența MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Dificultate — $profile';
  }

  @override
  String get difficultyNoProfile => 'Fără profil';

  @override
  String get difficultyNoDifficulty => 'Fără dificultate';

  @override
  String get difficultyLabel => 'Dificultate';

  @override
  String get difficultyTooltipNoProfile => 'Niciun profil selectat';

  @override
  String get difficultyTooltipEdit =>
      'Editează dificultatea pentru acest profil';

  @override
  String get difficultyTooltipNoEditable =>
      'Acest profil nu are dificultate editabilă';

  @override
  String get preset => 'Presetare';

  @override
  String get presetNovice => 'Novice';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Greu';

  @override
  String get presetCustom => 'Personalizat';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Presetarea stocată nu e recunoscută ($preset). Poți totuși salva modificările Flow Helper / Permadeath, sau alege o presetare de mai sus ca să o suprascrii.';
  }

  @override
  String get closeCombatFlowHelper =>
      'Ajutor pentru fluiditatea luptei corp la corp';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Indisponibil pe Novice';

  @override
  String get levelCombat => 'Luptă';

  @override
  String get levelResources => 'Resurse';

  @override
  String get levelProgression => 'Progres';

  @override
  String get difficultyAppliesToAllSaves =>
      'Dificultatea se aplică tuturor salvărilor din acest profil.';

  @override
  String get savingDifficultyFailed => 'Salvarea dificultății a eșuat.';

  @override
  String get addItemDialogTitle => 'Adaugă obiect';

  @override
  String get searchItems => 'Caută obiecte';

  @override
  String failedToLoadCatalog(String error) {
    return 'Încărcarea catalogului a eșuat: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Niciun obiect disponibil de adăugat';

  @override
  String get noItemsMatch => 'Niciun obiect nu corespunde';

  @override
  String get countMustBeAtLeast1 => 'Trebuie ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Trebuie ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Adaugă NPC';

  @override
  String get noNpcsAvailableToAdd => 'Niciun NPC disponibil de adăugat';

  @override
  String get noNpcsMatch => 'Niciun NPC nu corespunde';

  @override
  String get categoryAll => 'Toate';

  @override
  String allWithCount(int count) {
    return 'Toate ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Adaugă intrare de cunoștințe';

  @override
  String get searchEntries => 'Caută intrări';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nicio intrare de cunoștințe disponibilă de adăugat';

  @override
  String get noEntriesMatch => 'Nicio intrare nu corespunde';

  @override
  String get heroGroupMainStats => 'Statistici principale';

  @override
  String get heroGroupCombatMovement => 'Luptă / Mișcare';

  @override
  String get heroGroupResistances => 'Rezistențe';

  @override
  String get heroGroupThieving => 'Hoție';

  @override
  String get heroGroupAdvanced => 'Avansat';

  @override
  String get heroGroupDiving => 'Scufundare';

  @override
  String get heroDivingSkillNote =>
      'Odată ce Scufundarea e învățată, jocul resetează respirația și regenerarea la valorile abilității de fiecare dată când se încarcă salvarea. Aerul consumat pe secundă rămâne cum l-ai setat.';

  @override
  String get heroGroupSleep => 'Somn';

  @override
  String get heroGroupIntoxication => 'Intoxicare';

  @override
  String get heroEntryHeroTransform => 'Poziție';

  @override
  String attributeEmpty(String name) {
    return '$name e gol — introdu o valoare sau restaurează originalul înainte de salvare.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Număr invalid pentru $name: \"$text\"';
  }

  @override
  String get loadingEditorData => 'Se încarcă datele editorului';

  @override
  String savingProgress(int done, int total) {
    return 'Se salvează… $done din $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extrase $idCount id-uri în $languageCount limbi';
  }

  @override
  String get skillSmithing1H => 'Fierărie o mână';

  @override
  String get skillSmithing2H => 'Fierărie două mâini';

  @override
  String get skillCircleNovice => 'Magician novice';

  @override
  String get skillCircle1 => 'Primul cerc al magiei';

  @override
  String get skillCircle2 => 'Al doilea cerc al magiei';

  @override
  String get skillCircle3 => 'Al treilea cerc al magiei';

  @override
  String get skillCircle4 => 'Al patrulea cerc al magiei';

  @override
  String get skillCircle5 => 'Al cincilea cerc al magiei';

  @override
  String get skillCircle6 => 'Al șaselea cerc al magiei';

  @override
  String get sectionGlossary => 'Glosar';

  @override
  String get glossarySearch => 'Caută în glosar';

  @override
  String get glossaryOldCamp => 'Lagărul Vechi';

  @override
  String get glossaryNewCamp => 'Lagărul Nou';

  @override
  String get glossarySwampCamp => 'Lagărul din Mlaștină';

  @override
  String get glossaryOutsiders => 'Străini';

  @override
  String get glossaryCreatures => 'Creaturi';

  @override
  String get glossaryLocations => 'Locații';

  @override
  String get glossaryFilterLabel => 'Filtru';

  @override
  String get glossaryFilterTraders => 'Negustori';

  @override
  String get glossaryFilterTeachers => 'Învățători';

  @override
  String get roleTrader => 'Negustor';

  @override
  String get roleDead => 'Mort';

  @override
  String get roleTeacher => 'Învățător';

  @override
  String get roleArmorer => 'Armurier';

  @override
  String get glossaryFilterArmorers => 'Armurieri';

  @override
  String get glossaryFilterHostile => 'Ostili';

  @override
  String get glossaryRelationshipFilterNote =>
      'Arată suprascrierile permanente de inamic stocate în salvare. Relațiile dinamice de gildă, poveste, zonă și infracțiuni sunt calculate doar în joc.';

  @override
  String get glossaryFilterDead => 'Morți';

  @override
  String get glossaryAddEntry => 'Adaugă intrare în glosar';

  @override
  String get glossaryAddTitle => 'Adaugă intrare în glosar';

  @override
  String get glossaryResetChanges => 'Resetează modificările de glosar';

  @override
  String get glossaryNoVisibleEntries =>
      'Nicio intrare vizibilă din glosar nu corespunde acestei vederi.';

  @override
  String get glossaryNoHiddenEntries =>
      'Fiecare intrare disponibilă e deja vizibilă.';

  @override
  String get glossaryNoMatch => 'Nicio intrare din glosar nu corespunde.';

  @override
  String get glossarySelectEntry =>
      'Selectează o intrare din glosar ca să-i editezi înregistrările.';

  @override
  String glossaryEntryCount(int count) {
    return '$count intrări';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked din $total intrări';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portret deblocat';

  @override
  String get glossaryPortraitSilhouette => 'Siluetă — portretul nu e deblocat';

  @override
  String get glossarySegments => 'Intrări';

  @override
  String get glossaryPending => 'Modificare nesalvată';

  @override
  String get glossaryShowFullText => 'Arată textul complet al intrării';

  @override
  String get glossarySegmentIntroduction => 'Introducere / portret';

  @override
  String get glossarySegmentUnlock => 'Descoperire';

  @override
  String glossarySegmentEntry(int number) {
    return 'Intrarea $number';
  }

  @override
  String get questJournalAll => 'Toate misiunile';

  @override
  String get questJournalOldCamp => 'Lagărul Vechi';

  @override
  String get questJournalNewCamp => 'Lagărul Nou';

  @override
  String get questJournalSwampCamp => 'Lagărul din Mlaștină';

  @override
  String get questJournalColony => 'Colonia';

  @override
  String get questJournalCompleted => 'Finalizate';

  @override
  String get questJournalHint =>
      'Vedere din jurnalul din joc. Stările interne și cele încă neîncepute rămân disponibile sub Toate datele.';

  @override
  String get questJournalNoEntries =>
      'Nicio misiune din jurnal nu corespunde filtrelor curente.';

  @override
  String get glossaryTutorials => 'Tutoriale';

  @override
  String get tutorialGateNote =>
      'Aceste rânduri controlează porțile de deblocare a tutorialelor salvate. O poartă nu mapează neapărat unu-la-unu o pagină individuală de tutorial din joc.';

  @override
  String get tutorialResetChanges => 'Resetează modificările de tutorial';

  @override
  String get tutorialNoGates =>
      'Nicio poartă de deblocare a tutorialelor nu e disponibilă în această salvare.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked din $total porți de tutorial deblocate';
  }

  @override
  String get tutorialGateCombatBasics => 'Bazele luptei';

  @override
  String get tutorialGateCrafting => 'Meșteșug';

  @override
  String get tutorialGateCrime => 'Infracțiuni și consecințe';

  @override
  String get tutorialGateDrugs => 'Consumabile și efecte';

  @override
  String get tutorialGateLockpicking => 'Spargerea lacătului';

  @override
  String get tutorialGateMagic => 'Magie';

  @override
  String get tutorialGateMap => 'Hartă';

  @override
  String get tutorialGateMeleeCombat => 'Luptă corp la corp';

  @override
  String get tutorialGateNavigation => 'Mișcare și navigare';

  @override
  String get tutorialGatePerception => 'Percepție';

  @override
  String get tutorialGatePlayerProgression => 'Progresul personajului';

  @override
  String get tutorialGateRanged => 'Luptă la distanță';

  @override
  String get tutorialGateRiding => 'Călărit';

  @override
  String get tutorialGateSleep => 'Somn';

  @override
  String get tutorialGateTrading => 'Negustorie';

  @override
  String get windowMinimizeTooltip => 'Minimizează';

  @override
  String get windowMaximizeTooltip => 'Maximizează';

  @override
  String get windowRestoreTooltip => 'Restaurează';

  @override
  String get fallbackDialogEntry => 'Intrare de dialog';

  @override
  String get fallbackDialogChoice => 'Alegere de dialog';

  @override
  String get fallbackDialogTopic => 'Subiect de dialog';

  @override
  String get fallbackDialogInformation => 'Informație de dialog';

  @override
  String get fallbackQuest => 'Misiune';

  @override
  String get fallbackObjective => 'Obiectiv';

  @override
  String get fallbackItem => 'Obiect';

  @override
  String get attributeSkillPointsFallback => 'Puncte de abilități (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Echilibru',
      'MaxSuperArmor': 'Echilibru maxim',
      'DamageMultiplier': 'Daune primite',
      'SpeedModifier': 'Viteza de mișcare',
      'Oxygen': 'Respirație',
      'MaxOxygen': 'Respirație maximă',
      'OxygenDepletionRate': 'Respirație consumată pe secundă',
      'OxygenRecoveryRate': 'Respirație recuperată pe secundă',
      'CriticalLevelPercent': 'Avertisment respirație scăzută',
      'SleepTime': 'Ore de odihnă rămase',
      'MaxSleepTime': 'Ore de odihnă maxime',
      'SleepTimeRecoveryAmount': 'Ore de odihnă recuperate',
      'SleepTimeRecoveryPeriod': 'Interval de reumplere',
      'MaxRestTime': 'Timp maxim în pat',
      'Health_RecoveryRatePerHourOfSleep': 'Viață pe oră de somn',
      'Mana_RecoveryRatePerHourOfSleep': 'Mana pe oră de somn',
      'Alcohol': 'Nivel de alcool',
      'MaxAlcohol': 'Alcool maxim',
      'AlcoholDepletionRate': 'Viteza de trezire din beție',
      'Swampweed': 'Nivel de iarbă de mlaștină',
      'MaxSwampweed': 'Iarbă de mlaștină maximă',
      'SwampweedDepletionRate': 'Viteza de dispariție',
      'XPExecutedBounty': 'XP pentru lovitura de grație',
      'XPKillOrDefeatBounty': 'XP pentru înfrângere',
      'Level': 'Nivel',
      'LockpickDurability': 'Durabilitate șperaclu',
      'LockpickPrecision': 'Precizie șperaclu',
      'PickPocketing': 'Furatul din buzunare',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor':
          'Câtă pedeapsă absoarbe acest personaj înainte ca o lovitură să-l clatine.',
      'MaxSuperArmor':
          'Rezerva completă de echilibru; crește cu nivelul personajului și cu armura purtată.',
      'DamageMultiplier':
          'Factor aplicat daunelor pe care le primește acest personaj — 1 e normal, mai mare doare mai tare.',
      'SpeedModifier':
          'Factor al vitezei cu care se mișcă acest personaj — 1 e normal.',
      'Oxygen':
          'Secunde de aer rămase sub apă; la zero acest personaj se îneacă.',
      'MaxOxygen':
          'Câte secunde poate rămâne acest personaj sub apă; abilitatea Scufundare o mărește.',
      'OxygenDepletionRate': 'Aer consumat în fiecare secundă cât e scufundat.',
      'OxygenRecoveryRate':
          'Aer care revine în fiecare secundă după ieșirea la suprafață.',
      'CriticalLevelPercent':
          'Ponderea de aer rămas la care jocul avertizează de înec.',
      'SleepTime':
          'Ore de somn care mai restabilesc ceva; dincolo de ele jocul nu mai acordă bonus de odihnă.',
      'MaxSleepTime':
          'Cel mai mare buget de ore de odihnă pe care îl poate ține acest personaj.',
      'SleepTimeRecoveryAmount':
          'Ore de odihnă adăugate înapoi de fiecare dată când se reumple bugetul.',
      'SleepTimeRecoveryPeriod':
          'Cât durează până se reumple din nou bugetul de ore de odihnă.',
      'MaxRestTime':
          'Cea mai lungă ședere neîntreruptă în pat pe care o permite jocul.',
      'Health_RecoveryRatePerHourOfSleep':
          'Ponderea din viața maximă restabilită pentru fiecare oră dormită.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Ponderea din mana maximă restabilită pentru fiecare oră dormită.',
      'Alcohol':
          'Cât de beat e acest personaj; treptele mai înalte schimbă dexteritatea și mana pe forță.',
      'MaxAlcohol':
          'Cel mai înalt nivel de alcool pe care îl poate atinge acest personaj.',
      'AlcoholDepletionRate':
          'Cât de repede scade nivelul de alcool spre trezie.',
      'Swampweed':
          'Cât de drogat e acest personaj; treptele mai înalte îi mută atributele.',
      'MaxSwampweed':
          'Cel mai înalt nivel de iarbă de mlaștină pe care îl poate atinge acest personaj.',
      'SwampweedDepletionRate':
          'Cât de repede trece starea de pe iarbă de mlaștină.',
      'XPExecutedBounty':
          'Experiență pentru a ucide acest personaj pe când zace deja învins la pământ.',
      'XPKillOrDefeatBounty':
          'Experiență pentru a doborî acest personaj, fie că moare, fie că e doar bătut până la leșin.',
      'Level':
          'Nivelul personajului. Crește cu experiența și acordă puncte de învățare.',
      'LockpickDurability':
          'Setat de abilitatea Spargerea lacătului: 2 neantrenat, 4 antrenat, 6 maestru.',
      'LockpickPrecision':
          'Setat de abilitatea Spargerea lacătului: 0 neantrenat, 1 antrenat, 2 maestru.',
      'PickPocketing':
          'Setat de abilitatea Furatul din buzunare: -30 neantrenat, -10 antrenat, +10 maestru.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Replică vocală';

  @override
  String get knowledgeTypeOther => 'Altele';

  @override
  String get armorUpgradeUpper => 'Sus';

  @override
  String get armorUpgradeMiddle => 'Mijloc';

  @override
  String get armorUpgradeLower => 'Jos';

  @override
  String get knowledgeCategoryTopic => 'Subiect';

  @override
  String get knowledgeCategoryChoice => 'Alegere';

  @override
  String get knowledgeCategoryInfo => 'Informație';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Eșuat';

  @override
  String get missingSaveReference => 'Fișier lipsă';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav lipsește. Poate a fost șters, mutat sau redenumit; profilul încă îl referă.';
  }

  @override
  String get removeFromProfile => 'Elimină din profil';

  @override
  String get deleteSavegame => 'Șterge salvarea';

  @override
  String get deleteSavegameTitle => 'Ștergi salvarea?';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return 'Ștergi $save ($fileName)? Va fi eliminat din $profile și șters din folderul de salvări. GORE creează mai întâi o copie de rezervă.';
  }

  @override
  String get removeSaveFromProfileTitle => 'Elimini salvarea din profil?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Elimini $save din $profile? Fișierul de salvare în sine e păstrat dacă încă există.';
  }

  @override
  String get unassignedSave => 'Neatribuit unui profil';

  @override
  String get armorUpgradeLight => 'Ușor';

  @override
  String get armorUpgradeMedium => 'Mediu';

  @override
  String get armorUpgradeHeavy => 'Greu';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversație forțată';

  @override
  String get knowledgeCaptionFollowupTopic => 'Subiect de continuare';

  @override
  String get knowledgeCaptionFallbackTopic => 'Subiect de rezervă';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours h';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours h $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Date de profil invalide';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Metadatele salvării selectate lipsesc';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Necunoscut';

  @override
  String editorUnexpectedError(String details) {
    return 'Eroare neașteptată: $details';
  }

  @override
  String get editorOperationInProgress =>
      'O altă operațiune e în curs. Încearcă din nou într-o clipă.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Ai editări de salvare nesalvate. Salvează-le sau resetează-le înainte să schimbi dificultatea profilului.';

  @override
  String get editorNoSaveFolderSelected => 'Niciun folder de salvări selectat.';

  @override
  String get editorNoSaveSelected => 'Nicio salvare selectată.';

  @override
  String get coreUnknownError => 'Eroare de nucleu necunoscută';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Salvează sau resetează mai întâi modificările nesalvate — schimbarea profilului te-ar îndepărta de salvarea curentă.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Salvează sau resetează modificările nesalvate înainte să deschizi alt fișier.';

  @override
  String get editorSelectSavFile => 'Selectează un fișier de salvare .sav.';

  @override
  String get editorNotGothicGsav =>
      'Fișierul selectat nu e o salvare Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Salvează sau resetează modificările nesalvate înainte să schimbi profilul salvării.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Salvează sau resetează modificările nesalvate înainte să elimini o salvare din profilul ei.';

  @override
  String get editorUnsavedBeforeDeleteSave =>
      'Salvează sau resetează modificările nesalvate înainte să ștergi această salvare.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Ai editări de salvare nesalvate. Salvează-le sau resetează-le înainte să restaurezi o copie de rezervă a profilului.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Editări nesalvate conflictuale țintesc aceeași proprietate ($path) din două file. Resetează sau anulează una dintre ele, apoi salvează din nou.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'O modificare de segment din glosar și o altă editare nesalvată din Toate datele țintesc ambele array-ul Hero MemorizedEvents ($path). Modificările de glosar adaugă sau elimină înregistrări din acel array, deci editările nu pot fi salvate împreună — resetează sau anulează una dintre ele, apoi salvează din nou.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'O modificare de segment din glosar și o altă editare nesalvată țintesc aceeași proprietate CurrentState a misiunii ($path). Modificarea de glosar actualizează chiar acea stare — resetează sau anulează una dintre ele, apoi salvează din nou.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'O suprascriere de relație și o altă editare nesalvată din Toate datele țintesc ambele aceeași înregistrare de relație NPC ($path). Modificarea structurată de relație poate înlocui modificatorii din acea înregistrare, deci editările nu pot fi salvate împreună — resetează sau anulează una dintre ele, apoi salvează din nou.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Mai mult de o editare structurală nesalvată țintește același array ($path). Salvează sau resetează prima modificare înainte să pui alta în coadă.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'O modificare structurală de eveniment și o altă editare nesalvată din Toate datele țintesc ambele $path. Salvează sau resetează una dintre ele înainte să continui.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'O modificare de Abilități și o editare din Toate datele a aceluiași efect de actor (ActiveEffects › EffectSpec › Def) sunt ambele în coadă. Nu pot fi salvate împreună — resetează sau anulează una dintre ele, apoi salvează din nou.';

  @override
  String get editorInventoryResetConflict =>
      'O resetare de inventar și o altă editare a aceluiași inventar sunt ambele în coadă. Resetarea înlocuiește întregul inventar și ar anula cealaltă editare — resetează sau anulează una dintre ele, apoi salvează din nou.';

  @override
  String get editorUseFolder => 'Folosește folderul';

  @override
  String get editorGothicSavegameFileType => 'Salvare Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Nicio modificare de dificultate de scris';

  @override
  String get editorDifficultyWritten =>
      'Dificultatea a fost scrisă în profil (copie de rezervă creată)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count modificări salvate cu copie de rezervă',
      one: '1 modificare salvată cu copie de rezervă',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Mutarea a fost salvată, dar nota de anulare nu a putut fi scrisă: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profilul $profileId nu a fost găsit.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Niciun slot de salvare liber nu e disponibil în folderul de salvări al jocului (G1R-001 până la G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Salvare importată și atribuită profilului $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Salvare atribuită profilului $profileId (copii de rezervă pereche create)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Slotul de salvare $slot nu e atribuit profilului $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Salvare eliminată din profil';

  @override
  String get editorSaveDeleted => 'Salvare ștearsă; copie de rezervă creată';

  @override
  String editorRestoredBackup(String path) {
    return 'Copie de rezervă restaurată: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Copie de rezervă restaurată: $path (PersistentDataList.sav lăsat neschimbat — fără copie companion potrivită; metadatele slotului pot diferi)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Roundtrip codec reușit: chunk $chunkIndex recomprimat la $bytes octeți';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Nu s-a putut scrie dificultatea profilului: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Nu s-a putut atribui salvarea profilului: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Nu s-a putut elimina salvarea din profil: $details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return 'Nu s-a putut șterge salvarea: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Nu s-au putut salva modificările: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Scanarea salvărilor a eșuat: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Inspecția salvării a eșuat: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Încărcarea copiilor de rezervă a eșuat: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Nu s-a putut restaura copia de rezervă: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Copie de rezervă restaurată: $path, dar reîncărcarea salvării a eșuat: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Verificarea codecului a eșuat: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Roundtrip-ul codecului a eșuat: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Căutarea proprietăților a eșuat: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Selecția salvării s-a schimbat în timp ce se încărcau atributele eroului.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Încărcarea abilităților a eșuat: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Interogarea progresului a eșuat: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Lista NPC a eșuat: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Lista de personaje a eșuat: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Atributele NPC au eșuat: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Încărcarea poziției NPC a eșuat: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Inventarul NPC a eșuat: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Lista de facțiuni a eșuat: $details';
  }

  @override
  String get editorNoBackupPath => 'niciuna';

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
    return '$prefix: $backupPath; copie PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Starea localizării a eșuat: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Extragerea a eșuat: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Încărcarea glosarului a eșuat: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Eroare copie de rezervă: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Misiune',
      'document': 'Document',
      'story': 'Poveste',
      'exploration': 'Explorare',
      'combat': 'Luptă',
      'social': 'Social',
      'item': 'Obiecte',
      'learning': 'Învățare',
      'guild': 'Gildă',
      'crime': 'Infracțiune',
      'rest': 'Odihnă',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Misiune începută',
      'questSucceeded': 'Misiune finalizată',
      'questFailed': 'Misiune eșuată',
      'documentRead': 'Document citit',
      'documentSegmentUnlocked': 'Intrare descoperită',
      'documentSegmentViewed': 'Intrare vizualizată',
      'chapterCompleted': 'Capitol finalizat',
      'areaEntered': 'Zonă intrată',
      'areaLeft': 'Zonă părăsită',
      'characterKilled': 'Personaj ucis',
      'characterDefeated': 'Personaj învins',
      'combatDodge': 'Atac eschivat',
      'characterDebuffed': 'Debuff aplicat',
      'tradeAvailable': 'Negustorie deblocată',
      'itemObtained': 'Obiect obținut',
      'itemCrafted': 'Obiect creat',
      'skillStateRecorded': 'Stare abilitate înregistrată',
      'recipeLearned': 'Rețetă învățată',
      'guildJoined': 'Gildă alăturată',
      'crimeRecorded': 'Infracțiune înregistrată',
      'slept': 'A dormit',
      'storyEvent': 'Eveniment de poveste',
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
      'gameTime': 'Timp de joc',
      'duration': 'Durată',
      'chapter': 'Capitol',
      'instigator': 'Inițiat de',
      'affected': 'Afectat',
      'amount': 'Cantitate',
      'primaryObject': 'Obiect',
      'secondaryObject': 'Context',
      'segmentText': 'Text intrare',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Ziua $day, $time';
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
  String get memoryEventHero => 'Erou';

  @override
  String get memoryEventDetails => 'Detalii';

  @override
  String get memoryEventTags => 'Etichete';

  @override
  String get memoryEventTechnicalData => 'Date tehnice';

  @override
  String get memoryEventIndex => 'Index';

  @override
  String get memoryEventPosition => 'Poziție';

  @override
  String get memoryEventPayload => 'Date';

  @override
  String get memoryEventSubject => 'Subiect';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Acces',
      'AccessDenied': 'Acces refuzat',
      'AccesToTemple': 'Acces la templu',
      'Advice': 'Sfat',
      'AfterFight': 'După luptă',
      'AfterFireMages': 'După magii focului',
      'AfterNek': 'După Nek',
      'AfterQuest': 'După misiune',
      'Alone': 'Singur',
      'Amulet': 'Amuletă',
      'Annoying': 'Enervant',
      'Armor': 'Armură',
      'Avoid': 'Evită',
      'Backstory': 'Poveste de fundal',
      'BackStory': 'Poveste de fundal',
      'BasicMagic': 'Magie de bază',
      'Beated': 'Bătut',
      'BecomeMercenary': 'Devino mercenar',
      'Beer': 'Bere',
      'Bestiary': 'Bestiarium',
      'Blessing': 'Binecuvântare',
      'Boss': 'Șef',
      'Bully': 'Bătăuș',
      'BullyAdvice': 'Sfat despre bătăuș',
      'Camp': 'Lagăr',
      'CampDivided': 'Lagăr împărțit',
      'CareOfMessengers': 'Grijă de mesageri',
      'ChangeOpinion': 'Schimbare de opinie',
      'ChargeUriziel': 'Încarcă Uriziel',
      'Chosen': 'Ales',
      'Contact': 'Contact',
      'Courier': 'Curier',
      'CraftBows': 'Confecționează arcuri',
      'Crazy': 'Nebun',
      'DailyMeal': 'Masă zilnică',
      'DailyRation_Trader': 'Negustor de rații zilnice',
      'DAM': 'Baraj',
      'Dead': 'Mort',
      'Deal': 'Înțelegere',
      'Dealer': 'Dealer',
      'Deceived': 'Înșelat',
      'Dementia': 'Demență',
      'DenyAccess': 'Refuză accesul',
      'DifferentOpinion': 'Opinie diferită',
      'Discussion': 'Discuție',
      'DontTalk': 'Nu vorbi',
      'Duel': 'Duel',
      'Entrance': 'Intrare',
      'Escape': 'Evadare',
      'Extended': 'Extins',
      'Extra': 'Extra',
      'ExtraInfo': 'Informații extra',
      'Fanatic': 'Fanatic',
      'Fight': 'Luptă',
      'FindUlumulu': 'Găsește Ulu-Mulu',
      'FireMages': 'Magii focului',
      'FireMagesEscape': 'Evadarea magilor focului',
      'FiskNewDealer': 'Nou samsar pentru Fisk',
      'FiskNewDealerCompleted': 'Nou samsar pentru Fisk — finalizat',
      'FogTower': 'Turnul de Ceață',
      'Food': 'Mâncare',
      'Forgave': 'A iertat',
      'Forgive': 'Iartă',
      'Forgiven': 'Iertat',
      'FourFriends': 'Patru prieteni',
      'FreeHut': 'Colibă liberă',
      'FreeMine': 'Mina Liberă',
      'Fury': 'Furor',
      'GoodTeacher': 'Bun învățător',
      'Gossip': 'Bârfe',
      'GotScavenger': 'Scavenger obținut',
      'GrantedAccess': 'Acces acordat',
      'GRDArmor': 'Armură de gardă',
      'Guide': 'Ghid',
      'HateMages': 'Ură față de magi',
      'HateMagesExplanation': 'Explicația urii față de magi',
      'HateRiceLord': 'Ură față de Domnul Orezului',
      'Heal': 'Vindecă',
      'Healing': 'Vindecare',
      'Help': 'Ajutor',
      'Helper': 'Ajutor',
      'HelpKagan': 'Ajută-l pe Kagan',
      'HutStory': 'Povestea colibei',
      'Ignore': 'Ignoră',
      'Impress': 'Impresionează',
      'ImpressAlchemy': 'Impresionează cu alchimia',
      'ImpressInscription': 'Impresionează cu inscripțiile',
      'Info': 'Informații',
      'Interested': 'Interesat',
      'Introduction': 'Introducere',
      'Introduction_2': 'Introducere 2',
      'Introduction_Armor': 'Introducere – Armură',
      'Introduction_Teacher': 'Introducere – Învățător',
      'Introduction_Trader': 'Introducere – Negustor',
      'Invocation': 'Invocație',
      'JoinSC': 'Alătură-te Lagărului din Mlaștină',
      'Joint': 'Joint',
      'KalomCamp': 'Lagărul lui Kalom',
      'Leader': 'Lider',
      'Learning': 'Învățare',
      'LearnOrcish': 'Învață orcește',
      'LeftParty': 'A părăsit grupul',
      'Library': 'Bibliotecă',
      'Lie': 'Minciună',
      'Lock': 'Lacăt',
      'Lockpick': 'Șperaclu',
      'Mad': 'Nebun',
      'Mandibles': 'Mandibule de minecrawler',
      'MapMaker': 'Cartograf',
      'Monastery': 'Mănăstire',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Lagărul Nou',
      'NewCamper': 'Nou în lagăr',
      'NewLeader': 'Lider nou',
      'NightPatrol': 'Patrulă de noapte',
      'NotInterested': 'Neinteresat',
      'OldCamp': 'Lagărul Vechi',
      'OrcEnclaveEntrance': 'Intrarea în enclavea orcilor',
      'OrcGraveyard': 'Cimitirul orcilor',
      'OreArmor': 'Armură de minereu',
      'Party': 'Grup',
      'Pay': 'Plătește',
      'PayMoney': 'Plătește bani',
      'Permission': 'Permisiune',
      'Pet': 'Animal de companie',
      'PreparingInvocation': 'Pregătirea invocației',
      'Quest': 'Misiune',
      'RankUpFireMages': 'Promovare mag al focului',
      'RankUpGuard': 'Promovare gardă',
      'RanUpFireMagesCompleted': 'Promovare mag al focului finalizată',
      'Realocated': 'Mutat',
      'Reason': 'Motiv',
      'Respect': 'Respect',
      'ReturnToSC': 'Întoarcere la Lagărul din Mlaștină',
      'RicelordForeman': 'Maistrul Domnului Orezului',
      'RideScavenger': 'Călărește scavenger',
      'Robe': 'Robă',
      'Safe': 'În siguranță',
      'Scraper': 'Miner',
      'SecondChance': 'A doua șansă',
      'SecretLocation': 'Locație secretă',
      'SecretPassage': 'Pasaj secret',
      'SecretPath': 'Cale secretă',
      'SleeperFollower': 'Adept al Adormitului',
      'SleeperTemple': 'Templul Adormitului',
      'SmallInfo': 'Informație scurtă',
      'Stonehenge': 'Stonehenge',
      'StopFollowing': 'Oprește urmărirea',
      'SwampCamp': 'Lagărul din Mlaștină',
      'Talkative': 'Vorbăreț',
      'Teach': 'Predă',
      'TeachBow': 'Predă arcul',
      'Teacher': 'Învățător',
      'Teacher2': 'Învățător 2',
      'TeacherInscription': 'Învățător de inscripții',
      'TeacherMana': 'Învățător de mana',
      'TeachIchor': 'Predă extragerea ichorului de minecrawler',
      'TeachMagic': 'Predă magia',
      'TeachOrcish': 'Predă orcește',
      'TeachStats': 'Predă atributele',
      'TeachWeapon': 'Predă armele',
      'Teleport': 'Teleportare',
      'TheMysteriousOrc': 'Orcul misterios',
      'ThroneRoom': 'Sala tronului',
      'TradeBow': 'Negustorie de arcuri',
      'Trader': 'Negustor',
      'TradeSkins_Trader': 'Negustor de piei',
      'Traitor': 'Trădător',
      'Trial': 'Probă',
      'TrollCanyon': 'Canionul trollilor',
      'Trust': 'Încredere',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Neexperimentat',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Rună Uriziel',
      'Useful': 'Util',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrații',
      'WaitFreeMine': 'Așteaptă la Mina Liberă',
      'WaitInTrainingArea': 'Așteaptă în zona de antrenament',
      'Warning': 'Avertisment',
      'WarningTooLate': 'Avertismentul a venit prea târziu',
      'WaterMessenger': 'Mesager pentru magii apei',
      'Weapon': 'Armă',
      'Who': 'Cine',
      'Women': 'Femei',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Sloturi de inventar deteriorate';

  @override
  String slotRepairBody(int count) {
    return 'Această salvare ține $count sloturi de inventar al căror id nu mai corespunde poziției — în joc, dacă arunci un astfel de obiect, se elimină altul. Reparația rescrie doar id-urile: niciun obiect nu e adăugat, eliminat sau schimbat. La salvare se creează o copie de rezervă, ca întotdeauna.';
  }

  @override
  String get slotRepairQueued =>
      'Reparație în coadă — salvează ca să o aplici.';

  @override
  String get slotRepairAction => 'Repară';

  @override
  String get slotRepairDiscard => 'Renunță';

  @override
  String get editorInventorySlotEditConflict =>
      'O editare directă a unui slot de inventar e în coadă împreună cu o modificare care pretinde sloturi întregi (reparare, adăugare sau eliminare). A doua ar suprascrie-o pe prima — anulează una dintre ele, apoi salvează din nou.';

  @override
  String get editorTraderArrayConflict =>
      'O modificare de negoț e în coadă împreună cu o editare directă a array-ului de negustori. Acea editare renumerotează rândurile pe care le adresează modificarea de negoț, deci una din două ar ateriza pe negustorul greșit — anulează una dintre ele, apoi salvează din nou.';

  @override
  String get backupFactFile => 'Fișier';

  @override
  String get renameBackupTooltip => 'Denumește această copie de rezervă';

  @override
  String get renameBackupTitle => 'Denumește copia de rezervă';

  @override
  String get renameBackupLabel => 'Nume';

  @override
  String renameBackupHelp(String fileName) {
    return 'Afișat în locul numelui de fișier $fileName. Lasă gol ca să elimini numele; fișierul în sine nu e redenumit.';
  }

  @override
  String get deleteBackupTooltip => 'Șterge această copie de rezervă';

  @override
  String get deleteBackupTitle => 'Șterge copia de rezervă';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Ștergi „$name” ($fileName)? Fișierul e eliminat de pe disc și nu poate fi adus înapoi.';
  }

  @override
  String get deleteBackupConfirm => 'Șterge';

  @override
  String editorDeletedBackup(String path) {
    return 'Copie de rezervă ștearsă: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Nu s-a putut șterge copia de rezervă: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Nu s-a putut denumi copia de rezervă: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Repararea nu e posibilă acum — această salvare nu poate fi scrisă.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Copie de rezervă ștearsă: $path — numele ei nu a putut fi eliminat: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'Reparația nu e disponibilă pentru această salvare.';

  @override
  String get statisticsTitle => 'Statistici';

  @override
  String get statisticsSubtitle =>
      'Un rezumat compact al progresului personajului, misiunilor, lumii și salvării.';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': 'Timp',
      'character': 'Personaj',
      'quests': 'Misiuni',
      'progress': 'Progres',
      'encounters': 'Luptă și contacte',
      'inventory': 'Abilități și inventar',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': 'Jucat',
      'worldTime': 'Timpul lumii',
      'level': 'Nivel',
      'experience': 'Experiență',
      'learningPoints': 'Puncte de învățare',
      'guild': 'Gildă',
      'health': 'Viață',
      'mana': 'Mana',
      'chapter': 'Capitol',
      'location': 'Locație',
      'kills': 'Ucideri NPC',
      'knownCharacters': 'Personaje cunoscute',
      'killedMonsters': 'Monștri uciși',
      'defeatedNpcs': 'NPC-uri învinse',
      'killedNpcs': 'NPC-uri ucise',
      'knownNpcs': 'NPC-uri cunoscute',
      'knownTeachers': 'Învățători cunoscuți',
      'learnedSkills': 'Abilități învățate',
      'knowledge': 'Intrări de cunoștințe',
      'deadCharacters': 'Personaje moarte',
      'traders': 'Negustori cunoscuți',
      'inventoryStacks': 'Stive de obiecte',
      'inventoryItems': 'Obiecte',
      'ore': 'Minereu',
      'equipped': 'Echipat',
      'hostileFactions': 'Facțiuni ostile',
      'openCrimes': 'Infracțiuni deschise',
      'position': 'Poziție',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': 'Lagărul Vechi · Umbra',
      'oldCampGuard': 'Lagărul Vechi · Gardă',
      'oldCampFireMage': 'Lagărul Vechi · Mag al focului',
      'newCampRogue': 'Lagărul Nou · Bandit',
      'newCampMercenary': 'Lagărul Nou · Mercenar',
      'newCampWaterMage': 'Lagărul Nou · Mag al apei',
      'swampCampNovice': 'Lagărul din Mlaștină · Novice',
      'swampCampTemplar': 'Lagărul din Mlaștină · Templier',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => 'Indisponibil';

  @override
  String get statisticsMore => 'Mai multe statistici';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return 'Nivel $level, $guild, capitolul $chapter. $completed misiuni finalizate, $failed eșuate. Timp de joc: $playTime.';
  }

  @override
  String get locksSidebar => 'Lacăte';

  @override
  String editorLockListFailed(String details) {
    return 'Lista de lăcate a eșuat: $details';
  }

  @override
  String get locksSearchHint => 'Caută lacăt sau cheie';

  @override
  String get locksAllRegions => 'Toate regiunile';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$shown din $total';
  }

  @override
  String get locksFilterChests => 'Cufere';

  @override
  String get locksFilterDoors => 'Uși';

  @override
  String get locksFilterUnlocked => 'Descuiate';

  @override
  String get locksFilterLocked => 'Încuiate';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return 'Dificultate $bars din 4 (treaptă internă $level din 7)';
  }

  @override
  String get locksKeyOnly => 'Doar cheie';

  @override
  String locksKeyLabel(String keys) {
    return 'Cheie: $keys';
  }

  @override
  String get locksPermalocked => 'Sigilat permanent';

  @override
  String get locksReadOnly => 'Această salvare nu are set de lăcate editabil.';

  @override
  String get locksUnknownEntry => 'Nu e în această versiune a jocului';

  @override
  String get locksDoorLeafHint =>
      'Încuierea din nou a unei uși o închide și pe ușă.';

  @override
  String get locksResetPending => 'Renunță la modificările de lăcate din coadă';
}
