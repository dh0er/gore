// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager unavailable';

  @override
  String get coreDllMissingMessage =>
      'The required gore_ffi.dll was not found.';

  @override
  String get coreDllLoadFailedMessage =>
      'The native GORE Core library could not be loaded.';

  @override
  String get coreVerificationFailedMessage =>
      'The native GORE Core library could not be verified.';

  @override
  String get coreManagerTooOldMessage =>
      'This GORE Core version is newer than the Mod Manager. Update the Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'This GORE Core version is older than the Mod Manager. Update or repair the complete Mod Manager installation.';

  @override
  String get coreCommandsMissingMessage =>
      'The GORE Core library does not provide all commands required by this Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Update or repair the complete Mod Manager package, then restart the app.';

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
  String get preflightAttention => 'GORE cannot continue yet.';

  @override
  String get preflightGameRunning =>
      'Gothic is still running. Close the game before changing mods.';

  @override
  String get managerOperationFailed => 'The Manager operation failed.';

  @override
  String get libraryOperationFailed => 'The library could not be refreshed.';

  @override
  String get conflictsUnavailable => 'Conflicts could not be checked.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Applied: $applied. Warnings: $warnings.';
  }

  @override
  String get modDetailKind => 'Kind';

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
  String get componentAudio => 'Audio';

  @override
  String get componentAngelScript => 'AngelScript';

  @override
  String get componentTexture => 'Texture';

  @override
  String get componentKindLocalizationPatch => 'Localization patch';

  @override
  String get componentKindAudioPatch => 'Audio patch';

  @override
  String get componentKindAngelScriptPatch => 'AngelScript patch';

  @override
  String get componentKindTexturePatch => 'Texture patch';

  @override
  String get componentKindLoosePak => 'Loose PAK';

  @override
  String get componentKindTriplet => 'PAK triplet';

  @override
  String get componentKindUe4ssLua => 'UE4SS Lua';

  @override
  String get componentKindRawFile => 'Raw file';

  @override
  String get componentKindFilePatch => 'File patch';

  @override
  String get componentKindPakFilePatch => 'PAK file patch';

  @override
  String get componentKindVoiceArchivePatch => 'Voice archive patch';

  @override
  String get conflictKindLocalization => 'Localization';

  @override
  String get conflictKindAudio => 'Audio';

  @override
  String get conflictKindAsset => 'Asset';

  @override
  String get conflictKindCdo => 'CDO';

  @override
  String get conflictKindUe4ssUnknown => 'Unknown UE4SS';

  @override
  String get conflictKindScriptModule => 'Script module';

  @override
  String get conflictKindVoiceArchive => 'Voice archive';

  @override
  String get conflictKindRawFile => 'Raw file';

  @override
  String get conflictKindLooseFile => 'Loose file';

  @override
  String get preflightUnavailable => 'GORE could not check the installation.';

  @override
  String get preflightRetry => 'Check again';

  @override
  String get preflightReviewStatus => 'Review status';

  @override
  String get preflightReviewRecovery => 'Recovery help';

  @override
  String get installRecoveryTitle => 'Installation recovery';

  @override
  String get installRecoveryBody =>
      'GORE found recovery data for an installation or script build. The related operation may still be running, or the data may be left over from an operation that has ended. GORE cannot safely repair this automatically.';

  @override
  String get installRecoverySteps =>
      'If the related operation is still running, wait for it to finish. Do not stop it or delete any lock files. Only after you are sure that no related operation is running, follow README.txt in the recovery folder below. If no folder is listed or you are unsure, leave the recovery data unchanged and get help. Then check again.';

  @override
  String get installRecoveryEvidence => 'Detected recovery data';

  @override
  String get managerRecoveryTitle => 'Recover interrupted Manager change';

  @override
  String get managerRecoveryConfirm =>
      'GORE found a clearly interrupted Manager change. Continue only if you want GORE to verify the recorded operation and restore the installation to a known state. Savegames are never changed.';

  @override
  String get managerRecoveryAlreadyClean =>
      'The interrupted change was already resolved. The installation was checked again.';

  @override
  String get managerRecoveryBusy =>
      'The operation is active again. Nothing was changed; wait for it to finish and check again.';

  @override
  String get managerRecoveryLockCleared =>
      'The interrupted operation had not changed the installation. Its obsolete lock was removed safely.';

  @override
  String get managerRecoveryRestoredPristine =>
      'The interrupted change was rolled back and the installation\'s recorded baseline state was restored.';

  @override
  String get managerRecoveryApplyPreserved =>
      'Apply had already completed. The recorded state was preserved, and the status was checked again.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'Removal had completed. Remaining transaction data was cleaned up, and the status was checked again.';

  @override
  String get managerRecoveryCompileRequired =>
      'This belongs to script-build recovery, so Manager left it unchanged. Review the recovery help.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE could not verify the interrupted operation safely. Nothing was changed; review the current recovery details.';

  @override
  String get managerRecoveryFailed =>
      'Recovery could not be completed. GORE tried to check the installation again, but its current state may be unknown. Review the status before trying again.';

  @override
  String get statusUnknown => 'Unknown';

  @override
  String statusDetailsTitle(String status) {
    return 'Deployment: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Show deployment details: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Choose a game installation in Settings to inspect its deployment status.';

  @override
  String get statusDetailsNoDeployment =>
      'No Manager deployment is installed for this game.';

  @override
  String get statusDetailsInSyncDescription =>
      'The deployed mods match the current loadout.';

  @override
  String get statusDetailsDeployedLoadout => 'Deployed load order';

  @override
  String get statusDetailsChangesDescription =>
      'The current deployment differs from what Apply will install.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Currently deployed';

  @override
  String get statusDetailsAfterApply => 'After Apply';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Game files changed after the last deployment. Reapply the loadout to restore the Manager-owned files.';

  @override
  String get statusDetailsDriftedFiles => 'Changed files';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio currently owns this game installation. Take over before applying a Manager loadout.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Studio did not report a mod name.';

  @override
  String get statusDetailsRecoveryDescription =>
      'A deployment was interrupted. Recover it before applying or removing Manager mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'Deployment status could not be verified. Refresh before applying mods.';

  @override
  String get statusDetailsUnavailable =>
      'The installed core did not provide these details.';

  @override
  String get statusDetailsEmptyLoadout => 'No mods in this loadout.';

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
  String get recoveryAction => 'Recover';

  @override
  String get recoveryRequiredConfirm =>
      'Recover the interrupted deployment and remove any partially deployed files?';

  @override
  String get statusRecoveryRequired => 'Recovery required';

  @override
  String get statusDetailsOwnershipTitle => 'Recorded ownership evidence';

  @override
  String get statusDetailsOwnershipDescription =>
      'Paths recorded in the Manager deploy record. They do not prove that those paths still exist.';

  @override
  String get statusDetailsOwnershipLive => 'Replaced game files';

  @override
  String get statusDetailsOwnershipBackups => 'Pristine backups';

  @override
  String get statusDetailsOwnershipAdditive => 'Added pak and container files';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS mod directories';

  @override
  String get statusDetailsOwnershipRecovery => 'Recovery files and holders';

  @override
  String get statusDetailsOwnershipEmpty => 'No paths recorded in this group.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '$shown of $total recorded paths shown.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Settings';

  @override
  String get settingsGameExe => 'Game executable';

  @override
  String get settingsGameExePick => 'Choose…';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get statusInSync => 'In sync';

  @override
  String get statusChangesPending => 'Changes pending';

  @override
  String get statusGameUpdated => 'Game updated';

  @override
  String get statusStudioDeploy => 'Studio deployment active';

  @override
  String get statusNothingDeployed => 'Nothing deployed';

  @override
  String get actionImport => 'Import';

  @override
  String get actionApply => 'Apply';

  @override
  String get actionUndeployAll => 'Undeploy all';

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
    return 'Added “$name” to the library.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Updated “$name” in the library.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name” is already in the library.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'No existing library entry was matched.',
      'source': 'Matched by the same import source.',
      'content': 'Matched by verified identical content.',
      'entry_id': 'Matched by mod ID.',
      'other': 'Match details are unavailable.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'This import matches more than one library entry. Review or remove the duplicates, then try again.';

  @override
  String get importRefusalIdentityConflict =>
      'The import source and its content match different library entries. Review or remove the conflicting entries, then try again.';

  @override
  String get importFailed =>
      'The import could not be completed. Supported sources: folders, ZIP, loose *_P.pak files, complete .utoc/.ucas sets (optional .pak), .lcache, .bank, and PrecompiledScript*.Cache. Extract .7z or .rar first, then import the folder. The source may be unsupported, corrupt, or incomplete. The mod may already have been added or updated; refresh and check the library before trying again.';

  @override
  String get importPickerFailed =>
      'The file or folder picker could not be opened. No import was started. Try again.';

  @override
  String get importOutcomeUnknown =>
      'The import result could not be verified. Choose Refresh to check the library.';

  @override
  String get applyTooltip => 'Apply the loadout to the game';

  @override
  String get undeployAllAction => 'Undeploy all';

  @override
  String get undeployAllConfirm =>
      'Remove everything the manager deployed from the game?';

  @override
  String get takeOverTitle => 'Studio deployment active';

  @override
  String get takeOverBody =>
      'mod-studio has deployed a mod to the game. Take over so the manager can apply this loadout?';

  @override
  String get takeOverAction => 'Take over';

  @override
  String get refreshAction => 'Refresh';

  @override
  String conflictsTitle(int count) {
    return 'Findings ($count)';
  }

  @override
  String get conflictWinner => 'intended winner';

  @override
  String get noConflicts => 'No recognized conflicts.';

  @override
  String get conflictCoverageIncomplete =>
      'Conflict knowledge is incomplete for enabled mods; additional conflicts may exist.';

  @override
  String get loadOrderDirection =>
      'Load order: lower priority first; later mods have higher intended priority.';

  @override
  String get footprintCoverageScope =>
      'Coverage describes recognized conflict targets only; it does not prove runtime priority.';

  @override
  String get footprintCoverageExact =>
      'Exact — the component\'s conflict-target list is complete.';

  @override
  String get footprintCoveragePartial =>
      'Partial — listed conflict targets are known, but the component can affect more.';

  @override
  String get footprintCoverageAdvisory =>
      'Advisory — listed targets are hints, not exhaustive proof.';

  @override
  String get footprintCoverageOpaque =>
      'Opaque — the component\'s conflict targets are unknown.';

  @override
  String get footprintCoverageExactLabel => 'Exact';

  @override
  String get footprintCoveragePartialLabel => 'Partial';

  @override
  String get footprintCoverageAdvisoryLabel => 'Advisory';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaque';

  @override
  String get conflictsUnverified =>
      'Conflicts are unverified until the library state is refreshed.';

  @override
  String get componentsTitle => 'Components';

  @override
  String targetsMore(int count) {
    return '+$count more';
  }

  @override
  String get removeModDeploymentHint =>
      'Removing it from the library does not change an existing deployment immediately. If the mod is already deployed, choose Apply afterwards to update the game installation.';

  @override
  String removeModSuccess(String name) {
    return 'Removed “$name” from the library.';
  }

  @override
  String removeModFailed(String name) {
    return 'Could not remove “$name”';
  }

  @override
  String removeModPartialFailure(String name) {
    return 'Removed “$name”, but the library could not be updated completely.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Could not verify whether “$name” was removed.';
  }

  @override
  String get libraryStateUnknown =>
      'The library state could not be verified. Choose Refresh before changing or applying mods.';

  @override
  String get removeModAction => 'Remove';

  @override
  String removeModConfirm(String name) {
    return 'Remove “$name” from the library?';
  }

  @override
  String get errorSetGamePath => 'Set the game path in Settings first.';

  @override
  String applyReportApplied(int count) {
    return 'Applied $count mods.';
  }

  @override
  String get warningsTitle => 'Warnings';

  @override
  String get modDisabledHint => 'Disabled';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'raw file';

  @override
  String get kindMixed => 'mixed';

  @override
  String get sevHard => 'hard';

  @override
  String get sevSoft => 'soft';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'About';

  @override
  String get aboutCopyright => '© 2026 GORE contributors';

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
  String get uiScale => 'UI scale';

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
