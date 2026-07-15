// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Dialoghi';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Texture';

  @override
  String get tabScripts => 'Script';

  @override
  String get changesAll => 'Tutti';

  @override
  String get sectionItemValues => 'Valori degli oggetti';

  @override
  String get sectionLocalizedText => 'Testi localizzati';

  @override
  String get audioCatCreatures => 'Creature';

  @override
  String get audioCatObjects => 'Oggetti';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimento';

  @override
  String get audioCatWorld => 'Mondo';

  @override
  String get audioCatAction => 'Azioni';

  @override
  String get audioCatCombat => 'Combattimento';

  @override
  String get audioCatPhysics => 'Fisica';

  @override
  String get audioCatItems => 'Item';

  @override
  String get audioCatUi => 'Interfaccia';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Sott\'acqua';

  @override
  String get audioCatVision => 'Visioni';

  @override
  String get audioCatDialog => 'Dialogo';

  @override
  String get audioCatOther => 'Altro';

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
  String get extractLocalizedText => 'Estrai testi localizzati';

  @override
  String get lightMode => 'Modalità chiara';

  @override
  String get darkMode => 'Modalità scura';

  @override
  String get language => 'Lingua';

  @override
  String get exportMod => 'Esporta mod';

  @override
  String exportModWithCount(int count) {
    return 'Esporta mod ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Seleziona un oggetto per modificarne i campi.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dati di gioco: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dati di gioco: inclusi';

  @override
  String get loadGameDataDump => 'Carica dump dei dati di gioco…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json dalla mod gore-dump';

  @override
  String get useBundledData => 'Usa i dati inclusi';

  @override
  String get alreadyBundled => 'già inclusi';

  @override
  String get gameDataFileGroupLabel => 'dati di gioco';

  @override
  String get minimize => 'Riduci a icona';

  @override
  String get restore => 'Ripristina';

  @override
  String get maximize => 'Ingrandisci';

  @override
  String get close => 'Chiudi';

  @override
  String get about => 'Informazioni';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 collaboratori di GORE';

  @override
  String get aboutLicense => 'Concesso in licenza secondo la licenza MIT.';

  @override
  String get categoryMeleeWeapons => 'Armi da mischia';

  @override
  String get categoryRangedWeapons => 'Armi a distanza';

  @override
  String get categoryAmmunition => 'Munizioni';

  @override
  String get categoryRunes => 'Rune';

  @override
  String get categorySpellScrolls => 'Pergamene magiche';

  @override
  String get categoryFoodAndPotions => 'Cibo e pozioni';

  @override
  String get categoryMiscellaneous => 'Varie';

  @override
  String get categoryAmulets => 'Amuleti';

  @override
  String get categoryRings => 'Anelli';

  @override
  String get categoryAnimalTrophies => 'Trofei di animali';

  @override
  String get categoryWritings => 'Scritti';

  @override
  String get categoryMissionItems => 'Oggetti della missione';

  @override
  String get categoryKeys => 'Chiavi';

  @override
  String get categoryOther => 'Altro';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Cerca oggetti';

  @override
  String get noItemsMatch => 'Nessun oggetto corrispondente';

  @override
  String failedToLoadCatalog(String error) {
    return 'Impossibile caricare il catalogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Modifiche in sospeso ($count)';
  }

  @override
  String get clearAll => 'Cancella tutto';

  @override
  String get noPendingOverrides =>
      'Nessuna modifica in sospeso.\nModifica i campi degli oggetti per aggiungerne.';

  @override
  String get removeOverride => 'Rimuovi modifica';

  @override
  String get searchChanges => 'Cerca modifiche';

  @override
  String get noChangesMatch => 'Nessuna modifica corrispondente';

  @override
  String get clearSection => 'Cancella questo gruppo';

  @override
  String get modName => 'Nome della mod';

  @override
  String get loadDelayLabel => 'Ritardo di caricamento (ms, 0 = immediato)';

  @override
  String get noFolderSelected => 'Nessuna cartella selezionata';

  @override
  String get chooseFolder => 'Scegli cartella';

  @override
  String get packageAsZip => 'Crea pacchetto .zip';

  @override
  String get cancel => 'Annulla';

  @override
  String get export => 'Esporta';

  @override
  String get exportHere => 'Esporta qui';

  @override
  String get mustBeNonNegativeInteger => 'Deve essere un intero non negativo';

  @override
  String get extractingLocalizedText =>
      'Estrazione dei testi localizzati del gioco…';

  @override
  String get localizedTextExtractionCancelled =>
      'Estrazione dei testi localizzati annullata.';

  @override
  String get localizedTextExtracted => 'Testi localizzati estratti.';

  @override
  String get extractionFailed => 'Estrazione non riuscita.';

  @override
  String get localizationCacheFileGroupLabel => 'cache di localizzazione';

  @override
  String get extractLocalizedTextQuestion =>
      'Estrarre i testi localizzati del gioco?';

  @override
  String get extractLocalizedTextBody =>
      'I testi localizzati del gioco non sono ancora stati estratti. Estrarli ora dalla tua installazione del gioco? (facoltativo)';

  @override
  String get notNow => 'Non ora';

  @override
  String get extract => 'Estrai';

  @override
  String get validationRequired => 'Obbligatorio';

  @override
  String get validationMustBeWholeNumber => 'Deve essere un numero intero';

  @override
  String get validationMustBeNumber => 'Deve essere un numero';

  @override
  String get validationMustBeFinite => 'Deve essere un numero finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve essere ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve essere ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve essere true o false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve essere uno tra: $options';
  }

  @override
  String get modNameRequired => 'Obbligatorio';

  @override
  String get modNameControlCharacters =>
      'Non deve contenere caratteri di controllo';

  @override
  String get modNamePathSeparators =>
      'Non deve contenere separatori di percorso';

  @override
  String get modNameNotAFolderName => 'Nome cartella non valido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID estratti in $languageCount lingue';
  }

  @override
  String get managerDeployActive =>
      'È attivo un loadout del mod-manager. Esegui prima l\'undeploy in gore-manager.';

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
      'Il nuovo progetto è aperto, ma non è stato possibile ripulire completamente la sessione del progetto precedente. La pulizia non verrà ritentata. Riavvia Mod Studio prima di riaprire il progetto precedente.';

  @override
  String get projectNewManagedRevision3 => 'Nuovo progetto mod gestito…';

  @override
  String get projectNewLegacy => 'Nuovo progetto legacy';

  @override
  String get projectCreateGamePathRequired =>
      'Imposta il percorso di Gothic 1 Remake nelle Impostazioni prima di creare un progetto mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Crea qui il progetto mod gestito';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Progetto mod gestito $projectId creato';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Creazione del progetto mod gestito non riuscita: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Crea un progetto mod';

  @override
  String get projectCreateNameLabel => 'Nome del progetto';

  @override
  String get projectCreateNameHelper => 'Il nome visualizzato in Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versione';

  @override
  String get projectCreateVersionHelper =>
      'Una versione iniziale, ad esempio 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autore';

  @override
  String get projectCreateAuthorHelper =>
      'Il tuo nome o quello del team di modding.';

  @override
  String get projectCreateLocalesLabel => 'Lingue di authoring';

  @override
  String get projectCreateLocalesHelper =>
      'Tag canonici separati da virgole, ad esempio: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Questo crea un progetto offline gestito e vuoto. Non compila, distribuisce o esegue una mod e non modifica i file del gioco o i salvataggi.';

  @override
  String get projectCreateSubmit => 'Crea progetto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label è obbligatorio.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label non può iniziare o terminare con spazi.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label non può contenere caratteri di controllo.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contiene testo non valido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label supera il limite UTF-8 di $maxBytes byte.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Inserisci almeno una lingua di authoring.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Rimuovi la voce vuota della lingua.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Usa al massimo $maxLocales lingue di authoring.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'Il locale «$locale» deve essere ASCII e di lunghezza limitata.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'Il locale «$locale» richiede una lingua minuscola da 2 a 8 lettere.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'Il locale «$locale» contiene un segmento non valido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'Il locale «$locale» non è canonico; usa «$canonical».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Panoramica';

  @override
  String get managedWorkspaceContentLabel => 'Contenuti';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Questa mod';

  @override
  String get managedWorkspaceHomeLabel => 'Inizio';

  @override
  String get managedWorkspaceStoryLabel => 'Storia';

  @override
  String get managedWorkspaceWorldLabel => 'Mondo';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localizzazione e voci';

  @override
  String get managedWorkspaceValidateTestLabel => 'Convalida e test';

  @override
  String get managedWorkspaceBuildReleaseLabel =>
      'Compilazione e pubblicazione';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Impostazioni e modalità esperta';

  @override
  String get managedSectionStoryDescription => 'PNG, missioni e dialoghi.';

  @override
  String get managedSectionWorldDescription =>
      'Il posizionamento nel mondo e i relativi flussi di lavoro sono pianificati.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Gli strumenti di produzione vocale sono disponibili; la modifica delle localizzazioni nel progetto gestito è pianificata.';

  @override
  String get managedSectionValidateTestDescription =>
      'Verifica l’integrità esatta del progetto e i checkpoint; non attesta test in esecuzione.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'I bundle vocali sono disponibili; le build giocabili complete e la distribuzione non lo sono.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Le impostazioni sono disponibili; gli strumenti esperti non sono ancora integrati.';

  @override
  String get managedSectionStatusHeading => 'Stato';

  @override
  String get managedSectionActionsHeading => 'Azioni';

  @override
  String get managedCapabilityAvailable => 'Disponibile';

  @override
  String get managedCapabilityPartial => 'Parziale';

  @override
  String get managedCapabilityPlanned => 'Pianificato';

  @override
  String get managedCapabilityUnavailable => 'Non disponibile';

  @override
  String get managedProjectSubtitle =>
      'Area di creazione offline allineata esattamente alla versione corrente';

  @override
  String get managedProjectLandingTitle =>
      'Area di lavoro del progetto gestito';

  @override
  String get managedProjectLandingDescription =>
      'Usa il nuovo flusso Home, Contenuti, Storia, Voce, convalida e pubblicazione in un unico progetto gestito.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Strumenti di compatibilità legacy';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Le schede qui sotto contengono i vecchi strumenti di sostituzione diretta. Restano disponibili mentre l’area di lavoro del progetto gestito continua a crescere.';

  @override
  String get managedProjectTechnicalDetails => 'Dettagli tecnici del progetto';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Riapri il progetto gestito prima di leggerne i contenuti.';

  @override
  String get managedDashboardUntitledProject => 'Progetto senza titolo';

  @override
  String get managedDashboardDraftStatus => 'Bozza';

  @override
  String get managedDashboardProjectVersion => 'Versione';

  @override
  String get managedDashboardProjectAuthor => 'Autore';

  @override
  String get managedDashboardNotProvided => 'Non specificato';

  @override
  String get managedDashboardContentCounts => 'Contenuti del progetto';

  @override
  String get managedDashboardNpcDrafts => 'Bozze di PNG';

  @override
  String get managedDashboardQuestDrafts => 'Bozze di missioni';

  @override
  String get managedDashboardDialogLines => 'Righe di dialogo';

  @override
  String get managedDashboardVoiceTakes => 'Registrazioni vocali';

  @override
  String get managedDashboardAssets => 'Risorse';

  @override
  String get managedDashboardUnresolvedReferences => 'Riferimenti irrisolti';

  @override
  String get managedDashboardReadiness => 'Cosa funziona ora';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Creazione offline disponibile';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Crea e modifica i contenuti del progetto supportati senza cambiare l’installazione del gioco o i file di salvataggio.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Build generale della mod non disponibile';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'È possibile creare solo bundle Voice offline sigillati; non è ancora possibile creare una mod completa e giocabile.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Runtime non ancora verificato';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio non ha ancora verificato questi contenuti del progetto all’interno del gioco in esecuzione.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Integrità dei riferimenti';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Questo conteggio verifica solo i riferimenti del progetto; non indica che la build o il runtime siano pronti.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Configurazione del gioco necessaria';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configura l’installazione di Gothic 1 Remake nelle Impostazioni prima di usare azioni che richiedono dati verificati dal gioco installato.';

  @override
  String get managedDashboardCreateHeading => 'Crea';

  @override
  String get managedDashboardToolsHeading => 'Strumenti del progetto';

  @override
  String get managedDashboardLoading =>
      'Caricamento della panoramica del progetto';

  @override
  String get managedDashboardLoadError =>
      'Panoramica del progetto non disponibile';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Non è stato possibile caricare la panoramica verificata del progetto. I contenuti del progetto non sono stati modificati.';

  @override
  String get managedDashboardRetry => 'Riprova';

  @override
  String get managedActionNewNpcTitle => 'Nuovo PNG';

  @override
  String get managedActionNewNpcDescription =>
      'Crea una bozza di PNG offline e circoscritta da dati verificati del gioco installato.';

  @override
  String get managedActionNewQuestTitle => 'Nuova missione';

  @override
  String get managedActionNewQuestDescription =>
      'Crea una bozza di missione offline con obiettivi e identità principali verificate.';

  @override
  String get managedActionAddVoiceTakeTitle => 'Aggiungi registrazione vocale';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importa una registrazione Ogg Vorbis in questo progetto senza distribuirla.';

  @override
  String get managedActionManageVoiceTakesTitle =>
      'Gestisci registrazioni vocali';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Esamina le registrazioni e seleziona quelle approvate per gli slot Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle =>
      'Risolvi destinazione Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Associa gli slot Voice del progetto ai membri esatti degli archivi installati senza modificare il gioco.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Crea bundle Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Crea un bundle offline sigillato da membri esistenti; non viene eseguita alcuna distribuzione.';

  @override
  String get managedActionDataAssetsTitle => 'Modifiche ai DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Ispeziona i pacchetti installati e prepara nel progetto modifiche verificate a valori di larghezza fissa.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Esplora i contenuti esatti del progetto e i relativi riferimenti risolti o non risolti.';

  @override
  String get managedActionSettingsTitle => 'Impostazioni';

  @override
  String get managedActionSettingsDescription =>
      'Configura l’installazione di Gothic 1 Remake e le preferenze di Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Il progetto $projectId è stato creato in sicurezza, ma la configurazione iniziale non si è aperta. Il progetto vuoto valido resta attivo.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Il progetto $projectId è stato creato, ma Mod Studio non può verificare l’esito dell’avvio. Riapri il progetto gestito prima di continuare; il gioco e i salvataggi non sono stati modificati.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Il progetto $projectId è stato creato. L’avvio NPC non è stato aggiunto, quindi il progetto vuoto valido resta attivo.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Avvio NPC salvato nella revisione $projectRevision. Resta bloccato per la compilazione, non qualificato in esecuzione e non viene generato.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Il progetto $projectId è stato creato. L’avvio missione non è stato aggiunto, quindi il progetto vuoto valido resta attivo.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Avvio missione salvato nella revisione $projectRevision. Resta bloccato per la compilazione e non qualificato in esecuzione.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Avvio del progetto';

  @override
  String get projectStarterPrompt => 'Come vuoi iniziare?';

  @override
  String get projectStarterWriteBoundary =>
      'La scelta di un avvio non scrive nulla. Il progetto viene creato solo dopo l’invio del modulo e la scelta di una cartella vuota.';

  @override
  String get projectStarterEmptyTitle => 'Progetto vuoto';

  @override
  String get projectStarterEmptyDescription =>
      'Crea solo il progetto gestito. Aggiungi contenuti quando vuoi.';

  @override
  String get projectStarterNpcDraftTitle => 'Bozza NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Crea prima il progetto vuoto, quindi apri la configurazione guidata della bozza NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Bozza missione';

  @override
  String get projectStarterQuestDraftDescription =>
      'Crea prima il progetto vuoto, quindi apri la configurazione guidata della bozza missione.';

  @override
  String get projectStarterPartialOutcome =>
      'Se annulli la configurazione guidata di NPC o missione, oppure la bozza non riesce, resta un progetto vuoto valido. La scelta non scrive nel gioco o in un salvataggio.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Esplora';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Modifiche verificate';

  @override
  String get managedContentScopeBaseGameLabel => 'Gioco base';

  @override
  String get managedContentScopeInstalledLabel => 'Installato';

  @override
  String get managedBaseGameBrowserTitle =>
      'Punti di partenza supportati del gioco base';

  @override
  String get managedBaseGameBrowserDescription =>
      'Esplora le prove esatte del gioco installato che Mod Studio può ispezionare o usare come punto di partenza sicuro per una bozza. Non è un catalogo completo dei contenuti originali.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Lettura delle prove esatte del gioco base…';

  @override
  String get managedBaseGameBrowserRefresh => 'Leggi un nuovo catalogo esatto';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Cerca nei contenuti supportati del gioco base';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Missioni';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'Punti di partenza NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle =>
      'Punti di partenza missione';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Archetipi NPC solo da ispezionare';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Cerca per includere altre prove NPC con collegamento statico. Queste righe non possono creare una bozza.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Nessun risultato supportato del gioco base corrisponde alla ricerca.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Prove del gioco base non disponibili';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Impossibile leggere il catalogo esatto supportato. Nessun file di progetto, gioco o salvataggio è stato modificato.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Bozza offline supportata';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Solo ispezione';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'Usa come avvio NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Usa come avvio missione';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Definizione di generazione';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint attore';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Sono mostrati i primi 100 risultati solo da ispezionare. Affina la ricerca per risultati più specifici.';

  @override
  String get managedInstalledBrowserLoading =>
      'Lettura dell’inventario esatto dei pacchetti installati…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count pacchetti installati candidati';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count pacchetti installati candidati — risultato parziale';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'I metadati della directory sono stati letti e l’istantanea installata è rimasta esatta.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Alcuni metadati dei pacchetti mancavano o non erano canonici; i risultati aiutano la ricerca ma non sono completi.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Questo ambito mostra solo i metadati dei pacchetti DataAsset installati. Ispezionare o copiare un percorso non concede autorità di compilazione, distribuzione, esecuzione o scrittura nel gioco.';

  @override
  String get managedInstalledBrowserRefresh =>
      'Leggi una nuova istantanea esatta';

  @override
  String get managedInstalledBrowserSearchLabel => 'Cerca DataAsset installati';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nome risorsa o percorso /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Digita un nome risorsa o un percorso /Game da cercare.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Nessun DataAsset installato corrispondente';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Prova un altro nome risorsa o un percorso /Game più ampio.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Sono mostrati i primi 100 risultati. Affina la ricerca per restringere l’istantanea esatta.';

  @override
  String get managedInstalledBrowserKindBadge => 'Pacchetto DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Solo metadati';

  @override
  String get managedInstalledBrowserOpenInspector =>
      'Ispeziona pacchetto esatto';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventario dei pacchetti installati non disponibile';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Impossibile leggere l’istantanea installata esatta. Nessun file di progetto, gioco o salvataggio è stato modificato.';
}
