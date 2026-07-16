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

  /// No description provided for @managedWorkspaceHistoryLabel.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get managedWorkspaceHistoryLabel;

  /// No description provided for @managedWorkspaceSettingsExpertLabel.
  ///
  /// In en, this message translates to:
  /// **'Settings & Expert'**
  String get managedWorkspaceSettingsExpertLabel;

  /// No description provided for @managedProjectHistoryTitle.
  ///
  /// In en, this message translates to:
  /// **'Project history'**
  String get managedProjectHistoryTitle;

  /// No description provided for @managedProjectHistoryDescription.
  ///
  /// In en, this message translates to:
  /// **'Return to an earlier project version without erasing the versions that came after it.'**
  String get managedProjectHistoryDescription;

  /// No description provided for @managedProjectHistoryBoundary.
  ///
  /// In en, this message translates to:
  /// **'History changes only this managed project. It does not modify the game installation or save files.'**
  String get managedProjectHistoryBoundary;

  /// No description provided for @managedProjectHistoryRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh project history'**
  String get managedProjectHistoryRefresh;

  /// No description provided for @managedProjectHistoryLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading project history…'**
  String get managedProjectHistoryLoading;

  /// No description provided for @managedProjectHistoryLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Project history could not be loaded'**
  String get managedProjectHistoryLoadFailed;

  /// No description provided for @managedProjectHistoryRetry.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get managedProjectHistoryRetry;

  /// No description provided for @managedProjectHistoryCurrentVersion.
  ///
  /// In en, this message translates to:
  /// **'Current version'**
  String get managedProjectHistoryCurrentVersion;

  /// No description provided for @managedProjectHistoryPreviousVersions.
  ///
  /// In en, this message translates to:
  /// **'Previous versions'**
  String get managedProjectHistoryPreviousVersions;

  /// No description provided for @managedProjectHistoryUndo.
  ///
  /// In en, this message translates to:
  /// **'Undo last change'**
  String get managedProjectHistoryUndo;

  /// No description provided for @managedProjectHistoryRestoreVersion.
  ///
  /// In en, this message translates to:
  /// **'Restore this version'**
  String get managedProjectHistoryRestoreVersion;

  /// No description provided for @managedProjectHistoryRestoreTitle.
  ///
  /// In en, this message translates to:
  /// **'Restore project version?'**
  String get managedProjectHistoryRestoreTitle;

  /// No description provided for @managedProjectHistoryRestoreBody.
  ///
  /// In en, this message translates to:
  /// **'The content from revision {revision} will be saved as new revision {nextRevision}. The current version remains in history.'**
  String managedProjectHistoryRestoreBody(int revision, int nextRevision);

  /// No description provided for @managedProjectHistoryRestoreBoundary.
  ///
  /// In en, this message translates to:
  /// **'Only the project changes. The game installation and save files remain untouched.'**
  String get managedProjectHistoryRestoreBoundary;

  /// No description provided for @managedProjectHistoryCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get managedProjectHistoryCancel;

  /// No description provided for @managedProjectHistoryRestore.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get managedProjectHistoryRestore;

  /// No description provided for @managedProjectHistoryRestoring.
  ///
  /// In en, this message translates to:
  /// **'Restoring project version…'**
  String get managedProjectHistoryRestoring;

  /// No description provided for @managedProjectHistoryRestoreFailed.
  ///
  /// In en, this message translates to:
  /// **'The project version could not be restored safely. Refresh the history before trying again.'**
  String get managedProjectHistoryRestoreFailed;

  /// No description provided for @managedProjectHistoryRestoreSucceeded.
  ///
  /// In en, this message translates to:
  /// **'Revision {revision} was restored as a new project version.'**
  String managedProjectHistoryRestoreSucceeded(int revision);

  /// No description provided for @managedProjectHistoryEmpty.
  ///
  /// In en, this message translates to:
  /// **'No previous project versions have been recorded yet.'**
  String get managedProjectHistoryEmpty;

  /// No description provided for @managedProjectHistoryRecordingStartsAt.
  ///
  /// In en, this message translates to:
  /// **'History recording starts at revision {revision}; older versions were not guessed from storage.'**
  String managedProjectHistoryRecordingStartsAt(int revision);

  /// No description provided for @managedProjectHistoryTruncated.
  ///
  /// In en, this message translates to:
  /// **'Older project versions have expired from history. Every version shown here is still retained and authenticated by the current project history.'**
  String get managedProjectHistoryTruncated;

  /// No description provided for @managedProjectHistoryRevision.
  ///
  /// In en, this message translates to:
  /// **'Revision {revision}'**
  String managedProjectHistoryRevision(int revision);

  /// No description provided for @managedProjectHistoryCurrentBadge.
  ///
  /// In en, this message translates to:
  /// **'Current'**
  String get managedProjectHistoryCurrentBadge;

  /// No description provided for @managedProjectHistoryDirtyBlocked.
  ///
  /// In en, this message translates to:
  /// **'Finish or discard the open text edit before restoring another project version.'**
  String get managedProjectHistoryDirtyBlocked;

  /// No description provided for @managedProjectHistoryBusy.
  ///
  /// In en, this message translates to:
  /// **'Another project action is still in progress.'**
  String get managedProjectHistoryBusy;

  /// No description provided for @managedProjectHistoryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'This managed project session does not support authenticated history.'**
  String get managedProjectHistoryUnavailable;

  /// No description provided for @managedSectionStoryDescription.
  ///
  /// In en, this message translates to:
  /// **'NPCs, quests, and dialogue.'**
  String get managedSectionStoryDescription;

  /// No description provided for @managedStoryWorkspaceLoading.
  ///
  /// In en, this message translates to:
  /// **'Opening the current Story drafts…'**
  String get managedStoryWorkspaceLoading;

  /// No description provided for @managedStoryWorkspaceAuthorityNotice.
  ///
  /// In en, this message translates to:
  /// **'Project-only NPC and Quest drafts. Build readiness has not been evaluated; runtime behavior remains unqualified.'**
  String get managedStoryWorkspaceAuthorityNotice;

  /// No description provided for @managedStoryWorkspaceSearchHint.
  ///
  /// In en, this message translates to:
  /// **'Search NPC and Quest names, objectives, speakers, or IDs'**
  String get managedStoryWorkspaceSearchHint;

  /// No description provided for @managedStoryWorkspaceCreatingNpc.
  ///
  /// In en, this message translates to:
  /// **'Creating NPC draft…'**
  String get managedStoryWorkspaceCreatingNpc;

  /// No description provided for @managedStoryWorkspaceCreatingQuest.
  ///
  /// In en, this message translates to:
  /// **'Creating Quest draft…'**
  String get managedStoryWorkspaceCreatingQuest;

  /// No description provided for @managedStoryWorkspaceEmpty.
  ///
  /// In en, this message translates to:
  /// **'No NPC or Quest drafts yet'**
  String get managedStoryWorkspaceEmpty;

  /// No description provided for @managedStoryWorkspaceNoMatches.
  ///
  /// In en, this message translates to:
  /// **'No NPC or Quest drafts match this search'**
  String get managedStoryWorkspaceNoMatches;

  /// No description provided for @managedStoryWorkspaceSelectDraft.
  ///
  /// In en, this message translates to:
  /// **'Select an NPC or Quest draft to continue'**
  String get managedStoryWorkspaceSelectDraft;

  /// No description provided for @managedStoryWorkspaceLoadErrorTitle.
  ///
  /// In en, this message translates to:
  /// **'Story drafts could not be opened'**
  String get managedStoryWorkspaceLoadErrorTitle;

  /// No description provided for @managedStoryWorkspaceCheckpointMismatch.
  ///
  /// In en, this message translates to:
  /// **'The project changed while Story was loading. Refresh the exact current checkpoint and try again.'**
  String get managedStoryWorkspaceCheckpointMismatch;

  /// No description provided for @managedStoryWorkspacePublishedSelectionStale.
  ///
  /// In en, this message translates to:
  /// **'The saved Story draft could not be selected at its exact project revision. Check the current Story list before continuing.'**
  String get managedStoryWorkspacePublishedSelectionStale;

  /// No description provided for @managedStoryWorkspaceCheckpointSummary.
  ///
  /// In en, this message translates to:
  /// **'NPC and Quest drafts: {count} · project revision {revision}'**
  String managedStoryWorkspaceCheckpointSummary(int count, int revision);

  /// No description provided for @managedStoryWorkspaceLoadErrorDetails.
  ///
  /// In en, this message translates to:
  /// **'The exact current Story view could not be read: {error}'**
  String managedStoryWorkspaceLoadErrorDetails(String error);

  /// No description provided for @managedStoryWorkspaceCreateErrorDetails.
  ///
  /// In en, this message translates to:
  /// **'The Story draft could not be created: {error}'**
  String managedStoryWorkspaceCreateErrorDetails(String error);

  /// No description provided for @managedStoryWorkspaceDetailsSheetLabel.
  ///
  /// In en, this message translates to:
  /// **'{entityName} Story details'**
  String managedStoryWorkspaceDetailsSheetLabel(String entityName);

  /// No description provided for @managedStoryWorkspaceRemovePairUnavailable.
  ///
  /// In en, this message translates to:
  /// **'This draft is not an exact removable draft and generated-script pair.'**
  String get managedStoryWorkspaceRemovePairUnavailable;

  /// No description provided for @managedStoryWorkspaceRemoveBusy.
  ///
  /// In en, this message translates to:
  /// **'Another Story action is still in progress.'**
  String get managedStoryWorkspaceRemoveBusy;

  /// No description provided for @managedStoryWorkspaceRemoveRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'Reopen this managed project before removing a draft.'**
  String get managedStoryWorkspaceRemoveRequiresReopen;

  /// No description provided for @managedStoryWorkspaceRemoveBlocked.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 incoming project reference must be removed first.} other{{count} incoming project references must be removed first.}}'**
  String managedStoryWorkspaceRemoveBlocked(int count);

  /// No description provided for @managedStoryWorkspaceRemoveDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove draft from project?'**
  String get managedStoryWorkspaceRemoveDialogTitle;

  /// No description provided for @managedStoryWorkspaceRemoveDialogSummary.
  ///
  /// In en, this message translates to:
  /// **'This removes the draft \'{draftName}\' together with its uniquely owned generated script \'{scriptName}\'.'**
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  );

  /// No description provided for @managedStoryWorkspaceRemoveNoUndo.
  ///
  /// In en, this message translates to:
  /// **'This removal cannot be undone in version 1.'**
  String get managedStoryWorkspaceRemoveNoUndo;

  /// No description provided for @managedStoryWorkspaceRemoveBoundary.
  ///
  /// In en, this message translates to:
  /// **'Only the current project registry is changed. The game installation and save games stay unchanged.'**
  String get managedStoryWorkspaceRemoveBoundary;

  /// No description provided for @managedStoryWorkspaceRemoveCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get managedStoryWorkspaceRemoveCancel;

  /// No description provided for @managedStoryWorkspaceRemoveConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove draft'**
  String get managedStoryWorkspaceRemoveConfirm;

  /// No description provided for @managedStoryWorkspaceRemoveBlockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Draft is still referenced'**
  String get managedStoryWorkspaceRemoveBlockedTitle;

  /// No description provided for @managedStoryWorkspaceRemoveBlockedDescription.
  ///
  /// In en, this message translates to:
  /// **'Open every source below and remove its project reference before trying again.'**
  String get managedStoryWorkspaceRemoveBlockedDescription;

  /// No description provided for @managedStoryWorkspaceRemoveBlockerLabel.
  ///
  /// In en, this message translates to:
  /// **'{sourceName} · {role}'**
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  );

  /// No description provided for @managedStoryWorkspaceRemoveOpenBlocker.
  ///
  /// In en, this message translates to:
  /// **'Open referencing source'**
  String get managedStoryWorkspaceRemoveOpenBlocker;

  /// No description provided for @managedStoryWorkspaceRemoveBlockedClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get managedStoryWorkspaceRemoveBlockedClose;

  /// No description provided for @managedStoryWorkspaceRemoveSucceeded.
  ///
  /// In en, this message translates to:
  /// **'Removed \'{draftName}\' and its generated script from the project. Game files and save games were not changed.'**
  String managedStoryWorkspaceRemoveSucceeded(String draftName);

  /// No description provided for @managedStoryWorkspaceRemoveError.
  ///
  /// In en, this message translates to:
  /// **'The draft was not removed. The Story view was refreshed without retrying automatically: {error}'**
  String managedStoryWorkspaceRemoveError(String error);

  /// No description provided for @managedSectionWorldDescription.
  ///
  /// In en, this message translates to:
  /// **'World placement and workflows are planned.'**
  String get managedSectionWorldDescription;

  /// No description provided for @managedSectionLocalizationVoiceDescription.
  ///
  /// In en, this message translates to:
  /// **'Write and translate project dialog, then review each language\'s takes, selection, and target in the same workspace.'**
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

  /// No description provided for @managedLocalizationVoiceContextTitle.
  ///
  /// In en, this message translates to:
  /// **'Voice for this dialog line'**
  String get managedLocalizationVoiceContextTitle;

  /// No description provided for @managedLocalizationVoiceSelectLine.
  ///
  /// In en, this message translates to:
  /// **'Select a dialog line above'**
  String get managedLocalizationVoiceSelectLine;

  /// No description provided for @managedLocalizationVoiceSetupExists.
  ///
  /// In en, this message translates to:
  /// **'setup exists'**
  String get managedLocalizationVoiceSetupExists;

  /// No description provided for @managedLocalizationVoiceSetupMissing.
  ///
  /// In en, this message translates to:
  /// **'no setup yet'**
  String get managedLocalizationVoiceSetupMissing;

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

  /// No description provided for @managedLocalizationVoiceActionFailed.
  ///
  /// In en, this message translates to:
  /// **'The Voice action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.'**
  String get managedLocalizationVoiceActionFailed;

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
  /// **'Recover or reopen the managed project before reading its content.'**
  String get managedProjectRecoveryContentLocked;

  /// No description provided for @managedProjectRecoveryDescription.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio will safely reopen this project while keeping its lock. This does not change the game or any save.'**
  String get managedProjectRecoveryDescription;

  /// No description provided for @managedProjectRecoveryTry.
  ///
  /// In en, this message translates to:
  /// **'Try recovery'**
  String get managedProjectRecoveryTry;

  /// No description provided for @managedProjectRecoveryTrying.
  ///
  /// In en, this message translates to:
  /// **'Trying recovery…'**
  String get managedProjectRecoveryTrying;

  /// No description provided for @managedProjectRecoveryAlternative.
  ///
  /// In en, this message translates to:
  /// **'If recovery does not work, close and open the project again.'**
  String get managedProjectRecoveryAlternative;

  /// No description provided for @managedProjectRecoverySucceeded.
  ///
  /// In en, this message translates to:
  /// **'Project recovery completed. You can continue working.'**
  String get managedProjectRecoverySucceeded;

  /// No description provided for @managedProjectRecoveryFailed.
  ///
  /// In en, this message translates to:
  /// **'Recovery did not complete. Try again, or close and open the project again.'**
  String get managedProjectRecoveryFailed;

  /// No description provided for @managedProjectRecoveryUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Recovery is not available for this project. Close and open the project again.'**
  String get managedProjectRecoveryUnavailable;

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

  /// No description provided for @managedStoryWorkbenchMoreActions.
  ///
  /// In en, this message translates to:
  /// **'More actions'**
  String get managedStoryWorkbenchMoreActions;

  /// No description provided for @managedStoryWorkbenchRemoveDraft.
  ///
  /// In en, this message translates to:
  /// **'Remove draft…'**
  String get managedStoryWorkbenchRemoveDraft;

  /// No description provided for @managedStoryWorkbenchRemovingDraft.
  ///
  /// In en, this message translates to:
  /// **'Removing draft…'**
  String get managedStoryWorkbenchRemovingDraft;

  /// No description provided for @managedStoryWorkbenchReviewRemovalBlockers.
  ///
  /// In en, this message translates to:
  /// **'Review removal blockers'**
  String get managedStoryWorkbenchReviewRemovalBlockers;

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

  /// No description provided for @projectExportActionTitle.
  ///
  /// In en, this message translates to:
  /// **'Export project copy…'**
  String get projectExportActionTitle;

  /// No description provided for @projectExportActionDescription.
  ///
  /// In en, this message translates to:
  /// **'Write an exact portable copy of the current saved project checkpoint.'**
  String get projectExportActionDescription;

  /// No description provided for @projectExportActionDirtyBlocked.
  ///
  /// In en, this message translates to:
  /// **'Save or discard the open localization edits before exporting a project copy.'**
  String get projectExportActionDirtyBlocked;

  /// No description provided for @projectExportDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Export project copy'**
  String get projectExportDialogTitle;

  /// No description provided for @projectExportPortableCopyTitle.
  ///
  /// In en, this message translates to:
  /// **'Portable project copy'**
  String get projectExportPortableCopyTitle;

  /// No description provided for @projectExportPortableCopyDescription.
  ///
  /// In en, this message translates to:
  /// **'This writes the exact current saved project checkpoint to a new .goremod file. The open project stays current and unchanged.'**
  String get projectExportPortableCopyDescription;

  /// No description provided for @projectExportCapabilityBoundary.
  ///
  /// In en, this message translates to:
  /// **'This copy is not a playable mod, build, deployment, or restorable backup. It does not read or change the game or any save.'**
  String get projectExportCapabilityBoundary;

  /// No description provided for @projectExportKeepOriginal.
  ///
  /// In en, this message translates to:
  /// **'Importing this managed copy is not available yet. Keep the original project folder.'**
  String get projectExportKeepOriginal;

  /// No description provided for @projectExportFileNameLabel.
  ///
  /// In en, this message translates to:
  /// **'New project-copy file'**
  String get projectExportFileNameLabel;

  /// No description provided for @projectExportFileNameHelper.
  ///
  /// In en, this message translates to:
  /// **'Use a new portable file name ending in .goremod.'**
  String get projectExportFileNameHelper;

  /// No description provided for @projectExportChooseDestination.
  ///
  /// In en, this message translates to:
  /// **'Choose destination folder'**
  String get projectExportChooseDestination;

  /// No description provided for @projectExportNoDestination.
  ///
  /// In en, this message translates to:
  /// **'No destination folder selected'**
  String get projectExportNoDestination;

  /// No description provided for @projectExportNewFile.
  ///
  /// In en, this message translates to:
  /// **'New file'**
  String get projectExportNewFile;

  /// No description provided for @projectExportCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get projectExportCancel;

  /// No description provided for @projectExportClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get projectExportClose;

  /// No description provided for @projectExportSubmit.
  ///
  /// In en, this message translates to:
  /// **'Export copy'**
  String get projectExportSubmit;

  /// No description provided for @projectExportExporting.
  ///
  /// In en, this message translates to:
  /// **'Exporting…'**
  String get projectExportExporting;

  /// No description provided for @projectExportParentRequired.
  ///
  /// In en, this message translates to:
  /// **'Choose an existing destination folder.'**
  String get projectExportParentRequired;

  /// No description provided for @projectExportParentAbsolute.
  ///
  /// In en, this message translates to:
  /// **'Choose an absolute existing destination folder.'**
  String get projectExportParentAbsolute;

  /// No description provided for @projectExportParentLink.
  ///
  /// In en, this message translates to:
  /// **'The selected destination is a link. Choose a real existing folder.'**
  String get projectExportParentLink;

  /// No description provided for @projectExportParentInspectFailed.
  ///
  /// In en, this message translates to:
  /// **'The destination folder could not be inspected safely. Nothing was created.'**
  String get projectExportParentInspectFailed;

  /// No description provided for @projectExportFileNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a new project-copy file name.'**
  String get projectExportFileNameRequired;

  /// No description provided for @projectExportFileNameTooLong.
  ///
  /// In en, this message translates to:
  /// **'The file name must be at most 128 ASCII characters.'**
  String get projectExportFileNameTooLong;

  /// No description provided for @projectExportFileNameInvalid.
  ///
  /// In en, this message translates to:
  /// **'Start with a letter or digit, use only ASCII letters, digits, dots, underscores, or hyphens, and end with .goremod.'**
  String get projectExportFileNameInvalid;

  /// No description provided for @projectExportFileNameReserved.
  ///
  /// In en, this message translates to:
  /// **'That file name is reserved by Windows.'**
  String get projectExportFileNameReserved;

  /// No description provided for @projectExportOutputExists.
  ///
  /// In en, this message translates to:
  /// **'That file already exists. Choose a new file name; existing files are never overwritten.'**
  String get projectExportOutputExists;

  /// No description provided for @projectExportOutputLink.
  ///
  /// In en, this message translates to:
  /// **'The new file path is a link. Choose a different file name.'**
  String get projectExportOutputLink;

  /// No description provided for @projectExportOutputRejected.
  ///
  /// In en, this message translates to:
  /// **'The destination was rejected before the new local file was created. Nothing was created. Choose a different file name or destination folder.'**
  String get projectExportOutputRejected;

  /// No description provided for @projectExportStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed before export started. No output was created. Close this window and open Export project copy again.'**
  String get projectExportStale;

  /// No description provided for @projectExportRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.'**
  String get projectExportRequiresReopen;

  /// No description provided for @projectExportUnsupported.
  ///
  /// In en, this message translates to:
  /// **'This managed project session cannot export exact portable copies. Nothing was created.'**
  String get projectExportUnsupported;

  /// No description provided for @projectExportFailedBeforeStart.
  ///
  /// In en, this message translates to:
  /// **'The project copy could not be prepared exactly. Nothing was created.'**
  String get projectExportFailedBeforeStart;

  /// No description provided for @projectExportPrepublicationFailed.
  ///
  /// In en, this message translates to:
  /// **'Export stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.'**
  String get projectExportPrepublicationFailed;

  /// No description provided for @projectExportMayExist.
  ///
  /// In en, this message translates to:
  /// **'The export did not return a verified receipt. Do not retry. Close this window and check the destination: {output}'**
  String projectExportMayExist(String output);

  /// No description provided for @projectExportResultMismatch.
  ///
  /// In en, this message translates to:
  /// **'The completed export does not match this checkpoint or destination. Do not retry; inspect the destination: {output}'**
  String projectExportResultMismatch(String output);

  /// No description provided for @projectExportPublished.
  ///
  /// In en, this message translates to:
  /// **'The exact portable project copy was created as a new local file.'**
  String get projectExportPublished;

  /// No description provided for @projectExportPublishedCleanupWarning.
  ///
  /// In en, this message translates to:
  /// **'The exact project copy was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.'**
  String get projectExportPublishedCleanupWarning;

  /// No description provided for @projectExportPublicationUncertain.
  ///
  /// In en, this message translates to:
  /// **'The local file may have been created. Do not retry. Check whether this destination exists: {output}'**
  String projectExportPublicationUncertain(String output);

  /// No description provided for @projectExportArchiveBytes.
  ///
  /// In en, this message translates to:
  /// **'Archive bytes'**
  String get projectExportArchiveBytes;

  /// No description provided for @projectExportArchiveSha256.
  ///
  /// In en, this message translates to:
  /// **'Archive SHA-256'**
  String get projectExportArchiveSha256;

  /// No description provided for @projectExportCurrentProjectUnchanged.
  ///
  /// In en, this message translates to:
  /// **'The current project remains open and unchanged. The game and saves were not touched.'**
  String get projectExportCurrentProjectUnchanged;

  /// No description provided for @managedVoiceTakeRemoveAction.
  ///
  /// In en, this message translates to:
  /// **'Remove from this line…'**
  String get managedVoiceTakeRemoveAction;

  /// No description provided for @managedVoiceTakeRemoveTooltip.
  ///
  /// In en, this message translates to:
  /// **'Remove this recording from the current dialog line and language'**
  String get managedVoiceTakeRemoveTooltip;

  /// No description provided for @managedVoiceTakeRemoveDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove Voice take?'**
  String get managedVoiceTakeRemoveDialogTitle;

  /// No description provided for @managedVoiceTakeRemoveDialogSummary.
  ///
  /// In en, this message translates to:
  /// **'Remove “{take}” from {line} ({locale})?'**
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  );

  /// No description provided for @managedVoiceTakeRemoveScope.
  ///
  /// In en, this message translates to:
  /// **'Only the link for this dialog line and language is removed. Other project uses remain unchanged.'**
  String get managedVoiceTakeRemoveScope;

  /// No description provided for @managedVoiceTakeRemoveInternalRetention.
  ///
  /// In en, this message translates to:
  /// **'The audio file remains stored internally. This action does not free project storage and has no undo yet.'**
  String get managedVoiceTakeRemoveInternalRetention;

  /// No description provided for @managedVoiceTakeRemoveGameBoundary.
  ///
  /// In en, this message translates to:
  /// **'The game installation and save games are not changed.'**
  String get managedVoiceTakeRemoveGameBoundary;

  /// No description provided for @managedVoiceTakeRemoveSelectedWarning.
  ///
  /// In en, this message translates to:
  /// **'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.'**
  String get managedVoiceTakeRemoveSelectedWarning;

  /// No description provided for @managedVoiceTakeRemoveCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get managedVoiceTakeRemoveCancel;

  /// No description provided for @managedVoiceTakeRemoveConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove from line'**
  String get managedVoiceTakeRemoveConfirm;

  /// No description provided for @managedVoiceTakeRemoveUniqueSuccess.
  ///
  /// In en, this message translates to:
  /// **'The take was removed from this line and from the current project graph. Its internal audio data remains retained.'**
  String get managedVoiceTakeRemoveUniqueSuccess;

  /// No description provided for @managedVoiceTakeRemoveSharedSuccess.
  ///
  /// In en, this message translates to:
  /// **'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.'**
  String get managedVoiceTakeRemoveSharedSuccess;

  /// No description provided for @managedVoiceTakeRemoveSelectionClearedSuccess.
  ///
  /// In en, this message translates to:
  /// **'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.'**
  String get managedVoiceTakeRemoveSelectionClearedSuccess;

  /// No description provided for @managedVoiceTakeRemoveStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.'**
  String get managedVoiceTakeRemoveStale;

  /// No description provided for @managedVoiceTakeRemoveRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.'**
  String get managedVoiceTakeRemoveRequiresReopen;

  /// No description provided for @managedVoiceTakeRemoveSavedUnconfirmed.
  ///
  /// In en, this message translates to:
  /// **'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.'**
  String get managedVoiceTakeRemoveSavedUnconfirmed;

  /// No description provided for @managedVoiceTakeRemoveSavedReloadFailed.
  ///
  /// In en, this message translates to:
  /// **'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.'**
  String get managedVoiceTakeRemoveSavedReloadFailed;

  /// No description provided for @managedVoiceTakeRemoveFailed.
  ///
  /// In en, this message translates to:
  /// **'The take was not removed: {error}'**
  String managedVoiceTakeRemoveFailed(String error);

  /// No description provided for @managedVoiceTakeRemoveReloadConfirmed.
  ///
  /// In en, this message translates to:
  /// **'The saved removal was confirmed from the latest project.'**
  String get managedVoiceTakeRemoveReloadConfirmed;

  /// No description provided for @managedVoiceSlotRemoveAction.
  ///
  /// In en, this message translates to:
  /// **'Remove empty Voice setup…'**
  String get managedVoiceSlotRemoveAction;

  /// No description provided for @managedVoiceSlotRemoveDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove empty Voice setup?'**
  String get managedVoiceSlotRemoveDialogTitle;

  /// No description provided for @managedVoiceSlotRemoveDialogSummary.
  ///
  /// In en, this message translates to:
  /// **'Remove the empty {locale} Voice setup from {line}?'**
  String managedVoiceSlotRemoveDialogSummary(String line, String locale);

  /// No description provided for @managedVoiceSlotRemoveRetention.
  ///
  /// In en, this message translates to:
  /// **'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.'**
  String get managedVoiceSlotRemoveRetention;

  /// No description provided for @managedVoiceSlotRemoveTargetWarning.
  ///
  /// In en, this message translates to:
  /// **'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.'**
  String get managedVoiceSlotRemoveTargetWarning;

  /// No description provided for @managedVoiceSlotRemoveRecreate.
  ///
  /// In en, this message translates to:
  /// **'You can add a new take later; the required Voice setup will then be created again automatically.'**
  String get managedVoiceSlotRemoveRecreate;

  /// No description provided for @managedVoiceSlotRemoveCancel.
  ///
  /// In en, this message translates to:
  /// **'Keep setup'**
  String get managedVoiceSlotRemoveCancel;

  /// No description provided for @managedVoiceSlotRemoveConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove setup'**
  String get managedVoiceSlotRemoveConfirm;

  /// No description provided for @managedVoiceSlotRemoveSuccess.
  ///
  /// In en, this message translates to:
  /// **'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.'**
  String get managedVoiceSlotRemoveSuccess;

  /// No description provided for @managedVoiceSlotRemoveStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.'**
  String get managedVoiceSlotRemoveStale;

  /// No description provided for @managedVoiceSlotRemoveRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'Reopen the managed project before removing this Voice setup.'**
  String get managedVoiceSlotRemoveRequiresReopen;

  /// No description provided for @managedVoiceSlotRemoveSavedUnconfirmed.
  ///
  /// In en, this message translates to:
  /// **'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.'**
  String get managedVoiceSlotRemoveSavedUnconfirmed;

  /// No description provided for @managedVoiceSlotRemoveSavedReloadFailed.
  ///
  /// In en, this message translates to:
  /// **'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.'**
  String get managedVoiceSlotRemoveSavedReloadFailed;

  /// No description provided for @managedVoiceSlotRemoveFailed.
  ///
  /// In en, this message translates to:
  /// **'The empty Voice setup could not be removed: {error}'**
  String managedVoiceSlotRemoveFailed(String error);

  /// No description provided for @managedVoiceSlotRemoveReloadConfirmed.
  ///
  /// In en, this message translates to:
  /// **'Saved empty Voice setup removal confirmed from the latest project.'**
  String get managedVoiceSlotRemoveReloadConfirmed;

  /// No description provided for @managedVoicePreviewTooltip.
  ///
  /// In en, this message translates to:
  /// **'Preview selected local Ogg'**
  String get managedVoicePreviewTooltip;

  /// No description provided for @managedVoicePreviewOpened.
  ///
  /// In en, this message translates to:
  /// **'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.'**
  String get managedVoicePreviewOpened;

  /// No description provided for @managedVoicePreviewFailed.
  ///
  /// In en, this message translates to:
  /// **'The local recording preview could not be opened: {error}'**
  String managedVoicePreviewFailed(String error);

  /// No description provided for @managedStoryWorkbenchEditNpcProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit name & archetype'**
  String get managedStoryWorkbenchEditNpcProfile;

  /// No description provided for @managedStoryWorkbenchNpcDisplayNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Character name'**
  String get managedStoryWorkbenchNpcDisplayNameLabel;

  /// No description provided for @managedNpcProfileEditTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit name & archetype'**
  String get managedNpcProfileEditTitle;

  /// No description provided for @managedNpcProfileEditDescription.
  ///
  /// In en, this message translates to:
  /// **'Change the friendly character name or choose another verified structural starting point.'**
  String get managedNpcProfileEditDescription;

  /// No description provided for @managedNpcProfileEditNameLabel.
  ///
  /// In en, this message translates to:
  /// **'Character name'**
  String get managedNpcProfileEditNameLabel;

  /// No description provided for @managedNpcProfileEditNameHint.
  ///
  /// In en, this message translates to:
  /// **'Shown to authors in this project.'**
  String get managedNpcProfileEditNameHint;

  /// No description provided for @managedNpcProfileEditArchetypeLabel.
  ///
  /// In en, this message translates to:
  /// **'Archetype / base character'**
  String get managedNpcProfileEditArchetypeLabel;

  /// No description provided for @managedNpcProfileEditArchetypeHelp.
  ///
  /// In en, this message translates to:
  /// **'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.'**
  String get managedNpcProfileEditArchetypeHelp;

  /// No description provided for @managedNpcProfileEditBoundary.
  ///
  /// In en, this message translates to:
  /// **'Only the offline project draft changes. The game installation and save games remain unchanged.'**
  String get managedNpcProfileEditBoundary;

  /// No description provided for @managedNpcProfileEditLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading current NPC details…'**
  String get managedNpcProfileEditLoading;

  /// No description provided for @managedNpcProfileEditCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get managedNpcProfileEditCancel;

  /// No description provided for @managedNpcProfileEditClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get managedNpcProfileEditClose;

  /// No description provided for @managedNpcProfileEditSave.
  ///
  /// In en, this message translates to:
  /// **'Save changes'**
  String get managedNpcProfileEditSave;

  /// No description provided for @managedNpcProfileEditSaving.
  ///
  /// In en, this message translates to:
  /// **'Saving…'**
  String get managedNpcProfileEditSaving;

  /// No description provided for @managedNpcProfileEditRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get managedNpcProfileEditRetry;

  /// No description provided for @managedNpcProfileEditLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'NPC details and verified archetypes could not be loaded. No files were changed.'**
  String get managedNpcProfileEditLoadFailed;

  /// No description provided for @managedNpcProfileEditCatalogChanged.
  ///
  /// In en, this message translates to:
  /// **'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.'**
  String get managedNpcProfileEditCatalogChanged;

  /// No description provided for @managedNpcProfileEditCurrentArchetypeUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.'**
  String get managedNpcProfileEditCurrentArchetypeUnavailable;

  /// No description provided for @managedNpcProfileEditStale.
  ///
  /// In en, this message translates to:
  /// **'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.'**
  String get managedNpcProfileEditStale;

  /// No description provided for @managedNpcProfileEditRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.'**
  String get managedNpcProfileEditRequiresReopen;

  /// No description provided for @managedNpcProfileEditSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.'**
  String get managedNpcProfileEditSaveFailed;

  /// No description provided for @managedNpcProfileEditNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a character name.'**
  String get managedNpcProfileEditNameRequired;

  /// No description provided for @managedNpcProfileEditNameTooLong.
  ///
  /// In en, this message translates to:
  /// **'The character name must be at most 256 UTF-8 bytes.'**
  String get managedNpcProfileEditNameTooLong;

  /// No description provided for @managedNpcProfileEditNameControl.
  ///
  /// In en, this message translates to:
  /// **'The character name contains an unsupported control character.'**
  String get managedNpcProfileEditNameControl;

  /// No description provided for @managedNpcProfileEditReviewSelection.
  ///
  /// In en, this message translates to:
  /// **'Review and choose an archetype before saving.'**
  String get managedNpcProfileEditReviewSelection;

  /// No description provided for @managedNpcProfileEditDiscardTitle.
  ///
  /// In en, this message translates to:
  /// **'Discard NPC changes?'**
  String get managedNpcProfileEditDiscardTitle;

  /// No description provided for @managedNpcProfileEditDiscardBody.
  ///
  /// In en, this message translates to:
  /// **'Your unsaved name and archetype choice will be lost.'**
  String get managedNpcProfileEditDiscardBody;

  /// No description provided for @managedNpcProfileEditKeepEditing.
  ///
  /// In en, this message translates to:
  /// **'Keep editing'**
  String get managedNpcProfileEditKeepEditing;

  /// No description provided for @managedNpcProfileEditDiscard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get managedNpcProfileEditDiscard;

  /// No description provided for @managedNpcProfileEditSaved.
  ///
  /// In en, this message translates to:
  /// **'{name} was saved in project revision {revision}. It remains an offline, build-blocked draft.'**
  String managedNpcProfileEditSaved(String name, int revision);

  /// No description provided for @managedVoiceBuildReadinessTitle.
  ///
  /// In en, this message translates to:
  /// **'Voice readiness'**
  String get managedVoiceBuildReadinessTitle;

  /// No description provided for @managedVoiceBuildReadinessRefresh.
  ///
  /// In en, this message translates to:
  /// **'Refresh Voice readiness'**
  String get managedVoiceBuildReadinessRefresh;

  /// No description provided for @managedVoiceBuildReadinessChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking exact Voice readiness'**
  String get managedVoiceBuildReadinessChecking;

  /// No description provided for @managedVoiceBuildReadinessLoadError.
  ///
  /// In en, this message translates to:
  /// **'Voice readiness could not be verified for the current project. No build is available from this result.'**
  String get managedVoiceBuildReadinessLoadError;

  /// No description provided for @managedVoiceBuildReadinessReadyTitle.
  ///
  /// In en, this message translates to:
  /// **'Voice is ready'**
  String get managedVoiceBuildReadinessReadyTitle;

  /// No description provided for @managedVoiceBuildReadinessBlockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Voice needs attention'**
  String get managedVoiceBuildReadinessBlockedTitle;

  /// No description provided for @managedVoiceBuildReadinessCount.
  ///
  /// In en, this message translates to:
  /// **'{readySlots} of {totalSlots} Voice slots are ready.'**
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots);

  /// No description provided for @managedVoiceBuildReadinessBlockedBoundary.
  ///
  /// In en, this message translates to:
  /// **'No bundle was created and deployment was not performed.'**
  String get managedVoiceBuildReadinessBlockedBoundary;

  /// No description provided for @managedVoiceBuildReadinessBuildBundle.
  ///
  /// In en, this message translates to:
  /// **'Build bundle'**
  String get managedVoiceBuildReadinessBuildBundle;

  /// No description provided for @managedVoiceBuildReadinessBuildReleaseGuidance.
  ///
  /// In en, this message translates to:
  /// **'Voice content is ready. Open Build & Release to create the offline bundle.'**
  String get managedVoiceBuildReadinessBuildReleaseGuidance;

  /// No description provided for @managedVoiceBuildReadinessConfigureGameGuidance.
  ///
  /// In en, this message translates to:
  /// **'Voice content is ready. Configure the game installation before creating an offline bundle.'**
  String get managedVoiceBuildReadinessConfigureGameGuidance;

  /// No description provided for @managedVoiceBuildReadinessHideBlockers.
  ///
  /// In en, this message translates to:
  /// **'Hide blockers'**
  String get managedVoiceBuildReadinessHideBlockers;

  /// No description provided for @managedVoiceBuildReadinessShowBlockers.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Show 1 blocker} other{Show {count} blockers}}'**
  String managedVoiceBuildReadinessShowBlockers(int count);

  /// No description provided for @managedVoiceBuildReadinessWorkflowFailed.
  ///
  /// In en, this message translates to:
  /// **'The selected Voice workflow could not be opened. Refresh and try again.'**
  String get managedVoiceBuildReadinessWorkflowFailed;

  /// No description provided for @managedVoiceBuildReadinessBuildWorkflowFailed.
  ///
  /// In en, this message translates to:
  /// **'The Voice build workflow could not be opened.'**
  String get managedVoiceBuildReadinessBuildWorkflowFailed;

  /// No description provided for @managedVoiceBuildReadinessExactRevision.
  ///
  /// In en, this message translates to:
  /// **'Exact project revision {revision}'**
  String managedVoiceBuildReadinessExactRevision(int revision);

  /// No description provided for @managedVoiceBuildReadinessResolveTarget.
  ///
  /// In en, this message translates to:
  /// **'Resolve target'**
  String get managedVoiceBuildReadinessResolveTarget;

  /// No description provided for @managedVoiceBuildReadinessManageTakes.
  ///
  /// In en, this message translates to:
  /// **'Manage takes'**
  String get managedVoiceBuildReadinessManageTakes;

  /// No description provided for @managedVoiceBuildBlockerNoSlots.
  ///
  /// In en, this message translates to:
  /// **'No Voice setups exist in this project.'**
  String get managedVoiceBuildBlockerNoSlots;

  /// No description provided for @managedVoiceBuildBlockerPayloadBudget.
  ///
  /// In en, this message translates to:
  /// **'The selected Voice recordings exceed the safe bundle memory budget.'**
  String get managedVoiceBuildBlockerPayloadBudget;

  /// No description provided for @managedVoiceBuildBlockerUnresolvedTarget.
  ///
  /// In en, this message translates to:
  /// **'Resolve this Voice target.'**
  String get managedVoiceBuildBlockerUnresolvedTarget;

  /// No description provided for @managedVoiceBuildBlockerAmbiguousTarget.
  ///
  /// In en, this message translates to:
  /// **'This Voice target is ambiguous.'**
  String get managedVoiceBuildBlockerAmbiguousTarget;

  /// No description provided for @managedVoiceBuildBlockerUnqualifiedAdd.
  ///
  /// In en, this message translates to:
  /// **'This target is not a sealed existing-member replacement.'**
  String get managedVoiceBuildBlockerUnqualifiedAdd;

  /// No description provided for @managedVoiceBuildBlockerMissingTake.
  ///
  /// In en, this message translates to:
  /// **'Select an approved Voice take.'**
  String get managedVoiceBuildBlockerMissingTake;

  /// No description provided for @managedVoiceBuildBlockerTakeNotApproved.
  ///
  /// In en, this message translates to:
  /// **'The selected Voice take is not approved.'**
  String get managedVoiceBuildBlockerTakeNotApproved;

  /// No description provided for @managedVoiceBuildBlockerCodecUnqualified.
  ///
  /// In en, this message translates to:
  /// **'The selected Voice take uses an unsupported codec.'**
  String get managedVoiceBuildBlockerCodecUnqualified;

  /// No description provided for @managedVoiceBuildBlockerSlotLimit.
  ///
  /// In en, this message translates to:
  /// **'This project exceeds the 1024-slot Voice bundle limit.'**
  String get managedVoiceBuildBlockerSlotLimit;

  /// No description provided for @managedVoiceBuildOfflineNotice.
  ///
  /// In en, this message translates to:
  /// **'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.'**
  String get managedVoiceBuildOfflineNotice;

  /// No description provided for @managedVoiceBuildNewFolderName.
  ///
  /// In en, this message translates to:
  /// **'New folder name'**
  String get managedVoiceBuildNewFolderName;

  /// No description provided for @managedVoiceBuildNewFolderHelp.
  ///
  /// In en, this message translates to:
  /// **'The bundle must be written to a brand-new child folder.'**
  String get managedVoiceBuildNewFolderHelp;

  /// No description provided for @managedVoiceBuildChooseParent.
  ///
  /// In en, this message translates to:
  /// **'Choose parent folder'**
  String get managedVoiceBuildChooseParent;

  /// No description provided for @managedVoiceBuildNoParentSelected.
  ///
  /// In en, this message translates to:
  /// **'No parent folder selected'**
  String get managedVoiceBuildNoParentSelected;

  /// No description provided for @managedVoiceBuildNewOutput.
  ///
  /// In en, this message translates to:
  /// **'New output'**
  String get managedVoiceBuildNewOutput;

  /// No description provided for @managedVoiceBuildOfflineBundle.
  ///
  /// In en, this message translates to:
  /// **'Build offline bundle'**
  String get managedVoiceBuildOfflineBundle;

  /// No description provided for @managedVoiceBuildParentInspectFailed.
  ///
  /// In en, this message translates to:
  /// **'The parent folder could not be inspected safely. No build or deployment was attempted.'**
  String get managedVoiceBuildParentInspectFailed;

  /// No description provided for @managedVoiceBuildChooseExistingParent.
  ///
  /// In en, this message translates to:
  /// **'Choose an existing parent folder.'**
  String get managedVoiceBuildChooseExistingParent;

  /// No description provided for @managedVoiceBuildTargetSymlink.
  ///
  /// In en, this message translates to:
  /// **'The target path is a symlink. Choose a different new folder name.'**
  String get managedVoiceBuildTargetSymlink;

  /// No description provided for @managedVoiceBuildTargetExists.
  ///
  /// In en, this message translates to:
  /// **'The target already exists. Choose a different new folder name.'**
  String get managedVoiceBuildTargetExists;

  /// No description provided for @managedVoiceBuildRequiresReopen.
  ///
  /// In en, this message translates to:
  /// **'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.'**
  String get managedVoiceBuildRequiresReopen;

  /// No description provided for @managedVoiceBuildStaleCheckpoint.
  ///
  /// In en, this message translates to:
  /// **'The managed project changed while this window was open. Close this build window and open it again from the current project.'**
  String get managedVoiceBuildStaleCheckpoint;

  /// No description provided for @managedVoiceBuildFailed.
  ///
  /// In en, this message translates to:
  /// **'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.'**
  String get managedVoiceBuildFailed;

  /// No description provided for @managedVoiceBuildPlanFailed.
  ///
  /// In en, this message translates to:
  /// **'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.'**
  String get managedVoiceBuildPlanFailed;

  /// No description provided for @managedVoiceBuildParentAbsolute.
  ///
  /// In en, this message translates to:
  /// **'Choose an absolute existing parent folder.'**
  String get managedVoiceBuildParentAbsolute;

  /// No description provided for @managedVoiceBuildParentSymlink.
  ///
  /// In en, this message translates to:
  /// **'The selected parent is a symlink. Choose a real existing folder.'**
  String get managedVoiceBuildParentSymlink;

  /// No description provided for @managedVoiceBuildFolderRequired.
  ///
  /// In en, this message translates to:
  /// **'Enter a new folder name.'**
  String get managedVoiceBuildFolderRequired;

  /// No description provided for @managedVoiceBuildFolderWhitespace.
  ///
  /// In en, this message translates to:
  /// **'The folder name cannot start or end with whitespace.'**
  String get managedVoiceBuildFolderWhitespace;

  /// No description provided for @managedVoiceBuildFolderTooLong.
  ///
  /// In en, this message translates to:
  /// **'The folder name is too long.'**
  String get managedVoiceBuildFolderTooLong;

  /// No description provided for @managedVoiceBuildFolderPortable.
  ///
  /// In en, this message translates to:
  /// **'Use one portable folder name without separators or reserved characters.'**
  String get managedVoiceBuildFolderPortable;

  /// No description provided for @managedVoiceBuildFolderWindowsReserved.
  ///
  /// In en, this message translates to:
  /// **'That folder name is reserved by Windows.'**
  String get managedVoiceBuildFolderWindowsReserved;

  /// No description provided for @managedVoiceBuildExecutableUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.'**
  String get managedVoiceBuildExecutableUnavailable;

  /// No description provided for @managedVoiceBuildExecutableMismatch.
  ///
  /// In en, this message translates to:
  /// **'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.'**
  String get managedVoiceBuildExecutableMismatch;

  /// No description provided for @managedVoiceBuildGameUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.'**
  String get managedVoiceBuildGameUnavailable;

  /// No description provided for @managedVoiceBuildStoreGameAlias.
  ///
  /// In en, this message translates to:
  /// **'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.'**
  String get managedVoiceBuildStoreGameAlias;

  /// No description provided for @managedVoiceBuildGameOutputAlias.
  ///
  /// In en, this message translates to:
  /// **'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.'**
  String get managedVoiceBuildGameOutputAlias;

  /// No description provided for @managedVoiceBuildStoreOutputAlias.
  ///
  /// In en, this message translates to:
  /// **'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.'**
  String get managedVoiceBuildStoreOutputAlias;

  /// No description provided for @managedVoiceBuildOutputUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.'**
  String get managedVoiceBuildOutputUnavailable;

  /// No description provided for @managedVoiceBuildOutputFailed.
  ///
  /// In en, this message translates to:
  /// **'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.'**
  String get managedVoiceBuildOutputFailed;

  /// No description provided for @managedVoiceBuildPromotionFailed.
  ///
  /// In en, this message translates to:
  /// **'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.'**
  String get managedVoiceBuildPromotionFailed;

  /// No description provided for @managedVoiceBuildCleanupFailed.
  ///
  /// In en, this message translates to:
  /// **'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.'**
  String get managedVoiceBuildCleanupFailed;

  /// No description provided for @managedVoiceBuildPublicationUnconfirmed.
  ///
  /// In en, this message translates to:
  /// **'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.'**
  String get managedVoiceBuildPublicationUnconfirmed;

  /// No description provided for @managedVoiceBuildStoreRootChanged.
  ///
  /// In en, this message translates to:
  /// **'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.'**
  String get managedVoiceBuildStoreRootChanged;

  /// No description provided for @managedVoiceBuildGameRootChanged.
  ///
  /// In en, this message translates to:
  /// **'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.'**
  String get managedVoiceBuildGameRootChanged;

  /// No description provided for @managedVoiceBuildOutputRootChanged.
  ///
  /// In en, this message translates to:
  /// **'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.'**
  String get managedVoiceBuildOutputRootChanged;

  /// No description provided for @managedVoiceBuildVerifyFailed.
  ///
  /// In en, this message translates to:
  /// **'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.'**
  String get managedVoiceBuildVerifyFailed;

  /// No description provided for @managedVoiceBuildBundleInvalid.
  ///
  /// In en, this message translates to:
  /// **'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.'**
  String get managedVoiceBuildBundleInvalid;

  /// No description provided for @managedVoiceBuildInputInvalid.
  ///
  /// In en, this message translates to:
  /// **'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.'**
  String get managedVoiceBuildInputInvalid;

  /// No description provided for @managedVoiceBuildResponseLimit.
  ///
  /// In en, this message translates to:
  /// **'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.'**
  String get managedVoiceBuildResponseLimit;

  /// No description provided for @managedVoiceBuildBuiltTitle.
  ///
  /// In en, this message translates to:
  /// **'Sealed Voice bundle built'**
  String get managedVoiceBuildBuiltTitle;

  /// No description provided for @managedVoiceBuildOfflineReceipt.
  ///
  /// In en, this message translates to:
  /// **'Offline receipt only. Deployment was not performed.'**
  String get managedVoiceBuildOfflineReceipt;

  /// No description provided for @managedVoiceBuildBasisRevision.
  ///
  /// In en, this message translates to:
  /// **'Basis project revision'**
  String get managedVoiceBuildBasisRevision;

  /// No description provided for @managedVoiceBuildOutputLabel.
  ///
  /// In en, this message translates to:
  /// **'Output'**
  String get managedVoiceBuildOutputLabel;

  /// No description provided for @managedVoiceBuildArchiveEdits.
  ///
  /// In en, this message translates to:
  /// **'Archive edits'**
  String get managedVoiceBuildArchiveEdits;

  /// No description provided for @managedVoiceBuildBundleFiles.
  ///
  /// In en, this message translates to:
  /// **'Bundle files'**
  String get managedVoiceBuildBundleFiles;

  /// No description provided for @managedVoiceBuildSealedBytes.
  ///
  /// In en, this message translates to:
  /// **'Sealed bytes'**
  String get managedVoiceBuildSealedBytes;

  /// No description provided for @managedVoiceBuildBundleSha256.
  ///
  /// In en, this message translates to:
  /// **'Bundle SHA-256'**
  String get managedVoiceBuildBundleSha256;

  /// No description provided for @managedVoiceBuildParentPickerTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose Voice bundle parent'**
  String get managedVoiceBuildParentPickerTitle;

  /// No description provided for @managedVoiceBuildBuiltMessage.
  ///
  /// In en, this message translates to:
  /// **'Sealed Voice bundle built at {output}. Deployment was not performed.'**
  String managedVoiceBuildBuiltMessage(String output);

  /// No description provided for @managedVoiceBuildBlockedMessage.
  ///
  /// In en, this message translates to:
  /// **'Voice build blocked by {count} exact requirements. No bundle was created or deployed.'**
  String managedVoiceBuildBlockedMessage(int count);
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
