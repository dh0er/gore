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

  /// No description provided for @coreBlockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Mod Manager can\'t start'**
  String get coreBlockedTitle;

  /// No description provided for @coreDllMissingMessage.
  ///
  /// In en, this message translates to:
  /// **'A required program file is missing (gore_ffi.dll).'**
  String get coreDllMissingMessage;

  /// No description provided for @coreDllLoadFailedMessage.
  ///
  /// In en, this message translates to:
  /// **'A required program file could not be loaded.'**
  String get coreDllLoadFailedMessage;

  /// No description provided for @coreVerificationFailedMessage.
  ///
  /// In en, this message translates to:
  /// **'A required program file could not be verified.'**
  String get coreVerificationFailedMessage;

  /// No description provided for @coreManagerTooOldMessage.
  ///
  /// In en, this message translates to:
  /// **'The program files are newer than the Mod Manager. Update the Mod Manager.'**
  String get coreManagerTooOldMessage;

  /// No description provided for @coreNativeTooOldMessage.
  ///
  /// In en, this message translates to:
  /// **'The program files are older than the Mod Manager. Reinstall the Mod Manager.'**
  String get coreNativeTooOldMessage;

  /// No description provided for @coreCommandsMissingMessage.
  ///
  /// In en, this message translates to:
  /// **'The program files are missing features this Mod Manager needs.'**
  String get coreCommandsMissingMessage;

  /// No description provided for @coreBlockedRepairHint.
  ///
  /// In en, this message translates to:
  /// **'Reinstall or repair the Mod Manager, then start it again.'**
  String get coreBlockedRepairHint;

  /// No description provided for @coreTechnicalDetails.
  ///
  /// In en, this message translates to:
  /// **'Technical details'**
  String get coreTechnicalDetails;

  /// No description provided for @coreCopyTechnicalDetails.
  ///
  /// In en, this message translates to:
  /// **'Copy technical details'**
  String get coreCopyTechnicalDetails;

  /// No description provided for @coreTechnicalDetailsCopied.
  ///
  /// In en, this message translates to:
  /// **'Technical details copied'**
  String get coreTechnicalDetailsCopied;

  /// No description provided for @coreTechnicalDetailsCopyFailed.
  ///
  /// In en, this message translates to:
  /// **'Technical details could not be copied. Try again.'**
  String get coreTechnicalDetailsCopyFailed;

  /// No description provided for @preflightAttention.
  ///
  /// In en, this message translates to:
  /// **'Something needs your attention before mods can change.'**
  String get preflightAttention;

  /// No description provided for @preflightGameRunning.
  ///
  /// In en, this message translates to:
  /// **'Gothic is still running. Close the game before changing mods.'**
  String get preflightGameRunning;

  /// No description provided for @managerOperationFailed.
  ///
  /// In en, this message translates to:
  /// **'The operation failed.'**
  String get managerOperationFailed;

  /// No description provided for @libraryOperationFailed.
  ///
  /// In en, this message translates to:
  /// **'The mod list could not be loaded.'**
  String get libraryOperationFailed;

  /// No description provided for @conflictsUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Conflicts could not be checked.'**
  String get conflictsUnavailable;

  /// No description provided for @applyReportAppliedWithWarnings.
  ///
  /// In en, this message translates to:
  /// **'Applied: {applied}. Warnings: {warnings}.'**
  String applyReportAppliedWithWarnings(int applied, int warnings);

  /// No description provided for @modDetailKind.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get modDetailKind;

  /// No description provided for @modDetailVersion.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get modDetailVersion;

  /// No description provided for @modDetailAuthor.
  ///
  /// In en, this message translates to:
  /// **'Author'**
  String get modDetailAuthor;

  /// No description provided for @modDetailSource.
  ///
  /// In en, this message translates to:
  /// **'Source'**
  String get modDetailSource;

  /// No description provided for @modDetailImported.
  ///
  /// In en, this message translates to:
  /// **'Imported'**
  String get modDetailImported;

  /// No description provided for @componentLocalization.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get componentLocalization;

  /// No description provided for @componentAudio.
  ///
  /// In en, this message translates to:
  /// **'Sound'**
  String get componentAudio;

  /// No description provided for @componentAngelScript.
  ///
  /// In en, this message translates to:
  /// **'Scripts'**
  String get componentAngelScript;

  /// No description provided for @componentTexture.
  ///
  /// In en, this message translates to:
  /// **'Textures'**
  String get componentTexture;

  /// No description provided for @componentGameFiles.
  ///
  /// In en, this message translates to:
  /// **'Game files'**
  String get componentGameFiles;

  /// No description provided for @componentVoice.
  ///
  /// In en, this message translates to:
  /// **'Voice'**
  String get componentVoice;

  /// No description provided for @componentKindLocalizationPatch.
  ///
  /// In en, this message translates to:
  /// **'Text changes'**
  String get componentKindLocalizationPatch;

  /// No description provided for @componentKindAudioPatch.
  ///
  /// In en, this message translates to:
  /// **'Sound changes'**
  String get componentKindAudioPatch;

  /// No description provided for @componentKindAngelScriptPatch.
  ///
  /// In en, this message translates to:
  /// **'Script changes'**
  String get componentKindAngelScriptPatch;

  /// No description provided for @componentKindTexturePatch.
  ///
  /// In en, this message translates to:
  /// **'Texture changes'**
  String get componentKindTexturePatch;

  /// No description provided for @componentKindLoosePak.
  ///
  /// In en, this message translates to:
  /// **'PAK file'**
  String get componentKindLoosePak;

  /// No description provided for @componentKindTriplet.
  ///
  /// In en, this message translates to:
  /// **'IoStore container'**
  String get componentKindTriplet;

  /// No description provided for @componentKindUe4ssLua.
  ///
  /// In en, this message translates to:
  /// **'UE4SS script'**
  String get componentKindUe4ssLua;

  /// No description provided for @componentKindRawFile.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get componentKindRawFile;

  /// No description provided for @componentKindFilePatch.
  ///
  /// In en, this message translates to:
  /// **'Replaced game file'**
  String get componentKindFilePatch;

  /// No description provided for @componentKindPakFilePatch.
  ///
  /// In en, this message translates to:
  /// **'Game file from a ~mods PAK'**
  String get componentKindPakFilePatch;

  /// No description provided for @componentKindVoiceArchivePatch.
  ///
  /// In en, this message translates to:
  /// **'Voice lines'**
  String get componentKindVoiceArchivePatch;

  /// No description provided for @rawTargetGameText.
  ///
  /// In en, this message translates to:
  /// **'All game text'**
  String get rawTargetGameText;

  /// No description provided for @rawTargetGameScripts.
  ///
  /// In en, this message translates to:
  /// **'All game scripts'**
  String get rawTargetGameScripts;

  /// No description provided for @rawTargetSoundBank.
  ///
  /// In en, this message translates to:
  /// **'Sound bank'**
  String get rawTargetSoundBank;

  /// No description provided for @rawTargetSoundBankNamed.
  ///
  /// In en, this message translates to:
  /// **'Sound bank: {name}'**
  String rawTargetSoundBankNamed(String name);

  /// No description provided for @conflictKindLocalization.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get conflictKindLocalization;

  /// No description provided for @conflictKindAudio.
  ///
  /// In en, this message translates to:
  /// **'Sound'**
  String get conflictKindAudio;

  /// No description provided for @conflictKindAsset.
  ///
  /// In en, this message translates to:
  /// **'Game data'**
  String get conflictKindAsset;

  /// No description provided for @conflictKindCdo.
  ///
  /// In en, this message translates to:
  /// **'Object values'**
  String get conflictKindCdo;

  /// No description provided for @conflictKindUe4ssUnknown.
  ///
  /// In en, this message translates to:
  /// **'UE4SS (unclear)'**
  String get conflictKindUe4ssUnknown;

  /// No description provided for @conflictKindScriptModule.
  ///
  /// In en, this message translates to:
  /// **'Game script'**
  String get conflictKindScriptModule;

  /// No description provided for @conflictKindVoiceArchive.
  ///
  /// In en, this message translates to:
  /// **'Voice lines'**
  String get conflictKindVoiceArchive;

  /// No description provided for @conflictKindRawFile.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get conflictKindRawFile;

  /// No description provided for @conflictKindLooseFile.
  ///
  /// In en, this message translates to:
  /// **'Game file'**
  String get conflictKindLooseFile;

  /// No description provided for @preflightUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The game installation could not be checked.'**
  String get preflightUnavailable;

  /// No description provided for @preflightRetry.
  ///
  /// In en, this message translates to:
  /// **'Check again'**
  String get preflightRetry;

  /// No description provided for @preflightReviewStatus.
  ///
  /// In en, this message translates to:
  /// **'Show status'**
  String get preflightReviewStatus;

  /// No description provided for @preflightReviewRecovery.
  ///
  /// In en, this message translates to:
  /// **'Show help'**
  String get preflightReviewRecovery;

  /// No description provided for @installRecoveryTitle.
  ///
  /// In en, this message translates to:
  /// **'Interrupted installation'**
  String get installRecoveryTitle;

  /// No description provided for @installRecoveryBody.
  ///
  /// In en, this message translates to:
  /// **'GORE found leftover data from an installation or a script build. That job may still be running, or it ended and left this behind. GORE cannot clean it up safely on its own.'**
  String get installRecoveryBody;

  /// No description provided for @installRecoverySteps.
  ///
  /// In en, this message translates to:
  /// **'If the job is still running, wait for it to finish — do not stop it and do not delete any files. Once you are sure nothing is running, follow README.txt in the folder below and check again. If no folder is listed or you are unsure, leave everything as it is and ask for help.'**
  String get installRecoverySteps;

  /// No description provided for @installRecoveryEvidence.
  ///
  /// In en, this message translates to:
  /// **'What GORE found'**
  String get installRecoveryEvidence;

  /// No description provided for @managerRecoveryTitle.
  ///
  /// In en, this message translates to:
  /// **'Repair interrupted change'**
  String get managerRecoveryTitle;

  /// No description provided for @managerRecoveryConfirm.
  ///
  /// In en, this message translates to:
  /// **'GORE found an interrupted change and can put the game back into a known state. Your savegames are never touched.'**
  String get managerRecoveryConfirm;

  /// No description provided for @managerRecoveryAlreadyClean.
  ///
  /// In en, this message translates to:
  /// **'Nothing left to repair. The status was checked again.'**
  String get managerRecoveryAlreadyClean;

  /// No description provided for @managerRecoveryBusy.
  ///
  /// In en, this message translates to:
  /// **'The job is running again. Nothing was changed — wait for it to finish.'**
  String get managerRecoveryBusy;

  /// No description provided for @managerRecoveryLockCleared.
  ///
  /// In en, this message translates to:
  /// **'The interrupted job had not changed anything yet. It was cleaned up.'**
  String get managerRecoveryLockCleared;

  /// No description provided for @managerRecoveryRestoredPristine.
  ///
  /// In en, this message translates to:
  /// **'The change was rolled back. The game is back to its earlier state.'**
  String get managerRecoveryRestoredPristine;

  /// No description provided for @managerRecoveryApplyPreserved.
  ///
  /// In en, this message translates to:
  /// **'Apply had already finished. Nothing was lost.'**
  String get managerRecoveryApplyPreserved;

  /// No description provided for @managerRecoveryUndeployConfirmed.
  ///
  /// In en, this message translates to:
  /// **'The removal had already finished. Leftovers were cleaned up.'**
  String get managerRecoveryUndeployConfirmed;

  /// No description provided for @managerRecoveryCompileRequired.
  ///
  /// In en, this message translates to:
  /// **'This belongs to a script build, so nothing was changed. Open the repair help.'**
  String get managerRecoveryCompileRequired;

  /// No description provided for @managerRecoveryInspectionFailed.
  ///
  /// In en, this message translates to:
  /// **'GORE could not check the interrupted job safely. Nothing was changed.'**
  String get managerRecoveryInspectionFailed;

  /// No description provided for @managerRecoveryFailed.
  ///
  /// In en, this message translates to:
  /// **'The repair could not be finished. Check the status before trying again.'**
  String get managerRecoveryFailed;

  /// Deployment status is unavailable or uses an unsupported future state.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get statusUnknown;

  /// No description provided for @statusDetailsTitle.
  ///
  /// In en, this message translates to:
  /// **'Status: {status}'**
  String statusDetailsTitle(String status);

  /// No description provided for @statusDetailsOpen.
  ///
  /// In en, this message translates to:
  /// **'Show details: {status}'**
  String statusDetailsOpen(String status);

  /// No description provided for @statusDetailsNoRoot.
  ///
  /// In en, this message translates to:
  /// **'Choose your Gothic installation in Settings first.'**
  String get statusDetailsNoRoot;

  /// No description provided for @statusDetailsNoDeployment.
  ///
  /// In en, this message translates to:
  /// **'No mods are installed in the game right now.'**
  String get statusDetailsNoDeployment;

  /// No description provided for @statusDetailsInSyncDescription.
  ///
  /// In en, this message translates to:
  /// **'The game has exactly the mods you ticked here.'**
  String get statusDetailsInSyncDescription;

  /// No description provided for @statusDetailsDeployedLoadout.
  ///
  /// In en, this message translates to:
  /// **'Mods in the game'**
  String get statusDetailsDeployedLoadout;

  /// No description provided for @statusDetailsChangesDescription.
  ///
  /// In en, this message translates to:
  /// **'Your selection differs from what is in the game.'**
  String get statusDetailsChangesDescription;

  /// No description provided for @statusDetailsCurrentlyDeployed.
  ///
  /// In en, this message translates to:
  /// **'In the game now'**
  String get statusDetailsCurrentlyDeployed;

  /// No description provided for @statusDetailsAfterApply.
  ///
  /// In en, this message translates to:
  /// **'After Apply'**
  String get statusDetailsAfterApply;

  /// No description provided for @statusDetailsGameUpdatedDescription.
  ///
  /// In en, this message translates to:
  /// **'The game was updated and overwrote mod files. Apply again to put them back.'**
  String get statusDetailsGameUpdatedDescription;

  /// No description provided for @statusDetailsDriftedFiles.
  ///
  /// In en, this message translates to:
  /// **'Affected files'**
  String get statusDetailsDriftedFiles;

  /// No description provided for @statusDetailsStudioDescription.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio currently has mods in this game. Take the game over before the Manager applies yours.'**
  String get statusDetailsStudioDescription;

  /// No description provided for @statusDetailsStudioMod.
  ///
  /// In en, this message translates to:
  /// **'Studio mod: {name}'**
  String statusDetailsStudioMod(String name);

  /// No description provided for @statusDetailsStudioNameUnknown.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio did not report a name.'**
  String get statusDetailsStudioNameUnknown;

  /// No description provided for @statusDetailsRecoveryDescription.
  ///
  /// In en, this message translates to:
  /// **'A change was interrupted. Repair it before changing mods.'**
  String get statusDetailsRecoveryDescription;

  /// No description provided for @statusDetailsUnknownDescription.
  ///
  /// In en, this message translates to:
  /// **'The status could not be read. Refresh first.'**
  String get statusDetailsUnknownDescription;

  /// No description provided for @statusDetailsUnavailable.
  ///
  /// In en, this message translates to:
  /// **'No details available.'**
  String get statusDetailsUnavailable;

  /// No description provided for @statusDetailsEmptyLoadout.
  ///
  /// In en, this message translates to:
  /// **'No mods.'**
  String get statusDetailsEmptyLoadout;

  /// No description provided for @statusDetailsLastError.
  ///
  /// In en, this message translates to:
  /// **'Last error'**
  String get statusDetailsLastError;

  /// No description provided for @statusDetailsLastApply.
  ///
  /// In en, this message translates to:
  /// **'Last Apply'**
  String get statusDetailsLastApply;

  /// No description provided for @statusDetailsAppliedMods.
  ///
  /// In en, this message translates to:
  /// **'Applied mods'**
  String get statusDetailsAppliedMods;

  /// No description provided for @statusDetailsWarnings.
  ///
  /// In en, this message translates to:
  /// **'Warnings'**
  String get statusDetailsWarnings;

  /// No description provided for @statusDetailsReapply.
  ///
  /// In en, this message translates to:
  /// **'Reapply'**
  String get statusDetailsReapply;

  /// No description provided for @statusDetailsOpenSettings.
  ///
  /// In en, this message translates to:
  /// **'Open Settings'**
  String get statusDetailsOpenSettings;

  /// No description provided for @recoveryAction.
  ///
  /// In en, this message translates to:
  /// **'Repair'**
  String get recoveryAction;

  /// No description provided for @recoveryRequiredConfirm.
  ///
  /// In en, this message translates to:
  /// **'Repair the interrupted change and remove any half-installed files?'**
  String get recoveryRequiredConfirm;

  /// No description provided for @statusRecoveryRequired.
  ///
  /// In en, this message translates to:
  /// **'Repair needed'**
  String get statusRecoveryRequired;

  /// No description provided for @statusDetailsOwnershipTitle.
  ///
  /// In en, this message translates to:
  /// **'Files GORE manages'**
  String get statusDetailsOwnershipTitle;

  /// No description provided for @statusDetailsOwnershipDescription.
  ///
  /// In en, this message translates to:
  /// **'Recorded when mods were applied — not a check that the files still exist.'**
  String get statusDetailsOwnershipDescription;

  /// No description provided for @statusDetailsOwnershipLive.
  ///
  /// In en, this message translates to:
  /// **'Replaced game files'**
  String get statusDetailsOwnershipLive;

  /// No description provided for @statusDetailsOwnershipBackups.
  ///
  /// In en, this message translates to:
  /// **'Backups of the originals'**
  String get statusDetailsOwnershipBackups;

  /// No description provided for @statusDetailsOwnershipAdditive.
  ///
  /// In en, this message translates to:
  /// **'Added mod files'**
  String get statusDetailsOwnershipAdditive;

  /// No description provided for @statusDetailsOwnershipUe4ss.
  ///
  /// In en, this message translates to:
  /// **'UE4SS mod directories'**
  String get statusDetailsOwnershipUe4ss;

  /// No description provided for @statusDetailsOwnershipRecovery.
  ///
  /// In en, this message translates to:
  /// **'Repair files'**
  String get statusDetailsOwnershipRecovery;

  /// No description provided for @statusDetailsOwnershipEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nothing recorded here.'**
  String get statusDetailsOwnershipEmpty;

  /// No description provided for @statusDetailsOwnershipShown.
  ///
  /// In en, this message translates to:
  /// **'Showing {shown} of {total} paths.'**
  String statusDetailsOwnershipShown(int shown, int total);

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'GORE Mod Manager'**
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
  /// **'Gothic installation'**
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

  /// No description provided for @libraryEmptyTitle.
  ///
  /// In en, this message translates to:
  /// **'No mods yet'**
  String get libraryEmptyTitle;

  /// No description provided for @libraryEmptyBody.
  ///
  /// In en, this message translates to:
  /// **'Import a folder or a mod file to get started.'**
  String get libraryEmptyBody;

  /// No description provided for @detailEmptyHint.
  ///
  /// In en, this message translates to:
  /// **'Pick a mod to see what it changes.'**
  String get detailEmptyHint;

  /// No description provided for @settingsAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced details'**
  String get settingsAdvanced;

  /// No description provided for @settingsAdvancedHint.
  ///
  /// In en, this message translates to:
  /// **'Show the technical side: affected entries, how reliable the conflict check is, and the files GORE manages.'**
  String get settingsAdvancedHint;

  /// No description provided for @updatesTitle.
  ///
  /// In en, this message translates to:
  /// **'Updates'**
  String get updatesTitle;

  /// No description provided for @checkForUpdatesAutomatically.
  ///
  /// In en, this message translates to:
  /// **'Check for updates automatically'**
  String get checkForUpdatesAutomatically;

  /// No description provided for @checkForUpdatesNow.
  ///
  /// In en, this message translates to:
  /// **'Check for updates now'**
  String get checkForUpdatesNow;

  /// No description provided for @updatesPortableNotice.
  ///
  /// In en, this message translates to:
  /// **'The portable version opens the download page in your browser. Replace your existing files with the new download.'**
  String get updatesPortableNotice;

  /// No description provided for @updateCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not check for updates. Please try again later.'**
  String get updateCheckFailed;

  /// No description provided for @updateUpToDate.
  ///
  /// In en, this message translates to:
  /// **'You are using the latest version.'**
  String get updateUpToDate;

  /// No description provided for @updateAvailableTitle.
  ///
  /// In en, this message translates to:
  /// **'Update available'**
  String get updateAvailableTitle;

  /// No description provided for @updateAvailableMessage.
  ///
  /// In en, this message translates to:
  /// **'Version {version} is available. You have {current}.'**
  String updateAvailableMessage(String version, String current);

  /// No description provided for @updateLater.
  ///
  /// In en, this message translates to:
  /// **'Later'**
  String get updateLater;

  /// No description provided for @updateDownload.
  ///
  /// In en, this message translates to:
  /// **'Download'**
  String get updateDownload;

  /// No description provided for @updateOpenFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not open the download page. You can reach it at {url}'**
  String updateOpenFailed(String url);

  /// No description provided for @statusInSync.
  ///
  /// In en, this message translates to:
  /// **'Up to date'**
  String get statusInSync;

  /// No description provided for @statusChangesPending.
  ///
  /// In en, this message translates to:
  /// **'Not applied yet'**
  String get statusChangesPending;

  /// No description provided for @statusGameUpdated.
  ///
  /// In en, this message translates to:
  /// **'Game was updated'**
  String get statusGameUpdated;

  /// No description provided for @statusStudioDeploy.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio active'**
  String get statusStudioDeploy;

  /// No description provided for @statusNothingDeployed.
  ///
  /// In en, this message translates to:
  /// **'No mods in game'**
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

  /// No description provided for @actionStartGame.
  ///
  /// In en, this message translates to:
  /// **'Start game'**
  String get actionStartGame;

  /// No description provided for @startGameTooltip.
  ///
  /// In en, this message translates to:
  /// **'Launch Gothic with the mods currently in the game'**
  String get startGameTooltip;

  /// No description provided for @startGameFailed.
  ///
  /// In en, this message translates to:
  /// **'Gothic could not be started. Check the game installation in Settings.'**
  String get startGameFailed;

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

  /// No description provided for @importOutcomeCreated.
  ///
  /// In en, this message translates to:
  /// **'Added “{name}”.'**
  String importOutcomeCreated(String name);

  /// No description provided for @importOutcomeUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated “{name}”.'**
  String importOutcomeUpdated(String name);

  /// No description provided for @importOutcomeUnchanged.
  ///
  /// In en, this message translates to:
  /// **'“{name}” is already in your list.'**
  String importOutcomeUnchanged(String name);

  /// No description provided for @importOutcomeMatchedBy.
  ///
  /// In en, this message translates to:
  /// **'{method, select, none {No existing mod matched.} source {Matched by the same import source.} content {Matched by verified identical content.} entry_id {Matched by mod ID.} other {Match details are unavailable.}}'**
  String importOutcomeMatchedBy(String method);

  /// No description provided for @importRefusalDuplicateAmbiguous.
  ///
  /// In en, this message translates to:
  /// **'This matches more than one mod you already have. Remove the duplicates, then try again.'**
  String get importRefusalDuplicateAmbiguous;

  /// No description provided for @importRefusalIdentityConflict.
  ///
  /// In en, this message translates to:
  /// **'The source and the contents match two different mods you already have. Sort those out, then try again.'**
  String get importRefusalIdentityConflict;

  /// No description provided for @importFailed.
  ///
  /// In en, this message translates to:
  /// **'This could not be imported. Supported: folders, ZIP archives and single mod files (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extract .7z or .rar first, then import the folder. It may already have been added or updated — refresh your list before trying again.'**
  String get importFailed;

  /// No description provided for @importPickerFailed.
  ///
  /// In en, this message translates to:
  /// **'The file picker could not be opened. Nothing was imported.'**
  String get importPickerFailed;

  /// No description provided for @importOutcomeUnknown.
  ///
  /// In en, this message translates to:
  /// **'The result is unclear. Refresh to check your mod list.'**
  String get importOutcomeUnknown;

  /// No description provided for @applyTooltip.
  ///
  /// In en, this message translates to:
  /// **'Install the ticked mods into the game'**
  String get applyTooltip;

  /// No description provided for @undeployAllAction.
  ///
  /// In en, this message translates to:
  /// **'Remove all from game'**
  String get undeployAllAction;

  /// No description provided for @undeployAllConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove every mod the Manager installed from the game?'**
  String get undeployAllConfirm;

  /// No description provided for @takeOverTitle.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio is active'**
  String get takeOverTitle;

  /// No description provided for @takeOverBody.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio currently has a mod in the game. Take over so the Manager can apply your selection?'**
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

  /// Labels the mod expected to win by intended Manager load order, not a proven runtime outcome.
  ///
  /// In en, this message translates to:
  /// **'wins'**
  String get conflictWinner;

  /// No description provided for @noConflicts.
  ///
  /// In en, this message translates to:
  /// **'No conflicts found.'**
  String get noConflicts;

  /// No description provided for @conflictCoverageIncomplete.
  ///
  /// In en, this message translates to:
  /// **'Some mods can\'t be checked completely, so there may be more conflicts.'**
  String get conflictCoverageIncomplete;

  /// No description provided for @loadOrderDirection.
  ///
  /// In en, this message translates to:
  /// **'Mods further down the list override the ones above them.'**
  String get loadOrderDirection;

  /// No description provided for @footprintCoverageScope.
  ///
  /// In en, this message translates to:
  /// **'Only known conflict targets are listed. This is no guarantee of what happens in game.'**
  String get footprintCoverageScope;

  /// No description provided for @footprintTargetsExact.
  ///
  /// In en, this message translates to:
  /// **'Affected entries — the full list:'**
  String get footprintTargetsExact;

  /// No description provided for @footprintTargetsPartial.
  ///
  /// In en, this message translates to:
  /// **'Affected entries — there may be more:'**
  String get footprintTargetsPartial;

  /// No description provided for @footprintTargetsAdvisory.
  ///
  /// In en, this message translates to:
  /// **'Probably affected entries — hints, not proof:'**
  String get footprintTargetsAdvisory;

  /// No description provided for @footprintTargetsOpaque.
  ///
  /// In en, this message translates to:
  /// **'GORE cannot tell what this changes.'**
  String get footprintTargetsOpaque;

  /// No description provided for @conflictsUnverified.
  ///
  /// In en, this message translates to:
  /// **'Conflicts unknown — refresh first.'**
  String get conflictsUnverified;

  /// No description provided for @componentsTitle.
  ///
  /// In en, this message translates to:
  /// **'What this mod changes'**
  String get componentsTitle;

  /// No description provided for @targetsMore.
  ///
  /// In en, this message translates to:
  /// **'+{count} more'**
  String targetsMore(int count);

  /// No description provided for @removeModDeploymentHint.
  ///
  /// In en, this message translates to:
  /// **'This only removes it from your list. If it is installed in the game, choose Apply afterwards.'**
  String get removeModDeploymentHint;

  /// No description provided for @removeModSuccess.
  ///
  /// In en, this message translates to:
  /// **'Removed “{name}”.'**
  String removeModSuccess(String name);

  /// No description provided for @removeModFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not remove “{name}”.'**
  String removeModFailed(String name);

  /// No description provided for @removeModPartialFailure.
  ///
  /// In en, this message translates to:
  /// **'Removed “{name}”, but the list could not be fully updated.'**
  String removeModPartialFailure(String name);

  /// No description provided for @removeModOutcomeUnknown.
  ///
  /// In en, this message translates to:
  /// **'Could not confirm whether “{name}” was removed.'**
  String removeModOutcomeUnknown(String name);

  /// No description provided for @libraryStateUnknown.
  ///
  /// In en, this message translates to:
  /// **'The mod list is out of date. Refresh before changing or applying mods.'**
  String get libraryStateUnknown;

  /// No description provided for @removeModAction.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get removeModAction;

  /// No description provided for @removeModConfirm.
  ///
  /// In en, this message translates to:
  /// **'Remove “{name}” from your list?'**
  String removeModConfirm(String name);

  /// No description provided for @errorSetGamePath.
  ///
  /// In en, this message translates to:
  /// **'Choose your Gothic installation in Settings first.'**
  String get errorSetGamePath;

  /// No description provided for @applyReportApplied.
  ///
  /// In en, this message translates to:
  /// **'Applied {count} mods.'**
  String applyReportApplied(int count);

  /// No description provided for @modDisabledHint.
  ///
  /// In en, this message translates to:
  /// **'Disabled'**
  String get modDisabledHint;

  /// No description provided for @kindGoremod.
  ///
  /// In en, this message translates to:
  /// **'GORE bundle'**
  String get kindGoremod;

  /// No description provided for @kindTriplet.
  ///
  /// In en, this message translates to:
  /// **'IoStore mod'**
  String get kindTriplet;

  /// No description provided for @kindPak.
  ///
  /// In en, this message translates to:
  /// **'PAK mod'**
  String get kindPak;

  /// No description provided for @kindUe4ss.
  ///
  /// In en, this message translates to:
  /// **'UE4SS'**
  String get kindUe4ss;

  /// No description provided for @kindRawfile.
  ///
  /// In en, this message translates to:
  /// **'Whole-file replacement'**
  String get kindRawfile;

  /// No description provided for @kindMixed.
  ///
  /// In en, this message translates to:
  /// **'Mixed'**
  String get kindMixed;

  /// No description provided for @sevHard.
  ///
  /// In en, this message translates to:
  /// **'Conflict'**
  String get sevHard;

  /// No description provided for @sevSoft.
  ///
  /// In en, this message translates to:
  /// **'Warning'**
  String get sevSoft;

  /// No description provided for @sevInfo.
  ///
  /// In en, this message translates to:
  /// **'Note'**
  String get sevInfo;

  /// No description provided for @aboutVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version} ({sha})'**
  String aboutVersion(String version, String sha);

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @aboutCopyright.
  ///
  /// In en, this message translates to:
  /// **'© 2026 Daniel Hoer'**
  String get aboutCopyright;

  /// No description provided for @aboutLicense.
  ///
  /// In en, this message translates to:
  /// **'Licensed under the MIT License.'**
  String get aboutLicense;

  /// No description provided for @appearanceTitle.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get appearanceTitle;

  /// No description provided for @theme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get theme;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeDark;

  /// No description provided for @themeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeSystem;

  /// No description provided for @uiScale.
  ///
  /// In en, this message translates to:
  /// **'Display size'**
  String get uiScale;

  /// No description provided for @resetZoomTooltip.
  ///
  /// In en, this message translates to:
  /// **'Reset zoom (Ctrl+0)'**
  String get resetZoomTooltip;

  /// No description provided for @zoomTip.
  ///
  /// In en, this message translates to:
  /// **'Tip: Ctrl + / Ctrl - changes the zoom anywhere in the app.'**
  String get zoomTip;

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
