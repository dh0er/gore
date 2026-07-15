// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get debugSectionTitle => 'Advanced (debug)';

  @override
  String get debugSectionSubtitle => 'Diagnostics & raw data for bug reports';

  @override
  String get showObjectIdsTitle => 'Show object IDs';

  @override
  String get showObjectIdsSubtitle =>
      'Show technical NPC, item, dialogue knowledge, and quest IDs in the editor.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor logo';

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
  String get skillsUnavailableBody =>
      'Skills can\'t be edited on this save — the hero has no effect data to modify.';

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
  String get skillTierBeginner => 'Beginner';

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
  String get skillNameOneHanded => 'One Handed';

  @override
  String get skillNameTwoHanded => 'Two Handed';

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
  String get skillNameTakeOrgans => 'Extract Organ';

  @override
  String get skillNameBreakTeeth => 'Extract Teeth';

  @override
  String get skillNameTakeClaws => 'Extract Claw';

  @override
  String get skillNameSkinFur => 'Take Fur';

  @override
  String get skillNameSkin => 'Take Skin';

  @override
  String get skillNameTakeFins => 'Take Fins';

  @override
  String get skillNameTakeStingers => 'Extract Stings';

  @override
  String get skillNameTakeSecretion => 'Extract Secretion';

  @override
  String get skillNameTakeSkullPlates => 'Take Skull Armor';

  @override
  String get skillNameSkinSwampshark => 'Take Shark Skin';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Take Plates';

  @override
  String get skillNameTakeScutes => 'Take Scutes';

  @override
  String get skillNameTakeUluMulu => 'Take Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Orc Weapons';

  @override
  String get skillNameMining => 'Mining';

  @override
  String get skillNameDiving => 'Diving';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Extract Mandibles';

  @override
  String get skillNameTakeShadowbeastHorn => 'Take Horn (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Extract Spine';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extract Shark Teeth';

  @override
  String get skillNameTakeFireTongue => 'Take Tongue of Fire';

  @override
  String get skillNameTakeTrollHorn => 'Take Horn (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobatics';

  @override
  String get skillNameWallClimbing => 'Climbing';

  @override
  String get skillNameRiding => 'Scavenger Riding';

  @override
  String get skillNameSneaking => 'Sneaking';

  @override
  String get skillNameAlchemy => 'Alchemy';

  @override
  String get skillNameRuneInscription => 'Inscription';

  @override
  String get skillNameBlacksmithing => 'Smithing';

  @override
  String get skillNameMagicCircle => 'Magic Circle';

  @override
  String get skillNameOrcish => 'Orcish';

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
  String get openSaveFile => 'Open file';

  @override
  String get externalSave => 'Externally opened save';

  @override
  String get saveProfileTitle => 'Save profile';

  @override
  String get saveProfileDescription =>
      'Assign this save to a different game profile. The save and profile index are backed up together.';

  @override
  String get saveProfileExternalHint =>
      'Select a profile to import this file into the game\'s save folder and register it there. The original file remains unchanged.';

  @override
  String get saveProfileNoProfiles =>
      'No editable game profiles were found in PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Select profile';

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
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'Inspection JSON';

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
  String get heroTransform => 'Position';

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
  String get attributeBaseValue => 'Base value';

  @override
  String get attributeCurrentValue => 'Current value';

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
  String get resetInventoryButton => 'Reset inventory';

  @override
  String get resetInventoryTooltipDefault =>
      'Replace this inventory with the game-start save\'s inventory';

  @override
  String get resetInventoryTooltipBlocked =>
      'Save or cancel the pending inventory changes first';

  @override
  String get pendingResetTitle => 'Reset to game-start inventory';

  @override
  String pendingResetSubtitle(String level) {
    return 'Resources level: $level';
  }

  @override
  String get cancelPendingReset => 'Cancel reset';

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
  String get npcRelationshipRowLabel => 'Relationship';

  @override
  String get npcRelationshipUnavailable => 'Relationship status unavailable';

  @override
  String get npcRelationshipAutomatic => 'Computed by game';

  @override
  String get npcRelationshipAutomaticHint =>
      'No permanent override is stored. Guild, story, area, and crime rules are evaluated in game.';

  @override
  String get npcRelationshipStoredHint =>
      'Stored as a permanent NPC-to-player override. Guild, story, area, and crime rules can still change the effective status in game.';

  @override
  String get npcRelationshipFriend => 'Friend';

  @override
  String get npcRelationshipNeutral => 'Neutral';

  @override
  String get npcRelationshipEnemy => 'Enemy';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Will be $relationship on save';
  }

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
      'Queue this memory event for removal? The save file is changed only when you press Save.';

  @override
  String get memoryEventRemovalQueued =>
      'Event removal queued — press Save to apply it.';

  @override
  String get duplicateEvent => 'Duplicate event';

  @override
  String get duplicateMemoryEventTitle => 'Duplicate memory event?';

  @override
  String get duplicateMemoryEventBody =>
      'Queue a duplicate of this memory event? The save file is changed only when you press Save.';

  @override
  String get memoryEventDuplicationQueued =>
      'Event duplication queued — press Save to apply it.';

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
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE contributors';

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
  String get heroEntryHeroTransform => 'Position';

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
  String savingProgress(int done, int total) {
    return 'Saving… $done of $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extracted $idCount ids across $languageCount languages';
  }

  @override
  String get skillSmithing1H => 'One-Hand Smithing';

  @override
  String get skillSmithing2H => 'Two-Hand Smithing';

  @override
  String get skillCircleNovice => 'Novice Magician';

  @override
  String get skillCircle1 => 'First Circle of Magic';

  @override
  String get skillCircle2 => 'Second Circle of Magic';

  @override
  String get skillCircle3 => 'Third Circle of Magic';

  @override
  String get skillCircle4 => 'Fourth Circle of Magic';

  @override
  String get skillCircle5 => 'Fifth Circle of Magic';

  @override
  String get skillCircle6 => 'Sixth Circle of Magic';

  @override
  String get sectionGlossary => 'Glossary';

  @override
  String get glossarySearch => 'Search glossary';

  @override
  String get glossaryOldCamp => 'Old Camp';

  @override
  String get glossaryNewCamp => 'New Camp';

  @override
  String get glossarySwampCamp => 'Swamp Camp';

  @override
  String get glossaryOutsiders => 'Outsiders';

  @override
  String get glossaryCreatures => 'Creatures';

  @override
  String get glossaryLocations => 'Locations';

  @override
  String get glossaryFilterLabel => 'Filter';

  @override
  String get glossaryFilterTraders => 'Traders';

  @override
  String get glossaryFilterTeachers => 'Teachers';

  @override
  String get glossaryFilterArmorers => 'Armorers';

  @override
  String get glossaryFilterHostile => 'Hostile';

  @override
  String get glossaryRelationshipFilterNote =>
      'Shows permanent enemy overrides stored in the save. Dynamic guild, story, area, and crime relationships are computed only in game.';

  @override
  String get glossaryFilterDead => 'Dead';

  @override
  String get glossaryAddEntry => 'Add glossary entry';

  @override
  String get glossaryAddTitle => 'Add glossary entry';

  @override
  String get glossaryResetChanges => 'Reset glossary changes';

  @override
  String get glossaryNoVisibleEntries =>
      'No visible glossary entries match this view.';

  @override
  String get glossaryNoHiddenEntries =>
      'Every available entry is already visible.';

  @override
  String get glossaryNoMatch => 'No glossary entries match.';

  @override
  String get glossarySelectEntry =>
      'Select a glossary entry to edit its entries.';

  @override
  String glossaryEntryCount(int count) {
    return '$count entries';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked of $total entries';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portrait unlocked';

  @override
  String get glossaryPortraitSilhouette => 'Silhouette — portrait not unlocked';

  @override
  String get glossarySegments => 'Entries';

  @override
  String get glossaryPending => 'Unsaved change';

  @override
  String get glossaryShowFullText => 'Show full entry text';

  @override
  String get glossarySegmentIntroduction => 'Introduction / portrait';

  @override
  String get glossarySegmentUnlock => 'Discovery';

  @override
  String glossarySegmentEntry(int number) {
    return 'Entry $number';
  }

  @override
  String get questJournalAll => 'All quests';

  @override
  String get questJournalOldCamp => 'Old Camp';

  @override
  String get questJournalNewCamp => 'New Camp';

  @override
  String get questJournalSwampCamp => 'Swamp Camp';

  @override
  String get questJournalColony => 'The Colony';

  @override
  String get questJournalCompleted => 'Completed';

  @override
  String get questJournalHint =>
      'In-game journal view. Internal and not-yet-started quest states remain available under All Data.';

  @override
  String get questJournalNoEntries =>
      'No journal quests match the current filters.';

  @override
  String get glossaryTutorials => 'Tutorials';

  @override
  String get tutorialGateNote =>
      'These rows control saved tutorial unlock gates. A gate does not necessarily map one-to-one to an individual in-game tutorial page.';

  @override
  String get tutorialResetChanges => 'Reset tutorial changes';

  @override
  String get tutorialNoGates =>
      'No tutorial unlock gates are available in this save.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked of $total tutorial gates unlocked';
  }

  @override
  String get tutorialGateCombatBasics => 'Combat basics';

  @override
  String get tutorialGateCrafting => 'Crafting';

  @override
  String get tutorialGateCrime => 'Crime and consequences';

  @override
  String get tutorialGateDrugs => 'Consumables and effects';

  @override
  String get tutorialGateLockpicking => 'Lockpicking';

  @override
  String get tutorialGateMagic => 'Magic';

  @override
  String get tutorialGateMap => 'Map';

  @override
  String get tutorialGateMeleeCombat => 'Melee combat';

  @override
  String get tutorialGateNavigation => 'Movement and navigation';

  @override
  String get tutorialGatePerception => 'Perception';

  @override
  String get tutorialGatePlayerProgression => 'Character progression';

  @override
  String get tutorialGateRanged => 'Ranged combat';

  @override
  String get tutorialGateRiding => 'Riding';

  @override
  String get tutorialGateSleep => 'Sleeping';

  @override
  String get tutorialGateTrading => 'Trading';
}
