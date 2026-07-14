import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_de.dart';
import 'app_localizations_en.dart';
import 'app_localizations_es.dart';
import 'app_localizations_fr.dart';
import 'app_localizations_it.dart';
import 'app_localizations_ja.dart';
import 'app_localizations_pl.dart';
import 'app_localizations_pt.dart';
import 'app_localizations_ru.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('de'),
    Locale('en'),
    Locale('es'),
    Locale('fr'),
    Locale('it'),
    Locale('ja'),
    Locale('pl'),
    Locale('pt'),
    Locale('pt', 'BR'),
    Locale('ru'),
    Locale('zh'),
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  ];

  /// No description provided for @tabItems.
  ///
  /// In en, this message translates to:
  /// **'Items'**
  String get tabItems;

  /// No description provided for @tabOverrides.
  ///
  /// In en, this message translates to:
  /// **'Changes'**
  String get tabOverrides;

  /// No description provided for @tabSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get tabSettings;

  /// No description provided for @tabDialogs.
  ///
  /// In en, this message translates to:
  /// **'Dialogs'**
  String get tabDialogs;

  /// No description provided for @tabAudio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get tabAudio;

  /// No description provided for @tabTextures.
  ///
  /// In en, this message translates to:
  /// **'Textures'**
  String get tabTextures;

  /// No description provided for @tabScripts.
  ///
  /// In en, this message translates to:
  /// **'Scripts'**
  String get tabScripts;

  /// No description provided for @changesAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get changesAll;

  /// No description provided for @sectionItemValues.
  ///
  /// In en, this message translates to:
  /// **'Item values'**
  String get sectionItemValues;

  /// No description provided for @sectionLocalizedText.
  ///
  /// In en, this message translates to:
  /// **'Localized text'**
  String get sectionLocalizedText;

  /// No description provided for @audioCatCreatures.
  ///
  /// In en, this message translates to:
  /// **'Creatures'**
  String get audioCatCreatures;

  /// No description provided for @audioCatObjects.
  ///
  /// In en, this message translates to:
  /// **'Objects'**
  String get audioCatObjects;

  /// No description provided for @audioCatMagic.
  ///
  /// In en, this message translates to:
  /// **'Magic'**
  String get audioCatMagic;

  /// No description provided for @audioCatMovement.
  ///
  /// In en, this message translates to:
  /// **'Movement'**
  String get audioCatMovement;

  /// No description provided for @audioCatWorld.
  ///
  /// In en, this message translates to:
  /// **'World'**
  String get audioCatWorld;

  /// No description provided for @audioCatAction.
  ///
  /// In en, this message translates to:
  /// **'Action'**
  String get audioCatAction;

  /// No description provided for @audioCatCombat.
  ///
  /// In en, this message translates to:
  /// **'Combat'**
  String get audioCatCombat;

  /// No description provided for @audioCatPhysics.
  ///
  /// In en, this message translates to:
  /// **'Physics'**
  String get audioCatPhysics;

  /// No description provided for @audioCatItems.
  ///
  /// In en, this message translates to:
  /// **'Items'**
  String get audioCatItems;

  /// No description provided for @audioCatUi.
  ///
  /// In en, this message translates to:
  /// **'UI'**
  String get audioCatUi;

  /// No description provided for @audioCatFoley.
  ///
  /// In en, this message translates to:
  /// **'Foley'**
  String get audioCatFoley;

  /// No description provided for @audioCatUnderwater.
  ///
  /// In en, this message translates to:
  /// **'Underwater'**
  String get audioCatUnderwater;

  /// No description provided for @audioCatVision.
  ///
  /// In en, this message translates to:
  /// **'Vision'**
  String get audioCatVision;

  /// No description provided for @audioCatDialog.
  ///
  /// In en, this message translates to:
  /// **'Dialog'**
  String get audioCatDialog;

  /// No description provided for @audioCatOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get audioCatOther;

  /// No description provided for @gameExecutable.
  ///
  /// In en, this message translates to:
  /// **'Game executable'**
  String get gameExecutable;

  /// No description provided for @gameExecutableSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Path to the game\'s .exe. Used to auto-detect localized text and the game install.'**
  String get gameExecutableSubtitle;

  /// No description provided for @gameExecutableNotSet.
  ///
  /// In en, this message translates to:
  /// **'Not set'**
  String get gameExecutableNotSet;

  /// No description provided for @chooseGameExecutable.
  ///
  /// In en, this message translates to:
  /// **'Choose…'**
  String get chooseGameExecutable;

  /// No description provided for @settingsDataSourceSection.
  ///
  /// In en, this message translates to:
  /// **'Game data'**
  String get settingsDataSourceSection;

  /// No description provided for @settingsLocalizationSection.
  ///
  /// In en, this message translates to:
  /// **'Localized text'**
  String get settingsLocalizationSection;

  /// No description provided for @extractLocalizedText.
  ///
  /// In en, this message translates to:
  /// **'Extract localized text'**
  String get extractLocalizedText;

  /// No description provided for @lightMode.
  ///
  /// In en, this message translates to:
  /// **'Light mode'**
  String get lightMode;

  /// No description provided for @darkMode.
  ///
  /// In en, this message translates to:
  /// **'Dark mode'**
  String get darkMode;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @exportMod.
  ///
  /// In en, this message translates to:
  /// **'Export mod'**
  String get exportMod;

  /// No description provided for @exportModWithCount.
  ///
  /// In en, this message translates to:
  /// **'Export mod ({count})'**
  String exportModWithCount(int count);

  /// No description provided for @selectAnItemToEdit.
  ///
  /// In en, this message translates to:
  /// **'Select an item to edit its fields.'**
  String get selectAnItemToEdit;

  /// No description provided for @gameDataActiveTooltip.
  ///
  /// In en, this message translates to:
  /// **'Game data: {name}'**
  String gameDataActiveTooltip(String name);

  /// No description provided for @gameDataBundledTooltip.
  ///
  /// In en, this message translates to:
  /// **'Game data: bundled'**
  String get gameDataBundledTooltip;

  /// No description provided for @loadGameDataDump.
  ///
  /// In en, this message translates to:
  /// **'Load game-data dump…'**
  String get loadGameDataDump;

  /// No description provided for @loadGameDataDumpSubtitle.
  ///
  /// In en, this message translates to:
  /// **'gore_game_data.json from the gore-dump mod'**
  String get loadGameDataDumpSubtitle;

  /// No description provided for @useBundledData.
  ///
  /// In en, this message translates to:
  /// **'Use bundled data'**
  String get useBundledData;

  /// No description provided for @alreadyBundled.
  ///
  /// In en, this message translates to:
  /// **'already bundled'**
  String get alreadyBundled;

  /// No description provided for @gameDataFileGroupLabel.
  ///
  /// In en, this message translates to:
  /// **'game data'**
  String get gameDataFileGroupLabel;

  /// No description provided for @minimize.
  ///
  /// In en, this message translates to:
  /// **'Minimize'**
  String get minimize;

  /// No description provided for @restore.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get restore;

  /// No description provided for @maximize.
  ///
  /// In en, this message translates to:
  /// **'Maximize'**
  String get maximize;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @aboutVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version} ({sha})'**
  String aboutVersion(String version, String sha);

  /// No description provided for @aboutCopyright.
  ///
  /// In en, this message translates to:
  /// **'© 2026 GORE contributors'**
  String get aboutCopyright;

  /// No description provided for @aboutLicense.
  ///
  /// In en, this message translates to:
  /// **'Licensed under the MIT License.'**
  String get aboutLicense;

  /// No description provided for @categoryMeleeWeapons.
  ///
  /// In en, this message translates to:
  /// **'Melee weapons'**
  String get categoryMeleeWeapons;

  /// No description provided for @categoryRangedWeapons.
  ///
  /// In en, this message translates to:
  /// **'Ranged weapons'**
  String get categoryRangedWeapons;

  /// No description provided for @categoryAmmunition.
  ///
  /// In en, this message translates to:
  /// **'Ammunition'**
  String get categoryAmmunition;

  /// No description provided for @categoryRunes.
  ///
  /// In en, this message translates to:
  /// **'Runes'**
  String get categoryRunes;

  /// No description provided for @categorySpellScrolls.
  ///
  /// In en, this message translates to:
  /// **'Spell scrolls'**
  String get categorySpellScrolls;

  /// No description provided for @categoryFoodAndPotions.
  ///
  /// In en, this message translates to:
  /// **'Food & potions'**
  String get categoryFoodAndPotions;

  /// No description provided for @categoryMiscellaneous.
  ///
  /// In en, this message translates to:
  /// **'Miscellaneous'**
  String get categoryMiscellaneous;

  /// No description provided for @categoryAmulets.
  ///
  /// In en, this message translates to:
  /// **'Amulets'**
  String get categoryAmulets;

  /// No description provided for @categoryRings.
  ///
  /// In en, this message translates to:
  /// **'Rings'**
  String get categoryRings;

  /// No description provided for @categoryAnimalTrophies.
  ///
  /// In en, this message translates to:
  /// **'Animal trophies'**
  String get categoryAnimalTrophies;

  /// No description provided for @categoryWritings.
  ///
  /// In en, this message translates to:
  /// **'Writings'**
  String get categoryWritings;

  /// No description provided for @categoryMissionItems.
  ///
  /// In en, this message translates to:
  /// **'Mission items'**
  String get categoryMissionItems;

  /// No description provided for @categoryKeys.
  ///
  /// In en, this message translates to:
  /// **'Keys'**
  String get categoryKeys;

  /// No description provided for @categoryOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get categoryOther;

  /// No description provided for @categoryWithCount.
  ///
  /// In en, this message translates to:
  /// **'{label} ({count})'**
  String categoryWithCount(String label, int count);

  /// No description provided for @searchItems.
  ///
  /// In en, this message translates to:
  /// **'Search items'**
  String get searchItems;

  /// No description provided for @noItemsMatch.
  ///
  /// In en, this message translates to:
  /// **'No items match'**
  String get noItemsMatch;

  /// No description provided for @failedToLoadCatalog.
  ///
  /// In en, this message translates to:
  /// **'Failed to load catalog: {error}'**
  String failedToLoadCatalog(String error);

  /// No description provided for @pendingOverridesWithCount.
  ///
  /// In en, this message translates to:
  /// **'Pending overrides ({count})'**
  String pendingOverridesWithCount(int count);

  /// No description provided for @clearAll.
  ///
  /// In en, this message translates to:
  /// **'Clear all'**
  String get clearAll;

  /// No description provided for @noPendingOverrides.
  ///
  /// In en, this message translates to:
  /// **'No pending overrides.\nEdit item fields to add some.'**
  String get noPendingOverrides;

  /// No description provided for @removeOverride.
  ///
  /// In en, this message translates to:
  /// **'Remove override'**
  String get removeOverride;

  /// No description provided for @searchChanges.
  ///
  /// In en, this message translates to:
  /// **'Search changes'**
  String get searchChanges;

  /// No description provided for @noChangesMatch.
  ///
  /// In en, this message translates to:
  /// **'No changes match'**
  String get noChangesMatch;

  /// No description provided for @clearSection.
  ///
  /// In en, this message translates to:
  /// **'Clear this group'**
  String get clearSection;

  /// No description provided for @modName.
  ///
  /// In en, this message translates to:
  /// **'Mod name'**
  String get modName;

  /// No description provided for @loadDelayLabel.
  ///
  /// In en, this message translates to:
  /// **'Load delay (ms, 0 = instant)'**
  String get loadDelayLabel;

  /// No description provided for @noFolderSelected.
  ///
  /// In en, this message translates to:
  /// **'No folder selected'**
  String get noFolderSelected;

  /// No description provided for @chooseFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose folder'**
  String get chooseFolder;

  /// No description provided for @packageAsZip.
  ///
  /// In en, this message translates to:
  /// **'Package as .zip'**
  String get packageAsZip;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @export.
  ///
  /// In en, this message translates to:
  /// **'Export'**
  String get export;

  /// No description provided for @exportHere.
  ///
  /// In en, this message translates to:
  /// **'Export here'**
  String get exportHere;

  /// No description provided for @mustBeNonNegativeInteger.
  ///
  /// In en, this message translates to:
  /// **'Must be a non-negative integer'**
  String get mustBeNonNegativeInteger;

  /// No description provided for @extractingLocalizedText.
  ///
  /// In en, this message translates to:
  /// **'Extracting localized game text…'**
  String get extractingLocalizedText;

  /// No description provided for @localizedTextExtractionCancelled.
  ///
  /// In en, this message translates to:
  /// **'Localized text extraction cancelled.'**
  String get localizedTextExtractionCancelled;

  /// No description provided for @localizedTextExtracted.
  ///
  /// In en, this message translates to:
  /// **'Localized text extracted.'**
  String get localizedTextExtracted;

  /// No description provided for @extractionFailed.
  ///
  /// In en, this message translates to:
  /// **'Extraction failed.'**
  String get extractionFailed;

  /// No description provided for @localizationCacheFileGroupLabel.
  ///
  /// In en, this message translates to:
  /// **'localization cache'**
  String get localizationCacheFileGroupLabel;

  /// No description provided for @extractLocalizedTextQuestion.
  ///
  /// In en, this message translates to:
  /// **'Extract localized game text?'**
  String get extractLocalizedTextQuestion;

  /// No description provided for @extractLocalizedTextBody.
  ///
  /// In en, this message translates to:
  /// **'Localized game text isn\'t extracted yet. Extract it now from your game install? (optional)'**
  String get extractLocalizedTextBody;

  /// No description provided for @notNow.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get notNow;

  /// No description provided for @extract.
  ///
  /// In en, this message translates to:
  /// **'Extract'**
  String get extract;

  /// No description provided for @validationRequired.
  ///
  /// In en, this message translates to:
  /// **'Required'**
  String get validationRequired;

  /// No description provided for @validationMustBeWholeNumber.
  ///
  /// In en, this message translates to:
  /// **'Must be a whole number'**
  String get validationMustBeWholeNumber;

  /// No description provided for @validationMustBeNumber.
  ///
  /// In en, this message translates to:
  /// **'Must be a number'**
  String get validationMustBeNumber;

  /// No description provided for @validationMustBeFinite.
  ///
  /// In en, this message translates to:
  /// **'Must be a finite number'**
  String get validationMustBeFinite;

  /// No description provided for @validationMustBeAtLeast.
  ///
  /// In en, this message translates to:
  /// **'Must be ≥ {min}'**
  String validationMustBeAtLeast(String min);

  /// No description provided for @validationMustBeAtMost.
  ///
  /// In en, this message translates to:
  /// **'Must be ≤ {max}'**
  String validationMustBeAtMost(String max);

  /// No description provided for @validationMustBeBool.
  ///
  /// In en, this message translates to:
  /// **'Must be true or false'**
  String get validationMustBeBool;

  /// No description provided for @validationMustBeOneOf.
  ///
  /// In en, this message translates to:
  /// **'Must be one of: {options}'**
  String validationMustBeOneOf(String options);

  /// No description provided for @modNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Required'**
  String get modNameRequired;

  /// No description provided for @modNameControlCharacters.
  ///
  /// In en, this message translates to:
  /// **'Must not contain control characters'**
  String get modNameControlCharacters;

  /// No description provided for @modNamePathSeparators.
  ///
  /// In en, this message translates to:
  /// **'Must not contain path separators'**
  String get modNamePathSeparators;

  /// No description provided for @modNameNotAFolderName.
  ///
  /// In en, this message translates to:
  /// **'Not a valid folder name'**
  String get modNameNotAFolderName;

  /// No description provided for @localizedTextExtractedCount.
  ///
  /// In en, this message translates to:
  /// **'Extracted {idCount} ids across {languageCount} languages'**
  String localizedTextExtractedCount(int idCount, int languageCount);

  /// No description provided for @managerDeployActive.
  ///
  /// In en, this message translates to:
  /// **'A mod-manager loadout is active. Undeploy it in gore-manager first.'**
  String get managerDeployActive;

  /// No description provided for @projectOpenLegacy.
  ///
  /// In en, this message translates to:
  /// **'Open legacy project…'**
  String get projectOpenLegacy;

  /// No description provided for @projectOpenManagedRevision3.
  ///
  /// In en, this message translates to:
  /// **'Open managed revision-3 project…'**
  String get projectOpenManagedRevision3;

  /// No description provided for @projectVerifyCurrentHead.
  ///
  /// In en, this message translates to:
  /// **'Verify current head'**
  String get projectVerifyCurrentHead;

  /// No description provided for @projectManagedRevision3Title.
  ///
  /// In en, this message translates to:
  /// **'Managed revision-3 project'**
  String get projectManagedRevision3Title;

  /// No description provided for @projectManagedRevision3IdentityOnly.
  ///
  /// In en, this message translates to:
  /// **'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.'**
  String get projectManagedRevision3IdentityOnly;

  /// No description provided for @projectRoot.
  ///
  /// In en, this message translates to:
  /// **'Root'**
  String get projectRoot;

  /// No description provided for @projectId.
  ///
  /// In en, this message translates to:
  /// **'Project ID'**
  String get projectId;

  /// No description provided for @projectRevision.
  ///
  /// In en, this message translates to:
  /// **'Project revision'**
  String get projectRevision;

  /// No description provided for @projectHeadSha256.
  ///
  /// In en, this message translates to:
  /// **'Head SHA-256'**
  String get projectHeadSha256;

  /// No description provided for @projectSnapshotBytes.
  ///
  /// In en, this message translates to:
  /// **'Snapshot bytes'**
  String get projectSnapshotBytes;

  /// No description provided for @projectNoCurrent.
  ///
  /// In en, this message translates to:
  /// **'No current project'**
  String get projectNoCurrent;

  /// No description provided for @projectManagedRevision3Opened.
  ///
  /// In en, this message translates to:
  /// **'Opened managed revision-3 project {projectId}'**
  String projectManagedRevision3Opened(String projectId);

  /// No description provided for @projectManagedRevision3OpenFailed.
  ///
  /// In en, this message translates to:
  /// **'Managed revision-3 project open failed: {error}'**
  String projectManagedRevision3OpenFailed(String error);

  /// No description provided for @projectManagedRevision3Verified.
  ///
  /// In en, this message translates to:
  /// **'Verified revision-3 head {headSha256}'**
  String projectManagedRevision3Verified(String headSha256);

  /// No description provided for @projectManagedRevision3VerifyFailed.
  ///
  /// In en, this message translates to:
  /// **'Revision-3 head verification failed: {error}'**
  String projectManagedRevision3VerifyFailed(String error);

  /// No description provided for @projectManagedRevision3RequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.'**
  String get projectManagedRevision3RequiresReopen;

  /// No description provided for @projectManagedRevision3VerifyBlocked.
  ///
  /// In en, this message translates to:
  /// **'Verification is blocked until the managed project is reopened.'**
  String get projectManagedRevision3VerifyBlocked;

  /// No description provided for @projectTransitionCleanupWarning.
  ///
  /// In en, this message translates to:
  /// **'The new project is open, but the previous project session could not be cleaned up completely. No cleanup retry will be attempted. Restart Mod Studio before reopening the retired project.'**
  String get projectTransitionCleanupWarning;

  /// No description provided for @projectNewManagedRevision3.
  ///
  /// In en, this message translates to:
  /// **'New managed mod project…'**
  String get projectNewManagedRevision3;

  /// No description provided for @projectNewLegacy.
  ///
  /// In en, this message translates to:
  /// **'New legacy project'**
  String get projectNewLegacy;

  /// No description provided for @projectCreateGamePathRequired.
  ///
  /// In en, this message translates to:
  /// **'Set the Gothic 1 Remake game path in Settings before creating a mod project.'**
  String get projectCreateGamePathRequired;

  /// No description provided for @projectCreateDirectoryPickerTitle.
  ///
  /// In en, this message translates to:
  /// **'Create managed mod project here'**
  String get projectCreateDirectoryPickerTitle;

  /// No description provided for @projectManagedRevision3Created.
  ///
  /// In en, this message translates to:
  /// **'Created managed mod project {projectId}'**
  String projectManagedRevision3Created(String projectId);

  /// No description provided for @projectManagedRevision3CreateFailed.
  ///
  /// In en, this message translates to:
  /// **'Managed mod project creation failed: {error}'**
  String projectManagedRevision3CreateFailed(String error);

  /// No description provided for @projectCreateDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Create a mod project'**
  String get projectCreateDialogTitle;

  /// No description provided for @projectCreateNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Project name'**
  String get projectCreateNameLabel;

  /// No description provided for @projectCreateNameHelper.
  ///
  /// In en, this message translates to:
  /// **'The name shown in Mod Studio.'**
  String get projectCreateNameHelper;

  /// No description provided for @projectCreateVersionLabel.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get projectCreateVersionLabel;

  /// No description provided for @projectCreateVersionHelper.
  ///
  /// In en, this message translates to:
  /// **'A starting version, such as 0.1.0.'**
  String get projectCreateVersionHelper;

  /// No description provided for @projectCreateAuthorLabel.
  ///
  /// In en, this message translates to:
  /// **'Author'**
  String get projectCreateAuthorLabel;

  /// No description provided for @projectCreateAuthorHelper.
  ///
  /// In en, this message translates to:
  /// **'Your name or mod-team name.'**
  String get projectCreateAuthorHelper;

  /// No description provided for @projectCreateLocalesLabel.
  ///
  /// In en, this message translates to:
  /// **'Authoring languages'**
  String get projectCreateLocalesLabel;

  /// No description provided for @projectCreateLocalesHelper.
  ///
  /// In en, this message translates to:
  /// **'Comma-separated canonical tags, for example: en, de, en-US.'**
  String get projectCreateLocalesHelper;

  /// No description provided for @projectCreateBoundary.
  ///
  /// In en, this message translates to:
  /// **'This creates an empty managed offline project. It does not build, deploy, or run a mod, and it does not change game files or save files.'**
  String get projectCreateBoundary;

  /// No description provided for @projectCreateSubmit.
  ///
  /// In en, this message translates to:
  /// **'Create project'**
  String get projectCreateSubmit;

  /// No description provided for @projectCreateMetadataRequired.
  ///
  /// In en, this message translates to:
  /// **'{label} is required.'**
  String projectCreateMetadataRequired(String label);

  /// No description provided for @projectCreateMetadataNoOuterWhitespace.
  ///
  /// In en, this message translates to:
  /// **'{label} cannot start or end with whitespace.'**
  String projectCreateMetadataNoOuterWhitespace(String label);

  /// No description provided for @projectCreateMetadataControlCharacters.
  ///
  /// In en, this message translates to:
  /// **'{label} cannot contain control characters.'**
  String projectCreateMetadataControlCharacters(String label);

  /// No description provided for @projectCreateMetadataMalformed.
  ///
  /// In en, this message translates to:
  /// **'{label} contains malformed text.'**
  String projectCreateMetadataMalformed(String label);

  /// No description provided for @projectCreateMetadataTooLong.
  ///
  /// In en, this message translates to:
  /// **'{label} exceeds its {maxBytes}-byte UTF-8 limit.'**
  String projectCreateMetadataTooLong(String label, int maxBytes);

  /// No description provided for @projectCreateLocalesRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter at least one authoring language.'**
  String get projectCreateLocalesRequired;

  /// No description provided for @projectCreateLocalesEmptyEntry.
  ///
  /// In en, this message translates to:
  /// **'Remove the empty authoring-language entry.'**
  String get projectCreateLocalesEmptyEntry;

  /// No description provided for @projectCreateLocalesTooMany.
  ///
  /// In en, this message translates to:
  /// **'Use at most {maxLocales} authoring languages.'**
  String projectCreateLocalesTooMany(int maxLocales);

  /// No description provided for @projectCreateLocaleBoundedAscii.
  ///
  /// In en, this message translates to:
  /// **'Locale \"{locale}\" must be bounded ASCII.'**
  String projectCreateLocaleBoundedAscii(String locale);

  /// No description provided for @projectCreateLocaleLanguage.
  ///
  /// In en, this message translates to:
  /// **'Locale \"{locale}\" needs a 2-8 letter lowercase language.'**
  String projectCreateLocaleLanguage(String locale);

  /// No description provided for @projectCreateLocaleInvalidSegment.
  ///
  /// In en, this message translates to:
  /// **'Locale \"{locale}\" has an invalid segment.'**
  String projectCreateLocaleInvalidSegment(String locale);

  /// No description provided for @projectCreateLocaleNotCanonical.
  ///
  /// In en, this message translates to:
  /// **'Locale \"{locale}\" is not canonical; use \"{canonical}\".'**
  String projectCreateLocaleNotCanonical(String locale, String canonical);

  /// No description provided for @managedWorkspaceOverviewLabel.
  ///
  /// In en, this message translates to:
  /// **'Overview'**
  String get managedWorkspaceOverviewLabel;

  /// No description provided for @managedWorkspaceContentLabel.
  ///
  /// In en, this message translates to:
  /// **'Content'**
  String get managedWorkspaceContentLabel;

  /// No description provided for @managedWorkspaceDataAssetsLabel.
  ///
  /// In en, this message translates to:
  /// **'DataAssets'**
  String get managedWorkspaceDataAssetsLabel;

  /// No description provided for @managedContentWorkspaceLibraryLabel.
  ///
  /// In en, this message translates to:
  /// **'This mod'**
  String get managedContentWorkspaceLibraryLabel;

  /// No description provided for @managedWorkspaceHomeLabel.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get managedWorkspaceHomeLabel;

  /// No description provided for @managedWorkspaceStoryLabel.
  ///
  /// In en, this message translates to:
  /// **'Story'**
  String get managedWorkspaceStoryLabel;

  /// No description provided for @managedWorkspaceWorldLabel.
  ///
  /// In en, this message translates to:
  /// **'World'**
  String get managedWorkspaceWorldLabel;

  /// No description provided for @managedWorkspaceLocalizationVoiceLabel.
  ///
  /// In en, this message translates to:
  /// **'Localization & Voice'**
  String get managedWorkspaceLocalizationVoiceLabel;

  /// No description provided for @managedWorkspaceValidateTestLabel.
  ///
  /// In en, this message translates to:
  /// **'Validate & Test'**
  String get managedWorkspaceValidateTestLabel;

  /// No description provided for @managedWorkspaceBuildReleaseLabel.
  ///
  /// In en, this message translates to:
  /// **'Build & Release'**
  String get managedWorkspaceBuildReleaseLabel;

  /// No description provided for @managedWorkspaceSettingsExpertLabel.
  ///
  /// In en, this message translates to:
  /// **'Settings & Expert'**
  String get managedWorkspaceSettingsExpertLabel;

  /// No description provided for @managedSectionStoryDescription.
  ///
  /// In en, this message translates to:
  /// **'NPCs, quests, and dialogue.'**
  String get managedSectionStoryDescription;

  /// No description provided for @managedSectionWorldDescription.
  ///
  /// In en, this message translates to:
  /// **'World placement and workflows are planned.'**
  String get managedSectionWorldDescription;

  /// No description provided for @managedSectionLocalizationVoiceDescription.
  ///
  /// In en, this message translates to:
  /// **'Voice production tools are available; managed localization editing is planned.'**
  String get managedSectionLocalizationVoiceDescription;

  /// No description provided for @managedSectionValidateTestDescription.
  ///
  /// In en, this message translates to:
  /// **'Verify exact project integrity and checkpoints; no runtime test is claimed.'**
  String get managedSectionValidateTestDescription;

  /// No description provided for @managedSectionBuildReleaseDescription.
  ///
  /// In en, this message translates to:
  /// **'Voice bundles are available; full playable builds and deployment are unavailable.'**
  String get managedSectionBuildReleaseDescription;

  /// No description provided for @managedSectionSettingsExpertDescription.
  ///
  /// In en, this message translates to:
  /// **'Settings are available; expert tools are not yet integrated.'**
  String get managedSectionSettingsExpertDescription;

  /// No description provided for @managedSectionStatusHeading.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get managedSectionStatusHeading;

  /// No description provided for @managedSectionActionsHeading.
  ///
  /// In en, this message translates to:
  /// **'Actions'**
  String get managedSectionActionsHeading;

  /// No description provided for @managedCapabilityAvailable.
  ///
  /// In en, this message translates to:
  /// **'Available'**
  String get managedCapabilityAvailable;

  /// No description provided for @managedCapabilityPartial.
  ///
  /// In en, this message translates to:
  /// **'Partial'**
  String get managedCapabilityPartial;

  /// No description provided for @managedCapabilityPlanned.
  ///
  /// In en, this message translates to:
  /// **'Planned'**
  String get managedCapabilityPlanned;

  /// No description provided for @managedCapabilityUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Unavailable'**
  String get managedCapabilityUnavailable;

  /// No description provided for @managedProjectSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Exact-current offline authoring workspace'**
  String get managedProjectSubtitle;

  /// No description provided for @managedProjectLandingTitle.
  ///
  /// In en, this message translates to:
  /// **'Managed project workspace'**
  String get managedProjectLandingTitle;

  /// No description provided for @managedProjectLandingDescription.
  ///
  /// In en, this message translates to:
  /// **'Use the new Home, Content, Story, Voice, validation, and release workflow in one managed project.'**
  String get managedProjectLandingDescription;

  /// No description provided for @legacyCompatibilityToolsTitle.
  ///
  /// In en, this message translates to:
  /// **'Legacy compatibility tools'**
  String get legacyCompatibilityToolsTitle;

  /// No description provided for @legacyCompatibilityToolsDescription.
  ///
  /// In en, this message translates to:
  /// **'The tabs below are older direct-replacement tools. They remain available while the managed project workspace grows.'**
  String get legacyCompatibilityToolsDescription;

  /// No description provided for @managedProjectTechnicalDetails.
  ///
  /// In en, this message translates to:
  /// **'Technical project details'**
  String get managedProjectTechnicalDetails;

  /// No description provided for @managedProjectRecoveryContentLocked.
  ///
  /// In en, this message translates to:
  /// **'Reopen the managed project before reading its content.'**
  String get managedProjectRecoveryContentLocked;

  /// No description provided for @managedDashboardUntitledProject.
  ///
  /// In en, this message translates to:
  /// **'Untitled project'**
  String get managedDashboardUntitledProject;

  /// No description provided for @managedDashboardDraftStatus.
  ///
  /// In en, this message translates to:
  /// **'Draft'**
  String get managedDashboardDraftStatus;

  /// No description provided for @managedDashboardProjectVersion.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get managedDashboardProjectVersion;

  /// No description provided for @managedDashboardProjectAuthor.
  ///
  /// In en, this message translates to:
  /// **'Author'**
  String get managedDashboardProjectAuthor;

  /// No description provided for @managedDashboardNotProvided.
  ///
  /// In en, this message translates to:
  /// **'Not provided'**
  String get managedDashboardNotProvided;

  /// No description provided for @managedDashboardContentCounts.
  ///
  /// In en, this message translates to:
  /// **'Project content'**
  String get managedDashboardContentCounts;

  /// No description provided for @managedDashboardNpcDrafts.
  ///
  /// In en, this message translates to:
  /// **'NPC drafts'**
  String get managedDashboardNpcDrafts;

  /// No description provided for @managedDashboardQuestDrafts.
  ///
  /// In en, this message translates to:
  /// **'Quest drafts'**
  String get managedDashboardQuestDrafts;

  /// No description provided for @managedDashboardDialogLines.
  ///
  /// In en, this message translates to:
  /// **'Dialog lines'**
  String get managedDashboardDialogLines;

  /// No description provided for @managedDashboardVoiceTakes.
  ///
  /// In en, this message translates to:
  /// **'Voice takes'**
  String get managedDashboardVoiceTakes;

  /// No description provided for @managedDashboardAssets.
  ///
  /// In en, this message translates to:
  /// **'Assets'**
  String get managedDashboardAssets;

  /// No description provided for @managedDashboardUnresolvedReferences.
  ///
  /// In en, this message translates to:
  /// **'Unresolved references'**
  String get managedDashboardUnresolvedReferences;

  /// No description provided for @managedDashboardReadiness.
  ///
  /// In en, this message translates to:
  /// **'What works now'**
  String get managedDashboardReadiness;

  /// No description provided for @managedDashboardOfflineAuthoringTitle.
  ///
  /// In en, this message translates to:
  /// **'Offline authoring available'**
  String get managedDashboardOfflineAuthoringTitle;

  /// No description provided for @managedDashboardOfflineAuthoringDescription.
  ///
  /// In en, this message translates to:
  /// **'Create and edit supported project content without changing the game installation or save files.'**
  String get managedDashboardOfflineAuthoringDescription;

  /// No description provided for @managedDashboardGeneralBuildBlockedTitle.
  ///
  /// In en, this message translates to:
  /// **'General mod build unavailable'**
  String get managedDashboardGeneralBuildBlockedTitle;

  /// No description provided for @managedDashboardGeneralBuildBlockedDescription.
  ///
  /// In en, this message translates to:
  /// **'Only sealed offline Voice bundles can be built; a complete playable mod cannot be built yet.'**
  String get managedDashboardGeneralBuildBlockedDescription;

  /// No description provided for @managedDashboardRuntimeUnqualifiedTitle.
  ///
  /// In en, this message translates to:
  /// **'Runtime not yet verified'**
  String get managedDashboardRuntimeUnqualifiedTitle;

  /// No description provided for @managedDashboardRuntimeUnqualifiedDescription.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio has not proven this project content inside the running game.'**
  String get managedDashboardRuntimeUnqualifiedDescription;

  /// No description provided for @managedDashboardReferenceIntegrityTitle.
  ///
  /// In en, this message translates to:
  /// **'Reference integrity'**
  String get managedDashboardReferenceIntegrityTitle;

  /// No description provided for @managedDashboardReferenceIntegrityDescription.
  ///
  /// In en, this message translates to:
  /// **'This count checks project references only; it is not build or runtime readiness.'**
  String get managedDashboardReferenceIntegrityDescription;

  /// No description provided for @managedDashboardMissingGameTitle.
  ///
  /// In en, this message translates to:
  /// **'Game setup required'**
  String get managedDashboardMissingGameTitle;

  /// No description provided for @managedDashboardMissingGameDescription.
  ///
  /// In en, this message translates to:
  /// **'Configure the Gothic 1 Remake installation in Settings before using actions that need installed-game evidence.'**
  String get managedDashboardMissingGameDescription;

  /// No description provided for @managedDashboardCreateHeading.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get managedDashboardCreateHeading;

  /// No description provided for @managedDashboardToolsHeading.
  ///
  /// In en, this message translates to:
  /// **'Project tools'**
  String get managedDashboardToolsHeading;

  /// No description provided for @managedDashboardLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading project overview'**
  String get managedDashboardLoading;

  /// No description provided for @managedDashboardLoadError.
  ///
  /// In en, this message translates to:
  /// **'Project overview unavailable'**
  String get managedDashboardLoadError;

  /// No description provided for @managedDashboardLoadErrorDescription.
  ///
  /// In en, this message translates to:
  /// **'The verified project overview could not be loaded. Project content was not changed.'**
  String get managedDashboardLoadErrorDescription;

  /// No description provided for @managedDashboardRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get managedDashboardRetry;

  /// No description provided for @managedActionNewNpcTitle.
  ///
  /// In en, this message translates to:
  /// **'New NPC'**
  String get managedActionNewNpcTitle;

  /// No description provided for @managedActionNewNpcDescription.
  ///
  /// In en, this message translates to:
  /// **'Create a bounded offline NPC draft from verified installed-game evidence.'**
  String get managedActionNewNpcDescription;

  /// No description provided for @managedActionNewQuestTitle.
  ///
  /// In en, this message translates to:
  /// **'New Quest'**
  String get managedActionNewQuestTitle;

  /// No description provided for @managedActionNewQuestDescription.
  ///
  /// In en, this message translates to:
  /// **'Create an offline Quest draft with objectives and verified parent identities.'**
  String get managedActionNewQuestDescription;

  /// No description provided for @managedActionAddVoiceTakeTitle.
  ///
  /// In en, this message translates to:
  /// **'Add Voice take'**
  String get managedActionAddVoiceTakeTitle;

  /// No description provided for @managedActionAddVoiceTakeDescription.
  ///
  /// In en, this message translates to:
  /// **'Import an Ogg Vorbis recording into this project without deploying it.'**
  String get managedActionAddVoiceTakeDescription;

  /// No description provided for @managedActionManageVoiceTakesTitle.
  ///
  /// In en, this message translates to:
  /// **'Manage Voice takes'**
  String get managedActionManageVoiceTakesTitle;

  /// No description provided for @managedActionManageVoiceTakesDescription.
  ///
  /// In en, this message translates to:
  /// **'Review takes and select approved recordings for Voice slots.'**
  String get managedActionManageVoiceTakesDescription;

  /// No description provided for @managedActionResolveVoiceTargetTitle.
  ///
  /// In en, this message translates to:
  /// **'Resolve Voice target'**
  String get managedActionResolveVoiceTargetTitle;

  /// No description provided for @managedActionResolveVoiceTargetDescription.
  ///
  /// In en, this message translates to:
  /// **'Match project Voice slots to exact installed archive members without changing the game.'**
  String get managedActionResolveVoiceTargetDescription;

  /// No description provided for @managedActionBuildVoiceBundleTitle.
  ///
  /// In en, this message translates to:
  /// **'Build Voice bundle'**
  String get managedActionBuildVoiceBundleTitle;

  /// No description provided for @managedActionBuildVoiceBundleDescription.
  ///
  /// In en, this message translates to:
  /// **'Build a sealed offline existing-member bundle; deployment is not performed.'**
  String get managedActionBuildVoiceBundleDescription;

  /// No description provided for @managedActionDataAssetsTitle.
  ///
  /// In en, this message translates to:
  /// **'DataAsset edits'**
  String get managedActionDataAssetsTitle;

  /// No description provided for @managedActionDataAssetsDescription.
  ///
  /// In en, this message translates to:
  /// **'Inspect installed packages and stage verified fixed-width value edits in the project.'**
  String get managedActionDataAssetsDescription;

  /// No description provided for @managedActionBrowseProjectContentDescription.
  ///
  /// In en, this message translates to:
  /// **'Browse exact project content and its resolved or unresolved references.'**
  String get managedActionBrowseProjectContentDescription;

  /// No description provided for @managedActionSettingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get managedActionSettingsTitle;

  /// No description provided for @managedActionSettingsDescription.
  ///
  /// In en, this message translates to:
  /// **'Configure the Gothic 1 Remake installation and Mod Studio preferences.'**
  String get managedActionSettingsDescription;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>[
    'de',
    'en',
    'es',
    'fr',
    'it',
    'ja',
    'pl',
    'pt',
    'ru',
    'zh',
  ].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when language+script codes are specified.
  switch (locale.languageCode) {
    case 'zh':
      {
        switch (locale.scriptCode) {
          case 'Hans':
            return AppLocalizationsZhHans();
        }
        break;
      }
  }

  // Lookup logic when language+country codes are specified.
  switch (locale.languageCode) {
    case 'pt':
      {
        switch (locale.countryCode) {
          case 'BR':
            return AppLocalizationsPtBr();
        }
        break;
      }
  }

  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'de':
      return AppLocalizationsDe();
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
    case 'fr':
      return AppLocalizationsFr();
    case 'it':
      return AppLocalizationsIt();
    case 'ja':
      return AppLocalizationsJa();
    case 'pl':
      return AppLocalizationsPl();
    case 'pt':
      return AppLocalizationsPt();
    case 'ru':
      return AppLocalizationsRu();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
