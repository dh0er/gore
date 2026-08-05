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
  String get projectOpenManagedRevision3 => 'Open mod project…';

  @override
  String get projectVerifyCurrentHead => 'Check project';

  @override
  String get projectManagedRevision3Title => 'Mod project';

  @override
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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
  String get projectManagedRevision3Opened => 'Mod project opened.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod project could not be opened: $error';
  }

  @override
  String get projectManagedRevision3Verified => 'Project checked.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Project check failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'The project could not be checked safely. Recover or reopen it before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Recover or reopen the project before checking it again.';

  @override
  String get projectTransitionCleanupWarning =>
      'Il nuovo progetto è aperto, ma non è stato possibile ripulire completamente la sessione del progetto precedente. La pulizia non verrà ritentata. Riavvia Mod Studio prima di riaprire il progetto precedente.';

  @override
  String get projectNewManagedRevision3 => 'Nuovo progetto mod…';

  @override
  String get projectCreateGamePathRequired =>
      'Imposta il percorso di Gothic 1 Remake nelle Impostazioni prima di creare un progetto mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Crea qui il progetto mod gestito';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Progetto mod $projectId creato';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Creazione del progetto mod non riuscita: $error';
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
  String get managedWorkspaceSettingsExpertLabel =>
      'Impostazioni e modalità esperta';

  @override
  String get managedProjectHistoryTitle => 'Project history';

  @override
  String get managedProjectHistoryDescription =>
      'Return to an earlier project version without erasing the versions that came after it.';

  @override
  String get managedProjectHistoryBoundary =>
      'History changes only this managed project. It does not modify the game installation or save files.';

  @override
  String get managedProjectHistoryRefresh => 'Refresh project history';

  @override
  String get managedProjectHistoryLoading => 'Loading project history…';

  @override
  String get managedProjectHistoryLoadFailed =>
      'Project history could not be loaded';

  @override
  String get managedProjectHistoryRetry => 'Try again';

  @override
  String get managedProjectHistoryCurrentVersion => 'Current version';

  @override
  String get managedProjectHistoryPreviousVersions => 'Previous versions';

  @override
  String get managedProjectHistoryUndo => 'Undo last change';

  @override
  String get managedProjectHistoryRestoreVersion => 'Restore this version';

  @override
  String get managedProjectHistoryRestoreTitle => 'Restore project version?';

  @override
  String managedProjectHistoryRestoreBody(int revision, int nextRevision) {
    return 'The content from revision $revision will be saved as new revision $nextRevision. The current version remains in history.';
  }

  @override
  String get managedProjectHistoryRestoreBoundary =>
      'Only the project changes. The game installation and save files remain untouched.';

  @override
  String get managedProjectHistoryCancel => 'Cancel';

  @override
  String get managedProjectHistoryRestore => 'Restore';

  @override
  String get managedProjectHistoryRestoring => 'Restoring project version…';

  @override
  String get managedProjectHistoryRestoreFailed =>
      'The project version could not be restored safely. Refresh the history before trying again.';

  @override
  String managedProjectHistoryRestoreSucceeded(int revision) {
    return 'Revision $revision was restored as a new project version.';
  }

  @override
  String get managedProjectHistoryEmpty =>
      'No previous project versions have been recorded yet.';

  @override
  String managedProjectHistoryRecordingStartsAt(int revision) {
    return 'History recording starts at revision $revision; older versions were not guessed from storage.';
  }

  @override
  String get managedProjectHistoryTruncated =>
      'Older project versions have expired from history. Every version shown here is still retained and authenticated by the current project history.';

  @override
  String managedProjectHistoryRevision(int revision) {
    return 'Revision $revision';
  }

  @override
  String get managedProjectHistoryCurrentBadge => 'Current';

  @override
  String get managedProjectHistoryDirtyBlocked =>
      'Finish or discard the open text edit before restoring another project version.';

  @override
  String get managedProjectHistoryBusy =>
      'Another project action is still in progress.';

  @override
  String get managedProjectHistoryUnavailable =>
      'This managed project session does not support authenticated history.';

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
  String get managedStoryWorkspaceCreateNpcOpening =>
      'Create Character + first greeting';

  @override
  String get managedStoryWorkspaceCreatingNpcOpening =>
      'Creating Character + first greeting…';

  @override
  String get managedStoryWorkspaceCreateQuestOpening =>
      'Create Quest + opening line';

  @override
  String get managedStoryWorkspaceCreatingQuestOpening =>
      'Creating Quest + opening line…';

  @override
  String get managedStoryWorkspaceCreateAdvanced => 'Advanced creation options';

  @override
  String get managedStoryWorkspaceCreateNpcAdvanced =>
      'Create Character draft only (advanced)';

  @override
  String get managedStoryWorkspaceCreateQuestAdvanced =>
      'Create Quest draft only (advanced)';

  @override
  String get managedStoryWorkspaceMutationRequiresReopen =>
      'Reopen this project before changing Story content.';

  @override
  String get managedStoryWorkspaceMutationDirtyBlocked =>
      'Save or discard the open localization edits before changing Story content.';

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
      'This dialog has no local rollback. After removal, Project History or global Undo can restore an earlier version while it remains available.';

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
  String get managedLocalizationVoiceContextTitle =>
      'Voice for this dialog line';

  @override
  String get managedLocalizationVoiceSelectLine => 'Select a dialog line above';

  @override
  String get managedLocalizationVoiceSetupExists => 'setup exists';

  @override
  String get managedLocalizationVoiceSetupMissing => 'no setup yet';

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
      'This project has unsaved edits. Switching now would discard them.';

  @override
  String get managedLocalizationVoiceUnsavedTitle =>
      'Save text before continuing?';

  @override
  String get managedLocalizationVoiceUnsavedDescription =>
      'Save these text changes and continue directly to the selected action, keep editing, or deliberately discard the text changes.';

  @override
  String get managedLocalizationDiscardAndContinue => 'Discard and continue';

  @override
  String get managedLocalizationSaveAndContinue => 'Save and continue';

  @override
  String get managedLocalizationGlobalAddVoice => 'Add take for any line';

  @override
  String get managedLocalizationGlobalManageVoice =>
      'Manage takes for any line';

  @override
  String get managedLocalizationGlobalResolveVoice =>
      'Resolve target for any line';

  @override
  String get managedVoiceFolderImportTitle => 'Import recordings folder';

  @override
  String get managedVoiceFolderImportDescription =>
      'Review a folder of named Ogg recordings, then add every ready take in one all-or-nothing project update.';

  @override
  String get managedVoiceFolderImportChooseFolder => 'Choose recordings folder';

  @override
  String get managedVoiceFolderImportDirtyBlocked =>
      'Save or discard the open localization edits before importing recordings.';

  @override
  String managedVoiceFolderImportSaved(int count, int revision) {
    return 'Imported $count recordings in project revision $revision. They are project-only Recorded takes; selection, game files, and saves were not changed.';
  }

  @override
  String managedVoiceTakeSaved(int revision) {
    return 'Voice take saved in project revision $revision. It is saved to the project only and is not yet usable in game.';
  }

  @override
  String managedVoiceSelectionCleared(int revision) {
    return 'Voice selection cleared in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceSelectionSelected(int revision) {
    return 'Approved Voice take selected in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceTargetUnresolvedSaved(int revision) {
    return 'No installed archive member matched. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetResolvedSaved(int revision) {
    return 'One installed archive member was sealed. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetAmbiguousSaved(int count, int revision) {
    return '$count installed archive members matched; nothing was chosen implicitly. Voice target evidence saved in project revision $revision.';
  }

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
  String get managedLocalizationVoiceActionFailed =>
      'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Le impostazioni e il DataAsset Lab di sola lettura sono disponibili.';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => 'Avvia un progetto mod';

  @override
  String get managedProjectLandingDescription =>
      'Crea un progetto, apri una cartella di progetto esistente o ripristina un backup.';

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
  String get managedDashboardChangesDescription =>
      'Everything currently saved in this exact project, grouped by what you can work on. Generated helpers stay attached only when their relationship is proven.';

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
  String get managedDashboardItemPatches => 'Items';

  @override
  String get managedDashboardLocalizationEntries => 'Project text';

  @override
  String get managedDashboardVoiceSlots => 'Voice target';

  @override
  String get managedDashboardGeneratedScripts => 'Generated script';

  @override
  String get managedDashboardSelectedVoiceTake => 'Selected take';

  @override
  String get managedDashboardTechnicalContent => 'Technical content';

  @override
  String get managedDashboardTechnicalContentDescription =>
      'Generated or problematic helpers that cannot be safely assigned to an author-facing change.';

  @override
  String get managedDashboardEmptyChangesTitle => 'No changes yet';

  @override
  String get managedDashboardEmptyChangesDescription =>
      'Use Create, Content, or Story to add the first project change. Nothing has been written to the game or a save.';

  @override
  String get managedDashboardOpenChange => 'Open this exact project change';

  @override
  String get managedDashboardChangeActionFailed =>
      'This project change is no longer current. Reload the project overview and try again.';

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
  String get managedContentOpenInStory => 'Open in Story';

  @override
  String get managedContentOpenInStoryDescription =>
      'Continue this Quest or NPC in the complete Story workspace.';

  @override
  String get managedContentOpenInStoryRequiresReopen =>
      'Reopen this project before opening Story.';

  @override
  String get managedContentOpenInStoryFailed =>
      'Story could not be opened. The project was not changed.';

  @override
  String get managedStoryWorkbenchActionFailed =>
      'Could not open this editor. Please try again.';

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
  String managedNpcDraftSaved(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision. It remains build-blocked, runtime-unqualified, and is not spawned.';
  }

  @override
  String get managedNpcOpeningRecipeTitle => 'Character + first greeting';

  @override
  String get managedNpcOpeningRecipeDescription =>
      'Recommended: create a project-only Character draft, then write and insert its first localized greeting. This uses two project checkpoints and does not create a playable conversation or spawn.';

  @override
  String get managedNpcOpeningRecipeIntroduction =>
      'This guided flow first saves the Character draft, then opens its first greeting line. If you stop after step 1, the draft stays saved. It does not create dialog logic, runtime behavior, a spawn, or change the game or save files.';

  @override
  String get managedNpcOpeningRecipeStart => 'Start guided Character';

  @override
  String get managedNpcOpeningGreetingTitle => 'Step 2 of 2: First greeting';

  @override
  String get managedNpcOpeningGreetingIntroduction =>
      'Write the first localized greeting line for this Character draft. Saving creates the line and its text, then inserts it at the start of the draft\'s greeting list. It does not add choices, conditions, effects, or playable conversation behavior.';

  @override
  String managedNpcOpeningRecipePartial(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision; no greeting was added. Continue in Story > Dialog & Voice.';
  }

  @override
  String get managedNpcOpeningRecipeFailed =>
      'The guided Character could not be started. The exact project checkpoint is unchanged; game and save files were not changed.';

  @override
  String get managedNpcOpeningRecipeStopped =>
      'The guided flow stopped because its exact project checkpoint or publication could not be verified. No further step will run automatically; inspect Story and continue manually.';

  @override
  String get managedNpcOpeningRecipeRequiresReopen =>
      'The guided flow could not safely continue. Reopen this project and inspect Story before retrying or continuing manually.';

  @override
  String managedNpcOpeningRecipeComplete(int projectRevision) {
    return 'Character draft and first greeting saved in project revision $projectRevision. Draft only: no playable conversation or spawn was created; game and save files were not changed.';
  }

  @override
  String get managedActionNewQuestTitle => 'Nuova missione';

  @override
  String get managedActionNewQuestDescription =>
      'Crea una bozza di missione offline con obiettivi e identità principali verificate.';

  @override
  String get managedQuestOpeningRecipeTitle => 'Missione + prima battuta';

  @override
  String get managedQuestOpeningRecipeDescription =>
      'Consigliato: crea una bozza di missione, quindi scrivi e inserisci la prima battuta localizzata. Questo flusso usa due punti di controllo del progetto e non crea un dialogo giocabile.';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'Questo flusso guidato salva prima la missione e poi apre la sua prima battuta. Se interrompi dopo il passaggio 1, la missione resta salvata. Non crea un dialogo giocabile e non modifica né il gioco né i salvataggi.';

  @override
  String get managedQuestOpeningRecipeStart => 'Avvia missione guidata';

  @override
  String get managedQuestOpeningLineTitle =>
      'Passaggio 2 di 2: prima battuta di dialogo';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'Scrivi la prima battuta localizzata di questa missione. Il salvataggio crea la battuta e il relativo testo, quindi la inserisce all’inizio della trascrizione della missione.';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'Missione salvata nella revisione $projectRevision del progetto. Preparazione della prima battuta…';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'Missione salvata nella revisione $projectRevision del progetto; non è stata aggiunta alcuna prima battuta. Continua in Storia > Dialoghi e voce.';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'Impossibile avviare la missione guidata. Non è stata pubblicata alcuna modifica del progetto.';

  @override
  String get managedQuestOpeningRecipeStopped =>
      'Il flusso guidato si è interrotto perché lo stato corrente esatto del progetto è cambiato. Nessun altro passaggio verrà eseguito automaticamente; controlla Storia e continua manualmente.';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'Il flusso guidato non ha potuto continuare in sicurezza. Riapri questo progetto e controlla Storia prima di riprovare o continuare manualmente.';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'Missione e prima battuta salvate nella revisione $projectRevision del progetto. Solo bozza: non è stato creato alcun dialogo giocabile e non sono stati modificati il gioco o i salvataggi.';
  }

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
  String get managedItemsBundledReferenceBadge => 'Bundled reference';

  @override
  String get managedItemsBundledReferenceBoundary =>
      'Read-only reference shipped with Mod Studio. It has not been refreshed or generation-qualified against your configured game installation.';

  @override
  String get managedItemsNoKnownFields =>
      'No modeled scalar fields are available for this item.';

  @override
  String get managedItemsCategorySpecial => 'Special';

  @override
  String get managedItemsCategoryArmor => 'Armor';

  @override
  String get managedItemsExactSchemaBadge => 'Exact project schema';

  @override
  String get managedItemsEditableBadge => 'Managed edit';

  @override
  String get managedItemsBuildPendingBadge => 'Build support pending';

  @override
  String get managedItemsInvalidNumber => 'Enter a valid number.';

  @override
  String managedItemsNumberOutsideNativeRange(String minimum, String maximum) {
    return 'Enter a value from $minimum to $maximum.';
  }

  @override
  String get managedItemsAuthoringBoundary =>
      'Changes are saved only to this managed project. This editor does not write to the game or a save. Item bundle build is not available yet.';

  @override
  String managedItemsCurrentChanges(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changed fields',
      one: '1 changed field',
      zero: 'No item changes',
    );
    return '$_temp0';
  }

  @override
  String get managedItemsChangeField => 'Change this field';

  @override
  String get managedItemsUseGameDefault => 'Use game default';

  @override
  String get managedItemsSaveChanges => 'Save item changes';

  @override
  String get managedItemsRevertItem => 'Revert item to game defaults';

  @override
  String get managedItemsClearChanges => 'Clear all item changes';

  @override
  String get managedItemsNoUnsavedChanges => 'No unsaved changes.';

  @override
  String managedItemsSaved(int revision) {
    return 'Item changes saved in project revision $revision.';
  }

  @override
  String get managedItemsSaveStale =>
      'The project or item catalog changed. Nothing was saved. Reload the current item data before editing again.';

  @override
  String get managedItemsSaveRequiresReopen =>
      'The project checkpoint can no longer be verified safely. Nothing was saved. Use project recovery, or close and reopen the project.';

  @override
  String get managedItemsSaveNoChanges =>
      'There is no current item change to save. Reload the item data to continue.';

  @override
  String get managedItemsSaveUnsupported =>
      'This change no longer fits the current safe item schema. Nothing was saved. Reload the item data before continuing.';

  @override
  String get managedItemsSaveUnexpected =>
      'Item changes could not be saved safely. Nothing was changed. Reopen the project and try again.';

  @override
  String get managedItemsReloadDiscardDraft =>
      'Reload item data and discard this draft';

  @override
  String get managedItemsCatalogLoadTitle => 'Items are unavailable';

  @override
  String get managedItemsCatalogStale =>
      'The project or exact item catalog changed before the item data could be loaded. Nothing was changed.';

  @override
  String get managedItemsCatalogRequiresReopen =>
      'The exact project checkpoint can no longer be verified safely. Recover the project, or close and reopen it, before editing items.';

  @override
  String get managedItemsCatalogUnsupported =>
      'This project contains item data that the current exact game schema cannot edit safely. Nothing was changed.';

  @override
  String get managedItemsCatalogLoadUnexpected =>
      'The item data could not be loaded safely. Nothing was changed. Try loading it again.';

  @override
  String get managedItemsCatalogReload => 'Reload item data';

  @override
  String get managedItemsUnsupportedSchema =>
      'This item change no longer matches the current safe catalog or field schema. You can still revert the whole item.';

  @override
  String get managedItemsDefaultUnknown => 'Game default not recorded';

  @override
  String managedItemsGameDefault(String value) {
    return 'Game default: $value';
  }

  @override
  String get managedItemsModValue => 'Mod value';

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
  String get projectExportActionTitle => 'Create project backup…';

  @override
  String get projectExportActionDescription =>
      'Write an exact restorable backup of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before creating a project backup.';

  @override
  String get projectExportDialogTitle => 'Create project backup';

  @override
  String get projectExportPortableCopyTitle =>
      'Restorable Mod Studio project backup';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. It can be restored into a new project folder later; the open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This backup is not a playable mod, build, deployment, or runtime qualification. Creating it does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'A restore preserves this project\'s identity and history. Use Clone or Save As for a separate project identity when those workflows become available.';

  @override
  String get projectExportFileNameLabel => 'New project-backup file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new backup file name ending in .goremod.';

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
  String get projectExportSubmit => 'Create backup';

  @override
  String get projectExportExporting => 'Creating backup…';

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
      'Enter a new project-backup file name.';

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
      'The project changed before backup creation started. No output was created. Close this window and open Create project backup again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot create exact restorable backups. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project backup could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Backup creation stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'Backup creation did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed backup does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact restorable project backup was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact restorable project backup was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

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
  String get projectRestoreActionTitle => 'Restore project backup…';

  @override
  String get projectRestoreActionDescription =>
      'Verify an exact .goremod backup, restore it into a new folder, and open that project safely.';

  @override
  String get projectRestoreDialogTitle => 'Restore project backup';

  @override
  String get projectRestoreNoticeTitle => 'Restore into a new project folder';

  @override
  String get projectRestoreNoticeDescription =>
      'Choose a restorable Mod Studio .goremod backup. Studio verifies the complete archive before creating a new project folder and preserves the backed-up project identity and history.';

  @override
  String get projectRestoreCapabilityBoundary =>
      'Restore does not build, deploy, launch, or qualify the mod at runtime. It does not read or change the game or any save.';

  @override
  String get projectRestoreChooseBackup => 'Choose backup file';

  @override
  String get projectRestoreNoBackup => 'No verified backup selected';

  @override
  String get projectRestoreInspecting => 'Verifying backup…';

  @override
  String get projectRestoreVerified =>
      'This exact V2 project backup is complete and restorable.';

  @override
  String get projectRestoreSource => 'Backup file';

  @override
  String get projectRestoreProjectRevision => 'Project revision';

  @override
  String get projectRestoreArchiveBytes => 'Archive bytes';

  @override
  String get projectRestoreStoreObjects => 'Stored project objects';

  @override
  String get projectRestoreInvalidSource =>
      'The selected file is not a valid exact project backup. Nothing was created.';

  @override
  String get projectRestoreInspectionFailed =>
      'The backup could not be verified completely. Nothing was created.';

  @override
  String get projectRestoreUnavailable =>
      'Exact project restore is unavailable on this system. Nothing was created.';

  @override
  String get projectRestoreChooseDestinationParent => 'Choose parent folder';

  @override
  String get projectRestoreNoDestinationParent => 'No parent folder selected';

  @override
  String get projectRestoreFolderNameLabel => 'New project folder name';

  @override
  String get projectRestoreFolderNameHelper =>
      'Studio creates this new folder; it must not already exist.';

  @override
  String get projectRestoreNewFolder => 'New project folder';

  @override
  String get projectRestoreFolderNameRequired =>
      'Enter a new project folder name.';

  @override
  String get projectRestoreFolderNameTooLong => 'The folder name is too long.';

  @override
  String get projectRestoreFolderNameInvalid =>
      'Use one ordinary folder name without path separators, control characters, a trailing dot, or a trailing space.';

  @override
  String get projectRestoreFolderNameReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get projectRestoreDestinationExists =>
      'That destination already exists. Choose a new folder name; existing content is never overwritten.';

  @override
  String get projectRestoreDestinationLink =>
      'The new project destination is a link. Choose a different folder name.';

  @override
  String get projectRestoreDestinationInvalid =>
      'The destination was rejected before a project receipt was created. Nothing was opened. Choose a different new folder after verifying the backup again.';

  @override
  String get projectRestoreInspectionExpired =>
      'The backup changed after verification. Nothing was opened. Verify the backup again before choosing another destination.';

  @override
  String get projectRestoreMaterializationFailed =>
      'Restore did not return a verified project receipt. Nothing was opened. Do not reuse this attempt; inspect the chosen destination before starting again.';

  @override
  String projectRestorePublicationUncertain(String destination) {
    return 'Studio cannot prove whether the project folder ‘$destination’ was published. Nothing was opened. Do not retry this restore; inspect that destination first.';
  }

  @override
  String get projectRestoreStale =>
      'This restore window is no longer current. Nothing was opened. If materialization had started, inspect the chosen destination before trying anything else.';

  @override
  String get projectRestoreCancel => 'Cancel';

  @override
  String get projectRestoreClose => 'Close';

  @override
  String get projectRestoreSubmit => 'Restore and open';

  @override
  String get projectRestoreRestoring => 'Restoring…';

  @override
  String get projectRestoreSucceeded =>
      'The exact project backup was restored into the new folder.';

  @override
  String get projectRestoreSucceededCleanupWarning =>
      'The exact project backup was restored, but private temporary cleanup was incomplete. The restored project is valid; do not repeat the restore.';

  @override
  String get projectRestoreOpened => 'Project backup restored and opened.';

  @override
  String get projectRestoreOpenedCleanupWarning =>
      'Project backup restored and opened. Private temporary cleanup was incomplete; do not repeat the restore.';

  @override
  String get projectRestoreOpening => 'Opening the restored project safely…';

  @override
  String projectRestoreOpenFailed(String destination) {
    return 'The project folder ‘$destination’ was restored, but Studio could not prove it safe to open. Any previously open project remains current; otherwise no project was opened. Do not repeat the restore; inspect or open the restored folder separately.';
  }

  @override
  String get projectRestoreCandidateCleanupWarning =>
      'No project was adopted. Studio could not completely clean up the rejected candidate session. Restart Mod Studio before opening the restored destination manually.';

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

  @override
  String get managedVoiceSlotRemoveAction => 'Remove empty Voice setup…';

  @override
  String get managedVoiceSlotRemoveDialogTitle => 'Remove empty Voice setup?';

  @override
  String managedVoiceSlotRemoveDialogSummary(String line, String locale) {
    return 'Remove the empty $locale Voice setup from $line?';
  }

  @override
  String get managedVoiceSlotRemoveRetention =>
      'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.';

  @override
  String get managedVoiceSlotRemoveTargetWarning =>
      'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.';

  @override
  String get managedVoiceSlotRemoveRecreate =>
      'You can add a new take later; the required Voice setup will then be created again automatically.';

  @override
  String get managedVoiceSlotRemoveCancel => 'Keep setup';

  @override
  String get managedVoiceSlotRemoveConfirm => 'Remove setup';

  @override
  String get managedVoiceSlotRemoveSuccess =>
      'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.';

  @override
  String get managedVoiceSlotPlanSuccess =>
      'Recording planned. An empty Voice setup was added for this line and language. No audio, game file, or save was changed; build and runtime remain unqualified.';

  @override
  String get managedVoiceSlotRemoveStale =>
      'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.';

  @override
  String get managedVoiceSlotRemoveRequiresReopen =>
      'Reopen the managed project before removing this Voice setup.';

  @override
  String get managedVoiceSlotRemoveSavedUnconfirmed =>
      'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.';

  @override
  String get managedVoiceSlotRemoveSavedReloadFailed =>
      'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.';

  @override
  String managedVoiceSlotRemoveFailed(String error) {
    return 'The empty Voice setup could not be removed: $error';
  }

  @override
  String get managedVoiceSlotRemoveReloadConfirmed =>
      'Saved empty Voice setup removal confirmed from the latest project.';

  @override
  String get managedVoicePreviewTooltip => 'Preview selected local Ogg';

  @override
  String get managedVoicePreviewOpened =>
      'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.';

  @override
  String managedVoicePreviewFailed(String error) {
    return 'The local recording preview could not be opened: $error';
  }

  @override
  String get managedStoryWorkbenchEditNpcProfile => 'Edit name & archetype';

  @override
  String get managedStoryWorkbenchNpcDraftSetupTitle => 'Write this Character';

  @override
  String get managedStoryWorkbenchNpcDraftSetupDescription =>
      'This view tracks the exact Character details and first authored greeting as two project steps in the current revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsTitle =>
      '1. Character details';

  @override
  String get managedStoryWorkbenchNpcDraftSetupFirstGreetingTitle =>
      '2. First greeting';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCompleteStatus =>
      'Saved in project';

  @override
  String get managedStoryWorkbenchNpcDraftSetupNextStatus =>
      'Recommended next step';

  @override
  String get managedStoryWorkbenchNpcDraftSetupOpenStatus => 'Still open';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsComplete =>
      'The exact Character name and reviewed archetype parents are present in this project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsUnavailable =>
      'The exact current Character details could not be verified.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupFirstGreetingPending =>
      'Link the first authored greeting in Dialog & Voice.';

  @override
  String
  get managedStoryWorkbenchNpcDraftSetupFirstGreetingDetailsUnavailable =>
      'Text and Voice coverage for the first greeting could not be verified in this exact project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupRecommendedNext =>
      'Recommended next step';

  @override
  String get managedStoryWorkbenchNpcDraftSetupWriteFirstGreeting =>
      'Write first greeting';

  @override
  String get managedStoryWorkbenchNpcDraftSetupReviewDialogVoice =>
      'Review greetings in Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDraftSetupActionUnavailable =>
      'Dialog & Voice is unavailable for this exact project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupBoundary =>
      'Draft setup tracks current authored project content only. A greeting link is not a playable dialog topic and does not prove publication history, build, or runtime behavior.';

  @override
  String managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count authored greeting links',
      one: '1 authored greeting link',
      zero: 'No authored greeting links',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupTextLanguageCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count text languages',
      one: '1 text language',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupVoiceTakeCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Voice takes',
      one: '1 Voice take',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupSelectedVoiceCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count selected Voice takes',
      one: '1 selected Voice take',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchNpcDisplayNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditTitle => 'Edit name & archetype';

  @override
  String get managedNpcProfileEditDescription =>
      'Change the friendly character name or choose another verified structural starting point.';

  @override
  String get managedNpcProfileEditNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditNameHint =>
      'Shown to authors in this project.';

  @override
  String get managedNpcProfileEditArchetypeLabel =>
      'Archetype / base character';

  @override
  String get managedNpcProfileEditArchetypeHelp =>
      'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.';

  @override
  String get managedNpcProfileEditBoundary =>
      'Only the offline project draft changes. The game installation and save games remain unchanged.';

  @override
  String get managedNpcProfileEditLoading => 'Loading current NPC details…';

  @override
  String get managedNpcProfileEditCancel => 'Cancel';

  @override
  String get managedNpcProfileEditClose => 'Close';

  @override
  String get managedNpcProfileEditSave => 'Save changes';

  @override
  String get managedNpcProfileEditSaving => 'Saving…';

  @override
  String get managedNpcProfileEditRetry => 'Retry';

  @override
  String get managedNpcProfileEditLoadFailed =>
      'NPC details and verified archetypes could not be loaded. No files were changed.';

  @override
  String get managedNpcProfileEditCatalogChanged =>
      'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.';

  @override
  String get managedNpcProfileEditCurrentArchetypeUnavailable =>
      'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.';

  @override
  String get managedNpcProfileEditStale =>
      'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.';

  @override
  String get managedNpcProfileEditRequiresReopen =>
      'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.';

  @override
  String get managedNpcProfileEditSaveFailed =>
      'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.';

  @override
  String get managedNpcProfileEditNameRequired => 'Enter a character name.';

  @override
  String get managedNpcProfileEditNameTooLong =>
      'The character name must be at most 256 UTF-8 bytes.';

  @override
  String get managedNpcProfileEditNameControl =>
      'The character name contains an unsupported control character.';

  @override
  String get managedNpcProfileEditReviewSelection =>
      'Review and choose an archetype before saving.';

  @override
  String get managedNpcProfileEditDiscardTitle => 'Discard NPC changes?';

  @override
  String get managedNpcProfileEditDiscardBody =>
      'Your unsaved name and archetype choice will be lost.';

  @override
  String get managedNpcProfileEditKeepEditing => 'Keep editing';

  @override
  String get managedNpcProfileEditDiscard => 'Discard';

  @override
  String managedNpcProfileEditSaved(String name, int revision) {
    return '$name was saved in project revision $revision. It remains an offline, build-blocked draft.';
  }

  @override
  String get managedVoiceBuildReadinessTitle => 'Voice readiness';

  @override
  String get managedVoiceBuildReadinessRefresh => 'Refresh Voice readiness';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Checking exact Voice readiness';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Voice readiness could not be verified for the current project. No build is available from this result.';

  @override
  String get managedVoiceBuildReadinessReadyTitle => 'Voice is ready';

  @override
  String get managedVoiceBuildReadinessBlockedTitle => 'Voice needs attention';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots of $totalSlots Voice slots are ready.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'No bundle was created and deployment was not performed.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Build bundle';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Voice content is ready. Open Build & Release to create the offline bundle.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Voice content is ready. Configure the game installation before creating an offline bundle.';

  @override
  String get managedVoiceBuildReadinessHideBlockers => 'Hide blockers';

  @override
  String managedVoiceBuildReadinessShowBlockers(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Show $count blockers',
      one: 'Show 1 blocker',
    );
    return '$_temp0';
  }

  @override
  String get managedVoiceBuildReadinessWorkflowFailed =>
      'The selected Voice workflow could not be opened. Refresh and try again.';

  @override
  String get managedVoiceBuildReadinessBuildWorkflowFailed =>
      'The Voice build workflow could not be opened.';

  @override
  String managedVoiceBuildReadinessExactRevision(int revision) {
    return 'Exact project revision $revision';
  }

  @override
  String get managedVoiceBuildReadinessResolveTarget => 'Resolve target';

  @override
  String get managedVoiceBuildReadinessManageTakes => 'Manage takes';

  @override
  String get managedVoiceBuildBlockerNoSlots =>
      'No Voice setups exist in this project.';

  @override
  String get managedVoiceBuildBlockerPayloadBudget =>
      'The selected Voice recordings exceed the safe bundle memory budget.';

  @override
  String get managedVoiceBuildBlockerUnresolvedTarget =>
      'Resolve this Voice target.';

  @override
  String get managedVoiceBuildBlockerAmbiguousTarget =>
      'This Voice target is ambiguous.';

  @override
  String get managedVoiceBuildBlockerUnqualifiedAdd =>
      'This target is not a sealed existing-member replacement.';

  @override
  String get managedVoiceBuildBlockerMissingTake =>
      'Select an approved Voice take.';

  @override
  String get managedVoiceBuildBlockerTakeNotApproved =>
      'The selected Voice take is not approved.';

  @override
  String get managedVoiceBuildBlockerCodecUnqualified =>
      'The selected Voice take uses an unsupported codec.';

  @override
  String get managedVoiceBuildBlockerSlotLimit =>
      'This project exceeds the 1024-slot Voice bundle limit.';

  @override
  String get managedVoiceBuildOfflineNotice =>
      'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.';

  @override
  String get managedVoiceBuildNewFolderName => 'New folder name';

  @override
  String get managedVoiceBuildNewFolderHelp =>
      'The bundle must be written to a brand-new child folder.';

  @override
  String get managedVoiceBuildChooseParent => 'Choose parent folder';

  @override
  String get managedVoiceBuildNoParentSelected => 'No parent folder selected';

  @override
  String get managedVoiceBuildNewOutput => 'New output';

  @override
  String get managedVoiceBuildOfflineBundle => 'Build offline bundle';

  @override
  String get managedVoiceBuildParentInspectFailed =>
      'The parent folder could not be inspected safely. No build or deployment was attempted.';

  @override
  String get managedVoiceBuildChooseExistingParent =>
      'Choose an existing parent folder.';

  @override
  String get managedVoiceBuildTargetSymlink =>
      'The target path is a symlink. Choose a different new folder name.';

  @override
  String get managedVoiceBuildTargetExists =>
      'The target already exists. Choose a different new folder name.';

  @override
  String get managedVoiceBuildRequiresReopen =>
      'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.';

  @override
  String get managedVoiceBuildStaleCheckpoint =>
      'The managed project changed while this window was open. Close this build window and open it again from the current project.';

  @override
  String get managedVoiceBuildFailed =>
      'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.';

  @override
  String get managedVoiceBuildPlanFailed =>
      'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.';

  @override
  String get managedVoiceBuildParentAbsolute =>
      'Choose an absolute existing parent folder.';

  @override
  String get managedVoiceBuildParentSymlink =>
      'The selected parent is a symlink. Choose a real existing folder.';

  @override
  String get managedVoiceBuildFolderRequired => 'Enter a new folder name.';

  @override
  String get managedVoiceBuildFolderWhitespace =>
      'The folder name cannot start or end with whitespace.';

  @override
  String get managedVoiceBuildFolderTooLong => 'The folder name is too long.';

  @override
  String get managedVoiceBuildFolderPortable =>
      'Use one portable folder name without separators or reserved characters.';

  @override
  String get managedVoiceBuildFolderWindowsReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get managedVoiceBuildExecutableUnavailable =>
      'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildExecutableMismatch =>
      'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameUnavailable =>
      'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreGameAlias =>
      'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameOutputAlias =>
      'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreOutputAlias =>
      'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputUnavailable =>
      'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.';

  @override
  String get managedVoiceBuildOutputFailed =>
      'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPromotionFailed =>
      'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildCleanupFailed =>
      'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPublicationUnconfirmed =>
      'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreRootChanged =>
      'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameRootChanged =>
      'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputRootChanged =>
      'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildVerifyFailed =>
      'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildBundleInvalid =>
      'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildInputInvalid =>
      'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildResponseLimit =>
      'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.';

  @override
  String get managedVoiceBuildBuiltTitle => 'Sealed Voice bundle built';

  @override
  String get managedVoiceBuildOfflineReceipt =>
      'Offline receipt only. Deployment was not performed.';

  @override
  String get managedVoiceBuildBasisRevision => 'Basis project revision';

  @override
  String get managedVoiceBuildOutputLabel => 'Output';

  @override
  String get managedVoiceBuildArchiveEdits => 'Archive edits';

  @override
  String get managedVoiceBuildBundleFiles => 'Bundle files';

  @override
  String get managedVoiceBuildSealedBytes => 'Sealed bytes';

  @override
  String get managedVoiceBuildBundleSha256 => 'Bundle SHA-256';

  @override
  String get managedVoiceBuildParentPickerTitle => 'Choose Voice bundle parent';

  @override
  String managedVoiceBuildBuiltMessage(String output) {
    return 'Sealed Voice bundle built at $output. Deployment was not performed.';
  }

  @override
  String managedVoiceBuildBlockedMessage(int count) {
    return 'Voice build blocked by $count exact requirements. No bundle was created or deployed.';
  }

  @override
  String get managedTextureSetupTitle => 'Choose the game installation';

  @override
  String get managedTextureSetupDescription =>
      'Textures are read from the configured Gothic 1 Remake installation. Nothing is changed in the game or project.';

  @override
  String get managedTextureSetupAction => 'Open Settings';

  @override
  String get managedTextureLoading => 'Loading the installed texture catalog…';

  @override
  String get managedTextureLoadingDescription =>
      'The first exact scan can take several minutes. Mod Studio runs only one scan at a time and queues the latest refresh.';

  @override
  String managedTextureCatalogCount(int count) {
    return '$count installed textures';
  }

  @override
  String managedTextureSearchCount(int matches, int total) {
    return '$matches matches · $total total';
  }

  @override
  String get managedTextureEmptyTitle => 'No textures found';

  @override
  String get managedTextureEmptyDescription =>
      'The exact installed catalog contains no texture entries.';

  @override
  String get managedTextureErrorTitle => 'Texture catalog unavailable';

  @override
  String get managedTextureErrorDescription =>
      'The installed texture catalog could not be loaded for this exact game build.';

  @override
  String get managedTextureRetry => 'Retry';

  @override
  String get managedTextureRefreshTooltip =>
      'Refresh installed texture catalog';

  @override
  String get managedTextureSearchLabel => 'Search textures';

  @override
  String get managedTextureSearchHint => 'Name or Unreal asset path';

  @override
  String get managedTextureClearSearchTooltip => 'Clear texture search';

  @override
  String get managedTextureSelectPrompt =>
      'Select a texture to inspect its original installed image.';

  @override
  String get managedTexturePreviewLoading => 'Extracting the original texture…';

  @override
  String get managedTexturePreviewErrorTitle => 'Preview unavailable';

  @override
  String get managedTexturePreviewErrorDescription =>
      'The original texture could not be extracted from the selected game build.';

  @override
  String get managedTexturePreviewRetry => 'Retry preview';

  @override
  String get managedTextureBackToCatalog => 'Back to textures';

  @override
  String get managedTextureInspectionOnly =>
      'Installed reference · inspect only. This does not edit the project, game installation, or a save.';

  @override
  String get managedTextureInstalledBadge => 'Installed source';

  @override
  String get managedTextureRegularBadge => 'Regular texture';

  @override
  String get managedTextureVirtualBadge => 'Virtual texture';

  @override
  String managedTextureVirtualLayerCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count VT layers',
      one: '1 VT layer',
    );
    return '$_temp0';
  }

  @override
  String get managedTextureMipmappedBadge => 'Mipmapped';

  @override
  String get managedTextureSingleMipBadge => 'Single mip';

  @override
  String get managedTextureReplaceableBadge =>
      'Replacement supported · editing not yet available';

  @override
  String get managedTextureNotReplaceableBadge =>
      'Replacement unavailable · inspect only';

  @override
  String get managedTextureUnknownReplaceabilityBadge =>
      'Replacement not qualified · inspect only';

  @override
  String get managedTextureUnknownFormat => 'Unknown source format';

  @override
  String get managedWorkspaceTextVoiceLabel => 'Testo e voci';

  @override
  String get managedWorkspaceTestReleaseLabel => 'Test e pubblicazione';

  @override
  String get managedTestReleaseTitle => 'Test e pubblicazione';

  @override
  String get managedTestReleaseDescription =>
      'Controlla ogni parte della mod prima di creare file giocabili o installarli.';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      'Nulla viene considerato pronto automaticamente. Un risultato verificato vale solo per questa esatta versione salvata del progetto.';

  @override
  String get managedTestReleaseChecksHeading => 'Controlli del progetto';

  @override
  String get managedTestReleaseReleaseHeading => 'Output giocabile';

  @override
  String get managedTestReleaseStatusNotChecked => 'Non controllato';

  @override
  String get managedTestReleaseStatusChecking => 'Controllo in corso';

  @override
  String get managedTestReleaseStatusChecked => 'Controllato';

  @override
  String get managedTestReleaseStatusNeedsAttention => 'Richiede attenzione';

  @override
  String get managedTestReleaseStatusBlocked => 'Bloccato';

  @override
  String get managedTestReleaseStatusNotAvailable => 'Non disponibile';

  @override
  String get managedTestReleaseStatusAvailable => 'Disponibile';

  @override
  String get managedTestReleaseEvidenceLabel => 'Evidenza';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      'Questo risultato appartiene a un’altra versione del progetto. Esegui di nuovo il controllo.';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      'Esiste un’evidenza, ma questa azione non è ancora collegata nell’area di lavoro corrente.';

  @override
  String get managedTestReleaseProblemsHeading => 'Problemi da risolvere';

  @override
  String get managedTestReleaseVoiceHeading => 'Controllo delle voci';

  @override
  String get managedTestReleaseProjectStructureTitle =>
      'Struttura del progetto';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      'Esamina qui sotto l’elenco attivo dei problemi per controllare i riferimenti e la struttura del progetto gestito.';

  @override
  String get managedTestReleaseProjectStructureAction => 'Esamina i problemi';

  @override
  String get managedTestReleaseScriptsTitle => 'Script';

  @override
  String get managedTestReleaseScriptsDescription =>
      'Esegui una volta il compilatore del gioco per tutti gli script di questa esatta versione salvata del progetto. Il risultato è solo una prova di controllo; l’output viene eliminato.';

  @override
  String get managedTestReleaseScriptsAction => 'Esegui il controllo';

  @override
  String get managedProjectCompilerRetryAction => 'Ripeti il controllo';

  @override
  String get managedProjectCompilerReviewAction =>
      'Vedi risultato / ricontrolla';

  @override
  String get managedProjectCompilerDialogTitle => 'Controlla tutti gli script';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      'Chiudi Gothic 1 Remake prima di iniziare. Mod Studio controlla temporaneamente tutti gli script del progetto con il compilatore del gioco, ripristina l’installazione ed elimina tutto l’output del compilatore. Questo risultato non può creare file giocabili né installare la mod.';

  @override
  String get managedProjectCompilerCloseAction => 'Chiudi';

  @override
  String get managedProjectCompilerNoGame =>
      'Seleziona l’installazione di Gothic 1 Remake nelle Impostazioni prima di eseguire questo controllo.';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      'L’installazione del gioco non è pronta per il controllo. Chiudi il gioco o risolvi l’avviso di ripristino, quindi riprova.';

  @override
  String get managedProjectCompilerCompiled =>
      'Tutti gli script del progetto sono stati accettati per questa esatta versione salvata. L’output del compilatore è stato eliminato.';

  @override
  String get managedProjectCompilerEmpty =>
      'Questa versione salvata non contiene script da compilare. Il risultato vuoto è stato verificato esattamente.';

  @override
  String get managedProjectCompilerRejected =>
      'Il compilatore ha trovato problemi in uno o più script del progetto. Correggi i messaggi qui sotto e riprova.';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      'Il compilatore non è stato avviato. Chiudi il gioco, controlla l’installazione configurata e riprova.';

  @override
  String get managedProjectCompilerDrifted =>
      'Il progetto o i dati del gioco sono cambiati, oppure il controllo finale non era più esatto. Il risultato è stato eliminato; ripeti il controllo per la versione corrente.';

  @override
  String get managedProjectCompilerRequiresReopen =>
      'Questo progetto deve essere chiuso e riaperto prima di un altro controllo esatto.';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      'Non è stato possibile verificare il completamento della pulizia dell’output privato del compilatore o del ripristino esatto dell’installazione del gioco. Ulteriori controlli del compilatore e l’installazione restano bloccati finché un nuovo controllo di sicurezza non riesce.';

  @override
  String get managedProjectCompilerFailed =>
      'Il controllo non è stato completato o verificato. Nessun risultato è stato conservato; riprova quando l’installazione del gioco è pronta.';

  @override
  String get managedProjectCompilerFailureDetails =>
      'Messaggio del compilatore';

  @override
  String get managedProjectCompilerDiagnosticsHeading =>
      'Messaggi del compilatore';

  @override
  String get managedProjectCompilerCaptureCaptured =>
      'I messaggi strutturati del compilatore sono stati acquisiti.';

  @override
  String get managedProjectCompilerCaptureFallback =>
      'Il collegamento diagnostico non era disponibile, quindi è stato usato il normale compilatore del gioco come alternativa.';

  @override
  String get managedProjectCompilerCaptureInvalid =>
      'Non è stato possibile verificare l’acquisizione dei messaggi del compilatore.';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      'Il collegamento diagnostico non era disponibile dopo l’esecuzione; non è stata necessaria una seconda esecuzione.';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed =>
      'Il processo del compilatore non ha confermato la chiusura.';

  @override
  String get managedProjectCompilerCaptureDisabled =>
      'Per questa esecuzione non erano disponibili messaggi strutturati del compilatore.';

  @override
  String get managedProjectCompilerSeverityError => 'Errore';

  @override
  String get managedProjectCompilerSeverityWarning => 'Avviso';

  @override
  String get managedProjectCompilerSeverityNote => 'Nota';

  @override
  String get managedProjectCompilerFileLabel => 'File';

  @override
  String get managedProjectCompilerLineLabel => 'Riga';

  @override
  String get managedProjectCompilerColumnLabel => 'Colonna';

  @override
  String get managedProjectCompilerOmittedDiagnostics =>
      'altri messaggi del compilatore omessi';

  @override
  String get managedTestReleaseVoiceTitle => 'Testo e voci';

  @override
  String get managedTestReleaseVoiceDescription =>
      'Usa qui sotto il controllo delle voci per la versione attualmente salvata del progetto.';

  @override
  String get managedTestReleaseVoiceAction => 'Controlla le voci';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      'I DataAsset preparati sono visibili nei Problemi, ma non esiste ancora un’evidenza completa della build dell’intero progetto.';

  @override
  String get managedTestReleaseDataAssetsAction => 'Esamina i DataAsset';

  @override
  String get managedTestReleasePlayableBuildTitle => 'File giocabili';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      'Crea una build giocabile verificata da questa esatta versione salvata del progetto.';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      'Non esiste ancora un’evidenza esatta della build completa del progetto per questa versione salvata.';

  @override
  String get managedTestReleaseCreatePlayableFilesAction =>
      'Crea file giocabili';

  @override
  String get managedTestReleaseDeploymentTitle => 'Installazione';

  @override
  String get managedTestReleaseDeploymentDescription =>
      'Installa nel gioco configurato una build giocabile verificata con esattezza.';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      'Non esiste ancora un’evidenza esatta di una build distribuibile per questa versione salvata del progetto.';

  @override
  String get managedTestReleaseInstallAction => 'Installa';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return 'Sezione attuale: $section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return 'Progetto $project. Sezione attuale: $section.';
  }

  @override
  String get managedProjectCommandBarUndoLabel => 'Annulla';

  @override
  String get managedProjectCommandBarSearchLabel => 'Cerca';

  @override
  String get managedProjectCommandBarCreateLabel => 'Crea';

  @override
  String get managedProjectCommandBarProblemsLabel => 'Problemi';

  @override
  String get managedProjectCommandBarHistoryLabel => 'Cronologia';

  @override
  String get managedProjectCommandBarSettingsLabel => 'Impostazioni';

  @override
  String get managedProjectCommandBarMoreActionsTooltip =>
      'Altre azioni del progetto';

  @override
  String get managedProjectCommandBarBusyLabel =>
      'Completamento dell’azione del progetto in corso…';

  @override
  String get managedProjectCommandBarBusyDisabledReason =>
      'Attendi che l’azione del progetto in corso sia completata.';
}
