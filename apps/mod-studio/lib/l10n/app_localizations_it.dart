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
      'Il posizionamento nel mondo e i relativi flussi di lavoro sono pianificati.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Scrivi e traduci i dialoghi del progetto in un unico posto, quindi continua con le voci.';

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
  String get managedActionNewDialogLineTitle => 'Aggiungi riga di dialogo';

  @override
  String get managedActionNewDialogLineDescription =>
      'Scrivi testo di progetto localizzato o collega un testo inutilizzato già presente nel progetto. Questo non crea un argomento di dialogo giocabile.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Riga di dialogo salvata nella revisione $projectRevision del progetto. Il gioco e i salvataggi non sono stati modificati.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Scrivi una nuova riga di dialogo localizzata o collega un testo che appartiene già a questo progetto.';

  @override
  String get managedDialogLineBoundary =>
      'Vengono modificati solo i file del progetto. Questo non crea un argomento AngelScript né un dialogo giocabile e non modifica mai l’installazione del gioco o i salvataggi. Il campo del parlante è solo un’etichetta e non collega alcun PNG.';

  @override
  String get managedDialogLineCreateMode => 'Scrivi nuovo testo';

  @override
  String get managedDialogLineReuseMode => 'Usa testo del progetto';

  @override
  String get managedDialogLineNameLabel => 'Nome della riga';

  @override
  String get managedDialogLineNameHint => 'Saluto all’ingresso della miniera';

  @override
  String get managedDialogLineSpeakerLabel =>
      'Etichetta del parlante (opzionale)';

  @override
  String get managedDialogLineSpeakerHint => 'Ad esempio, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Lingua';

  @override
  String get managedDialogLineTextLabel => 'Testo del dialogo';

  @override
  String get managedDialogLineReuseSearch =>
      'Cerca testo del progetto inutilizzato';

  @override
  String get managedDialogLineNoReusableText =>
      'Non c’è testo di progetto inutilizzato e strutturalmente valido da collegare. Scrivi invece un nuovo testo.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Prepara questa lingua per Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Crea uno slot Voice vuoto e non risolto nel progetto. Non aggiunge né distribuisce alcuna registrazione.';

  @override
  String get managedDialogLineCancel => 'Annulla';

  @override
  String get managedDialogLineSave => 'Salva nel progetto';

  @override
  String get managedDialogLineSaving => 'Salvataggio…';

  @override
  String get managedDialogLineLoading =>
      'Lettura del contenuto esatto del progetto…';

  @override
  String get managedDialogLineLoadFailed =>
      'Impossibile leggere il contenuto corrente esatto del progetto. Non è stato modificato nulla.';

  @override
  String get managedDialogLineRetry => 'Riprova';

  @override
  String get managedDialogLineStale =>
      'Il progetto è cambiato mentre questa finestra era aperta. Chiudila e riprova dal progetto corrente.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Il progetto corrente non può più essere verificato in modo sicuro. Chiudi questa finestra e riapri il progetto gestito.';

  @override
  String get managedDialogLineInvalidInput =>
      'Controlla l’input del progetto evidenziato e scegli un’opzione corrente esatta.';

  @override
  String get managedDialogLineSaveFailed =>
      'Non è stato possibile salvare in modo sicuro la riga di dialogo. Il gioco e i salvataggi non sono stati modificati.';

  @override
  String get managedDialogLineDone => 'Fatto';

  @override
  String get managedDialogLineAddRecording => 'Aggiungi registrazione';

  @override
  String get managedActionAddVoiceTakeTitle => 'Aggiungi registrazione vocale';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importa una registrazione Ogg Vorbis in questo progetto senza distribuirla.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

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

  @override
  String get managedGlobalSearchScopeLabel => 'Cerca ovunque';

  @override
  String get managedGlobalSearchTitle => 'Cerca in tutti i contenuti';

  @override
  String get managedGlobalSearchLabel =>
      'PNG, missione, battuta, risorsa, ID o percorso /Game';

  @override
  String get managedGlobalSearchAction => 'Cerca';

  @override
  String get managedGlobalSearchClear => 'Cancella';

  @override
  String get managedGlobalSearchPrompt =>
      'Inserisci una ricerca per consultare le tre fonti separatamente.';

  @override
  String get managedGlobalSearchNoResults =>
      'Nessuna corrispondenza in questa fonte.';

  @override
  String get managedGlobalSearchLoading => 'Lettura della fonte esatta…';

  @override
  String get managedGlobalSearchFailed => 'Impossibile leggere questa fonte.';

  @override
  String get managedGlobalSearchComplete => 'Completo';

  @override
  String get managedGlobalSearchPartial => 'Parziale';

  @override
  String get managedGlobalSearchTruncated =>
      'Sono mostrate le prime 100 corrispondenze. Affina la ricerca.';

  @override
  String get managedGlobalSearchOpen => 'Apri';

  @override
  String get managedGlobalSearchCreateDraft => 'Crea bozza';

  @override
  String get managedGlobalSearchInspect => 'Ispeziona';

  @override
  String get managedGlobalSearchKindModEntity => 'Contenuto del mod';

  @override
  String get managedGlobalSearchKindModAsset => 'Risorsa del mod';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Punto di partenza PNG';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Punto di partenza missione';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Evidenza PNG';

  @override
  String get managedGlobalSearchReadinessExact => 'Progetto corrente esatto';

  @override
  String get managedGlobalSearchReadinessProblems => 'Esatto, con problemi';

  @override
  String get managedGlobalSearchResultStale =>
      'Questo risultato non è più nel progetto corrente. Ripeti la ricerca.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Solo bozza';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Compilazione bloccata';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Esecuzione non verificata';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Panoramica';

  @override
  String get managedStoryWorkbenchProfileTab => 'Profilo';

  @override
  String get managedStoryWorkbenchStoryTab => 'Storia';

  @override
  String get managedStoryWorkbenchLogicTab => 'Logica';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Routine';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventario';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Dialoghi e voce';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Riferimenti';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => 'Problemi e verifiche';

  @override
  String get managedStoryWorkbenchEditOverview => 'Modifica nome e obiettivi';

  @override
  String get managedStoryWorkbenchEditStory =>
      'Modifica descrizione e collegamenti';

  @override
  String get managedStoryWorkbenchEditLogic => 'Modifica stati e transizioni';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Apri codice sorgente e verifiche del compilatore';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Apri profilo e verifiche del compilatore';

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
      'Non ancora modellato';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Le relazioni con missioni e storia non sono ancora modellate per le bozze dei PNG.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'La routine e il posizionamento nel mondo non sono ancora modellati.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'L’inventario, l’equipaggiamento e il commercio non sono ancora modellati.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Le relazioni con dialoghi, localizzazione e voce non sono ancora modellate per le bozze dei PNG.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Le relazioni con dialoghi, localizzazione e voce non sono ancora modellate per le bozze delle missioni.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Nessun riferimento di progetto irrisolto';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count riferimenti di progetto irrisolti',
      one: '1 riferimento di progetto irrisolto',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Indica solo lo stato dei riferimenti; non garantisce che il progetto sia pronto per la compilazione o l’esecuzione.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Dettagli tecnici';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Bozza missione';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Bozza PNG';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Titolo missione';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'ID tecnico';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Obiettivi';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Nome univoco';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Spazio dei nomi del modulo';

  @override
  String get managedStoryWorkbenchQuestGiverLabel =>
      'Assegnatore della missione';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => 'Classe base a runtime';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Gli stati del ciclo di vita della missione, gli eventi di attivazione, le condizioni e gli effetti vengono modificati come un’unica operazione atomica sullo stato corrente esatto.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'In uscita';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Nessun riferimento previsto';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'In ingresso';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Nessun riferimento di progetto in ingresso';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel => 'Identità semantica';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origine';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Revisione entità';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'ID stabile';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Riferimento risolto';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Riferimento non risolto';

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

  @override
  String get managedVoiceTakeRemoveAction => 'Remove from this line…';

  @override
  String get managedVoiceTakeRemoveTooltip =>
      'Remove this recording from the current dialog line and language';

  @override
  String get managedVoiceTakeRemoveDialogTitle => 'Remove Voice take?';

  @override
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  ) {
    return 'Remove “$take” from $line ($locale)?';
  }

  @override
  String get managedVoiceTakeRemoveScope =>
      'Only the link for this dialog line and language is removed. Other project uses remain unchanged.';

  @override
  String get managedVoiceTakeRemoveInternalRetention =>
      'The audio file remains stored internally. This action does not free project storage and has no undo yet.';

  @override
  String get managedVoiceTakeRemoveGameBoundary =>
      'The game installation and save games are not changed.';

  @override
  String get managedVoiceTakeRemoveSelectedWarning =>
      'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveCancel => 'Cancel';

  @override
  String get managedVoiceTakeRemoveConfirm => 'Remove from line';

  @override
  String get managedVoiceTakeRemoveUniqueSuccess =>
      'The take was removed from this line and from the current project graph. Its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSharedSuccess =>
      'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSelectionClearedSuccess =>
      'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveStale =>
      'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.';

  @override
  String get managedVoiceTakeRemoveRequiresReopen =>
      'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedUnconfirmed =>
      'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedReloadFailed =>
      'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.';

  @override
  String managedVoiceTakeRemoveFailed(String error) {
    return 'The take was not removed: $error';
  }

  @override
  String get managedVoiceTakeRemoveReloadConfirmed =>
      'The saved removal was confirmed from the latest project.';
}
