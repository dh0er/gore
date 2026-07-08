// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor logo';

  @override
  String get zoomTooltip => 'Premi Ctrl +/- per ingrandire/rimpicciolire';

  @override
  String get switchToLightMode => 'Passa alla modalità chiara';

  @override
  String get switchToDarkMode => 'Passa alla modalità scura';

  @override
  String get about => 'Informazioni';

  @override
  String get tabOverview => 'Panoramica';

  @override
  String get tabPlayer => 'Giocatore';

  @override
  String get tabAttribute => 'Attributi';

  @override
  String get heroGroupSkills => 'Abilità';

  @override
  String get skillsNoneBody =>
      'Nessuna abilità trovata per questo personaggio.';

  @override
  String get skillsUnavailableBody =>
      'Le abilità non possono essere modificate in questo salvataggio: l\'eroe non ha dati sugli effetti da modificare.';

  @override
  String get skillNotLearned => 'Non appresa';

  @override
  String get skillLearn => 'Apprendi';

  @override
  String get skillActionLearn => 'apprendi';

  @override
  String get skillActionUnlearn => 'dimentica';

  @override
  String get skillTierUntrained => 'Inesperto';

  @override
  String get skillTierBeginner => 'Principiante';

  @override
  String get skillTierTrained => 'Esperto';

  @override
  String get skillTierMaster => 'Maestro';

  @override
  String get skillTierNovice => 'Novizio';

  @override
  String get skillTierAmateur => 'Dilettante (Cerchio 0)';

  @override
  String get skillTierLearned => 'Appresa';

  @override
  String skillTierCircle(int n) {
    return 'Cerchio $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Armi a una mano';

  @override
  String get skillHintBlacksmith2H => 'Armi a due mani';

  @override
  String get skillCategoryCombat => 'Combattimento';

  @override
  String get skillCategoryCrafting => 'Artigianato';

  @override
  String get skillCategoryHunting => 'Caccia';

  @override
  String get skillCategoryLanguage => 'Lingua';

  @override
  String get skillCategoryMagic => 'Magia';

  @override
  String get skillCategoryMovement => 'Movimento';

  @override
  String get skillCategoryThievery => 'Furto';

  @override
  String get skillNameOneHanded => 'A una mano';

  @override
  String get skillNameTwoHanded => 'A due mani';

  @override
  String get skillNameFists => 'Pugni';

  @override
  String get skillNameBow => 'Arco';

  @override
  String get skillNameCrossbow => 'Balestra';

  @override
  String get skillNameLockpicking => 'Scassinamento';

  @override
  String get skillNamePickpocketing => 'Borseggio';

  @override
  String get skillNameTakeOrgans => 'Estrai organi';

  @override
  String get skillNameBreakTeeth => 'Estrai denti';

  @override
  String get skillNameTakeClaws => 'Estrai artigli';

  @override
  String get skillNameSkinFur => 'Prendi pelliccia';

  @override
  String get skillNameSkin => 'Scuoia';

  @override
  String get skillNameTakeFins => 'Prendi pinne';

  @override
  String get skillNameTakeStingers => 'Estrai pungiglioni';

  @override
  String get skillNameTakeSecretion => 'Estrai secrezioni';

  @override
  String get skillNameTakeSkullPlates => 'Prendi corazza craniale';

  @override
  String get skillNameSkinSwampshark => 'Scuoia squalo';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Prendi piastre';

  @override
  String get skillNameTakeScutes => 'Prendi placche';

  @override
  String get skillNameTakeUluMulu => 'Prendi Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Armi orchesche';

  @override
  String get skillNameMining => 'Estrazione del metallo';

  @override
  String get skillNameDiving => 'Immersione';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Estrai mandibole';

  @override
  String get skillNameTakeShadowbeastHorn => 'Prendi corno (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Estrai spine dorsali';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Estrai denti di squalo';

  @override
  String get skillNameTakeFireTongue => 'Prendi lingua di fuoco';

  @override
  String get skillNameTakeTrollHorn => 'Prendi corno (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobazie';

  @override
  String get skillNameWallClimbing => 'Arrampicata';

  @override
  String get skillNameRiding => 'Cavalcata di saprofagi';

  @override
  String get skillNameSneaking => 'Furtività';

  @override
  String get skillNameAlchemy => 'Alchimia';

  @override
  String get skillNameRuneInscription => 'Iscrizione';

  @override
  String get skillNameBlacksmithing => 'Forgiatura';

  @override
  String get skillNameMagicCircle => 'Cerchio magico';

  @override
  String get skillNameOrcish => 'Orchese';

  @override
  String get tabInventory => 'Inventario';

  @override
  String get tabWorld => 'Mondo';

  @override
  String get tabCharacters => 'Personaggi';

  @override
  String get characterNoActorBody =>
      'Questo personaggio non ha un attore nel mondo, quindi non ha attributi, inventario o eventi.';

  @override
  String get characterNoEventsBody => 'Nessun evento per questo personaggio.';

  @override
  String get characterOrphanGroup => 'Altri';

  @override
  String get tabAllData => 'Tutti i dati';

  @override
  String get tabBackups => 'Backup';

  @override
  String get tabSettings => 'Impostazioni';

  @override
  String get reset => 'Reimposta';

  @override
  String get save => 'Salva';

  @override
  String saveWithCount(int count) {
    return 'Salva ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Annulla';

  @override
  String get confirm => 'Conferma';

  @override
  String get close => 'Chiudi';

  @override
  String get add => 'Aggiungi';

  @override
  String get equippedBadge => 'Equipaggiato';

  @override
  String get armorUpgradesLabel => 'Potenziamenti';

  @override
  String get browse => 'Sfoglia';

  @override
  String get noSavFilesFound => 'Nessun file .sav trovato';

  @override
  String get profile => 'Profilo';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count salvataggi)';
  }

  @override
  String get switchProfile => 'Cambia profilo';

  @override
  String get rescanSaveFolder => 'Riscansiona la cartella dei salvataggi';

  @override
  String get discardUnsavedChangesTitle =>
      'Annullare le modifiche non salvate?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'le tue $count modifiche non salvate',
      one: 'la tua $count modifica non salvata',
    );
    return 'La riscansione ricarica ogni salvataggio e annulla $_temp0.';
  }

  @override
  String get discardAndRescan => 'Annulla e riscansiona';

  @override
  String chapterLabel(Object id) {
    return 'Capitolo $id';
  }

  @override
  String get quickSave => 'Salvataggio rapido';

  @override
  String get autoSave => 'Salvataggio automatico';

  @override
  String get manualSave => 'Salvataggio manuale';

  @override
  String get errorTitle => 'Errore';

  @override
  String get selectASaveTitle => 'Seleziona un salvataggio';

  @override
  String get selectASaveBody => 'I dettagli del salvataggio appariranno qui.';

  @override
  String get diagnosticsTitle => 'Diagnostica e dettagli';

  @override
  String get diagnosticsSubtitle => 'Ispezione del formato in sola lettura';

  @override
  String get metricFormat => 'Formato';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Capitolo';

  @override
  String get metricTimePlayed => 'Tempo di gioco';

  @override
  String get metricSaveKind => 'Tipo di salvataggio';

  @override
  String get metricFileSize => 'Dimensione file';

  @override
  String get metricCompression => 'Compressione';

  @override
  String get metricChunks => 'Blocchi';

  @override
  String get metricUncompressed => 'Non compresso';

  @override
  String get metricPrivate => 'Privato';

  @override
  String get metricSlotName => 'Nome slot';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Privato decodificato';

  @override
  String get metricPrivateStrings => 'Stringhe private';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count byte';
  }

  @override
  String get inspectionJsonTitle => 'JSON di ispezione';

  @override
  String get inspectionJsonSubtitle =>
      'Dati grezzi di ispezione del salvataggio';

  @override
  String get copy => 'Copia';

  @override
  String get savegameFallbackTitle => 'Salvataggio';

  @override
  String screenshotForSlot(String slot) {
    return 'Screenshot per $slot';
  }

  @override
  String get publicSaveName => 'Nome pubblico del salvataggio';

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
  String get required => 'Obbligatorio';

  @override
  String get playerLockedBody =>
      'Le modifiche private del giocatore richiedono un codec in grado di comprimere.';

  @override
  String get heroTransform => 'Posizione';

  @override
  String get locationX => 'Posizione X';

  @override
  String get locationY => 'Posizione Y';

  @override
  String get locationZ => 'Posizione Z';

  @override
  String get rotationPitch => 'Beccheggio (pitch)';

  @override
  String get rotationYaw => 'Imbardata (yaw)';

  @override
  String get rotationRoll => 'Rollio (roll)';

  @override
  String get invalid => 'Non valido';

  @override
  String get heroAttributes => 'Attributi dell\'eroe';

  @override
  String attributeBase(String name) {
    return '$name base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name attuale';
  }

  @override
  String get inventoryTitle => 'Inventario';

  @override
  String get inventoryEmpty => 'Questo inventario è vuoto.';

  @override
  String get inventoryNeedsDecoded =>
      'La modifica dell\'inventario richiede dati privati decodificati dal codec.';

  @override
  String get inventoryNoStacks =>
      'Nessuna pila di oggetti trovata nei dati privati decodificati.';

  @override
  String get resetInventoryChanges => 'Reimposta le modifiche all\'inventario';

  @override
  String get addItemTooltipPendingAdd =>
      'Salva prima le modifiche in sospeso — un nuovo oggetto per salvataggio';

  @override
  String get addItemTooltipPendingRemove =>
      'Salva prima la rimozione in sospeso — una modifica strutturale per salvataggio';

  @override
  String get addItemTooltipPendingCount =>
      'Salva o reimposta prima le modifiche alla quantità in sospeso — una modifica strutturale deve essere salvata da sola';

  @override
  String get addItemTooltipDefault => 'Aggiungi un oggetto all\'inventario';

  @override
  String get addItemButton => 'Aggiungi oggetto';

  @override
  String get resetInventoryButton => 'Reset inventory';

  @override
  String get resetInventoryTooltipDefault =>
      'Replace this inventory with the game-start save\'s inventory';

  @override
  String get resetInventoryTooltipBlocked =>
      'Save or cancel the pending inventory changes first';

  @override
  String get pendingResetTitle => 'Reset to game-start inventory';

  @override
  String pendingResetSubtitle(String level) {
    return 'Resources level: $level';
  }

  @override
  String get cancelPendingReset => 'Cancel reset';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — aggiunta in sospeso (non ancora salvata)';
  }

  @override
  String get cancelPendingAdd => 'Annulla l\'aggiunta in sospeso';

  @override
  String get pendingRemovalSubtitle =>
      'rimozione in sospeso (non ancora salvata)';

  @override
  String get cancelPendingRemoval => 'Annulla la rimozione in sospeso';

  @override
  String get filterItems => 'Filtra oggetti';

  @override
  String noItemsMatchQuery(String query) {
    return 'Nessun oggetto corrisponde a «$query».';
  }

  @override
  String get pendingRemovalHidesAll =>
      'La rimozione in sospeso nasconde ogni oggetto — salva per applicarla.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Armi da mischia';

  @override
  String get itemCategoryRangedWeapon => 'Armi a distanza';

  @override
  String get itemCategoryAmmunition => 'Munizioni';

  @override
  String get itemCategoryArmor => 'Armature';

  @override
  String get itemCategoryRune => 'Rune';

  @override
  String get itemCategoryScroll => 'Pergamene magiche';

  @override
  String get itemCategoryFood => 'Cibo e pozioni';

  @override
  String get itemCategoryMisc => 'Varie';

  @override
  String get itemCategoryAmulet => 'Amuleti';

  @override
  String get itemCategoryRing => 'Anelli';

  @override
  String get itemCategoryTrophy => 'Trofei animali';

  @override
  String get itemCategoryWriting => 'Scritti';

  @override
  String get itemCategoryMission => 'Oggetti della missione';

  @override
  String get itemCategoryKey => 'Chiavi';

  @override
  String get itemCategoryOther => 'Altro';

  @override
  String get count => 'Quantità';

  @override
  String get min1 => 'Min 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Impossibile eliminare: questo oggetto è probabilmente equipaggiato o assegnato a uno slot rapido';

  @override
  String get removeBlockedTooltip =>
      'Salva o reimposta prima le modifiche all\'inventario in sospeso — un\'aggiunta o una rimozione deve essere salvata da sola';

  @override
  String get removeItemFromInventory => 'Rimuovi l\'oggetto dall\'inventario';

  @override
  String get progressionLockedBody =>
      'I dati di progressione richiedono dati privati decodificati dal codec.';

  @override
  String get progressionNeedsTyped =>
      'I dati di progressione strutturati richiedono un salvataggio completamente decodificato con un\'analisi tipizzata verificata.';

  @override
  String get sectionQuests => 'Missioni';

  @override
  String get sectionKnowledge => 'Conoscenze';

  @override
  String get sectionEvents => 'Eventi';

  @override
  String get firstPage => 'Prima pagina';

  @override
  String get previousPage => 'Pagina precedente';

  @override
  String get nextPage => 'Pagina successiva';

  @override
  String get lastPage => 'Ultima pagina';

  @override
  String pageOfPages(int page, int total) {
    return 'Pagina $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last di $total';
  }

  @override
  String get perPage => 'Per pagina:';

  @override
  String get resetQuestChanges => 'Reimposta le modifiche alle missioni';

  @override
  String get searchQuests => 'Cerca missioni';

  @override
  String get allGroups => 'Tutti i gruppi';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Nessuno';

  @override
  String get questStateAvailable => 'Disponibile';

  @override
  String get questStateRunning => 'In corso';

  @override
  String get questStateSucceeded => 'Completata';

  @override
  String get questStateFailed => 'Fallita';

  @override
  String get questStateUnknown => 'sconosciuto';

  @override
  String get dialogKnowledge => 'Conoscenze dei dialoghi';

  @override
  String get resetKnowledgeChanges => 'Reimposta le modifiche alle conoscenze';

  @override
  String get addNpc => 'Aggiungi NPC';

  @override
  String get searchNpcs => 'Cerca NPC';

  @override
  String get npcStatusRowLabel => 'Stato';

  @override
  String get npcStatusAlive => 'vivo';

  @override
  String get npcStatusDead => 'morto';

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PS $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Rianima';

  @override
  String get npcReviveQueued => 'Verrà rianimato al salvataggio';

  @override
  String entriesForCharacter(String name) {
    return 'Voci — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Seleziona un NPC per vedere le voci';

  @override
  String get addKnowledgeEntry => 'Aggiungi voce di conoscenza';

  @override
  String get browseCatalog => 'Sfoglia il catalogo';

  @override
  String get alreadyExistsForCharacter => 'Esiste già per questo personaggio.';

  @override
  String get alreadyInPendingChanges =>
      'Già presente nelle modifiche in sospeso.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Controllo duplicati non riuscito — riprova: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Aggiunte in sospeso ($count)';
  }

  @override
  String get undoAdd => 'Annulla aggiunta';

  @override
  String get undoRemove => 'Annulla rimozione';

  @override
  String get removeEntry => 'Rimuovi voce';

  @override
  String get selectNpcFromList => 'Seleziona un NPC dall\'elenco';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Eventi della memoria';

  @override
  String get searchCharacters => 'Cerca personaggi';

  @override
  String eventsForCharacter(String name) {
    return 'Eventi — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Seleziona un personaggio per vedere gli eventi';

  @override
  String get noTags => '(nessun tag)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Rimuovi evento';

  @override
  String get removeMemoryEventTitle => 'Rimuovere l\'evento della memoria?';

  @override
  String get removeMemoryEventBody =>
      'Rimuovere questo evento della memoria? Viene creato prima un backup.';

  @override
  String get duplicateEvent => 'Duplica evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicare l\'evento della memoria?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicare questo evento della memoria? Viene creato prima un backup.';

  @override
  String get selectCharacterFromList => 'Seleziona un personaggio dall\'elenco';

  @override
  String get factionsSidebar => 'Fazioni';

  @override
  String get factionsForgiveButton => 'Perdona';

  @override
  String get factionHostile => 'Ostile';

  @override
  String get factionFriendly => 'Amichevole';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count omicidi',
      one: '$count omicidio',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count aggressioni',
      one: '$count aggressione',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count furti',
      one: '$count furto',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count violazioni',
      one: '$count violazione',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count minacce',
      one: '$count minaccia',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count altri crimini',
      one: '$count altro crimine',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'perdono in corso…';

  @override
  String get factionsEmpty => 'Nessun crimine aperto contro le fazioni.';

  @override
  String get factionGuildOldCamp => 'Vecchio Campo';

  @override
  String get factionGuildNewCamp => 'Nuovo Campo';

  @override
  String get factionGuildSwampCamp => 'Campo della Palude';

  @override
  String get factionGuildOther => 'Altri/individui';

  @override
  String get allDataLockedBody =>
      'Il browser completo delle proprietà richiede dati privati decodificati dal codec.';

  @override
  String get allDataDescription =>
      'Cerca ogni proprietà tipizzata per nome o percorso. Scalari, stringhe, enum e percorsi di oggetti sono modificabili; gli struct sono mostrati in sola lettura per ora.';

  @override
  String get searchPropertiesLabel =>
      'Cerca proprietà (vuoto = elenca tutto) — es. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Decodifica del salvataggio…';

  @override
  String get decodingSaveBody =>
      'Decodifica dell\'intero payload privato per la prima ricerca. Questa operazione viene eseguita una volta per salvataggio, poi le ricerche sono istantanee.';

  @override
  String get searchTheSaveTitle => 'Cerca nel salvataggio';

  @override
  String get searchTheSaveBody =>
      'Digita il nome di una proprietà e premi Invio. Lascia vuoto per elencare tutto.';

  @override
  String get searchFailedTitle => 'Ricerca non riuscita';

  @override
  String get noMatchesTitle => 'Nessun risultato';

  @override
  String get noMatchesBody =>
      'Nessun percorso di proprietà conteneva tutti questi termini.';

  @override
  String get value => 'Valore';

  @override
  String get backupsTitle => 'Backup';

  @override
  String get refreshBackups => 'Aggiorna i backup';

  @override
  String get noBackupsTitle => 'Nessun backup';

  @override
  String get noBackupsBody =>
      'I salvataggi modificati creano file di backup accanto allo slot selezionato.';

  @override
  String get slotBackups => 'Backup dello slot';

  @override
  String get profileBackups => 'Backup del profilo';

  @override
  String get backupFactName => 'Nome';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Creato';

  @override
  String get backupFactSize => 'Dimensione';

  @override
  String get backupFactStatus => 'Stato';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Ripristina $fileName';
  }

  @override
  String get appearanceTitle => 'Aspetto';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Chiaro';

  @override
  String get themeDark => 'Scuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Scala dell\'interfaccia';

  @override
  String get resetZoomTooltip => 'Reimposta lo zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Suggerimento: Ctrl + / Ctrl - cambia lo zoom ovunque nell\'app.';

  @override
  String get language => 'Lingua';

  @override
  String get updatesTitle => 'Aggiornamenti';

  @override
  String get checkForUpdatesAutomatically =>
      'Controlla automaticamente gli aggiornamenti';

  @override
  String get checkForUpdatesNow => 'Controlla ora gli aggiornamenti';

  @override
  String get updatesPortableNotice =>
      'La versione portatile apre la pagina di download nel browser. Sostituisci i file esistenti con il nuovo download.';

  @override
  String get updateAvailableTitle => 'Aggiornamento disponibile';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'La versione $version è disponibile. Hai la $current.';
  }

  @override
  String get updateDownload => 'Scarica';

  @override
  String get updateLater => 'Più tardi';

  @override
  String get updateUpToDate => 'Stai usando l\'ultima versione.';

  @override
  String get updateCheckFailed =>
      'Impossibile verificare gli aggiornamenti. Riprova più tardi.';

  @override
  String get gameTextTitle => 'Testo del gioco';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Estratti: $ids id su $languages lingue.';
  }

  @override
  String get gameTextExtracted =>
      'Il testo localizzato del gioco è stato estratto.';

  @override
  String get gameTextNotExtracted =>
      'Il testo localizzato del gioco non è ancora stato estratto.';

  @override
  String get extracting => 'Estrazione in corso…';

  @override
  String get extractRefreshLocalizedText =>
      'Estrai / aggiorna il testo localizzato';

  @override
  String get extractLocalizedTextTitle =>
      'Estrarre il testo localizzato del gioco?';

  @override
  String get extractLocalizedTextBody =>
      'Il testo localizzato del gioco non è ancora stato estratto. Estrarlo ora dalla tua installazione del gioco? (facoltativo)';

  @override
  String get notNow => 'Non ora';

  @override
  String get extract => 'Estrai';

  @override
  String get extractionComplete => 'Estrazione completata';

  @override
  String get extractionFailed => 'Estrazione non riuscita';

  @override
  String get localizationCacheFileType => 'Cache di localizzazione';

  @override
  String get savegameDirectoryTitle => 'Cartella dei salvataggi';

  @override
  String get folder => 'Cartella';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Verifica';

  @override
  String get roundtrip => 'Andata e ritorno';

  @override
  String get noCodecStatus => 'Nessuno stato del codec';

  @override
  String get codecReady => 'Codec pronto';

  @override
  String get codecReadOnly => 'Codec in sola lettura';

  @override
  String get codecUnavailable => 'Codec non disponibile';

  @override
  String get details => 'Dettagli';

  @override
  String codecStatusLine(String status) {
    return 'Stato: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Decompressione: $decompress | Compressione: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'sì';

  @override
  String get no => 'no';

  @override
  String aboutVersion(String version, String sha) {
    return 'Versione $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 collaboratori di goresave';

  @override
  String get aboutLicense => 'Concesso in licenza secondo la licenza MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Difficoltà — $profile';
  }

  @override
  String get difficultyNoProfile => 'Nessun profilo';

  @override
  String get difficultyNoDifficulty => 'Nessuna difficoltà';

  @override
  String get difficultyLabel => 'Difficoltà';

  @override
  String get difficultyTooltipNoProfile => 'Nessun profilo selezionato';

  @override
  String get difficultyTooltipEdit =>
      'Modifica la difficoltà per questo profilo';

  @override
  String get difficultyTooltipNoEditable =>
      'Questo profilo non ha una difficoltà modificabile';

  @override
  String get preset => 'Preimpostazione';

  @override
  String get presetNovice => 'Principiante';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difficile';

  @override
  String get presetCustom => 'Personalizzata';

  @override
  String unrecognisedPreset(Object preset) {
    return 'La preimpostazione salvata non è riconosciuta ($preset). Puoi comunque salvare le modifiche a Assistente al flusso di combattimento / Permadeath, oppure scegliere una preimpostazione qui sopra per sovrascriverla.';
  }

  @override
  String get closeCombatFlowHelper => 'Assistente al combattimento ravvicinato';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Non disponibile in modalità Principiante';

  @override
  String get levelCombat => 'Combattimento';

  @override
  String get levelResources => 'Risorse';

  @override
  String get levelProgression => 'Progressione';

  @override
  String get difficultyAppliesToAllSaves =>
      'La difficoltà si applica a tutti i salvataggi di questo profilo.';

  @override
  String get savingDifficultyFailed =>
      'Salvataggio della difficoltà non riuscito.';

  @override
  String get addItemDialogTitle => 'Aggiungi oggetto';

  @override
  String get searchItems => 'Cerca oggetti';

  @override
  String failedToLoadCatalog(String error) {
    return 'Impossibile caricare il catalogo: $error';
  }

  @override
  String get noItemsAvailableToAdd =>
      'Nessun oggetto disponibile da aggiungere';

  @override
  String get noItemsMatch => 'Nessun oggetto corrispondente';

  @override
  String get countMustBeAtLeast1 => 'Deve essere ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Deve essere ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Aggiungi NPC';

  @override
  String get noNpcsAvailableToAdd => 'Nessun NPC disponibile da aggiungere';

  @override
  String get noNpcsMatch => 'Nessun NPC corrispondente';

  @override
  String get categoryAll => 'Tutti';

  @override
  String allWithCount(int count) {
    return 'Tutti ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Aggiungi voce di conoscenza';

  @override
  String get searchEntries => 'Cerca voci';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nessuna voce di conoscenza disponibile da aggiungere';

  @override
  String get noEntriesMatch => 'Nessuna voce corrispondente';

  @override
  String get heroGroupMainStats => 'Statistiche principali';

  @override
  String get heroGroupCombatSkills => 'Abilità di combattimento';

  @override
  String get heroGroupResistances => 'Resistenze';

  @override
  String get heroGroupThieving => 'Furto';

  @override
  String get heroGroupAdvanced => 'Avanzate';

  @override
  String get heroEntryHeroTransform => 'Posizione';

  @override
  String attributeEmpty(String name) {
    return '$name è vuoto — inserisci un valore o ripristina quello originale prima di salvare.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Numero non valido per $name: «$text»';
  }

  @override
  String get loadingEditorData => 'Caricamento dei dati dell\'editor';

  @override
  String savingProgress(int done, int total) {
    return 'Saving… $done of $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID estratti in $languageCount lingue';
  }

  @override
  String get skillSmithing1H => 'Forgiatura di armi a una mano';

  @override
  String get skillSmithing2H => 'Forgiatura di armi a due mani';

  @override
  String get skillCircleNovice => 'Mago principiante';

  @override
  String get skillCircle1 => 'Primo Cerchio Magico';

  @override
  String get skillCircle2 => 'Secondo Cerchio Magico';

  @override
  String get skillCircle3 => 'Terzo Cerchio Magico';

  @override
  String get skillCircle4 => 'Quarto Cerchio Magico';

  @override
  String get skillCircle5 => 'Quinto Cerchio Magico';

  @override
  String get skillCircle6 => 'Sesto Cerchio Magico';
}
