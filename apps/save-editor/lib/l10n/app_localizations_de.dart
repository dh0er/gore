// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get debugSectionTitle => 'Erweitert (Debug)';

  @override
  String get debugSectionSubtitle => 'Diagnose & Rohdaten für Fehlerberichte';

  @override
  String get showObjectIdsTitle => 'Zusätzliche technische IDs anzeigen';

  @override
  String get showObjectIdsSubtitle =>
      'Technische IDs von Gegenständen, Dialogwissen, Quests und verwaisten Akteuren im Editor anzeigen. NPC-IDs werden immer angezeigt.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logo des GORE Save Editors';

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
  String get tabAttribute => 'Attribute';

  @override
  String get heroGroupSkills => 'Talente';

  @override
  String get skillsNoneBody =>
      'Für diesen Charakter wurden keine Talente gefunden.';

  @override
  String get skillsUnavailableBody =>
      'Talente lassen sich in diesem Spielstand nicht bearbeiten — der Held hat keine Effektdaten.';

  @override
  String get skillNotLearned => 'Nicht gelernt';

  @override
  String get skillLearn => 'Lernen';

  @override
  String get skillActionLearn => 'lernen';

  @override
  String get skillActionUnlearn => 'verlernen';

  @override
  String get skillTierUntrained => 'Untrainiert';

  @override
  String get skillTierBeginner => 'Anfänger';

  @override
  String get skillTierTrained => 'Trainiert';

  @override
  String get skillTierMaster => 'Meister';

  @override
  String get skillTierNovice => 'Novize';

  @override
  String get skillTierAmateur => 'Amateur (Kreis 0)';

  @override
  String get skillTierLearned => 'Gelernt';

  @override
  String skillTierCircle(int n) {
    return 'Kreis $n';
  }

  @override
  String get skillHintBlacksmith1H => '1H-Waffen';

  @override
  String get skillHintBlacksmith2H => '2H-Waffen';

  @override
  String get skillCategoryCombat => 'Kampf';

  @override
  String get skillCategoryCrafting => 'Handwerk';

  @override
  String get skillCategoryHunting => 'Jagd';

  @override
  String get skillCategoryLanguage => 'Sprache';

  @override
  String get skillCategoryMagic => 'Magie';

  @override
  String get skillCategoryMovement => 'Bewegung';

  @override
  String get skillCategoryThievery => 'Diebeskunst';

  @override
  String get skillNameOneHanded => 'Einhand';

  @override
  String get skillNameTwoHanded => 'Zweihand';

  @override
  String get skillNameFists => 'Fäuste';

  @override
  String get skillNameBow => 'Bogen';

  @override
  String get skillNameCrossbow => 'Armbrust';

  @override
  String get skillNameLockpicking => 'Schlösserknacken';

  @override
  String get skillNamePickpocketing => 'Taschendiebstahl';

  @override
  String get skillNameTakeOrgans => 'Organe entnehmen';

  @override
  String get skillNameBreakTeeth => 'Zähne ziehen';

  @override
  String get skillNameTakeClaws => 'Krallen ziehen';

  @override
  String get skillNameSkinFur => 'Fell abziehen';

  @override
  String get skillNameSkin => 'Haut abziehen';

  @override
  String get skillNameTakeFins => 'Flossen entnehmen';

  @override
  String get skillNameTakeStingers => 'Stachel ziehen';

  @override
  String get skillNameTakeSecretion => 'Sekret entnehmen';

  @override
  String get skillNameTakeSkullPlates => 'Schädelpanzerung entnehmen';

  @override
  String get skillNameSkinSwampshark => 'Haihaut abziehen';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Platten entnehmen';

  @override
  String get skillNameTakeScutes => 'Hornschuppen entnehmen';

  @override
  String get skillNameTakeUluMulu => 'Ulu-Mulu entnehmen';

  @override
  String get skillNameOrcWeapons => 'Ork-Waffen';

  @override
  String get skillNameMining => 'Bergbau';

  @override
  String get skillNameDiving => 'Tauchen';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Zangen entnehmen';

  @override
  String get skillNameTakeShadowbeastHorn => 'Horn entnehmen (Schattenläufer)';

  @override
  String get skillNameTakeSpines => 'Wirbelsäule entnehmen';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Haizähne ziehen';

  @override
  String get skillNameTakeFireTongue => 'Flammenzunge entnehmen';

  @override
  String get skillNameTakeTrollHorn => 'Horn entnehmen (Troll)';

  @override
  String get skillNameAcrobatics => 'Akrobatik';

  @override
  String get skillNameWallClimbing => 'Klettern';

  @override
  String get skillNameRiding => 'Scavenger-Reiten';

  @override
  String get skillNameSneaking => 'Schleichen';

  @override
  String get skillNameAlchemy => 'Alchemie';

  @override
  String get skillNameRuneInscription => 'Inskribieren';

  @override
  String get skillNameBlacksmithing => 'Schmieden';

  @override
  String get skillNameMagicCircle => 'Magischer Kreis';

  @override
  String get skillNameOrcish => 'Orkisch';

  @override
  String get tabInventory => 'Inventar';

  @override
  String get tabWorld => 'Welt';

  @override
  String get tabCharacters => 'Charaktere';

  @override
  String get characterNoActorBody =>
      'Dieser Charakter hat keinen Akteur in der Welt und daher keine Attribute, kein Inventar und keine Ereignisse.';

  @override
  String get characterNoEventsBody => 'Keine Ereignisse für diesen Charakter.';

  @override
  String get characterOrphanGroup => 'Weitere';

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
  String get equippedBadge => 'Angelegt';

  @override
  String get armorUpgradesLabel => 'Verbesserungen';

  @override
  String get browse => 'Durchsuchen';

  @override
  String get noSavFilesFound => 'Keine .sav-Dateien gefunden';

  @override
  String get profile => 'Profil';

  @override
  String get otherSaves => 'Andere Spielstände';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count Spielstände)';
  }

  @override
  String get switchProfile => 'Profil wechseln';

  @override
  String get openSaveFile => 'Datei öffnen';

  @override
  String get externalSave => 'Extern geöffneter Spielstand';

  @override
  String get saveProfileTitle => 'Spielstand-Profil';

  @override
  String get saveProfileDescription =>
      'Diesen Spielstand einem anderen Spielprofil zuordnen. Spielstand und Profilindex werden gemeinsam gesichert.';

  @override
  String get saveProfileExternalHint =>
      'Wähle ein Profil, um diese Datei in den Spiel-Speicherordner zu importieren und dort zu registrieren. Die Originaldatei bleibt unverändert.';

  @override
  String get saveProfileNoProfiles =>
      'In PersistentDataList.sav wurden keine bearbeitbaren Spielprofile gefunden.';

  @override
  String get saveProfileSelect => 'Profil auswählen';

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
  String bytesValue(String count) {
    return '$count Bytes';
  }

  @override
  String get inspectionJsonTitle => 'Inspektions-JSON';

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
  String get gameTimeTitle => 'Spielzeit';

  @override
  String get gameTimeDay => 'Tag';

  @override
  String get gameTimeHours => 'Stunden';

  @override
  String get gameTimeMinutes => 'Minuten';

  @override
  String get gameTimeSeconds => 'Sekunden';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s gesamt';
  }

  @override
  String get gameTimeInvalid =>
      'Ganze Zahlen eingeben — Tag ≥ 0, Stunden 0–23, Minuten und Sekunden 0–59.';

  @override
  String get required => 'Erforderlich';

  @override
  String get playerLockedBody =>
      'Private Spielerbearbeitungen benötigen einen komprimierfähigen Codec.';

  @override
  String get heroTransform => 'Position';

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
  String get attributeBaseValue => 'Basiswert';

  @override
  String get attributeCurrentValue => 'Aktueller Wert';

  @override
  String get inventoryTitle => 'Inventar';

  @override
  String get inventoryEmpty => 'Dieses Inventar ist leer.';

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
  String get resetInventoryButton => 'Inventar zurücksetzen';

  @override
  String get resetInventoryTooltipDefault =>
      'Inventar durch das Spielstart-Inventar ersetzen';

  @override
  String get resetInventoryTooltipBlocked =>
      'Zuerst die ausstehenden Inventaränderungen speichern oder verwerfen';

  @override
  String get pendingResetTitle => 'Auf Spielstart-Inventar zurücksetzen';

  @override
  String pendingResetSubtitle(String level) {
    return 'Ressourcen-Stufe: $level';
  }

  @override
  String get cancelPendingReset => 'Zurücksetzen abbrechen';

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
  String get itemCategoryArmor => 'Rüstungen';

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
  String get npcStatusRowLabel => 'Status';

  @override
  String get npcStatusAlive => 'lebend';

  @override
  String get npcStatusDead => 'tot';

  @override
  String get npcRelationshipRowLabel => 'Beziehung';

  @override
  String get npcRelationshipUnavailable => 'Beziehungsstatus nicht verfügbar';

  @override
  String get npcRelationshipAutomatic => 'Vom Spiel berechnet';

  @override
  String get npcRelationshipAutomaticHint =>
      'Kein permanenter Override gespeichert. Gilden-, Story-, Gebiets- und Verbrechensregeln werden erst im Spiel ausgewertet.';

  @override
  String get npcRelationshipStoredHint =>
      'Als permanenter NPC-zu-Spieler-Override gespeichert. Gilden-, Story-, Gebiets- und Verbrechensregeln können den effektiven Status im Spiel trotzdem verändern.';

  @override
  String get npcRelationshipFriend => 'Freund';

  @override
  String get npcRelationshipNeutral => 'Neutral';

  @override
  String get npcRelationshipEnemy => 'Feind';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Wird beim Speichern zu „$relationship“';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Wiederbeleben';

  @override
  String get npcReviveQueued => 'Wird beim Speichern wiederbelebt';

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
      'Dieses Erinnerungsereignis zum Entfernen vormerken? Der Spielstand wird erst mit „Speichern“ geändert.';

  @override
  String get memoryEventRemovalQueued =>
      'Entfernen vorgemerkt – mit „Speichern“ anwenden.';

  @override
  String get duplicateEvent => 'Ereignis duplizieren';

  @override
  String get duplicateMemoryEventTitle => 'Erinnerungsereignis duplizieren?';

  @override
  String get duplicateMemoryEventBody =>
      'Eine Kopie dieses Erinnerungsereignisses vormerken? Der Spielstand wird erst mit „Speichern“ geändert.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplizieren vorgemerkt – mit „Speichern“ anwenden.';

  @override
  String get selectCharacterFromList => 'Wähle einen Charakter aus der Liste';

  @override
  String get factionsSidebar => 'Fraktionen';

  @override
  String get factionsForgiveButton => 'Befrieden';

  @override
  String get factionHostile => 'Feindselig';

  @override
  String get factionFriendly => 'Friedlich';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Morde',
      one: '$count Mord',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Übergriffe',
      one: '$count Übergriff',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Diebstähle',
      one: '$count Diebstahl',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Hausfriedensbrüche',
      one: '$count Hausfriedensbruch',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Drohungen',
      one: '$count Drohung',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sonstige Verbrechen',
      one: '$count sonstiges Verbrechen',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'wird befriedet …';

  @override
  String get factionsEmpty => 'Keine offenen Verbrechen gegen Fraktionen.';

  @override
  String get factionGuildOldCamp => 'Altes Lager';

  @override
  String get factionGuildNewCamp => 'Neues Lager';

  @override
  String get factionGuildSwampCamp => 'Sumpflager';

  @override
  String get factionGuildOther => 'Sonstige/Einzelpersonen';

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
      'Die portable Version öffnet die Download-Seite im Browser. Ersetze deine vorhandenen Dateien durch den neuen Download.';

  @override
  String get updateAvailableTitle => 'Update verfügbar';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Version $version ist verfügbar. Du hast $current.';
  }

  @override
  String get updateDownload => 'Herunterladen';

  @override
  String get updateLater => 'Später';

  @override
  String get updateUpToDate => 'Du verwendest die neueste Version.';

  @override
  String get updateCheckFailed =>
      'Suche nach Updates fehlgeschlagen. Bitte später erneut versuchen.';

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
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE-Mitwirkende';

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
  String get presetNovice => 'Novize';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Schwer';

  @override
  String get presetCustom => 'Individuell';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Die gespeicherte Voreinstellung wird nicht erkannt ($preset). Du kannst trotzdem Änderungen am Kampffluss-Helfer / Permadeath speichern oder oben eine Voreinstellung wählen, um sie zu überschreiben.';
  }

  @override
  String get closeCombatFlowHelper => 'Nahkampf Flow Helper';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Nicht verfügbar auf Anfänger';

  @override
  String get levelCombat => 'Kampf';

  @override
  String get levelResources => 'Ressourcen';

  @override
  String get levelProgression => 'Progression';

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
  String get heroEntryHeroTransform => 'Position';

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
  String savingProgress(int done, int total) {
    return 'Speichern… $done von $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs in $languageCount Sprachen extrahiert';
  }

  @override
  String get skillSmithing1H => 'Einhand-Schmieden';

  @override
  String get skillSmithing2H => 'Zweihand-Schmieden';

  @override
  String get skillCircleNovice => 'Magier-Novize';

  @override
  String get skillCircle1 => 'Erster Kreis der Magie';

  @override
  String get skillCircle2 => 'Zweiter Kreis der Magie';

  @override
  String get skillCircle3 => 'Dritter Kreis der Magie';

  @override
  String get skillCircle4 => 'Vierter Kreis der Magie';

  @override
  String get skillCircle5 => 'Fünfter Kreis der Magie';

  @override
  String get skillCircle6 => 'Sechster Kreis der Magie';

  @override
  String get sectionGlossary => 'Glossar';

  @override
  String get glossarySearch => 'Glossar durchsuchen';

  @override
  String get glossaryOldCamp => 'Altes Lager';

  @override
  String get glossaryNewCamp => 'Neues Lager';

  @override
  String get glossarySwampCamp => 'Sumpflager';

  @override
  String get glossaryOutsiders => 'Außenseiter';

  @override
  String get glossaryCreatures => 'Kreaturen';

  @override
  String get glossaryLocations => 'Orte';

  @override
  String get glossaryFilterLabel => 'Filter';

  @override
  String get glossaryFilterTraders => 'Händler';

  @override
  String get glossaryFilterTeachers => 'Lehrer';

  @override
  String get glossaryFilterArmorers => 'Rüstungsbauer';

  @override
  String get glossaryFilterHostile => 'Feindlich';

  @override
  String get glossaryRelationshipFilterNote =>
      'Zeigt permanente Feind-Overrides aus dem Spielstand. Dynamische Gilden-, Story-, Gebiets- und Verbrechensbeziehungen berechnet erst das Spiel.';

  @override
  String get glossaryFilterDead => 'Tot';

  @override
  String get glossaryAddEntry => 'Glossareintrag hinzufügen';

  @override
  String get glossaryAddTitle => 'Glossareintrag hinzufügen';

  @override
  String get glossaryResetChanges => 'Glossaränderungen zurücksetzen';

  @override
  String get glossaryNoVisibleEntries =>
      'Für diese Ansicht sind keine passenden Glossareinträge sichtbar.';

  @override
  String get glossaryNoHiddenEntries =>
      'Alle verfügbaren Einträge sind bereits sichtbar.';

  @override
  String get glossaryNoMatch => 'Keine passenden Glossareinträge.';

  @override
  String get glossarySelectEntry =>
      'Wähle einen Glossareintrag aus, um seine Einträge zu bearbeiten.';

  @override
  String glossaryEntryCount(int count) {
    return '$count Einträge';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked von $total Einträgen';
  }

  @override
  String get glossaryPortraitUnlocked => 'Porträt freigeschaltet';

  @override
  String get glossaryPortraitSilhouette =>
      'Silhouette — Porträt nicht freigeschaltet';

  @override
  String get glossarySegments => 'Einträge';

  @override
  String get glossaryPending => 'Ungespeicherte Änderung';

  @override
  String get glossaryShowFullText => 'Vollständigen Eintragstext anzeigen';

  @override
  String get glossarySegmentIntroduction => 'Begegnung / Porträt';

  @override
  String get glossarySegmentUnlock => 'Entdeckung';

  @override
  String glossarySegmentEntry(int number) {
    return 'Eintrag $number';
  }

  @override
  String get questJournalAll => 'Alle Quests';

  @override
  String get questJournalOldCamp => 'Altes Lager';

  @override
  String get questJournalNewCamp => 'Neues Lager';

  @override
  String get questJournalSwampCamp => 'Sumpflager';

  @override
  String get questJournalColony => 'Die Kolonie';

  @override
  String get questJournalCompleted => 'Abgeschlossen';

  @override
  String get questJournalHint =>
      'Ansicht wie im Spieljournal. Interne und noch nicht gestartete Quest-Zustände bleiben unter „Alle Daten“ verfügbar.';

  @override
  String get questJournalNoEntries =>
      'Keine Journal-Quests entsprechen den aktuellen Filtern.';

  @override
  String get glossaryTutorials => 'Tutorials';

  @override
  String get tutorialGateNote =>
      'Diese Zeilen steuern gespeicherte Tutorial-Freischaltungen. Eine Freischaltung entspricht nicht zwingend genau einer einzelnen Tutorial-Seite im Spiel.';

  @override
  String get tutorialResetChanges => 'Tutorial-Änderungen zurücksetzen';

  @override
  String get tutorialNoGates =>
      'In diesem Spielstand sind keine Tutorial-Freischaltungen verfügbar.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked von $total Tutorial-Freischaltungen aktiv';
  }

  @override
  String get tutorialGateCombatBasics => 'Kampfgrundlagen';

  @override
  String get tutorialGateCrafting => 'Handwerk';

  @override
  String get tutorialGateCrime => 'Verbrechen und Folgen';

  @override
  String get tutorialGateDrugs => 'Verbrauchsgegenstände und Effekte';

  @override
  String get tutorialGateLockpicking => 'Schlösser knacken';

  @override
  String get tutorialGateMagic => 'Magie';

  @override
  String get tutorialGateMap => 'Karte';

  @override
  String get tutorialGateMeleeCombat => 'Nahkampf';

  @override
  String get tutorialGateNavigation => 'Bewegung und Navigation';

  @override
  String get tutorialGatePerception => 'Wahrnehmung';

  @override
  String get tutorialGatePlayerProgression => 'Charakterentwicklung';

  @override
  String get tutorialGateRanged => 'Fernkampf';

  @override
  String get tutorialGateRiding => 'Reiten';

  @override
  String get tutorialGateSleep => 'Schlafen';

  @override
  String get tutorialGateTrading => 'Handeln';

  @override
  String get windowMinimizeTooltip => 'Minimieren';

  @override
  String get windowMaximizeTooltip => 'Maximieren';

  @override
  String get windowRestoreTooltip => 'Wiederherstellen';

  @override
  String get fallbackDialogEntry => 'Dialogeintrag';

  @override
  String get fallbackDialogChoice => 'Dialogauswahl';

  @override
  String get fallbackDialogTopic => 'Dialogthema';

  @override
  String get fallbackDialogInformation => 'Dialoginformation';

  @override
  String get fallbackQuest => 'Quest';

  @override
  String get fallbackObjective => 'Ziel';

  @override
  String get fallbackItem => 'Gegenstand';

  @override
  String get attributeSkillPointsFallback => 'Lernpunkte (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Alkohol',
      'AlcoholDepletionRate': 'Alkohol-Abbaurate',
      'MaxAlcohol': 'Maximaler Alkoholwert',
      'MaxSuperArmor': 'Maximale Superrüstung',
      'SuperArmor': 'Superrüstung',
      'Fatigue': 'Erschöpfung',
      'FillRatio': 'Füllverhältnis',
      'FillRatioPeriod': 'Zeitraum des Füllverhältnisses',
      'MaxFatigue': 'Maximale Erschöpfung',
      'MaxThresholdIndex': 'Maximaler Schwellenindex',
      'RecoveryRatePerHourOfSleep': 'Erholung pro Schlafstunde',
      'DamageMultiplier': 'Schadensmultiplikator',
      'Toughness': 'Zähigkeit',
      'ToughnessA': 'Zähigkeit A',
      'ToughnessB': 'Zähigkeit B',
      'ToughnessC': 'Zähigkeit C',
      'XPExecutedBounty': 'EP-Belohnung für Hinrichtungen',
      'XPKillOrDefeatBounty': 'EP-Belohnung für Tötungen oder Niederlagen',
      'SpeedModifier': 'Geschwindigkeitsmodifikator',
      'CriticalLevelPercent': 'Kritische Stufe (%)',
      'MaxOxygen': 'Maximaler Sauerstoffwert',
      'Oxygen': 'Sauerstoff',
      'OxygenDepletionRate': 'Sauerstoff-Abbaurate',
      'OxygenRecoveryRate': 'Sauerstoff-Erholungsrate',
      'MaxRestTime': 'Maximale Ruhezeit',
      'MaxSleepTime': 'Maximale Schlafzeit',
      'SleepTime': 'Schlafzeit',
      'SleepTimeRecoveryAmount': 'Schlafzeit-Erholungsmenge',
      'SleepTimeRecoveryPeriod': 'Schlafzeit-Erholungsintervall',
      'MaxSwampweed': 'Maximaler Sumpfkrautwert',
      'Swampweed': 'Sumpfkraut',
      'SwampweedDepletionRate': 'Sumpfkraut-Abbaurate',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Sprachzeile';

  @override
  String get knowledgeTypeOther => 'Sonstiges';

  @override
  String get armorUpgradeUpper => 'Oben';

  @override
  String get armorUpgradeMiddle => 'Mitte';

  @override
  String get armorUpgradeLower => 'Unten';

  @override
  String get knowledgeCategoryTopic => 'Thema';

  @override
  String get knowledgeCategoryChoice => 'Auswahl';

  @override
  String get knowledgeCategoryInfo => 'Information';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Fehlgeschlagen';

  @override
  String get missingSaveReference => 'Datei fehlt';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav fehlt. Die Datei wurde möglicherweise gelöscht, verschoben oder umbenannt; das Profil verweist noch darauf.';
  }

  @override
  String get removeFromProfile => 'Aus Profil entfernen';

  @override
  String get removeSaveFromProfileTitle => 'Spielstand aus Profil entfernen?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '$save aus $profile entfernen? Die Spielstanddatei selbst bleibt erhalten, sofern sie noch vorhanden ist.';
  }

  @override
  String get unassignedSave => 'Keinem Profil zugewiesen';

  @override
  String get armorUpgradeLight => 'Leicht';

  @override
  String get armorUpgradeMedium => 'Mittel';

  @override
  String get armorUpgradeHeavy => 'Schwer';

  @override
  String get knowledgeCaptionForcedConversation => 'Erzwungenes Gespräch';

  @override
  String get knowledgeCaptionFollowupTopic => 'Folgethema';

  @override
  String get knowledgeCaptionFallbackTopic => 'Ersatzthema';

  @override
  String durationMinutes(int minutes) {
    return '$minutes Min.';
  }

  @override
  String durationHours(int hours) {
    return '$hours Std.';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours Std. $minutes Min.';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Ungültige Profildaten';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Metadaten des ausgewählten Spielstands fehlen';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Unbekannt';

  @override
  String editorUnexpectedError(String details) {
    return 'Unerwarteter Fehler: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Ein anderer Vorgang läuft bereits. Versuche es gleich noch einmal.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Es gibt ungespeicherte Spielstandänderungen. Speichere sie oder setze sie zurück, bevor du den Profilschwierigkeitsgrad änderst.';

  @override
  String get editorNoSaveFolderSelected => 'Kein Spielstandordner ausgewählt.';

  @override
  String get editorNoSaveSelected => 'Kein Spielstand ausgewählt.';

  @override
  String get coreUnknownError => 'Unbekannter Core-Fehler';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Speichere deine Änderungen oder setze sie zurück, bevor du das Profil wechselst; sonst würdest du den aktuellen Spielstand verlassen.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Speichere deine Änderungen oder setze sie zurück, bevor du eine andere Datei öffnest.';

  @override
  String get editorSelectSavFile =>
      'Wähle eine Spielstanddatei mit der Endung .sav aus.';

  @override
  String get editorNotGothicGsav =>
      'Die ausgewählte Datei ist kein Gothic-Spielstand im GSAV-Format.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Speichere deine Änderungen oder setze sie zurück, bevor du die Profilzuordnung des Spielstands änderst.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Speichere deine Änderungen oder setze sie zurück, bevor du einen Spielstand aus seinem Profil entfernst.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Es gibt ungespeicherte Spielstandänderungen. Speichere sie oder setze sie zurück, bevor du eine Profilsicherung wiederherstellst.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Ungespeicherte Änderungen aus zwei Tabs betreffen dieselbe Eigenschaft ($path). Setze eine davon zurück oder mache sie rückgängig und speichere anschließend erneut.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Eine Glossarsegment-Änderung und eine weitere ungespeicherte Bearbeitung im Tab „Alle Daten“ betreffen beide das Array „Hero MemorizedEvents“ ($path). Glossaränderungen fügen dort Einträge hinzu oder entfernen sie; beide Änderungen lassen sich daher nicht gemeinsam speichern. Setze eine davon zurück oder mache sie rückgängig und speichere erneut.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Eine Glossarsegment-Änderung und eine weitere ungespeicherte Bearbeitung betreffen dieselbe CurrentState-Eigenschaft einer Quest ($path). Die Glossaränderung aktualisiert diesen Zustand selbst. Setze eine der Änderungen zurück oder mache sie rückgängig und speichere erneut.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Eine Beziehungsüberschreibung und eine weitere ungespeicherte Bearbeitung im Tab „Alle Daten“ betreffen denselben NPC-Beziehungseintrag ($path). Die strukturierte Beziehungsänderung kann Modifikatoren dieses Eintrags ersetzen; beide Änderungen lassen sich daher nicht gemeinsam speichern. Setze eine davon zurück oder mache sie rückgängig und speichere erneut.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Mehrere ungespeicherte Strukturänderungen betreffen dasselbe Array ($path). Speichere die erste Änderung oder setze sie zurück, bevor du eine weitere vormerkst.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Eine strukturelle Ereignisänderung und eine weitere ungespeicherte Bearbeitung im Tab „Alle Daten“ betreffen beide $path. Speichere eine davon oder setze sie zurück, bevor du fortfährst.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Eine Änderung der Fertigkeiten und eine Bearbeitung im Tab „Alle Daten“ für denselben Akteurseffekt (ActiveEffects › EffectSpec › Def) sind vorgemerkt. Sie lassen sich nicht gemeinsam speichern. Setze eine davon zurück oder mache sie rückgängig und speichere erneut.';

  @override
  String get editorInventoryResetConflict =>
      'Ein Zurücksetzen des Inventars und eine weitere Bearbeitung desselben Inventars sind vorgemerkt. Das Zurücksetzen ersetzt das gesamte Inventar und würde die andere Änderung verwerfen. Setze eine davon zurück oder mache sie rückgängig und speichere erneut.';

  @override
  String get editorUseFolder => 'Ordner verwenden';

  @override
  String get editorGothicSavegameFileType => 'Gothic-Savegame';

  @override
  String get editorNoDifficultyChanges =>
      'Keine Schwierigkeitsänderungen zum Speichern';

  @override
  String get editorDifficultyWritten =>
      'Schwierigkeit im Profil gespeichert (Sicherung erstellt)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Änderungen mit Sicherung gespeichert',
      one: '1 Änderung mit Sicherung gespeichert',
    );
    return '$_temp0';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profil $profileId wurde nicht gefunden.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Im Spielstandordner ist kein freier Slot verfügbar (G1R-001 bis G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Spielstand importiert und dem Profil $profileId zugewiesen';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Spielstand dem Profil $profileId zugewiesen (zusammengehörige Sicherungen erstellt)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Spielstand-Slot $slot ist dem Profil $profileId nicht zugewiesen.';
  }

  @override
  String get editorSaveRemovedFromProfile =>
      'Spielstand aus dem Profil entfernt';

  @override
  String editorRestoredBackup(String path) {
    return 'Sicherung wiederhergestellt: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Sicherung wiederhergestellt: $path (PersistentDataList.sav blieb unverändert – keine passende Begleitsicherung; die Slot-Metadaten können abweichen)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Codec-Roundtrip bestanden: Chunk $chunkIndex neu komprimiert auf $bytes Bytes';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Der Profilschwierigkeitsgrad konnte nicht gespeichert werden: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Der Spielstand konnte dem Profil nicht zugewiesen werden: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Der Spielstand konnte nicht aus dem Profil entfernt werden: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Die Änderungen konnten nicht gespeichert werden: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Die Spielstände konnten nicht eingelesen werden: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Der Spielstand konnte nicht geprüft werden: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Die Sicherungen konnten nicht geladen werden: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Die Sicherung konnte nicht wiederhergestellt werden: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Sicherung wiederhergestellt: $path, aber der Spielstand konnte anschließend nicht neu geladen werden: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Codec-Prüfung fehlgeschlagen: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Codec-Roundtrip fehlgeschlagen: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Eigenschaftssuche fehlgeschlagen: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Die Spielstandauswahl hat sich beim Laden der Heldenattribute geändert.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Die Fertigkeiten konnten nicht geladen werden: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Fortschrittsabfrage fehlgeschlagen: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Die NPC-Liste konnte nicht geladen werden: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Die Charakterliste konnte nicht geladen werden: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Die NPC-Attribute konnten nicht geladen werden: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Das NPC-Inventar konnte nicht geladen werden: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Die Fraktionsliste konnte nicht geladen werden: $details';
  }

  @override
  String get editorNoBackupPath => 'keine';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix: $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix: $backupPath; PersistentDataList-Sicherung: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Der Lokalisierungsstatus konnte nicht geladen werden: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Extraktion fehlgeschlagen: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Das Glossar konnte nicht geladen werden: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Sicherungsfehler: $details';
  }

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Zugang',
      'AccessDenied': 'Zugriff verweigert',
      'AccesToTemple': 'Zugang zum Tempel',
      'Advice': 'Rat',
      'AfterFight': 'Nach dem Kampf',
      'AfterFireMages': 'Nach den Feuermagiern',
      'AfterNek': 'Nach Nek',
      'AfterQuest': 'Nach der Quest',
      'Alone': 'Allein',
      'Amulet': 'Amulett',
      'Annoying': 'Nervig',
      'Armor': 'Rüstung',
      'Avoid': 'Meiden',
      'Backstory': 'Hintergrundgeschichte',
      'BackStory': 'Hintergrundgeschichte',
      'BasicMagic': 'Grundlegende Magie',
      'Beated': 'Geschlagen',
      'BecomeMercenary': 'Söldner werden',
      'Beer': 'Bier',
      'Bestiary': 'Bestiarium',
      'Blessing': 'Segen',
      'Boss': 'Boss',
      'Bully': 'Schläger',
      'BullyAdvice': 'Rat zum Schläger',
      'Camp': 'Lager',
      'CampDivided': 'Gespaltenes Lager',
      'CareOfMessengers': 'Boten versorgen',
      'ChangeOpinion': 'Meinungsänderung',
      'ChargeUriziel': 'Uriziel aufladen',
      'Chosen': 'Auserwählt',
      'Contact': 'Kontakt',
      'Courier': 'Kurier',
      'CraftBows': 'Bögen herstellen',
      'Crazy': 'Verrückt',
      'DailyMeal': 'Tägliche Mahlzeit',
      'DailyRation_Trader': 'Händler für Tagesrationen',
      'DAM': 'Damm',
      'Dead': 'Tot',
      'Deal': 'Abmachung',
      'Dealer': 'Händler',
      'Deceived': 'Getäuscht',
      'Dementia': 'Demenz',
      'DenyAccess': 'Zugang verweigern',
      'DifferentOpinion': 'Andere Meinung',
      'Discussion': 'Diskussion',
      'DontTalk': 'Nicht reden',
      'Duel': 'Duell',
      'Entrance': 'Eingang',
      'Escape': 'Flucht',
      'Extended': 'Erweitert',
      'Extra': 'Extra',
      'ExtraInfo': 'Zusätzliche Informationen',
      'Fanatic': 'Fanatiker',
      'Fight': 'Kampf',
      'FindUlumulu': 'Finde Ulu-Mulu',
      'FireMages': 'Feuermagier',
      'FireMagesEscape': 'Flucht der Feuermagier',
      'FiskNewDealer': 'Neuer Hehler für Fisk',
      'FiskNewDealerCompleted': 'Neuer Hehler für Fisk – abgeschlossen',
      'FogTower': 'Nebelturm',
      'Food': 'Nahrung',
      'Forgave': 'Verziehen',
      'Forgive': 'Verzeihen',
      'Forgiven': 'Verziehen',
      'FourFriends': 'Vier Freunde',
      'FreeHut': 'Freie Hütte',
      'FreeMine': 'Freie Mine',
      'Fury': 'Wut',
      'GoodTeacher': 'Guter Lehrer',
      'Gossip': 'Klatsch',
      'GotScavenger': 'Scavenger erhalten',
      'GrantedAccess': 'Zugang gewährt',
      'GRDArmor': 'Gardistenrüstung',
      'Guide': 'Führung',
      'HateMages': 'Hass auf Magier',
      'HateMagesExplanation': 'Erklärung zum Hass auf Magier',
      'HateRiceLord': 'Hass auf den Reislord',
      'Heal': 'Heilen',
      'Healing': 'Heilung',
      'Help': 'Hilfe',
      'Helper': 'Helfer',
      'HelpKagan': 'Kagan helfen',
      'HutStory': 'Hüttengeschichte',
      'Ignore': 'Ignorieren',
      'Impress': 'Beeindrucken',
      'ImpressAlchemy': 'Mit Alchemie beeindrucken',
      'ImpressInscription': 'Mit Inschriften beeindrucken',
      'Info': 'Information',
      'Interested': 'Interessiert',
      'Introduction': 'Begegnung / Porträt',
      'Introduction_2': 'Begegnung / Porträt 2',
      'Introduction_Armor': 'Einführung: Rüstung',
      'Introduction_Teacher': 'Einführung: Lehrer',
      'Introduction_Trader': 'Einführung: Händler',
      'Invocation': 'Anrufung',
      'JoinSC': 'Dem Sumpflager beitreten',
      'Joint': 'Joint',
      'KalomCamp': 'Kalom-Lager',
      'Leader': 'Anführer',
      'Learning': 'Lernen',
      'LearnOrcish': 'Lerne Orkisch',
      'LeftParty': 'Gruppe verlassen',
      'Library': 'Bibliothek',
      'Lie': 'Lüge',
      'Lock': 'Schloss',
      'Lockpick': 'Dietrich',
      'Mad': 'Verrückt',
      'Mandibles': 'Minecrawler-Zangen',
      'MapMaker': 'Kartograf',
      'Monastery': 'Kloster',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Neues Lager',
      'NewCamper': 'Neu im Lager',
      'NewLeader': 'Neuer Anführer',
      'NightPatrol': 'Nachtpatrouille',
      'NotInterested': 'Kein Interesse',
      'OldCamp': 'Altes Lager',
      'OrcEnclaveEntrance': 'Eingang zur Ork-Enklave',
      'OrcGraveyard': 'Ork-Friedhof',
      'OreArmor': 'Erzrüstung',
      'Party': 'Gruppe',
      'Pay': 'Bezahlen',
      'PayMoney': 'Geld bezahlen',
      'Permission': 'Erlaubnis',
      'Pet': 'Haustier',
      'PreparingInvocation': 'Anrufung vorbereiten',
      'Quest': 'Quest',
      'RankUpFireMages': 'Aufstieg zum Feuermagier',
      'RankUpGuard': 'Aufstieg zum Gardisten',
      'RanUpFireMagesCompleted': 'Aufstieg zum Feuermagier abgeschlossen',
      'Realocated': 'Umgesiedelt',
      'Reason': 'Grund',
      'Respect': 'Respekt',
      'ReturnToSC': 'Rückkehr ins Sumpflager',
      'RicelordForeman': 'Vorarbeiter des Reislords',
      'RideScavenger': 'Scavenger reiten',
      'Robe': 'Robe',
      'Safe': 'Sicher',
      'Scraper': 'Schürfer',
      'SecondChance': 'Zweite Chance',
      'SecretLocation': 'Geheimer Ort',
      'SecretPassage': 'Geheimgang',
      'SecretPath': 'Geheimer Weg',
      'SleeperFollower': 'Anhänger des Schläfers',
      'SleeperTemple': 'Tempel des Schläfers',
      'SmallInfo': 'Kurze Information',
      'Stonehenge': 'Stonehenge',
      'StopFollowing': 'Nicht mehr folgen',
      'SwampCamp': 'Sumpflager',
      'Talkative': 'Gesprächig',
      'Teach': 'Lehren',
      'TeachBow': 'Bogenkampf lehren',
      'Teacher': 'Lehrer',
      'Teacher2': 'Lehrer 2',
      'TeacherInscription': 'Lehrer für Inschriften',
      'TeacherMana': 'Mana-Lehrer',
      'TeachIchor': 'Minecrawler-Sekretgewinnung lehren',
      'TeachMagic': 'Lehre Magie',
      'TeachOrcish': 'Orkisch beibringen',
      'TeachStats': 'Attribute lehren',
      'TeachWeapon': 'Waffe beibringen',
      'Teleport': 'Teleportation',
      'TheMysteriousOrc': 'Der geheimnisvolle Ork',
      'ThroneRoom': 'Thronsaal',
      'TradeBow': 'Bogenhandel',
      'Trader': 'Händler',
      'TradeSkins_Trader': 'Fellhändler',
      'Traitor': 'Verräter',
      'Trial': 'Prüfung',
      'TrollCanyon': 'Trollschlucht',
      'Trust': 'Vertrauen',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Unerfahren',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Uriziel-Rune',
      'Useful': 'Nützlich',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrationen',
      'WaitFreeMine': 'Bei der Freien Mine warten',
      'WaitInTrainingArea': 'Im Trainingsbereich warten',
      'Warning': 'Warnung',
      'WarningTooLate': 'Zu späte Warnung',
      'WaterMessenger': 'Bote der Wassermagier',
      'Weapon': 'Waffe',
      'Who': 'Wer',
      'Women': 'Frauen',
      'other': '$fallback',
    });
    return '$_temp0';
  }
}
