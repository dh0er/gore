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
  String get managedProjectSubtitle =>
      'Espace de création hors ligne correspondant exactement à la version actuelle';

  @override
  String get managedProjectTechnicalDetails => 'Détails techniques du projet';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Rouvrez le projet géré avant de lire son contenu.';

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
  String get managedActionAddVoiceTakeTitle => 'Ajouter une prise de voix';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importez un enregistrement Ogg Vorbis dans ce projet sans le déployer.';

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
  String get managedActionSettingsTitle => 'Paramètres';

  @override
  String get managedActionSettingsDescription =>
      'Configurez l’installation de Gothic 1 Remake et les préférences de Mod Studio.';
}
