// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Einstellungen';

  @override
  String get settingsGameExe => 'Spiel-Programmdatei';

  @override
  String get settingsGameExePick => 'Auswählen…';

  @override
  String get settingsLanguage => 'Sprache';

  @override
  String get statusInSync => 'Synchron';

  @override
  String get statusChangesPending => 'Änderungen ausstehend';

  @override
  String get statusGameUpdated => 'Spiel aktualisiert';

  @override
  String get statusStudioDeploy => 'Studio-Bereitstellung aktiv';

  @override
  String get statusNothingDeployed => 'Nichts bereitgestellt';

  @override
  String get actionImport => 'Importieren';

  @override
  String get actionApply => 'Anwenden';

  @override
  String get actionUndeployAll => 'Alle Bereitstellungen aufheben';

  @override
  String get commonCancel => 'Abbrechen';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Ordner importieren…';

  @override
  String get importFile => 'Datei importieren…';

  @override
  String get applyTooltip => 'Ladeliste auf das Spiel anwenden';

  @override
  String get undeployAllAction => 'Alle Bereitstellungen aufheben';

  @override
  String get undeployAllConfirm =>
      'Alles vom Manager Bereitgestellte aus dem Spiel entfernen?';

  @override
  String get takeOverTitle => 'Studio-Bereitstellung aktiv';

  @override
  String get takeOverBody =>
      'mod-studio hat einen Mod im Spiel bereitgestellt. Übernehmen, damit der Manager diese Ladeliste anwenden kann?';

  @override
  String get takeOverAction => 'Übernehmen';

  @override
  String get refreshAction => 'Aktualisieren';

  @override
  String conflictsTitle(int count) {
    return 'Konflikte ($count)';
  }

  @override
  String get conflictWinner => 'Gewinner';

  @override
  String get componentsTitle => 'Komponenten';

  @override
  String targetsMore(int count) {
    return '+$count weitere';
  }

  @override
  String get removeModAction => 'Entfernen';

  @override
  String removeModConfirm(String name) {
    return '„$name“ aus der Bibliothek entfernen?';
  }

  @override
  String get errorSetGamePath =>
      'Lege zuerst den Spielpfad in den Einstellungen fest.';

  @override
  String applyReportApplied(int count) {
    return '$count Mods angewendet.';
  }

  @override
  String get warningsTitle => 'Warnungen';

  @override
  String get modDisabledHint => 'Deaktiviert';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'Triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Rohdatei';

  @override
  String get kindMixed => 'gemischt';

  @override
  String get sevHard => 'hart';

  @override
  String get sevSoft => 'weich';

  @override
  String get sevInfo => 'Info';
}
