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
}
