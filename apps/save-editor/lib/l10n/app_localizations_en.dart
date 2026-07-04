// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Gothic Remake Savegame Editor';

  @override
  String get appLogoSemanticLabel => 'goresave logo';

  @override
  String get zoomTooltip => 'Press Ctrl +/- to zoom in/out';

  @override
  String get switchToLightMode => 'Switch to light mode';

  @override
  String get switchToDarkMode => 'Switch to dark mode';

  @override
  String get about => 'About';

  @override
  String get tabOverview => 'Overview';

  @override
  String get tabPlayer => 'Player';

  @override
  String get tabAttribute => 'Attributes';

  @override
  String get heroGroupSkills => 'Skills';

  @override
  String get skillsNoneBody => 'No skills found for this character.';

  @override
  String get skillNotLearned => 'Not learned';

  @override
  String get skillLearn => 'Learn';

  @override
  String get skillActionLearn => 'learn';

  @override
  String get skillActionUnlearn => 'unlearn';

  @override
  String get skillTierUntrained => 'Untrained';

  @override
  String get skillTierTrained => 'Trained';

  @override
  String get skillTierMaster => 'Master';

  @override
  String get skillTierNovice => 'Novice';

  @override
  String get skillTierAmateur => 'Amateur (Circle 0)';

  @override
  String get skillTierLearned => 'Learned';

  @override
  String skillTierCircle(int n) {
    return 'Circle $n';
  }

  @override
  String get skillHintBlacksmith1H => '1H weapons';

  @override
  String get skillHintBlacksmith2H => '2H weapons';

  @override
  String get skillCategoryCombat => 'Combat';

  @override
  String get skillCategoryCrafting => 'Crafting';

  @override
  String get skillCategoryHunting => 'Hunting';

  @override
  String get skillCategoryLanguage => 'Language';

  @override
  String get skillCategoryMagic => 'Magic';

  @override
  String get skillCategoryMovement => 'Movement';

  @override
  String get skillCategoryThievery => 'Thievery';

  @override
  String get skillNameOneHanded => 'One-Handed';

  @override
  String get skillNameTwoHanded => 'Two-Handed';

  @override
  String get skillNameFists => 'Fists';

  @override
  String get skillNameBow => 'Bow';

  @override
  String get skillNameCrossbow => 'Crossbow';

  @override
  String get skillNameLockpicking => 'Lockpicking';

  @override
  String get skillNamePickpocketing => 'Pickpocketing';

  @override
  String get skillNameTakeOrgans => 'Take Organs';

  @override
  String get skillNameBreakTeeth => 'Break Teeth';

  @override
  String get skillNameTakeClaws => 'Take Claws';

  @override
  String get skillNameSkinFur => 'Skin Fur';

  @override
  String get skillNameSkin => 'Skin';

  @override
  String get skillNameTakeFins => 'Take Fins';

  @override
  String get skillNameTakeStingers => 'Take Stingers';

  @override
  String get skillNameTakeSecretion => 'Take Secretion';

  @override
  String get skillNameTakeSkullPlates => 'Take Skull Plates';

  @override
  String get skillNameSkinSwampshark => 'Skin Swampshark';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Take Minecrawler Plates';

  @override
  String get skillNameTakeScutes => 'Take Scutes';

  @override
  String get skillNameTakeUluMulu => 'Take Ulu-Mulu Trophies';

  @override
  String get skillNameAcrobatics => 'Acrobatics';

  @override
  String get skillNameWallClimbing => 'Wall Climbing';

  @override
  String get skillNameRiding => 'Riding';

  @override
  String get skillNameSneaking => 'Sneaking';

  @override
  String get skillNameAlchemy => 'Alchemy';

  @override
  String get skillNameRuneInscription => 'Rune Inscription';

  @override
  String get skillNameBlacksmithing => 'Blacksmithing';

  @override
  String get skillNameMagicCircle => 'Magic Circle';

  @override
  String get skillNameOrcish => 'Orcish Language';

  @override
  String get tabInventory => 'Inventory';

  @override
  String get tabWorld => 'World';

  @override
  String get tabCharacters => 'Characters';

  @override
  String get characterNoActorBody =>
      'This character has no in-world actor, so it has no attributes, inventory, or events.';

  @override
  String get characterNoEventsBody => 'No events for this character.';

  @override
  String get characterOrphanGroup => 'Other';

  @override
  String get tabAllData => 'All data';

  @override
  String get tabBackups => 'Backups';

  @override
  String get tabSettings => 'Settings';

  @override
  String get reset => 'Reset';

  @override
  String get save => 'Save';

  @override
  String saveWithCount(int count) {
    return 'Save ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Cancel';

  @override
  String get confirm => 'Confirm';

  @override
  String get close => 'Close';

  @override
  String get add => 'Add';

  @override
  String get equippedBadge => 'Equipped';

  @override
  String get armorUpgradesLabel => 'Upgrades';

  @override
  String get browse => 'Browse';

  @override
  String get noSavFilesFound => 'No .sav files found';

  @override
  String get profile => 'Profile';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count saves)';
  }

  @override
  String get switchProfile => 'Switch profile';

  @override
  String get rescanSaveFolder => 'Rescan save folder';

  @override
  String get discardUnsavedChangesTitle => 'Discard unsaved changes?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'changes',
      one: 'change',
    );
    return 'Rescanning reloads every save and discards your $count unsaved $_temp0.';
  }

  @override
  String get discardAndRescan => 'Discard and rescan';

  @override
  String chapterLabel(Object id) {
    return 'Chapter $id';
  }

  @override
  String get quickSave => 'Quick save';

  @override
  String get autoSave => 'Auto save';

  @override
  String get manualSave => 'Manual save';

  @override
  String get errorTitle => 'Error';

  @override
  String get selectASaveTitle => 'Select a save';

  @override
  String get selectASaveBody => 'The save details will appear here.';

  @override
  String get diagnosticsTitle => 'Diagnostics & details';

  @override
  String get diagnosticsSubtitle => 'Read-only format inspection';

  @override
  String get metricFormat => 'Format';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Chapter';

  @override
  String get metricTimePlayed => 'Time played';

  @override
  String get metricSaveKind => 'Save kind';

  @override
  String get metricFileSize => 'File size';

  @override
  String get metricCompression => 'Compression';

  @override
  String get metricChunks => 'Chunks';

  @override
  String get metricUncompressed => 'Uncompressed';

  @override
  String get metricPrivate => 'Private';

  @override
  String get metricSlotName => 'Slot name';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Decoded private';

  @override
  String get metricPrivateStrings => 'Private strings';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'Inspection JSON';

  @override
  String get inspectionJsonSubtitle => 'Raw save inspection data';

  @override
  String get copy => 'Copy';

  @override
  String get savegameFallbackTitle => 'Savegame';

  @override
  String screenshotForSlot(String slot) {
    return 'Screenshot for $slot';
  }

  @override
  String get publicSaveName => 'Public save name';

  @override
  String get gameTimeTitle => 'Game time';

  @override
  String get gameTimeDay => 'Day';

  @override
  String get gameTimeHours => 'Hours';

  @override
  String get gameTimeMinutes => 'Minutes';

  @override
  String get gameTimeSeconds => 'Seconds';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s total';
  }

  @override
  String get gameTimeInvalid =>
      'Enter whole numbers — day ≥ 0, hours 0–23, minutes and seconds 0–59.';

  @override
  String get required => 'Required';

  @override
  String get playerLockedBody =>
      'Private player edits need a compress-ready codec.';

  @override
  String get heroTransform => 'Hero transform';

  @override
  String get locationX => 'Location X';

  @override
  String get locationY => 'Location Y';

  @override
  String get locationZ => 'Location Z';

  @override
  String get rotationPitch => 'Rotation pitch';

  @override
  String get rotationYaw => 'Rotation yaw';

  @override
  String get rotationRoll => 'Rotation roll';

  @override
  String get invalid => 'Invalid';

  @override
  String get heroAttributes => 'Hero attributes';

  @override
  String attributeBase(String name) {
    return '$name base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name current';
  }

  @override
  String get inventoryTitle => 'Inventory';

  @override
  String get inventoryEmpty => 'This inventory is empty.';

  @override
  String get inventoryNeedsDecoded =>
      'Inventory editing needs decoded private payload data from the codec.';

  @override
  String get inventoryNoStacks =>
      'No item stacks found in the decoded private payload.';

  @override
  String get resetInventoryChanges => 'Reset inventory changes';

  @override
  String get addItemTooltipPendingAdd =>
      'Save pending changes first — one new item per save';

  @override
  String get addItemTooltipPendingRemove =>
      'Save the pending removal first — one structural change per save';

  @override
  String get addItemTooltipPendingCount =>
      'Save or reset pending count changes first — a structural edit must be saved on its own';

  @override
  String get addItemTooltipDefault => 'Add item to inventory';

  @override
  String get addItemButton => 'Add item';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — pending add (not yet saved)';
  }

  @override
  String get cancelPendingAdd => 'Cancel pending add';

  @override
  String get pendingRemovalSubtitle => 'pending removal (not yet saved)';

  @override
  String get cancelPendingRemoval => 'Cancel pending removal';

  @override
  String get filterItems => 'Filter items';

  @override
  String noItemsMatchQuery(String query) {
    return 'No items match \"$query\".';
  }

  @override
  String get pendingRemovalHidesAll =>
      'The pending removal hides every item — save to apply it.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Melee weapons';

  @override
  String get itemCategoryRangedWeapon => 'Ranged weapons';

  @override
  String get itemCategoryAmmunition => 'Ammunition';

  @override
  String get itemCategoryArmor => 'Armor';

  @override
  String get itemCategoryRune => 'Runes';

  @override
  String get itemCategoryScroll => 'Spell scrolls';

  @override
  String get itemCategoryFood => 'Food & potions';

  @override
  String get itemCategoryMisc => 'Miscellaneous';

  @override
  String get itemCategoryAmulet => 'Amulets';

  @override
  String get itemCategoryRing => 'Rings';

  @override
  String get itemCategoryTrophy => 'Animal trophies';

  @override
  String get itemCategoryWriting => 'Writings';

  @override
  String get itemCategoryMission => 'Mission items';

  @override
  String get itemCategoryKey => 'Keys';

  @override
  String get itemCategoryOther => 'Other';

  @override
  String get count => 'Count';

  @override
  String get min1 => 'Min 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Can\'t delete: this item is likely equipped or assigned to a hotkey slot';

  @override
  String get removeBlockedTooltip =>
      'Save or reset your pending inventory changes first — an add or remove must be saved on its own';

  @override
  String get removeItemFromInventory => 'Remove item from inventory';

  @override
  String get progressionLockedBody =>
      'Progression data needs decoded private payload data from the codec.';

  @override
  String get progressionNeedsTyped =>
      'Structured progression data needs a fully decoded save with a verified typed parse.';

  @override
  String get sectionQuests => 'Quests';

  @override
  String get sectionKnowledge => 'Knowledge';

  @override
  String get sectionEvents => 'Events';

  @override
  String get firstPage => 'First page';

  @override
  String get previousPage => 'Previous page';

  @override
  String get nextPage => 'Next page';

  @override
  String get lastPage => 'Last page';

  @override
  String pageOfPages(int page, int total) {
    return 'Page $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last of $total';
  }

  @override
  String get perPage => 'Per page:';

  @override
  String get resetQuestChanges => 'Reset quest changes';

  @override
  String get searchQuests => 'Search quests';

  @override
  String get allGroups => 'All groups';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'None';

  @override
  String get questStateAvailable => 'Available';

  @override
  String get questStateRunning => 'Running';

  @override
  String get questStateSucceeded => 'Succeeded';

  @override
  String get questStateFailed => 'Failed';

  @override
  String get questStateUnknown => 'unknown';

  @override
  String get dialogKnowledge => 'Dialog Knowledge';

  @override
  String get resetKnowledgeChanges => 'Reset knowledge changes';

  @override
  String get addNpc => 'Add NPC';

  @override
  String get searchNpcs => 'Search NPCs';

  @override
  String get npcStatusRowLabel => 'Status';

  @override
  String get npcStatusAlive => 'alive';

  @override
  String get npcStatusDead => 'dead';

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Revive';

  @override
  String get npcReviveQueued => 'Will be revived on save';

  @override
  String entriesForCharacter(String name) {
    return 'Entries — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Select an NPC to see entries';

  @override
  String get addKnowledgeEntry => 'Add knowledge entry';

  @override
  String get browseCatalog => 'Browse catalog';

  @override
  String get alreadyExistsForCharacter => 'Already exists for this character.';

  @override
  String get alreadyInPendingChanges => 'Already in pending changes.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Duplicate check failed — try again: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Pending adds ($count)';
  }

  @override
  String get undoAdd => 'Undo add';

  @override
  String get undoRemove => 'Undo remove';

  @override
  String get removeEntry => 'Remove entry';

  @override
  String get selectNpcFromList => 'Select an NPC from the list';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Memory Events';

  @override
  String get searchCharacters => 'Search characters';

  @override
  String eventsForCharacter(String name) {
    return 'Events — $name';
  }

  @override
  String get selectCharacterToSeeEvents => 'Select a character to see events';

  @override
  String get noTags => '(no tags)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Remove event';

  @override
  String get removeMemoryEventTitle => 'Remove memory event?';

  @override
  String get removeMemoryEventBody =>
      'Remove this memory event? A backup is written first.';

  @override
  String get duplicateEvent => 'Duplicate event';

  @override
  String get duplicateMemoryEventTitle => 'Duplicate memory event?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicate this memory event? A backup is written first.';

  @override
  String get selectCharacterFromList => 'Select a character from the list';

  @override
  String get factionsSidebar => 'Factions';

  @override
  String get factionsForgiveButton => 'Forgive';

  @override
  String get factionHostile => 'Hostile';

  @override
  String get factionFriendly => 'Friendly';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count murders',
      one: '$count murder',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count assaults',
      one: '$count assault',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count thefts',
      one: '$count theft',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count trespasses',
      one: '$count trespass',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count threats',
      one: '$count threat',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count other crimes',
      one: '$count other crime',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'being forgiven…';

  @override
  String get factionsEmpty => 'No open crimes against factions.';

  @override
  String get factionGuildOldCamp => 'Old Camp';

  @override
  String get factionGuildNewCamp => 'New Camp';

  @override
  String get factionGuildSwampCamp => 'Swamp Camp';

  @override
  String get factionGuildOther => 'Others / individuals';

  @override
  String get allDataLockedBody =>
      'The full property browser needs decoded private payload data from the codec.';

  @override
  String get allDataDescription =>
      'Search every typed property by name or path. Scalars, strings, enums and object paths are editable; structs are shown read-only for now.';

  @override
  String get searchPropertiesLabel =>
      'Search properties (empty = list everything) — e.g. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Decoding save…';

  @override
  String get decodingSaveBody =>
      'Decoding the full private payload for the first search. This runs once per save, then searches are instant.';

  @override
  String get searchTheSaveTitle => 'Search the save';

  @override
  String get searchTheSaveBody =>
      'Type a property name and press enter. Leave it empty to list everything.';

  @override
  String get searchFailedTitle => 'Search failed';

  @override
  String get noMatchesTitle => 'No matches';

  @override
  String get noMatchesBody => 'No property path contained all of those terms.';

  @override
  String get value => 'Value';

  @override
  String get backupsTitle => 'Backups';

  @override
  String get refreshBackups => 'Refresh backups';

  @override
  String get noBackupsTitle => 'No backups';

  @override
  String get noBackupsBody =>
      'Edited saves create backup files next to the selected slot.';

  @override
  String get slotBackups => 'Slot backups';

  @override
  String get profileBackups => 'Profile backups';

  @override
  String get backupFactName => 'Name';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Created';

  @override
  String get backupFactSize => 'Size';

  @override
  String get backupFactStatus => 'Status';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restore $fileName';
  }

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
  String get language => 'Language';

  @override
  String get updatesTitle => 'Updates';

  @override
  String get checkForUpdatesAutomatically => 'Check for updates automatically';

  @override
  String get checkForUpdatesNow => 'Check for updates now';

  @override
  String get updatesPortableNotice =>
      'The portable version opens the download page in your browser. Replace your existing files with the new download.';

  @override
  String get updateAvailableTitle => 'Update available';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Version $version is available. You have $current.';
  }

  @override
  String get updateDownload => 'Download';

  @override
  String get updateLater => 'Later';

  @override
  String get updateUpToDate => 'You are using the latest version.';

  @override
  String get updateCheckFailed =>
      'Could not check for updates. Please try again later.';

  @override
  String get gameTextTitle => 'Game text';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extracted: $ids ids across $languages languages.';
  }

  @override
  String get gameTextExtracted => 'Localized game text is extracted.';

  @override
  String get gameTextNotExtracted =>
      'Localized game text is not extracted yet.';

  @override
  String get extracting => 'Extracting…';

  @override
  String get extractRefreshLocalizedText => 'Extract / refresh localized text';

  @override
  String get extractLocalizedTextTitle => 'Extract localized game text?';

  @override
  String get extractLocalizedTextBody =>
      'Localized game text isn\'t extracted yet. Extract it now from your game install? (optional)';

  @override
  String get notNow => 'Not now';

  @override
  String get extract => 'Extract';

  @override
  String get extractionComplete => 'Extraction complete';

  @override
  String get extractionFailed => 'Extraction failed';

  @override
  String get localizationCacheFileType => 'Localization cache';

  @override
  String get savegameDirectoryTitle => 'Savegame directory';

  @override
  String get folder => 'Folder';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Check';

  @override
  String get roundtrip => 'Roundtrip';

  @override
  String get noCodecStatus => 'No codec status';

  @override
  String get codecReady => 'Codec ready';

  @override
  String get codecReadOnly => 'Codec read-only';

  @override
  String get codecUnavailable => 'Codec unavailable';

  @override
  String get details => 'Details';

  @override
  String codecStatusLine(String status) {
    return 'Status: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Decompress: $decompress | Compress: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'yes';

  @override
  String get no => 'no';

  @override
  String get aboutSubtitle => 'Gothic Remake Savegame Editor';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 goresave contributors';

  @override
  String get aboutLicense => 'Licensed under the MIT License.';

  @override
  String difficultyTitle(String profile) {
    return 'Difficulty — $profile';
  }

  @override
  String get difficultyNoProfile => 'No profile';

  @override
  String get difficultyNoDifficulty => 'No difficulty';

  @override
  String get difficultyLabel => 'Difficulty';

  @override
  String get difficultyTooltipNoProfile => 'No profile selected';

  @override
  String get difficultyTooltipEdit => 'Edit difficulty for this profile';

  @override
  String get difficultyTooltipNoEditable =>
      'This profile has no editable difficulty';

  @override
  String get preset => 'Preset';

  @override
  String get presetNovice => 'Novice';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Hard';

  @override
  String get presetCustom => 'Custom';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Stored preset is unrecognised ($preset). You can still save Flow Helper / Permadeath changes, or pick a preset above to overwrite it.';
  }

  @override
  String get closeCombatFlowHelper => 'Close Combat Flow Helper';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Not available on Novice';

  @override
  String get levelCombat => 'Combat';

  @override
  String get levelResources => 'Resources';

  @override
  String get levelProgression => 'Progression';

  @override
  String get difficultyAppliesToAllSaves =>
      'Difficulty applies to all saves in this profile.';

  @override
  String get savingDifficultyFailed => 'Saving difficulty failed.';

  @override
  String get addItemDialogTitle => 'Add item';

  @override
  String get searchItems => 'Search items';

  @override
  String failedToLoadCatalog(String error) {
    return 'Failed to load catalog: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'No items available to add';

  @override
  String get noItemsMatch => 'No items match';

  @override
  String get countMustBeAtLeast1 => 'Must be ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Must be ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Add NPC';

  @override
  String get noNpcsAvailableToAdd => 'No NPCs available to add';

  @override
  String get noNpcsMatch => 'No NPCs match';

  @override
  String get categoryAll => 'All';

  @override
  String allWithCount(int count) {
    return 'All ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Add knowledge entry';

  @override
  String get searchEntries => 'Search entries';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'No knowledge entries available to add';

  @override
  String get noEntriesMatch => 'No entries match';

  @override
  String get heroGroupMainStats => 'Main stats';

  @override
  String get heroGroupCombatSkills => 'Combat skills';

  @override
  String get heroGroupResistances => 'Resistances';

  @override
  String get heroGroupThieving => 'Thieving';

  @override
  String get heroGroupAdvanced => 'Advanced';

  @override
  String get heroEntryHeroTransform => 'Hero transform';

  @override
  String attributeEmpty(String name) {
    return '$name is empty — enter a value or restore the original before saving.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Invalid number for $name: \"$text\"';
  }

  @override
  String get loadingEditorData => 'Loading editor data';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extracted $idCount ids across $languageCount languages';
  }
}
