// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Änderungen';

  @override
  String get tabSettings => 'Einstellungen';

  @override
  String get tabDialogs => 'Dialoge';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Texturen';

  @override
  String get tabScripts => 'Scripte';

  @override
  String get changesAll => 'Alle';

  @override
  String get sectionItemValues => 'Item-Werte';

  @override
  String get sectionLocalizedText => 'Lokalisierte Texte';

  @override
  String get audioCatCreatures => 'Kreaturen';

  @override
  String get audioCatObjects => 'Objekte';

  @override
  String get audioCatMagic => 'Magie';

  @override
  String get audioCatMovement => 'Bewegung';

  @override
  String get audioCatWorld => 'Welt';

  @override
  String get audioCatAction => 'Aktionen';

  @override
  String get audioCatCombat => 'Kampf';

  @override
  String get audioCatPhysics => 'Physik';

  @override
  String get audioCatItems => 'Items';

  @override
  String get audioCatUi => 'UI';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Unterwasser';

  @override
  String get audioCatVision => 'Visionen';

  @override
  String get audioCatDialog => 'Dialog';

  @override
  String get audioCatOther => 'Sonstige';

  @override
  String get gameExecutable => 'Spiel-Programmdatei';

  @override
  String get gameExecutableSubtitle =>
      'Pfad zur .exe des Spiels. Wird zur automatischen Erkennung der lokalisierten Texte und der Spielinstallation genutzt.';

  @override
  String get gameExecutableNotSet => 'Nicht gesetzt';

  @override
  String get chooseGameExecutable => 'Auswählen…';

  @override
  String get settingsDataSourceSection => 'Spieldaten';

  @override
  String get settingsLocalizationSection => 'Lokalisierte Texte';

  @override
  String get extractLocalizedText => 'Lokalisierte Texte extrahieren';

  @override
  String get lightMode => 'Heller Modus';

  @override
  String get darkMode => 'Dunkler Modus';

  @override
  String get language => 'Sprache';

  @override
  String get exportMod => 'Mod exportieren';

  @override
  String exportModWithCount(int count) {
    return 'Mod exportieren ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Wähle einen Gegenstand, um seine Felder zu bearbeiten.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Spieldaten: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Spieldaten: mitgeliefert';

  @override
  String get loadGameDataDump => 'Spieldaten-Dump laden…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json aus der gore-dump-Mod';

  @override
  String get useBundledData => 'Mitgelieferte Daten verwenden';

  @override
  String get alreadyBundled => 'bereits mitgeliefert';

  @override
  String get gameDataFileGroupLabel => 'Spieldaten';

  @override
  String get minimize => 'Minimieren';

  @override
  String get restore => 'Wiederherstellen';

  @override
  String get maximize => 'Maximieren';

  @override
  String get close => 'Schließen';

  @override
  String get about => 'Über';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE-Mitwirkende';

  @override
  String get aboutLicense => 'Lizenziert unter der MIT-Lizenz.';

  @override
  String get categoryMeleeWeapons => 'Nahkampfwaffen';

  @override
  String get categoryRangedWeapons => 'Fernkampfwaffen';

  @override
  String get categoryAmmunition => 'Munition';

  @override
  String get categoryRunes => 'Runen';

  @override
  String get categorySpellScrolls => 'Zauberschriftrollen';

  @override
  String get categoryFoodAndPotions => 'Nahrung & Tränke';

  @override
  String get categoryMiscellaneous => 'Verschiedenes';

  @override
  String get categoryAmulets => 'Amulette';

  @override
  String get categoryRings => 'Ringe';

  @override
  String get categoryAnimalTrophies => 'Tiertrophäen';

  @override
  String get categoryWritings => 'Schriftstücke';

  @override
  String get categoryMissionItems => 'Questgegenstände';

  @override
  String get categoryKeys => 'Schlüssel';

  @override
  String get categoryOther => 'Sonstiges';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Gegenstände suchen';

  @override
  String get noItemsMatch => 'Keine passenden Gegenstände';

  @override
  String failedToLoadCatalog(String error) {
    return 'Katalog konnte nicht geladen werden: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Ausstehende Änderungen ($count)';
  }

  @override
  String get clearAll => 'Alle löschen';

  @override
  String get noPendingOverrides =>
      'Keine ausstehenden Änderungen.\nBearbeite Felder, um welche hinzuzufügen.';

  @override
  String get removeOverride => 'Änderung entfernen';

  @override
  String get searchChanges => 'Änderungen durchsuchen';

  @override
  String get noChangesMatch => 'Keine Änderungen gefunden';

  @override
  String get clearSection => 'Gruppe leeren';

  @override
  String get modName => 'Mod-Name';

  @override
  String get loadDelayLabel => 'Ladeverzögerung (ms, 0 = sofort)';

  @override
  String get noFolderSelected => 'Kein Ordner ausgewählt';

  @override
  String get chooseFolder => 'Ordner wählen';

  @override
  String get packageAsZip => 'Als .zip verpacken';

  @override
  String get cancel => 'Abbrechen';

  @override
  String get export => 'Exportieren';

  @override
  String get exportHere => 'Hierher exportieren';

  @override
  String get mustBeNonNegativeInteger =>
      'Muss eine nicht-negative Ganzzahl sein';

  @override
  String get extractingLocalizedText =>
      'Lokalisierte Spieltexte werden extrahiert…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extraktion der lokalisierten Texte abgebrochen.';

  @override
  String get localizedTextExtracted => 'Lokalisierte Texte extrahiert.';

  @override
  String get extractionFailed => 'Extraktion fehlgeschlagen.';

  @override
  String get localizationCacheFileGroupLabel => 'Lokalisierungs-Cache';

  @override
  String get extractLocalizedTextQuestion =>
      'Lokalisierte Spieltexte extrahieren?';

  @override
  String get extractLocalizedTextBody =>
      'Lokalisierte Spieltexte wurden noch nicht extrahiert. Jetzt aus deiner Spielinstallation extrahieren? (optional)';

  @override
  String get notNow => 'Nicht jetzt';

  @override
  String get extract => 'Extrahieren';

  @override
  String get validationRequired => 'Erforderlich';

  @override
  String get validationMustBeWholeNumber => 'Muss eine ganze Zahl sein';

  @override
  String get validationMustBeNumber => 'Muss eine Zahl sein';

  @override
  String get validationMustBeFinite => 'Muss eine endliche Zahl sein';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Muss ≥ $min sein';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Muss ≤ $max sein';
  }

  @override
  String get validationMustBeBool => 'Muss true oder false sein';

  @override
  String validationMustBeOneOf(String options) {
    return 'Muss einer von: $options sein';
  }

  @override
  String get modNameRequired => 'Erforderlich';

  @override
  String get modNameControlCharacters => 'Darf keine Steuerzeichen enthalten';

  @override
  String get modNamePathSeparators => 'Darf keine Pfadtrennzeichen enthalten';

  @override
  String get modNameNotAFolderName => 'Kein gültiger Ordnername';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs in $languageCount Sprachen extrahiert';
  }

  @override
  String get managerDeployActive =>
      'Ein mod-manager-Loadout ist aktiv. Bitte zuerst in gore-manager undeployen.';

  @override
  String get projectOpenLegacy => 'Legacy-Projekt öffnen…';

  @override
  String get projectOpenManagedRevision3 => 'Mod-Studio-Projekt öffnen…';

  @override
  String get projectVerifyCurrentHead => 'Aktuellen Head verifizieren';

  @override
  String get projectManagedRevision3Title => 'Mod-Studio-Projekt';

  @override
  String get projectClose => 'Projekt schließen';

  @override
  String projectCloseFailed(String error) {
    return 'Das Projekt konnte nicht geschlossen werden: $error';
  }

  @override
  String get projectManagedRevision3IdentityOnly =>
      'Diese Oberfläche zeigt derzeit ausschließlich die verifizierte Projektidentität. Strg+S öffnet den exakten aktuellen Head erneut und verifiziert ihn; Legacy-Editoren, Build/Deploy und Speichern unter sind nicht verfügbar.';

  @override
  String get projectRoot => 'Projektordner';

  @override
  String get projectId => 'Projekt-ID';

  @override
  String get projectRevision => 'Projektrevision';

  @override
  String get projectHeadSha256 => 'Head-SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot-Bytes';

  @override
  String get projectNoCurrent => 'Kein aktuelles Projekt';

  @override
  String get projectManagedRevision3Opened => 'Mod-Studio-Projekt geöffnet.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod-Studio-Projekt konnte nicht geöffnet werden: $error';
  }

  @override
  String get projectManagedRevision3Verified =>
      'Projekt-Checkpoint verifiziert.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Projekt-Checkpoint konnte nicht verifiziert werden: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Die Verifizierung des exakten Heads konnte nicht sicher abgeschlossen werden. Diese Sitzung muss jetzt wiederhergestellt werden; weitere Verifizierungen sind gesperrt. Schließe Mod Studio und öffne dieses Projekt danach erneut.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Die Verifizierung ist gesperrt, bis das verwaltete Projekt erneut geöffnet wurde.';

  @override
  String get projectTransitionCleanupWarning =>
      'Das neue Projekt ist geöffnet, aber die vorherige Projektsitzung konnte nicht vollständig bereinigt werden. Es wird kein erneuter Bereinigungsversuch durchgeführt. Starte Mod Studio neu, bevor du das vorherige Projekt erneut öffnest.';

  @override
  String get projectNewManagedRevision3 => 'Neues verwaltetes Mod-Projekt…';

  @override
  String get projectNewLegacy => 'Neues Legacy-Projekt';

  @override
  String get projectCreateGamePathRequired =>
      'Lege vor dem Erstellen eines Mod-Projekts unter Einstellungen den Pfad zu Gothic 1 Remake fest.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Verwaltetes Mod-Projekt hier erstellen';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Verwaltetes Mod-Projekt $projectId erstellt';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Verwaltetes Mod-Projekt konnte nicht erstellt werden: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Mod-Projekt erstellen';

  @override
  String get projectCreateNameLabel => 'Projektname';

  @override
  String get projectCreateNameHelper => 'Der in Mod Studio angezeigte Name.';

  @override
  String get projectCreateVersionLabel => 'Version';

  @override
  String get projectCreateVersionHelper =>
      'Eine Startversion, zum Beispiel 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Dein Name oder der Name deines Mod-Teams.';

  @override
  String get projectCreateLocalesLabel => 'Bearbeitungssprachen';

  @override
  String get projectCreateLocalesHelper =>
      'Kommagetrennte kanonische Tags, zum Beispiel: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Dies erstellt ein leeres, verwaltetes Offline-Projekt. Dabei wird keine Mod gebaut, bereitgestellt oder ausgeführt, und Spiel- sowie Speicherdateien bleiben unverändert.';

  @override
  String get projectCreateSubmit => 'Projekt erstellen';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label ist erforderlich.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label darf nicht mit Leerraum beginnen oder enden.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label darf keine Steuerzeichen enthalten.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label enthält fehlerhaften Text.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label überschreitet das UTF-8-Limit von $maxBytes Byte.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Gib mindestens eine Bearbeitungssprache ein.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Entferne den leeren Eintrag für die Bearbeitungssprache.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Verwende höchstens $maxLocales Bearbeitungssprachen.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'Das Locale „$locale“ muss begrenztes ASCII sein.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'Das Locale „$locale“ benötigt eine kleingeschriebene Sprache mit 2–8 Buchstaben.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'Das Locale „$locale“ enthält ein ungültiges Segment.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'Das Locale „$locale“ ist nicht kanonisch; verwende „$canonical“.';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Übersicht';

  @override
  String get managedWorkspaceContentLabel => 'Inhalte';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Diese Mod';

  @override
  String get managedWorkspaceHomeLabel => 'Start';

  @override
  String get managedWorkspaceStoryLabel => 'Story';

  @override
  String get managedWorkspaceWorldLabel => 'Welt';

  @override
  String get managedWorkspaceLocalizationVoiceLabel =>
      'Lokalisierung & Vertonung';

  @override
  String get managedWorkspaceValidateTestLabel => 'Prüfen & Testen';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Bauen & Veröffentlichen';

  @override
  String get managedWorkspaceHistoryLabel => 'Verlauf';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Einstellungen & Expertenmodus';

  @override
  String get managedProjectHistoryTitle => 'Projektverlauf';

  @override
  String get managedProjectHistoryDescription =>
      'Kehre zu einer früheren Projektversion zurück, ohne die danach entstandenen Versionen zu löschen.';

  @override
  String get managedProjectHistoryBoundary =>
      'Der Verlauf ändert nur dieses verwaltete Projekt. Spielinstallation und Spielstände bleiben unverändert.';

  @override
  String get managedProjectHistoryRefresh => 'Projektverlauf aktualisieren';

  @override
  String get managedProjectHistoryLoading => 'Projektverlauf wird geladen…';

  @override
  String get managedProjectHistoryLoadFailed =>
      'Der Projektverlauf konnte nicht geladen werden';

  @override
  String get managedProjectHistoryRetry => 'Erneut versuchen';

  @override
  String get managedProjectHistoryCurrentVersion => 'Aktuelle Version';

  @override
  String get managedProjectHistoryPreviousVersions => 'Frühere Versionen';

  @override
  String get managedProjectHistoryUndo => 'Letzte Änderung rückgängig machen';

  @override
  String get managedProjectHistoryRestoreVersion =>
      'Diese Version wiederherstellen';

  @override
  String get managedProjectHistoryRestoreTitle =>
      'Projektversion wiederherstellen?';

  @override
  String managedProjectHistoryRestoreBody(int revision, int nextRevision) {
    return 'Der Inhalt aus Revision $revision wird als neue Revision $nextRevision gespeichert. Der aktuelle Stand bleibt im Verlauf erhalten.';
  }

  @override
  String get managedProjectHistoryRestoreBoundary =>
      'Nur das Projekt wird geändert. Spielinstallation und Spielstände bleiben unverändert.';

  @override
  String get managedProjectHistoryCancel => 'Abbrechen';

  @override
  String get managedProjectHistoryRestore => 'Wiederherstellen';

  @override
  String get managedProjectHistoryRestoring =>
      'Projektversion wird wiederhergestellt…';

  @override
  String get managedProjectHistoryRestoreFailed =>
      'Die Projektversion konnte nicht sicher wiederhergestellt werden. Aktualisiere den Verlauf vor einem neuen Versuch.';

  @override
  String managedProjectHistoryRestoreSucceeded(int revision) {
    return 'Revision $revision wurde als neue Projektversion wiederhergestellt.';
  }

  @override
  String get managedProjectHistoryEmpty =>
      'Es wurden noch keine früheren Projektversionen aufgezeichnet.';

  @override
  String managedProjectHistoryRecordingStartsAt(int revision) {
    return 'Die Verlaufsaufzeichnung beginnt bei Revision $revision; ältere Stände wurden nicht aus dem Speicher geraten.';
  }

  @override
  String get managedProjectHistoryTruncated =>
      'Ältere Projektversionen sind aus dem Verlauf abgelaufen. Jeder hier angezeigte Stand wird vom aktuellen Projektverlauf weiterhin aufbewahrt und authentifiziert.';

  @override
  String managedProjectHistoryRevision(int revision) {
    return 'Revision $revision';
  }

  @override
  String get managedProjectHistoryCurrentBadge => 'Aktuell';

  @override
  String get managedProjectHistoryDirtyBlocked =>
      'Beende oder verwirf die offene Textbearbeitung, bevor du eine andere Projektversion wiederherstellst.';

  @override
  String get managedProjectHistoryBusy =>
      'Eine andere Projektaktion läuft noch.';

  @override
  String get managedProjectHistoryUnavailable =>
      'Diese verwaltete Projektsitzung unterstützt keinen authentifizierten Verlauf.';

  @override
  String get managedSectionStoryDescription => 'NPCs, Quests und Dialoge.';

  @override
  String get managedStoryWorkspaceLoading =>
      'Aktuelle Story-Entwürfe werden geöffnet…';

  @override
  String get managedStoryWorkspaceAuthorityNotice =>
      'Hier siehst du ausschließlich NPC- und Quest-Entwürfe aus diesem Projekt. Die Build-Bereitschaft wurde noch nicht bewertet; das Laufzeitverhalten ist weiterhin nicht qualifiziert.';

  @override
  String get managedStoryWorkspaceSearchHint =>
      'NPC- und Questnamen, Ziele, Sprecher oder IDs durchsuchen';

  @override
  String get managedStoryWorkspaceCreatingNpc => 'NPC-Entwurf wird erstellt…';

  @override
  String get managedStoryWorkspaceCreatingQuest =>
      'Quest-Entwurf wird erstellt…';

  @override
  String get managedStoryWorkspaceCreateQuestOpening =>
      'Quest + erste Dialogzeile erstellen';

  @override
  String get managedStoryWorkspaceCreatingQuestOpening =>
      'Quest + erste Dialogzeile wird erstellt…';

  @override
  String get managedStoryWorkspaceCreateAdvanced =>
      'Erweiterte Erstelloptionen';

  @override
  String get managedStoryWorkspaceCreateQuestAdvanced =>
      'Nur Quest-Entwurf erstellen (erweitert)';

  @override
  String get managedStoryWorkspaceMutationRequiresReopen =>
      'Öffne dieses Projekt erneut, bevor du Story-Inhalte änderst.';

  @override
  String get managedStoryWorkspaceMutationDirtyBlocked =>
      'Speichere oder verwirf die offenen Lokalisierungsänderungen, bevor du Story-Inhalte änderst.';

  @override
  String get managedStoryWorkspaceEmpty =>
      'Noch keine NPC- oder Quest-Entwürfe';

  @override
  String get managedStoryWorkspaceNoMatches =>
      'Keine NPC- oder Quest-Entwürfe passen zu dieser Suche';

  @override
  String get managedStoryWorkspaceSelectDraft =>
      'Wähle einen NPC- oder Quest-Entwurf aus, um weiterzuarbeiten';

  @override
  String get managedStoryWorkspaceLoadErrorTitle =>
      'Story-Entwürfe konnten nicht geöffnet werden';

  @override
  String get managedStoryWorkspaceCheckpointMismatch =>
      'Das Projekt hat sich während des Ladens der Story geändert. Aktualisiere den exakt aktuellen Checkpoint und versuche es erneut.';

  @override
  String get managedStoryWorkspacePublishedSelectionStale =>
      'Der gespeicherte Story-Entwurf konnte nicht in seiner exakten Projektrevision ausgewählt werden. Prüfe die aktuelle Story-Liste, bevor du weiterarbeitest.';

  @override
  String managedStoryWorkspaceCheckpointSummary(int count, int revision) {
    return 'NPC- und Quest-Entwürfe: $count · Projektrevision $revision';
  }

  @override
  String managedStoryWorkspaceLoadErrorDetails(String error) {
    return 'Die exakt aktuelle Story-Ansicht konnte nicht gelesen werden: $error';
  }

  @override
  String managedStoryWorkspaceCreateErrorDetails(String error) {
    return 'Der Story-Entwurf konnte nicht erstellt werden: $error';
  }

  @override
  String managedStoryWorkspaceDetailsSheetLabel(String entityName) {
    return 'Story-Details für $entityName';
  }

  @override
  String get managedStoryWorkspaceRemovePairUnavailable =>
      'Dieser Entwurf ist kein exakt entfernbares Paar aus Entwurf und generiertem Skript.';

  @override
  String get managedStoryWorkspaceRemoveBusy =>
      'Eine andere Story-Aktion läuft noch.';

  @override
  String get managedStoryWorkspaceRemoveRequiresReopen =>
      'Öffne dieses verwaltete Projekt erneut, bevor du einen Entwurf entfernst.';

  @override
  String managedStoryWorkspaceRemoveBlocked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Zuerst müssen $count eingehende Projektreferenzen entfernt werden.',
      one: 'Zuerst muss 1 eingehende Projektreferenz entfernt werden.',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkspaceRemoveDialogTitle =>
      'Entwurf aus dem Projekt entfernen?';

  @override
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  ) {
    return 'Der Entwurf ‚$draftName‘ und sein eindeutig zugeordnetes generiertes Skript ‚$scriptName‘ werden gemeinsam entfernt.';
  }

  @override
  String get managedStoryWorkspaceRemoveNoUndo =>
      'Diese Aktion kann in Version 1 nicht rückgängig gemacht werden.';

  @override
  String get managedStoryWorkspaceRemoveBoundary =>
      'Nur die aktuelle Projektregistrierung wird geändert. Spielinstallation und Spielstände bleiben unverändert.';

  @override
  String get managedStoryWorkspaceRemoveCancel => 'Abbrechen';

  @override
  String get managedStoryWorkspaceRemoveConfirm => 'Entwurf entfernen';

  @override
  String get managedStoryWorkspaceRemoveBlockedTitle =>
      'Der Entwurf wird noch referenziert';

  @override
  String get managedStoryWorkspaceRemoveBlockedDescription =>
      'Öffne jede Quelle unten und entferne ihre Projektreferenz, bevor du es erneut versuchst.';

  @override
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  ) {
    return '$sourceName · $role';
  }

  @override
  String get managedStoryWorkspaceRemoveOpenBlocker =>
      'Referenzierende Quelle öffnen';

  @override
  String get managedStoryWorkspaceRemoveBlockedClose => 'Schließen';

  @override
  String managedStoryWorkspaceRemoveSucceeded(String draftName) {
    return '‚$draftName‘ und das generierte Skript wurden aus dem Projekt entfernt. Spieldateien und Spielstände wurden nicht geändert.';
  }

  @override
  String managedStoryWorkspaceRemoveError(String error) {
    return 'Der Entwurf wurde nicht entfernt. Die Story-Ansicht wurde ohne automatischen Neuversuch aktualisiert: $error';
  }

  @override
  String get managedSectionWorldDescription =>
      'Weltplatzierung und zugehörige Arbeitsabläufe sind geplant.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Schreibe und übersetze Projektdialoge und prüfe Aufnahmen, Auswahl und Ziel jeder Sprache im selben Arbeitsbereich.';

  @override
  String get managedLocalizationProjectTextsLabel => 'Projekttexte';

  @override
  String get managedLocalizationSearchLabel => 'Projekttexte durchsuchen';

  @override
  String get managedLocalizationRefresh => 'Aktualisieren';

  @override
  String get managedLocalizationEmptyTitle => 'Noch keine Projekttexte';

  @override
  String get managedLocalizationEmptyDescription =>
      'Erstelle eine Dialogzeile, um Text zu schreiben und zu übersetzen.';

  @override
  String get managedLocalizationLoadFailed =>
      'Projekttexte konnten nicht geöffnet werden';

  @override
  String get managedLocalizationSelectText =>
      'Wähle einen Projekttext zum Bearbeiten aus';

  @override
  String get managedLocalizationLanguagesLabel => 'Sprachen';

  @override
  String get managedLocalizationUsedByLines => 'Von Dialogzeilen verwendet';

  @override
  String get managedLocalizationVoiceContextTitle =>
      'Voice für diese Dialogzeile';

  @override
  String get managedLocalizationVoiceSelectLine =>
      'Wähle oben eine Dialogzeile aus';

  @override
  String get managedLocalizationVoiceSetupExists => 'Voice-Setup vorhanden';

  @override
  String get managedLocalizationVoiceSetupMissing => 'noch kein Voice-Setup';

  @override
  String get managedLocalizationNoLine => 'Noch keiner Dialogzeile zugeordnet';

  @override
  String get managedLocalizationSpeakerLabel => 'Sprecherbezeichnung';

  @override
  String get managedLocalizationAddLanguage => 'Sprache hinzufügen';

  @override
  String get managedLocalizationRemoveLanguage => 'Sprache entfernen';

  @override
  String get managedLocalizationLanguageHint =>
      'Zum Beispiel de, en oder pt-BR';

  @override
  String get managedLocalizationLanguageExists =>
      'Diese Sprache ist bereits vorhanden.';

  @override
  String get managedLocalizationAdd => 'Hinzufügen';

  @override
  String get managedLocalizationSaved => 'Projekttext gespeichert';

  @override
  String get managedLocalizationVoiceLocked =>
      'Für diesen Text gibt es bereits Sprachaufnahmen. Deshalb ist der Text in diesem Editor gesperrt.';

  @override
  String get managedLocalizationVoiceSlotRemovalLocked =>
      'Diese Sprache ist mit einem Voice-Slot verknüpft und kann hier nicht entfernt werden.';

  @override
  String get managedLocalizationMinimumLanguageLocked =>
      'Behalte mindestens eine Sprache für diesen Projekttext.';

  @override
  String get managedLocalizationSharedNotice =>
      'Dieser Projekttext wird gemeinsam verwendet. Speichern aktualisiert jede aufgeführte Dialogzeile.';

  @override
  String get managedLocalizationOfflineNotice =>
      'Änderungen werden nur in diesem verwalteten Projekt gespeichert. Build und Verhalten im Spiel bleiben getrennte Schritte.';

  @override
  String get managedLocalizationUnsavedTitle =>
      'Ungespeicherte Änderungen verwerfen?';

  @override
  String get managedLocalizationUnsavedDescription =>
      'Du hast diesen Projekttext geändert. Beim Wechseln gehen diese Änderungen verloren.';

  @override
  String get managedLocalizationVoiceUnsavedTitle =>
      'Textänderungen vor dem Fortfahren speichern?';

  @override
  String get managedLocalizationVoiceUnsavedDescription =>
      'Speichere die Textänderungen und öffne direkt die gewählte Aktion, bearbeite den Text weiter oder verwirf die Änderungen bewusst.';

  @override
  String get managedLocalizationDiscardAndContinue =>
      'Verwerfen und fortfahren';

  @override
  String get managedLocalizationSaveAndContinue => 'Speichern und fortfahren';

  @override
  String get managedLocalizationGlobalAddVoice =>
      'Aufnahme für beliebige Zeile';

  @override
  String get managedLocalizationGlobalManageVoice =>
      'Aufnahmen beliebiger Zeile';

  @override
  String get managedLocalizationGlobalResolveVoice =>
      'Ziel für beliebige Zeile';

  @override
  String get managedVoiceFolderImportTitle => 'Aufnahmen-Ordner importieren';

  @override
  String get managedVoiceFolderImportDescription =>
      'Prüfe einen Ordner benannter Ogg-Aufnahmen und füge danach alle bereiten Takes in genau einer atomaren Projektänderung hinzu.';

  @override
  String get managedVoiceFolderImportChooseFolder =>
      'Aufnahmen-Ordner auswählen';

  @override
  String get managedVoiceFolderImportDirtyBlocked =>
      'Speichere oder verwirf die offenen Lokalisierungsänderungen, bevor du Aufnahmen importierst.';

  @override
  String managedVoiceFolderImportSaved(int count, int revision) {
    return '$count Aufnahmen in Projektrevision $revision importiert. Sie sind nur als Recorded-Takes im Projekt gespeichert; Auswahl, Spieldateien und Spielstände wurden nicht geändert.';
  }

  @override
  String managedVoiceTakeSaved(int revision) {
    return 'Voice-Take in Projektrevision $revision gespeichert. Er ist nur im Projekt gespeichert und noch nicht im Spiel nutzbar.';
  }

  @override
  String managedVoiceSelectionCleared(int revision) {
    return 'Voice-Auswahl in Projektrevision $revision geleert. Der Voice-Build bleibt ein separater Offline-Schritt; für die Laufzeit ist noch nichts qualifiziert.';
  }

  @override
  String managedVoiceSelectionSelected(int revision) {
    return 'Freigegebenen Voice-Take in Projektrevision $revision ausgewählt. Der Voice-Build bleibt ein separater Offline-Schritt; für die Laufzeit ist noch nichts qualifiziert.';
  }

  @override
  String managedVoiceTargetUnresolvedSaved(int revision) {
    return 'Kein Eintrag im installierten Archiv passte. Der Nachweis zum Voice-Ziel wurde in Projektrevision $revision gespeichert.';
  }

  @override
  String managedVoiceTargetResolvedSaved(int revision) {
    return 'Ein Eintrag im installierten Archiv wurde eindeutig versiegelt. Der Nachweis zum Voice-Ziel wurde in Projektrevision $revision gespeichert.';
  }

  @override
  String managedVoiceTargetAmbiguousSaved(int count, int revision) {
    return '$count Einträge im installierten Archiv passten; es wurde nichts stillschweigend ausgewählt. Der Nachweis zum Voice-Ziel wurde in Projektrevision $revision gespeichert.';
  }

  @override
  String get managedLocalizationDiscard => 'Änderungen verwerfen';

  @override
  String get managedLocalizationKeepEditing => 'Weiter bearbeiten';

  @override
  String get managedLocalizationStale =>
      'Das Projekt wurde geändert, während dieser Text geöffnet war. Aktualisiere die Ansicht und versuche es erneut.';

  @override
  String get managedLocalizationReopen =>
      'Das Projekt muss neu geöffnet werden, bevor du Projekttexte weiter bearbeiten kannst.';

  @override
  String get managedLocalizationInvalid =>
      'Prüfe, ob jede Sprache und jeder Dialogtext gültig und nicht leer ist.';

  @override
  String get managedLocalizationSaveFailed =>
      'Der Projekttext konnte nicht gespeichert werden.';

  @override
  String get managedLocalizationVoiceActionFailed =>
      'Die gewählte Aktion wurde nicht sauber abgeschlossen. Aktualisiere das Projekt vor einem erneuten Versuch; das exakt aktuelle Projekt zeigt, ob eine Änderung veröffentlicht wurde. Dieser Arbeitsbereich hat keine Spiel- oder Speicherdateien geändert.';

  @override
  String get managedSectionValidateTestDescription =>
      'Prüft die exakte Projektintegrität und Checkpoints; ein Laufzeittest wird nicht zugesichert.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Voice-Bundles sind verfügbar; vollständige spielbare Builds und Bereitstellung sind nicht verfügbar.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Einstellungen sind verfügbar; Expertenwerkzeuge sind noch nicht integriert.';

  @override
  String get managedSectionStatusHeading => 'Status';

  @override
  String get managedSectionActionsHeading => 'Aktionen';

  @override
  String get managedCapabilityAvailable => 'Verfügbar';

  @override
  String get managedCapabilityPartial => 'Teilweise';

  @override
  String get managedCapabilityPlanned => 'Geplant';

  @override
  String get managedCapabilityUnavailable => 'Nicht verfügbar';

  @override
  String get managedProjectSubtitle =>
      'Offline-Arbeitsbereich auf exakt aktuellem Stand';

  @override
  String get managedProjectLandingTitle =>
      'Arbeitsbereich für verwaltete Projekte';

  @override
  String get managedProjectLandingDescription =>
      'Nutze den neuen Ablauf für Start, Inhalte, Story, Sprachausgabe, Prüfung und Veröffentlichung in einem einzigen verwalteten Projekt.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Kompatibilitätswerkzeuge für Legacy-Projekte';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Die Tabs unten enthalten ältere Werkzeuge für direkte Ersetzungen. Sie bleiben verfügbar, während der Arbeitsbereich für verwaltete Projekte weiter ausgebaut wird.';

  @override
  String get managedProjectTechnicalDetails => 'Technische Projektdetails';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Stelle das verwaltete Projekt wieder her oder öffne es erneut, bevor seine Inhalte gelesen werden.';

  @override
  String get managedProjectRecoveryDescription =>
      'Mod Studio öffnet dieses Projekt sicher neu, während die Sperre gehalten wird. Dabei werden weder das Spiel noch ein Spielstand verändert.';

  @override
  String get managedProjectRecoveryTry => 'Wiederherstellung versuchen';

  @override
  String get managedProjectRecoveryTrying => 'Wiederherstellung läuft…';

  @override
  String get managedProjectRecoveryAlternative =>
      'Falls die Wiederherstellung nicht funktioniert, schließe das Projekt und öffne es erneut.';

  @override
  String get managedProjectRecoverySucceeded =>
      'Das Projekt wurde wiederhergestellt. Du kannst weiterarbeiten.';

  @override
  String get managedProjectRecoveryFailed =>
      'Die Wiederherstellung wurde nicht abgeschlossen. Versuche es erneut oder schließe das Projekt und öffne es wieder.';

  @override
  String get managedProjectRecoveryUnavailable =>
      'Die Wiederherstellung ist für dieses Projekt nicht verfügbar. Schließe das Projekt und öffne es erneut.';

  @override
  String get managedDashboardUntitledProject => 'Unbenanntes Projekt';

  @override
  String get managedDashboardDraftStatus => 'Entwurf';

  @override
  String get managedDashboardProjectVersion => 'Version';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'Nicht angegeben';

  @override
  String get managedDashboardContentCounts => 'Projektinhalte';

  @override
  String get managedDashboardNpcDrafts => 'NPC-Entwürfe';

  @override
  String get managedDashboardQuestDrafts => 'Quest-Entwürfe';

  @override
  String get managedDashboardDialogLines => 'Dialogzeilen';

  @override
  String get managedDashboardVoiceTakes => 'Sprachaufnahmen';

  @override
  String get managedDashboardAssets => 'Assets';

  @override
  String get managedDashboardUnresolvedReferences => 'Ungelöste Referenzen';

  @override
  String get managedDashboardReadiness => 'Was jetzt funktioniert';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Offline-Bearbeitung verfügbar';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Erstelle und bearbeite unterstützte Projektinhalte, ohne die Spielinstallation oder Speicherdateien zu verändern.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Allgemeiner Mod-Build nicht verfügbar';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Nur versiegelte Offline-Voice-Bundles können gebaut werden; eine vollständige spielbare Mod kann noch nicht gebaut werden.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Laufzeit noch nicht verifiziert';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio hat diese Projektinhalte noch nicht im laufenden Spiel nachgewiesen.';

  @override
  String get managedDashboardReferenceIntegrityTitle => 'Referenzintegrität';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Diese Anzahl prüft nur Projektreferenzen; sie bestätigt keine Build- oder Laufzeitbereitschaft.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Spieleinrichtung erforderlich';

  @override
  String get managedDashboardMissingGameDescription =>
      'Richte die Gothic-1-Remake-Installation in den Einstellungen ein, bevor du Aktionen verwendest, die Nachweise aus dem installierten Spiel benötigen.';

  @override
  String get managedDashboardCreateHeading => 'Erstellen';

  @override
  String get managedDashboardToolsHeading => 'Projektwerkzeuge';

  @override
  String get managedDashboardContinueHeading => 'Weiterarbeiten';

  @override
  String get managedHomeStoryEmptyTitle => 'Figur oder Quest erstellen';

  @override
  String get managedHomeStoryContinueTitle => 'Story weiterführen';

  @override
  String get managedHomeStoryDescription =>
      'Erstelle und entwickle NPC- und Quest-Entwürfe im vollständigen Story-Arbeitsbereich.';

  @override
  String get managedHomeDialogVoiceTitle => 'Dialog & Vertonung';

  @override
  String get managedHomeDialogVoiceDescription =>
      'Schreibe Projekttexte und Dialogzeilen und verwalte Sprachaufnahmen an einem Ort.';

  @override
  String get managedHomeProblemsTitle => 'Probleme prüfen';

  @override
  String get managedHomeProblemsDescription =>
      'Prüfe exakte Projektprobleme und Verifikation, ohne einen Laufzeittest zu behaupten.';

  @override
  String get managedHomeContentTitle => 'Inhalte durchsuchen';

  @override
  String get managedHomeContentDescription =>
      'Finde Projekt-, Basisspiel-, installierte und verifizierte DataAsset-Inhalte.';

  @override
  String get managedHomeBuildTitle => 'Ausgabe erstellen';

  @override
  String get managedHomeBuildDescription =>
      'Öffne die ehrliche Build-Ansicht. Voice-Bundles sind verfügbar; eine vollständige spielbare Mod ist noch blockiert.';

  @override
  String get managedContentOpenInStory => 'In Story öffnen';

  @override
  String get managedContentOpenInStoryDescription =>
      'Bearbeite diese Quest oder diesen NPC im vollständigen Story-Arbeitsbereich weiter.';

  @override
  String get managedContentOpenInStoryRequiresReopen =>
      'Öffne dieses Projekt erneut, bevor du Story öffnest.';

  @override
  String get managedContentOpenInStoryFailed =>
      'Story konnte nicht geöffnet werden. Das Projekt wurde nicht verändert.';

  @override
  String get managedStoryWorkbenchActionFailed =>
      'Dieser Editor konnte nicht geöffnet werden. Versuche es erneut.';

  @override
  String get managedDashboardLoading => 'Projektübersicht wird geladen';

  @override
  String get managedDashboardLoadError => 'Projektübersicht nicht verfügbar';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Die verifizierte Projektübersicht konnte nicht geladen werden. Projektinhalte wurden nicht verändert.';

  @override
  String get managedDashboardRetry => 'Erneut versuchen';

  @override
  String get managedActionNewNpcTitle => 'Neuer NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Erstelle anhand verifizierter Nachweise aus dem installierten Spiel einen begrenzten Offline-NPC-Entwurf.';

  @override
  String managedNpcDraftSaved(int projectRevision) {
    return 'Charakterentwurf in Projektrevision $projectRevision gespeichert. Er bleibt für Builds gesperrt, ist für die Laufzeit ungeprüft und wird nicht gespawnt.';
  }

  @override
  String get managedActionNewQuestTitle => 'Neue Quest';

  @override
  String get managedActionNewQuestDescription =>
      'Erstelle einen Offline-Quest-Entwurf mit Zielen und verifizierten übergeordneten Identitäten.';

  @override
  String get managedQuestOpeningRecipeTitle => 'Quest + erste Dialogzeile';

  @override
  String get managedQuestOpeningRecipeDescription =>
      'Empfohlen: Erstelle einen Quest-Entwurf und schreibe danach direkt die erste lokalisierte Dialogzeile. Das nutzt zwei Projektstände und erzeugt noch keinen spielbaren Dialog.';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'Dieser geführte Ablauf speichert zuerst die Quest und öffnet danach ihre erste Dialogzeile. Wenn du nach Schritt 1 aufhörst, bleibt die Quest gespeichert. Es entsteht noch kein spielbarer Dialog; Spiel und Spielstände werden nicht verändert.';

  @override
  String get managedQuestOpeningRecipeStart => 'Geführte Quest starten';

  @override
  String get managedQuestOpeningLineTitle =>
      'Schritt 2 von 2: Erste Dialogzeile';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'Schreibe die erste lokalisierte Zeile dieser Quest. Beim Speichern werden Zeile und Text erstellt und am Anfang des Quest-Transkripts eingefügt.';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'Quest in Projektrevision $projectRevision gespeichert. Die erste Dialogzeile wird vorbereitet ...';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'Quest in Projektrevision $projectRevision gespeichert; es wurde keine erste Dialogzeile hinzugefügt. Fahre unter Story > Dialog & Sprachausgabe fort.';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'Die geführte Quest konnte nicht gestartet werden. Es wurden keine Projektänderungen veröffentlicht.';

  @override
  String get managedQuestOpeningRecipeStopped =>
      'Der geführte Ablauf wurde angehalten, weil sich der exakte aktuelle Projektstand geändert hat. Es wird kein weiterer Schritt automatisch ausgeführt; prüfe Story und fahre manuell fort.';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'Der geführte Ablauf konnte nicht sicher fortgesetzt werden. Öffne dieses Projekt erneut und prüfe Story, bevor du es erneut versuchst oder manuell fortfährst.';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'Quest und erste Dialogzeile in Projektrevision $projectRevision gespeichert. Nur Entwurf: Kein spielbarer Dialog; Spiel und Spielstände wurden nicht verändert.';
  }

  @override
  String get managedActionNewDialogLineTitle => 'Dialogzeile hinzufügen';

  @override
  String get managedActionNewDialogLineDescription =>
      'Schreibe lokalisierten Projekttext oder verbinde einen noch unbenutzten Text aus diesem Projekt. Dadurch entsteht noch kein spielbares Dialogthema.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Dialogzeile in Projektrevision $projectRevision gespeichert. Spiel und Spielstände wurden nicht verändert.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Schreibe eine neue lokalisierte Dialogzeile oder verbinde Text, der bereits zu diesem Projekt gehört.';

  @override
  String get managedDialogLineBoundary =>
      'Nur Projektdateien werden geändert. Dadurch entsteht weder ein AngelScript-Thema noch ein spielbarer Dialog; Spielinstallation und Spielstände bleiben unverändert. Das Sprecherfeld ist nur ein Label und verbindet keinen NPC.';

  @override
  String get managedDialogLineCreateMode => 'Neuen Text schreiben';

  @override
  String get managedDialogLineReuseMode => 'Projekttext verwenden';

  @override
  String get managedDialogLineNameLabel => 'Name der Zeile';

  @override
  String get managedDialogLineNameHint => 'Begrüßung am Mineneingang';

  @override
  String get managedDialogLineSpeakerLabel => 'Sprecher-Label (optional)';

  @override
  String get managedDialogLineSpeakerHint => 'Zum Beispiel Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Sprache';

  @override
  String get managedDialogLineTextLabel => 'Dialogtext';

  @override
  String get managedDialogLineReuseSearch => 'Unbenutzten Projekttext suchen';

  @override
  String get managedDialogLineNoReusableText =>
      'Es gibt keinen unbenutzten, strukturell intakten Projekttext zum Verbinden. Schreibe stattdessen neuen Text.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Diese Sprache für Voice vorbereiten';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Erstellt einen leeren, noch nicht aufgelösten Voice-Slot im Projekt. Es wird keine Aufnahme hinzugefügt oder bereitgestellt.';

  @override
  String get managedDialogLineCancel => 'Abbrechen';

  @override
  String get managedDialogLineSave => 'Im Projekt speichern';

  @override
  String get managedDialogLineSaving => 'Wird gespeichert…';

  @override
  String get managedDialogLineLoading => 'Exakter Projektinhalt wird gelesen…';

  @override
  String get managedDialogLineLoadFailed =>
      'Der exakte aktuelle Projektinhalt konnte nicht gelesen werden. Es wurde nichts geändert.';

  @override
  String get managedDialogLineRetry => 'Erneut versuchen';

  @override
  String get managedDialogLineStale =>
      'Das Projekt wurde geändert, während dieses Fenster geöffnet war. Schließe es und versuche es vom aktuellen Projektstand erneut.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Das aktuelle Projekt kann nicht mehr sicher bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut.';

  @override
  String get managedDialogLineInvalidInput =>
      'Prüfe die markierte Projekteingabe und wähle eine exakte aktuelle Option.';

  @override
  String get managedDialogLineSaveFailed =>
      'Die Dialogzeile konnte nicht sicher gespeichert werden. Spiel und Spielstände wurden nicht verändert.';

  @override
  String get managedDialogLineDone => 'Fertig';

  @override
  String get managedDialogLineAddRecording => 'Aufnahme hinzufügen';

  @override
  String get managedActionAddVoiceTakeTitle => 'Sprachaufnahme hinzufügen';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importiere eine Ogg-Vorbis-Aufnahme für eine vorhandene Dialogzeile, ohne sie bereitzustellen.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Erstelle oder repariere zuerst eine Dialogzeile mit genau einem gültigen Lokalisierungseintrag, bevor du Voice-Werkzeuge verwendest.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Sprachaufnahmen verwalten';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Prüfe Aufnahmen und wähle freigegebene Aufnahmen für Voice-Slots aus.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Voice-Ziel auflösen';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Ordne Voice-Slots des Projekts exakten Mitgliedern installierter Archive zu, ohne das Spiel zu verändern.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Voice-Bundle bauen';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Baue ein versiegeltes Offline-Bundle aus vorhandenen Mitgliedern; es wird nicht bereitgestellt.';

  @override
  String get managedActionDataAssetsTitle => 'DataAsset-Änderungen';

  @override
  String get managedActionDataAssetsDescription =>
      'Prüfe installierte Pakete und stelle verifizierte Werteänderungen fester Breite im Projekt bereit.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Durchsuche den exakten Projektinhalt sowie seine aufgelösten oder nicht aufgelösten Verweise.';

  @override
  String get managedActionSettingsTitle => 'Einstellungen';

  @override
  String get managedActionSettingsDescription =>
      'Konfiguriere die Gothic-1-Remake-Installation und die Mod-Studio-Einstellungen.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Projekt $projectId wurde sicher erstellt, aber die Starter-Einrichtung konnte nicht geöffnet werden. Das gültige leere Projekt bleibt aktuell.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Projekt $projectId wurde erstellt, aber Mod Studio kann den Starter-Ausgang nicht verifizieren. Öffne das verwaltete Projekt vor dem Fortfahren neu; Spiel und Spielstände wurden nicht verändert.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Projekt $projectId wurde erstellt. Der NPC-Starter wurde nicht hinzugefügt; das gültige leere Projekt bleibt aktuell.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'NPC-Starter in Projektrevision $projectRevision gespeichert. Er bleibt für Builds gesperrt, ist nicht für die Laufzeit qualifiziert und wird nicht gespawnt.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Projekt $projectId wurde erstellt. Der Quest-Starter wurde nicht hinzugefügt; das gültige leere Projekt bleibt aktuell.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Quest-Starter in Projektrevision $projectRevision gespeichert. Er bleibt für Builds gesperrt und ist nicht für die Laufzeit qualifiziert.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Projektstarter';

  @override
  String get projectStarterPrompt => 'Wie möchtest du beginnen?';

  @override
  String get projectStarterWriteBoundary =>
      'Die Starter-Auswahl schreibt nichts. Das Projekt wird erst erstellt, nachdem du dieses Formular absendest und einen leeren Ordner auswählst.';

  @override
  String get projectStarterEmptyTitle => 'Leeres Projekt';

  @override
  String get projectStarterEmptyDescription =>
      'Erstelle nur das verwaltete Projekt. Inhalte kannst du jederzeit hinzufügen.';

  @override
  String get projectStarterNpcDraftTitle => 'NPC-Entwurf';

  @override
  String get projectStarterNpcDraftDescription =>
      'Erstelle zuerst das leere Projekt und öffne danach die geführte NPC-Entwurfseinrichtung.';

  @override
  String get projectStarterQuestDraftTitle => 'Quest-Entwurf';

  @override
  String get projectStarterQuestDraftDescription =>
      'Erstelle zuerst das leere Projekt und öffne danach die geführte Quest-Entwurfseinrichtung.';

  @override
  String get projectStarterPartialOutcome =>
      'Wenn du die geführte NPC- oder Quest-Einrichtung abbrichst oder der Entwurf fehlschlägt, bleibt ein gültiges leeres Projekt erhalten. Die Starter-Auswahl schreibt weder ins Spiel noch in einen Spielstand.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Durchsuchen';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Verifizierte Änderungen';

  @override
  String get managedContentScopeBaseGameLabel => 'Basisspiel';

  @override
  String get managedContentScopeInstalledLabel => 'Installiert';

  @override
  String get managedBaseGameBrowserTitle =>
      'Unterstützte Startpunkte aus dem Basisspiel';

  @override
  String get managedBaseGameBrowserDescription =>
      'Durchsuche exakte Belege aus der installierten Spielversion, die Mod Studio derzeit prüfen oder als sicheren Entwurfsstart verwenden kann. Dies ist kein vollständiger Katalog aller Vanilla-Inhalte.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Exakte Basisspiel-Belege werden gelesen…';

  @override
  String get managedBaseGameBrowserRefresh => 'Neuen exakten Katalog lesen';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Unterstützte Basisspiel-Inhalte durchsuchen';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPCs';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Quests';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'NPC-Startpunkte';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => 'Quest-Startpunkte';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Nur prüfbare NPC-Archetypen';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Suche, um weitere statisch verknüpfte NPC-Belege einzubeziehen. Aus diesen Zeilen lässt sich kein Entwurf erstellen.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Kein unterstütztes Basisspiel-Ergebnis entspricht dieser Suche.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Basisspiel-Belege nicht verfügbar';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Der exakte unterstützte Katalog konnte nicht gelesen werden. Projekt-, Spiel- und Spielstanddateien wurden nicht verändert.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Offline-Entwurf unterstützt';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Nur prüfen';

  @override
  String get managedBaseGameBrowserCreateNpcDraft =>
      'Als NPC-Startpunkt verwenden';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Als Quest-Startpunkt verwenden';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Spawn-Definition';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Akteur-Blueprint';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Die ersten 100 ausschließlich prüfbaren Treffer werden angezeigt. Verfeinere die Suche für genauere Ergebnisse.';

  @override
  String get managedInstalledBrowserLoading =>
      'Exaktes Inventar installierter Pakete wird gelesen…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count installierte Paketkandidaten';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count installierte Paketkandidaten — Teilergebnis';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Die Verzeichnismetadaten wurden gelesen und der installierte Schnappschuss blieb exakt.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Einige Paketmetadaten fehlten oder waren nicht kanonisch. Die Ergebnisse helfen bei der Suche, sind aber nicht vollständig.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Dieser Bereich zeigt nur Metadaten installierter DataAsset-Pakete. Das Prüfen oder Kopieren eines Pfads erteilt keine Berechtigung zum Bauen, Bereitstellen, zur Laufzeit oder zum Schreiben ins Spiel.';

  @override
  String get managedInstalledBrowserRefresh =>
      'Neuen exakten Schnappschuss lesen';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Installierte DataAssets durchsuchen';

  @override
  String get managedInstalledBrowserSearchHint => 'Asset-Name oder /Game-Pfad';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Gib zum Suchen einen Asset-Namen oder /Game-Pfad ein.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Kein passendes installiertes DataAsset';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Versuche einen anderen Asset-Namen oder einen allgemeineren /Game-Pfad.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Die ersten 100 Treffer werden angezeigt. Verfeinere die Suche, um den exakten Schnappschuss einzugrenzen.';

  @override
  String get managedInstalledBrowserKindBadge => 'DataAsset-Paket';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Nur Metadaten';

  @override
  String get managedInstalledBrowserOpenInspector => 'Exaktes Paket prüfen';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventar installierter Pakete nicht verfügbar';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Der exakte installierte Schnappschuss konnte nicht gelesen werden. Projekt-, Spiel- und Spielstanddateien wurden nicht verändert.';

  @override
  String get managedGlobalSearchScopeLabel => 'Alles durchsuchen';

  @override
  String get managedGlobalSearchTitle => 'Alle Inhalte durchsuchen';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, Quest, Dialogzeile, Asset, ID oder /Game-Pfad';

  @override
  String get managedGlobalSearchAction => 'Suchen';

  @override
  String get managedGlobalSearchClear => 'Leeren';

  @override
  String get managedGlobalSearchPrompt =>
      'Gib einen Suchbegriff ein, um die drei Quellen unabhängig voneinander zu lesen.';

  @override
  String get managedGlobalSearchNoResults => 'Keine Treffer in dieser Quelle.';

  @override
  String get managedGlobalSearchLoading => 'Exakte Quelle wird gelesen…';

  @override
  String get managedGlobalSearchFailed =>
      'Diese Quelle konnte nicht gelesen werden.';

  @override
  String get managedGlobalSearchComplete => 'Vollständig';

  @override
  String get managedGlobalSearchPartial => 'Teilweise';

  @override
  String get managedGlobalSearchTruncated =>
      'Die ersten 100 Treffer werden angezeigt. Suche präzisieren.';

  @override
  String get managedGlobalSearchOpen => 'Öffnen';

  @override
  String get managedGlobalSearchCreateDraft => 'Entwurf erstellen';

  @override
  String get managedGlobalSearchInspect => 'Prüfen';

  @override
  String get managedGlobalSearchKindModEntity => 'Mod-Inhalt';

  @override
  String get managedGlobalSearchKindModAsset => 'Mod-Asset';

  @override
  String get managedGlobalSearchKindBaseNpc => 'NPC-Ausgangspunkt';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Quest-Ausgangspunkt';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'NPC-Nachweis';

  @override
  String get managedGlobalSearchReadinessExact => 'Exaktes aktuelles Projekt';

  @override
  String get managedGlobalSearchReadinessProblems => 'Exakt, mit Problemen';

  @override
  String get managedGlobalSearchResultStale =>
      'Dieser Treffer ist nicht mehr im aktuellen Projekt enthalten. Erneut suchen.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Nur Entwurf';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Build blockiert';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Laufzeit nicht verifiziert';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Quest-Verlauf';

  @override
  String get managedStoryWorkbenchProfileTab => 'Profil';

  @override
  String get managedStoryWorkbenchStoryTab => 'Story';

  @override
  String get managedStoryWorkbenchLogicTab => 'Logik';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Routine';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventar';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Dialog & Sprachausgabe';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Referenzen';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => 'Probleme & Prüfungen';

  @override
  String get managedStoryWorkbenchEditOverview => 'Name & Ziele bearbeiten';

  @override
  String get managedStoryWorkbenchEditStory =>
      'Beschreibung & Verknüpfungen bearbeiten';

  @override
  String get managedStoryWorkbenchEditLogic =>
      'Zustände & Übergänge bearbeiten';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Quelltext & Compilerprüfungen öffnen';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Profil & Compilerprüfungen öffnen';

  @override
  String get managedStoryWorkbenchMoreActions => 'Weitere Aktionen';

  @override
  String get managedStoryWorkbenchRemoveDraft => 'Entwurf entfernen…';

  @override
  String get managedStoryWorkbenchRemovingDraft => 'Entwurf wird entfernt…';

  @override
  String get managedStoryWorkbenchReviewRemovalBlockers =>
      'Blockierende Referenzen prüfen';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable =>
      'Noch nicht modelliert';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Quest- und Story-Beziehungen sind für NPC-Entwürfe noch nicht modelliert.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'Routine und Platzierung in der Welt sind noch nicht modelliert.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'Inventar, Ausrüstung und Handel sind noch nicht modelliert.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Dialog-, Lokalisierungs- und Sprachausgabebeziehungen sind für NPC-Entwürfe noch nicht modelliert.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Dialog-, Lokalisierungs- und Sprachausgabebeziehungen sind für Quest-Entwürfe noch nicht modelliert.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Keine ungelösten Projektreferenzen';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ungelöste Projektreferenzen',
      one: '1 ungelöste Projektreferenz',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Nur Referenzstatus; dies ist keine Build- oder Laufzeitbereitschaft.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Technische Details';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Quest-Entwurf';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'NPC-Entwurf';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Quest-Titel';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'Technische ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Ziele';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Eindeutiger Name';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => 'Modul-Namensraum';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Questgeber';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => 'Laufzeit-Basisklasse';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Quest-Lebenszykluszustände, Auslöser, Bedingungen und Effekte werden in einer einzigen atomaren Operation am exakten aktuellen Stand bearbeitet.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Ausgehend';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Keine projizierten Referenzen';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Eingehend';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Keine eingehenden Projektreferenzen';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Semantische Identität';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Ursprung';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Entitätsrevision';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'Stabile ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Referenz aufgelöst';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Referenz nicht aufgelöst';

  @override
  String get managedProblemsTitle => 'Probleme & Bereitschaft';

  @override
  String get managedProblemsDescription =>
      'Sieh, was Aufmerksamkeit braucht, und öffne direkt den genau betroffenen Projektinhalt.';

  @override
  String get managedProblemsScopeNotice =>
      'Jeder Status gilt nur für den genannten Bereich. Fehlerfreie Referenzen bedeuten nicht, dass der Mod gebaut oder im Spiel getestet werden kann.';

  @override
  String get managedProblemsRefresh => 'Probleme aktualisieren';

  @override
  String get managedProblemsPartialTitle =>
      'Einige Prüfungen sind nicht verfügbar';

  @override
  String get managedProblemsDataAssetsUnavailable =>
      'DataAsset-Änderungen konnten nicht geprüft werden. Andere exakte Projektbefunde werden weiterhin angezeigt.';

  @override
  String get managedProblemsOverviewHeading => 'Bereitschaft nach Bereich';

  @override
  String get managedProblemsSearchLabel => 'Probleme durchsuchen';

  @override
  String get managedProblemsClearSearch => 'Problemsuche leeren';

  @override
  String get managedProblemsListHeading => 'Probleme';

  @override
  String get managedProblemsEmptyTitle =>
      'Keine modellierten Strukturprobleme gefunden';

  @override
  String get managedProblemsEmptyDescription =>
      'Die derzeit in Mod Studio modellierten exakten Prüfungen fanden nichts zu reparieren.';

  @override
  String get managedProblemsEmptyBoundary =>
      'Compilerbelege wurden nicht geprüft, der vollständige verwaltete Build ist nicht verfügbar und das Laufzeitverhalten bleibt unqualifiziert.';

  @override
  String get managedProblemsFilteredEmptyTitle => 'Keine passenden Probleme';

  @override
  String get managedProblemsFilteredEmptyDescription =>
      'Ändere Suche oder Bereichsfilter, um andere Befunde zu sehen.';

  @override
  String get managedProblemsSelectTitle => 'Problem auswählen';

  @override
  String get managedProblemsSelectDescription =>
      'Wähle einen Befund, um Bedeutung und sicherste verfügbare nächste Aktion zu sehen.';

  @override
  String get managedProblemsDetailHeading => 'Problemdetails';

  @override
  String get managedProblemsCloseDetail => 'Problemdetails schließen';

  @override
  String get managedProblemsCategoryLabel => 'Bereich';

  @override
  String get managedProblemsSeverityLabel => 'Dringlichkeit';

  @override
  String get managedProblemsSourceLabel => 'Beleg';

  @override
  String get managedProblemsOpenSourceEntity => 'Ausgangsinhalt öffnen';

  @override
  String get managedProblemsOpenReferencedAsset => 'Verknüpftes Asset öffnen';

  @override
  String get managedProblemsOpenDataAssetEdits => 'DataAsset-Änderungen öffnen';

  @override
  String get managedProblemsActionFailed =>
      'Das exakte Ziel konnte nicht geöffnet werden. Aktualisiere die Projektprobleme und versuche es erneut.';

  @override
  String get managedProblemsActionProgress =>
      'Exaktes Projektziel wird geöffnet';

  @override
  String get managedProblemsCategoryReferences => 'Referenzen';

  @override
  String get managedProblemsCategorySetup => 'Einrichtung';

  @override
  String get managedProblemsCategoryDataAssets => 'DataAssets';

  @override
  String get managedProblemsSeverityInformation => 'Information';

  @override
  String get managedProblemsSeverityWarning => 'Braucht Aufmerksamkeit';

  @override
  String get managedProblemsSeverityBlocking => 'Blockiert diesen Bereich';

  @override
  String get managedProblemsScopeReferencesTitle => 'Referenzintegrität';

  @override
  String get managedProblemsScopeReferencesDescription =>
      'Prüft exakte Verknüpfungen zwischen aktuellem Projektinhalt und Assets.';

  @override
  String get managedProblemsScopeDataAssetsTitle =>
      'DataAsset-Änderungsregister';

  @override
  String get managedProblemsScopeDataAssetsDescription =>
      'Prüft, ob die exakte aktuelle Liste gespeicherter DataAsset-Änderungen gelesen werden konnte.';

  @override
  String get managedProblemsScopeGameTitle => 'Spieleinrichtung';

  @override
  String get managedProblemsScopeGameDescription =>
      'Zeigt, ob eine Spielinstallation für begrenzte schreibgeschützte Werkzeuge konfiguriert ist.';

  @override
  String get managedProblemsScopeCompilerTitle => 'Quelltext- & Compilerbelege';

  @override
  String get managedProblemsScopeCompilerDescription =>
      'Compilerprüfungen laufen nur, wenn du sie für eine exakte Entität ausdrücklich öffnest und startest.';

  @override
  String get managedProblemsScopeBuildTitle => 'Verwalteter Projekt-Build';

  @override
  String get managedProblemsScopeBuildDescription =>
      'Ein vollständiger Build-Pfad für verwaltete NPC-, Quest-, Dialog- und DataAsset-Änderungen ist noch nicht verfügbar.';

  @override
  String get managedProblemsScopeRuntimeTitle => 'Verhalten im Spiel';

  @override
  String get managedProblemsScopeRuntimeDescription =>
      'Es wird keine allgemeine Laufzeit-, Spielstand-, Bereitstellungs- oder Bereinigungsqualifikation behauptet.';

  @override
  String get managedProblemsReadinessClear => 'In diesem Bereich geprüft';

  @override
  String get managedProblemsReadinessIssues => 'Braucht Aufmerksamkeit';

  @override
  String get managedProblemsReadinessUnavailable => 'Prüfung nicht verfügbar';

  @override
  String get managedProblemsReadinessNotEvaluated => 'Nicht geprüft';

  @override
  String get managedProblemsReadinessBlocked => 'Build-Pfad nicht verfügbar';

  @override
  String get managedProblemsReadinessUnqualified => 'Laufzeit unqualifiziert';

  @override
  String get managedProblemsEvidenceContent =>
      'Exakter aktueller Projektinhalt';

  @override
  String get managedProblemsEvidenceDataAssets =>
      'Exaktes aktuelles DataAsset-Register';

  @override
  String get managedProblemsEvidenceConfiguration =>
      'Aktuelle App-Konfiguration';

  @override
  String get managedProblemsEvidenceUnavailable =>
      'Belegquelle nicht verfügbar';

  @override
  String get managedProblemsEvidenceBoundary => 'Bekannte Funktionsgrenze';

  @override
  String get managedProblemsForeignReferenceTitle =>
      'Referenz zeigt in ein anderes Projekt';

  @override
  String get managedProblemsMissingEntityTitle =>
      'Verknüpfter Projektinhalt fehlt';

  @override
  String get managedProblemsEntityKindTitle =>
      'Verknüpfter Projektinhalt hat den falschen Typ';

  @override
  String get managedProblemsMissingAssetTitle =>
      'Verknüpfte Projektdatei fehlt';

  @override
  String get managedProblemsAssetLengthTitle =>
      'Verknüpfte Projektdatei hat eine unerwartete Größe';

  @override
  String get managedProblemsAssetTypeTitle =>
      'Verknüpfte Projektdatei hat einen unerwarteten Typ';

  @override
  String get managedProblemsGameSetupTitle =>
      'Spielinstallation ist nicht konfiguriert';

  @override
  String get managedProblemsDataAssetRegistryTitle =>
      'DataAsset-Änderungen konnten nicht geprüft werden';

  @override
  String get managedProblemsDataAssetOfflineTitle =>
      'DataAsset-Änderung ist nur ein Entwurf';

  @override
  String managedProblemsEntityReferenceDescription(String source) {
    return 'Öffne $source und repariere diese exakte Projektinhalt-Verknüpfung.';
  }

  @override
  String managedProblemsAssetReferenceDescription(String source) {
    return 'Öffne $source und repariere diese exakte Projektdatei-Verknüpfung.';
  }

  @override
  String get managedProblemsDataAssetRegistryDescription =>
      'Aktualisiere den exakten aktuellen Projektstand. Bis diese Quelle verfügbar ist, wird keine Aussage über gespeicherte DataAsset-Änderungen getroffen.';

  @override
  String managedProblemsDataAssetOfflineDescription(String targetPath) {
    return 'Die gespeicherte Änderung für $targetPath kann unter DataAsset-Änderungen geprüft werden, aber noch nicht durch einen verwalteten Projekt-Build ausgegeben oder als im Spiel funktionsfähig bezeichnet werden.';
  }

  @override
  String get projectExportActionTitle => 'Projektkopie exportieren…';

  @override
  String get projectExportActionDescription =>
      'Eine exakte portable Kopie des aktuell gespeicherten Projektstands schreiben.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Speichere oder verwirf die offenen Lokalisierungsänderungen, bevor du eine Projektkopie exportierst.';

  @override
  String get projectExportDialogTitle => 'Projektkopie exportieren';

  @override
  String get projectExportPortableCopyTitle => 'Portable Projektkopie';

  @override
  String get projectExportPortableCopyDescription =>
      'Dies schreibt den exakten aktuell gespeicherten Projektstand in eine neue .goremod-Datei. Das offene Projekt bleibt aktuell und unverändert.';

  @override
  String get projectExportCapabilityBoundary =>
      'Diese Kopie ist kein spielbarer Mod, Build, Deployment oder wiederherstellbares Backup. Spiel und Spielstände werden weder gelesen noch verändert.';

  @override
  String get projectExportKeepOriginal =>
      'Der Import dieser verwalteten Kopie ist noch nicht verfügbar. Bewahre den ursprünglichen Projektordner auf.';

  @override
  String get projectExportFileNameLabel => 'Neue Projektkopie-Datei';

  @override
  String get projectExportFileNameHelper =>
      'Verwende einen neuen portablen Dateinamen mit der Endung .goremod.';

  @override
  String get projectExportChooseDestination => 'Zielordner auswählen';

  @override
  String get projectExportNoDestination => 'Kein Zielordner ausgewählt';

  @override
  String get projectExportNewFile => 'Neue Datei';

  @override
  String get projectExportCancel => 'Abbrechen';

  @override
  String get projectExportClose => 'Schließen';

  @override
  String get projectExportSubmit => 'Kopie exportieren';

  @override
  String get projectExportExporting => 'Export wird erstellt…';

  @override
  String get projectExportParentRequired =>
      'Wähle einen vorhandenen Zielordner aus.';

  @override
  String get projectExportParentAbsolute =>
      'Wähle einen absoluten vorhandenen Zielordner aus.';

  @override
  String get projectExportParentLink =>
      'Das ausgewählte Ziel ist ein Link. Wähle einen echten vorhandenen Ordner.';

  @override
  String get projectExportParentInspectFailed =>
      'Der Zielordner konnte nicht sicher geprüft werden. Nichts wurde erstellt.';

  @override
  String get projectExportFileNameRequired =>
      'Gib einen neuen Dateinamen für die Projektkopie ein.';

  @override
  String get projectExportFileNameTooLong =>
      'Der Dateiname darf höchstens 128 ASCII-Zeichen lang sein.';

  @override
  String get projectExportFileNameInvalid =>
      'Beginne mit einem Buchstaben oder einer Ziffer, verwende nur ASCII-Buchstaben, Ziffern, Punkte, Unterstriche oder Bindestriche und ende mit .goremod.';

  @override
  String get projectExportFileNameReserved =>
      'Dieser Dateiname ist unter Windows reserviert.';

  @override
  String get projectExportOutputExists =>
      'Diese Datei existiert bereits. Wähle einen neuen Dateinamen; vorhandene Dateien werden niemals überschrieben.';

  @override
  String get projectExportOutputLink =>
      'Der neue Dateipfad ist ein Link. Wähle einen anderen Dateinamen.';

  @override
  String get projectExportOutputRejected =>
      'Das Ziel wurde abgelehnt, bevor die neue lokale Datei erstellt wurde. Nichts wurde erstellt. Wähle einen anderen Dateinamen oder Zielordner.';

  @override
  String get projectExportStale =>
      'Das Projekt wurde vor dem Export geändert. Es wurde keine Ausgabe erstellt. Schließe dieses Fenster und öffne Projektkopie exportieren erneut.';

  @override
  String get projectExportRequiresReopen =>
      'Dieses Projekt kann nicht mehr als aktuell verifiziert werden. Es wurde keine Ausgabe erstellt. Schließe dieses Fenster und stelle das Projekt wieder her oder öffne es erneut.';

  @override
  String get projectExportUnsupported =>
      'Diese verwaltete Projektsitzung kann keine exakten portablen Kopien exportieren. Nichts wurde erstellt.';

  @override
  String get projectExportFailedBeforeStart =>
      'Die Projektkopie konnte nicht exakt vorbereitet werden. Nichts wurde erstellt.';

  @override
  String get projectExportPrepublicationFailed =>
      'Der Export wurde sicher beendet, bevor die neue lokale Datei erstellt wurde. Nichts wurde erstellt. Schließe dieses Fenster und prüfe Projekt und Ziel, bevor du es erneut versuchst.';

  @override
  String projectExportMayExist(String output) {
    return 'Der Export hat keinen verifizierten Beleg geliefert. Nicht erneut versuchen. Schließe dieses Fenster und prüfe das Ziel: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'Der abgeschlossene Export stimmt nicht mit diesem Projektstand oder Ziel überein. Nicht erneut versuchen; prüfe das Ziel: $output';
  }

  @override
  String get projectExportPublished =>
      'Die exakte portable Projektkopie wurde als neue lokale Datei erstellt.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'Die exakte Projektkopie wurde als lokale Datei erstellt, aber die interne temporäre Bereinigung blieb unvollständig. Die erstellte Datei ist gültig; nicht erneut versuchen.';

  @override
  String projectExportPublicationUncertain(String output) {
    return 'Die lokale Datei könnte erstellt worden sein. Nicht erneut versuchen. Prüfe, ob dieses Ziel existiert: $output';
  }

  @override
  String get projectExportArchiveBytes => 'Archivgröße in Bytes';

  @override
  String get projectExportArchiveSha256 => 'Archiv-SHA-256';

  @override
  String get projectExportCurrentProjectUnchanged =>
      'Das aktuelle Projekt bleibt offen und unverändert. Spiel und Spielstände wurden nicht berührt.';

  @override
  String get managedVoiceTakeRemoveAction => 'Aus dieser Zeile entfernen…';

  @override
  String get managedVoiceTakeRemoveTooltip =>
      'Diese Aufnahme aus der aktuellen Dialogzeile und Sprache entfernen';

  @override
  String get managedVoiceTakeRemoveDialogTitle => 'Voice-Take entfernen?';

  @override
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  ) {
    return '„$take“ aus $line ($locale) entfernen?';
  }

  @override
  String get managedVoiceTakeRemoveScope =>
      'Nur die Verknüpfung für diese Dialogzeile und Sprache wird gelöst. Andere Verwendungen im Projekt bleiben unverändert.';

  @override
  String get managedVoiceTakeRemoveInternalRetention =>
      'Die Audiodatei bleibt intern gespeichert. Diese Aktion gibt keinen Projektspeicher frei und kann noch nicht rückgängig gemacht werden.';

  @override
  String get managedVoiceTakeRemoveGameBoundary =>
      'Spielinstallation und Spielstände werden nicht verändert.';

  @override
  String get managedVoiceTakeRemoveSelectedWarning =>
      'Dies ist der aktive Take. Beim Entfernen wird die Auswahl atomar geleert. Es wird kein Ersatz automatisch gewählt; der Voice-Build bleibt blockiert, bis ein freigegebener Take ausgewählt wurde.';

  @override
  String get managedVoiceTakeRemoveCancel => 'Abbrechen';

  @override
  String get managedVoiceTakeRemoveConfirm => 'Aus Zeile entfernen';

  @override
  String get managedVoiceTakeRemoveUniqueSuccess =>
      'Der Take wurde aus dieser Zeile und dem aktuellen Projektgraphen entfernt. Seine internen Audiodaten bleiben erhalten.';

  @override
  String get managedVoiceTakeRemoveSharedSuccess =>
      'Die Verknüpfung wurde aus dieser Zeile und Sprache gelöst. Der Take bleibt für andere Verwendungen im Projekt verfügbar; seine internen Audiodaten bleiben erhalten.';

  @override
  String get managedVoiceTakeRemoveSelectionClearedSuccess =>
      'Die aktive Auswahl wurde atomar geleert. Es wurde kein Ersatz gewählt; der Voice-Build bleibt blockiert, bis ein freigegebener Take ausgewählt wurde.';

  @override
  String get managedVoiceTakeRemoveStale =>
      'Das Projekt wurde geändert, bevor der Take entfernt werden konnte. Lade die aktuellen Voice-Takes neu und prüfe die Aktion erneut.';

  @override
  String get managedVoiceTakeRemoveRequiresReopen =>
      'Das Ergebnis der Entfernung konnte nicht bestätigt werden. Nicht erneut versuchen. Schließe dieses Fenster und öffne das verwaltete Projekt erneut oder stelle es wieder her.';

  @override
  String get managedVoiceTakeRemoveSavedUnconfirmed =>
      'Die Entfernung wurde gespeichert, aber der aktuelle Projektstand konnte nicht bestätigt werden. Wiederhole die Entfernung nicht. Schließe dieses Fenster und öffne das verwaltete Projekt erneut oder stelle es wieder her.';

  @override
  String get managedVoiceTakeRemoveSavedReloadFailed =>
      'Die Entfernung wurde gespeichert, aber die aktuellen Voice-Takes konnten nicht geladen werden. Lade die Takes neu; die Entfernung wird nicht wiederholt.';

  @override
  String managedVoiceTakeRemoveFailed(String error) {
    return 'Der Take wurde nicht entfernt: $error';
  }

  @override
  String get managedVoiceTakeRemoveReloadConfirmed =>
      'Die gespeicherte Entfernung wurde im aktuellen Projektstand bestätigt.';

  @override
  String get managedVoiceSlotRemoveAction => 'Leeres Voice-Setup entfernen…';

  @override
  String get managedVoiceSlotRemoveDialogTitle =>
      'Leeres Voice-Setup entfernen?';

  @override
  String managedVoiceSlotRemoveDialogSummary(String line, String locale) {
    return 'Das leere Voice-Setup für $locale aus $line entfernen?';
  }

  @override
  String get managedVoiceSlotRemoveRetention =>
      'Der Dialogtext bleibt im Projekt. Keine Aufnahme, kein Audio-Blob, keine Spieldatei und kein Spielstand werden gelöscht.';

  @override
  String get managedVoiceSlotRemoveTargetWarning =>
      'Dabei wird auch der gespeicherte Nachweis zum installierten Ziel für diese Zeile und Sprache entfernt. Das installierte Archiv selbst bleibt unberührt.';

  @override
  String get managedVoiceSlotRemoveRecreate =>
      'Du kannst später einen neuen Take hinzufügen; das benötigte Voice-Setup wird dann automatisch neu erstellt.';

  @override
  String get managedVoiceSlotRemoveCancel => 'Setup behalten';

  @override
  String get managedVoiceSlotRemoveConfirm => 'Setup entfernen';

  @override
  String get managedVoiceSlotRemoveSuccess =>
      'Das leere Voice-Setup wurde entfernt. Dialogtext, Audiospeicher, Spieldateien und Spielstände wurden nicht verändert.';

  @override
  String get managedVoiceSlotRemoveStale =>
      'Das Projekt wurde geändert, bevor das leere Voice-Setup entfernt werden konnte. Lade die aktuellen Voice-Takes neu und versuche es erneut.';

  @override
  String get managedVoiceSlotRemoveRequiresReopen =>
      'Öffne das verwaltete Projekt erneut, bevor du dieses Voice-Setup entfernst.';

  @override
  String get managedVoiceSlotRemoveSavedUnconfirmed =>
      'Das Ergebnis konnte nicht bestätigt werden; das leere Voice-Setup wurde möglicherweise gespeichert. Wiederhole die Entfernung nicht. Schließe dieses Fenster, öffne das verwaltete Projekt erneut und prüfe die Zeile.';

  @override
  String get managedVoiceSlotRemoveSavedReloadFailed =>
      'Das leere Voice-Setup wurde gespeichert, aber das Neuladen ist fehlgeschlagen. Lade neu, um die Änderung zu bestätigen; die Entfernung wird nicht wiederholt.';

  @override
  String managedVoiceSlotRemoveFailed(String error) {
    return 'Das leere Voice-Setup konnte nicht entfernt werden: $error';
  }

  @override
  String get managedVoiceSlotRemoveReloadConfirmed =>
      'Die gespeicherte Entfernung des leeren Voice-Setups wurde im aktuellen Projektstand bestätigt.';

  @override
  String get managedVoicePreviewTooltip =>
      'Ausgewählte lokale Ogg-Datei vorhören';

  @override
  String get managedVoicePreviewOpened =>
      'Die ausgewählte lokale Aufnahme wurde zur Autoren-Vorschau geöffnet. Dadurch wird das Audio weder freigegeben noch für das Spiel qualifiziert.';

  @override
  String managedVoicePreviewFailed(String error) {
    return 'Die lokale Aufnahme konnte nicht zur Vorschau geöffnet werden: $error';
  }

  @override
  String get managedStoryWorkbenchEditNpcProfile =>
      'Name & Archetyp bearbeiten';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepTitle =>
      'Nächster Schritt: Dialog & Sprachausgabe';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepDescription =>
      'Nur Entwurf: Fahre mit Begrüßungszeilen, Text und Sprachausgabe fort. Dabei werden nur Projektinhalte verknüpft; es entsteht kein spielbarer Dialog und kein Laufzeitnachweis.';

  @override
  String get managedStoryWorkbenchContinueToNpcDialogVoice =>
      'Weiter zu Dialog & Sprachausgabe';

  @override
  String get managedStoryWorkbenchNpcDisplayNameLabel => 'NPC-Name';

  @override
  String get managedNpcProfileEditTitle => 'Name & Archetyp bearbeiten';

  @override
  String get managedNpcProfileEditDescription =>
      'Ändere den sichtbaren NPC-Namen oder wähle einen anderen verifizierten strukturellen Ausgangspunkt.';

  @override
  String get managedNpcProfileEditNameLabel => 'NPC-Name';

  @override
  String get managedNpcProfileEditNameHint =>
      'Wird Mod-Autoren in diesem Projekt angezeigt.';

  @override
  String get managedNpcProfileEditArchetypeLabel => 'Archetyp / Ausgangsfigur';

  @override
  String get managedNpcProfileEditArchetypeHelp =>
      'Aussehen, Werte, Fraktion, Routine, Inventar, Dialog und Spawn werden hier nicht bearbeitet.';

  @override
  String get managedNpcProfileEditBoundary =>
      'Nur der Offline-Projektentwurf wird geändert. Spielinstallation und Spielstände bleiben unverändert.';

  @override
  String get managedNpcProfileEditLoading =>
      'Aktuelle NPC-Daten werden geladen…';

  @override
  String get managedNpcProfileEditCancel => 'Abbrechen';

  @override
  String get managedNpcProfileEditClose => 'Schließen';

  @override
  String get managedNpcProfileEditSave => 'Änderungen speichern';

  @override
  String get managedNpcProfileEditSaving => 'Wird gespeichert…';

  @override
  String get managedNpcProfileEditRetry => 'Erneut versuchen';

  @override
  String get managedNpcProfileEditLoadFailed =>
      'NPC-Daten und verifizierte Archetypen konnten nicht geladen werden. Es wurden keine Dateien geändert.';

  @override
  String get managedNpcProfileEditCatalogChanged =>
      'Die verifizierten Archetypen haben sich geändert. Prüfe und wähle den Archetyp vor dem Speichern erneut.';

  @override
  String get managedNpcProfileEditCurrentArchetypeUnavailable =>
      'Der aktuelle NPC-Archetyp ist in diesem Spielkatalog nicht mehr exakt abbildbar. Es wurde kein Ersatz geraten.';

  @override
  String get managedNpcProfileEditStale =>
      'Das Projekt wurde geändert. Schließe den Editor und öffne den NPC aus der aktualisierten Story-Ansicht erneut.';

  @override
  String get managedNpcProfileEditRequiresReopen =>
      'Das Speicherergebnis kann nicht verifiziert werden. Nicht erneut versuchen. Schließe den Editor und öffne das verwaltete Projekt erneut oder stelle es wieder her.';

  @override
  String get managedNpcProfileEditSaveFailed =>
      'Die NPC-Änderungen konnten nicht sicher gespeichert werden. Es wurde nichts gebaut, installiert oder in das Spiel geschrieben.';

  @override
  String get managedNpcProfileEditNameRequired => 'Gib einen NPC-Namen ein.';

  @override
  String get managedNpcProfileEditNameTooLong =>
      'Der NPC-Name darf höchstens 256 UTF-8-Bytes lang sein.';

  @override
  String get managedNpcProfileEditNameControl =>
      'Der NPC-Name enthält ein nicht unterstütztes Steuerzeichen.';

  @override
  String get managedNpcProfileEditReviewSelection =>
      'Prüfe und wähle vor dem Speichern einen Archetyp.';

  @override
  String get managedNpcProfileEditDiscardTitle => 'NPC-Änderungen verwerfen?';

  @override
  String get managedNpcProfileEditDiscardBody =>
      'Der ungespeicherte Name und die Archetyp-Auswahl gehen verloren.';

  @override
  String get managedNpcProfileEditKeepEditing => 'Weiter bearbeiten';

  @override
  String get managedNpcProfileEditDiscard => 'Verwerfen';

  @override
  String managedNpcProfileEditSaved(String name, int revision) {
    return '$name wurde in Projektrevision $revision gespeichert. Der NPC bleibt ein build-blockierter Offline-Entwurf.';
  }

  @override
  String get managedVoiceBuildReadinessTitle => 'Voice-Bereitschaft';

  @override
  String get managedVoiceBuildReadinessRefresh =>
      'Voice-Bereitschaft aktualisieren';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Exakte Voice-Bereitschaft wird geprüft';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Die Voice-Bereitschaft des aktuellen Projekts konnte nicht verifiziert werden. Aus diesem Ergebnis ist kein Build verfügbar.';

  @override
  String get managedVoiceBuildReadinessReadyTitle => 'Voice ist bereit';

  @override
  String get managedVoiceBuildReadinessBlockedTitle =>
      'Voice benötigt Aufmerksamkeit';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots von $totalSlots Voice-Slots sind bereit.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'Es wurde kein Bundle erstellt und nichts bereitgestellt.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Bundle bauen';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Der Voice-Inhalt ist bereit. Öffne Build & Release, um das Offline-Bundle zu erstellen.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Der Voice-Inhalt ist bereit. Konfiguriere die Spielinstallation, bevor du ein Offline-Bundle erstellst.';

  @override
  String get managedVoiceBuildReadinessHideBlockers => 'Blocker ausblenden';

  @override
  String managedVoiceBuildReadinessShowBlockers(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Blocker anzeigen',
      one: '1 Blocker anzeigen',
    );
    return '$_temp0';
  }

  @override
  String get managedVoiceBuildReadinessWorkflowFailed =>
      'Der ausgewählte Voice-Workflow konnte nicht geöffnet werden. Aktualisiere die Ansicht und versuche es erneut.';

  @override
  String get managedVoiceBuildReadinessBuildWorkflowFailed =>
      'Der Voice-Build-Workflow konnte nicht geöffnet werden.';

  @override
  String managedVoiceBuildReadinessExactRevision(int revision) {
    return 'Exakte Projektrevision $revision';
  }

  @override
  String get managedVoiceBuildReadinessResolveTarget => 'Ziel auflösen';

  @override
  String get managedVoiceBuildReadinessManageTakes => 'Takes verwalten';

  @override
  String get managedVoiceBuildBlockerNoSlots =>
      'Dieses Projekt enthält keine Voice-Setups.';

  @override
  String get managedVoiceBuildBlockerPayloadBudget =>
      'Die ausgewählten Voice-Aufnahmen überschreiten das sichere Speicherbudget des Bundles.';

  @override
  String get managedVoiceBuildBlockerUnresolvedTarget =>
      'Löse dieses Voice-Ziel auf.';

  @override
  String get managedVoiceBuildBlockerAmbiguousTarget =>
      'Dieses Voice-Ziel ist nicht eindeutig.';

  @override
  String get managedVoiceBuildBlockerUnqualifiedAdd =>
      'Dieses Ziel ist kein versiegelter Ersatz eines vorhandenen Eintrags.';

  @override
  String get managedVoiceBuildBlockerMissingTake =>
      'Wähle einen freigegebenen Voice-Take aus.';

  @override
  String get managedVoiceBuildBlockerTakeNotApproved =>
      'Der ausgewählte Voice-Take ist nicht freigegeben.';

  @override
  String get managedVoiceBuildBlockerCodecUnqualified =>
      'Der ausgewählte Voice-Take verwendet einen nicht unterstützten Codec.';

  @override
  String get managedVoiceBuildBlockerSlotLimit =>
      'Dieses Projekt überschreitet das Limit von 1024 Voice-Slots pro Bundle.';

  @override
  String get managedVoiceBuildOfflineNotice =>
      'Nur Offline-Build. Dadurch wird ein versiegeltes Voice-Bundle aus vorhandenen Einträgen erstellt. Es wird weder bereitgestellt noch in das Spiel geschrieben.';

  @override
  String get managedVoiceBuildNewFolderName => 'Name des neuen Ordners';

  @override
  String get managedVoiceBuildNewFolderHelp =>
      'Das Bundle muss in einen völlig neuen Unterordner geschrieben werden.';

  @override
  String get managedVoiceBuildChooseParent => 'Übergeordneten Ordner wählen';

  @override
  String get managedVoiceBuildNoParentSelected =>
      'Kein übergeordneter Ordner ausgewählt';

  @override
  String get managedVoiceBuildNewOutput => 'Neue Ausgabe';

  @override
  String get managedVoiceBuildOfflineBundle => 'Offline-Bundle bauen';

  @override
  String get managedVoiceBuildParentInspectFailed =>
      'Der übergeordnete Ordner konnte nicht sicher geprüft werden. Es wurde weder gebaut noch etwas bereitgestellt.';

  @override
  String get managedVoiceBuildChooseExistingParent =>
      'Wähle einen vorhandenen übergeordneten Ordner aus.';

  @override
  String get managedVoiceBuildTargetSymlink =>
      'Der Zielpfad ist ein symbolischer Link. Wähle einen anderen neuen Ordnernamen.';

  @override
  String get managedVoiceBuildTargetExists =>
      'Das Ziel existiert bereits. Wähle einen anderen neuen Ordnernamen.';

  @override
  String get managedVoiceBuildRequiresReopen =>
      'Dieses Projekt kann nicht mehr als aktuell verifiziert werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut, bevor du ein weiteres Voice-Bundle baust.';

  @override
  String get managedVoiceBuildStaleCheckpoint =>
      'Das verwaltete Projekt wurde geändert, während dieses Fenster geöffnet war. Schließe dieses Build-Fenster und öffne es erneut aus dem aktuellen Projekt.';

  @override
  String get managedVoiceBuildFailed =>
      'Das Voice-Bundle konnte nicht exakt gebaut werden. Es wurde nichts bereitgestellt. Falls eine Ausgabe angelegt wurde, wähle vor dem nächsten Versuch einen neuen Ordnernamen.';

  @override
  String get managedVoiceBuildPlanFailed =>
      'Die Voice-Bereitschaft des exakten aktuellen Projekts konnte nicht verifiziert werden. Ausgabeauswahl und Build bleiben gesperrt, bis die Verifizierung erfolgreich ist.';

  @override
  String get managedVoiceBuildParentAbsolute =>
      'Wähle einen absoluten Pfad zu einem vorhandenen übergeordneten Ordner.';

  @override
  String get managedVoiceBuildParentSymlink =>
      'Der ausgewählte übergeordnete Ordner ist ein symbolischer Link. Wähle einen echten vorhandenen Ordner.';

  @override
  String get managedVoiceBuildFolderRequired =>
      'Gib einen neuen Ordnernamen ein.';

  @override
  String get managedVoiceBuildFolderWhitespace =>
      'Der Ordnername darf nicht mit Leerraum beginnen oder enden.';

  @override
  String get managedVoiceBuildFolderTooLong => 'Der Ordnername ist zu lang.';

  @override
  String get managedVoiceBuildFolderPortable =>
      'Verwende einen portablen Ordnernamen ohne Trennzeichen oder reservierte Zeichen.';

  @override
  String get managedVoiceBuildFolderWindowsReserved =>
      'Dieser Ordnername ist unter Windows reserviert.';

  @override
  String get managedVoiceBuildExecutableUnavailable =>
      'Die installierte Spiel-EXE konnte nicht gelesen werden. Schließe ein laufendes Spielupdate ab und prüfe die konfigurierte Installation. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildExecutableMismatch =>
      'Die installierte Spiel-EXE entspricht nicht mehr dieser Projektgeneration. Importiere das verwaltete Projekt neu oder richte es neu aus. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildGameUnavailable =>
      'Die konfigurierte Gothic-1-Remake-Installation ist nicht verfügbar. Prüfe sie in den Einstellungen. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildStoreGameAlias =>
      'Der Projektordner überschneidet sich mit der konfigurierten Spielinstallation. Verschiebe das Projekt aus dem Spielordner, bevor du baust. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildGameOutputAlias =>
      'Die Bundle-Ausgabe überschneidet sich mit einer Gothic-1-Remake-Installation. Wähle einen übergeordneten Ordner außerhalb aller Spielinstallationen. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildStoreOutputAlias =>
      'Die Bundle-Ausgabe überschneidet sich mit dem verwalteten Projekt. Wähle einen übergeordneten Ordner außerhalb des Projekts. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildOutputUnavailable =>
      'Der ausgewählte Ausgabeordner ist nicht verfügbar oder kann nicht sicher durchlaufen werden. Wähle einen echten vorhandenen Ordner außerhalb von Projekt und Spiel.';

  @override
  String get managedVoiceBuildOutputFailed =>
      'Der neue Bundle-Ordner konnte nicht vollständig geschrieben werden. Verwende keine dort verbliebene Ausgabe und wähle vor dem nächsten Versuch einen anderen neuen Ordnernamen. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildPromotionFailed =>
      'Das versiegelte Bundle konnte nicht in den gewünschten neuen Ausgabeordner übernommen werden. Eine kollidierende Ausgabe blieb unverändert; das eigene Staging wurde entfernt. Wähle vor dem nächsten Versuch einen anderen neuen Ordnernamen. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildCleanupFailed =>
      'Das Voice-Bundle wurde nicht veröffentlicht, aber sein temporärer Staging-Ordner konnte nicht vollständig entfernt werden. Entferne den gemeldeten Staging-Ordner vor einem neuen Versuch. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildPublicationUnconfirmed =>
      'Die atomare Veröffentlichung könnte erfolgreich gewesen sein, ihre endgültige Identität oder Dauerhaftigkeit konnte jedoch nicht bestätigt werden. Wiederhole, ersetze oder lösche diese exakte Ausgabe noch nicht. Schließe das Fenster und prüfe den gemeldeten Ordner. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildStoreRootChanged =>
      'Der Stammordner des verwalteten Projekts wurde während des Builds geändert. Schließe dieses Fenster und öffne das Projekt erneut. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildGameRootChanged =>
      'Die Spielinstallation wurde während des Builds geändert. Schließe das Update oder den Dateivorgang ab und versuche es mit einem neuen Ordnernamen erneut. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildOutputRootChanged =>
      'Der übergeordnete Ausgabeordner wurde während des Builds geändert. Schließe den Dateivorgang ab, prüfe den Ordner und versuche es mit einem neuen Ordnernamen erneut. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildVerifyFailed =>
      'Das geschriebene Bundle konnte nicht exakt verifiziert werden. Verwende diese Ausgabe nicht und wähle vor dem nächsten Versuch einen anderen neuen Ordnernamen. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildBundleInvalid =>
      'Der ausgewählte Voice-Inhalt konnte nicht in ein exaktes versiegeltes Bundle überführt werden. Öffne das Projekt erneut, prüfe seine Voice-Slots und versuche es noch einmal. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildInputInvalid =>
      'Die Voice-Build-Anfrage oder der Ausgabepfad überschreitet die sicheren unterstützten Grenzen. Wähle einen kürzeren neuen Ausgabepfad. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildResponseLimit =>
      'Das Bundle war zu groß, um eine exakte Build-Quittung zurückzugeben. Verwende keine Ausgabe ohne Quittung; reduziere zuerst den Voice-Build und wähle erst danach einen neuen Ordner. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildBuiltTitle => 'Versiegeltes Voice-Bundle gebaut';

  @override
  String get managedVoiceBuildOfflineReceipt =>
      'Nur Offline-Quittung. Es wurde nichts bereitgestellt.';

  @override
  String get managedVoiceBuildBasisRevision => 'Basis-Projektrevision';

  @override
  String get managedVoiceBuildOutputLabel => 'Ausgabe';

  @override
  String get managedVoiceBuildArchiveEdits => 'Archivänderungen';

  @override
  String get managedVoiceBuildBundleFiles => 'Bundle-Dateien';

  @override
  String get managedVoiceBuildSealedBytes => 'Versiegelte Bytes';

  @override
  String get managedVoiceBuildBundleSha256 => 'Bundle-SHA-256';

  @override
  String get managedVoiceBuildParentPickerTitle =>
      'Übergeordneten Ordner für das Voice-Bundle wählen';

  @override
  String managedVoiceBuildBuiltMessage(String output) {
    return 'Das versiegelte Voice-Bundle wurde unter $output gebaut. Es wurde nichts bereitgestellt.';
  }

  @override
  String managedVoiceBuildBlockedMessage(int count) {
    return 'Der Voice-Build ist durch $count exakte Anforderungen blockiert. Es wurde kein Bundle erstellt oder bereitgestellt.';
  }
}
