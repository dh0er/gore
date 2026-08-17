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
  /// **'Mod Manager unavailable'**
  String get coreBlockedTitle;

  /// No description provided for @coreDllMissingMessage.
  ///
  /// In en, this message translates to:
  /// **'The required gore_ffi.dll was not found.'**
  String get coreDllMissingMessage;

  /// No description provided for @coreDllLoadFailedMessage.
  ///
  /// In en, this message translates to:
  /// **'The native GORE Core library could not be loaded.'**
  String get coreDllLoadFailedMessage;

  /// No description provided for @coreVerificationFailedMessage.
  ///
  /// In en, this message translates to:
  /// **'The native GORE Core library could not be verified.'**
  String get coreVerificationFailedMessage;

  /// No description provided for @coreManagerTooOldMessage.
  ///
  /// In en, this message translates to:
  /// **'This GORE Core version is newer than the Mod Manager. Update the Mod Manager.'**
  String get coreManagerTooOldMessage;

  /// No description provided for @coreNativeTooOldMessage.
  ///
  /// In en, this message translates to:
  /// **'This GORE Core version is older than the Mod Manager. Update or repair the complete Mod Manager installation.'**
  String get coreNativeTooOldMessage;

  /// No description provided for @coreCommandsMissingMessage.
  ///
  /// In en, this message translates to:
  /// **'The GORE Core library does not provide all commands required by this Mod Manager.'**
  String get coreCommandsMissingMessage;

  /// No description provided for @coreBlockedRepairHint.
  ///
  /// In en, this message translates to:
  /// **'Update or repair the complete Mod Manager package, then restart the app.'**
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
  /// **'Setup needs attention.'**
  String get preflightAttention;

  /// No description provided for @preflightUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Setup diagnosis is unavailable.'**
  String get preflightUnavailable;

  /// No description provided for @preflightRetry.
  ///
  /// In en, this message translates to:
  /// **'Check again'**
  String get preflightRetry;

  /// No description provided for @preflightReviewStatus.
  ///
  /// In en, this message translates to:
  /// **'Review status'**
  String get preflightReviewStatus;

  /// No description provided for @preflightReviewRecovery.
  ///
  /// In en, this message translates to:
  /// **'Recovery help'**
  String get preflightReviewRecovery;

  /// No description provided for @installRecoveryTitle.
  ///
  /// In en, this message translates to:
  /// **'Installation recovery'**
  String get installRecoveryTitle;

  /// No description provided for @installRecoveryBody.
  ///
  /// In en, this message translates to:
  /// **'GORE found recovery data from an interrupted installation or script build. Automatic repair is unsafe because the previous operation and the original file state cannot be proven.'**
  String get installRecoveryBody;

  /// No description provided for @installRecoverySteps.
  ///
  /// In en, this message translates to:
  /// **'Close Gothic, Mod Studio, and other GORE tasks. Follow README.txt in the recovery folder listed below. If no folder is listed, leave the listed recovery data unchanged and get help instead of deleting anything. Never remove a lock while a task is running. Then check again.'**
  String get installRecoverySteps;

  /// No description provided for @installRecoveryEvidence.
  ///
  /// In en, this message translates to:
  /// **'Detected recovery data'**
  String get installRecoveryEvidence;

  /// Deployment status is unavailable or uses an unsupported future state.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get statusUnknown;

  /// No description provided for @statusDetailsTitle.
  ///
  /// In en, this message translates to:
  /// **'Deployment: {status}'**
  String statusDetailsTitle(String status);

  /// No description provided for @statusDetailsOpen.
  ///
  /// In en, this message translates to:
  /// **'Show deployment details: {status}'**
  String statusDetailsOpen(String status);

  /// No description provided for @statusDetailsNoRoot.
  ///
  /// In en, this message translates to:
  /// **'Choose a game installation in Settings to inspect its deployment status.'**
  String get statusDetailsNoRoot;

  /// No description provided for @statusDetailsNoDeployment.
  ///
  /// In en, this message translates to:
  /// **'No Manager deployment is installed for this game.'**
  String get statusDetailsNoDeployment;

  /// No description provided for @statusDetailsInSyncDescription.
  ///
  /// In en, this message translates to:
  /// **'The deployed mods match the current loadout.'**
  String get statusDetailsInSyncDescription;

  /// No description provided for @statusDetailsDeployedLoadout.
  ///
  /// In en, this message translates to:
  /// **'Deployed load order'**
  String get statusDetailsDeployedLoadout;

  /// No description provided for @statusDetailsChangesDescription.
  ///
  /// In en, this message translates to:
  /// **'The current deployment differs from what Apply will install.'**
  String get statusDetailsChangesDescription;

  /// No description provided for @statusDetailsCurrentlyDeployed.
  ///
  /// In en, this message translates to:
  /// **'Currently deployed'**
  String get statusDetailsCurrentlyDeployed;

  /// No description provided for @statusDetailsAfterApply.
  ///
  /// In en, this message translates to:
  /// **'After Apply'**
  String get statusDetailsAfterApply;

  /// No description provided for @statusDetailsGameUpdatedDescription.
  ///
  /// In en, this message translates to:
  /// **'Game files changed after the last deployment. Reapply the loadout to restore the Manager-owned files.'**
  String get statusDetailsGameUpdatedDescription;

  /// No description provided for @statusDetailsDriftedFiles.
  ///
  /// In en, this message translates to:
  /// **'Changed files'**
  String get statusDetailsDriftedFiles;

  /// No description provided for @statusDetailsStudioDescription.
  ///
  /// In en, this message translates to:
  /// **'Mod Studio currently owns this game installation. Take over before applying a Manager loadout.'**
  String get statusDetailsStudioDescription;

  /// No description provided for @statusDetailsStudioMod.
  ///
  /// In en, this message translates to:
  /// **'Studio mod: {name}'**
  String statusDetailsStudioMod(String name);

  /// No description provided for @statusDetailsStudioNameUnknown.
  ///
  /// In en, this message translates to:
  /// **'Studio did not report a mod name.'**
  String get statusDetailsStudioNameUnknown;

  /// No description provided for @statusDetailsRecoveryDescription.
  ///
  /// In en, this message translates to:
  /// **'A deployment was interrupted. Recover it before applying or removing Manager mods.'**
  String get statusDetailsRecoveryDescription;

  /// No description provided for @statusDetailsUnknownDescription.
  ///
  /// In en, this message translates to:
  /// **'Deployment status could not be verified. Refresh before applying mods.'**
  String get statusDetailsUnknownDescription;

  /// No description provided for @statusDetailsUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The installed core did not provide these details.'**
  String get statusDetailsUnavailable;

  /// No description provided for @statusDetailsEmptyLoadout.
  ///
  /// In en, this message translates to:
  /// **'No mods in this loadout.'**
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
  /// **'Recover'**
  String get recoveryAction;

  /// No description provided for @recoveryRequiredConfirm.
  ///
  /// In en, this message translates to:
  /// **'Recover the interrupted deployment and remove any partially deployed files?'**
  String get recoveryRequiredConfirm;

  /// No description provided for @statusRecoveryRequired.
  ///
  /// In en, this message translates to:
  /// **'Recovery required'**
  String get statusRecoveryRequired;

  /// No description provided for @statusDetailsOwnershipTitle.
  ///
  /// In en, this message translates to:
  /// **'Recorded ownership evidence'**
  String get statusDetailsOwnershipTitle;

  /// No description provided for @statusDetailsOwnershipDescription.
  ///
  /// In en, this message translates to:
  /// **'Paths recorded in the Manager deploy record. They do not prove that those paths still exist.'**
  String get statusDetailsOwnershipDescription;

  /// No description provided for @statusDetailsOwnershipLive.
  ///
  /// In en, this message translates to:
  /// **'Replaced game files'**
  String get statusDetailsOwnershipLive;

  /// No description provided for @statusDetailsOwnershipBackups.
  ///
  /// In en, this message translates to:
  /// **'Pristine backups'**
  String get statusDetailsOwnershipBackups;

  /// No description provided for @statusDetailsOwnershipAdditive.
  ///
  /// In en, this message translates to:
  /// **'Added pak and container files'**
  String get statusDetailsOwnershipAdditive;

  /// No description provided for @statusDetailsOwnershipUe4ss.
  ///
  /// In en, this message translates to:
  /// **'UE4SS mod directories'**
  String get statusDetailsOwnershipUe4ss;

  /// No description provided for @statusDetailsOwnershipRecovery.
  ///
  /// In en, this message translates to:
  /// **'Recovery files and holders'**
  String get statusDetailsOwnershipRecovery;

  /// No description provided for @statusDetailsOwnershipEmpty.
  ///
  /// In en, this message translates to:
  /// **'No paths recorded in this group.'**
  String get statusDetailsOwnershipEmpty;

  /// No description provided for @statusDetailsOwnershipShown.
  ///
  /// In en, this message translates to:
  /// **'{shown} of {total} recorded paths shown.'**
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

  /// No description provided for @importOutcomeCreated.
  ///
  /// In en, this message translates to:
  /// **'Added “{name}” to the library.'**
  String importOutcomeCreated(String name);

  /// No description provided for @importOutcomeUpdated.
  ///
  /// In en, this message translates to:
  /// **'Updated “{name}” in the library.'**
  String importOutcomeUpdated(String name);

  /// No description provided for @importOutcomeUnchanged.
  ///
  /// In en, this message translates to:
  /// **'“{name}” is already in the library.'**
  String importOutcomeUnchanged(String name);

  /// No description provided for @importOutcomeMatchedBy.
  ///
  /// In en, this message translates to:
  /// **'{method, select, none {No existing library entry was matched.} source {Matched by the same import source.} content {Matched by verified identical content.} entry_id {Matched by mod ID.} other {Match details are unavailable.}}'**
  String importOutcomeMatchedBy(String method);

  /// No description provided for @importRefusalDuplicateAmbiguous.
  ///
  /// In en, this message translates to:
  /// **'This import matches more than one library entry. Review or remove the duplicates, then try again.'**
  String get importRefusalDuplicateAmbiguous;

  /// No description provided for @importRefusalIdentityConflict.
  ///
  /// In en, this message translates to:
  /// **'The import source and its content match different library entries. Review or remove the conflicting entries, then try again.'**
  String get importRefusalIdentityConflict;

  /// No description provided for @importFailed.
  ///
  /// In en, this message translates to:
  /// **'The import could not be completed. Supported sources: folders, ZIP, loose *_P.pak files, complete .utoc/.ucas sets (optional .pak), .lcache, .bank, and PrecompiledScript*.Cache. Extract .7z or .rar first, then import the folder. The source may be unsupported, corrupt, or incomplete. The mod may already have been added or updated; refresh and check the library before trying again.'**
  String get importFailed;

  /// No description provided for @importPickerFailed.
  ///
  /// In en, this message translates to:
  /// **'The file or folder picker could not be opened. No import was started. Try again.'**
  String get importPickerFailed;

  /// No description provided for @importOutcomeUnknown.
  ///
  /// In en, this message translates to:
  /// **'The import result could not be verified. Choose Refresh to check the library.'**
  String get importOutcomeUnknown;

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
  /// **'Findings ({count})'**
  String conflictsTitle(int count);

  /// Labels the mod expected to win by intended Manager load order, not a proven runtime outcome.
  ///
  /// In en, this message translates to:
  /// **'intended winner'**
  String get conflictWinner;

  /// No description provided for @noConflicts.
  ///
  /// In en, this message translates to:
  /// **'No recognized conflicts.'**
  String get noConflicts;

  /// No description provided for @conflictCoverageIncomplete.
  ///
  /// In en, this message translates to:
  /// **'Conflict knowledge is incomplete for enabled mods; additional conflicts may exist.'**
  String get conflictCoverageIncomplete;

  /// No description provided for @loadOrderDirection.
  ///
  /// In en, this message translates to:
  /// **'Load order: lower priority first; later mods have higher intended priority.'**
  String get loadOrderDirection;

  /// No description provided for @footprintCoverageScope.
  ///
  /// In en, this message translates to:
  /// **'Coverage describes recognized conflict targets only; it does not prove runtime priority.'**
  String get footprintCoverageScope;

  /// No description provided for @footprintCoverageExact.
  ///
  /// In en, this message translates to:
  /// **'Exact — the component\'s conflict-target list is complete.'**
  String get footprintCoverageExact;

  /// No description provided for @footprintCoveragePartial.
  ///
  /// In en, this message translates to:
  /// **'Partial — listed conflict targets are known, but the component can affect more.'**
  String get footprintCoveragePartial;

  /// No description provided for @footprintCoverageAdvisory.
  ///
  /// In en, this message translates to:
  /// **'Advisory — listed targets are hints, not exhaustive proof.'**
  String get footprintCoverageAdvisory;

  /// No description provided for @footprintCoverageOpaque.
  ///
  /// In en, this message translates to:
  /// **'Opaque — the component\'s conflict targets are unknown.'**
  String get footprintCoverageOpaque;

  /// No description provided for @footprintCoverageExactLabel.
  ///
  /// In en, this message translates to:
  /// **'Exact'**
  String get footprintCoverageExactLabel;

  /// No description provided for @footprintCoveragePartialLabel.
  ///
  /// In en, this message translates to:
  /// **'Partial'**
  String get footprintCoveragePartialLabel;

  /// No description provided for @footprintCoverageAdvisoryLabel.
  ///
  /// In en, this message translates to:
  /// **'Advisory'**
  String get footprintCoverageAdvisoryLabel;

  /// No description provided for @footprintCoverageOpaqueLabel.
  ///
  /// In en, this message translates to:
  /// **'Opaque'**
  String get footprintCoverageOpaqueLabel;

  /// No description provided for @conflictsUnverified.
  ///
  /// In en, this message translates to:
  /// **'Conflicts are unverified until the library state is refreshed.'**
  String get conflictsUnverified;

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

  /// No description provided for @removeModDeploymentHint.
  ///
  /// In en, this message translates to:
  /// **'Removing it from the library does not change an existing deployment immediately. If the mod is already deployed, choose Apply afterwards to update the game installation.'**
  String get removeModDeploymentHint;

  /// No description provided for @removeModSuccess.
  ///
  /// In en, this message translates to:
  /// **'Removed “{name}” from the library.'**
  String removeModSuccess(String name);

  /// No description provided for @removeModFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not remove “{name}”: {error}'**
  String removeModFailed(String name, String error);

  /// No description provided for @removeModPartialFailure.
  ///
  /// In en, this message translates to:
  /// **'Removed “{name}”, but follow-up processing reported an error. The library state was reloaded: {error}'**
  String removeModPartialFailure(String name, String error);

  /// No description provided for @removeModOutcomeUnknown.
  ///
  /// In en, this message translates to:
  /// **'Could not verify whether “{name}” was removed: {error} — Refresh to check the library state.'**
  String removeModOutcomeUnknown(String name, String error);

  /// No description provided for @libraryStateUnknown.
  ///
  /// In en, this message translates to:
  /// **'The library state could not be verified. Choose Refresh before changing or applying mods.'**
  String get libraryStateUnknown;

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
  /// **'© 2026 GORE contributors'**
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
  /// **'UI scale'**
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
