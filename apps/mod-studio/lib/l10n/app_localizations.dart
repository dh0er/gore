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
