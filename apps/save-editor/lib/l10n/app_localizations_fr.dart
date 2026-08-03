// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get debugSectionTitle => 'Avancé (débogage)';

  @override
  String get debugSectionSubtitle =>
      'Diagnostic et données brutes pour les rapports de bogue';

  @override
  String get showObjectIdsTitle =>
      'Afficher les identifiants techniques supplémentaires';

  @override
  String get showObjectIdsSubtitle =>
      'Affiche les identifiants techniques des objets, connaissances de dialogue, quêtes et acteurs orphelins. Les identifiants des PNJ sont toujours affichés.';

  @override
  String get storyStateSidebar => 'État de l’histoire';

  @override
  String get storyStateDescription =>
      'Catalogue de référence des états persistants déclarés par les scripts livrés avec le jeu. Les entrées enregistrées affichent leur valeur brute ; les champs du catalogue absents de cette sauvegarde sont marqués comme non définis. Les repères temporels déclarés dans le code sont affichés en temps de jeu ; les autres entiers peuvent être des booléens, compteurs ou états à plusieurs niveaux.';

  @override
  String get storyStateReadOnly =>
      'Lecture seule tant que la signification des valeurs dans les scripts et l’écriture sûre de la map ne sont pas établies. Le texte de glossaire associé donne du contexte ; ce n’est pas une traduction directe de l’ID technique.';

  @override
  String get storyStateStructureReadOnly =>
      'La structure StoryPropertyValues de cette sauvegarde n’a pas pu être identifiée de manière univoque et sûre. Les valeurs d’histoire restent en lecture seule pour cette sauvegarde.';

  @override
  String get storyStateSearch => 'Rechercher dans l’état de l’histoire';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown valeurs d’histoire sur $total';
  }

  @override
  String get storyStateInteger => 'Entier';

  @override
  String get storyStateTimeMarker => 'Repère temporel';

  @override
  String get storyStateChapter => 'Chapitre';

  @override
  String get storyStateUnknown => 'Type source inconnu';

  @override
  String get storyStateUnknownDetail =>
      'Cet ID enregistré est absent du catalogue de scripts actuel (par exemple à cause d’un mod ou d’une version plus récente du jeu). Sa valeur sérialisée est un int32, mais sa signification n’est pas déduite.';

  @override
  String get storyStateStored => 'Enregistré';

  @override
  String get storyStateUnset => 'Non défini';

  @override
  String get storyStateUnsetDetail =>
      'Ce champ du catalogue n’est pas sérialisé dans cette sauvegarde ; le jeu utilise donc son état non défini ou sa valeur par défaut.';

  @override
  String get storyStateRawValue => 'Valeur brute';

  @override
  String storyStateElapsed(String duration) {
    return 'Temps écoulé à la sauvegarde : $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Dans le futur à la sauvegarde : $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days jours',
      one: '1 jour',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Entrée de glossaire associée';

  @override
  String get storyStateTechnicalPath => 'Chemin technique';

  @override
  String get storyStateEditingGuidance =>
      'Chaque entrée reste modifiable sur toute la plage int32 signée. Les indicateurs et suggestions de valeurs issus des scripts sont fournis à titre indicatif ; la saisie brute reste toujours disponible. Les changements d’état de l’histoire peuvent court-circuiter des transitions de dialogue, de quête ou du monde : enregistrez-les avec précaution. Une sauvegarde de sécurité est créée automatiquement.';

  @override
  String get storyStatePending => 'En attente';

  @override
  String storyStatePendingValue(String value) {
    return 'Sera enregistré sous la valeur $value';
  }

  @override
  String get storyStatePendingRemoval => 'Sera supprimé de la sauvegarde';

  @override
  String get storyStateEditValue => 'Modifier la valeur';

  @override
  String get storyStateSetValue => 'Définir la valeur';

  @override
  String get storyStateRemoveValue => 'Supprimer de la sauvegarde';

  @override
  String get storyStateUndoChange => 'Annuler le changement d’histoire';

  @override
  String get storyStateResetChanges =>
      'Réinitialiser les changements d’histoire';

  @override
  String storyStateDialogTitle(String id) {
    return 'Modifier $id';
  }

  @override
  String get storyStateRawInput => 'Valeur int32 signée';

  @override
  String get storyStateInvalidInt32 =>
      'Saisissez un nombre entier compris entre -2147483648 et 2147483647.';

  @override
  String get storyStateQueueChange => 'Mettre le changement en attente';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valeurs attestées dans les scripts fournis : $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Les suggestions ne sont pas des limites de validation ; le code natif, les mods ou les versions ultérieures du jeu peuvent utiliser d’autres valeurs.';

  @override
  String get storyStateUseCurrentTime =>
      'Utiliser l’heure actuelle de la sauvegarde';

  @override
  String get storyStateStructuredTime => 'Jour / heure';

  @override
  String get storyStateRawMode => 'int32 brut';

  @override
  String get storyStateChapterWarning =>
      'Modifier uniquement le chapitre ne synchronise ni les quêtes, ni les PNJ, ni l’inventaire, ni l’état du monde.';

  @override
  String get storyStateDormantWarning =>
      'Aucune lecture ni écriture active de ce champ n’a été trouvée dans le cache des scripts fournis. Il peut être ancien, contrôlé par le code natif ou réservé.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Les scripts fournis lisent ce champ, mais ne contiennent aucune écriture par script. Le code natif peut néanmoins en être responsable.';

  @override
  String get storyStateUnknownEditWarning =>
      'Cet ID provenant d’un mod ou d’une version ultérieure ne possède aucune sémantique source intégrée. Modifiez uniquement sa valeur int32 brute.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Indicateur binaire',
      'finiteState': 'Valeur à plusieurs états',
      'counterOrScore': 'Compteur / score',
      'calendarDay': 'Jour calendaire',
      'derivedOrOpaqueInteger': 'Entier dérivé / opaque',
      'readOnlyInSourceInteger': 'Lecture seule dans les scripts fournis',
      'dormantOrLegacyInteger': 'Inutilisé dans les scripts fournis',
      'other': 'Entier',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Un 0 enregistré et une entrée absente de la table sont deux états de fichier distincts. « Supprimer de la sauvegarde » restaure l’état défini par le constructeur ou l’état par défaut.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo de GORE Save Editor';

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
  String get heroGroupSkills => 'Compétences';

  @override
  String get skillsNoneBody => 'Aucune compétence trouvée pour ce personnage.';

  @override
  String get skillsUnavailableBody =>
      'Les compétences ne peuvent pas être modifiées dans cette sauvegarde — le héros n\'a aucune donnée d\'effet à modifier.';

  @override
  String get skillNotLearned => 'Non apprise';

  @override
  String get skillLearn => 'Apprendre';

  @override
  String get skillActionLearn => 'apprendre';

  @override
  String get skillActionUnlearn => 'oublier';

  @override
  String get skillTierUntrained => 'Non entraîné';

  @override
  String get skillTierBeginner => 'Débutant';

  @override
  String get skillTierTrained => 'Entraîné';

  @override
  String get skillTierMaster => 'Maître';

  @override
  String get skillTierNovice => 'Novice';

  @override
  String get skillTierAmateur => 'Amateur (Cercle 0)';

  @override
  String get skillTierLearned => 'Apprise';

  @override
  String skillTierCircle(int n) {
    return 'Cercle $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Armes à une main';

  @override
  String get skillHintBlacksmith2H => 'Armes à deux mains';

  @override
  String get skillCategoryCombat => 'Combat';

  @override
  String get skillCategoryCrafting => 'Artisanat';

  @override
  String get skillCategoryHunting => 'Chasse';

  @override
  String get skillCategoryLanguage => 'Langue';

  @override
  String get skillCategoryMagic => 'Magie';

  @override
  String get skillCategoryMovement => 'Déplacement';

  @override
  String get skillCategoryThievery => 'Vol';

  @override
  String get skillNameOneHanded => 'À une main';

  @override
  String get skillNameTwoHanded => 'À deux mains';

  @override
  String get skillNameFists => 'Poings';

  @override
  String get skillNameBow => 'Arc';

  @override
  String get skillNameCrossbow => 'Arbalète';

  @override
  String get skillNameLockpicking => 'Crochetage de serrures';

  @override
  String get skillNamePickpocketing => 'Vol à la tire';

  @override
  String get skillNameTakeOrgans => 'Extraire les organes';

  @override
  String get skillNameBreakTeeth => 'Extraire les dents';

  @override
  String get skillNameTakeClaws => 'Extraire les griffes';

  @override
  String get skillNameSkinFur => 'Prélever la fourrure';

  @override
  String get skillNameSkin => 'Prélever la peau';

  @override
  String get skillNameTakeFins => 'Prélever les voiles dorsales';

  @override
  String get skillNameTakeStingers => 'Extraire les aiguillons';

  @override
  String get skillNameTakeSecretion => 'Extraire les sécrétions';

  @override
  String get skillNameTakeSkullPlates => 'Prélever l’armure crânienne';

  @override
  String get skillNameSkinSwampshark => 'Prélever la peau de requin';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Prélever les plaques';

  @override
  String get skillNameTakeScutes => 'Prélever les écailles';

  @override
  String get skillNameTakeUluMulu => 'Prendre l’Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Armes orques';

  @override
  String get skillNameMining => 'Extraction de minerai';

  @override
  String get skillNameDiving => 'Plongée';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Prélever les mandibules';

  @override
  String get skillNameTakeShadowbeastHorn => 'Prélever la corne (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Extraire l’épine dorsale';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extraire les dents de requin';

  @override
  String get skillNameTakeFireTongue => 'Extraire la langue de feu';

  @override
  String get skillNameTakeTrollHorn => 'Prélever la corne (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobatie';

  @override
  String get skillNameWallClimbing => 'Escalade';

  @override
  String get skillNameRiding => 'Monte à dos de charognard';

  @override
  String get skillNameSneaking => 'Déplacement furtif';

  @override
  String get skillNameAlchemy => 'Alchimie';

  @override
  String get skillNameRuneInscription => 'Inscription';

  @override
  String get skillNameBlacksmithing => 'Forge';

  @override
  String get skillNameMagicCircle => 'Cercle de la magie';

  @override
  String get skillNameOrcish => 'Langue orc';

  @override
  String get tabInventory => 'Inventaire';

  @override
  String get tabWorld => 'Monde';

  @override
  String get tabCharacters => 'Personnages';

  @override
  String get characterNoActorBody =>
      'Ce personnage n\'a pas d\'acteur dans le monde ; il n\'a donc ni attributs, ni inventaire, ni événements.';

  @override
  String get characterNoEventsBody => 'Aucun événement pour ce personnage.';

  @override
  String get characterOrphanGroup => 'Autres';

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
  String get otherSaves => 'Autres sauvegardes';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count sauvegardes)';
  }

  @override
  String get switchProfile => 'Changer de profil';

  @override
  String get openSaveFile => 'Ouvrir un fichier';

  @override
  String get externalSave => 'Sauvegarde ouverte depuis l’extérieur';

  @override
  String get saveProfileTitle => 'Profil de sauvegarde';

  @override
  String get saveProfileDescription =>
      'Attribuez cette sauvegarde à un autre profil de jeu. La sauvegarde et l’index des profils sont sauvegardés ensemble.';

  @override
  String get saveProfileExternalHint =>
      'Sélectionnez un profil pour importer ce fichier dans le dossier des sauvegardes du jeu et l’y enregistrer. Le fichier d’origine reste inchangé.';

  @override
  String get saveProfileNoProfiles =>
      'Aucun profil de jeu modifiable n’a été trouvé dans PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Sélectionner un profil';

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
  String bytesValue(String count) {
    return '$count octets';
  }

  @override
  String get inspectionJsonTitle => 'JSON d\'inspection';

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
  String get gameTimeTitle => 'Temps de jeu';

  @override
  String get gameTimeDay => 'Jour';

  @override
  String get gameTimeHours => 'Heures';

  @override
  String get gameTimeMinutes => 'Minutes';

  @override
  String get gameTimeSeconds => 'Secondes';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s au total';
  }

  @override
  String get gameTimeInvalid =>
      'Saisissez des nombres entiers : jour ≥ 0, heures 0–23, minutes et secondes 0–59.';

  @override
  String get required => 'Requis';

  @override
  String get playerLockedBody =>
      'Les modifications privées du joueur nécessitent un codec capable de compresser.';

  @override
  String get heroTransform => 'Position';

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
  String get spawnPositionSection => 'Position d’apparition (référence)';

  @override
  String get positionNotReadable =>
      'La position enregistrée de ce personnage n’a pas pu être lue.';

  @override
  String get npcPositionReadOnly =>
      'Le jeu restaure la position d’un PNJ à partir du niveau et non de la sauvegarde : ces valeurs peuvent être lues, mais pas modifiées.';

  @override
  String get pickLocation => 'Choisir un lieu…';

  @override
  String get pickLocationDialogTitle => 'Choisir un lieu';

  @override
  String get applySpotRotation => 'Appliquer aussi l’orientation du lieu';

  @override
  String get locationAreaOther => 'Autres';

  @override
  String get locationAreaCavalornValley => 'Vallée de Cavalorn';

  @override
  String get locationAreaEastForest => 'Forêt de l\'Est';

  @override
  String get locationAreaFogTower => 'Tour de brume';

  @override
  String get locationAreaIllegalWeedMixers => 'Mélangeurs d\'herbe clandestins';

  @override
  String get locationAreaOrcArena => 'Arène des orques';

  @override
  String get locationAreaOrcGraveyard => 'Cimetière orc';

  @override
  String get locationAreaShipwreck => 'Épave';

  @override
  String get locationAreaTundra => 'Toundra';

  @override
  String get locationCatalogUnavailable =>
      'Le catalogue des lieux n’a pas pu être chargé.';

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
  String get attributeBaseValue => 'Valeur de base';

  @override
  String get attributeCurrentValue => 'Valeur actuelle';

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
  String get resetInventoryButton => 'Réinitialiser l’inventaire';

  @override
  String get resetInventoryTooltipDefault =>
      'Remplacer cet inventaire par celui du début de partie';

  @override
  String get resetInventoryTooltipBlocked =>
      'Enregistrez ou annulez d’abord les modifications d’inventaire en attente';

  @override
  String get pendingResetTitle =>
      'Réinitialiser à l’inventaire de début de partie';

  @override
  String pendingResetSubtitle(String level) {
    return 'Niveau des ressources : $level';
  }

  @override
  String get cancelPendingReset => 'Annuler la réinitialisation';

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
  String get npcRelationshipRowLabel => 'Relation';

  @override
  String get npcRelationshipUnavailable => 'Statut de relation indisponible';

  @override
  String get npcRelationshipAutomatic => 'Calculée par le jeu';

  @override
  String get npcRelationshipAutomaticHint =>
      'Aucun statut permanent n’est enregistré. Le jeu évalue les règles de guilde, d’histoire, de zone et de crime.';

  @override
  String get npcRelationshipStoredHint =>
      'Enregistrée comme statut permanent du PNJ envers le joueur. Les règles de guilde, d’histoire, de zone et de crime peuvent encore modifier la relation effective dans le jeu.';

  @override
  String get npcRelationshipFriend => 'Ami';

  @override
  String get npcRelationshipNeutral => 'Neutre';

  @override
  String get npcRelationshipEnemy => 'Ennemi';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Sera $relationship lors de l’enregistrement';
  }

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
  String get memoryEventRemovalQueued =>
      'Suppression de l’événement mise en attente — cliquez sur Enregistrer pour l’appliquer.';

  @override
  String get duplicateEvent => 'Dupliquer l\'événement';

  @override
  String get duplicateMemoryEventTitle => 'Dupliquer l\'événement mémoriel ?';

  @override
  String get duplicateMemoryEventBody =>
      'Dupliquer cet événement mémoriel ? Une sauvegarde est créée au préalable.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplication de l’événement mise en attente — cliquez sur Enregistrer pour l’appliquer.';

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
  String get factionGuildOldCamp => 'Ancien camp';

  @override
  String get factionGuildNewCamp => 'Nouveau camp';

  @override
  String get factionGuildSwampCamp => 'Camp du marais';

  @override
  String get factionGuildOther => 'Autres/individus';

  @override
  String get allDataLockedBody =>
      'L’explorateur exhaustif des sources est actuellement disponible pour les sauvegardes GSAV.';

  @override
  String get allDataDescription =>
      'Parcourez les métadonnées GSAV et tous les nœuds typés PUBLIC/PRIVATE. Les valeurs scalaires et les structures natives sûres sont modifiables ; les conteneurs et les octets opaques restent visibles.';

  @override
  String get allDataEditable => 'Modifiable';

  @override
  String get allDataReadOnly => 'Lecture seule';

  @override
  String get allDataType => 'Type';

  @override
  String get allDataScalars => 'Valeurs scalaires';

  @override
  String get allDataStructs => 'Structures';

  @override
  String get allDataContainers => 'Conteneurs';

  @override
  String get allDataOpaque => 'Données opaques';

  @override
  String get allDataNodes => 'Nœuds';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sous-éléments',
      one: '1 sous-élément',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'En attente';

  @override
  String get allDataTagInputHint =>
      'Balises séparées par des virgules ou des sauts de ligne';

  @override
  String allDataTypedSource(String source) {
    return 'Source typée : $source';
  }

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
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 contributeurs de GORE';

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
  String get presetNovice => 'Facile';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difficile';

  @override
  String get presetCustom => 'Personnalisée';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Le préréglage enregistré n\'est pas reconnu ($preset). Vous pouvez quand même enregistrer les modifications de l\'Assistant de combat / Permadeath, ou choisir un préréglage ci-dessus pour l\'écraser.';
  }

  @override
  String get closeCombatFlowHelper => 'Aide à l’enchaînement combat rapproché';

  @override
  String get permadeath => 'Mort permanente';

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
  String get heroEntryHeroTransform => 'Position';

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
  String savingProgress(int done, int total) {
    return 'Enregistrement… $done sur $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount identifiants extraits dans $languageCount langues';
  }

  @override
  String get skillSmithing1H => 'Forge d’armes à une main';

  @override
  String get skillSmithing2H => 'Forge d’armes à deux mains';

  @override
  String get skillCircleNovice => 'Magicien novice';

  @override
  String get skillCircle1 => 'Premier Cercle de la magie';

  @override
  String get skillCircle2 => 'Deuxième Cercle de magie';

  @override
  String get skillCircle3 => 'Troisième Cercle de magie';

  @override
  String get skillCircle4 => 'Quatrième Cercle de la magie';

  @override
  String get skillCircle5 => 'Cinquième Cercle de la magie';

  @override
  String get skillCircle6 => 'Sixième Cercle de la magie';

  @override
  String get sectionGlossary => 'Glossaire';

  @override
  String get glossarySearch => 'Rechercher dans le glossaire';

  @override
  String get glossaryOldCamp => 'Ancien camp';

  @override
  String get glossaryNewCamp => 'Nouveau camp';

  @override
  String get glossarySwampCamp => 'Camp du marais';

  @override
  String get glossaryOutsiders => 'Étrangers';

  @override
  String get glossaryCreatures => 'Créatures';

  @override
  String get glossaryLocations => 'Lieux';

  @override
  String get glossaryFilterLabel => 'Filtre';

  @override
  String get glossaryFilterTraders => 'Marchands';

  @override
  String get glossaryFilterTeachers => 'Enseignants';

  @override
  String get glossaryFilterArmorers => 'Armuriers';

  @override
  String get glossaryFilterHostile => 'Hostiles';

  @override
  String get glossaryRelationshipFilterNote =>
      'Affiche les statuts d’ennemi permanents enregistrés dans la sauvegarde. Les relations dynamiques de guilde, d’histoire, de zone et de crime ne sont calculées que dans le jeu.';

  @override
  String get glossaryFilterDead => 'Morts';

  @override
  String get glossaryAddEntry => 'Ajouter une entrée au glossaire';

  @override
  String get glossaryAddTitle => 'Ajouter une entrée au glossaire';

  @override
  String get glossaryResetChanges =>
      'Réinitialiser les modifications du glossaire';

  @override
  String get glossaryNoVisibleEntries =>
      'Aucune entrée visible du glossaire ne correspond à cette vue.';

  @override
  String get glossaryNoHiddenEntries =>
      'Toutes les entrées disponibles sont déjà visibles.';

  @override
  String get glossaryNoMatch => 'Aucune entrée du glossaire ne correspond.';

  @override
  String get glossarySelectEntry =>
      'Sélectionnez une entrée du glossaire pour modifier ses sections.';

  @override
  String glossaryEntryCount(int count) {
    return '$count entrées';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked entrées sur $total';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portrait déverrouillé';

  @override
  String get glossaryPortraitSilhouette =>
      'Silhouette — portrait non déverrouillé';

  @override
  String get glossarySegments => 'Entrées';

  @override
  String get glossaryPending => 'Modification non enregistrée';

  @override
  String get glossaryShowFullText => 'Afficher le texte complet de l’entrée';

  @override
  String get glossarySegmentIntroduction => 'Introduction / portrait';

  @override
  String get glossarySegmentUnlock => 'Découverte';

  @override
  String glossarySegmentEntry(int number) {
    return 'Entrée $number';
  }

  @override
  String get questJournalAll => 'Toutes les quêtes';

  @override
  String get questJournalOldCamp => 'Ancien camp';

  @override
  String get questJournalNewCamp => 'Nouveau camp';

  @override
  String get questJournalSwampCamp => 'Camp du marais';

  @override
  String get questJournalColony => 'La Colonie';

  @override
  String get questJournalCompleted => 'Terminées';

  @override
  String get questJournalHint =>
      'Vue du journal en jeu. Les états internes et les quêtes non commencées restent disponibles sous Toutes les données.';

  @override
  String get questJournalNoEntries =>
      'Aucune quête du journal ne correspond aux filtres actuels.';

  @override
  String get glossaryTutorials => 'Tutoriels';

  @override
  String get tutorialGateNote =>
      'Ces lignes contrôlent les déverrouillages de tutoriel enregistrés. Un déverrouillage ne correspond pas nécessairement à une seule page de tutoriel en jeu.';

  @override
  String get tutorialResetChanges =>
      'Réinitialiser les modifications des tutoriels';

  @override
  String get tutorialNoGates =>
      'Aucun déverrouillage de tutoriel n’est disponible dans cette sauvegarde.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked tutoriels déverrouillés sur $total';
  }

  @override
  String get tutorialGateCombatBasics => 'Bases du combat';

  @override
  String get tutorialGateCrafting => 'Artisanat';

  @override
  String get tutorialGateCrime => 'Crimes et conséquences';

  @override
  String get tutorialGateDrugs => 'Consommables et effets';

  @override
  String get tutorialGateLockpicking => 'Crochetage';

  @override
  String get tutorialGateMagic => 'Magie';

  @override
  String get tutorialGateMap => 'Carte';

  @override
  String get tutorialGateMeleeCombat => 'Combat au corps à corps';

  @override
  String get tutorialGateNavigation => 'Déplacement et navigation';

  @override
  String get tutorialGatePerception => 'Perception';

  @override
  String get tutorialGatePlayerProgression => 'Progression du personnage';

  @override
  String get tutorialGateRanged => 'Combat à distance';

  @override
  String get tutorialGateRiding => 'Équitation';

  @override
  String get tutorialGateSleep => 'Sommeil';

  @override
  String get tutorialGateTrading => 'Commerce';

  @override
  String get windowMinimizeTooltip => 'Réduire';

  @override
  String get windowMaximizeTooltip => 'Agrandir';

  @override
  String get windowRestoreTooltip => 'Restaurer';

  @override
  String get fallbackDialogEntry => 'Entrée de dialogue';

  @override
  String get fallbackDialogChoice => 'Choix de dialogue';

  @override
  String get fallbackDialogTopic => 'Sujet de dialogue';

  @override
  String get fallbackDialogInformation => 'Information de dialogue';

  @override
  String get fallbackQuest => 'Quête';

  @override
  String get fallbackObjective => 'Objectif';

  @override
  String get fallbackItem => 'Objet';

  @override
  String get attributeSkillPointsFallback => 'Points d’apprentissage (PA)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Alcool',
      'AlcoholDepletionRate': 'Taux d’élimination de l’alcool',
      'MaxAlcohol': 'Niveau maximal d’alcool',
      'MaxSuperArmor': 'Super-armure maximale',
      'SuperArmor': 'Super-armure',
      'Fatigue': 'Fatigue',
      'FillRatio': 'Taux de remplissage',
      'FillRatioPeriod': 'Période de remplissage',
      'MaxFatigue': 'Fatigue maximale',
      'MaxThresholdIndex': 'Indice de seuil maximal',
      'RecoveryRatePerHourOfSleep': 'Récupération par heure de sommeil',
      'DamageMultiplier': 'Multiplicateur de dégâts',
      'Toughness': 'Robustesse',
      'ToughnessA': 'Robustesse A',
      'ToughnessB': 'Robustesse B',
      'ToughnessC': 'Robustesse C',
      'XPExecutedBounty': 'Récompense d’EXP pour une exécution',
      'XPKillOrDefeatBounty':
          'Récompense d’EXP pour une élimination ou une défaite',
      'SpeedModifier': 'Modificateur de vitesse',
      'CriticalLevelPercent': 'Niveau critique (%)',
      'MaxOxygen': 'Oxygène maximal',
      'Oxygen': 'Oxygène',
      'OxygenDepletionRate': 'Taux d’épuisement de l’oxygène',
      'OxygenRecoveryRate': 'Taux de récupération de l’oxygène',
      'MaxRestTime': 'Temps de repos maximal',
      'MaxSleepTime': 'Temps de sommeil maximal',
      'SleepTime': 'Temps de sommeil',
      'SleepTimeRecoveryAmount': 'Quantité récupérée pendant le sommeil',
      'SleepTimeRecoveryPeriod':
          'Intervalle de récupération pendant le sommeil',
      'MaxSwampweed': 'Quantité maximale d’herbe des marais',
      'Swampweed': 'Herbe des marais',
      'SwampweedDepletionRate': 'Taux d’épuisement de l’herbe des marais',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Réplique vocale';

  @override
  String get knowledgeTypeOther => 'Autre';

  @override
  String get armorUpgradeUpper => 'Haut';

  @override
  String get armorUpgradeMiddle => 'Milieu';

  @override
  String get armorUpgradeLower => 'Bas';

  @override
  String get knowledgeCategoryTopic => 'Sujet';

  @override
  String get knowledgeCategoryChoice => 'Choix';

  @override
  String get knowledgeCategoryInfo => 'Information';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Échec';

  @override
  String get missingSaveReference => 'Fichier manquant';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav est manquant. Il a peut-être été supprimé, déplacé ou renommé ; le profil le référence toujours.';
  }

  @override
  String get removeFromProfile => 'Retirer du profil';

  @override
  String get removeSaveFromProfileTitle => 'Retirer la sauvegarde du profil ?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Retirer $save de $profile ? Le fichier de sauvegarde lui-même sera conservé s’il existe encore.';
  }

  @override
  String get unassignedSave => 'Non attribuée à un profil';

  @override
  String get armorUpgradeLight => 'Légère';

  @override
  String get armorUpgradeMedium => 'Moyenne';

  @override
  String get armorUpgradeHeavy => 'Lourde';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversation imposée';

  @override
  String get knowledgeCaptionFollowupTopic => 'Sujet de suivi';

  @override
  String get knowledgeCaptionFallbackTopic => 'Sujet de secours';

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
      'Données de profil non valides';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Les métadonnées de la sauvegarde sélectionnée sont manquantes';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Inconnu';

  @override
  String editorUnexpectedError(String details) {
    return 'Erreur inattendue : $details';
  }

  @override
  String get editorOperationInProgress =>
      'Une autre opération est en cours. Réessayez dans un instant.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'La sauvegarde contient des modifications non enregistrées. Enregistrez-les ou réinitialisez-les avant de modifier la difficulté du profil.';

  @override
  String get editorNoSaveFolderSelected =>
      'Aucun dossier de sauvegarde sélectionné.';

  @override
  String get editorNoSaveSelected => 'Aucune sauvegarde sélectionnée.';

  @override
  String get coreUnknownError => 'Erreur interne inconnue';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Enregistrez ou réinitialisez d’abord vos modifications — changer de profil vous ferait quitter la sauvegarde actuelle.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Enregistrez ou réinitialisez vos modifications avant d’ouvrir un autre fichier.';

  @override
  String get editorSelectSavFile =>
      'Sélectionnez un fichier de sauvegarde .sav.';

  @override
  String get editorNotGothicGsav =>
      'Le fichier sélectionné n’est pas une sauvegarde Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Enregistrez ou réinitialisez vos modifications avant de changer le profil de la sauvegarde.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Enregistrez ou réinitialisez vos modifications avant de retirer une sauvegarde de son profil.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'La sauvegarde contient des modifications non enregistrées. Enregistrez-les ou réinitialisez-les avant de restaurer une copie de sauvegarde du profil.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Des modifications non enregistrées effectuées dans deux onglets ciblent la même propriété ($path). Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Une modification de segment du glossaire et une autre modification non enregistrée dans Toutes les données ciblent toutes deux le tableau Hero MemorizedEvents ($path). Les modifications du glossaire ajoutent ou suppriment des entrées dans ce tableau ; elles ne peuvent donc pas être enregistrées ensemble. Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Une modification de segment du glossaire et une autre modification non enregistrée ciblent la même propriété CurrentState d’une quête ($path). La modification du glossaire met elle-même cet état à jour. Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Une modification de relation et une autre modification non enregistrée dans Toutes les données ciblent toutes deux la même entrée de relation d’un PNJ ($path). La modification structurée de la relation peut remplacer des modificateurs dans cette entrée ; les deux modifications ne peuvent donc pas être enregistrées ensemble. Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Plusieurs modifications structurelles non enregistrées ciblent le même tableau ($path). Enregistrez ou réinitialisez la première modification avant d’en ajouter une autre.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Une modification structurelle d’événement et une autre modification non enregistrée dans Toutes les données ciblent toutes deux $path. Enregistrez ou réinitialisez l’une des deux avant de continuer.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Une modification des compétences et une modification dans Toutes les données portant sur le même effet de personnage (ActiveEffects › EffectSpec › Def) sont toutes deux en attente. Elles ne peuvent pas être enregistrées ensemble. Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';

  @override
  String get editorInventoryResetConflict =>
      'Une réinitialisation de l’inventaire et une autre modification du même inventaire sont toutes deux en attente. La réinitialisation remplace tout l’inventaire et annulerait l’autre modification. Réinitialisez ou annulez l’une des deux, puis enregistrez de nouveau.';

  @override
  String get editorUseFolder => 'Utiliser le dossier';

  @override
  String get editorGothicSavegameFileType => 'Sauvegarde Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Aucune modification de difficulté à enregistrer';

  @override
  String get editorDifficultyWritten =>
      'Difficulté enregistrée dans le profil (copie de sauvegarde créée)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count modifications enregistrées avec une copie de sauvegarde',
      one: '1 modification enregistrée avec une copie de sauvegarde',
    );
    return '$_temp0';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profil $profileId introuvable.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Aucun emplacement de sauvegarde libre n’est disponible dans le dossier des sauvegardes du jeu (G1R-001 à G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Sauvegarde importée et attribuée au profil $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Sauvegarde attribuée au profil $profileId (copies de sauvegarde associées créées)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'L’emplacement de sauvegarde $slot n’est pas attribué au profil $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Sauvegarde retirée du profil';

  @override
  String editorRestoredBackup(String path) {
    return 'Copie de sauvegarde restaurée : $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Copie de sauvegarde restaurée : $path (PersistentDataList.sav est resté inchangé : aucune copie associée correspondante ; les métadonnées de l’emplacement peuvent différer)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Vérification aller-retour du codec réussie : le bloc $chunkIndex a été recompressé à $bytes octets';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Impossible d’enregistrer la difficulté du profil : $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Impossible d’attribuer la sauvegarde au profil : $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Impossible de retirer la sauvegarde du profil : $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Impossible d’enregistrer les modifications : $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Impossible d’analyser les sauvegardes : $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Impossible d’inspecter la sauvegarde : $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Impossible de charger les copies de sauvegarde : $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Impossible de restaurer la copie de sauvegarde : $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Copie de sauvegarde restaurée : $path, mais le rechargement de la sauvegarde a échoué : $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Échec de la vérification du codec : $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Échec de la vérification aller-retour du codec : $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Échec de la recherche de propriétés : $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'La sauvegarde sélectionnée a changé pendant le chargement des attributs du héros.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Échec du chargement des compétences : $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Échec de la requête de progression : $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Échec du chargement de la liste des PNJ : $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Échec du chargement de la liste des personnages : $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Échec du chargement des attributs du PNJ : $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Échec du chargement de la position du PNJ : $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Échec du chargement de l’inventaire du PNJ : $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Échec du chargement de la liste des factions : $details';
  }

  @override
  String get editorNoBackupPath => 'aucun';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix : $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix : $backupPath ; copie de sauvegarde de PersistentDataList : $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Impossible d’obtenir l’état de la localisation : $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Échec de l’extraction : $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Échec du chargement du glossaire : $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Erreur de copie de sauvegarde : $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Quête',
      'document': 'Document',
      'story': 'Histoire',
      'exploration': 'Exploration',
      'combat': 'Combat',
      'social': 'Social',
      'item': 'Objets',
      'learning': 'Apprentissage',
      'guild': 'Guilde',
      'crime': 'Crime',
      'rest': 'Repos',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Quête commencée',
      'questSucceeded': 'Quête terminée',
      'questFailed': 'Quête échouée',
      'documentRead': 'Document lu',
      'documentSegmentUnlocked': 'Entrée découverte',
      'documentSegmentViewed': 'Entrée consultée',
      'chapterCompleted': 'Chapitre terminé',
      'areaEntered': 'Zone visitée',
      'areaLeft': 'Zone quittée',
      'characterKilled': 'Personnage tué',
      'characterDefeated': 'Personnage vaincu',
      'combatDodge': 'Attaque esquivée',
      'characterDebuffed': 'Affaiblissement appliqué',
      'tradeAvailable': 'Commerce débloqué',
      'itemObtained': 'Objet obtenu',
      'itemCrafted': 'Objet fabriqué',
      'skillStateRecorded': 'État des compétences enregistré',
      'recipeLearned': 'Recette apprise',
      'guildJoined': 'Guilde rejointe',
      'crimeRecorded': 'Crime enregistré',
      'slept': 'Sommeil',
      'storyEvent': 'Événement d’histoire',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action : $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'Temps de jeu',
      'duration': 'Durée',
      'chapter': 'Chapitre',
      'instigator': 'Déclenché par',
      'affected': 'Cible',
      'amount': 'Quantité',
      'primaryObject': 'Objet',
      'secondaryObject': 'Contexte',
      'segmentText': 'Texte de l’entrée',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Jour $day, $time';
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
  String get memoryEventHero => 'Héros';

  @override
  String get memoryEventDetails => 'Détails';

  @override
  String get memoryEventTags => 'Balises';

  @override
  String get memoryEventTechnicalData => 'Données techniques';

  @override
  String get memoryEventIndex => 'Index';

  @override
  String get memoryEventPosition => 'Position';

  @override
  String get memoryEventPayload => 'Données utiles';

  @override
  String get memoryEventSubject => 'Sujet';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Accès',
      'AccessDenied': 'Accès refusé',
      'AccesToTemple': 'Accès au temple',
      'Advice': 'Conseils',
      'AfterFight': 'Après le combat',
      'AfterFireMages': 'Après les Mages du Feu',
      'AfterNek': 'Après Nek',
      'AfterQuest': 'Après la quête',
      'Alone': 'Seul',
      'Amulet': 'Amulette',
      'Annoying': 'Ennuyeux',
      'Armor': 'Armure',
      'Avoid': 'Éviter',
      'Backstory': 'Histoire personnelle',
      'BackStory': 'Histoire personnelle',
      'BasicMagic': 'Magie de base',
      'Beated': 'Vaincu',
      'BecomeMercenary': 'Devenir mercenaire',
      'Beer': 'Bière',
      'Bestiary': 'Bestiaire',
      'Blessing': 'Bénédiction',
      'Boss': 'Chef',
      'Bully': 'Brute',
      'BullyAdvice': 'Conseil sur la brute',
      'Camp': 'Camp',
      'CampDivided': 'Camp divisé',
      'CareOfMessengers': 'S’occuper des messagers',
      'ChangeOpinion': 'Changement d’avis',
      'ChargeUriziel': 'Charger Uriziel',
      'Chosen': 'Élu',
      'Contact': 'Contact',
      'Courier': 'Messager',
      'CraftBows': 'Fabriquer des arcs',
      'Crazy': 'Fou',
      'DailyMeal': 'Repas quotidien',
      'DailyRation_Trader': 'Marchand de rations quotidiennes',
      'DAM': 'Barrage',
      'Dead': 'Mort',
      'Deal': 'Accord',
      'Dealer': 'Marchand',
      'Deceived': 'Trompé',
      'Dementia': 'Démence',
      'DenyAccess': 'Refuser l’accès',
      'DifferentOpinion': 'Opinion différente',
      'Discussion': 'Discussion',
      'DontTalk': 'Ne pas parler',
      'Duel': 'Duel',
      'Entrance': 'Entrée',
      'Escape': 'Fuite',
      'Extended': 'Étendu',
      'Extra': 'Supplémentaire',
      'ExtraInfo': 'Informations supplémentaires',
      'Fanatic': 'Fanatique',
      'Fight': 'Combat',
      'FindUlumulu': 'Trouver Ulu-Mulu',
      'FireMages': 'Mages du Feu',
      'FireMagesEscape': 'Fuite des Mages du Feu',
      'FiskNewDealer': 'Nouveau receleur pour Fisk',
      'FiskNewDealerCompleted': 'Nouveau receleur pour Fisk — terminé',
      'FogTower': 'Tour du Brouillard',
      'Food': 'Nourriture',
      'Forgave': 'A pardonné',
      'Forgive': 'Pardonner',
      'Forgiven': 'Pardonné',
      'FourFriends': 'Quatre amis',
      'FreeHut': 'Cabane libre',
      'FreeMine': 'Mine libre',
      'Fury': 'Fureur',
      'GoodTeacher': 'Bon professeur',
      'Gossip': 'Potins',
      'GotScavenger': 'Charognard obtenu',
      'GrantedAccess': 'Accès accordé',
      'GRDArmor': 'Armure de garde',
      'Guide': 'Guide',
      'HateMages': 'Haine des mages',
      'HateMagesExplanation': 'Explication de la haine des mages',
      'HateRiceLord': 'Haine du Seigneur des rizières',
      'Heal': 'Soins',
      'Healing': 'Soin',
      'Help': 'Aide',
      'Helper': 'Aide',
      'HelpKagan': 'Aider Kagan',
      'HutStory': 'Histoire de cabane',
      'Ignore': 'Ignorer',
      'Impress': 'Impressionner',
      'ImpressAlchemy': 'Impressionner avec l’alchimie',
      'ImpressInscription': 'Impressionner avec les inscriptions',
      'Info': 'Informations',
      'Interested': 'Intéressé',
      'Introduction': 'Introduction / portrait',
      'Introduction_2': 'Introduction / portrait 2',
      'Introduction_Armor': 'Introduction à l’armure',
      'Introduction_Teacher': 'Introduction : enseignant',
      'Introduction_Trader': 'Introduction : marchand',
      'Invocation': 'Invocation',
      'JoinSC': 'Rejoindre le Camp du marais',
      'Joint': 'Joint',
      'KalomCamp': 'Campement de Kalom',
      'Leader': 'Chef',
      'Learning': 'Apprentissage',
      'LearnOrcish': 'Apprendre la langue orc',
      'LeftParty': 'A quitté le groupe',
      'Library': 'Bibliothèque',
      'Lie': 'Mensonge',
      'Lock': 'Serrure',
      'Lockpick': 'Rossignol',
      'Mad': 'Fou',
      'Mandibles': 'Mandibules de mante des galeries',
      'MapMaker': 'Cartographe',
      'Monastery': 'Monastère',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Nouveau camp',
      'NewCamper': 'Nouveau au camp',
      'NewLeader': 'Nouveau chef',
      'NightPatrol': 'Patrouille de nuit',
      'NotInterested': 'Pas intéressé',
      'OldCamp': 'Ancien camp',
      'OrcEnclaveEntrance': 'Entrée de l’enclave orque',
      'OrcGraveyard': 'Cimetière des Orques',
      'OreArmor': 'Armure de minerai',
      'Party': 'Groupe',
      'Pay': 'Payer',
      'PayMoney': 'Payer de l’argent',
      'Permission': 'Autorisation',
      'Pet': 'Animal de compagnie',
      'PreparingInvocation': 'Préparation de l’invocation',
      'Quest': 'Quête',
      'RankUpFireMages': 'Promotion de Mage du Feu',
      'RankUpGuard': 'Promotion de garde',
      'RanUpFireMagesCompleted': 'Promotion de Mage du Feu terminée',
      'Realocated': 'Réinstallé',
      'Reason': 'Raison',
      'Respect': 'Respect',
      'ReturnToSC': 'Retour au Camp du marais',
      'RicelordForeman': 'Contremaître du Seigneur des rizières',
      'RideScavenger': 'Monter le charognard',
      'Robe': 'Robe',
      'Safe': 'En sécurité',
      'Scraper': 'Mineur',
      'SecondChance': 'Deuxième chance',
      'SecretLocation': 'Emplacement secret',
      'SecretPassage': 'Passage secret',
      'SecretPath': 'Chemin secret',
      'SleeperFollower': 'Adepte du Dormeur',
      'SleeperTemple': 'Temple du Dormeur',
      'SmallInfo': 'Brève information',
      'Stonehenge': 'Mégalithes',
      'StopFollowing': 'Ne plus suivre',
      'SwampCamp': 'Camp du marais',
      'Talkative': 'Bavard',
      'Teach': 'Enseigner',
      'TeachBow': 'Enseigner le tir à l’arc',
      'Teacher': 'Enseignant',
      'Teacher2': 'Enseignant 2',
      'TeacherInscription': 'Enseignant des inscriptions',
      'TeacherMana': 'Enseignant du mana',
      'TeachIchor': 'Enseigner l’extraction de l’ichor des mantes des galeries',
      'TeachMagic': 'Enseigner la magie',
      'TeachOrcish': 'Enseigner la langue orc',
      'TeachStats': 'Enseigner les attributs',
      'TeachWeapon': 'Enseigner le maniement des armes',
      'Teleport': 'Téléportation',
      'TheMysteriousOrc': 'L’Orc mystérieux',
      'ThroneRoom': 'Salle du Trône',
      'TradeBow': 'Commerce d’arcs',
      'Trader': 'Commerçant',
      'TradeSkins_Trader': 'Marchand de peaux',
      'Traitor': 'Traître',
      'Trial': 'Épreuve',
      'TrollCanyon': 'Canyon gardé par un troll',
      'Trust': 'Confiance',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Inexpérimenté',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Rune d’Uriziel',
      'Useful': 'Utile',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrations',
      'WaitFreeMine': 'Attendre à la Mine Libre',
      'WaitInTrainingArea': 'Attendre dans la zone d’entraînement',
      'Warning': 'Avertissement',
      'WarningTooLate': 'Avertissement trop tardif',
      'WaterMessenger': 'Messager des Mages de l’Eau',
      'Weapon': 'Arme',
      'Who': 'Qui',
      'Women': 'Femmes',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Emplacements d’inventaire endommagés';

  @override
  String slotRepairBody(int count) {
    return 'Cette sauvegarde contient $count emplacements d’inventaire dont l’identifiant ne correspond plus à leur position — dans le jeu, jeter un tel objet en supprime un autre. La réparation ne réécrit que les identifiants : aucun objet n’est ajouté, supprimé ni modifié. Une sauvegarde de secours est créée à l’enregistrement, comme toujours.';
  }

  @override
  String get slotRepairQueued =>
      'Réparation en attente — enregistrez pour l’appliquer.';

  @override
  String get slotRepairAction => 'Réparer';

  @override
  String get slotRepairDiscard => 'Annuler';

  @override
  String get editorInventorySlotRepairConflict =>
      'Une modification d’inventaire qui renumérote les emplacements et une modification directe d’un identifiant d’emplacement sont toutes deux en attente. La renumérotation supprimerait la modification directe — annulez l’une des deux, puis enregistrez de nouveau.';

  @override
  String get backupFactFile => 'Fichier';

  @override
  String get renameBackupTooltip => 'Nommer cette sauvegarde';

  @override
  String get renameBackupTitle => 'Nommer la sauvegarde';

  @override
  String get renameBackupLabel => 'Nom';

  @override
  String renameBackupHelp(String fileName) {
    return 'Affiché à la place du nom de fichier $fileName. Laissez vide pour supprimer le nom ; le fichier lui-même n’est pas renommé.';
  }

  @override
  String get deleteBackupTooltip => 'Supprimer cette sauvegarde';

  @override
  String get deleteBackupTitle => 'Supprimer la sauvegarde';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Supprimer « $name » ($fileName) ? Le fichier est effacé du disque et ne peut pas être récupéré.';
  }

  @override
  String get deleteBackupConfirm => 'Supprimer';

  @override
  String editorDeletedBackup(String path) {
    return 'Sauvegarde supprimée : $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Impossible de supprimer la sauvegarde : $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Impossible de nommer la sauvegarde : $details';
  }
}
