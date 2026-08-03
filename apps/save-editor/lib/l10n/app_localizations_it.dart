// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get debugSectionTitle => 'Avanzate (debug)';

  @override
  String get debugSectionSubtitle =>
      'Diagnostica e dati grezzi per le segnalazioni di bug';

  @override
  String get showObjectIdsTitle => 'Mostra ID tecnici aggiuntivi';

  @override
  String get showObjectIdsSubtitle =>
      'Mostra gli ID tecnici di oggetti, conoscenze di dialogo, missioni e attori orfani. Gli ID dei PNG sono sempre visibili.';

  @override
  String get storyStateSidebar => 'Stato della storia';

  @override
  String get storyStateDescription =>
      'Catalogo autorevole degli stati persistenti dichiarati dagli script distribuiti con il gioco. Le voci salvate mostrano il valore grezzo; i campi del catalogo assenti dal salvataggio sono indicati come non impostati. I marcatori temporali dichiarati nel codice sono formattati come tempo di gioco; gli altri interi possono essere booleani, contatori o stati a più livelli.';

  @override
  String get storyStateReadOnly =>
      'Sola lettura finché non sono noti il significato dei valori negli script e una scrittura sicura della mappa. Il testo del glossario collegato offre contesto, non è una traduzione diretta dell’ID tecnico.';

  @override
  String get storyStateStructureReadOnly =>
      'Non è stato possibile individuare in modo univoco e sicuro la struttura StoryPropertyValues di questo salvataggio. I valori della storia rimangono di sola lettura per questo salvataggio.';

  @override
  String get storyStateSearch => 'Cerca nello stato della storia';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown di $total valori della storia';
  }

  @override
  String get storyStateInteger => 'Intero';

  @override
  String get storyStateTimeMarker => 'Marcatore temporale';

  @override
  String get storyStateChapter => 'Capitolo';

  @override
  String get storyStateUnknown => 'Tipo sorgente sconosciuto';

  @override
  String get storyStateUnknownDetail =>
      'Questo ID salvato non è presente nel catalogo degli script attuale (ad esempio per una mod o una versione più recente del gioco). Il valore serializzato è int32, ma il significato non viene dedotto.';

  @override
  String get storyStateStored => 'Salvato';

  @override
  String get storyStateUnset => 'Non impostato';

  @override
  String get storyStateUnsetDetail =>
      'Questo campo del catalogo non è serializzato nel salvataggio; il gioco usa quindi lo stato non impostato o predefinito.';

  @override
  String get storyStateRawValue => 'Valore grezzo';

  @override
  String storyStateElapsed(String duration) {
    return 'Tempo trascorso al salvataggio: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Nel futuro al salvataggio: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days giorni',
      one: '1 giorno',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Voce del glossario collegata';

  @override
  String get storyStateTechnicalPath => 'Percorso tecnico';

  @override
  String get storyStateEditingGuidance =>
      'Ogni voce resta modificabile nell’intero intervallo int32 con segno. Gli indicatori e i suggerimenti di valore ricavati dagli script sono solo indicativi; l’inserimento del valore grezzo è sempre disponibile. Le modifiche allo stato della storia possono saltare transizioni di dialoghi, missioni o del mondo, quindi salvale con cautela; viene creata automaticamente una copia di sicurezza.';

  @override
  String get storyStatePending => 'In sospeso';

  @override
  String storyStatePendingValue(String value) {
    return 'Verrà salvato come $value';
  }

  @override
  String get storyStatePendingRemoval => 'Verrà rimosso dal salvataggio';

  @override
  String get storyStateEditValue => 'Modifica valore';

  @override
  String get storyStateSetValue => 'Imposta valore';

  @override
  String get storyStateRemoveValue => 'Rimuovi dal salvataggio';

  @override
  String get storyStateUndoChange => 'Annulla modifica alla storia';

  @override
  String get storyStateResetChanges => 'Reimposta modifiche alla storia';

  @override
  String storyStateDialogTitle(String id) {
    return 'Modifica $id';
  }

  @override
  String get storyStateRawInput => 'Valore int32 con segno';

  @override
  String get storyStateInvalidInt32 =>
      'Inserisci un numero intero compreso tra -2147483648 e 2147483647.';

  @override
  String get storyStateQueueChange => 'Accoda modifica';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valori riscontrati negli script forniti: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'I suggerimenti non sono limiti di convalida; il codice nativo, le mod o versioni successive del gioco potrebbero usare altri valori.';

  @override
  String get storyStateUseCurrentTime => 'Usa l’ora attuale del salvataggio';

  @override
  String get storyStateStructuredTime => 'Giorno / ora';

  @override
  String get storyStateRawMode => 'int32 grezzo';

  @override
  String get storyStateChapterWarning =>
      'La modifica del solo capitolo non sincronizza missioni, PNG, inventario o stato del mondo.';

  @override
  String get storyStateDormantWarning =>
      'Nella cache degli script forniti non sono state trovate letture o scritture attive per questo campo. Potrebbe essere obsoleto, controllato dal codice nativo o riservato.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Gli script forniti leggono questo campo, ma non contengono alcuna scrittura tramite script. Il codice nativo potrebbe comunque gestirlo.';

  @override
  String get storyStateUnknownEditWarning =>
      'Questo ID proveniente da una mod o da una versione successiva non dispone di semantica del sorgente inclusa. Modifica solo il suo valore int32 grezzo.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Indicatore binario',
      'finiteState': 'Valore multistato',
      'counterOrScore': 'Contatore / punteggio',
      'calendarDay': 'Giorno di calendario',
      'derivedOrOpaqueInteger': 'Intero derivato / opaco',
      'readOnlyInSourceInteger': 'Sola lettura negli script forniti',
      'dormantOrLegacyInteger': 'Inutilizzato negli script forniti',
      'other': 'Intero',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Uno 0 salvato e una voce assente dalla mappa sono stati del file distinti. «Rimuovi dal salvataggio» ripristina lo stato del costruttore o quello predefinito.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo di GORE Save Editor';

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
  String get skillNameMagicCircle => 'Cerchio Magico';

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
  String get otherSaves => 'Altri salvataggi';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count salvataggi)';
  }

  @override
  String get switchProfile => 'Cambia profilo';

  @override
  String get openSaveFile => 'Apri file';

  @override
  String get externalSave => 'Salvataggio aperto esternamente';

  @override
  String get saveProfileTitle => 'Profilo del salvataggio';

  @override
  String get saveProfileDescription =>
      'Assegna questo salvataggio a un altro profilo di gioco. Il salvataggio e l’indice dei profili vengono sottoposti insieme a backup.';

  @override
  String get saveProfileExternalHint =>
      'Seleziona un profilo per importare questo file nella cartella dei salvataggi del gioco e registrarlo. Il file originale resta invariato.';

  @override
  String get saveProfileNoProfiles =>
      'Nessun profilo di gioco modificabile trovato in PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Seleziona profilo';

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
  String bytesValue(String count) {
    return '$count byte';
  }

  @override
  String get inspectionJsonTitle => 'JSON di ispezione';

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
  String get gameTimeTitle => 'Tempo di gioco';

  @override
  String get gameTimeDay => 'Giorno';

  @override
  String get gameTimeHours => 'Ore';

  @override
  String get gameTimeMinutes => 'Minuti';

  @override
  String get gameTimeSeconds => 'Secondi';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s totali';
  }

  @override
  String get gameTimeInvalid =>
      'Inserisci numeri interi: giorno ≥ 0, ore 0–23, minuti e secondi 0–59.';

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
  String get spawnPositionSection => 'Posizione di generazione (riferimento)';

  @override
  String get positionNotReadable =>
      'Non è stato possibile leggere la posizione salvata di questo personaggio.';

  @override
  String get npcPositionReadOnly =>
      'Il gioco ripristina la posizione di un PNG dal livello, non dal salvataggio: questi valori si possono leggere ma non modificare.';

  @override
  String get pickLocation => 'Scegli una posizione…';

  @override
  String get pickLocationDialogTitle => 'Scegli una posizione';

  @override
  String get applySpotRotation => 'Applica anche l’orientamento del punto';

  @override
  String get locationAreaOther => 'Altro';

  @override
  String get locationAreaCavalornValley => 'Valle di Cavalorn';

  @override
  String get locationAreaEastForest => 'Foresta Orientale';

  @override
  String get locationAreaFogTower => 'Torre della Nebbia';

  @override
  String get locationAreaIllegalWeedMixers => 'Mescolatori d\'erba illegali';

  @override
  String get locationAreaOrcArena => 'Arena degli orchi';

  @override
  String get locationAreaOrcGraveyard => 'Cimitero degli orchi';

  @override
  String get locationAreaShipwreck => 'Relitto';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'Impossibile caricare il catalogo delle posizioni.';

  @override
  String get invalid => 'Non valido';

  @override
  String get heroAttributes => 'Attributi dell\'eroe';

  @override
  String attributeBase(String name) {
    return 'Valore base di $name';
  }

  @override
  String attributeCurrent(String name) {
    return '$name attuale';
  }

  @override
  String get attributeBaseValue => 'Valore base';

  @override
  String get attributeCurrentValue => 'Valore attuale';

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
  String get resetInventoryButton => 'Ripristina inventario';

  @override
  String get resetInventoryTooltipDefault =>
      'Sostituisci questo inventario con quello di inizio partita';

  @override
  String get resetInventoryTooltipBlocked =>
      'Prima salva o annulla le modifiche all’inventario in sospeso';

  @override
  String get pendingResetTitle => 'Ripristina l’inventario di inizio partita';

  @override
  String pendingResetSubtitle(String level) {
    return 'Livello risorse: $level';
  }

  @override
  String get cancelPendingReset => 'Annulla ripristino';

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
  String get npcRelationshipRowLabel => 'Relazione';

  @override
  String get npcRelationshipUnavailable =>
      'Stato della relazione non disponibile';

  @override
  String get npcRelationshipAutomatic => 'Calcolata dal gioco';

  @override
  String get npcRelationshipAutomaticHint =>
      'Non è memorizzata alcuna relazione permanente. Il gioco valuta le regole di gilda, storia, area e crimini.';

  @override
  String get npcRelationshipStoredHint =>
      'Memorizzata come relazione permanente tra PNG e giocatore. Le regole di gilda, storia, area e crimini possono comunque modificare la relazione effettiva nel gioco.';

  @override
  String get npcRelationshipFriend => 'Amico';

  @override
  String get npcRelationshipNeutral => 'Neutrale';

  @override
  String get npcRelationshipEnemy => 'Nemico';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Al salvataggio sarà $relationship';
  }

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
  String get memoryEventRemovalQueued =>
      'Rimozione dell’evento in coda — premi Salva per applicarla.';

  @override
  String get duplicateEvent => 'Duplica evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicare l\'evento della memoria?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicare questo evento della memoria? Viene creato prima un backup.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplicazione dell’evento in coda — premi Salva per applicarla.';

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
  String get factionGuildOldCamp => 'Campo Vecchio';

  @override
  String get factionGuildNewCamp => 'Campo Nuovo';

  @override
  String get factionGuildSwampCamp => 'Campo Palude';

  @override
  String get factionGuildOther => 'Altri/individui';

  @override
  String get allDataLockedBody =>
      'Il browser completo delle sorgenti è attualmente disponibile per i salvataggi GSAV.';

  @override
  String get allDataDescription =>
      'Esplora i metadati GSAV e tutti i nodi tipizzati PUBLIC/PRIVATE. I valori scalari e le strutture native sicure sono modificabili; i contenitori e i byte opachi restano visibili.';

  @override
  String get allDataEditable => 'Modificabile';

  @override
  String get allDataReadOnly => 'Sola lettura';

  @override
  String get allDataType => 'Tipo';

  @override
  String get allDataScalars => 'Scalari';

  @override
  String get allDataStructs => 'Strutture';

  @override
  String get allDataContainers => 'Contenitori';

  @override
  String get allDataOpaque => 'Opachi';

  @override
  String get allDataNodes => 'Nodi';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count elementi figli',
      one: '1 elemento figlio',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'In sospeso';

  @override
  String get allDataTagInputHint =>
      'Tag separati da virgole o interruzioni di riga';

  @override
  String allDataTypedSource(String source) {
    return 'Sorgente tipizzata: $source';
  }

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
  String get aboutCopyright => '© 2026 collaboratori di GORE';

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
  String get presetNovice => 'Facile';

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
  String get closeCombatFlowHelper => 'Combattimento ravvicinato assistito';

  @override
  String get permadeath => 'Morte permanente';

  @override
  String get notAvailableOnNovice => 'Non disponibile in modalità Principiante';

  @override
  String get levelCombat => 'Combattimento';

  @override
  String get levelResources => 'Risorse';

  @override
  String get levelProgression => 'Progressi';

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
    return 'Salvataggio… $done di $total';
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

  @override
  String get sectionGlossary => 'Glossario';

  @override
  String get glossarySearch => 'Cerca nel glossario';

  @override
  String get glossaryOldCamp => 'Campo Vecchio';

  @override
  String get glossaryNewCamp => 'Campo Nuovo';

  @override
  String get glossarySwampCamp => 'Campo Palude';

  @override
  String get glossaryOutsiders => 'Esterni';

  @override
  String get glossaryCreatures => 'Creature';

  @override
  String get glossaryLocations => 'Luoghi';

  @override
  String get glossaryFilterLabel => 'Filtro';

  @override
  String get glossaryFilterTraders => 'Mercanti';

  @override
  String get glossaryFilterTeachers => 'Maestri';

  @override
  String get glossaryFilterArmorers => 'Armaioli';

  @override
  String get glossaryFilterHostile => 'Ostili';

  @override
  String get glossaryRelationshipFilterNote =>
      'Mostra le ostilità permanenti memorizzate nel salvataggio. Le relazioni dinamiche di gilda, storia, area e crimini vengono calcolate solo nel gioco.';

  @override
  String get glossaryFilterDead => 'Morti';

  @override
  String get glossaryAddEntry => 'Aggiungi voce al glossario';

  @override
  String get glossaryAddTitle => 'Aggiungi voce al glossario';

  @override
  String get glossaryResetChanges => 'Ripristina modifiche del glossario';

  @override
  String get glossaryNoVisibleEntries =>
      'Nessuna voce visibile del glossario corrisponde a questa vista.';

  @override
  String get glossaryNoHiddenEntries =>
      'Tutte le voci disponibili sono già visibili.';

  @override
  String get glossaryNoMatch => 'Nessuna voce del glossario corrisponde.';

  @override
  String get glossarySelectEntry =>
      'Seleziona una voce del glossario per modificarne le sezioni.';

  @override
  String glossaryEntryCount(int count) {
    return '$count voci';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked di $total voci';
  }

  @override
  String get glossaryPortraitUnlocked => 'Ritratto sbloccato';

  @override
  String get glossaryPortraitSilhouette => 'Sagoma — ritratto non sbloccato';

  @override
  String get glossarySegments => 'Voci';

  @override
  String get glossaryPending => 'Modifica non salvata';

  @override
  String get glossaryShowFullText => 'Mostra il testo completo della voce';

  @override
  String get glossarySegmentIntroduction => 'Introduzione / ritratto';

  @override
  String get glossarySegmentUnlock => 'Scoperta';

  @override
  String glossarySegmentEntry(int number) {
    return 'Voce $number';
  }

  @override
  String get questJournalAll => 'Tutte le missioni';

  @override
  String get questJournalOldCamp => 'Campo Vecchio';

  @override
  String get questJournalNewCamp => 'Campo Nuovo';

  @override
  String get questJournalSwampCamp => 'Campo Palude';

  @override
  String get questJournalColony => 'La Colonia';

  @override
  String get questJournalCompleted => 'Completate';

  @override
  String get questJournalHint =>
      'Vista del diario di gioco. Gli stati interni e le missioni non ancora iniziate restano disponibili in Tutti i dati.';

  @override
  String get questJournalNoEntries =>
      'Nessuna missione del diario corrisponde ai filtri attuali.';

  @override
  String get glossaryTutorials => 'Tutorial';

  @override
  String get tutorialGateNote =>
      'Queste righe controllano gli sblocchi dei tutorial salvati. Uno sblocco non corrisponde necessariamente a una singola pagina del tutorial nel gioco.';

  @override
  String get tutorialResetChanges => 'Ripristina modifiche dei tutorial';

  @override
  String get tutorialNoGates =>
      'Nessuno sblocco di tutorial disponibile in questo salvataggio.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked di $total tutorial sbloccati';
  }

  @override
  String get tutorialGateCombatBasics => 'Basi del combattimento';

  @override
  String get tutorialGateCrafting => 'Creazione';

  @override
  String get tutorialGateCrime => 'Crimini e conseguenze';

  @override
  String get tutorialGateDrugs => 'Consumabili ed effetti';

  @override
  String get tutorialGateLockpicking => 'Scassinamento';

  @override
  String get tutorialGateMagic => 'Magia';

  @override
  String get tutorialGateMap => 'Mappa';

  @override
  String get tutorialGateMeleeCombat => 'Combattimento corpo a corpo';

  @override
  String get tutorialGateNavigation => 'Movimento e navigazione';

  @override
  String get tutorialGatePerception => 'Percezione';

  @override
  String get tutorialGatePlayerProgression => 'Progressione del personaggio';

  @override
  String get tutorialGateRanged => 'Combattimento a distanza';

  @override
  String get tutorialGateRiding => 'Cavalcare';

  @override
  String get tutorialGateSleep => 'Dormire';

  @override
  String get tutorialGateTrading => 'Commercio';

  @override
  String get windowMinimizeTooltip => 'Riduci a icona';

  @override
  String get windowMaximizeTooltip => 'Ingrandisci';

  @override
  String get windowRestoreTooltip => 'Ripristina';

  @override
  String get fallbackDialogEntry => 'Voce di dialogo';

  @override
  String get fallbackDialogChoice => 'Scelta di dialogo';

  @override
  String get fallbackDialogTopic => 'Argomento di dialogo';

  @override
  String get fallbackDialogInformation => 'Informazione di dialogo';

  @override
  String get fallbackQuest => 'Missione';

  @override
  String get fallbackObjective => 'Obiettivo';

  @override
  String get fallbackItem => 'Oggetto';

  @override
  String get attributeSkillPointsFallback => 'Punti apprendimento (PA)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Alcol',
      'AlcoholDepletionRate': 'Tasso di smaltimento dell’alcol',
      'MaxAlcohol': 'Livello massimo di alcol',
      'MaxSuperArmor': 'Super armatura massima',
      'SuperArmor': 'Super armatura',
      'Fatigue': 'Fatica',
      'FillRatio': 'Rapporto di riempimento',
      'FillRatioPeriod': 'Periodo di riempimento',
      'MaxFatigue': 'Fatica massima',
      'MaxThresholdIndex': 'Indice soglia massimo',
      'RecoveryRatePerHourOfSleep': 'Recupero per ora di sonno',
      'DamageMultiplier': 'Moltiplicatore danni',
      'Toughness': 'Tenacia',
      'ToughnessA': 'Tenacia A',
      'ToughnessB': 'Tenacia B',
      'ToughnessC': 'Tenacia C',
      'XPExecutedBounty': 'Ricompensa PE per esecuzione',
      'XPKillOrDefeatBounty': 'Ricompensa PE per uccisione o sconfitta',
      'SpeedModifier': 'Modificatore velocità',
      'CriticalLevelPercent': 'Livello critico (%)',
      'MaxOxygen': 'Ossigeno massimo',
      'Oxygen': 'Ossigeno',
      'OxygenDepletionRate': 'Tasso di consumo dell’ossigeno',
      'OxygenRecoveryRate': 'Tasso di recupero dell’ossigeno',
      'MaxRestTime': 'Tempo massimo di riposo',
      'MaxSleepTime': 'Tempo massimo di sonno',
      'SleepTime': 'Tempo di sonno',
      'SleepTimeRecoveryAmount': 'Quantità recuperata durante il sonno',
      'SleepTimeRecoveryPeriod': 'Intervallo di recupero durante il sonno',
      'MaxSwampweed': 'Quantità massima di erba palustre',
      'Swampweed': 'Erba palustre',
      'SwampweedDepletionRate': 'Tasso di consumo dell’erba palustre',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Battuta vocale';

  @override
  String get knowledgeTypeOther => 'Altro';

  @override
  String get armorUpgradeUpper => 'Superiore';

  @override
  String get armorUpgradeMiddle => 'Centrale';

  @override
  String get armorUpgradeLower => 'Inferiore';

  @override
  String get knowledgeCategoryTopic => 'Argomento';

  @override
  String get knowledgeCategoryChoice => 'Scelta';

  @override
  String get knowledgeCategoryInfo => 'Informazione';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Non riuscito';

  @override
  String get missingSaveReference => 'File mancante';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav è mancante. Potrebbe essere stato eliminato, spostato o rinominato; il profilo continua a farvi riferimento.';
  }

  @override
  String get removeFromProfile => 'Rimuovi dal profilo';

  @override
  String get removeSaveFromProfileTitle =>
      'Rimuovere il salvataggio dal profilo?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Rimuovere $save da $profile? Il file di salvataggio verrà conservato, se esiste ancora.';
  }

  @override
  String get unassignedSave => 'Non assegnato a un profilo';

  @override
  String get armorUpgradeLight => 'Leggero';

  @override
  String get armorUpgradeMedium => 'Medio';

  @override
  String get armorUpgradeHeavy => 'Pesante';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversazione forzata';

  @override
  String get knowledgeCaptionFollowupTopic => 'Argomento successivo';

  @override
  String get knowledgeCaptionFallbackTopic => 'Argomento di riserva';

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
  String get backupStatusInvalidProfileStructure =>
      'Dati del profilo non validi';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Mancano i metadati del salvataggio selezionato';

  @override
  String defaultProfileName(int id) {
    return 'Profilo $id';
  }

  @override
  String get statusUnknown => 'Sconosciuto';

  @override
  String editorUnexpectedError(String details) {
    return 'Errore imprevisto: $details';
  }

  @override
  String get editorOperationInProgress =>
      'È in corso un’altra operazione. Riprova tra poco.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Il salvataggio contiene modifiche non salvate. Salvale o reimpostale prima di cambiare la difficoltà del profilo.';

  @override
  String get editorNoSaveFolderSelected =>
      'Nessuna cartella dei salvataggi selezionata.';

  @override
  String get editorNoSaveSelected => 'Nessun salvataggio selezionato.';

  @override
  String get coreUnknownError => 'Errore interno sconosciuto';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Prima salva o reimposta le modifiche in sospeso: cambiando profilo lasceresti il salvataggio attuale.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Salva o reimposta le modifiche in sospeso prima di aprire un altro file.';

  @override
  String get editorSelectSavFile => 'Seleziona un file di salvataggio .sav.';

  @override
  String get editorNotGothicGsav =>
      'Il file selezionato non è un salvataggio Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Salva o reimposta le modifiche in sospeso prima di cambiare il profilo del salvataggio.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Salva o reimposta le modifiche in sospeso prima di rimuovere un salvataggio dal suo profilo.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Il salvataggio contiene modifiche non salvate. Salvale o reimpostale prima di ripristinare un backup del profilo.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Le modifiche in sospeso di due schede riguardano la stessa proprietà ($path). Reimposta o annulla una delle due, quindi salva di nuovo.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Una modifica a un segmento del glossario e un’altra modifica in sospeso in Tutti i dati riguardano entrambe l’array Hero MemorizedEvents ($path). Le modifiche del glossario aggiungono o rimuovono voci dall’array, quindi non possono essere salvate insieme. Reimposta o annulla una delle due, quindi salva di nuovo.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Una modifica a un segmento del glossario e un’altra modifica in sospeso riguardano la stessa proprietà CurrentState di una missione ($path). La modifica del glossario aggiorna direttamente quello stato. Reimposta o annulla una delle due, quindi salva di nuovo.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Una modifica a una relazione e un’altra modifica in sospeso in Tutti i dati riguardano entrambe la stessa voce di relazione di un PNG ($path). La modifica strutturata della relazione può sostituire i modificatori della voce, quindi non possono essere salvate insieme. Reimposta o annulla una delle due, quindi salva di nuovo.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Più modifiche strutturali in sospeso riguardano lo stesso array ($path). Salva o reimposta la prima modifica prima di aggiungerne un’altra.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Una modifica strutturale a un evento e un’altra modifica in sospeso in Tutti i dati riguardano entrambe $path. Salva o reimposta una delle due prima di continuare.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Sono in sospeso una modifica alle Abilità e una modifica in Tutti i dati per lo stesso effetto del personaggio (ActiveEffects › EffectSpec › Def). Non possono essere salvate insieme. Reimposta o annulla una delle due, quindi salva di nuovo.';

  @override
  String get editorInventoryResetConflict =>
      'Sono in sospeso un ripristino dell’inventario e un’altra modifica allo stesso inventario. Il ripristino sostituisce l’intero inventario e annullerebbe l’altra modifica. Reimposta o annulla una delle due, quindi salva di nuovo.';

  @override
  String get editorUseFolder => 'Usa la cartella';

  @override
  String get editorGothicSavegameFileType => 'Salvataggio Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Nessuna modifica alla difficoltà da salvare';

  @override
  String get editorDifficultyWritten =>
      'Difficoltà salvata nel profilo (backup creato)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count modifiche salvate con backup',
      one: '1 modifica salvata con backup',
    );
    return '$_temp0';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profilo $profileId non trovato.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Non sono disponibili slot liberi nella cartella dei salvataggi del gioco (da G1R-001 a G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Salvataggio importato e assegnato al profilo $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Salvataggio assegnato al profilo $profileId (creati i backup abbinati)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Lo slot di salvataggio $slot non è assegnato al profilo $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Salvataggio rimosso dal profilo';

  @override
  String editorRestoredBackup(String path) {
    return 'Backup ripristinato: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Backup ripristinato: $path (PersistentDataList.sav non è stato modificato perché manca un backup associato corrispondente; i metadati dello slot potrebbero essere diversi)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Verifica di andata e ritorno del codec riuscita: il blocco $chunkIndex è stato ricompresso in $bytes byte';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Impossibile salvare la difficoltà del profilo: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Impossibile assegnare il salvataggio al profilo: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Impossibile rimuovere il salvataggio dal profilo: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Impossibile salvare le modifiche: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Impossibile analizzare i salvataggi: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Impossibile esaminare il salvataggio: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Impossibile caricare i backup: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Impossibile ripristinare il backup: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Backup ripristinato: $path, ma non è stato possibile ricaricare il salvataggio: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Verifica del codec non riuscita: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Verifica di andata e ritorno del codec non riuscita: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Ricerca delle proprietà non riuscita: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Il salvataggio selezionato è cambiato durante il caricamento degli attributi dell’eroe.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Caricamento delle abilità non riuscito: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Query di progressione non riuscita: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Caricamento dell’elenco dei PNG non riuscito: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Caricamento dell’elenco dei personaggi non riuscito: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Caricamento degli attributi del PNG non riuscito: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Caricamento della posizione del PNG non riuscito: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Caricamento dell’inventario del PNG non riuscito: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Caricamento dell’elenco delle fazioni non riuscito: $details';
  }

  @override
  String get editorNoBackupPath => 'nessuno';

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
    return '$prefix: $backupPath; backup di PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Impossibile ottenere lo stato della localizzazione: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Estrazione non riuscita: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Caricamento del glossario non riuscito: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Errore del backup: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Missione',
      'document': 'Documento',
      'story': 'Storia',
      'exploration': 'Esplorazione',
      'combat': 'Combattimento',
      'social': 'Sociale',
      'item': 'Oggetti',
      'learning': 'Apprendimento',
      'guild': 'Gilda',
      'crime': 'Crimine',
      'rest': 'Riposo',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Missione iniziata',
      'questSucceeded': 'Missione completata',
      'questFailed': 'Missione fallita',
      'documentRead': 'Documento letto',
      'documentSegmentUnlocked': 'Voce scoperta',
      'documentSegmentViewed': 'Voce visualizzata',
      'chapterCompleted': 'Capitolo completato',
      'areaEntered': 'Area raggiunta',
      'areaLeft': 'Area lasciata',
      'characterKilled': 'Personaggio ucciso',
      'characterDefeated': 'Personaggio sconfitto',
      'combatDodge': 'Attacco schivato',
      'characterDebuffed': 'Malus applicato',
      'tradeAvailable': 'Commercio sbloccato',
      'itemObtained': 'Oggetto ottenuto',
      'itemCrafted': 'Oggetto creato',
      'skillStateRecorded': 'Stato abilità registrato',
      'recipeLearned': 'Ricetta appresa',
      'guildJoined': 'Ingresso nella gilda',
      'crimeRecorded': 'Crimine registrato',
      'slept': 'Riposo',
      'storyEvent': 'Evento della storia',
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
      'gameTime': 'Tempo di gioco',
      'duration': 'Durata',
      'chapter': 'Capitolo',
      'instigator': 'Avviato da',
      'affected': 'Interessato',
      'amount': 'Quantità',
      'primaryObject': 'Oggetto',
      'secondaryObject': 'Contesto',
      'segmentText': 'Testo della voce',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Giorno $day, $time';
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
  String get memoryEventHero => 'Eroe';

  @override
  String get memoryEventDetails => 'Dettagli';

  @override
  String get memoryEventTags => 'Tag';

  @override
  String get memoryEventTechnicalData => 'Dati tecnici';

  @override
  String get memoryEventIndex => 'Indice';

  @override
  String get memoryEventPosition => 'Posizione';

  @override
  String get memoryEventPayload => 'Contenuto';

  @override
  String get memoryEventSubject => 'Soggetto';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Accesso',
      'AccessDenied': 'Accesso negato',
      'AccesToTemple': 'Accesso al tempio',
      'Advice': 'Consiglio',
      'AfterFight': 'Dopo il combattimento',
      'AfterFireMages': 'Dopo i maghi del fuoco',
      'AfterNek': 'Dopo Nek',
      'AfterQuest': 'Dopo la missione',
      'Alone': 'Solo',
      'Amulet': 'Amuleto',
      'Annoying': 'Fastidioso',
      'Armor': 'Armatura',
      'Avoid': 'Evitare',
      'Backstory': 'Storia personale',
      'BackStory': 'Storia personale',
      'BasicMagic': 'Magia di base',
      'Beated': 'Sconfitto',
      'BecomeMercenary': 'Diventare mercenario',
      'Beer': 'Birra',
      'Bestiary': 'Bestiario',
      'Blessing': 'Benedizione',
      'Boss': 'Capo',
      'Bully': 'Bullo',
      'BullyAdvice': 'Consiglio sul bullo',
      'Camp': 'Campo',
      'CampDivided': 'Campo diviso',
      'CareOfMessengers': 'Prendersi cura dei messaggeri',
      'ChangeOpinion': 'Cambio di opinione',
      'ChargeUriziel': 'Carica Uriziel',
      'Chosen': 'Prescelto',
      'Contact': 'Contatto',
      'Courier': 'Corriere',
      'CraftBows': 'Fabbricare archi',
      'Crazy': 'Pazzo',
      'DailyMeal': 'Pasto quotidiano',
      'DailyRation_Trader': 'Mercante di razioni giornaliere',
      'DAM': 'Diga',
      'Dead': 'Morto',
      'Deal': 'Affare',
      'Dealer': 'Mercante',
      'Deceived': 'Ingannato',
      'Dementia': 'Demenza',
      'DenyAccess': 'Nega l’accesso',
      'DifferentOpinion': 'Opinione diversa',
      'Discussion': 'Discussione',
      'DontTalk': 'Non parlare',
      'Duel': 'Duello',
      'Entrance': 'Ingresso',
      'Escape': 'Fuga',
      'Extended': 'Esteso',
      'Extra': 'Extra',
      'ExtraInfo': 'Informazioni aggiuntive',
      'Fanatic': 'Fanatico',
      'Fight': 'Combattimento',
      'FindUlumulu': 'Trova Ulu-Mulu',
      'FireMages': 'Maghi del fuoco',
      'FireMagesEscape': 'Fuga dei Maghi del Fuoco',
      'FiskNewDealer': 'Nuovo ricettatore per Fisk',
      'FiskNewDealerCompleted': 'Nuovo ricettatore per Fisk — completato',
      'FogTower': 'Torre della Nebbia',
      'Food': 'Cibo',
      'Forgave': 'Ha perdonato',
      'Forgive': 'Perdona',
      'Forgiven': 'Perdonato',
      'FourFriends': 'Quattro amici',
      'FreeHut': 'Capanna libera',
      'FreeMine': 'Miniera Libera',
      'Fury': 'Furia',
      'GoodTeacher': 'Buon insegnante',
      'Gossip': 'Pettegolezzi',
      'GotScavenger': 'Saprofago ottenuto',
      'GrantedAccess': 'Accesso concesso',
      'GRDArmor': 'Armatura da guardia',
      'Guide': 'Guida',
      'HateMages': 'Odio per i maghi',
      'HateMagesExplanation': 'Spiegazione dell’odio per i maghi',
      'HateRiceLord': 'Odio per il Signore del Riso',
      'Heal': 'Cura',
      'Healing': 'Guarigione',
      'Help': 'Aiuto',
      'Helper': 'Aiutante',
      'HelpKagan': 'Aiutare Kagan',
      'HutStory': 'Storia della capanna',
      'Ignore': 'Ignora',
      'Impress': 'Impressionare',
      'ImpressAlchemy': 'Impressionare con l’alchimia',
      'ImpressInscription': 'Impressionare con le iscrizioni',
      'Info': 'Informazioni',
      'Interested': 'Interessato',
      'Introduction': 'Introduzione / ritratto',
      'Introduction_2': 'Introduzione / ritratto 2',
      'Introduction_Armor': 'Introduzione: armatura',
      'Introduction_Teacher': 'Introduzione: maestro',
      'Introduction_Trader': 'Introduzione: mercante',
      'Invocation': 'Invocazione',
      'JoinSC': 'Unirsi a Campo Palude',
      'Joint': 'Spinello',
      'KalomCamp': 'Campo di Kalom',
      'Leader': 'Capo',
      'Learning': 'Apprendimento',
      'LearnOrcish': 'Imparare l’orchese',
      'LeftParty': 'Ha lasciato il gruppo',
      'Library': 'Biblioteca',
      'Lie': 'Menzogna',
      'Lock': 'Serratura',
      'Lockpick': 'Grimaldello',
      'Mad': 'Pazzo',
      'Mandibles': 'Mandibole di pidocchio di miniera',
      'MapMaker': 'Cartografo',
      'Monastery': 'Monastero',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Campo Nuovo',
      'NewCamper': 'Nuovo al campo',
      'NewLeader': 'Nuovo leader',
      'NightPatrol': 'Pattuglia notturna',
      'NotInterested': 'Non interessato',
      'OldCamp': 'Campo Vecchio',
      'OrcEnclaveEntrance': 'Ingresso dell’Enclave degli Orchi',
      'OrcGraveyard': 'Cimitero degli Orchi',
      'OreArmor': 'Armatura minerale',
      'Party': 'Gruppo',
      'Pay': 'Paga',
      'PayMoney': 'Paga soldi',
      'Permission': 'Autorizzazione',
      'Pet': 'Animale domestico',
      'PreparingInvocation': 'Preparazione dell’invocazione',
      'Quest': 'Missione',
      'RankUpFireMages': 'Promozione a Mago del Fuoco',
      'RankUpGuard': 'Promozione a guardia',
      'RanUpFireMagesCompleted': 'Promozione a Mago del Fuoco completata',
      'Realocated': 'Trasferito',
      'Reason': 'Motivo',
      'Respect': 'Rispetto',
      'ReturnToSC': 'Ritorno a Campo Palude',
      'RicelordForeman': 'Caposquadra del Signore del Riso',
      'RideScavenger': 'Cavalcare un saprofago',
      'Robe': 'Veste',
      'Safe': 'Sicuro',
      'Scraper': 'Tritarocce',
      'SecondChance': 'Seconda possibilità',
      'SecretLocation': 'Posizione segreta',
      'SecretPassage': 'Passaggio segreto',
      'SecretPath': 'Sentiero segreto',
      'SleeperFollower': 'Seguace del Dormiente',
      'SleeperTemple': 'Tempio del Dormiente',
      'SmallInfo': 'Breve informazione',
      'Stonehenge': 'Monumento di menhir',
      'StopFollowing': 'Smettere di seguire',
      'SwampCamp': 'Campo Palude',
      'Talkative': 'Loquace',
      'Teach': 'Insegna',
      'TeachBow': 'Insegnare il tiro con l’arco',
      'Teacher': 'Maestro',
      'Teacher2': 'Maestro 2',
      'TeacherInscription': 'Maestro di iscrizioni',
      'TeacherMana': 'Maestro di mana',
      'TeachIchor': 'Insegnare a estrarre l’icore dei pidocchi di miniera',
      'TeachMagic': 'Insegnare la magia',
      'TeachOrcish': 'Insegnare l’orchese',
      'TeachStats': 'Insegnare gli attributi',
      'TeachWeapon': 'Insegnare l’uso delle armi',
      'Teleport': 'Teletrasporto',
      'TheMysteriousOrc': 'L’Orco misterioso',
      'ThroneRoom': 'Sala del Trono',
      'TradeBow': 'Commercio di archi',
      'Trader': 'Commerciante',
      'TradeSkins_Trader': 'Mercante di pelli',
      'Traitor': 'Traditore',
      'Trial': 'Prova',
      'TrollCanyon': 'Canyon con i troll',
      'Trust': 'Fiducia',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Inesperto',
      'Uriziel': 'URIZIEL',
      'UrizielRune': 'Runa Uriziel',
      'Useful': 'Utile',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrazioni',
      'WaitFreeMine': 'Aspettare alla Miniera Libera',
      'WaitInTrainingArea': 'Aspettare nell’area di addestramento',
      'Warning': 'Avvertimento',
      'WarningTooLate': 'Avvertimento troppo tardivo',
      'WaterMessenger': 'Messaggero dei Maghi dell’Acqua',
      'Weapon': 'Arma',
      'Who': 'Chi',
      'Women': 'Donne',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Slot dell’inventario danneggiati';

  @override
  String slotRepairBody(int count) {
    return 'Questo salvataggio contiene $count slot dell’inventario il cui id non corrisponde più alla loro posizione: nel gioco, lasciare cadere un oggetto del genere ne rimuove un altro. La riparazione riscrive solo gli id: nessun oggetto viene aggiunto, rimosso o modificato. Al salvataggio viene creato un backup, come sempre.';
  }

  @override
  String get slotRepairQueued => 'Riparazione in coda — salva per applicarla.';

  @override
  String get slotRepairAction => 'Ripara';

  @override
  String get slotRepairDiscard => 'Annulla';

  @override
  String get editorInventorySlotRepairConflict =>
      'Sono in coda sia una modifica dell’inventario che rinumera gli slot sia una modifica diretta di un id di slot. La rinumerazione scarterebbe la modifica diretta: annullane una, poi salva di nuovo.';

  @override
  String get backupFactFile => 'File';

  @override
  String get renameBackupTooltip => 'Assegna un nome a questo backup';

  @override
  String get renameBackupTitle => 'Assegna un nome al backup';

  @override
  String get renameBackupLabel => 'Nome';

  @override
  String renameBackupHelp(String fileName) {
    return 'Mostrato al posto del nome file $fileName. Lascia vuoto per rimuovere il nome; il file non viene rinominato.';
  }

  @override
  String get deleteBackupTooltip => 'Elimina questo backup';

  @override
  String get deleteBackupTitle => 'Elimina backup';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Eliminare «$name» ($fileName)? Il file viene rimosso dal disco e non può essere recuperato.';
  }

  @override
  String get deleteBackupConfirm => 'Elimina';

  @override
  String editorDeletedBackup(String path) {
    return 'Backup eliminato: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Impossibile eliminare il backup: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Impossibile assegnare un nome al backup: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'La riparazione non è possibile al momento: questo salvataggio non può essere scritto.';
}
