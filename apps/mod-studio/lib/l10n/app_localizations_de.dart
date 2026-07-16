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
  String get projectOpenManagedRevision3 =>
      'Verwaltetes Revision-3-Projekt öffnen…';

  @override
  String get projectVerifyCurrentHead => 'Aktuellen Head verifizieren';

  @override
  String get projectManagedRevision3Title => 'Verwaltetes Revision-3-Projekt';

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
  String projectManagedRevision3Opened(String projectId) {
    return 'Verwaltetes Revision-3-Projekt $projectId geöffnet';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Verwaltetes Revision-3-Projekt konnte nicht geöffnet werden: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Revision-3-Head $headSha256 verifiziert';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Verifizierung des Revision-3-Heads fehlgeschlagen: $error';
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
  String get managedWorkspaceSettingsExpertLabel =>
      'Einstellungen & Expertenmodus';

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
  String get managedSectionWorldDescription =>
      'Weltplatzierung und zugehörige Arbeitsabläufe sind geplant.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Schreibe und übersetze Projektdialoge an einem Ort und arbeite anschließend direkt an der Sprachausgabe weiter.';

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
  String get managedActionNewQuestTitle => 'Neue Quest';

  @override
  String get managedActionNewQuestDescription =>
      'Erstelle einen Offline-Quest-Entwurf mit Zielen und verifizierten übergeordneten Identitäten.';

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
  String get managedStoryWorkbenchOverviewTab => 'Überblick';

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
}
