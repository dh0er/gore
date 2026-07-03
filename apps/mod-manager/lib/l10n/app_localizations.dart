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

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'gore-manager'**
  String get appTitle;

  /// No description provided for @tabMods.
  ///
  /// In en, this message translates to:
  /// **'Mods'**
  String get tabMods;

  /// No description provided for @tabSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get tabSettings;

  /// No description provided for @settingsGameExe.
  ///
  /// In en, this message translates to:
  /// **'Game executable'**
  String get settingsGameExe;

  /// No description provided for @settingsGameExePick.
  ///
  /// In en, this message translates to:
  /// **'Choose…'**
  String get settingsGameExePick;

  /// No description provided for @settingsLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsLanguage;

  /// No description provided for @statusInSync.
  ///
  /// In en, this message translates to:
  /// **'In sync'**
  String get statusInSync;

  /// No description provided for @statusChangesPending.
  ///
  /// In en, this message translates to:
  /// **'Changes pending'**
  String get statusChangesPending;

  /// No description provided for @statusGameUpdated.
  ///
  /// In en, this message translates to:
  /// **'Game updated'**
  String get statusGameUpdated;

  /// No description provided for @statusStudioDeploy.
  ///
  /// In en, this message translates to:
  /// **'Studio deployment active'**
  String get statusStudioDeploy;

  /// No description provided for @statusNothingDeployed.
  ///
  /// In en, this message translates to:
  /// **'Nothing deployed'**
  String get statusNothingDeployed;

  /// No description provided for @actionImport.
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get actionImport;

  /// No description provided for @actionApply.
  ///
  /// In en, this message translates to:
  /// **'Apply'**
  String get actionApply;

  /// No description provided for @actionUndeployAll.
  ///
  /// In en, this message translates to:
  /// **'Undeploy all'**
  String get actionUndeployAll;

  /// No description provided for @commonCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get commonCancel;

  /// No description provided for @commonOk.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get commonOk;

  /// No description provided for @importFolder.
  ///
  /// In en, this message translates to:
  /// **'Import folder…'**
  String get importFolder;

  /// No description provided for @importFile.
  ///
  /// In en, this message translates to:
  /// **'Import file…'**
  String get importFile;

  /// No description provided for @applyTooltip.
  ///
  /// In en, this message translates to:
  /// **'Apply the loadout to the game'**
  String get applyTooltip;

  /// No description provided for @undeployAllAction.
  ///
  /// In en, this message translates to:
  /// **'Undeploy all'**
  String get undeployAllAction;

  /// No description provided for @undeployAllConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove everything the manager deployed from the game?'**
  String get undeployAllConfirm;

  /// No description provided for @takeOverTitle.
  ///
  /// In en, this message translates to:
  /// **'Studio deployment active'**
  String get takeOverTitle;

  /// No description provided for @takeOverBody.
  ///
  /// In en, this message translates to:
  /// **'mod-studio has deployed a mod to the game. Take over so the manager can apply this loadout?'**
  String get takeOverBody;

  /// No description provided for @takeOverAction.
  ///
  /// In en, this message translates to:
  /// **'Take over'**
  String get takeOverAction;

  /// No description provided for @refreshAction.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get refreshAction;

  /// No description provided for @conflictsTitle.
  ///
  /// In en, this message translates to:
  /// **'Conflicts ({count})'**
  String conflictsTitle(int count);

  /// No description provided for @conflictWinner.
  ///
  /// In en, this message translates to:
  /// **'winner'**
  String get conflictWinner;

  /// No description provided for @componentsTitle.
  ///
  /// In en, this message translates to:
  /// **'Components'**
  String get componentsTitle;

  /// No description provided for @targetsMore.
  ///
  /// In en, this message translates to:
  /// **'+{count} more'**
  String targetsMore(int count);

  /// No description provided for @removeModAction.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get removeModAction;

  /// No description provided for @removeModConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove “{name}” from the library?'**
  String removeModConfirm(String name);

  /// No description provided for @errorSetGamePath.
  ///
  /// In en, this message translates to:
  /// **'Set the game path in Settings first.'**
  String get errorSetGamePath;

  /// No description provided for @applyReportApplied.
  ///
  /// In en, this message translates to:
  /// **'Applied {count} mods.'**
  String applyReportApplied(int count);

  /// No description provided for @warningsTitle.
  ///
  /// In en, this message translates to:
  /// **'Warnings'**
  String get warningsTitle;

  /// No description provided for @modDisabledHint.
  ///
  /// In en, this message translates to:
  /// **'Disabled'**
  String get modDisabledHint;

  /// No description provided for @kindGoremod.
  ///
  /// In en, this message translates to:
  /// **'goremod'**
  String get kindGoremod;

  /// No description provided for @kindTriplet.
  ///
  /// In en, this message translates to:
  /// **'triplet'**
  String get kindTriplet;

  /// No description provided for @kindPak.
  ///
  /// In en, this message translates to:
  /// **'pak'**
  String get kindPak;

  /// No description provided for @kindUe4ss.
  ///
  /// In en, this message translates to:
  /// **'UE4SS'**
  String get kindUe4ss;

  /// No description provided for @kindRawfile.
  ///
  /// In en, this message translates to:
  /// **'raw file'**
  String get kindRawfile;

  /// No description provided for @kindMixed.
  ///
  /// In en, this message translates to:
  /// **'mixed'**
  String get kindMixed;

  /// No description provided for @sevHard.
  ///
  /// In en, this message translates to:
  /// **'hard'**
  String get sevHard;

  /// No description provided for @sevSoft.
  ///
  /// In en, this message translates to:
  /// **'soft'**
  String get sevSoft;

  /// No description provided for @sevInfo.
  ///
  /// In en, this message translates to:
  /// **'info'**
  String get sevInfo;
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
