// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager can\'t start';

  @override
  String get coreDllMissingMessage =>
      'A required program file is missing (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'A required program file could not be loaded.';

  @override
  String get coreVerificationFailedMessage =>
      'A required program file could not be verified.';

  @override
  String get coreManagerTooOldMessage =>
      'The program files are newer than the Mod Manager. Update the Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'The program files are older than the Mod Manager. Reinstall the Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'The program files are missing features this Mod Manager needs.';

  @override
  String get coreBlockedRepairHint =>
      'Reinstall or repair the Mod Manager, then start it again.';

  @override
  String get coreTechnicalDetails => 'Technical details';

  @override
  String get coreCopyTechnicalDetails => 'Copy technical details';

  @override
  String get coreTechnicalDetailsCopied => 'Technical details copied';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Technical details could not be copied. Try again.';

  @override
  String get preflightAttention =>
      'Something needs your attention before mods can change.';

  @override
  String get preflightGameRunning =>
      'Gothic is still running. Close the game before changing mods.';

  @override
  String get managerOperationFailed => 'The operation failed.';

  @override
  String get libraryOperationFailed => 'The mod list could not be loaded.';

  @override
  String get conflictsUnavailable => 'Conflicts could not be checked.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Applied: $applied. Warnings: $warnings.';
  }

  @override
  String get modDetailKind => 'Type';

  @override
  String get modDetailVersion => 'Version';

  @override
  String get modDetailAuthor => 'Author';

  @override
  String get modDetailSource => 'Source';

  @override
  String get modDetailImported => 'Imported';

  @override
  String get componentLocalization => 'Text';

  @override
  String get componentAudio => 'Sound';

  @override
  String get componentAngelScript => 'Scripts';

  @override
  String get componentTexture => 'Textures';

  @override
  String get componentGameFiles => 'Game files';

  @override
  String get componentVoice => 'Voice';

  @override
  String get componentKindLocalizationPatch => 'Text changes';

  @override
  String get componentKindAudioPatch => 'Sound changes';

  @override
  String get componentKindAngelScriptPatch => 'Script changes';

  @override
  String get componentKindTexturePatch => 'Texture changes';

  @override
  String get componentKindLoosePak => 'PAK file';

  @override
  String get componentKindTriplet => 'IoStore container';

  @override
  String get componentKindUe4ssLua => 'UE4SS script';

  @override
  String get componentKindRawFile => 'File';

  @override
  String get componentKindFilePatch => 'Replaced game file';

  @override
  String get componentKindPakFilePatch => 'Game file from a ~mods PAK';

  @override
  String get componentKindVoiceArchivePatch => 'Voice lines';

  @override
  String get rawTargetGameText => 'All game text';

  @override
  String get rawTargetGameScripts => 'All game scripts';

  @override
  String get rawTargetSoundBank => 'Sound bank';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Sound bank: $name';
  }

  @override
  String get conflictKindLocalization => 'Text';

  @override
  String get conflictKindAudio => 'Sound';

  @override
  String get conflictKindAsset => 'Game data';

  @override
  String get conflictKindCdo => 'Object values';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (unclear)';

  @override
  String get conflictKindScriptModule => 'Game script';

  @override
  String get conflictKindVoiceArchive => 'Voice lines';

  @override
  String get conflictKindRawFile => 'File';

  @override
  String get conflictKindLooseFile => 'Game file';

  @override
  String get preflightUnavailable =>
      'The game installation could not be checked.';

  @override
  String get preflightRetry => 'Check again';

  @override
  String get preflightReviewStatus => 'Show status';

  @override
  String get preflightReviewRecovery => 'Show help';

  @override
  String get installRecoveryTitle => 'Interrupted installation';

  @override
  String get installRecoveryBody =>
      'GORE found leftover data from an installation or a script build. That job may still be running, or it ended and left this behind. GORE cannot clean it up safely on its own.';

  @override
  String get installRecoverySteps =>
      'If the job is still running, wait for it to finish — do not stop it and do not delete any files. Once you are sure nothing is running, follow README.txt in the folder below and check again. If no folder is listed or you are unsure, leave everything as it is and ask for help.';

  @override
  String get installRecoveryEvidence => 'What GORE found';

  @override
  String get managerRecoveryTitle => 'Repair interrupted change';

  @override
  String get managerRecoveryConfirm =>
      'GORE found an interrupted change and can put the game back into a known state. Your savegames are never touched.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Nothing left to repair. The status was checked again.';

  @override
  String get managerRecoveryBusy =>
      'The job is running again. Nothing was changed — wait for it to finish.';

  @override
  String get managerRecoveryLockCleared =>
      'The interrupted job had not changed anything yet. It was cleaned up.';

  @override
  String get managerRecoveryRestoredPristine =>
      'The change was rolled back. The game is back to its earlier state.';

  @override
  String get managerRecoveryApplyPreserved =>
      'Apply had already finished. Nothing was lost.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'The removal had already finished. Leftovers were cleaned up.';

  @override
  String get managerRecoveryCompileRequired =>
      'This belongs to a script build, so nothing was changed. Open the repair help.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE could not check the interrupted job safely. Nothing was changed.';

  @override
  String get managerRecoveryFailed =>
      'The repair could not be finished. Check the status before trying again.';

  @override
  String get statusUnknown => 'Unknown';

  @override
  String statusDetailsTitle(String status) {
    return 'Status: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Show details: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Choose your Gothic installation in Settings first.';

  @override
  String get statusDetailsNoDeployment =>
      'No mods are installed in the game right now.';

  @override
  String get statusDetailsInSyncDescription =>
      'The game has exactly the mods you ticked here.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods in the game';

  @override
  String get statusDetailsChangesDescription =>
      'Your selection differs from what is in the game.';

  @override
  String get statusDetailsCurrentlyDeployed => 'In the game now';

  @override
  String get statusDetailsAfterApply => 'After Apply';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'The game was updated and overwrote mod files. Apply again to put them back.';

  @override
  String get statusDetailsDriftedFiles => 'Affected files';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio currently has mods in this game. Take the game over before the Manager applies yours.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio did not report a name.';

  @override
  String get statusDetailsRecoveryDescription =>
      'A change was interrupted. Repair it before changing mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'The status could not be read. Refresh first.';

  @override
  String get statusDetailsUnavailable => 'No details available.';

  @override
  String get statusDetailsEmptyLoadout => 'No mods.';

  @override
  String get statusDetailsLastError => 'Last error';

  @override
  String get statusDetailsLastApply => 'Last Apply';

  @override
  String get statusDetailsAppliedMods => 'Applied mods';

  @override
  String get statusDetailsWarnings => 'Warnings';

  @override
  String get statusDetailsReapply => 'Reapply';

  @override
  String get statusDetailsOpenSettings => 'Open Settings';

  @override
  String get recoveryAction => 'Repair';

  @override
  String get recoveryRequiredConfirm =>
      'Repair the interrupted change and remove any half-installed files?';

  @override
  String get statusRecoveryRequired => 'Repair needed';

  @override
  String get statusDetailsOwnershipTitle => 'Files GORE manages';

  @override
  String get statusDetailsOwnershipDescription =>
      'Recorded when mods were applied — not a check that the files still exist.';

  @override
  String get statusDetailsOwnershipLive => 'Replaced game files';

  @override
  String get statusDetailsOwnershipBackups => 'Backups of the originals';

  @override
  String get statusDetailsOwnershipAdditive => 'Added mod files';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS mod directories';

  @override
  String get statusDetailsOwnershipRecovery => 'Repair files';

  @override
  String get statusDetailsOwnershipEmpty => 'Nothing recorded here.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Showing $shown of $total paths.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Settings';

  @override
  String get settingsGameExe => 'Gothic installation';

  @override
  String get settingsGameExePick => 'Choose…';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get libraryEmptyTitle => 'No mods yet';

  @override
  String get libraryEmptyBody =>
      'Import a folder or a mod file to get started.';

  @override
  String get detailEmptyHint => 'Pick a mod to see what it changes.';

  @override
  String get settingsAdvanced => 'Advanced details';

  @override
  String get settingsAdvancedHint =>
      'Show the technical side: affected entries, how reliable the conflict check is, and the files GORE manages.';

  @override
  String get updatesTitle => 'Updates';

  @override
  String get checkForUpdatesAutomatically => 'Check for updates automatically';

  @override
  String get checkForUpdatesNow => 'Check for updates now';

  @override
  String get updatesPortableNotice =>
      'The portable version opens the download page in your browser. Replace your existing files with the new download.';

  @override
  String get updateCheckFailed =>
      'Could not check for updates. Please try again later.';

  @override
  String get updateUpToDate => 'You are using the latest version.';

  @override
  String get updateAvailableTitle => 'Update available';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'Version $version is available. You have $current.';
  }

  @override
  String get updateLater => 'Later';

  @override
  String get updateDownload => 'Download';

  @override
  String updateOpenFailed(String url) {
    return 'Could not open the download page. You can reach it at $url';
  }

  @override
  String get statusInSync => 'Up to date';

  @override
  String get statusChangesPending => 'Not applied yet';

  @override
  String get statusGameUpdated => 'Game was updated';

  @override
  String get statusStudioDeploy => 'Mod Studio active';

  @override
  String get statusNothingDeployed => 'No mods in game';

  @override
  String get actionImport => 'Import';

  @override
  String get actionApply => 'Apply';

  @override
  String get actionStartGame => 'Start game';

  @override
  String get startGameTooltip =>
      'Launch Gothic with the mods currently in the game';

  @override
  String get startGameFailed =>
      'Gothic could not be started. Check the game installation in Settings.';

  @override
  String get commonCancel => 'Cancel';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Import folder…';

  @override
  String get importFile => 'Import file…';

  @override
  String importOutcomeCreated(String name) {
    return 'Added “$name”.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Updated “$name”.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name” is already in your list.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'No existing mod matched.',
      'source': 'Matched by the same import source.',
      'content': 'Matched by verified identical content.',
      'entry_id': 'Matched by mod ID.',
      'other': 'Match details are unavailable.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'This matches more than one mod you already have. Remove the duplicates, then try again.';

  @override
  String get importRefusalIdentityConflict =>
      'The source and the contents match two different mods you already have. Sort those out, then try again.';

  @override
  String get importFailed =>
      'This could not be imported. Supported: folders, ZIP archives and single mod files (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extract .7z or .rar first, then import the folder. It may already have been added or updated — refresh your list before trying again.';

  @override
  String get importPickerFailed =>
      'The file picker could not be opened. Nothing was imported.';

  @override
  String get importOutcomeUnknown =>
      'The result is unclear. Refresh to check your mod list.';

  @override
  String get applyTooltip => 'Install the ticked mods into the game';

  @override
  String get undeployAllAction => 'Remove all from game';

  @override
  String get undeployAllConfirm =>
      'Remove every mod the Manager installed from the game?';

  @override
  String get takeOverTitle => 'Mod Studio is active';

  @override
  String get takeOverBody =>
      'Mod Studio currently has a mod in the game. Take over so the Manager can apply your selection?';

  @override
  String get takeOverAction => 'Take over';

  @override
  String get refreshAction => 'Refresh';

  @override
  String conflictsTitle(int count) {
    return 'Conflicts ($count)';
  }

  @override
  String get conflictWinner => 'wins';

  @override
  String get noConflicts => 'No conflicts found.';

  @override
  String get conflictCoverageIncomplete =>
      'Some mods can\'t be checked completely, so there may be more conflicts.';

  @override
  String get loadOrderDirection =>
      'Mods further down the list override the ones above them.';

  @override
  String get footprintCoverageScope =>
      'Only known conflict targets are listed. This is no guarantee of what happens in game.';

  @override
  String get footprintTargetsExact => 'Affected entries — the full list:';

  @override
  String get footprintTargetsPartial => 'Affected entries — there may be more:';

  @override
  String get footprintTargetsAdvisory =>
      'Probably affected entries — hints, not proof:';

  @override
  String get footprintTargetsOpaque => 'GORE cannot tell what this changes.';

  @override
  String get conflictsUnverified => 'Conflicts unknown — refresh first.';

  @override
  String get componentsTitle => 'What this mod changes';

  @override
  String targetsMore(int count) {
    return '+$count more';
  }

  @override
  String get removeModDeploymentHint =>
      'This only removes it from your list. If it is installed in the game, choose Apply afterwards.';

  @override
  String removeModSuccess(String name) {
    return 'Removed “$name”.';
  }

  @override
  String removeModFailed(String name) {
    return 'Could not remove “$name”.';
  }

  @override
  String removeModPartialFailure(String name) {
    return 'Removed “$name”, but the list could not be fully updated.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Could not confirm whether “$name” was removed.';
  }

  @override
  String get libraryStateUnknown =>
      'The mod list is out of date. Refresh before changing or applying mods.';

  @override
  String get removeModAction => 'Remove';

  @override
  String removeModConfirm(String name) {
    return 'Remove “$name” from your list?';
  }

  @override
  String get errorSetGamePath =>
      'Choose your Gothic installation in Settings first.';

  @override
  String applyReportApplied(int count) {
    return 'Applied $count mods.';
  }

  @override
  String get modDisabledHint => 'Disabled';

  @override
  String get kindGoremod => 'GORE bundle';

  @override
  String get kindTriplet => 'IoStore mod';

  @override
  String get kindPak => 'PAK mod';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Whole-file replacement';

  @override
  String get kindMixed => 'Mixed';

  @override
  String get sevHard => 'Conflict';

  @override
  String get sevSoft => 'Warning';

  @override
  String get sevInfo => 'Note';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'About';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Licensed under the MIT License.';

  @override
  String get appearanceTitle => 'Appearance';

  @override
  String get theme => 'Theme';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get themeSystem => 'System';

  @override
  String get uiScale => 'Display size';

  @override
  String get resetZoomTooltip => 'Reset zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Tip: Ctrl + / Ctrl - changes the zoom anywhere in the app.';

  @override
  String get lightMode => 'Light mode';

  @override
  String get darkMode => 'Dark mode';

  @override
  String get minimize => 'Minimize';

  @override
  String get restore => 'Restore';

  @override
  String get maximize => 'Maximize';

  @override
  String get close => 'Close';
}
