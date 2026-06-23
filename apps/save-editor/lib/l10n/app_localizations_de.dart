// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get appTitle => 'Gothic Remake Spielstand-Editor';

  @override
  String get appLogoSemanticLabel => 'goresave-Logo';

  @override
  String get zoomTooltip => 'Strg +/- zum Vergrößern/Verkleinern drücken';

  @override
  String get switchToLightMode => 'Zum hellen Modus wechseln';

  @override
  String get switchToDarkMode => 'Zum dunklen Modus wechseln';

  @override
  String get about => 'Über';

  @override
  String get tabOverview => 'Übersicht';

  @override
  String get tabPlayer => 'Spieler';

  @override
  String get tabInventory => 'Inventar';

  @override
  String get tabProgression => 'Fortschritt';

  @override
  String get tabAllData => 'Alle Daten';

  @override
  String get tabBackups => 'Sicherungen';

  @override
  String get tabSettings => 'Einstellungen';

  @override
  String get reset => 'Zurücksetzen';

  @override
  String get save => 'Speichern';

  @override
  String saveWithCount(int count) {
    return 'Speichern ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Abbrechen';

  @override
  String get confirm => 'Bestätigen';

  @override
  String get close => 'Schließen';

  @override
  String get add => 'Hinzufügen';

  @override
  String get browse => 'Durchsuchen';

  @override
  String get noSavFilesFound => 'Keine .sav-Dateien gefunden';

  @override
  String get profile => 'Profil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count Spielstände)';
  }

  @override
  String get switchProfile => 'Profil wechseln';

  @override
  String get rescanSaveFolder => 'Speicherordner neu einlesen';

  @override
  String get discardUnsavedChangesTitle =>
      'Ungespeicherte Änderungen verwerfen?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Änderungen',
      one: 'Änderung',
    );
    return 'Beim erneuten Einlesen wird jeder Spielstand neu geladen und $count ungespeicherte $_temp0 verworfen.';
  }

  @override
  String get discardAndRescan => 'Verwerfen und neu einlesen';

  @override
  String chapterLabel(Object id) {
    return 'Kapitel $id';
  }

  @override
  String get quickSave => 'Schnellspeicherung';

  @override
  String get autoSave => 'Automatische Speicherung';

  @override
  String get manualSave => 'Manuelle Speicherung';

  @override
  String get errorTitle => 'Fehler';

  @override
  String get selectASaveTitle => 'Spielstand auswählen';

  @override
  String get selectASaveBody => 'Die Spielstanddetails werden hier angezeigt.';

  @override
  String get diagnosticsTitle => 'Diagnose & Details';

  @override
  String get diagnosticsSubtitle => 'Schreibgeschützte Formatprüfung';

  @override
  String get metricFormat => 'Format';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Kapitel';

  @override
  String get metricTimePlayed => 'Spielzeit';

  @override
  String get metricSaveKind => 'Speicherart';

  @override
  String get metricFileSize => 'Dateigröße';

  @override
  String get metricCompression => 'Komprimierung';

  @override
  String get metricChunks => 'Chunks';

  @override
  String get metricUncompressed => 'Unkomprimiert';

  @override
  String get metricPrivate => 'Privat';

  @override
  String get metricSlotName => 'Slot-Name';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Entschlüsselt privat';

  @override
  String get metricPrivateStrings => 'Private Strings';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count Bytes';
  }

  @override
  String get inspectionJsonTitle => 'Inspektions-JSON';

  @override
  String get inspectionJsonSubtitle => 'Rohdaten der Spielstandprüfung';

  @override
  String get copy => 'Kopieren';

  @override
  String get savegameFallbackTitle => 'Spielstand';

  @override
  String screenshotForSlot(String slot) {
    return 'Screenshot für $slot';
  }

  @override
  String get publicSaveName => 'Öffentlicher Speichername';

  @override
  String get required => 'Erforderlich';

  @override
  String get playerLockedBody =>
      'Private Spielerbearbeitungen benötigen einen komprimierfähigen Codec.';

  @override
  String get heroTransform => 'Helden-Transform';

  @override
  String get locationX => 'Position X';

  @override
  String get locationY => 'Position Y';

  @override
  String get locationZ => 'Position Z';

  @override
  String get rotationPitch => 'Neigung (Pitch)';

  @override
  String get rotationYaw => 'Gierung (Yaw)';

  @override
  String get rotationRoll => 'Rollung (Roll)';

  @override
  String get invalid => 'Ungültig';

  @override
  String get heroAttributes => 'Heldenattribute';

  @override
  String attributeBase(String name) {
    return '$name Basiswert';
  }

  @override
  String attributeCurrent(String name) {
    return '$name aktuell';
  }

  @override
  String get inventoryTitle => 'Inventar';

  @override
  String get inventoryNeedsDecoded =>
      'Die Inventarbearbeitung benötigt entschlüsselte private Nutzdaten vom Codec.';

  @override
  String get inventoryNoStacks =>
      'Keine Item-Stapel in den entschlüsselten privaten Nutzdaten gefunden.';

  @override
  String get resetInventoryChanges => 'Inventaränderungen zurücksetzen';

  @override
  String get addItemTooltipPendingAdd =>
      'Ausstehende Änderungen zuerst speichern — ein neues Item pro Speichervorgang';

  @override
  String get addItemTooltipPendingRemove =>
      'Ausstehende Entfernung zuerst speichern — eine strukturelle Änderung pro Speichervorgang';

  @override
  String get addItemTooltipPendingCount =>
      'Ausstehende Anzahländerungen zuerst speichern oder zurücksetzen — eine strukturelle Bearbeitung muss separat gespeichert werden';

  @override
  String get addItemTooltipDefault => 'Item zum Inventar hinzufügen';

  @override
  String get addItemButton => 'Item hinzufügen';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — ausstehende Hinzufügung (noch nicht gespeichert)';
  }

  @override
  String get cancelPendingAdd => 'Ausstehende Hinzufügung abbrechen';

  @override
  String get pendingRemovalSubtitle =>
      'ausstehende Entfernung (noch nicht gespeichert)';

  @override
  String get cancelPendingRemoval => 'Ausstehende Entfernung abbrechen';

  @override
  String get filterItems => 'Items filtern';

  @override
  String noItemsMatchQuery(String query) {
    return 'Keine Items entsprechen „$query“.';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Die ausstehende Entfernung blendet jedes Item aus — zum Anwenden speichern.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Nahkampfwaffen';

  @override
  String get itemCategoryRangedWeapon => 'Fernkampfwaffen';

  @override
  String get itemCategoryAmmunition => 'Munition';

  @override
  String get itemCategoryRune => 'Runen';

  @override
  String get itemCategoryScroll => 'Zauberschriftrollen';

  @override
  String get itemCategoryFood => 'Essen & Tränke';

  @override
  String get itemCategoryMisc => 'Verschiedenes';

  @override
  String get itemCategoryAmulet => 'Amulette';

  @override
  String get itemCategoryRing => 'Ringe';

  @override
  String get itemCategoryTrophy => 'Tiertrophäen';

  @override
  String get itemCategoryWriting => 'Schriften';

  @override
  String get itemCategoryMission => 'Questgegenstände';

  @override
  String get itemCategoryKey => 'Schlüssel';

  @override
  String get itemCategoryOther => 'Sonstiges';

  @override
  String get count => 'Anzahl';

  @override
  String get min1 => 'Min. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Löschen nicht möglich: Dieses Item ist wahrscheinlich ausgerüstet oder einem Schnelltasten-Slot zugewiesen';

  @override
  String get removeBlockedTooltip =>
      'Ausstehende Inventaränderungen zuerst speichern oder zurücksetzen — eine Hinzufügung oder Entfernung muss separat gespeichert werden';

  @override
  String get removeItemFromInventory => 'Item aus Inventar entfernen';

  @override
  String get progressionLockedBody =>
      'Fortschrittsdaten benötigen entschlüsselte private Nutzdaten vom Codec.';

  @override
  String get progressionNeedsTyped =>
      'Strukturierte Fortschrittsdaten benötigen einen vollständig entschlüsselten Spielstand mit verifizierter typisierter Auswertung.';

  @override
  String get sectionQuests => 'Quests';

  @override
  String get sectionKnowledge => 'Wissen';

  @override
  String get sectionEvents => 'Ereignisse';

  @override
  String get firstPage => 'Erste Seite';

  @override
  String get previousPage => 'Vorherige Seite';

  @override
  String get nextPage => 'Nächste Seite';

  @override
  String get lastPage => 'Letzte Seite';

  @override
  String pageOfPages(int page, int total) {
    return 'Seite $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last von $total';
  }

  @override
  String get perPage => 'Pro Seite:';

  @override
  String get resetQuestChanges => 'Quest-Änderungen zurücksetzen';

  @override
  String get searchQuests => 'Quests suchen';

  @override
  String get allGroups => 'Alle Gruppen';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Keiner';

  @override
  String get questStateAvailable => 'Verfügbar';

  @override
  String get questStateRunning => 'Laufend';

  @override
  String get questStateSucceeded => 'Abgeschlossen';

  @override
  String get questStateFailed => 'Fehlgeschlagen';

  @override
  String get questStateUnknown => 'unbekannt';

  @override
  String get dialogKnowledge => 'Dialogwissen';

  @override
  String get resetKnowledgeChanges => 'Wissensänderungen zurücksetzen';

  @override
  String get addNpc => 'NPC hinzufügen';

  @override
  String get searchNpcs => 'NPCs suchen';

  @override
  String entriesForCharacter(String name) {
    return 'Einträge — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Wähle einen NPC, um Einträge zu sehen';

  @override
  String get addKnowledgeEntry => 'Wissenseintrag hinzufügen';

  @override
  String get browseCatalog => 'Katalog durchsuchen';

  @override
  String get alreadyExistsForCharacter =>
      'Existiert bereits für diesen Charakter.';

  @override
  String get alreadyInPendingChanges =>
      'Bereits in den ausstehenden Änderungen.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Duplikatprüfung fehlgeschlagen — erneut versuchen: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Ausstehende Hinzufügungen ($count)';
  }

  @override
  String get undoAdd => 'Hinzufügung rückgängig machen';

  @override
  String get undoRemove => 'Entfernung rückgängig machen';

  @override
  String get removeEntry => 'Eintrag entfernen';

  @override
  String get selectNpcFromList => 'Wähle einen NPC aus der Liste';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Erinnerungsereignisse';

  @override
  String get searchCharacters => 'Charaktere suchen';

  @override
  String eventsForCharacter(String name) {
    return 'Ereignisse — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Wähle einen Charakter, um Ereignisse zu sehen';

  @override
  String get noTags => '(keine Tags)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Ereignis entfernen';

  @override
  String get removeMemoryEventTitle => 'Erinnerungsereignis entfernen?';

  @override
  String get removeMemoryEventBody =>
      'Dieses Erinnerungsereignis entfernen? Zuvor wird eine Sicherung erstellt.';

  @override
  String get duplicateEvent => 'Ereignis duplizieren';

  @override
  String get duplicateMemoryEventTitle => 'Erinnerungsereignis duplizieren?';

  @override
  String get duplicateMemoryEventBody =>
      'Dieses Erinnerungsereignis duplizieren? Zuvor wird eine Sicherung erstellt.';

  @override
  String get selectCharacterFromList => 'Wähle einen Charakter aus der Liste';

  @override
  String get allDataLockedBody =>
      'Der vollständige Eigenschaften-Browser benötigt entschlüsselte private Nutzdaten vom Codec.';

  @override
  String get allDataDescription =>
      'Durchsuche jede typisierte Eigenschaft nach Name oder Pfad. Skalare, Strings, Enums und Objektpfade sind bearbeitbar; Structs werden vorerst schreibgeschützt angezeigt.';

  @override
  String get searchPropertiesLabel =>
      'Eigenschaften suchen (leer = alles anzeigen) — z. B. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Spielstand wird entschlüsselt…';

  @override
  String get decodingSaveBody =>
      'Die vollständigen privaten Nutzdaten werden für die erste Suche entschlüsselt. Dies geschieht einmal pro Spielstand, danach sind Suchen sofort möglich.';

  @override
  String get searchTheSaveTitle => 'Spielstand durchsuchen';

  @override
  String get searchTheSaveBody =>
      'Gib einen Eigenschaftsnamen ein und drücke die Eingabetaste. Lass das Feld leer, um alles anzuzeigen.';

  @override
  String get searchFailedTitle => 'Suche fehlgeschlagen';

  @override
  String get noMatchesTitle => 'Keine Treffer';

  @override
  String get noMatchesBody =>
      'Kein Eigenschaftspfad enthielt all diese Begriffe.';

  @override
  String get value => 'Wert';

  @override
  String get backupsTitle => 'Sicherungen';

  @override
  String get refreshBackups => 'Sicherungen aktualisieren';

  @override
  String get noBackupsTitle => 'Keine Sicherungen';

  @override
  String get noBackupsBody =>
      'Bearbeitete Spielstände erzeugen Sicherungsdateien neben dem ausgewählten Slot.';

  @override
  String get slotBackups => 'Slot-Sicherungen';

  @override
  String get profileBackups => 'Profil-Sicherungen';

  @override
  String get backupFactName => 'Name';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Erstellt';

  @override
  String get backupFactSize => 'Größe';

  @override
  String get backupFactStatus => 'Status';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '$fileName wiederherstellen';
  }

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
  String get language => 'Sprache';

  @override
  String get updatesTitle => 'Updates';

  @override
  String get checkForUpdatesAutomatically => 'Automatisch nach Updates suchen';

  @override
  String get checkForUpdatesNow => 'Jetzt nach Updates suchen';

  @override
  String get updatesPortableNotice =>
      'Updates sind nur für installierte Versionen verfügbar. Die portable Version muss manuell aktualisiert werden.';

  @override
  String get gameTextTitle => 'Spieltext';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extrahiert: $ids IDs über $languages Sprachen.';
  }

  @override
  String get gameTextExtracted => 'Lokalisierter Spieltext ist extrahiert.';

  @override
  String get gameTextNotExtracted =>
      'Lokalisierter Spieltext ist noch nicht extrahiert.';

  @override
  String get extracting => 'Wird extrahiert…';

  @override
  String get extractRefreshLocalizedText =>
      'Lokalisierten Text extrahieren / aktualisieren';

  @override
  String get extractLocalizedTextTitle =>
      'Lokalisierten Spieltext extrahieren?';

  @override
  String get extractLocalizedTextBody =>
      'Lokalisierter Spieltext ist noch nicht extrahiert. Jetzt aus deiner Spielinstallation extrahieren? (optional)';

  @override
  String get notNow => 'Nicht jetzt';

  @override
  String get extract => 'Extrahieren';

  @override
  String get extractionComplete => 'Extraktion abgeschlossen';

  @override
  String get extractionFailed => 'Extraktion fehlgeschlagen';

  @override
  String get localizationCacheFileType => 'Lokalisierungs-Cache';

  @override
  String get savegameDirectoryTitle => 'Spielstandverzeichnis';

  @override
  String get folder => 'Ordner';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Prüfen';

  @override
  String get roundtrip => 'Roundtrip';

  @override
  String get noCodecStatus => 'Kein Codec-Status';

  @override
  String get codecReady => 'Codec bereit';

  @override
  String get codecReadOnly => 'Codec schreibgeschützt';

  @override
  String get codecUnavailable => 'Codec nicht verfügbar';

  @override
  String get details => 'Details';

  @override
  String codecStatusLine(String status) {
    return 'Status: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Dekomprimieren: $decompress | Komprimieren: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'ja';

  @override
  String get no => 'nein';

  @override
  String get aboutSubtitle => 'Gothic Remake Spielstand-Editor';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 goresave-Mitwirkende';

  @override
  String get aboutLicense => 'Lizenziert unter der MIT-Lizenz.';

  @override
  String difficultyTitle(String profile) {
    return 'Schwierigkeit — $profile';
  }

  @override
  String get difficultyNoProfile => 'Kein Profil';

  @override
  String get difficultyNoDifficulty => 'Keine Schwierigkeit';

  @override
  String get difficultyLabel => 'Schwierigkeit';

  @override
  String get difficultyTooltipNoProfile => 'Kein Profil ausgewählt';

  @override
  String get difficultyTooltipEdit =>
      'Schwierigkeit für dieses Profil bearbeiten';

  @override
  String get difficultyTooltipNoEditable =>
      'Dieses Profil hat keine bearbeitbare Schwierigkeit';

  @override
  String get preset => 'Voreinstellung';

  @override
  String get presetNovice => 'Anfänger';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Schwer';

  @override
  String get presetCustom => 'Benutzerdefiniert';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Die gespeicherte Voreinstellung wird nicht erkannt ($preset). Du kannst trotzdem Änderungen am Kampffluss-Helfer / Permadeath speichern oder oben eine Voreinstellung wählen, um sie zu überschreiben.';
  }

  @override
  String get closeCombatFlowHelper => 'Nahkampf-Flusshelfer';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Nicht verfügbar auf Anfänger';

  @override
  String get levelCombat => 'Kampf';

  @override
  String get levelResources => 'Ressourcen';

  @override
  String get levelProgression => 'Fortschritt';

  @override
  String get difficultyAppliesToAllSaves =>
      'Die Schwierigkeit gilt für alle Spielstände in diesem Profil.';

  @override
  String get savingDifficultyFailed =>
      'Speichern der Schwierigkeit fehlgeschlagen.';

  @override
  String get addItemDialogTitle => 'Item hinzufügen';

  @override
  String get searchItems => 'Items suchen';

  @override
  String failedToLoadCatalog(String error) {
    return 'Katalog konnte nicht geladen werden: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Keine Items zum Hinzufügen verfügbar';

  @override
  String get noItemsMatch => 'Keine passenden Items';

  @override
  String get countMustBeAtLeast1 => 'Muss ≥ 1 sein';

  @override
  String countMustBeAtMost(int max) {
    return 'Muss ≤ $max sein';
  }

  @override
  String get addNpcDialogTitle => 'NPC hinzufügen';

  @override
  String get noNpcsAvailableToAdd => 'Keine NPCs zum Hinzufügen verfügbar';

  @override
  String get noNpcsMatch => 'Keine passenden NPCs';

  @override
  String get categoryAll => 'Alle';

  @override
  String allWithCount(int count) {
    return 'Alle ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Wissenseintrag hinzufügen';

  @override
  String get searchEntries => 'Einträge suchen';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Keine Wissenseinträge zum Hinzufügen verfügbar';

  @override
  String get noEntriesMatch => 'Keine passenden Einträge';

  @override
  String get heroGroupMainStats => 'Hauptwerte';

  @override
  String get heroGroupCombatSkills => 'Kampffertigkeiten';

  @override
  String get heroGroupResistances => 'Widerstände';

  @override
  String get heroGroupThieving => 'Diebeskunst';

  @override
  String get heroGroupAdvanced => 'Erweitert';

  @override
  String get heroEntryHeroTransform => 'Helden-Transform';

  @override
  String attributeEmpty(String name) {
    return '$name ist leer — gib einen Wert ein oder stelle den ursprünglichen Wert wieder her, bevor du speicherst.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Ungültige Zahl für $name: „$text“';
  }

  @override
  String get loadingEditorData => 'Editordaten werden geladen';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs in $languageCount Sprachen extrahiert';
  }
}
