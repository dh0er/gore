// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager nicht verfügbar';

  @override
  String get coreDllMissingMessage =>
      'Die erforderliche gore_ffi.dll wurde nicht gefunden.';

  @override
  String get coreDllLoadFailedMessage =>
      'Die native GORE-Core-Bibliothek konnte nicht geladen werden.';

  @override
  String get coreVerificationFailedMessage =>
      'Die native GORE-Core-Bibliothek konnte nicht überprüft werden.';

  @override
  String get coreManagerTooOldMessage =>
      'Diese GORE-Core-Version ist neuer als der Mod Manager. Aktualisiere den Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Diese GORE-Core-Version ist älter als der Mod Manager. Aktualisiere oder repariere die vollständige Mod-Manager-Installation.';

  @override
  String get coreCommandsMissingMessage =>
      'Die GORE-Core-Bibliothek stellt nicht alle Befehle bereit, die dieser Mod Manager benötigt.';

  @override
  String get coreBlockedRepairHint =>
      'Aktualisiere oder repariere das vollständige Mod-Manager-Paket und starte die App dann neu.';

  @override
  String get coreTechnicalDetails => 'Technische Details';

  @override
  String get coreCopyTechnicalDetails => 'Technische Details kopieren';

  @override
  String get coreTechnicalDetailsCopied => 'Technische Details kopiert';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Die technischen Details konnten nicht kopiert werden. Versuche es erneut.';

  @override
  String get statusUnknown => 'Unbekannt';

  @override
  String statusDetailsTitle(String status) {
    return 'Bereitstellung: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Bereitstellungsdetails anzeigen: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Wähle in den Einstellungen eine Spielinstallation aus, um ihren Bereitstellungsstatus zu prüfen.';

  @override
  String get statusDetailsNoDeployment =>
      'Für dieses Spiel ist keine Manager-Bereitstellung installiert.';

  @override
  String get statusDetailsInSyncDescription =>
      'Die bereitgestellten Mods entsprechen der aktuellen Ladeliste.';

  @override
  String get statusDetailsDeployedLoadout => 'Bereitgestellte Ladereihenfolge';

  @override
  String get statusDetailsChangesDescription =>
      'Die aktuelle Bereitstellung unterscheidet sich von dem, was Anwenden installieren wird.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Aktuell bereitgestellt';

  @override
  String get statusDetailsAfterApply => 'Nach Anwenden';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Spieldateien wurden seit der letzten Bereitstellung geändert. Wende die Ladeliste erneut an, um die Manager-Dateien wiederherzustellen.';

  @override
  String get statusDetailsDriftedFiles => 'Geänderte Dateien';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio besitzt derzeit diese Spielinstallation. Übernimm sie, bevor du eine Manager-Ladeliste anwendest.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio-Mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Studio hat keinen Mod-Namen gemeldet.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Eine Bereitstellung wurde unterbrochen. Stelle sie wieder her, bevor du Manager-Mods anwendest oder entfernst.';

  @override
  String get statusDetailsUnknownDescription =>
      'Der Bereitstellungsstatus konnte nicht überprüft werden. Aktualisiere ihn, bevor du Mods anwendest.';

  @override
  String get statusDetailsUnavailable =>
      'Der installierte Core hat diese Details nicht bereitgestellt.';

  @override
  String get statusDetailsEmptyLoadout => 'Keine Mods in dieser Ladeliste.';

  @override
  String get statusDetailsLastError => 'Letzter Fehler';

  @override
  String get statusDetailsLastApply => 'Letztes Anwenden';

  @override
  String get statusDetailsAppliedMods => 'Angewendete Mods';

  @override
  String get statusDetailsWarnings => 'Warnungen';

  @override
  String get statusDetailsReapply => 'Erneut anwenden';

  @override
  String get statusDetailsOpenSettings => 'Einstellungen öffnen';

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
    return 'Befunde ($count)';
  }

  @override
  String get conflictWinner => 'Gewinner';

  @override
  String get noConflicts => 'Keine erkannten Konflikte.';

  @override
  String get conflictCoverageIncomplete =>
      'Das Konfliktwissen für aktivierte Mods ist unvollständig; weitere Konflikte können bestehen.';

  @override
  String get loadOrderDirection =>
      'Ladereihenfolge: zuerst niedrige Priorität; spätere Mods haben eine höhere beabsichtigte Priorität.';

  @override
  String get footprintCoverageScope =>
      'Die Abdeckung beschreibt nur erkannte Konfliktziele; sie beweist keine Laufzeitpriorität.';

  @override
  String get footprintCoverageExact =>
      'Exakt — die Konfliktzielliste der Komponente ist vollständig.';

  @override
  String get footprintCoveragePartial =>
      'Teilweise — aufgeführte Konfliktziele sind bekannt, aber die Komponente kann weitere betreffen.';

  @override
  String get footprintCoverageAdvisory =>
      'Hinweis — aufgeführte Ziele sind Anhaltspunkte, kein vollständiger Nachweis.';

  @override
  String get footprintCoverageOpaque =>
      'Undurchsichtig — die Konfliktziele der Komponente sind unbekannt.';

  @override
  String get footprintCoverageExactLabel => 'Exakt';

  @override
  String get footprintCoveragePartialLabel => 'Teilweise';

  @override
  String get footprintCoverageAdvisoryLabel => 'Hinweis';

  @override
  String get footprintCoverageOpaqueLabel => 'Undurchsichtig';

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
