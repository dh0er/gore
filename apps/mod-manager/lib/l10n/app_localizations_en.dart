// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get statusUnknown => 'Unknown';

  @override
  String get recoveryAction => 'Recover';

  @override
  String get recoveryRequiredConfirm =>
      'Recover the interrupted deployment and remove any partially deployed files?';

  @override
  String get statusRecoveryRequired => 'Recovery required';

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
    return 'Conflicts ($count)';
  }

  @override
  String get conflictWinner => 'winner';

  @override
  String get noConflicts => 'No conflicts.';

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
  String removeModFailed(String name, String error) {
    return 'Could not remove “$name”: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return 'Removed “$name”, but follow-up processing reported an error. The library state was reloaded: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Could not verify whether “$name” was removed: $error — Refresh to check the library state.';
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
