// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get appTitle => 'Éditeur de sauvegardes Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'Logo goresave';

  @override
  String get zoomTooltip => 'Appuyez sur Ctrl +/- pour zoomer/dézoomer';

  @override
  String get switchToLightMode => 'Passer en mode clair';

  @override
  String get switchToDarkMode => 'Passer en mode sombre';

  @override
  String get about => 'À propos';

  @override
  String get tabOverview => 'Aperçu';

  @override
  String get tabPlayer => 'Joueur';

  @override
  String get tabAttribute => 'Attributs';

  @override
  String get tabInventory => 'Inventaire';

  @override
  String get tabProgression => 'Progression';

  @override
  String get tabCharacters => 'Personnages';

  @override
  String get characterNoActorBody =>
      'Ce personnage n\'a pas d\'acteur dans le monde ; il n\'a donc ni attributs, ni inventaire, ni événements.';

  @override
  String get tabAllData => 'Toutes les données';

  @override
  String get tabBackups => 'Sauvegardes';

  @override
  String get tabSettings => 'Paramètres';

  @override
  String get reset => 'Réinitialiser';

  @override
  String get save => 'Enregistrer';

  @override
  String saveWithCount(int count) {
    return 'Enregistrer ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Annuler';

  @override
  String get confirm => 'Confirmer';

  @override
  String get close => 'Fermer';

  @override
  String get add => 'Ajouter';

  @override
  String get equippedBadge => 'Équipé';

  @override
  String get armorUpgradesLabel => 'Améliorations';

  @override
  String get browse => 'Parcourir';

  @override
  String get noSavFilesFound => 'Aucun fichier .sav trouvé';

  @override
  String get profile => 'Profil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count sauvegardes)';
  }

  @override
  String get switchProfile => 'Changer de profil';

  @override
  String get rescanSaveFolder => 'Réanalyser le dossier de sauvegardes';

  @override
  String get discardUnsavedChangesTitle =>
      'Abandonner les modifications non enregistrées ?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'modifications non enregistrées',
      one: 'modification non enregistrée',
    );
    return 'La réanalyse recharge chaque sauvegarde et abandonne vos $count $_temp0.';
  }

  @override
  String get discardAndRescan => 'Abandonner et réanalyser';

  @override
  String chapterLabel(Object id) {
    return 'Chapitre $id';
  }

  @override
  String get quickSave => 'Sauvegarde rapide';

  @override
  String get autoSave => 'Sauvegarde automatique';

  @override
  String get manualSave => 'Sauvegarde manuelle';

  @override
  String get errorTitle => 'Erreur';

  @override
  String get selectASaveTitle => 'Sélectionner une sauvegarde';

  @override
  String get selectASaveBody =>
      'Les détails de la sauvegarde apparaîtront ici.';

  @override
  String get diagnosticsTitle => 'Diagnostics et détails';

  @override
  String get diagnosticsSubtitle => 'Inspection du format en lecture seule';

  @override
  String get metricFormat => 'Format';

  @override
  String get metricSlot => 'Emplacement';

  @override
  String get metricChapter => 'Chapitre';

  @override
  String get metricTimePlayed => 'Temps de jeu';

  @override
  String get metricSaveKind => 'Type de sauvegarde';

  @override
  String get metricFileSize => 'Taille du fichier';

  @override
  String get metricCompression => 'Compression';

  @override
  String get metricChunks => 'Blocs';

  @override
  String get metricUncompressed => 'Décompressé';

  @override
  String get metricPrivate => 'Privé';

  @override
  String get metricSlotName => 'Nom de l\'emplacement';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Privé décodé';

  @override
  String get metricPrivateStrings => 'Chaînes privées';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count octets';
  }

  @override
  String get inspectionJsonTitle => 'JSON d\'inspection';

  @override
  String get inspectionJsonSubtitle =>
      'Données brutes d\'inspection de la sauvegarde';

  @override
  String get copy => 'Copier';

  @override
  String get savegameFallbackTitle => 'Sauvegarde';

  @override
  String screenshotForSlot(String slot) {
    return 'Capture d\'écran pour $slot';
  }

  @override
  String get publicSaveName => 'Nom public de la sauvegarde';

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
  String get required => 'Requis';

  @override
  String get playerLockedBody =>
      'Les modifications privées du joueur nécessitent un codec capable de compresser.';

  @override
  String get heroTransform => 'Transform du héros';

  @override
  String get locationX => 'Position X';

  @override
  String get locationY => 'Position Y';

  @override
  String get locationZ => 'Position Z';

  @override
  String get rotationPitch => 'Tangage (pitch)';

  @override
  String get rotationYaw => 'Lacet (yaw)';

  @override
  String get rotationRoll => 'Roulis (roll)';

  @override
  String get invalid => 'Invalide';

  @override
  String get heroAttributes => 'Attributs du héros';

  @override
  String attributeBase(String name) {
    return '$name de base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name actuel';
  }

  @override
  String get inventoryTitle => 'Inventaire';

  @override
  String get inventoryEmpty => 'Cet inventaire est vide.';

  @override
  String get inventoryNeedsDecoded =>
      'La modification de l\'inventaire nécessite des données privées décodées par le codec.';

  @override
  String get inventoryNoStacks =>
      'Aucune pile d\'objets trouvée dans les données privées décodées.';

  @override
  String get resetInventoryChanges =>
      'Réinitialiser les modifications de l\'inventaire';

  @override
  String get addItemTooltipPendingAdd =>
      'Enregistrez d\'abord les modifications en attente — un nouvel objet par sauvegarde';

  @override
  String get addItemTooltipPendingRemove =>
      'Enregistrez d\'abord la suppression en attente — une modification structurelle par sauvegarde';

  @override
  String get addItemTooltipPendingCount =>
      'Enregistrez ou réinitialisez d\'abord les modifications de quantité en attente — une modification structurelle doit être enregistrée seule';

  @override
  String get addItemTooltipDefault => 'Ajouter un objet à l\'inventaire';

  @override
  String get addItemButton => 'Ajouter un objet';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — ajout en attente (pas encore enregistré)';
  }

  @override
  String get cancelPendingAdd => 'Annuler l\'ajout en attente';

  @override
  String get pendingRemovalSubtitle =>
      'suppression en attente (pas encore enregistrée)';

  @override
  String get cancelPendingRemoval => 'Annuler la suppression en attente';

  @override
  String get filterItems => 'Filtrer les objets';

  @override
  String noItemsMatchQuery(String query) {
    return 'Aucun objet ne correspond à « $query ».';
  }

  @override
  String get pendingRemovalHidesAll =>
      'La suppression en attente masque tous les objets — enregistrez pour l\'appliquer.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Armes de mêlée';

  @override
  String get itemCategoryRangedWeapon => 'Armes à distance';

  @override
  String get itemCategoryAmmunition => 'Munitions';

  @override
  String get itemCategoryArmor => 'Armures';

  @override
  String get itemCategoryRune => 'Runes';

  @override
  String get itemCategoryScroll => 'Parchemins de sort';

  @override
  String get itemCategoryFood => 'Nourriture et potions';

  @override
  String get itemCategoryMisc => 'Divers';

  @override
  String get itemCategoryAmulet => 'Amulettes';

  @override
  String get itemCategoryRing => 'Anneaux';

  @override
  String get itemCategoryTrophy => 'Trophées d\'animaux';

  @override
  String get itemCategoryWriting => 'Écrits';

  @override
  String get itemCategoryMission => 'Objets de quête';

  @override
  String get itemCategoryKey => 'Clés';

  @override
  String get itemCategoryOther => 'Autres';

  @override
  String get count => 'Quantité';

  @override
  String get min1 => 'Min 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Suppression impossible : cet objet est probablement équipé ou assigné à un emplacement de raccourci';

  @override
  String get removeBlockedTooltip =>
      'Enregistrez ou réinitialisez d\'abord vos modifications d\'inventaire en attente — un ajout ou une suppression doit être enregistré seul';

  @override
  String get removeItemFromInventory => 'Retirer l\'objet de l\'inventaire';

  @override
  String get progressionLockedBody =>
      'Les données de progression nécessitent des données privées décodées par le codec.';

  @override
  String get progressionNeedsTyped =>
      'Les données de progression structurées nécessitent une sauvegarde entièrement décodée avec une analyse typée vérifiée.';

  @override
  String get sectionQuests => 'Quêtes';

  @override
  String get sectionKnowledge => 'Connaissances';

  @override
  String get sectionEvents => 'Événements';

  @override
  String get firstPage => 'Première page';

  @override
  String get previousPage => 'Page précédente';

  @override
  String get nextPage => 'Page suivante';

  @override
  String get lastPage => 'Dernière page';

  @override
  String pageOfPages(int page, int total) {
    return 'Page $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last sur $total';
  }

  @override
  String get perPage => 'Par page :';

  @override
  String get resetQuestChanges => 'Réinitialiser les modifications de quêtes';

  @override
  String get searchQuests => 'Rechercher des quêtes';

  @override
  String get allGroups => 'Tous les groupes';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Aucun';

  @override
  String get questStateAvailable => 'Disponible';

  @override
  String get questStateRunning => 'En cours';

  @override
  String get questStateSucceeded => 'Réussie';

  @override
  String get questStateFailed => 'Échouée';

  @override
  String get questStateUnknown => 'inconnu';

  @override
  String get dialogKnowledge => 'Connaissances de dialogue';

  @override
  String get resetKnowledgeChanges =>
      'Réinitialiser les modifications de connaissances';

  @override
  String get addNpc => 'Ajouter un PNJ';

  @override
  String get searchNpcs => 'Rechercher des PNJ';

  @override
  String get npcStatusRowLabel => 'État';

  @override
  String get npcStatusAlive => 'vivant';

  @override
  String get npcStatusDead => 'mort';

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PV $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Ressusciter';

  @override
  String get npcReviveQueued => 'Sera ressuscité à la sauvegarde';

  @override
  String entriesForCharacter(String name) {
    return 'Entrées — $name';
  }

  @override
  String get selectNpcToSeeEntries =>
      'Sélectionnez un PNJ pour voir les entrées';

  @override
  String get addKnowledgeEntry => 'Ajouter une entrée de connaissance';

  @override
  String get browseCatalog => 'Parcourir le catalogue';

  @override
  String get alreadyExistsForCharacter => 'Existe déjà pour ce personnage.';

  @override
  String get alreadyInPendingChanges =>
      'Déjà dans les modifications en attente.';

  @override
  String duplicateCheckFailed(String error) {
    return 'La vérification des doublons a échoué — réessayez : $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Ajouts en attente ($count)';
  }

  @override
  String get undoAdd => 'Annuler l\'ajout';

  @override
  String get undoRemove => 'Annuler la suppression';

  @override
  String get removeEntry => 'Supprimer l\'entrée';

  @override
  String get selectNpcFromList => 'Sélectionnez un PNJ dans la liste';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Événements mémoriels';

  @override
  String get searchCharacters => 'Rechercher des personnages';

  @override
  String eventsForCharacter(String name) {
    return 'Événements — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Sélectionnez un personnage pour voir les événements';

  @override
  String get noTags => '(aucune balise)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Supprimer l\'événement';

  @override
  String get removeMemoryEventTitle => 'Supprimer l\'événement mémoriel ?';

  @override
  String get removeMemoryEventBody =>
      'Supprimer cet événement mémoriel ? Une sauvegarde est créée au préalable.';

  @override
  String get duplicateEvent => 'Dupliquer l\'événement';

  @override
  String get duplicateMemoryEventTitle => 'Dupliquer l\'événement mémoriel ?';

  @override
  String get duplicateMemoryEventBody =>
      'Dupliquer cet événement mémoriel ? Une sauvegarde est créée au préalable.';

  @override
  String get selectCharacterFromList =>
      'Sélectionnez un personnage dans la liste';

  @override
  String get factionsSidebar => 'Factions';

  @override
  String get factionsForgiveButton => 'Pardonner';

  @override
  String get factionHostile => 'Hostile';

  @override
  String get factionFriendly => 'Amical';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count meurtres',
      one: '$count meurtre',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count agressions',
      one: '$count agression',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count vols',
      one: '$count vol',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count intrusions',
      one: '$count intrusion',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count menaces',
      one: '$count menace',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count autres crimes',
      one: '$count autre crime',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'pardon en cours…';

  @override
  String get factionsEmpty => 'Aucun crime non réglé contre les factions.';

  @override
  String get factionGuildOldCamp => 'Vieux Camp';

  @override
  String get factionGuildNewCamp => 'Nouveau Camp';

  @override
  String get factionGuildSwampCamp => 'Camp du Marais';

  @override
  String get factionGuildOther => 'Autres/individus';

  @override
  String get allDataLockedBody =>
      'Le navigateur de propriétés complet nécessite des données privées décodées par le codec.';

  @override
  String get allDataDescription =>
      'Recherchez chaque propriété typée par nom ou par chemin. Les scalaires, chaînes, énumérations et chemins d\'objets sont modifiables ; les structs sont affichés en lecture seule pour l\'instant.';

  @override
  String get searchPropertiesLabel =>
      'Rechercher des propriétés (vide = tout lister) — p. ex. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Décodage de la sauvegarde…';

  @override
  String get decodingSaveBody =>
      'Décodage de l\'ensemble des données privées pour la première recherche. Cette opération s\'exécute une fois par sauvegarde, puis les recherches sont instantanées.';

  @override
  String get searchTheSaveTitle => 'Rechercher dans la sauvegarde';

  @override
  String get searchTheSaveBody =>
      'Saisissez un nom de propriété et appuyez sur Entrée. Laissez vide pour tout lister.';

  @override
  String get searchFailedTitle => 'Échec de la recherche';

  @override
  String get noMatchesTitle => 'Aucun résultat';

  @override
  String get noMatchesBody =>
      'Aucun chemin de propriété ne contenait tous ces termes.';

  @override
  String get value => 'Valeur';

  @override
  String get backupsTitle => 'Sauvegardes';

  @override
  String get refreshBackups => 'Actualiser les sauvegardes';

  @override
  String get noBackupsTitle => 'Aucune sauvegarde';

  @override
  String get noBackupsBody =>
      'Les sauvegardes modifiées créent des fichiers de sauvegarde à côté de l\'emplacement sélectionné.';

  @override
  String get slotBackups => 'Sauvegardes de l\'emplacement';

  @override
  String get profileBackups => 'Sauvegardes du profil';

  @override
  String get backupFactName => 'Nom';

  @override
  String get backupFactSlot => 'Emplacement';

  @override
  String get backupFactCreated => 'Créé le';

  @override
  String get backupFactSize => 'Taille';

  @override
  String get backupFactStatus => 'Statut';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restaurer $fileName';
  }

  @override
  String get appearanceTitle => 'Apparence';

  @override
  String get theme => 'Thème';

  @override
  String get themeLight => 'Clair';

  @override
  String get themeDark => 'Sombre';

  @override
  String get themeSystem => 'Système';

  @override
  String get uiScale => 'Échelle de l\'interface';

  @override
  String get resetZoomTooltip => 'Réinitialiser le zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Astuce : Ctrl + / Ctrl - modifie le zoom partout dans l\'application.';

  @override
  String get language => 'Langue';

  @override
  String get updatesTitle => 'Mises à jour';

  @override
  String get checkForUpdatesAutomatically =>
      'Vérifier automatiquement les mises à jour';

  @override
  String get checkForUpdatesNow => 'Vérifier les mises à jour maintenant';

  @override
  String get updatesPortableNotice =>
      'La version portable ouvre la page de téléchargement dans votre navigateur. Remplacez vos fichiers actuels par le nouveau téléchargement.';

  @override
  String get updateAvailableTitle => 'Mise à jour disponible';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'La version $version est disponible. Vous avez la $current.';
  }

  @override
  String get updateDownload => 'Télécharger';

  @override
  String get updateLater => 'Plus tard';

  @override
  String get updateUpToDate => 'Vous utilisez la dernière version.';

  @override
  String get updateCheckFailed =>
      'Impossible de rechercher des mises à jour. Veuillez réessayer plus tard.';

  @override
  String get gameTextTitle => 'Texte du jeu';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extrait : $ids identifiants pour $languages langues.';
  }

  @override
  String get gameTextExtracted => 'Le texte localisé du jeu est extrait.';

  @override
  String get gameTextNotExtracted =>
      'Le texte localisé du jeu n\'est pas encore extrait.';

  @override
  String get extracting => 'Extraction…';

  @override
  String get extractRefreshLocalizedText =>
      'Extraire / actualiser le texte localisé';

  @override
  String get extractLocalizedTextTitle => 'Extraire le texte localisé du jeu ?';

  @override
  String get extractLocalizedTextBody =>
      'Le texte localisé du jeu n\'est pas encore extrait. L\'extraire maintenant depuis votre installation du jeu ? (facultatif)';

  @override
  String get notNow => 'Pas maintenant';

  @override
  String get extract => 'Extraire';

  @override
  String get extractionComplete => 'Extraction terminée';

  @override
  String get extractionFailed => 'Échec de l\'extraction';

  @override
  String get localizationCacheFileType => 'Cache de localisation';

  @override
  String get savegameDirectoryTitle => 'Répertoire des sauvegardes';

  @override
  String get folder => 'Dossier';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Vérifier';

  @override
  String get roundtrip => 'Aller-retour';

  @override
  String get noCodecStatus => 'Aucun statut de codec';

  @override
  String get codecReady => 'Codec prêt';

  @override
  String get codecReadOnly => 'Codec en lecture seule';

  @override
  String get codecUnavailable => 'Codec indisponible';

  @override
  String get details => 'Détails';

  @override
  String codecStatusLine(String status) {
    return 'Statut : $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Décompression : $decompress | Compression : $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend : $backend';
  }

  @override
  String get yes => 'oui';

  @override
  String get no => 'non';

  @override
  String get aboutSubtitle => 'Éditeur de sauvegardes Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 contributeurs de goresave';

  @override
  String get aboutLicense => 'Sous licence MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Difficulté — $profile';
  }

  @override
  String get difficultyNoProfile => 'Aucun profil';

  @override
  String get difficultyNoDifficulty => 'Aucune difficulté';

  @override
  String get difficultyLabel => 'Difficulté';

  @override
  String get difficultyTooltipNoProfile => 'Aucun profil sélectionné';

  @override
  String get difficultyTooltipEdit => 'Modifier la difficulté pour ce profil';

  @override
  String get difficultyTooltipNoEditable =>
      'Ce profil n\'a pas de difficulté modifiable';

  @override
  String get preset => 'Préréglage';

  @override
  String get presetNovice => 'Novice';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difficile';

  @override
  String get presetCustom => 'Personnalisé';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Le préréglage enregistré n\'est pas reconnu ($preset). Vous pouvez quand même enregistrer les modifications de l\'Assistant de combat / Permadeath, ou choisir un préréglage ci-dessus pour l\'écraser.';
  }

  @override
  String get closeCombatFlowHelper => 'Assistant de combat rapproché';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Non disponible en mode Novice';

  @override
  String get levelCombat => 'Combat';

  @override
  String get levelResources => 'Ressources';

  @override
  String get levelProgression => 'Progression';

  @override
  String get difficultyAppliesToAllSaves =>
      'La difficulté s\'applique à toutes les sauvegardes de ce profil.';

  @override
  String get savingDifficultyFailed =>
      'L\'enregistrement de la difficulté a échoué.';

  @override
  String get addItemDialogTitle => 'Ajouter un objet';

  @override
  String get searchItems => 'Rechercher des objets';

  @override
  String failedToLoadCatalog(String error) {
    return 'Échec du chargement du catalogue : $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Aucun objet disponible à ajouter';

  @override
  String get noItemsMatch => 'Aucun objet correspondant';

  @override
  String get countMustBeAtLeast1 => 'Doit être ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Doit être ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Ajouter un PNJ';

  @override
  String get noNpcsAvailableToAdd => 'Aucun PNJ disponible à ajouter';

  @override
  String get noNpcsMatch => 'Aucun PNJ correspondant';

  @override
  String get categoryAll => 'Tous';

  @override
  String allWithCount(int count) {
    return 'Tous ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle =>
      'Ajouter une entrée de connaissance';

  @override
  String get searchEntries => 'Rechercher des entrées';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Aucune entrée de connaissance disponible à ajouter';

  @override
  String get noEntriesMatch => 'Aucune entrée correspondante';

  @override
  String get heroGroupMainStats => 'Statistiques principales';

  @override
  String get heroGroupCombatSkills => 'Compétences de combat';

  @override
  String get heroGroupResistances => 'Résistances';

  @override
  String get heroGroupThieving => 'Vol';

  @override
  String get heroGroupAdvanced => 'Avancé';

  @override
  String get heroEntryHeroTransform => 'Transform du héros';

  @override
  String attributeEmpty(String name) {
    return '$name est vide — saisissez une valeur ou restaurez la valeur d\'origine avant d\'enregistrer.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Nombre invalide pour $name : « $text »';
  }

  @override
  String get loadingEditorData => 'Chargement des données de l\'éditeur';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount identifiants extraits dans $languageCount langues';
  }
}
