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

  /// No description provided for @projectClose.
  ///
  /// In en, this message translates to:
  /// **'Close project'**
  String get projectClose;

  /// No description provided for @projectCloseFailed.
  ///
  /// In en, this message translates to:
  /// **'Project could not be closed: {error}'**
  String projectCloseFailed(String error);

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
  /// **'Write and translate project dialog in one place, then continue with voice work.'**
  String get managedSectionLocalizationVoiceDescription;

  /// No description provided for @managedLocalizationProjectTextsLabel.
  ///
  /// In en, this message translates to:
  /// **'Project texts'**
  String get managedLocalizationProjectTextsLabel;

  /// No description provided for @managedLocalizationSearchLabel.
  ///
  /// In en, this message translates to:
  /// **'Search project texts'**
  String get managedLocalizationSearchLabel;

  /// No description provided for @managedLocalizationRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get managedLocalizationRefresh;

  /// No description provided for @managedLocalizationEmptyTitle.
  ///
  /// In en, this message translates to:
  /// **'No project text yet'**
  String get managedLocalizationEmptyTitle;

  /// No description provided for @managedLocalizationEmptyDescription.
  ///
  /// In en, this message translates to:
  /// **'Create a dialog line to start writing and translating text.'**
  String get managedLocalizationEmptyDescription;

  /// No description provided for @managedLocalizationLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Project texts could not be opened'**
  String get managedLocalizationLoadFailed;

  /// No description provided for @managedLocalizationSelectText.
  ///
  /// In en, this message translates to:
  /// **'Select a project text to edit'**
  String get managedLocalizationSelectText;

  /// No description provided for @managedLocalizationLanguagesLabel.
  ///
  /// In en, this message translates to:
  /// **'Languages'**
  String get managedLocalizationLanguagesLabel;

  /// No description provided for @managedLocalizationUsedByLines.
  ///
  /// In en, this message translates to:
  /// **'Used by dialog lines'**
  String get managedLocalizationUsedByLines;

  /// No description provided for @managedLocalizationNoLine.
  ///
  /// In en, this message translates to:
  /// **'Not used by a dialog line yet'**
  String get managedLocalizationNoLine;

  /// No description provided for @managedLocalizationSpeakerLabel.
  ///
  /// In en, this message translates to:
  /// **'Speaker label'**
  String get managedLocalizationSpeakerLabel;

  /// No description provided for @managedLocalizationAddLanguage.
  ///
  /// In en, this message translates to:
  /// **'Add language'**
  String get managedLocalizationAddLanguage;

  /// No description provided for @managedLocalizationRemoveLanguage.
  ///
  /// In en, this message translates to:
  /// **'Remove language'**
  String get managedLocalizationRemoveLanguage;

  /// No description provided for @managedLocalizationLanguageHint.
  ///
  /// In en, this message translates to:
  /// **'For example de, en, or pt-BR'**
  String get managedLocalizationLanguageHint;

  /// No description provided for @managedLocalizationLanguageExists.
  ///
  /// In en, this message translates to:
  /// **'This language is already present.'**
  String get managedLocalizationLanguageExists;

  /// No description provided for @managedLocalizationAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get managedLocalizationAdd;

  /// No description provided for @managedLocalizationSaved.
  ///
  /// In en, this message translates to:
  /// **'Project text saved'**
  String get managedLocalizationSaved;

  /// No description provided for @managedLocalizationVoiceLocked.
  ///
  /// In en, this message translates to:
  /// **'This text has recorded voice takes, so its transcript is locked in this editor.'**
  String get managedLocalizationVoiceLocked;

  /// No description provided for @managedLocalizationVoiceSlotRemovalLocked.
  ///
  /// In en, this message translates to:
  /// **'This language is connected to a Voice slot and cannot be removed here.'**
  String get managedLocalizationVoiceSlotRemovalLocked;

  /// No description provided for @managedLocalizationMinimumLanguageLocked.
  ///
  /// In en, this message translates to:
  /// **'Keep at least one language for this project text.'**
  String get managedLocalizationMinimumLanguageLocked;

  /// No description provided for @managedLocalizationSharedNotice.
  ///
  /// In en, this message translates to:
  /// **'This project text is shared. Saving changes updates every listed dialog line.'**
  String get managedLocalizationSharedNotice;

  /// No description provided for @managedLocalizationOfflineNotice.
  ///
  /// In en, this message translates to:
  /// **'Changes are saved only to this managed project. Build and in-game behavior remain separate.'**
  String get managedLocalizationOfflineNotice;

  /// No description provided for @managedLocalizationUnsavedTitle.
  ///
  /// In en, this message translates to:
  /// **'Discard unsaved changes?'**
  String get managedLocalizationUnsavedTitle;

  /// No description provided for @managedLocalizationUnsavedDescription.
  ///
  /// In en, this message translates to:
  /// **'You changed this project text. Switching now would discard those edits.'**
  String get managedLocalizationUnsavedDescription;

  /// No description provided for @managedLocalizationDiscard.
  ///
  /// In en, this message translates to:
  /// **'Discard changes'**
  String get managedLocalizationDiscard;

  /// No description provided for @managedLocalizationKeepEditing.
  ///
  /// In en, this message translates to:
  /// **'Keep editing'**
  String get managedLocalizationKeepEditing;

  /// No description provided for @managedLocalizationStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed while this text was open. Refresh and try again.'**
  String get managedLocalizationStale;

  /// No description provided for @managedLocalizationReopen.
  ///
  /// In en, this message translates to:
  /// **'The project must be reopened before text editing can continue.'**
  String get managedLocalizationReopen;

  /// No description provided for @managedLocalizationInvalid.
  ///
  /// In en, this message translates to:
  /// **'Check that every language and dialog text is valid and not empty.'**
  String get managedLocalizationInvalid;

  /// No description provided for @managedLocalizationSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'The project text could not be saved.'**
  String get managedLocalizationSaveFailed;

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

  /// No description provided for @managedActionNewDialogLineTitle.
  ///
  /// In en, this message translates to:
  /// **'Add dialog line'**
  String get managedActionNewDialogLineTitle;

  /// No description provided for @managedActionNewDialogLineDescription.
  ///
  /// In en, this message translates to:
  /// **'Write localized project text or connect an unused text already in this project. This does not create a playable dialog topic.'**
  String get managedActionNewDialogLineDescription;

  /// No description provided for @managedActionNewDialogLineSaved.
  ///
  /// In en, this message translates to:
  /// **'Dialog line saved in project revision {projectRevision}. The game and save files were not changed.'**
  String managedActionNewDialogLineSaved(int projectRevision);

  /// No description provided for @managedDialogLineIntroduction.
  ///
  /// In en, this message translates to:
  /// **'Write a new localized dialog line or connect text that already belongs to this project.'**
  String get managedDialogLineIntroduction;

  /// No description provided for @managedDialogLineBoundary.
  ///
  /// In en, this message translates to:
  /// **'Only project files change. This does not create an AngelScript topic or a playable dialog, and it never changes the game installation or save files. The speaker field is only a label; it does not link an NPC.'**
  String get managedDialogLineBoundary;

  /// No description provided for @managedDialogLineCreateMode.
  ///
  /// In en, this message translates to:
  /// **'Write new text'**
  String get managedDialogLineCreateMode;

  /// No description provided for @managedDialogLineReuseMode.
  ///
  /// In en, this message translates to:
  /// **'Use project text'**
  String get managedDialogLineReuseMode;

  /// No description provided for @managedDialogLineNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Line name'**
  String get managedDialogLineNameLabel;

  /// No description provided for @managedDialogLineNameHint.
  ///
  /// In en, this message translates to:
  /// **'Mine entrance greeting'**
  String get managedDialogLineNameHint;

  /// No description provided for @managedDialogLineSpeakerLabel.
  ///
  /// In en, this message translates to:
  /// **'Speaker label (optional)'**
  String get managedDialogLineSpeakerLabel;

  /// No description provided for @managedDialogLineSpeakerHint.
  ///
  /// In en, this message translates to:
  /// **'For example, Viper'**
  String get managedDialogLineSpeakerHint;

  /// No description provided for @managedDialogLineLocaleLabel.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get managedDialogLineLocaleLabel;

  /// No description provided for @managedDialogLineTextLabel.
  ///
  /// In en, this message translates to:
  /// **'Dialog text'**
  String get managedDialogLineTextLabel;

  /// No description provided for @managedDialogLineReuseSearch.
  ///
  /// In en, this message translates to:
  /// **'Search unused project text'**
  String get managedDialogLineReuseSearch;

  /// No description provided for @managedDialogLineNoReusableText.
  ///
  /// In en, this message translates to:
  /// **'There is no unused, structurally intact project text to connect. Write new text instead.'**
  String get managedDialogLineNoReusableText;

  /// No description provided for @managedDialogLineCreateSlotLabel.
  ///
  /// In en, this message translates to:
  /// **'Prepare this language for Voice'**
  String get managedDialogLineCreateSlotLabel;

  /// No description provided for @managedDialogLineCreateSlotHelp.
  ///
  /// In en, this message translates to:
  /// **'Creates an empty unresolved Voice slot in the project. It does not add or deploy a recording.'**
  String get managedDialogLineCreateSlotHelp;

  /// No description provided for @managedDialogLineCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get managedDialogLineCancel;

  /// No description provided for @managedDialogLineSave.
  ///
  /// In en, this message translates to:
  /// **'Save to project'**
  String get managedDialogLineSave;

  /// No description provided for @managedDialogLineSaving.
  ///
  /// In en, this message translates to:
  /// **'Saving…'**
  String get managedDialogLineSaving;

  /// No description provided for @managedDialogLineLoading.
  ///
  /// In en, this message translates to:
  /// **'Reading exact project content…'**
  String get managedDialogLineLoading;

  /// No description provided for @managedDialogLineLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'The exact current project content could not be read. Nothing was changed.'**
  String get managedDialogLineLoadFailed;

  /// No description provided for @managedDialogLineRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get managedDialogLineRetry;

  /// No description provided for @managedDialogLineStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed while this window was open. Close it and try again from the current project.'**
  String get managedDialogLineStale;

  /// No description provided for @managedDialogLineRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'The current project can no longer be verified safely. Close this window and reopen the managed project.'**
  String get managedDialogLineRequiresReopen;

  /// No description provided for @managedDialogLineInvalidInput.
  ///
  /// In en, this message translates to:
  /// **'Check the highlighted project input and choose an exact current option.'**
  String get managedDialogLineInvalidInput;

  /// No description provided for @managedDialogLineSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'The dialog line could not be saved safely. No game or save files were changed.'**
  String get managedDialogLineSaveFailed;

  /// No description provided for @managedDialogLineDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get managedDialogLineDone;

  /// No description provided for @managedDialogLineAddRecording.
  ///
  /// In en, this message translates to:
  /// **'Add recording'**
  String get managedDialogLineAddRecording;

  /// No description provided for @managedActionAddVoiceTakeTitle.
  ///
  /// In en, this message translates to:
  /// **'Add Voice take'**
  String get managedActionAddVoiceTakeTitle;

  /// No description provided for @managedActionAddVoiceTakeDescription.
  ///
  /// In en, this message translates to:
  /// **'Import an Ogg Vorbis recording for an existing project dialog line without deploying it.'**
  String get managedActionAddVoiceTakeDescription;

  /// No description provided for @managedActionAddVoiceTakeRequiresDialogLine.
  ///
  /// In en, this message translates to:
  /// **'Create or repair a dialog line with one valid localization entry before using Voice tools.'**
  String get managedActionAddVoiceTakeRequiresDialogLine;

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

  /// No description provided for @projectStarterSetupOpenFailed.
  ///
  /// In en, this message translates to:
  /// **'Project {projectId} was created safely, but the starter setup did not open. The valid empty project remains current.'**
  String projectStarterSetupOpenFailed(String projectId);

  /// No description provided for @projectStarterOutcomeUnverified.
  ///
  /// In en, this message translates to:
  /// **'Project {projectId} was created, but Mod Studio cannot verify the starter outcome. Reopen the managed project before continuing; the game and save files were not changed.'**
  String projectStarterOutcomeUnverified(String projectId);

  /// No description provided for @projectStarterNpcCancelled.
  ///
  /// In en, this message translates to:
  /// **'Project {projectId} was created. The NPC starter was not added, so the valid empty project remains current.'**
  String projectStarterNpcCancelled(String projectId);

  /// No description provided for @projectStarterNpcSaved.
  ///
  /// In en, this message translates to:
  /// **'NPC starter saved in project revision {projectRevision}. It remains build-blocked, runtime-unqualified, and is not spawned.'**
  String projectStarterNpcSaved(int projectRevision);

  /// No description provided for @projectStarterQuestCancelled.
  ///
  /// In en, this message translates to:
  /// **'Project {projectId} was created. The Quest starter was not added, so the valid empty project remains current.'**
  String projectStarterQuestCancelled(String projectId);

  /// No description provided for @projectStarterQuestSaved.
  ///
  /// In en, this message translates to:
  /// **'Quest starter saved in project revision {projectRevision}. It remains build-blocked and runtime-unqualified.'**
  String projectStarterQuestSaved(int projectRevision);

  /// No description provided for @projectStarterSemanticsLabel.
  ///
  /// In en, this message translates to:
  /// **'Project starter'**
  String get projectStarterSemanticsLabel;

  /// No description provided for @projectStarterPrompt.
  ///
  /// In en, this message translates to:
  /// **'How would you like to start?'**
  String get projectStarterPrompt;

  /// No description provided for @projectStarterWriteBoundary.
  ///
  /// In en, this message translates to:
  /// **'Choosing a starter performs no writes. The project is created only after you submit this form and choose an empty folder.'**
  String get projectStarterWriteBoundary;

  /// No description provided for @projectStarterEmptyTitle.
  ///
  /// In en, this message translates to:
  /// **'Empty project'**
  String get projectStarterEmptyTitle;

  /// No description provided for @projectStarterEmptyDescription.
  ///
  /// In en, this message translates to:
  /// **'Create only the managed project. Add content whenever you are ready.'**
  String get projectStarterEmptyDescription;

  /// No description provided for @projectStarterNpcDraftTitle.
  ///
  /// In en, this message translates to:
  /// **'NPC Draft'**
  String get projectStarterNpcDraftTitle;

  /// No description provided for @projectStarterNpcDraftDescription.
  ///
  /// In en, this message translates to:
  /// **'Create the empty project first, then open the existing guided NPC Draft setup.'**
  String get projectStarterNpcDraftDescription;

  /// No description provided for @projectStarterQuestDraftTitle.
  ///
  /// In en, this message translates to:
  /// **'Quest Draft'**
  String get projectStarterQuestDraftTitle;

  /// No description provided for @projectStarterQuestDraftDescription.
  ///
  /// In en, this message translates to:
  /// **'Create the empty project first, then open the existing guided Quest Draft setup.'**
  String get projectStarterQuestDraftDescription;

  /// No description provided for @projectStarterPartialOutcome.
  ///
  /// In en, this message translates to:
  /// **'For an NPC or Quest starter, canceling the guided setup or a Draft failure leaves a valid empty project. No starter selection writes to the game or a save.'**
  String get projectStarterPartialOutcome;

  /// No description provided for @managedContentWorkspaceBrowseLabel.
  ///
  /// In en, this message translates to:
  /// **'Browse'**
  String get managedContentWorkspaceBrowseLabel;

  /// No description provided for @managedContentWorkspaceVerifiedEditsLabel.
  ///
  /// In en, this message translates to:
  /// **'Verified edits'**
  String get managedContentWorkspaceVerifiedEditsLabel;

  /// No description provided for @managedContentScopeBaseGameLabel.
  ///
  /// In en, this message translates to:
  /// **'Base game'**
  String get managedContentScopeBaseGameLabel;

  /// No description provided for @managedContentScopeInstalledLabel.
  ///
  /// In en, this message translates to:
  /// **'Installed'**
  String get managedContentScopeInstalledLabel;

  /// No description provided for @managedBaseGameBrowserTitle.
  ///
  /// In en, this message translates to:
  /// **'Supported Base game starting points'**
  String get managedBaseGameBrowserTitle;

  /// No description provided for @managedBaseGameBrowserDescription.
  ///
  /// In en, this message translates to:
  /// **'Browse exact installed-game evidence that Mod Studio can currently inspect or use as a safe Draft starting point. This is not a complete vanilla-content catalog.'**
  String get managedBaseGameBrowserDescription;

  /// No description provided for @managedBaseGameBrowserLoading.
  ///
  /// In en, this message translates to:
  /// **'Reading exact Base game evidence…'**
  String get managedBaseGameBrowserLoading;

  /// No description provided for @managedBaseGameBrowserRefresh.
  ///
  /// In en, this message translates to:
  /// **'Read a fresh exact catalog'**
  String get managedBaseGameBrowserRefresh;

  /// No description provided for @managedBaseGameBrowserSearchLabel.
  ///
  /// In en, this message translates to:
  /// **'Search supported Base game content'**
  String get managedBaseGameBrowserSearchLabel;

  /// No description provided for @managedBaseGameBrowserFilterNpcs.
  ///
  /// In en, this message translates to:
  /// **'NPCs'**
  String get managedBaseGameBrowserFilterNpcs;

  /// No description provided for @managedBaseGameBrowserFilterQuests.
  ///
  /// In en, this message translates to:
  /// **'Quests'**
  String get managedBaseGameBrowserFilterQuests;

  /// No description provided for @managedBaseGameBrowserNpcSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'NPC starting points'**
  String get managedBaseGameBrowserNpcSectionTitle;

  /// No description provided for @managedBaseGameBrowserQuestSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Quest starting points'**
  String get managedBaseGameBrowserQuestSectionTitle;

  /// No description provided for @managedBaseGameBrowserExperimentalNpcSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Inspect-only NPC archetypes'**
  String get managedBaseGameBrowserExperimentalNpcSectionTitle;

  /// No description provided for @managedBaseGameBrowserSearchForExperimental.
  ///
  /// In en, this message translates to:
  /// **'Search to include broader static-linkage NPC evidence. Those rows cannot create a Draft.'**
  String get managedBaseGameBrowserSearchForExperimental;

  /// No description provided for @managedBaseGameBrowserEmpty.
  ///
  /// In en, this message translates to:
  /// **'No supported Base game result matches this search.'**
  String get managedBaseGameBrowserEmpty;

  /// No description provided for @managedBaseGameBrowserLoadErrorTitle.
  ///
  /// In en, this message translates to:
  /// **'Base game evidence unavailable'**
  String get managedBaseGameBrowserLoadErrorTitle;

  /// No description provided for @managedBaseGameBrowserLoadErrorDescription.
  ///
  /// In en, this message translates to:
  /// **'The exact supported catalog could not be read. No project, game, or save files were changed.'**
  String get managedBaseGameBrowserLoadErrorDescription;

  /// No description provided for @managedBaseGameBrowserOfflineDraftBadge.
  ///
  /// In en, this message translates to:
  /// **'Offline Draft supported'**
  String get managedBaseGameBrowserOfflineDraftBadge;

  /// No description provided for @managedBaseGameBrowserInspectOnlyBadge.
  ///
  /// In en, this message translates to:
  /// **'Inspect only'**
  String get managedBaseGameBrowserInspectOnlyBadge;

  /// No description provided for @managedBaseGameBrowserCreateNpcDraft.
  ///
  /// In en, this message translates to:
  /// **'Use as NPC starting point'**
  String get managedBaseGameBrowserCreateNpcDraft;

  /// No description provided for @managedBaseGameBrowserCreateQuestDraft.
  ///
  /// In en, this message translates to:
  /// **'Use as Quest starting point'**
  String get managedBaseGameBrowserCreateQuestDraft;

  /// No description provided for @managedBaseGameBrowserSpawnClass.
  ///
  /// In en, this message translates to:
  /// **'Spawn definition'**
  String get managedBaseGameBrowserSpawnClass;

  /// No description provided for @managedBaseGameBrowserActorBlueprint.
  ///
  /// In en, this message translates to:
  /// **'Actor Blueprint'**
  String get managedBaseGameBrowserActorBlueprint;

  /// No description provided for @managedBaseGameBrowserExperimentalResultsCapped.
  ///
  /// In en, this message translates to:
  /// **'Showing the first 100 inspect-only matches. Refine the search for more specific results.'**
  String get managedBaseGameBrowserExperimentalResultsCapped;

  /// No description provided for @managedInstalledBrowserLoading.
  ///
  /// In en, this message translates to:
  /// **'Reading the exact installed package inventory…'**
  String get managedInstalledBrowserLoading;

  /// No description provided for @managedInstalledBrowserCompleteSummary.
  ///
  /// In en, this message translates to:
  /// **'{count} installed package candidates'**
  String managedInstalledBrowserCompleteSummary(int count);

  /// No description provided for @managedInstalledBrowserPartialSummary.
  ///
  /// In en, this message translates to:
  /// **'{count} installed package candidates — partial result'**
  String managedInstalledBrowserPartialSummary(int count);

  /// No description provided for @managedInstalledBrowserCompleteDescription.
  ///
  /// In en, this message translates to:
  /// **'Directory metadata was read and the installed snapshot stayed exact.'**
  String get managedInstalledBrowserCompleteDescription;

  /// No description provided for @managedInstalledBrowserPartialDescription.
  ///
  /// In en, this message translates to:
  /// **'Some package metadata was missing or noncanonical, so results are useful for discovery but not complete.'**
  String get managedInstalledBrowserPartialDescription;

  /// No description provided for @managedInstalledBrowserAuthorityNotice.
  ///
  /// In en, this message translates to:
  /// **'This scope exposes installed DataAsset package metadata only. Inspecting or copying a path grants no build, deployment, runtime, or game-write authority.'**
  String get managedInstalledBrowserAuthorityNotice;

  /// No description provided for @managedInstalledBrowserRefresh.
  ///
  /// In en, this message translates to:
  /// **'Read a fresh exact snapshot'**
  String get managedInstalledBrowserRefresh;

  /// No description provided for @managedInstalledBrowserSearchLabel.
  ///
  /// In en, this message translates to:
  /// **'Search installed DataAssets'**
  String get managedInstalledBrowserSearchLabel;

  /// No description provided for @managedInstalledBrowserSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Asset name or /Game path'**
  String get managedInstalledBrowserSearchHint;

  /// No description provided for @managedInstalledBrowserSearchPrompt.
  ///
  /// In en, this message translates to:
  /// **'Type an asset name or /Game path to search.'**
  String get managedInstalledBrowserSearchPrompt;

  /// No description provided for @managedInstalledBrowserNoMatchesTitle.
  ///
  /// In en, this message translates to:
  /// **'No matching installed DataAsset'**
  String get managedInstalledBrowserNoMatchesTitle;

  /// No description provided for @managedInstalledBrowserNoMatchesDescription.
  ///
  /// In en, this message translates to:
  /// **'Try another asset name or a broader /Game path.'**
  String get managedInstalledBrowserNoMatchesDescription;

  /// No description provided for @managedInstalledBrowserResultLimitDescription.
  ///
  /// In en, this message translates to:
  /// **'Showing the first 100 matches. Refine the search to narrow the exact snapshot.'**
  String get managedInstalledBrowserResultLimitDescription;

  /// No description provided for @managedInstalledBrowserKindBadge.
  ///
  /// In en, this message translates to:
  /// **'DataAsset package'**
  String get managedInstalledBrowserKindBadge;

  /// No description provided for @managedInstalledBrowserMetadataOnlyBadge.
  ///
  /// In en, this message translates to:
  /// **'Metadata only'**
  String get managedInstalledBrowserMetadataOnlyBadge;

  /// No description provided for @managedInstalledBrowserOpenInspector.
  ///
  /// In en, this message translates to:
  /// **'Inspect exact package'**
  String get managedInstalledBrowserOpenInspector;

  /// No description provided for @managedInstalledBrowserErrorTitle.
  ///
  /// In en, this message translates to:
  /// **'Installed package inventory unavailable'**
  String get managedInstalledBrowserErrorTitle;

  /// No description provided for @managedInstalledBrowserErrorDescription.
  ///
  /// In en, this message translates to:
  /// **'The exact installed snapshot could not be read. No project, game, or save files were changed.'**
  String get managedInstalledBrowserErrorDescription;

  /// No description provided for @managedGlobalSearchScopeLabel.
  ///
  /// In en, this message translates to:
  /// **'Search all'**
  String get managedGlobalSearchScopeLabel;

  /// No description provided for @managedGlobalSearchTitle.
  ///
  /// In en, this message translates to:
  /// **'Search all content'**
  String get managedGlobalSearchTitle;

  /// No description provided for @managedGlobalSearchLabel.
  ///
  /// In en, this message translates to:
  /// **'NPC, quest, line, asset, ID, or /Game path'**
  String get managedGlobalSearchLabel;

  /// No description provided for @managedGlobalSearchAction.
  ///
  /// In en, this message translates to:
  /// **'Search'**
  String get managedGlobalSearchAction;

  /// No description provided for @managedGlobalSearchClear.
  ///
  /// In en, this message translates to:
  /// **'Clear'**
  String get managedGlobalSearchClear;

  /// No description provided for @managedGlobalSearchPrompt.
  ///
  /// In en, this message translates to:
  /// **'Enter a search to read the three sources independently.'**
  String get managedGlobalSearchPrompt;

  /// No description provided for @managedGlobalSearchNoResults.
  ///
  /// In en, this message translates to:
  /// **'No matches in this source.'**
  String get managedGlobalSearchNoResults;

  /// No description provided for @managedGlobalSearchLoading.
  ///
  /// In en, this message translates to:
  /// **'Reading exact source…'**
  String get managedGlobalSearchLoading;

  /// No description provided for @managedGlobalSearchFailed.
  ///
  /// In en, this message translates to:
  /// **'This source could not be read.'**
  String get managedGlobalSearchFailed;

  /// No description provided for @managedGlobalSearchComplete.
  ///
  /// In en, this message translates to:
  /// **'Complete'**
  String get managedGlobalSearchComplete;

  /// No description provided for @managedGlobalSearchPartial.
  ///
  /// In en, this message translates to:
  /// **'Partial'**
  String get managedGlobalSearchPartial;

  /// No description provided for @managedGlobalSearchTruncated.
  ///
  /// In en, this message translates to:
  /// **'Showing the first 100 matches. Refine the search.'**
  String get managedGlobalSearchTruncated;

  /// No description provided for @managedGlobalSearchOpen.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get managedGlobalSearchOpen;

  /// No description provided for @managedGlobalSearchCreateDraft.
  ///
  /// In en, this message translates to:
  /// **'Create Draft'**
  String get managedGlobalSearchCreateDraft;

  /// No description provided for @managedGlobalSearchInspect.
  ///
  /// In en, this message translates to:
  /// **'Inspect'**
  String get managedGlobalSearchInspect;

  /// No description provided for @managedGlobalSearchKindModEntity.
  ///
  /// In en, this message translates to:
  /// **'Mod content'**
  String get managedGlobalSearchKindModEntity;

  /// No description provided for @managedGlobalSearchKindModAsset.
  ///
  /// In en, this message translates to:
  /// **'Mod asset'**
  String get managedGlobalSearchKindModAsset;

  /// No description provided for @managedGlobalSearchKindBaseNpc.
  ///
  /// In en, this message translates to:
  /// **'NPC starting point'**
  String get managedGlobalSearchKindBaseNpc;

  /// No description provided for @managedGlobalSearchKindBaseQuest.
  ///
  /// In en, this message translates to:
  /// **'Quest starting point'**
  String get managedGlobalSearchKindBaseQuest;

  /// No description provided for @managedGlobalSearchKindExperimentalNpc.
  ///
  /// In en, this message translates to:
  /// **'NPC evidence'**
  String get managedGlobalSearchKindExperimentalNpc;

  /// No description provided for @managedGlobalSearchReadinessExact.
  ///
  /// In en, this message translates to:
  /// **'Exact current project'**
  String get managedGlobalSearchReadinessExact;

  /// No description provided for @managedGlobalSearchReadinessProblems.
  ///
  /// In en, this message translates to:
  /// **'Exact, with problems'**
  String get managedGlobalSearchReadinessProblems;

  /// No description provided for @managedGlobalSearchResultStale.
  ///
  /// In en, this message translates to:
  /// **'This result is no longer in the current project. Search again.'**
  String get managedGlobalSearchResultStale;

  /// No description provided for @managedStoryWorkbenchDraftBadge.
  ///
  /// In en, this message translates to:
  /// **'Draft only'**
  String get managedStoryWorkbenchDraftBadge;

  /// No description provided for @managedStoryWorkbenchBuildBlockedBadge.
  ///
  /// In en, this message translates to:
  /// **'Build blocked'**
  String get managedStoryWorkbenchBuildBlockedBadge;

  /// No description provided for @managedStoryWorkbenchRuntimeUnqualifiedBadge.
  ///
  /// In en, this message translates to:
  /// **'Runtime not verified'**
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge;

  /// No description provided for @managedStoryWorkbenchOverviewTab.
  ///
  /// In en, this message translates to:
  /// **'Overview'**
  String get managedStoryWorkbenchOverviewTab;

  /// No description provided for @managedStoryWorkbenchProfileTab.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get managedStoryWorkbenchProfileTab;

  /// No description provided for @managedStoryWorkbenchStoryTab.
  ///
  /// In en, this message translates to:
  /// **'Story'**
  String get managedStoryWorkbenchStoryTab;

  /// No description provided for @managedStoryWorkbenchLogicTab.
  ///
  /// In en, this message translates to:
  /// **'Logic'**
  String get managedStoryWorkbenchLogicTab;

  /// No description provided for @managedStoryWorkbenchRoutineTab.
  ///
  /// In en, this message translates to:
  /// **'Routine'**
  String get managedStoryWorkbenchRoutineTab;

  /// No description provided for @managedStoryWorkbenchInventoryTab.
  ///
  /// In en, this message translates to:
  /// **'Inventory'**
  String get managedStoryWorkbenchInventoryTab;

  /// No description provided for @managedStoryWorkbenchDialogVoiceTab.
  ///
  /// In en, this message translates to:
  /// **'Dialog & Voice'**
  String get managedStoryWorkbenchDialogVoiceTab;

  /// No description provided for @managedStoryWorkbenchReferencesTab.
  ///
  /// In en, this message translates to:
  /// **'References'**
  String get managedStoryWorkbenchReferencesTab;

  /// No description provided for @managedStoryWorkbenchProblemsChecksTab.
  ///
  /// In en, this message translates to:
  /// **'Problems & Checks'**
  String get managedStoryWorkbenchProblemsChecksTab;

  /// No description provided for @managedStoryWorkbenchEditOverview.
  ///
  /// In en, this message translates to:
  /// **'Edit name & objectives'**
  String get managedStoryWorkbenchEditOverview;

  /// No description provided for @managedStoryWorkbenchEditStory.
  ///
  /// In en, this message translates to:
  /// **'Edit description & connections'**
  String get managedStoryWorkbenchEditStory;

  /// No description provided for @managedStoryWorkbenchEditLogic.
  ///
  /// In en, this message translates to:
  /// **'Edit states & transitions'**
  String get managedStoryWorkbenchEditLogic;

  /// No description provided for @managedStoryWorkbenchInspectQuest.
  ///
  /// In en, this message translates to:
  /// **'Open source & compiler checks'**
  String get managedStoryWorkbenchInspectQuest;

  /// No description provided for @managedStoryWorkbenchInspectNpc.
  ///
  /// In en, this message translates to:
  /// **'Open profile & compiler checks'**
  String get managedStoryWorkbenchInspectNpc;

  /// No description provided for @managedStoryWorkbenchCapabilityUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Not modeled yet'**
  String get managedStoryWorkbenchCapabilityUnavailable;

  /// No description provided for @managedStoryWorkbenchNpcStoryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Quest and story relationships are not modeled for NPC drafts yet.'**
  String get managedStoryWorkbenchNpcStoryUnavailable;

  /// No description provided for @managedStoryWorkbenchNpcRoutineUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Routine and world placement are not modeled yet.'**
  String get managedStoryWorkbenchNpcRoutineUnavailable;

  /// No description provided for @managedStoryWorkbenchNpcInventoryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Inventory, equipment, and trading are not modeled yet.'**
  String get managedStoryWorkbenchNpcInventoryUnavailable;

  /// No description provided for @managedStoryWorkbenchNpcDialogVoiceUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Dialog, localization, and voice relationships are not modeled for NPC drafts yet.'**
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable;

  /// No description provided for @managedStoryWorkbenchQuestDialogVoiceUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Dialog, localization, and voice relationships are not modeled for Quest drafts yet.'**
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable;

  /// No description provided for @managedStoryWorkbenchNoReferenceProblems.
  ///
  /// In en, this message translates to:
  /// **'No unresolved project references'**
  String get managedStoryWorkbenchNoReferenceProblems;

  /// No description provided for @managedStoryWorkbenchReferenceProblemCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 unresolved project reference} other{{count} unresolved project references}}'**
  String managedStoryWorkbenchReferenceProblemCount(int count);

  /// No description provided for @managedStoryWorkbenchReferenceScopeNotice.
  ///
  /// In en, this message translates to:
  /// **'Reference status only; this is not build or runtime readiness.'**
  String get managedStoryWorkbenchReferenceScopeNotice;

  /// No description provided for @managedStoryWorkbenchTechnicalDetails.
  ///
  /// In en, this message translates to:
  /// **'Technical details'**
  String get managedStoryWorkbenchTechnicalDetails;

  /// No description provided for @managedStoryWorkbenchQuestKindLabel.
  ///
  /// In en, this message translates to:
  /// **'Quest draft'**
  String get managedStoryWorkbenchQuestKindLabel;

  /// No description provided for @managedStoryWorkbenchNpcKindLabel.
  ///
  /// In en, this message translates to:
  /// **'NPC draft'**
  String get managedStoryWorkbenchNpcKindLabel;

  /// No description provided for @managedStoryWorkbenchQuestTitleLabel.
  ///
  /// In en, this message translates to:
  /// **'Quest title'**
  String get managedStoryWorkbenchQuestTitleLabel;

  /// No description provided for @managedStoryWorkbenchTechnicalIdLabel.
  ///
  /// In en, this message translates to:
  /// **'Technical ID'**
  String get managedStoryWorkbenchTechnicalIdLabel;

  /// No description provided for @managedStoryWorkbenchObjectivesLabel.
  ///
  /// In en, this message translates to:
  /// **'Objectives'**
  String get managedStoryWorkbenchObjectivesLabel;

  /// No description provided for @managedStoryWorkbenchUniqueNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Unique name'**
  String get managedStoryWorkbenchUniqueNameLabel;

  /// No description provided for @managedStoryWorkbenchModuleNamespaceLabel.
  ///
  /// In en, this message translates to:
  /// **'Module namespace'**
  String get managedStoryWorkbenchModuleNamespaceLabel;

  /// No description provided for @managedStoryWorkbenchQuestGiverLabel.
  ///
  /// In en, this message translates to:
  /// **'Quest giver'**
  String get managedStoryWorkbenchQuestGiverLabel;

  /// No description provided for @managedStoryWorkbenchRuntimeParentLabel.
  ///
  /// In en, this message translates to:
  /// **'Runtime parent'**
  String get managedStoryWorkbenchRuntimeParentLabel;

  /// No description provided for @managedStoryWorkbenchLogicDescription.
  ///
  /// In en, this message translates to:
  /// **'Quest lifecycle states, triggers, conditions, and effects are edited as one exact-current atomic operation.'**
  String get managedStoryWorkbenchLogicDescription;

  /// No description provided for @managedStoryWorkbenchOutgoingHeading.
  ///
  /// In en, this message translates to:
  /// **'Outgoing'**
  String get managedStoryWorkbenchOutgoingHeading;

  /// No description provided for @managedStoryWorkbenchNoOutgoingReferences.
  ///
  /// In en, this message translates to:
  /// **'No projected references'**
  String get managedStoryWorkbenchNoOutgoingReferences;

  /// No description provided for @managedStoryWorkbenchIncomingHeading.
  ///
  /// In en, this message translates to:
  /// **'Incoming'**
  String get managedStoryWorkbenchIncomingHeading;

  /// No description provided for @managedStoryWorkbenchNoIncomingReferences.
  ///
  /// In en, this message translates to:
  /// **'No incoming project references'**
  String get managedStoryWorkbenchNoIncomingReferences;

  /// No description provided for @managedStoryWorkbenchSemanticIdentityLabel.
  ///
  /// In en, this message translates to:
  /// **'Semantic identity'**
  String get managedStoryWorkbenchSemanticIdentityLabel;

  /// No description provided for @managedStoryWorkbenchOriginLabel.
  ///
  /// In en, this message translates to:
  /// **'Origin'**
  String get managedStoryWorkbenchOriginLabel;

  /// No description provided for @managedStoryWorkbenchEntityRevisionLabel.
  ///
  /// In en, this message translates to:
  /// **'Entity revision'**
  String get managedStoryWorkbenchEntityRevisionLabel;

  /// No description provided for @managedStoryWorkbenchStableIdLabel.
  ///
  /// In en, this message translates to:
  /// **'Stable ID'**
  String get managedStoryWorkbenchStableIdLabel;

  /// No description provided for @managedStoryWorkbenchReferenceResolvedLabel.
  ///
  /// In en, this message translates to:
  /// **'Reference resolved'**
  String get managedStoryWorkbenchReferenceResolvedLabel;

  /// No description provided for @managedStoryWorkbenchReferenceUnresolvedLabel.
  ///
  /// In en, this message translates to:
  /// **'Reference unresolved'**
  String get managedStoryWorkbenchReferenceUnresolvedLabel;

  /// No description provided for @managedProblemsTitle.
  ///
  /// In en, this message translates to:
  /// **'Problems & readiness'**
  String get managedProblemsTitle;

  /// No description provided for @managedProblemsDescription.
  ///
  /// In en, this message translates to:
  /// **'See what needs attention and open the exact affected project content.'**
  String get managedProblemsDescription;

  /// No description provided for @managedProblemsScopeNotice.
  ///
  /// In en, this message translates to:
  /// **'Every status covers only its named scope. A clear reference check does not mean the mod can be built or tested in-game.'**
  String get managedProblemsScopeNotice;

  /// No description provided for @managedProblemsRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh problems'**
  String get managedProblemsRefresh;

  /// No description provided for @managedProblemsPartialTitle.
  ///
  /// In en, this message translates to:
  /// **'Some checks are unavailable'**
  String get managedProblemsPartialTitle;

  /// No description provided for @managedProblemsDataAssetsUnavailable.
  ///
  /// In en, this message translates to:
  /// **'DataAsset edits could not be checked. Other exact project findings are still shown.'**
  String get managedProblemsDataAssetsUnavailable;

  /// No description provided for @managedProblemsOverviewHeading.
  ///
  /// In en, this message translates to:
  /// **'Readiness by area'**
  String get managedProblemsOverviewHeading;

  /// No description provided for @managedProblemsSearchLabel.
  ///
  /// In en, this message translates to:
  /// **'Search problems'**
  String get managedProblemsSearchLabel;

  /// No description provided for @managedProblemsClearSearch.
  ///
  /// In en, this message translates to:
  /// **'Clear problem search'**
  String get managedProblemsClearSearch;

  /// No description provided for @managedProblemsListHeading.
  ///
  /// In en, this message translates to:
  /// **'Problems'**
  String get managedProblemsListHeading;

  /// No description provided for @managedProblemsEmptyTitle.
  ///
  /// In en, this message translates to:
  /// **'No modeled structural problems found'**
  String get managedProblemsEmptyTitle;

  /// No description provided for @managedProblemsEmptyDescription.
  ///
  /// In en, this message translates to:
  /// **'The exact checks currently modeled by Mod Studio found nothing to repair.'**
  String get managedProblemsEmptyDescription;

  /// No description provided for @managedProblemsEmptyBoundary.
  ///
  /// In en, this message translates to:
  /// **'Compiler evidence was not evaluated, the full managed build is unavailable, and runtime behavior remains unqualified.'**
  String get managedProblemsEmptyBoundary;

  /// No description provided for @managedProblemsFilteredEmptyTitle.
  ///
  /// In en, this message translates to:
  /// **'No matching problems'**
  String get managedProblemsFilteredEmptyTitle;

  /// No description provided for @managedProblemsFilteredEmptyDescription.
  ///
  /// In en, this message translates to:
  /// **'Change the search or category filter to see other findings.'**
  String get managedProblemsFilteredEmptyDescription;

  /// No description provided for @managedProblemsSelectTitle.
  ///
  /// In en, this message translates to:
  /// **'Select a problem'**
  String get managedProblemsSelectTitle;

  /// No description provided for @managedProblemsSelectDescription.
  ///
  /// In en, this message translates to:
  /// **'Choose a finding to see what it means and the safest available next action.'**
  String get managedProblemsSelectDescription;

  /// No description provided for @managedProblemsDetailHeading.
  ///
  /// In en, this message translates to:
  /// **'Problem details'**
  String get managedProblemsDetailHeading;

  /// No description provided for @managedProblemsCloseDetail.
  ///
  /// In en, this message translates to:
  /// **'Close problem details'**
  String get managedProblemsCloseDetail;

  /// No description provided for @managedProblemsCategoryLabel.
  ///
  /// In en, this message translates to:
  /// **'Area'**
  String get managedProblemsCategoryLabel;

  /// No description provided for @managedProblemsSeverityLabel.
  ///
  /// In en, this message translates to:
  /// **'Attention'**
  String get managedProblemsSeverityLabel;

  /// No description provided for @managedProblemsSourceLabel.
  ///
  /// In en, this message translates to:
  /// **'Evidence'**
  String get managedProblemsSourceLabel;

  /// No description provided for @managedProblemsOpenSourceEntity.
  ///
  /// In en, this message translates to:
  /// **'Open source content'**
  String get managedProblemsOpenSourceEntity;

  /// No description provided for @managedProblemsOpenReferencedAsset.
  ///
  /// In en, this message translates to:
  /// **'Open referenced asset'**
  String get managedProblemsOpenReferencedAsset;

  /// No description provided for @managedProblemsOpenDataAssetEdits.
  ///
  /// In en, this message translates to:
  /// **'Open DataAsset edits'**
  String get managedProblemsOpenDataAssetEdits;

  /// No description provided for @managedProblemsActionFailed.
  ///
  /// In en, this message translates to:
  /// **'The exact target could not be opened. Refresh the project problems and try again.'**
  String get managedProblemsActionFailed;

  /// No description provided for @managedProblemsActionProgress.
  ///
  /// In en, this message translates to:
  /// **'Opening the exact project target'**
  String get managedProblemsActionProgress;

  /// No description provided for @managedProblemsCategoryReferences.
  ///
  /// In en, this message translates to:
  /// **'References'**
  String get managedProblemsCategoryReferences;

  /// No description provided for @managedProblemsCategorySetup.
  ///
  /// In en, this message translates to:
  /// **'Setup'**
  String get managedProblemsCategorySetup;

  /// No description provided for @managedProblemsCategoryDataAssets.
  ///
  /// In en, this message translates to:
  /// **'DataAssets'**
  String get managedProblemsCategoryDataAssets;

  /// No description provided for @managedProblemsSeverityInformation.
  ///
  /// In en, this message translates to:
  /// **'Information'**
  String get managedProblemsSeverityInformation;

  /// No description provided for @managedProblemsSeverityWarning.
  ///
  /// In en, this message translates to:
  /// **'Needs attention'**
  String get managedProblemsSeverityWarning;

  /// No description provided for @managedProblemsSeverityBlocking.
  ///
  /// In en, this message translates to:
  /// **'Blocks this scope'**
  String get managedProblemsSeverityBlocking;

  /// No description provided for @managedProblemsScopeReferencesTitle.
  ///
  /// In en, this message translates to:
  /// **'Reference integrity'**
  String get managedProblemsScopeReferencesTitle;

  /// No description provided for @managedProblemsScopeReferencesDescription.
  ///
  /// In en, this message translates to:
  /// **'Checks exact links between current project content and assets.'**
  String get managedProblemsScopeReferencesDescription;

  /// No description provided for @managedProblemsScopeDataAssetsTitle.
  ///
  /// In en, this message translates to:
  /// **'DataAsset edit registry'**
  String get managedProblemsScopeDataAssetsTitle;

  /// No description provided for @managedProblemsScopeDataAssetsDescription.
  ///
  /// In en, this message translates to:
  /// **'Checks whether the exact current list of saved DataAsset edits could be read.'**
  String get managedProblemsScopeDataAssetsDescription;

  /// No description provided for @managedProblemsScopeGameTitle.
  ///
  /// In en, this message translates to:
  /// **'Game setup'**
  String get managedProblemsScopeGameTitle;

  /// No description provided for @managedProblemsScopeGameDescription.
  ///
  /// In en, this message translates to:
  /// **'Shows whether a game installation is configured for bounded read-only tools.'**
  String get managedProblemsScopeGameDescription;

  /// No description provided for @managedProblemsScopeCompilerTitle.
  ///
  /// In en, this message translates to:
  /// **'Source & compiler evidence'**
  String get managedProblemsScopeCompilerTitle;

  /// No description provided for @managedProblemsScopeCompilerDescription.
  ///
  /// In en, this message translates to:
  /// **'Compiler checks run only when you explicitly open and start them for one exact entity.'**
  String get managedProblemsScopeCompilerDescription;

  /// No description provided for @managedProblemsScopeBuildTitle.
  ///
  /// In en, this message translates to:
  /// **'Managed project build'**
  String get managedProblemsScopeBuildTitle;

  /// No description provided for @managedProblemsScopeBuildDescription.
  ///
  /// In en, this message translates to:
  /// **'A complete build path for managed NPC, Quest, dialog, and DataAsset edits is not available yet.'**
  String get managedProblemsScopeBuildDescription;

  /// No description provided for @managedProblemsScopeRuntimeTitle.
  ///
  /// In en, this message translates to:
  /// **'In-game behavior'**
  String get managedProblemsScopeRuntimeTitle;

  /// No description provided for @managedProblemsScopeRuntimeDescription.
  ///
  /// In en, this message translates to:
  /// **'No general runtime, save, deployment, or cleanup qualification is claimed.'**
  String get managedProblemsScopeRuntimeDescription;

  /// No description provided for @managedProblemsReadinessClear.
  ///
  /// In en, this message translates to:
  /// **'Checked within this scope'**
  String get managedProblemsReadinessClear;

  /// No description provided for @managedProblemsReadinessIssues.
  ///
  /// In en, this message translates to:
  /// **'Needs attention'**
  String get managedProblemsReadinessIssues;

  /// No description provided for @managedProblemsReadinessUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Check unavailable'**
  String get managedProblemsReadinessUnavailable;

  /// No description provided for @managedProblemsReadinessNotEvaluated.
  ///
  /// In en, this message translates to:
  /// **'Not evaluated'**
  String get managedProblemsReadinessNotEvaluated;

  /// No description provided for @managedProblemsReadinessBlocked.
  ///
  /// In en, this message translates to:
  /// **'Build path unavailable'**
  String get managedProblemsReadinessBlocked;

  /// No description provided for @managedProblemsReadinessUnqualified.
  ///
  /// In en, this message translates to:
  /// **'Runtime unqualified'**
  String get managedProblemsReadinessUnqualified;

  /// No description provided for @managedProblemsEvidenceContent.
  ///
  /// In en, this message translates to:
  /// **'Exact current project content'**
  String get managedProblemsEvidenceContent;

  /// No description provided for @managedProblemsEvidenceDataAssets.
  ///
  /// In en, this message translates to:
  /// **'Exact current DataAsset registry'**
  String get managedProblemsEvidenceDataAssets;

  /// No description provided for @managedProblemsEvidenceConfiguration.
  ///
  /// In en, this message translates to:
  /// **'Current app configuration'**
  String get managedProblemsEvidenceConfiguration;

  /// No description provided for @managedProblemsEvidenceUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Evidence source unavailable'**
  String get managedProblemsEvidenceUnavailable;

  /// No description provided for @managedProblemsEvidenceBoundary.
  ///
  /// In en, this message translates to:
  /// **'Known capability boundary'**
  String get managedProblemsEvidenceBoundary;

  /// No description provided for @managedProblemsForeignReferenceTitle.
  ///
  /// In en, this message translates to:
  /// **'Reference points to another project'**
  String get managedProblemsForeignReferenceTitle;

  /// No description provided for @managedProblemsMissingEntityTitle.
  ///
  /// In en, this message translates to:
  /// **'Linked project content is missing'**
  String get managedProblemsMissingEntityTitle;

  /// No description provided for @managedProblemsEntityKindTitle.
  ///
  /// In en, this message translates to:
  /// **'Linked project content has the wrong type'**
  String get managedProblemsEntityKindTitle;

  /// No description provided for @managedProblemsMissingAssetTitle.
  ///
  /// In en, this message translates to:
  /// **'Linked project file is missing'**
  String get managedProblemsMissingAssetTitle;

  /// No description provided for @managedProblemsAssetLengthTitle.
  ///
  /// In en, this message translates to:
  /// **'Linked project file has an unexpected size'**
  String get managedProblemsAssetLengthTitle;

  /// No description provided for @managedProblemsAssetTypeTitle.
  ///
  /// In en, this message translates to:
  /// **'Linked project file has an unexpected type'**
  String get managedProblemsAssetTypeTitle;

  /// No description provided for @managedProblemsGameSetupTitle.
  ///
  /// In en, this message translates to:
  /// **'Game installation is not configured'**
  String get managedProblemsGameSetupTitle;

  /// No description provided for @managedProblemsDataAssetRegistryTitle.
  ///
  /// In en, this message translates to:
  /// **'DataAsset edits could not be checked'**
  String get managedProblemsDataAssetRegistryTitle;

  /// No description provided for @managedProblemsDataAssetOfflineTitle.
  ///
  /// In en, this message translates to:
  /// **'DataAsset edit is draft-only'**
  String get managedProblemsDataAssetOfflineTitle;

  /// No description provided for @managedProblemsEntityReferenceDescription.
  ///
  /// In en, this message translates to:
  /// **'Open {source} and repair this exact project-content link.'**
  String managedProblemsEntityReferenceDescription(String source);

  /// No description provided for @managedProblemsAssetReferenceDescription.
  ///
  /// In en, this message translates to:
  /// **'Open {source} and repair this exact project-file link.'**
  String managedProblemsAssetReferenceDescription(String source);

  /// No description provided for @managedProblemsDataAssetRegistryDescription.
  ///
  /// In en, this message translates to:
  /// **'Refresh the exact current project. No conclusion is drawn about saved DataAsset edits until this source is available.'**
  String get managedProblemsDataAssetRegistryDescription;

  /// No description provided for @managedProblemsDataAssetOfflineDescription.
  ///
  /// In en, this message translates to:
  /// **'The saved edit for {targetPath} can be reviewed in DataAsset edits, but it cannot be emitted by a managed project build or claimed as working in-game yet.'**
  String managedProblemsDataAssetOfflineDescription(String targetPath);
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
