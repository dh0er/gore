// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Dialogs';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Textures';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'All';

  @override
  String get sectionItemValues => 'Item values';

  @override
  String get sectionLocalizedText => 'Localized text';

  @override
  String get audioCatCreatures => 'Creatures';

  @override
  String get audioCatObjects => 'Objects';

  @override
  String get audioCatMagic => 'Magic';

  @override
  String get audioCatMovement => 'Movement';

  @override
  String get audioCatWorld => 'World';

  @override
  String get audioCatAction => 'Action';

  @override
  String get audioCatCombat => 'Combat';

  @override
  String get audioCatPhysics => 'Physics';

  @override
  String get audioCatItems => 'Items';

  @override
  String get audioCatUi => 'UI';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Underwater';

  @override
  String get audioCatVision => 'Vision';

  @override
  String get audioCatDialog => 'Dialog';

  @override
  String get audioCatOther => 'Other';

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
  String get extractLocalizedText => 'Extract localized text';

  @override
  String get lightMode => 'Light mode';

  @override
  String get darkMode => 'Dark mode';

  @override
  String get language => 'Language';

  @override
  String get exportMod => 'Export mod';

  @override
  String exportModWithCount(int count) {
    return 'Export mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Select an item to edit its fields.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Game data: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Game data: bundled';

  @override
  String get loadGameDataDump => 'Load game-data dump…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json from the gore-dump mod';

  @override
  String get useBundledData => 'Use bundled data';

  @override
  String get alreadyBundled => 'already bundled';

  @override
  String get gameDataFileGroupLabel => 'game data';

  @override
  String get minimize => 'Minimize';

  @override
  String get restore => 'Restore';

  @override
  String get maximize => 'Maximize';

  @override
  String get close => 'Close';

  @override
  String get about => 'About';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE contributors';

  @override
  String get aboutLicense => 'Licensed under the MIT License.';

  @override
  String get categoryMeleeWeapons => 'Melee weapons';

  @override
  String get categoryRangedWeapons => 'Ranged weapons';

  @override
  String get categoryAmmunition => 'Ammunition';

  @override
  String get categoryRunes => 'Runes';

  @override
  String get categorySpellScrolls => 'Spell scrolls';

  @override
  String get categoryFoodAndPotions => 'Food & potions';

  @override
  String get categoryMiscellaneous => 'Miscellaneous';

  @override
  String get categoryAmulets => 'Amulets';

  @override
  String get categoryRings => 'Rings';

  @override
  String get categoryAnimalTrophies => 'Animal trophies';

  @override
  String get categoryWritings => 'Writings';

  @override
  String get categoryMissionItems => 'Mission items';

  @override
  String get categoryKeys => 'Keys';

  @override
  String get categoryOther => 'Other';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Search items';

  @override
  String get noItemsMatch => 'No items match';

  @override
  String failedToLoadCatalog(String error) {
    return 'Failed to load catalog: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Pending overrides ($count)';
  }

  @override
  String get clearAll => 'Clear all';

  @override
  String get noPendingOverrides =>
      'No pending overrides.\nEdit item fields to add some.';

  @override
  String get removeOverride => 'Remove override';

  @override
  String get searchChanges => 'Search changes';

  @override
  String get noChangesMatch => 'No changes match';

  @override
  String get clearSection => 'Clear this group';

  @override
  String get modName => 'Mod name';

  @override
  String get loadDelayLabel => 'Load delay (ms, 0 = instant)';

  @override
  String get noFolderSelected => 'No folder selected';

  @override
  String get chooseFolder => 'Choose folder';

  @override
  String get packageAsZip => 'Package as .zip';

  @override
  String get cancel => 'Cancel';

  @override
  String get export => 'Export';

  @override
  String get exportHere => 'Export here';

  @override
  String get mustBeNonNegativeInteger => 'Must be a non-negative integer';

  @override
  String get extractingLocalizedText => 'Extracting localized game text…';

  @override
  String get localizedTextExtractionCancelled =>
      'Localized text extraction cancelled.';

  @override
  String get localizedTextExtracted => 'Localized text extracted.';

  @override
  String get extractionFailed => 'Extraction failed.';

  @override
  String get localizationCacheFileGroupLabel => 'localization cache';

  @override
  String get extractLocalizedTextQuestion => 'Extract localized game text?';

  @override
  String get extractLocalizedTextBody =>
      'Localized game text isn\'t extracted yet. Extract it now from your game install? (optional)';

  @override
  String get notNow => 'Not now';

  @override
  String get extract => 'Extract';

  @override
  String get validationRequired => 'Required';

  @override
  String get validationMustBeWholeNumber => 'Must be a whole number';

  @override
  String get validationMustBeNumber => 'Must be a number';

  @override
  String get validationMustBeFinite => 'Must be a finite number';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Must be ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Must be ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Must be true or false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Must be one of: $options';
  }

  @override
  String get modNameRequired => 'Required';

  @override
  String get modNameControlCharacters => 'Must not contain control characters';

  @override
  String get modNamePathSeparators => 'Must not contain path separators';

  @override
  String get modNameNotAFolderName => 'Not a valid folder name';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extracted $idCount ids across $languageCount languages';
  }

  @override
  String get managerDeployActive =>
      'A mod-manager loadout is active. Undeploy it in gore-manager first.';
}
