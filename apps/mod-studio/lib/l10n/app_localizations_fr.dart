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
}
