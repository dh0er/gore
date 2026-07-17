// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores de objetos';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimiento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Acciones';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Ítems';

  @override
  String get audioCatUi => 'Interfaz';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Bajo el agua';

  @override
  String get audioCatVision => 'Visiones';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Otros';

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
  String get extractLocalizedText => 'Extraer textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo oscuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Selecciona un objeto para editar sus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Datos del juego: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Datos del juego: incluidos';

  @override
  String get loadGameDataDump => 'Cargar volcado de datos del juego…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json del mod gore-dump';

  @override
  String get useBundledData => 'Usar los datos incluidos';

  @override
  String get alreadyBundled => 'ya incluidos';

  @override
  String get gameDataFileGroupLabel => 'datos del juego';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Cerrar';

  @override
  String get about => 'Acerca de';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores de GORE';

  @override
  String get aboutLicense => 'Distribuido bajo la licencia MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas cuerpo a cuerpo';

  @override
  String get categoryRangedWeapons => 'Armas a distancia';

  @override
  String get categoryAmmunition => 'Munición';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminos de hechizo';

  @override
  String get categoryFoodAndPotions => 'Comida y pociones';

  @override
  String get categoryMiscellaneous => 'Varios';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anillos';

  @override
  String get categoryAnimalTrophies => 'Trofeos de animales';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Objetos de misión';

  @override
  String get categoryKeys => 'Llaves';

  @override
  String get categoryOther => 'Otros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Buscar objetos';

  @override
  String get noItemsMatch => 'Ningún objeto coincide';

  @override
  String failedToLoadCatalog(String error) {
    return 'No se pudo cargar el catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Cambios pendientes ($count)';
  }

  @override
  String get clearAll => 'Borrar todo';

  @override
  String get noPendingOverrides =>
      'No hay cambios pendientes.\nEdita los campos de los objetos para añadir alguno.';

  @override
  String get removeOverride => 'Quitar cambio';

  @override
  String get searchChanges => 'Buscar cambios';

  @override
  String get noChangesMatch => 'Ningún cambio coincide';

  @override
  String get clearSection => 'Borrar este grupo';

  @override
  String get modName => 'Nombre del mod';

  @override
  String get loadDelayLabel => 'Retardo de carga (ms, 0 = inmediato)';

  @override
  String get noFolderSelected => 'Ninguna carpeta seleccionada';

  @override
  String get chooseFolder => 'Elegir carpeta';

  @override
  String get packageAsZip => 'Empaquetar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aquí';

  @override
  String get mustBeNonNegativeInteger => 'Debe ser un entero no negativo';

  @override
  String get extractingLocalizedText =>
      'Extrayendo los textos localizados del juego…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extracción de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Error en la extracción.';

  @override
  String get localizationCacheFileGroupLabel => 'caché de localización';

  @override
  String get extractLocalizedTextQuestion =>
      '¿Extraer los textos localizados del juego?';

  @override
  String get extractLocalizedTextBody =>
      'Los textos localizados del juego aún no se han extraído. ¿Extraerlos ahora desde tu instalación del juego? (opcional)';

  @override
  String get notNow => 'Ahora no';

  @override
  String get extract => 'Extraer';

  @override
  String get validationRequired => 'Obligatorio';

  @override
  String get validationMustBeWholeNumber => 'Debe ser un número entero';

  @override
  String get validationMustBeNumber => 'Debe ser un número';

  @override
  String get validationMustBeFinite => 'Debe ser un número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Debe ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Debe ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Debe ser true o false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Debe ser uno de: $options';
  }

  @override
  String get modNameRequired => 'Obligatorio';

  @override
  String get modNameControlCharacters =>
      'No debe contener caracteres de control';

  @override
  String get modNamePathSeparators => 'No debe contener separadores de ruta';

  @override
  String get modNameNotAFolderName => 'Nombre de carpeta no válido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID extraídos en $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Hay un loadout del mod-manager activo. Haz primero el undeploy en gore-manager.';

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
      'El nuevo proyecto está abierto, pero no se pudo limpiar por completo la sesión del proyecto anterior. No se volverá a intentar la limpieza. Reinicia Mod Studio antes de volver a abrir el proyecto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Nuevo proyecto de mod gestionado…';

  @override
  String get projectNewLegacy => 'Nuevo proyecto legacy';

  @override
  String get projectCreateGamePathRequired =>
      'Configura la ruta de Gothic 1 Remake en Ajustes antes de crear un proyecto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Crear aquí el proyecto de mod gestionado';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Proyecto de mod gestionado $projectId creado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'No se pudo crear el proyecto de mod gestionado: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Crear un proyecto de mod';

  @override
  String get projectCreateNameLabel => 'Nombre del proyecto';

  @override
  String get projectCreateNameHelper =>
      'El nombre que se muestra en Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versión';

  @override
  String get projectCreateVersionHelper => 'Una versión inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Tu nombre o el de tu equipo de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edición';

  @override
  String get projectCreateLocalesHelper =>
      'Etiquetas canónicas separadas por comas, por ejemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Esto crea un proyecto offline gestionado y vacío. No compila, instala ni ejecuta un mod, y no modifica los archivos del juego ni las partidas guardadas.';

  @override
  String get projectCreateSubmit => 'Crear proyecto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label es obligatorio.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label no puede empezar ni terminar con espacios.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label no puede contener caracteres de control.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contiene texto no válido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label supera el límite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Introduce al menos un idioma de edición.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Elimina la entrada vacía de idioma.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Usa como máximo $maxLocales idiomas de edición.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'La configuración regional «$locale» debe ser ASCII y tener una longitud limitada.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'La configuración regional «$locale» necesita un idioma en minúsculas de 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'La configuración regional «$locale» contiene un segmento no válido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'La configuración regional «$locale» no es canónica; usa «$canonical».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Resumen';

  @override
  String get managedWorkspaceContentLabel => 'Contenido';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Este mod';

  @override
  String get managedWorkspaceHomeLabel => 'Inicio';

  @override
  String get managedWorkspaceStoryLabel => 'Historia';

  @override
  String get managedWorkspaceWorldLabel => 'Mundo';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localización y voces';

  @override
  String get managedWorkspaceValidateTestLabel => 'Validar y probar';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Compilar y publicar';

  @override
  String get managedWorkspaceHistoryLabel => 'History';

  @override
  String get managedWorkspaceSettingsExpertLabel => 'Ajustes y modo experto';

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
  String get managedSectionStoryDescription => 'NPC, misiones y diálogos.';

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
      'La colocación en el mundo y sus flujos de trabajo están planificados.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Escribe y traduce diálogos del proyecto en un solo lugar y continúa después con las voces.';

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
      'You changed this project text. Switching now would discard those edits.';

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
  String get managedSectionValidateTestDescription =>
      'Verifica la integridad exacta del proyecto y sus puntos de control; no implica una prueba en ejecución.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Los paquetes de voces están disponibles; las compilaciones jugables completas y el despliegue no lo están.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Los ajustes están disponibles; las herramientas expertas aún no están integradas.';

  @override
  String get managedSectionStatusHeading => 'Estado';

  @override
  String get managedSectionActionsHeading => 'Acciones';

  @override
  String get managedCapabilityAvailable => 'Disponible';

  @override
  String get managedCapabilityPartial => 'Parcial';

  @override
  String get managedCapabilityPlanned => 'Planificado';

  @override
  String get managedCapabilityUnavailable => 'No disponible';

  @override
  String get managedProjectSubtitle =>
      'Espacio de autoría sin conexión con la versión actual exacta';

  @override
  String get managedProjectLandingTitle =>
      'Espacio de trabajo de proyectos gestionados';

  @override
  String get managedProjectLandingDescription =>
      'Usa el nuevo flujo de Inicio, Contenido, Historia, Voz, validación y publicación en un único proyecto gestionado.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Herramientas de compatibilidad heredadas';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Las pestañas de abajo contienen herramientas antiguas de reemplazo directo. Seguirán disponibles mientras ampliamos el espacio de trabajo de proyectos gestionados.';

  @override
  String get managedProjectTechnicalDetails => 'Detalles técnicos del proyecto';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Vuelve a abrir el proyecto gestionado antes de leer su contenido.';

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
  String get managedDashboardUntitledProject => 'Proyecto sin título';

  @override
  String get managedDashboardDraftStatus => 'Borrador';

  @override
  String get managedDashboardProjectVersion => 'Versión';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'No especificado';

  @override
  String get managedDashboardContentCounts => 'Contenido del proyecto';

  @override
  String get managedDashboardNpcDrafts => 'Borradores de PNJ';

  @override
  String get managedDashboardQuestDrafts => 'Borradores de misiones';

  @override
  String get managedDashboardDialogLines => 'Líneas de diálogo';

  @override
  String get managedDashboardVoiceTakes => 'Tomas de voz';

  @override
  String get managedDashboardAssets => 'Recursos';

  @override
  String get managedDashboardUnresolvedReferences => 'Referencias sin resolver';

  @override
  String get managedDashboardReadiness => 'Qué funciona ahora';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Edición sin conexión disponible';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Crea y edita contenido de proyecto compatible sin modificar la instalación del juego ni los archivos de guardado.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Compilación general de mods no disponible';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Solo se pueden compilar paquetes de voz sellados sin conexión; todavía no se puede compilar un mod completo y jugable.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Ejecución aún no verificada';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio aún no ha comprobado este contenido del proyecto dentro del juego en ejecución.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Integridad de las referencias';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Este recuento solo comprueba las referencias del proyecto; no confirma que esté listo para compilarse o ejecutarse.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Se requiere configurar el juego';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configura la instalación de Gothic 1 Remake en Ajustes antes de usar acciones que necesiten datos verificados del juego instalado.';

  @override
  String get managedDashboardCreateHeading => 'Crear';

  @override
  String get managedDashboardToolsHeading => 'Herramientas del proyecto';

  @override
  String get managedDashboardLoading => 'Cargando el resumen del proyecto';

  @override
  String get managedDashboardLoadError => 'Resumen del proyecto no disponible';

  @override
  String get managedDashboardLoadErrorDescription =>
      'No se pudo cargar el resumen verificado del proyecto. El contenido del proyecto no se modificó.';

  @override
  String get managedDashboardRetry => 'Reintentar';

  @override
  String get managedActionNewNpcTitle => 'Nuevo PNJ';

  @override
  String get managedActionNewNpcDescription =>
      'Crea un borrador de PNJ sin conexión y de alcance limitado a partir de datos verificados del juego instalado.';

  @override
  String get managedActionNewQuestTitle => 'Nueva misión';

  @override
  String get managedActionNewQuestDescription =>
      'Crea un borrador de misión sin conexión con objetivos e identidades superiores verificadas.';

  @override
  String get managedActionNewDialogLineTitle => 'Añadir línea de diálogo';

  @override
  String get managedActionNewDialogLineDescription =>
      'Escribe texto localizado del proyecto o vincula un texto sin usar que ya esté en este proyecto. Esto no crea un tema de diálogo jugable.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Línea de diálogo guardada en la revisión $projectRevision del proyecto. No se modificaron el juego ni las partidas guardadas.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Escribe una nueva línea de diálogo localizada o vincula texto que ya pertenece a este proyecto.';

  @override
  String get managedDialogLineBoundary =>
      'Solo se modifican archivos del proyecto. Esto no crea un tema de AngelScript ni un diálogo jugable, y nunca modifica la instalación del juego ni las partidas guardadas. El campo de hablante es solo una etiqueta; no vincula ningún PNJ.';

  @override
  String get managedDialogLineCreateMode => 'Escribir texto nuevo';

  @override
  String get managedDialogLineReuseMode => 'Usar texto del proyecto';

  @override
  String get managedDialogLineNameLabel => 'Nombre de la línea';

  @override
  String get managedDialogLineNameHint => 'Saludo en la entrada de la mina';

  @override
  String get managedDialogLineSpeakerLabel =>
      'Etiqueta del hablante (opcional)';

  @override
  String get managedDialogLineSpeakerHint => 'Por ejemplo, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Idioma';

  @override
  String get managedDialogLineTextLabel => 'Texto del diálogo';

  @override
  String get managedDialogLineReuseSearch =>
      'Buscar texto del proyecto sin usar';

  @override
  String get managedDialogLineNoReusableText =>
      'No hay texto de proyecto sin usar y estructuralmente válido que se pueda vincular. Escribe texto nuevo.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Preparar este idioma para Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Crea un espacio Voice vacío y sin resolver en el proyecto. No añade ni despliega ninguna grabación.';

  @override
  String get managedDialogLineCancel => 'Cancelar';

  @override
  String get managedDialogLineSave => 'Guardar en el proyecto';

  @override
  String get managedDialogLineSaving => 'Guardando…';

  @override
  String get managedDialogLineLoading =>
      'Leyendo el contenido exacto del proyecto…';

  @override
  String get managedDialogLineLoadFailed =>
      'No se pudo leer el contenido actual exacto del proyecto. No se modificó nada.';

  @override
  String get managedDialogLineRetry => 'Reintentar';

  @override
  String get managedDialogLineStale =>
      'El proyecto cambió mientras esta ventana estaba abierta. Ciérrala y vuelve a intentarlo desde el proyecto actual.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Ya no se puede verificar de forma segura el proyecto actual. Cierra esta ventana y vuelve a abrir el proyecto gestionado.';

  @override
  String get managedDialogLineInvalidInput =>
      'Revisa la entrada resaltada del proyecto y elige una opción actual exacta.';

  @override
  String get managedDialogLineSaveFailed =>
      'No se pudo guardar de forma segura la línea de diálogo. No se modificaron el juego ni las partidas guardadas.';

  @override
  String get managedDialogLineDone => 'Listo';

  @override
  String get managedDialogLineAddRecording => 'Añadir grabación';

  @override
  String get managedActionAddVoiceTakeTitle => 'Añadir toma de voz';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importa una grabación Ogg Vorbis en este proyecto sin desplegarla.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Gestionar tomas de voz';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Revisa las tomas y selecciona grabaciones aprobadas para los espacios de voz.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Resolver destino de voz';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Asocia los espacios de voz del proyecto con miembros exactos de los archivos instalados sin modificar el juego.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Compilar paquete de voz';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Compila un paquete sellado sin conexión con miembros existentes; no se realiza ningún despliegue.';

  @override
  String get managedActionDataAssetsTitle => 'Ediciones de DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspecciona paquetes instalados y prepara en el proyecto ediciones verificadas de valores de ancho fijo.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Explora el contenido exacto del proyecto y sus referencias resueltas o sin resolver.';

  @override
  String get managedActionSettingsTitle => 'Ajustes';

  @override
  String get managedActionSettingsDescription =>
      'Configura la instalación de Gothic 1 Remake y las preferencias de Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'El proyecto $projectId se creó de forma segura, pero no se abrió la configuración inicial. El proyecto vacío válido sigue activo.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Se creó el proyecto $projectId, pero Mod Studio no puede verificar el resultado del inicio. Vuelve a abrir el proyecto administrado antes de continuar; el juego y las partidas no cambiaron.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Se creó el proyecto $projectId. No se añadió el inicio de NPC, por lo que el proyecto vacío válido sigue activo.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Inicio de NPC guardado en la revisión $projectRevision. Sigue bloqueado para compilación, no está validado en ejecución y no se genera.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Se creó el proyecto $projectId. No se añadió el inicio de misión, por lo que el proyecto vacío válido sigue activo.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Inicio de misión guardado en la revisión $projectRevision. Sigue bloqueado para compilación y no está validado en ejecución.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Inicio del proyecto';

  @override
  String get projectStarterPrompt => '¿Cómo quieres empezar?';

  @override
  String get projectStarterWriteBoundary =>
      'Elegir un inicio no escribe nada. El proyecto solo se crea después de enviar este formulario y elegir una carpeta vacía.';

  @override
  String get projectStarterEmptyTitle => 'Proyecto vacío';

  @override
  String get projectStarterEmptyDescription =>
      'Crea solo el proyecto administrado. Añade contenido cuando quieras.';

  @override
  String get projectStarterNpcDraftTitle => 'Borrador de NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Crea primero el proyecto vacío y abre después la configuración guiada del borrador de NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Borrador de misión';

  @override
  String get projectStarterQuestDraftDescription =>
      'Crea primero el proyecto vacío y abre después la configuración guiada del borrador de misión.';

  @override
  String get projectStarterPartialOutcome =>
      'Si cancelas la configuración guiada de NPC o misión, o falla el borrador, queda un proyecto vacío válido. La selección no escribe en el juego ni en una partida guardada.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Explorar';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Ediciones verificadas';

  @override
  String get managedContentScopeBaseGameLabel => 'Juego base';

  @override
  String get managedContentScopeInstalledLabel => 'Instalado';

  @override
  String get managedBaseGameBrowserTitle =>
      'Puntos de partida compatibles del juego base';

  @override
  String get managedBaseGameBrowserDescription =>
      'Explora pruebas exactas del juego instalado que Mod Studio puede inspeccionar o usar como punto de partida seguro para un borrador. No es un catálogo completo del contenido original.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Leyendo pruebas exactas del juego base…';

  @override
  String get managedBaseGameBrowserRefresh => 'Leer un catálogo exacto nuevo';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Buscar contenido compatible del juego base';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Misiones';

  @override
  String get managedBaseGameBrowserNpcSectionTitle =>
      'Puntos de partida de NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle =>
      'Puntos de partida de misión';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Arquetipos de NPC solo para inspección';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Busca para incluir más pruebas de NPC con enlace estático. Esas filas no permiten crear un borrador.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Ningún resultado compatible del juego base coincide con la búsqueda.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Pruebas del juego base no disponibles';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'No se pudo leer el catálogo compatible exacto. No se modificó ningún archivo del proyecto, juego o partida.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Borrador sin conexión compatible';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Solo inspección';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'Usar como inicio de NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Usar como inicio de misión';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Definición de aparición';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint del actor';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Se muestran las primeras 100 coincidencias solo para inspección. Refina la búsqueda para obtener resultados más precisos.';

  @override
  String get managedInstalledBrowserLoading =>
      'Leyendo el inventario exacto de paquetes instalados…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count paquetes instalados candidatos';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count paquetes instalados candidatos — resultado parcial';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Se leyeron los metadatos del directorio y la instantánea instalada se mantuvo exacta.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Faltaban metadatos de algunos paquetes o no eran canónicos; los resultados sirven para descubrir contenido, pero no están completos.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Este ámbito solo muestra metadatos de paquetes DataAsset instalados. Inspeccionar o copiar una ruta no autoriza compilación, despliegue, ejecución ni escritura en el juego.';

  @override
  String get managedInstalledBrowserRefresh =>
      'Leer una instantánea exacta nueva';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Buscar DataAssets instalados';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nombre del recurso o ruta /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Escribe un nombre de recurso o una ruta /Game para buscar.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Ningún DataAsset instalado coincide';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Prueba otro nombre de recurso o una ruta /Game más amplia.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Se muestran las primeras 100 coincidencias. Refina la búsqueda para acotar la instantánea exacta.';

  @override
  String get managedInstalledBrowserKindBadge => 'Paquete DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Solo metadatos';

  @override
  String get managedInstalledBrowserOpenInspector =>
      'Inspeccionar paquete exacto';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventario de paquetes instalados no disponible';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'No se pudo leer la instantánea instalada exacta. No se modificó ningún archivo del proyecto, juego o partida.';

  @override
  String get managedGlobalSearchScopeLabel => 'Buscar en todo';

  @override
  String get managedGlobalSearchTitle => 'Buscar en todo el contenido';

  @override
  String get managedGlobalSearchLabel =>
      'PNJ, misión, línea, recurso, ID o ruta /Game';

  @override
  String get managedGlobalSearchAction => 'Buscar';

  @override
  String get managedGlobalSearchClear => 'Borrar';

  @override
  String get managedGlobalSearchPrompt =>
      'Introduce una búsqueda para consultar las tres fuentes de forma independiente.';

  @override
  String get managedGlobalSearchNoResults =>
      'No hay coincidencias en esta fuente.';

  @override
  String get managedGlobalSearchLoading => 'Leyendo la fuente exacta…';

  @override
  String get managedGlobalSearchFailed => 'No se pudo leer esta fuente.';

  @override
  String get managedGlobalSearchComplete => 'Completo';

  @override
  String get managedGlobalSearchPartial => 'Parcial';

  @override
  String get managedGlobalSearchTruncated =>
      'Se muestran las primeras 100 coincidencias. Refina la búsqueda.';

  @override
  String get managedGlobalSearchOpen => 'Abrir';

  @override
  String get managedGlobalSearchCreateDraft => 'Crear borrador';

  @override
  String get managedGlobalSearchInspect => 'Inspeccionar';

  @override
  String get managedGlobalSearchKindModEntity => 'Contenido del mod';

  @override
  String get managedGlobalSearchKindModAsset => 'Recurso del mod';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Punto de partida de PNJ';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Punto de partida de misión';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Evidencia de PNJ';

  @override
  String get managedGlobalSearchReadinessExact => 'Proyecto actual exacto';

  @override
  String get managedGlobalSearchReadinessProblems => 'Exacto, con problemas';

  @override
  String get managedGlobalSearchResultStale =>
      'Este resultado ya no está en el proyecto actual. Vuelve a buscar.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Solo borrador';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Compilación bloqueada';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Ejecución no verificada';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Resumen';

  @override
  String get managedStoryWorkbenchProfileTab => 'Perfil';

  @override
  String get managedStoryWorkbenchStoryTab => 'Historia';

  @override
  String get managedStoryWorkbenchLogicTab => 'Lógica';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Rutina';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventario';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Diálogo y voz';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Referencias';

  @override
  String get managedStoryWorkbenchProblemsChecksTab =>
      'Problemas y comprobaciones';

  @override
  String get managedStoryWorkbenchEditOverview => 'Editar nombre y objetivos';

  @override
  String get managedStoryWorkbenchEditStory =>
      'Editar descripción y conexiones';

  @override
  String get managedStoryWorkbenchEditLogic => 'Editar estados y transiciones';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Abrir código fuente y comprobaciones del compilador';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Abrir perfil y comprobaciones del compilador';

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
  String get managedStoryWorkbenchCapabilityUnavailable => 'Aún no modelado';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Las relaciones de misiones e historia aún no están modeladas para los borradores de PNJ.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'La rutina y la ubicación en el mundo aún no están modeladas.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'El inventario, el equipo y el comercio aún no están modelados.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Las relaciones de diálogo, localización y voz aún no están modeladas para los borradores de PNJ.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Las relaciones de diálogo, localización y voz aún no están modeladas para los borradores de misión.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'No hay referencias de proyecto sin resolver';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count referencias de proyecto sin resolver',
      one: '1 referencia de proyecto sin resolver',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Solo indica el estado de las referencias; no confirma que esté listo para compilarse ni ejecutarse.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Detalles técnicos';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Borrador de misión';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Borrador de PNJ';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Título de la misión';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'ID técnico';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Objetivos';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Nombre único';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Espacio de nombres del módulo';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Dador de la misión';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Clase base en tiempo de ejecución';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Los estados del ciclo de vida de la misión, los activadores, las condiciones y los efectos se editan como una única operación atómica sobre el estado actual exacto.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Salientes';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'No hay referencias proyectadas';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Entrantes';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'No hay referencias de proyecto entrantes';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Identidad semántica';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origen';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Revisión de entidad';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'ID estable';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Referencia resuelta';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Referencia sin resolver';

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
}
