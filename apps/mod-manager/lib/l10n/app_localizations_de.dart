// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager kann nicht starten';

  @override
  String get coreDllMissingMessage =>
      'Eine benötigte Programmdatei fehlt (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Eine benötigte Programmdatei konnte nicht geladen werden.';

  @override
  String get coreVerificationFailedMessage =>
      'Eine benötigte Programmdatei konnte nicht überprüft werden.';

  @override
  String get coreManagerTooOldMessage =>
      'Die Programmdateien sind neuer als der Mod Manager. Aktualisiere den Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Die Programmdateien sind älter als der Mod Manager. Installiere den Mod Manager neu.';

  @override
  String get coreCommandsMissingMessage =>
      'Den Programmdateien fehlen Funktionen, die dieser Mod Manager braucht.';

  @override
  String get coreBlockedRepairHint =>
      'Installiere oder repariere den Mod Manager und starte ihn dann neu.';

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
  String get preflightAttention =>
      'Bevor Mods geändert werden können, ist noch etwas zu tun.';

  @override
  String get preflightGameRunning =>
      'Gothic läuft noch. Schließe das Spiel, bevor du Mods änderst.';

  @override
  String get managerOperationFailed => 'Der Vorgang ist fehlgeschlagen.';

  @override
  String get libraryOperationFailed =>
      'Die Mod-Liste konnte nicht geladen werden.';

  @override
  String get conflictsUnavailable => 'Konflikte konnten nicht geprüft werden.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Angewendete Mods: $applied. Warnungen: $warnings.';
  }

  @override
  String get modDetailKind => 'Typ';

  @override
  String get modDetailVersion => 'Version';

  @override
  String get modDetailAuthor => 'Autor';

  @override
  String get modDetailSource => 'Quelle';

  @override
  String get modDetailImported => 'Importiert';

  @override
  String get componentLocalization => 'Texte';

  @override
  String get componentAudio => 'Sound';

  @override
  String get componentAngelScript => 'Skripte';

  @override
  String get componentTexture => 'Texturen';

  @override
  String get componentGameFiles => 'Spieldateien';

  @override
  String get componentVoice => 'Sprachausgabe';

  @override
  String get componentKindLocalizationPatch => 'Textänderungen';

  @override
  String get componentKindAudioPatch => 'Soundänderungen';

  @override
  String get componentKindAngelScriptPatch => 'Skriptänderungen';

  @override
  String get componentKindTexturePatch => 'Texturänderungen';

  @override
  String get componentKindLoosePak => 'PAK-Datei';

  @override
  String get componentKindTriplet => 'IoStore-Container';

  @override
  String get componentKindUe4ssLua => 'UE4SS-Skript';

  @override
  String get componentKindRawFile => 'Datei';

  @override
  String get componentKindFilePatch => 'Ersetzte Spieldatei';

  @override
  String get componentKindPakFilePatch => 'Spieldatei aus einer ~mods-PAK';

  @override
  String get componentKindVoiceArchivePatch => 'Sprachausgabe';

  @override
  String get rawTargetGameText => 'Alle Spieltexte';

  @override
  String get rawTargetGameScripts => 'Alle Spielskripte';

  @override
  String get rawTargetSoundBank => 'Sound-Bank';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Sound-Bank: $name';
  }

  @override
  String get conflictKindLocalization => 'Texte';

  @override
  String get conflictKindAudio => 'Sound';

  @override
  String get conflictKindAsset => 'Spieldaten';

  @override
  String get conflictKindCdo => 'Objektwerte';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (unklar)';

  @override
  String get conflictKindScriptModule => 'Spielskript';

  @override
  String get conflictKindVoiceArchive => 'Sprachausgabe';

  @override
  String get conflictKindRawFile => 'Datei';

  @override
  String get conflictKindLooseFile => 'Spieldatei';

  @override
  String get preflightUnavailable =>
      'Die Spielinstallation konnte nicht geprüft werden.';

  @override
  String get preflightRetry => 'Erneut prüfen';

  @override
  String get preflightReviewStatus => 'Status anzeigen';

  @override
  String get preflightReviewRecovery => 'Hilfe anzeigen';

  @override
  String get installRecoveryTitle => 'Unterbrochene Installation';

  @override
  String get installRecoveryBody =>
      'GORE hat Reste einer Installation oder einer Skript-Kompilierung gefunden. Der Vorgang läuft vielleicht noch, oder er ist beendet und hat das hier zurückgelassen. GORE kann das nicht sicher allein aufräumen.';

  @override
  String get installRecoverySteps =>
      'Falls der Vorgang noch läuft, warte, bis er fertig ist – beende ihn nicht und lösche keine Dateien. Wenn du sicher bist, dass nichts mehr läuft, folge der README.txt im Ordner unten und prüfe erneut. Ist kein Ordner genannt oder bist du unsicher, lass alles so und hole dir Hilfe.';

  @override
  String get installRecoveryEvidence => 'Was GORE gefunden hat';

  @override
  String get managerRecoveryTitle => 'Unterbrochenen Vorgang reparieren';

  @override
  String get managerRecoveryConfirm =>
      'GORE hat einen unterbrochenen Vorgang gefunden und kann das Spiel in einen sauberen Zustand zurückbringen. Deine Spielstände werden dabei nie angefasst.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Es gab nichts mehr zu reparieren. Der Status wurde neu geprüft.';

  @override
  String get managerRecoveryBusy =>
      'Der Vorgang läuft wieder. Es wurde nichts verändert – warte, bis er fertig ist.';

  @override
  String get managerRecoveryLockCleared =>
      'Der unterbrochene Vorgang hatte noch nichts verändert. Er wurde aufgeräumt.';

  @override
  String get managerRecoveryRestoredPristine =>
      'Die Änderung wurde zurückgenommen. Das Spiel ist wieder im vorherigen Zustand.';

  @override
  String get managerRecoveryApplyPreserved =>
      'Das Anwenden war schon fertig. Es ging nichts verloren.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'Das Entfernen war schon fertig. Reste wurden aufgeräumt.';

  @override
  String get managerRecoveryCompileRequired =>
      'Das gehört zu einer Skript-Kompilierung, deshalb wurde nichts verändert. Öffne die Reparaturhilfe.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE konnte den unterbrochenen Vorgang nicht sicher prüfen. Es wurde nichts verändert.';

  @override
  String get managerRecoveryFailed =>
      'Die Reparatur konnte nicht abgeschlossen werden. Prüfe den Status, bevor du es erneut versuchst.';

  @override
  String get statusUnknown => 'Unbekannt';

  @override
  String statusDetailsTitle(String status) {
    return 'Status: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Details anzeigen: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Wähle zuerst in den Einstellungen deine Gothic-Installation.';

  @override
  String get statusDetailsNoDeployment =>
      'Zurzeit sind keine Mods im Spiel installiert.';

  @override
  String get statusDetailsInSyncDescription =>
      'Im Spiel sind genau die Mods, die hier angehakt sind.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods im Spiel';

  @override
  String get statusDetailsChangesDescription =>
      'Deine Auswahl unterscheidet sich von dem, was im Spiel ist.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Jetzt im Spiel';

  @override
  String get statusDetailsAfterApply => 'Nach dem Anwenden';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Das Spiel wurde aktualisiert und hat Mod-Dateien überschrieben. Wende erneut an, um sie zurückzuholen.';

  @override
  String get statusDetailsDriftedFiles => 'Betroffene Dateien';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio hat gerade Mods in diesem Spiel. Übernimm das Spiel, bevor der Manager deine anwendet.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio-Mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio hat keinen Namen gemeldet.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Ein Vorgang wurde unterbrochen. Repariere ihn, bevor du Mods änderst.';

  @override
  String get statusDetailsUnknownDescription =>
      'Der Status konnte nicht gelesen werden. Aktualisiere zuerst.';

  @override
  String get statusDetailsUnavailable => 'Keine Details verfügbar.';

  @override
  String get statusDetailsEmptyLoadout => 'Keine Mods.';

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
  String get recoveryAction => 'Reparieren';

  @override
  String get recoveryRequiredConfirm =>
      'Den unterbrochenen Vorgang reparieren und halb installierte Dateien entfernen?';

  @override
  String get statusRecoveryRequired => 'Reparatur nötig';

  @override
  String get statusDetailsOwnershipTitle => 'Von GORE verwaltete Dateien';

  @override
  String get statusDetailsOwnershipDescription =>
      'Beim Anwenden aufgezeichnet – kein Nachweis, dass die Dateien noch da sind.';

  @override
  String get statusDetailsOwnershipLive => 'Ersetzte Spieldateien';

  @override
  String get statusDetailsOwnershipBackups => 'Sicherungen der Originale';

  @override
  String get statusDetailsOwnershipAdditive => 'Hinzugefügte Mod-Dateien';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS-Mod-Verzeichnisse';

  @override
  String get statusDetailsOwnershipRecovery => 'Reparaturdateien';

  @override
  String get statusDetailsOwnershipEmpty => 'Hier ist nichts aufgezeichnet.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '$shown von $total Pfaden angezeigt.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Einstellungen';

  @override
  String get settingsGameExe => 'Gothic-Installation';

  @override
  String get settingsGameExePick => 'Auswählen…';

  @override
  String get settingsLanguage => 'Sprache';

  @override
  String get libraryEmptyTitle => 'Noch keine Mods';

  @override
  String get libraryEmptyBody =>
      'Importiere einen Ordner oder eine Mod-Datei, um loszulegen.';

  @override
  String get detailEmptyHint =>
      'Wähle eine Mod aus, um zu sehen, was sie ändert.';

  @override
  String get settingsAdvanced => 'Erweiterte Details';

  @override
  String get settingsAdvancedHint =>
      'Zeigt die technische Seite: betroffene Einträge, wie verlässlich die Konfliktprüfung ist, und die von GORE verwalteten Dateien.';

  @override
  String get updatesTitle => 'Updates';

  @override
  String get checkForUpdatesAutomatically => 'Automatisch nach Updates suchen';

  @override
  String get checkForUpdatesNow => 'Jetzt nach Updates suchen';

  @override
  String get updatesPortableNotice =>
      'Die portable Version öffnet die Download-Seite im Browser. Ersetze deine vorhandenen Dateien durch den neuen Download.';

  @override
  String get updateCheckFailed =>
      'Suche nach Updates fehlgeschlagen. Bitte später erneut versuchen.';

  @override
  String get updateUpToDate => 'Du verwendest die neueste Version.';

  @override
  String get updateAvailableTitle => 'Update verfügbar';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'Version $version ist verfügbar. Du hast $current.';
  }

  @override
  String get updateLater => 'Später';

  @override
  String get updateDownload => 'Herunterladen';

  @override
  String updateOpenFailed(String url) {
    return 'Die Download-Seite konnte nicht geöffnet werden. Du erreichst sie unter $url';
  }

  @override
  String get statusInSync => 'Aktuell';

  @override
  String get statusChangesPending => 'Nicht angewendet';

  @override
  String get statusGameUpdated => 'Spiel wurde aktualisiert';

  @override
  String get statusStudioDeploy => 'Mod Studio aktiv';

  @override
  String get statusNothingDeployed => 'Keine Mods im Spiel';

  @override
  String get actionImport => 'Importieren';

  @override
  String get actionApply => 'Anwenden';

  @override
  String get actionStartGame => 'Spiel starten';

  @override
  String get startGameTooltip =>
      'Gothic mit den aktuell im Spiel installierten Mods starten';

  @override
  String get startGameFailed =>
      'Gothic konnte nicht gestartet werden. Prüfe die Spielinstallation in den Einstellungen.';

  @override
  String get commonCancel => 'Abbrechen';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Ordner importieren…';

  @override
  String get importFile => 'Datei importieren…';

  @override
  String importOutcomeCreated(String name) {
    return '„$name“ hinzugefügt.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '„$name“ aktualisiert.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '„$name“ ist schon in deiner Liste.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Keine vorhandene Mod passte dazu.',
      'source': 'Übereinstimmung anhand derselben Importquelle.',
      'content': 'Übereinstimmung anhand nachweislich identischer Inhalte.',
      'entry_id': 'Übereinstimmung anhand der Mod-ID.',
      'other': 'Details zur Übereinstimmung sind nicht verfügbar.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Das passt zu mehreren Mods, die du schon hast. Entferne die Duplikate und versuche es erneut.';

  @override
  String get importRefusalIdentityConflict =>
      'Quelle und Inhalt passen zu zwei verschiedenen Mods, die du schon hast. Kläre das und versuche es erneut.';

  @override
  String get importFailed =>
      'Das konnte nicht importiert werden. Unterstützt werden Ordner, ZIP-Archive und einzelne Mod-Dateien (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Entpacke .7z oder .rar zuerst und importiere dann den Ordner. Vielleicht wurde die Mod trotzdem hinzugefügt oder aktualisiert – aktualisiere die Liste, bevor du es erneut versuchst.';

  @override
  String get importPickerFailed =>
      'Die Dateiauswahl konnte nicht geöffnet werden. Es wurde nichts importiert.';

  @override
  String get importOutcomeUnknown =>
      'Das Ergebnis ist unklar. Aktualisiere, um deine Mod-Liste zu prüfen.';

  @override
  String get applyTooltip => 'Die angehakten Mods im Spiel installieren';

  @override
  String get undeployAllAction => 'Alle aus dem Spiel entfernen';

  @override
  String get undeployAllConfirm =>
      'Alle vom Manager installierten Mods aus dem Spiel entfernen?';

  @override
  String get takeOverTitle => 'Mod Studio ist aktiv';

  @override
  String get takeOverBody =>
      'Mod Studio hat gerade eine Mod im Spiel. Übernehmen, damit der Manager deine Auswahl anwenden kann?';

  @override
  String get takeOverAction => 'Übernehmen';

  @override
  String get refreshAction => 'Aktualisieren';

  @override
  String conflictsTitle(int count) {
    return 'Konflikte ($count)';
  }

  @override
  String get conflictWinner => 'gewinnt';

  @override
  String get noConflicts => 'Keine Konflikte gefunden.';

  @override
  String get conflictCoverageIncomplete =>
      'Manche Mods lassen sich nicht vollständig prüfen – es kann weitere Konflikte geben.';

  @override
  String get loadOrderDirection =>
      'Mods weiter unten in der Liste überschreiben die darüber.';

  @override
  String get footprintCoverageScope =>
      'Aufgelistet sind nur bekannte Konfliktziele. Was im Spiel passiert, ist damit nicht garantiert.';

  @override
  String get footprintTargetsExact =>
      'Betroffene Einträge – vollständige Liste:';

  @override
  String get footprintTargetsPartial =>
      'Betroffene Einträge – es können mehr sein:';

  @override
  String get footprintTargetsAdvisory =>
      'Vermutlich betroffene Einträge – Anhaltspunkte, kein Nachweis:';

  @override
  String get footprintTargetsOpaque =>
      'GORE kann nicht erkennen, was hier geändert wird.';

  @override
  String get conflictsUnverified =>
      'Konflikte unbekannt – bitte aktualisieren.';

  @override
  String get componentsTitle => 'Was diese Mod ändert';

  @override
  String targetsMore(int count) {
    return '+$count weitere';
  }

  @override
  String get removeModDeploymentHint =>
      'Das entfernt sie nur aus deiner Liste. Ist sie im Spiel installiert, wähle danach „Anwenden“.';

  @override
  String removeModSuccess(String name) {
    return '„$name“ entfernt.';
  }

  @override
  String removeModFailed(String name) {
    return '„$name“ konnte nicht entfernt werden.';
  }

  @override
  String removeModPartialFailure(String name) {
    return '„$name“ entfernt, aber die Liste konnte nicht vollständig aktualisiert werden.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Es konnte nicht bestätigt werden, ob „$name“ entfernt wurde.';
  }

  @override
  String get libraryStateUnknown =>
      'Die Mod-Liste ist nicht aktuell. Aktualisiere, bevor du Mods änderst oder anwendest.';

  @override
  String get removeModAction => 'Entfernen';

  @override
  String removeModConfirm(String name) {
    return '„$name“ aus deiner Liste entfernen?';
  }

  @override
  String get errorSetGamePath =>
      'Wähle zuerst in den Einstellungen deine Gothic-Installation.';

  @override
  String applyReportApplied(int count) {
    return '$count Mods angewendet.';
  }

  @override
  String get modDisabledHint => 'Deaktiviert';

  @override
  String get kindGoremod => 'GORE-Bundle';

  @override
  String get kindTriplet => 'IoStore-Mod';

  @override
  String get kindPak => 'PAK-Mod';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Ganze Dateien ersetzt';

  @override
  String get kindMixed => 'Gemischt';

  @override
  String get sevHard => 'Konflikt';

  @override
  String get sevSoft => 'Warnung';

  @override
  String get sevInfo => 'Hinweis';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Über';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => 'Anzeigegröße';

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
