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
  String get managedWorkspaceSettingsExpertLabel => 'Paramètres et mode expert';

  @override
  String get managedSectionStoryDescription => 'PNJ, quêtes et dialogues.';

  @override
  String get managedSectionWorldDescription =>
      'Le placement dans le monde et les flux associés sont planifiés.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Les outils de production vocale sont disponibles ; l’édition des localisations dans le projet géré est prévue.';

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
}
