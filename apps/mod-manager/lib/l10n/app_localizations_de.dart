// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get statusUnknown => 'Unbekannt';

  @override
  String get recoveryAction => 'Wiederherstellen';

  @override
  String get recoveryRequiredConfirm =>
      'Unterbrochene Bereitstellung wiederherstellen und alle teilweise bereitgestellten Dateien entfernen?';

  @override
  String get statusRecoveryRequired => 'Wiederherstellung erforderlich';

  @override
  String get appTitle => 'GORE Mod Manager';

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
  String get noConflicts => 'Keine Konflikte.';

  @override
  String get conflictsUnverified =>
      'Konflikte sind unverifiziert, bis der Bibliothekszustand aktualisiert wurde.';

  @override
  String get componentsTitle => 'Komponenten';

  @override
  String targetsMore(int count) {
    return '+$count weitere';
  }

  @override
  String get removeModDeploymentHint =>
      'Das Entfernen aus der Bibliothek ändert eine bestehende Bereitstellung nicht sofort. Falls die Mod bereits bereitgestellt ist, wähle anschließend „Anwenden“, um die Spielinstallation zu aktualisieren.';

  @override
  String removeModSuccess(String name) {
    return '„$name“ wurde aus der Bibliothek entfernt.';
  }

  @override
  String removeModFailed(String name, String error) {
    return '„$name“ konnte nicht entfernt werden: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '„$name“ wurde entfernt, aber die Nachbearbeitung meldete einen Fehler. Der Bibliothekszustand wurde neu geladen: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Es konnte nicht überprüft werden, ob „$name“ entfernt wurde: $error — Aktualisiere, um den Bibliothekszustand zu prüfen.';
  }

  @override
  String get libraryStateUnknown =>
      'Der Bibliothekszustand konnte nicht überprüft werden. Wähle „Aktualisieren“, bevor du Mods änderst oder anwendest.';

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

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Über';

  @override
  String get aboutCopyright => '© 2026 GORE-Mitwirkende';

  @override
  String get aboutLicense => 'Lizenziert unter der MIT-Lizenz.';

  @override
  String get appearanceTitle => 'Erscheinungsbild';

  @override
  String get theme => 'Design';

  @override
  String get themeLight => 'Hell';

  @override
  String get themeDark => 'Dunkel';

  @override
  String get themeSystem => 'System';

  @override
  String get uiScale => 'UI-Skalierung';

  @override
  String get resetZoomTooltip => 'Zoom zurücksetzen (Strg+0)';

  @override
  String get zoomTip =>
      'Tipp: Strg + / Strg - ändert den Zoom überall in der App.';

  @override
  String get lightMode => 'Heller Modus';

  @override
  String get darkMode => 'Dunkler Modus';

  @override
  String get minimize => 'Minimieren';

  @override
  String get restore => 'Wiederherstellen';

  @override
  String get maximize => 'Maximieren';

  @override
  String get close => 'Schließen';
}
