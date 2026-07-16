// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Dialogues';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Textures';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Tout';

  @override
  String get sectionItemValues => 'Valeurs des objets';

  @override
  String get sectionLocalizedText => 'Textes localisés';

  @override
  String get audioCatCreatures => 'Créatures';

  @override
  String get audioCatObjects => 'Objets';

  @override
  String get audioCatMagic => 'Magie';

  @override
  String get audioCatMovement => 'Mouvement';

  @override
  String get audioCatWorld => 'Monde';

  @override
  String get audioCatAction => 'Actions';

  @override
  String get audioCatCombat => 'Combat';

  @override
  String get audioCatPhysics => 'Physique';

  @override
  String get audioCatItems => 'Items';

  @override
  String get audioCatUi => 'Interface';

  @override
  String get audioCatFoley => 'Bruitages';

  @override
  String get audioCatUnderwater => 'Sous l\'eau';

  @override
  String get audioCatVision => 'Visions';

  @override
  String get audioCatDialog => 'Dialogue';

  @override
  String get audioCatOther => 'Autre';

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
  String get extractLocalizedText => 'Extraire les textes localisés';

  @override
  String get lightMode => 'Mode clair';

  @override
  String get darkMode => 'Mode sombre';

  @override
  String get language => 'Langue';

  @override
  String get exportMod => 'Exporter le mod';

  @override
  String exportModWithCount(int count) {
    return 'Exporter le mod ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Sélectionnez un objet pour modifier ses champs.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Données du jeu : $name';
  }

  @override
  String get gameDataBundledTooltip => 'Données du jeu : intégrées';

  @override
  String get loadGameDataDump => 'Charger un dump de données du jeu…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json du mod gore-dump';

  @override
  String get useBundledData => 'Utiliser les données intégrées';

  @override
  String get alreadyBundled => 'déjà intégrées';

  @override
  String get gameDataFileGroupLabel => 'données du jeu';

  @override
  String get minimize => 'Réduire';

  @override
  String get restore => 'Restaurer';

  @override
  String get maximize => 'Agrandir';

  @override
  String get close => 'Fermer';

  @override
  String get about => 'À propos';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 contributeurs de GORE';

  @override
  String get aboutLicense => 'Sous licence MIT.';

  @override
  String get categoryMeleeWeapons => 'Armes de mêlée';

  @override
  String get categoryRangedWeapons => 'Armes à distance';

  @override
  String get categoryAmmunition => 'Munitions';

  @override
  String get categoryRunes => 'Runes';

  @override
  String get categorySpellScrolls => 'Parchemins de sort';

  @override
  String get categoryFoodAndPotions => 'Nourriture & potions';

  @override
  String get categoryMiscellaneous => 'Divers';

  @override
  String get categoryAmulets => 'Amulettes';

  @override
  String get categoryRings => 'Anneaux';

  @override
  String get categoryAnimalTrophies => 'Trophées d\'animaux';

  @override
  String get categoryWritings => 'Écrits';

  @override
  String get categoryMissionItems => 'Objets de quête';

  @override
  String get categoryKeys => 'Clés';

  @override
  String get categoryOther => 'Autre';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Rechercher des objets';

  @override
  String get noItemsMatch => 'Aucun objet correspondant';

  @override
  String failedToLoadCatalog(String error) {
    return 'Échec du chargement du catalogue : $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Modifications en attente ($count)';
  }

  @override
  String get clearAll => 'Tout effacer';

  @override
  String get noPendingOverrides =>
      'Aucune modification en attente.\nModifiez des champs pour en ajouter.';

  @override
  String get removeOverride => 'Supprimer la modification';

  @override
  String get searchChanges => 'Rechercher des modifications';

  @override
  String get noChangesMatch => 'Aucune modification correspondante';

  @override
  String get clearSection => 'Effacer ce groupe';

  @override
  String get modName => 'Nom du mod';

  @override
  String get loadDelayLabel => 'Délai de chargement (ms, 0 = immédiat)';

  @override
  String get noFolderSelected => 'Aucun dossier sélectionné';

  @override
  String get chooseFolder => 'Choisir un dossier';

  @override
  String get packageAsZip => 'Empaqueter en .zip';

  @override
  String get cancel => 'Annuler';

  @override
  String get export => 'Exporter';

  @override
  String get exportHere => 'Exporter ici';

  @override
  String get mustBeNonNegativeInteger => 'Doit être un entier non négatif';

  @override
  String get extractingLocalizedText =>
      'Extraction des textes localisés du jeu…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extraction des textes localisés annulée.';

  @override
  String get localizedTextExtracted => 'Textes localisés extraits.';

  @override
  String get extractionFailed => 'Échec de l\'extraction.';

  @override
  String get localizationCacheFileGroupLabel => 'cache de localisation';

  @override
  String get extractLocalizedTextQuestion =>
      'Extraire les textes localisés du jeu ?';

  @override
  String get extractLocalizedTextBody =>
      'Les textes localisés du jeu ne sont pas encore extraits. Les extraire maintenant depuis votre installation du jeu ? (facultatif)';

  @override
  String get notNow => 'Pas maintenant';

  @override
  String get extract => 'Extraire';

  @override
  String get validationRequired => 'Requis';

  @override
  String get validationMustBeWholeNumber => 'Doit être un nombre entier';

  @override
  String get validationMustBeNumber => 'Doit être un nombre';

  @override
  String get validationMustBeFinite => 'Doit être un nombre fini';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Doit être ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Doit être ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Doit être true ou false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Doit être l\'un de : $options';
  }

  @override
  String get modNameRequired => 'Requis';

  @override
  String get modNameControlCharacters =>
      'Ne doit pas contenir de caractères de contrôle';

  @override
  String get modNamePathSeparators =>
      'Ne doit pas contenir de séparateurs de chemin';

  @override
  String get modNameNotAFolderName => 'Nom de dossier invalide';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount identifiants extraits dans $languageCount langues';
  }

  @override
  String get managerDeployActive =>
      'Un loadout du mod-manager est actif. Faites d\'abord l\'undeploy dans gore-manager.';

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
      'Le nouveau projet est ouvert, mais la session du projet précédent n’a pas pu être entièrement nettoyée. Aucun nouvel essai de nettoyage ne sera effectué. Redémarrez Mod Studio avant de rouvrir le projet précédent.';

  @override
  String get projectNewManagedRevision3 => 'Nouveau projet de mod géré…';

  @override
  String get projectNewLegacy => 'Nouveau projet historique';

  @override
  String get projectCreateGamePathRequired =>
      'Définissez le chemin de Gothic 1 Remake dans les paramètres avant de créer un projet de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Créer le projet de mod géré ici';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projet de mod géré $projectId créé';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Échec de la création du projet de mod géré : $error';
  }

  @override
  String get projectCreateDialogTitle => 'Créer un projet de mod';

  @override
  String get projectCreateNameLabel => 'Nom du projet';

  @override
  String get projectCreateNameHelper => 'Le nom affiché dans Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Version';

  @override
  String get projectCreateVersionHelper =>
      'Une version initiale, par exemple 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Auteur';

  @override
  String get projectCreateAuthorHelper =>
      'Votre nom ou celui de votre équipe de modding.';

  @override
  String get projectCreateLocalesLabel => 'Langues d’édition';

  @override
  String get projectCreateLocalesHelper =>
      'Balises canoniques séparées par des virgules, par exemple : en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Ceci crée un projet hors ligne géré et vide. Aucun mod n’est compilé, déployé ou exécuté, et les fichiers du jeu et de sauvegarde ne sont pas modifiés.';

  @override
  String get projectCreateSubmit => 'Créer le projet';

  @override
  String projectCreateMetadataRequired(String label) {
    return 'Le champ $label est obligatoire.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return 'Le champ $label ne peut pas commencer ou finir par un espace.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return 'Le champ $label ne peut pas contenir de caractères de contrôle.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return 'Le champ $label contient du texte mal formé.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return 'Le champ $label dépasse la limite UTF-8 de $maxBytes octets.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Saisissez au moins une langue d’édition.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Supprimez l’entrée de langue vide.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Utilisez au maximum $maxLocales langues d’édition.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'La locale « $locale » doit être en ASCII et de longueur limitée.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'La locale « $locale » doit commencer par une langue en minuscules de 2 à 8 lettres.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'La locale « $locale » contient un segment non valide.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'La locale « $locale » n’est pas canonique ; utilisez « $canonical ».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Vue d’ensemble';

  @override
  String get managedWorkspaceContentLabel => 'Contenu';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Ce mod';

  @override
  String get managedWorkspaceHomeLabel => 'Accueil';

  @override
  String get managedWorkspaceStoryLabel => 'Scénario';

  @override
  String get managedWorkspaceWorldLabel => 'Monde';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localisation et voix';

  @override
  String get managedWorkspaceValidateTestLabel => 'Valider et tester';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Compiler et publier';

  @override
  String get managedWorkspaceHistoryLabel => 'History';

  @override
  String get managedWorkspaceSettingsExpertLabel => 'Paramètres et mode expert';

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
  String get managedSectionStoryDescription => 'PNJ, quêtes et dialogues.';

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
      'Le placement dans le monde et les flux associés sont planifiés.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Écrivez et traduisez les dialogues du projet au même endroit, puis poursuivez avec les voix.';

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
      'Vérifie l’intégrité exacte du projet et ses points de contrôle ; aucun test en jeu n’est revendiqué.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Les bundles vocaux sont disponibles ; les builds jouables complets et le déploiement ne le sont pas.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Les paramètres sont disponibles ; les outils experts ne sont pas encore intégrés.';

  @override
  String get managedSectionStatusHeading => 'État';

  @override
  String get managedSectionActionsHeading => 'Actions';

  @override
  String get managedCapabilityAvailable => 'Disponible';

  @override
  String get managedCapabilityPartial => 'Partiel';

  @override
  String get managedCapabilityPlanned => 'Planifié';

  @override
  String get managedCapabilityUnavailable => 'Indisponible';

  @override
  String get managedProjectSubtitle =>
      'Espace de création hors ligne correspondant exactement à la version actuelle';

  @override
  String get managedProjectLandingTitle => 'Espace de travail de projet géré';

  @override
  String get managedProjectLandingDescription =>
      'Utilisez le nouveau flux Accueil, Contenu, Histoire, Voix, validation et publication dans un seul projet géré.';

  @override
  String get legacyCompatibilityToolsTitle => 'Outils de compatibilité hérités';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Les onglets ci-dessous regroupent les anciens outils de remplacement direct. Ils restent disponibles pendant l’évolution de l’espace de travail de projet géré.';

  @override
  String get managedProjectTechnicalDetails => 'Détails techniques du projet';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Rouvrez le projet géré avant de lire son contenu.';

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
  String get managedDashboardUntitledProject => 'Projet sans titre';

  @override
  String get managedDashboardDraftStatus => 'Brouillon';

  @override
  String get managedDashboardProjectVersion => 'Version';

  @override
  String get managedDashboardProjectAuthor => 'Auteur';

  @override
  String get managedDashboardNotProvided => 'Non renseigné';

  @override
  String get managedDashboardContentCounts => 'Contenu du projet';

  @override
  String get managedDashboardNpcDrafts => 'Brouillons de PNJ';

  @override
  String get managedDashboardQuestDrafts => 'Brouillons de quêtes';

  @override
  String get managedDashboardDialogLines => 'Lignes de dialogue';

  @override
  String get managedDashboardVoiceTakes => 'Prises de voix';

  @override
  String get managedDashboardAssets => 'Ressources';

  @override
  String get managedDashboardUnresolvedReferences => 'Références non résolues';

  @override
  String get managedDashboardReadiness => 'Fonctionnalités disponibles';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Création hors ligne disponible';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Créez et modifiez les contenus de projet pris en charge sans changer l’installation du jeu ni les fichiers de sauvegarde.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Compilation générale du mod indisponible';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Seuls les bundles Voice hors ligne scellés peuvent être générés ; il n’est pas encore possible de générer un mod complet et jouable.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Exécution pas encore vérifiée';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio n’a pas encore validé ce contenu de projet dans le jeu en cours d’exécution.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Intégrité des références';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Ce nombre vérifie uniquement les références du projet ; il ne confirme pas que le projet est prêt à être compilé ou exécuté.';

  @override
  String get managedDashboardMissingGameTitle => 'Configuration du jeu requise';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configurez l’installation de Gothic 1 Remake dans les paramètres avant d’utiliser les actions nécessitant des preuves issues du jeu installé.';

  @override
  String get managedDashboardCreateHeading => 'Créer';

  @override
  String get managedDashboardToolsHeading => 'Outils du projet';

  @override
  String get managedDashboardLoading =>
      'Chargement de la vue d’ensemble du projet';

  @override
  String get managedDashboardLoadError =>
      'Vue d’ensemble du projet indisponible';

  @override
  String get managedDashboardLoadErrorDescription =>
      'La vue d’ensemble vérifiée du projet n’a pas pu être chargée. Le contenu du projet n’a pas été modifié.';

  @override
  String get managedDashboardRetry => 'Réessayer';

  @override
  String get managedActionNewNpcTitle => 'Nouveau PNJ';

  @override
  String get managedActionNewNpcDescription =>
      'Créez un brouillon de PNJ hors ligne et limité à partir de données vérifiées du jeu installé.';

  @override
  String get managedActionNewQuestTitle => 'Nouvelle quête';

  @override
  String get managedActionNewQuestDescription =>
      'Créez un brouillon de quête hors ligne avec des objectifs et des identités parentes vérifiées.';

  @override
  String get managedActionNewDialogLineTitle => 'Ajouter une ligne de dialogue';

  @override
  String get managedActionNewDialogLineDescription =>
      'Écrivez un texte de projet localisé ou associez un texte inutilisé déjà présent dans ce projet. Cela ne crée aucun sujet de dialogue jouable.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Ligne de dialogue enregistrée dans la révision $projectRevision du projet. Le jeu et les sauvegardes n’ont pas été modifiés.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Écrivez une nouvelle ligne de dialogue localisée ou associez un texte appartenant déjà à ce projet.';

  @override
  String get managedDialogLineBoundary =>
      'Seuls les fichiers du projet sont modifiés. Cela ne crée ni sujet AngelScript ni dialogue jouable, et ne modifie jamais l’installation du jeu ou les sauvegardes. Le champ du locuteur n’est qu’une étiquette ; il ne relie aucun PNJ.';

  @override
  String get managedDialogLineCreateMode => 'Écrire un nouveau texte';

  @override
  String get managedDialogLineReuseMode => 'Utiliser un texte du projet';

  @override
  String get managedDialogLineNameLabel => 'Nom de la ligne';

  @override
  String get managedDialogLineNameHint => 'Accueil à l’entrée de la mine';

  @override
  String get managedDialogLineSpeakerLabel =>
      'Étiquette du locuteur (facultatif)';

  @override
  String get managedDialogLineSpeakerHint => 'Par exemple, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Langue';

  @override
  String get managedDialogLineTextLabel => 'Texte du dialogue';

  @override
  String get managedDialogLineReuseSearch =>
      'Rechercher un texte de projet inutilisé';

  @override
  String get managedDialogLineNoReusableText =>
      'Aucun texte de projet inutilisé et structurellement valide ne peut être associé. Écrivez plutôt un nouveau texte.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Préparer cette langue pour Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Crée un emplacement Voice vide et non résolu dans le projet. Aucun enregistrement n’est ajouté ni déployé.';

  @override
  String get managedDialogLineCancel => 'Annuler';

  @override
  String get managedDialogLineSave => 'Enregistrer dans le projet';

  @override
  String get managedDialogLineSaving => 'Enregistrement…';

  @override
  String get managedDialogLineLoading => 'Lecture du contenu exact du projet…';

  @override
  String get managedDialogLineLoadFailed =>
      'Le contenu actuel exact du projet n’a pas pu être lu. Rien n’a été modifié.';

  @override
  String get managedDialogLineRetry => 'Réessayer';

  @override
  String get managedDialogLineStale =>
      'Le projet a changé pendant que cette fenêtre était ouverte. Fermez-la et réessayez depuis le projet actuel.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Le projet actuel ne peut plus être vérifié en toute sécurité. Fermez cette fenêtre et rouvrez le projet géré.';

  @override
  String get managedDialogLineInvalidInput =>
      'Vérifiez la saisie de projet mise en évidence et choisissez une option actuelle exacte.';

  @override
  String get managedDialogLineSaveFailed =>
      'La ligne de dialogue n’a pas pu être enregistrée en toute sécurité. Aucun fichier de jeu ou de sauvegarde n’a été modifié.';

  @override
  String get managedDialogLineDone => 'Terminé';

  @override
  String get managedDialogLineAddRecording => 'Ajouter un enregistrement';

  @override
  String get managedActionAddVoiceTakeTitle => 'Ajouter une prise de voix';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importez un enregistrement Ogg Vorbis dans ce projet sans le déployer.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Gérer les prises de voix';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Examinez les prises et sélectionnez les enregistrements approuvés pour les emplacements Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Résoudre la cible Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Associez les emplacements Voice du projet aux membres exacts des archives installées sans modifier le jeu.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Générer le bundle Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Générez un bundle hors ligne scellé à partir de membres existants ; aucun déploiement n’est effectué.';

  @override
  String get managedActionDataAssetsTitle => 'Modifications de DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspectez les paquets installés et préparez dans le projet des modifications vérifiées de valeurs à largeur fixe.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Parcourez le contenu exact du projet ainsi que ses références résolues ou non résolues.';

  @override
  String get managedActionSettingsTitle => 'Paramètres';

  @override
  String get managedActionSettingsDescription =>
      'Configurez l’installation de Gothic 1 Remake et les préférences de Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Le projet $projectId a été créé en toute sécurité, mais la configuration de départ ne s’est pas ouverte. Le projet vide valide reste actif.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Le projet $projectId a été créé, mais Mod Studio ne peut pas vérifier le résultat du démarrage. Rouvrez le projet géré avant de continuer ; le jeu et les sauvegardes n’ont pas été modifiés.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Le projet $projectId a été créé. Le démarrage PNJ n’a pas été ajouté ; le projet vide valide reste actif.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Démarrage PNJ enregistré dans la révision $projectRevision. Il reste bloqué pour la génération, non qualifié à l’exécution et n’est pas instancié.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Le projet $projectId a été créé. Le démarrage de quête n’a pas été ajouté ; le projet vide valide reste actif.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Démarrage de quête enregistré dans la révision $projectRevision. Il reste bloqué pour la génération et non qualifié à l’exécution.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Démarrage du projet';

  @override
  String get projectStarterPrompt => 'Comment souhaitez-vous commencer ?';

  @override
  String get projectStarterWriteBoundary =>
      'Choisir un démarrage n’écrit rien. Le projet est créé uniquement après l’envoi de ce formulaire et le choix d’un dossier vide.';

  @override
  String get projectStarterEmptyTitle => 'Projet vide';

  @override
  String get projectStarterEmptyDescription =>
      'Créez uniquement le projet géré. Ajoutez du contenu quand vous le souhaitez.';

  @override
  String get projectStarterNpcDraftTitle => 'Brouillon de PNJ';

  @override
  String get projectStarterNpcDraftDescription =>
      'Créez d’abord le projet vide, puis ouvrez la configuration guidée du brouillon de PNJ.';

  @override
  String get projectStarterQuestDraftTitle => 'Brouillon de quête';

  @override
  String get projectStarterQuestDraftDescription =>
      'Créez d’abord le projet vide, puis ouvrez la configuration guidée du brouillon de quête.';

  @override
  String get projectStarterPartialOutcome =>
      'Si vous annulez la configuration guidée d’un PNJ ou d’une quête, ou si le brouillon échoue, un projet vide valide demeure. Aucun choix de démarrage n’écrit dans le jeu ni dans une sauvegarde.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Parcourir';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Modifications vérifiées';

  @override
  String get managedContentScopeBaseGameLabel => 'Jeu de base';

  @override
  String get managedContentScopeInstalledLabel => 'Installé';

  @override
  String get managedBaseGameBrowserTitle =>
      'Points de départ pris en charge du jeu de base';

  @override
  String get managedBaseGameBrowserDescription =>
      'Parcourez les preuves exactes du jeu installé que Mod Studio peut actuellement inspecter ou utiliser comme point de départ sûr pour un brouillon. Ce catalogue ne couvre pas tout le contenu d’origine.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Lecture des preuves exactes du jeu de base…';

  @override
  String get managedBaseGameBrowserRefresh => 'Lire un nouveau catalogue exact';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Rechercher le contenu pris en charge du jeu de base';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'PNJ';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Quêtes';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'Points de départ PNJ';

  @override
  String get managedBaseGameBrowserQuestSectionTitle =>
      'Points de départ de quête';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Archétypes de PNJ à inspecter uniquement';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Recherchez pour inclure davantage de preuves de PNJ à liaison statique. Ces lignes ne permettent pas de créer un brouillon.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Aucun résultat pris en charge du jeu de base ne correspond à cette recherche.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Preuves du jeu de base indisponibles';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Le catalogue exact pris en charge n’a pas pu être lu. Aucun fichier de projet, de jeu ou de sauvegarde n’a été modifié.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Brouillon hors ligne pris en charge';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Inspection uniquement';

  @override
  String get managedBaseGameBrowserCreateNpcDraft =>
      'Utiliser comme départ PNJ';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Utiliser comme départ de quête';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Définition d’apparition';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint d’acteur';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Les 100 premiers résultats à inspecter uniquement sont affichés. Affinez la recherche pour des résultats plus précis.';

  @override
  String get managedInstalledBrowserLoading =>
      'Lecture de l’inventaire exact des paquets installés…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count paquets installés candidats';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count paquets installés candidats — résultat partiel';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Les métadonnées du répertoire ont été lues et l’instantané installé est resté exact.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Certaines métadonnées de paquet étaient absentes ou non canoniques ; les résultats aident à la découverte, mais ne sont pas complets.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Cette portée affiche uniquement les métadonnées des paquets DataAsset installés. Inspecter ou copier un chemin n’accorde aucun droit de génération, de déploiement, d’exécution ou d’écriture dans le jeu.';

  @override
  String get managedInstalledBrowserRefresh =>
      'Lire un nouvel instantané exact';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Rechercher les DataAssets installés';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nom de ressource ou chemin /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Saisissez un nom de ressource ou un chemin /Game à rechercher.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Aucun DataAsset installé correspondant';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Essayez un autre nom de ressource ou un chemin /Game plus large.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Les 100 premiers résultats sont affichés. Affinez la recherche pour réduire l’instantané exact.';

  @override
  String get managedInstalledBrowserKindBadge => 'Paquet DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge =>
      'Métadonnées uniquement';

  @override
  String get managedInstalledBrowserOpenInspector =>
      'Inspecter le paquet exact';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventaire des paquets installés indisponible';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'L’instantané installé exact n’a pas pu être lu. Aucun fichier de projet, de jeu ou de sauvegarde n’a été modifié.';

  @override
  String get managedGlobalSearchScopeLabel => 'Tout rechercher';

  @override
  String get managedGlobalSearchTitle => 'Rechercher dans tout le contenu';

  @override
  String get managedGlobalSearchLabel =>
      'PNJ, quête, réplique, ressource, ID ou chemin /Game';

  @override
  String get managedGlobalSearchAction => 'Rechercher';

  @override
  String get managedGlobalSearchClear => 'Effacer';

  @override
  String get managedGlobalSearchPrompt =>
      'Saisissez une recherche pour consulter les trois sources indépendamment.';

  @override
  String get managedGlobalSearchNoResults =>
      'Aucun résultat dans cette source.';

  @override
  String get managedGlobalSearchLoading => 'Lecture de la source exacte…';

  @override
  String get managedGlobalSearchFailed => 'Impossible de lire cette source.';

  @override
  String get managedGlobalSearchComplete => 'Complet';

  @override
  String get managedGlobalSearchPartial => 'Partiel';

  @override
  String get managedGlobalSearchTruncated =>
      'Affichage des 100 premiers résultats. Affinez la recherche.';

  @override
  String get managedGlobalSearchOpen => 'Ouvrir';

  @override
  String get managedGlobalSearchCreateDraft => 'Créer un brouillon';

  @override
  String get managedGlobalSearchInspect => 'Inspecter';

  @override
  String get managedGlobalSearchKindModEntity => 'Contenu du mod';

  @override
  String get managedGlobalSearchKindModAsset => 'Ressource du mod';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Point de départ de PNJ';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Point de départ de quête';

  @override
  String get managedGlobalSearchKindExperimentalNpc =>
      'Élément de preuve de PNJ';

  @override
  String get managedGlobalSearchReadinessExact => 'Projet actuel exact';

  @override
  String get managedGlobalSearchReadinessProblems =>
      'Exact, avec des problèmes';

  @override
  String get managedGlobalSearchResultStale =>
      'Ce résultat ne figure plus dans le projet actuel. Relancez la recherche.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Brouillon uniquement';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Compilation bloquée';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Exécution non vérifiée';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Vue d’ensemble';

  @override
  String get managedStoryWorkbenchProfileTab => 'Profil';

  @override
  String get managedStoryWorkbenchStoryTab => 'Histoire';

  @override
  String get managedStoryWorkbenchLogicTab => 'Logique';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Routine';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventaire';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Dialogues et voix';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Références';

  @override
  String get managedStoryWorkbenchProblemsChecksTab =>
      'Problèmes et vérifications';

  @override
  String get managedStoryWorkbenchEditOverview =>
      'Modifier le nom et les objectifs';

  @override
  String get managedStoryWorkbenchEditStory =>
      'Modifier la description et les connexions';

  @override
  String get managedStoryWorkbenchEditLogic =>
      'Modifier les états et les transitions';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Ouvrir le code source et les vérifications du compilateur';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Ouvrir le profil et les vérifications du compilateur';

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
      'Pas encore modélisé';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Les relations avec les quêtes et l’histoire ne sont pas encore modélisées pour les brouillons de PNJ.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'La routine et le placement dans le monde ne sont pas encore modélisés.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'L’inventaire, l’équipement et le commerce ne sont pas encore modélisés.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Les relations avec les dialogues, la localisation et les voix ne sont pas encore modélisées pour les brouillons de PNJ.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Les relations avec les dialogues, la localisation et les voix ne sont pas encore modélisées pour les brouillons de quête.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Aucune référence de projet non résolue';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count références de projet non résolues',
      one: '1 référence de projet non résolue',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'État des références uniquement ; il ne garantit pas que le projet est prêt à être compilé ou exécuté.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Détails techniques';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Brouillon de quête';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Brouillon de PNJ';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Titre de la quête';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'Identifiant technique';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Objectifs';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Nom unique';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Espace de noms du module';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Donneur de quête';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Classe parente à l’exécution';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Les états du cycle de vie de la quête, les déclencheurs, les conditions et les effets sont modifiés en une seule opération atomique sur l’état actuel exact.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Sortantes';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Aucune référence projetée';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Entrantes';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Aucune référence entrante du projet';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Identité sémantique';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origine';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Révision d’entité';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'ID stable';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel => 'Référence résolue';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Référence non résolue';

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
}
