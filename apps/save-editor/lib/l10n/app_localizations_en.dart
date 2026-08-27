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
  String get showObjectIdsTitle => 'Show additional technical IDs';

  @override
  String get showObjectIdsSubtitle =>
      'Show technical item, dialogue knowledge, quest, and orphan actor IDs in the editor. NPC IDs are always shown.';

  @override
  String get storyStateSidebar => 'Story state';

  @override
  String get storyStateDescription =>
      'Authoritative catalog of persisted story state declared by the shipped game scripts. Stored entries show their raw value; catalog fields missing from this save are marked as not set. Source-declared time markers are formatted as game time, while other integers may be booleans, counters, or multi-state values.';

  @override
  String get storyStateReadOnly =>
      'Read-only until the script meaning of values and safe map writes are established. Related glossary text is context, not a direct translation of the technical ID.';

  @override
  String get storyStateStructureReadOnly =>
      'The StoryPropertyValues structure in this save could not be resolved uniquely and safely. Story values remain read-only for this save.';

  @override
  String get storyStateSearch => 'Search story state';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown of $total story values';
  }

  @override
  String get storyStateInteger => 'Integer';

  @override
  String get storyStateTimeMarker => 'Time marker';

  @override
  String get storyStateChapter => 'Chapter';

  @override
  String get storyStateUnknown => 'Unknown source type';

  @override
  String get storyStateUnknownDetail =>
      'This stored ID is absent from the current script catalog (for example, from a mod or newer game version). Its save wire value is int32, but its meaning is not inferred.';

  @override
  String get storyStateStored => 'Stored';

  @override
  String get storyStateUnset => 'Not set';

  @override
  String get storyStateUnsetDetail =>
      'This catalog field is not serialized in this save; the game therefore uses its unset or default state.';

  @override
  String get storyStateRawValue => 'Raw value';

  @override
  String storyStateElapsed(String duration) {
    return 'Elapsed at save time: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Ahead of save time: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days days',
      one: '1 day',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Related glossary entry';

  @override
  String get storyStateTechnicalPath => 'Technical path';

  @override
  String get storyStateEditingGuidance =>
      'Every entry remains editable across the full signed int32 range. Script-backed switches and value suggestions are guidance; raw input is always available. Story changes can skip dialogue, quest, or world transitions, so save them deliberately — a backup is created automatically.';

  @override
  String get storyStatePending => 'Pending';

  @override
  String storyStatePendingValue(String value) {
    return 'Will be stored as $value';
  }

  @override
  String get storyStatePendingRemoval => 'Will be removed from the save';

  @override
  String get storyStateEditValue => 'Edit value';

  @override
  String get storyStateSetValue => 'Set value';

  @override
  String get storyStateRemoveValue => 'Remove from save';

  @override
  String get storyStateUndoChange => 'Undo story change';

  @override
  String get storyStateResetChanges => 'Reset story changes';

  @override
  String storyStateDialogTitle(String id) {
    return 'Edit $id';
  }

  @override
  String get storyStateRawInput => 'Signed int32 value';

  @override
  String get storyStateInvalidInt32 =>
      'Enter a whole number from -2147483648 to 2147483647.';

  @override
  String get storyStateQueueChange => 'Queue change';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Values evidenced in the shipped scripts: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Suggestions are not validation limits; native code, mods, or later game versions may use other values.';

  @override
  String get storyStateUseCurrentTime => 'Use current save time';

  @override
  String get storyStateStructuredTime => 'Day / time';

  @override
  String get storyStateRawMode => 'Raw int32';

  @override
  String get storyStateChapterWarning =>
      'Changing the chapter alone does not synchronize quests, NPCs, inventory, or world state.';

  @override
  String get storyStateDormantWarning =>
      'No live read or write was found for this field in the shipped script cache. It may be legacy, native-controlled, or reserved.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'The shipped scripts read this field but contain no script write. Native code may still own it.';

  @override
  String get storyStateUnknownEditWarning =>
      'This modded or newer-version ID has no bundled source semantics. Edit only its raw int32 value.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Binary flag',
      'finiteState': 'Multi-state value',
      'counterOrScore': 'Counter / score',
      'calendarDay': 'Calendar day',
      'derivedOrOpaqueInteger': 'Derived / opaque integer',
      'readOnlyInSourceInteger': 'Read-only in shipped scripts',
      'dormantOrLegacyInteger': 'Unused in shipped scripts',
      'other': 'Integer',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'A stored 0 and a missing map entry are distinct file states. “Remove from save” restores the constructor/default state.';

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
  String get skillScutesTrained => 'Trained (bone scutes)';

  @override
  String get skillScutesMaster => 'Master (+ razor plates)';

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
  String get skillCategoryOther => 'Other';

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
  String get tabTrade => 'Trade';

  @override
  String get traderNotAMerchant => 'This character does not trade.';

  @override
  String get traderRetry => 'Try again';

  @override
  String get traderAmbiguousName =>
      'More than one trader record carries this name, so the editor cannot tell which shop belongs to this character. Editing is disabled rather than risk changing the wrong one.';

  @override
  String get traderOre => 'Ore (purchasing power)';

  @override
  String get traderNoOre => 'no ore';

  @override
  String get traderStockCurrent => 'Stock';

  @override
  String get traderStockBase => 'Restock baseline';

  @override
  String get traderStockBaseHint =>
      'What the merchant restocks back toward. It grows with story progress, so it is not a vanilla snapshot.';

  @override
  String get traderOreHint =>
      'The in-game figure differs: on load the game adds what accrued since his last trade — he sells surplus goods and restocks from it. This number is the starting point, not what the trade screen shows.';

  @override
  String get traderPriceWarning =>
      'Prices react to how much a merchant stocks and how much ore he holds, so changing these numbers can also move what he charges.';

  @override
  String get traderAddItem => 'Add item';

  @override
  String get traderRemoveItem => 'Remove line';

  @override
  String get traderReadOnlyCore => 'This core build can only read trader data.';

  @override
  String get traderDifficultyStockUnsupported =>
      'This merchant carries per-difficulty stock, which the editor does not model. Editing is disabled here, because a change would look successful while leaving that extra stock untouched.';

  @override
  String get traderRecordIncomplete =>
      'This merchant\'s stock lists are missing, or in a shape the editor does not support and cannot write. Editing is disabled here so a change cannot fail at save time.';

  @override
  String get traderEmptyStock => 'Nothing in stock.';

  @override
  String get traderUnknownItem => 'not in the item catalog';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Trader load failed: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count lines';
  }

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
  String get otherSaves => 'Other saves';

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
  String get spawnPositionSection => 'Spawn position (reference)';

  @override
  String get resetToSpawnPosition => 'Reset to spawn position';

  @override
  String get positionOutOfRange =>
      'Value must be between −10,000,000 and 10,000,000';

  @override
  String get positionNotEditable =>
      'The stored position could not be read for this character, so it cannot be edited.';

  @override
  String get positionNeverPlaced =>
      'This character has never been placed in the world (position 0, 0, 0) — the game may ignore the stored position.';

  @override
  String get npcStayInPlace => 'Disable his daily routine';

  @override
  String get npcStayInPlaceHint => 'He then stays where he is.';

  @override
  String get npcStayInPlaceLocked =>
      'His original daily routine is not recorded, so this can no longer be undone.';

  @override
  String get npcUndoPlacement => 'Take the move back';

  @override
  String get npcUndoPlacementStale =>
      'The savegame no longer holds what that move wrote, so restoring it would discard what happened since.';

  @override
  String get positionNotReadable =>
      'The stored position could not be read for this character.';

  @override
  String get npcPositionReadOnly =>
      'The game restores an NPC\'s position from the level, not from the savegame, so these values can be read but not changed.';

  @override
  String get pickLocation => 'Choose location…';

  @override
  String get pickLocationDialogTitle => 'Choose a location';

  @override
  String get applySpotRotation => 'Also apply the spot\'s orientation';

  @override
  String get locationAreaOther => 'Other';

  @override
  String get locationAreaCavalornValley => 'Cavalorn Valley';

  @override
  String get locationAreaEastForest => 'East Forest';

  @override
  String get locationAreaFogTower => 'Fog Tower';

  @override
  String get locationAreaIllegalWeedMixers => 'Illegal Weed Mixers';

  @override
  String get locationAreaOrcArena => 'Orc Arena';

  @override
  String get locationAreaOrcGraveyard => 'Orc Graveyard';

  @override
  String get locationAreaShipwreck => 'Shipwreck';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'The location catalog could not be loaded.';

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
      'The exhaustive source browser is currently available for GSAV save files.';

  @override
  String get allDataDescription =>
      'Browse GSAV metadata and every typed PUBLIC/PRIVATE node. Safe scalar and native-struct values are editable; containers and opaque bytes remain visible.';

  @override
  String get allDataEditable => 'Editable';

  @override
  String get allDataReadOnly => 'Read-only';

  @override
  String get allDataType => 'Type';

  @override
  String get allDataScalars => 'Scalars';

  @override
  String get allDataStructs => 'Structs';

  @override
  String get allDataContainers => 'Containers';

  @override
  String get allDataOpaque => 'Opaque';

  @override
  String get allDataNodes => 'Nodes';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count children',
      one: '1 child',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Pending';

  @override
  String get allDataTagInputHint => 'Comma- or line-separated tags';

  @override
  String allDataTypedSource(String source) {
    return '$source typed';
  }

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
  String updateOpenFailed(String url) {
    return 'Could not open the download page. You can reach it at $url';
  }

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
  String get itemImagesTitle => 'Item images';

  @override
  String get loadingTexts => 'Loading texts…';

  @override
  String get loadingImages => 'Loading images…';

  @override
  String get preparing => 'Preparing…';

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
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get heroGroupMainStats => 'Main Stats';

  @override
  String get heroGroupCombatMovement => 'Combat / Movement';

  @override
  String get heroGroupResistances => 'Resistances';

  @override
  String get heroGroupThieving => 'Thieving';

  @override
  String get heroGroupAdvanced => 'Advanced';

  @override
  String get heroGroupDiving => 'Diving';

  @override
  String get heroDivingSkillNote =>
      'Once Diving is learned, the game resets breath and recovery to the skill\'s own values every time the savegame loads. Breath used per second stays as you set it.';

  @override
  String get heroGroupSleep => 'Sleep & Rest';

  @override
  String get heroGroupIntoxication => 'Intoxication';

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

  @override
  String get windowMinimizeTooltip => 'Minimize';

  @override
  String get windowMaximizeTooltip => 'Maximize';

  @override
  String get windowRestoreTooltip => 'Restore';

  @override
  String get fallbackDialogEntry => 'Dialog entry';

  @override
  String get fallbackDialogChoice => 'Dialog choice';

  @override
  String get fallbackDialogTopic => 'Dialog topic';

  @override
  String get fallbackDialogInformation => 'Dialog information';

  @override
  String get fallbackQuest => 'Quest';

  @override
  String get fallbackObjective => 'Objective';

  @override
  String get fallbackItem => 'Item';

  @override
  String get attributeSkillPointsFallback => 'Skill points (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Poise',
      'MaxSuperArmor': 'Maximum poise',
      'DamageMultiplier': 'Damage taken',
      'SpeedModifier': 'Movement speed',
      'Oxygen': 'Breath',
      'MaxOxygen': 'Maximum breath',
      'OxygenDepletionRate': 'Breath used per second',
      'OxygenRecoveryRate': 'Breath regained per second',
      'CriticalLevelPercent': 'Low-breath warning',
      'SleepTime': 'Restful hours left',
      'MaxSleepTime': 'Maximum restful hours',
      'SleepTimeRecoveryAmount': 'Restful hours regained',
      'SleepTimeRecoveryPeriod': 'Refill interval',
      'MaxRestTime': 'Maximum time in bed',
      'Health_RecoveryRatePerHourOfSleep': 'Health per hour of sleep',
      'Mana_RecoveryRatePerHourOfSleep': 'Mana per hour of sleep',
      'Alcohol': 'Alcohol level',
      'MaxAlcohol': 'Maximum alcohol',
      'AlcoholDepletionRate': 'Sobering speed',
      'Swampweed': 'Swampweed level',
      'MaxSwampweed': 'Maximum swampweed',
      'SwampweedDepletionRate': 'Wear-off speed',
      'XPExecutedBounty': 'XP for finishing off',
      'XPKillOrDefeatBounty': 'XP for defeating',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor':
          'How much punishment this character absorbs before a hit staggers them.',
      'MaxSuperArmor':
          'The full poise pool; it grows with character level and with worn armour.',
      'DamageMultiplier':
          'Factor applied to the damage this character takes — 1 is normal, higher hurts more.',
      'SpeedModifier': 'Factor on how fast this character moves — 1 is normal.',
      'Oxygen':
          'Seconds of air left under water; at zero this character drowns.',
      'MaxOxygen':
          'How many seconds this character can stay under water; the Diving skill raises it.',
      'OxygenDepletionRate': 'Air used up each second while submerged.',
      'OxygenRecoveryRate': 'Air that comes back each second after surfacing.',
      'CriticalLevelPercent':
          'Share of remaining air at which the game warns of drowning.',
      'SleepTime':
          'Hours of sleep that still restore something; beyond them the game grants no resting bonus.',
      'MaxSleepTime':
          'The largest budget of restful hours this character can hold.',
      'SleepTimeRecoveryAmount':
          'Restful hours added back each time the budget refills.',
      'SleepTimeRecoveryPeriod':
          'How long it takes before the budget of restful hours refills again.',
      'MaxRestTime': 'The longest single stay in bed the game allows.',
      'Health_RecoveryRatePerHourOfSleep':
          'Share of maximum health restored for every hour slept.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Share of maximum mana restored for every hour slept.',
      'Alcohol':
          'How drunk this character is; the higher tiers trade dexterity and mana for strength.',
      'MaxAlcohol': 'The highest alcohol level this character can reach.',
      'AlcoholDepletionRate':
          'How quickly the alcohol level falls back towards sober.',
      'Swampweed':
          'How stoned this character is; the higher tiers shift their attributes around.',
      'MaxSwampweed': 'The highest swampweed level this character can reach.',
      'SwampweedDepletionRate': 'How quickly the swampweed high wears off.',
      'XPExecutedBounty':
          'Experience for killing this character while it already lies defeated on the ground.',
      'XPKillOrDefeatBounty':
          'Experience for bringing this character down, whether it dies or is only beaten unconscious.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Voice line';

  @override
  String get knowledgeTypeOther => 'Other';

  @override
  String get armorUpgradeUpper => 'Upper';

  @override
  String get armorUpgradeMiddle => 'Middle';

  @override
  String get armorUpgradeLower => 'Lower';

  @override
  String get knowledgeCategoryTopic => 'Topic';

  @override
  String get knowledgeCategoryChoice => 'Choice';

  @override
  String get knowledgeCategoryInfo => 'Information';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Failed';

  @override
  String get missingSaveReference => 'File missing';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav is missing. It may have been deleted, moved, or renamed; the profile still references it.';
  }

  @override
  String get removeFromProfile => 'Remove from profile';

  @override
  String get removeSaveFromProfileTitle => 'Remove save from profile?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Remove $save from $profile? The save file itself will be kept if it still exists.';
  }

  @override
  String get unassignedSave => 'Not assigned to a profile';

  @override
  String get armorUpgradeLight => 'Light';

  @override
  String get armorUpgradeMedium => 'Medium';

  @override
  String get armorUpgradeHeavy => 'Heavy';

  @override
  String get knowledgeCaptionForcedConversation => 'Forced conversation';

  @override
  String get knowledgeCaptionFollowupTopic => 'Follow-up topic';

  @override
  String get knowledgeCaptionFallbackTopic => 'Fallback topic';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours hr';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours hr $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Invalid profile data';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Selected save metadata is missing';

  @override
  String defaultProfileName(int id) {
    return 'Profile $id';
  }

  @override
  String get statusUnknown => 'Unknown';

  @override
  String editorUnexpectedError(String details) {
    return 'Unexpected error: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Another operation is in progress. Try again in a moment.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'You have unsaved save edits. Save or reset them before changing the profile difficulty.';

  @override
  String get editorNoSaveFolderSelected => 'No save folder selected.';

  @override
  String get editorNoSaveSelected => 'No save selected.';

  @override
  String get coreUnknownError => 'Unknown core error';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Save or reset your unsaved changes first — switching profiles would move away from the current save.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Save or reset your unsaved changes before opening another file.';

  @override
  String get editorSelectSavFile => 'Select a .sav savegame file.';

  @override
  String get editorNotGothicGsav =>
      'The selected file is not a Gothic GSAV savegame.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Save or reset your unsaved changes before changing the save profile.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Save or reset your unsaved changes before removing a save from its profile.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'You have unsaved save edits. Save or reset them before restoring a profile backup.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Conflicting unsaved edits target the same property ($path) from two tabs. Reset or revert one of them, then save again.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'A glossary segment change and another unsaved All-data edit both target the Hero MemorizedEvents array ($path). Glossary changes add or remove entries in that array, so the edits cannot be saved together — reset or revert one of them, then save again.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'A glossary segment change and another unsaved edit target the same quest CurrentState property ($path). The glossary change updates that state itself — reset or revert one of them, then save again.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'A relationship override and another unsaved All-data edit both target the same NPC relationship entry ($path). The structured relationship change can replace modifiers in that entry, so the edits cannot be saved together — reset or revert one of them, then save again.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'More than one unsaved structural edit targets the same array ($path). Save or reset the first change before queuing another.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'A structural event change and another unsaved All-data edit both target $path. Save or reset one of them before continuing.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'A Skills change and an All-data edit to the same actor’s effect (ActiveEffects › EffectSpec › Def) are both queued. They cannot be saved together — reset or revert one of them, then save again.';

  @override
  String get editorInventoryResetConflict =>
      'An inventory reset and another edit to the same inventory are both queued. The reset replaces the entire inventory and would discard the other edit — reset or revert one of them, then save again.';

  @override
  String get editorUseFolder => 'Use folder';

  @override
  String get editorGothicSavegameFileType => 'Gothic savegame';

  @override
  String get editorNoDifficultyChanges => 'No difficulty changes to write';

  @override
  String get editorDifficultyWritten =>
      'Difficulty written to the profile (backup created)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes saved with backup',
      one: '1 change saved with backup',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'The move was saved, but its undo note could not be written: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profile $profileId was not found.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'No free save slot is available in the game save folder (G1R-001 through G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Save imported and assigned to profile $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Save assigned to profile $profileId (paired backups created)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Save slot $slot is not assigned to profile $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Save removed from profile';

  @override
  String editorRestoredBackup(String path) {
    return 'Restored backup: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Restored backup: $path (PersistentDataList.sav left unchanged — no matching companion backup; slot metadata may differ)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Codec roundtrip passed: chunk $chunkIndex recompressed to $bytes bytes';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Could not write the profile difficulty: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Could not assign the save to the profile: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Could not remove the save from the profile: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Could not save the changes: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Failed to scan saves: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Failed to inspect save: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Failed to load backups: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Could not restore the backup: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Restored backup: $path, but reloading the save failed: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Codec check failed: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Codec roundtrip failed: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Property search failed: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Save selection changed while loading hero attributes.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Skills load failed: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Progression query failed: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'NPC list failed: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Character list failed: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'NPC attributes failed: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Loading the NPC position failed: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'NPC inventory failed: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Faction list failed: $details';
  }

  @override
  String get editorNoBackupPath => 'none';

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
    return '$prefix: $backupPath; PersistentDataList backup: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Localization status failed: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Extraction failed: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Glossary load failed: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Backup error: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Quest',
      'document': 'Document',
      'story': 'Story',
      'exploration': 'Exploration',
      'combat': 'Combat',
      'social': 'Social',
      'item': 'Items',
      'learning': 'Learning',
      'guild': 'Guild',
      'crime': 'Crime',
      'rest': 'Rest',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Quest started',
      'questSucceeded': 'Quest completed',
      'questFailed': 'Quest failed',
      'documentRead': 'Document read',
      'documentSegmentUnlocked': 'Entry discovered',
      'documentSegmentViewed': 'Entry viewed',
      'chapterCompleted': 'Chapter completed',
      'areaEntered': 'Area entered',
      'areaLeft': 'Area left',
      'characterKilled': 'Character killed',
      'characterDefeated': 'Character defeated',
      'combatDodge': 'Attack dodged',
      'characterDebuffed': 'Debuff applied',
      'tradeAvailable': 'Trading unlocked',
      'itemObtained': 'Item obtained',
      'itemCrafted': 'Item crafted',
      'skillStateRecorded': 'Skill state recorded',
      'recipeLearned': 'Recipe learned',
      'guildJoined': 'Guild joined',
      'crimeRecorded': 'Crime recorded',
      'slept': 'Slept',
      'storyEvent': 'Story event',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action: $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'Game time',
      'duration': 'Duration',
      'chapter': 'Chapter',
      'instigator': 'Initiated by',
      'affected': 'Affected',
      'amount': 'Amount',
      'primaryObject': 'Object',
      'secondaryObject': 'Context',
      'segmentText': 'Entry text',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Day $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value s';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Hero';

  @override
  String get memoryEventDetails => 'Details';

  @override
  String get memoryEventTags => 'Tags';

  @override
  String get memoryEventTechnicalData => 'Technical data';

  @override
  String get memoryEventIndex => 'Index';

  @override
  String get memoryEventPosition => 'Position';

  @override
  String get memoryEventPayload => 'Payload';

  @override
  String get memoryEventSubject => 'Subject';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Access',
      'AccessDenied': 'Access Denied',
      'AccesToTemple': 'Access to Temple',
      'Advice': 'Advice',
      'AfterFight': 'After Fight',
      'AfterFireMages': 'After Fire Mages',
      'AfterNek': 'After Nek',
      'AfterQuest': 'After Quest',
      'Alone': 'Alone',
      'Amulet': 'Amulet',
      'Annoying': 'Annoying',
      'Armor': 'Armor',
      'Avoid': 'Avoid',
      'Backstory': 'Backstory',
      'BackStory': 'Back Story',
      'BasicMagic': 'Basic Magic',
      'Beated': 'Beaten',
      'BecomeMercenary': 'Become Mercenary',
      'Beer': 'Beer',
      'Bestiary': 'Bestiary',
      'Blessing': 'Blessing',
      'Boss': 'Boss',
      'Bully': 'Bully',
      'BullyAdvice': 'Bully Advice',
      'Camp': 'Camp',
      'CampDivided': 'Camp Divided',
      'CareOfMessengers': 'Care of Messengers',
      'ChangeOpinion': 'Change Opinion',
      'ChargeUriziel': 'Charge Uriziel',
      'Chosen': 'Chosen',
      'Contact': 'Contact',
      'Courier': 'Courier',
      'CraftBows': 'Craft Bows',
      'Crazy': 'Crazy',
      'DailyMeal': 'Daily Meal',
      'DailyRation_Trader': 'Daily Ration Trader',
      'DAM': 'Dam',
      'Dead': 'Dead',
      'Deal': 'Deal',
      'Dealer': 'Dealer',
      'Deceived': 'Deceived',
      'Dementia': 'Dementia',
      'DenyAccess': 'Deny Access',
      'DifferentOpinion': 'Different Opinion',
      'Discussion': 'Discussion',
      'DontTalk': 'Don’t Talk',
      'Duel': 'Duel',
      'Entrance': 'Entrance',
      'Escape': 'Escape',
      'Extended': 'Extended',
      'Extra': 'Extra',
      'ExtraInfo': 'Extra Info',
      'Fanatic': 'Fanatic',
      'Fight': 'Fight',
      'FindUlumulu': 'Find Ulu-Mulu',
      'FireMages': 'Fire Mages',
      'FireMagesEscape': 'Fire Mages Escape',
      'FiskNewDealer': 'New Fence for Fisk',
      'FiskNewDealerCompleted': 'New Fence for Fisk — Completed',
      'FogTower': 'Fog Tower',
      'Food': 'Food',
      'Forgave': 'Forgave',
      'Forgive': 'Forgive',
      'Forgiven': 'Forgiven',
      'FourFriends': 'Four Friends',
      'FreeHut': 'Free Hut',
      'FreeMine': 'Free Mine',
      'Fury': 'Fury',
      'GoodTeacher': 'Good Teacher',
      'Gossip': 'Gossip',
      'GotScavenger': 'Got Scavenger',
      'GrantedAccess': 'Granted Access',
      'GRDArmor': 'Guard Armor',
      'Guide': 'Guide',
      'HateMages': 'Hate Mages',
      'HateMagesExplanation': 'Hate Mages Explanation',
      'HateRiceLord': 'Hate Rice Lord',
      'Heal': 'Heal',
      'Healing': 'Healing',
      'Help': 'Help',
      'Helper': 'Helper',
      'HelpKagan': 'Help Kagan',
      'HutStory': 'Hut Story',
      'Ignore': 'Ignore',
      'Impress': 'Impress',
      'ImpressAlchemy': 'Impress Alchemy',
      'ImpressInscription': 'Impress Inscription',
      'Info': 'Info',
      'Interested': 'Interested',
      'Introduction': 'Introduction',
      'Introduction_2': 'Introduction 2',
      'Introduction_Armor': 'Introduction – Armor',
      'Introduction_Teacher': 'Introduction – Teacher',
      'Introduction_Trader': 'Introduction – Trader',
      'Invocation': 'Invocation',
      'JoinSC': 'Join Swamp Camp',
      'Joint': 'Joint',
      'KalomCamp': 'Kalom Camp',
      'Leader': 'Leader',
      'Learning': 'Learning',
      'LearnOrcish': 'Learn Orcish',
      'LeftParty': 'Left Party',
      'Library': 'Library',
      'Lie': 'Lie',
      'Lock': 'Lock',
      'Lockpick': 'Lockpick',
      'Mad': 'Mad',
      'Mandibles': 'Minecrawler Mandibles',
      'MapMaker': 'Map Maker',
      'Monastery': 'Monastery',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'New Camp',
      'NewCamper': 'New Camper',
      'NewLeader': 'New Leader',
      'NightPatrol': 'Night Patrol',
      'NotInterested': 'Not Interested',
      'OldCamp': 'Old Camp',
      'OrcEnclaveEntrance': 'Orc Enclave Entrance',
      'OrcGraveyard': 'Orc Graveyard',
      'OreArmor': 'Ore Armor',
      'Party': 'Party',
      'Pay': 'Pay',
      'PayMoney': 'Pay Money',
      'Permission': 'Permission',
      'Pet': 'Pet',
      'PreparingInvocation': 'Preparing Invocation',
      'Quest': 'Quest',
      'RankUpFireMages': 'Fire Mage Promotion',
      'RankUpGuard': 'Guard Promotion',
      'RanUpFireMagesCompleted': 'Fire Mage Promotion Completed',
      'Realocated': 'Relocated',
      'Reason': 'Reason',
      'Respect': 'Respect',
      'ReturnToSC': 'Return to Swamp Camp',
      'RicelordForeman': 'Rice Lord’s Foreman',
      'RideScavenger': 'Ride Scavenger',
      'Robe': 'Robe',
      'Safe': 'Safe',
      'Scraper': 'Scraper',
      'SecondChance': 'Second Chance',
      'SecretLocation': 'Secret Location',
      'SecretPassage': 'Secret Passage',
      'SecretPath': 'Secret Path',
      'SleeperFollower': 'Sleeper Follower',
      'SleeperTemple': 'Sleeper Temple',
      'SmallInfo': 'Small Info',
      'Stonehenge': 'Stonehenge',
      'StopFollowing': 'Stop Following',
      'SwampCamp': 'Swamp Camp',
      'Talkative': 'Talkative',
      'Teach': 'Teach',
      'TeachBow': 'Teach Bow',
      'Teacher': 'Teacher',
      'Teacher2': 'Teacher 2',
      'TeacherInscription': 'Teacher Inscription',
      'TeacherMana': 'Teacher Mana',
      'TeachIchor': 'Teach Minecrawler Ichor Extraction',
      'TeachMagic': 'Teach Magic',
      'TeachOrcish': 'Teach Orcish',
      'TeachStats': 'Teach Stats',
      'TeachWeapon': 'Teach Weapon',
      'Teleport': 'Teleport',
      'TheMysteriousOrc': 'The Mysterious Orc',
      'ThroneRoom': 'Throne Room',
      'TradeBow': 'Trade Bow',
      'Trader': 'Trader',
      'TradeSkins_Trader': 'Skin Trader',
      'Traitor': 'Traitor',
      'Trial': 'Trial',
      'TrollCanyon': 'Troll Canyon',
      'Trust': 'Trust',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Inexperienced',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Uriziel Rune',
      'Useful': 'Useful',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrations',
      'WaitFreeMine': 'Wait at Free Mine',
      'WaitInTrainingArea': 'Wait In Training Area',
      'Warning': 'Warning',
      'WarningTooLate': 'Warning Came Too Late',
      'WaterMessenger': 'Messenger for the Water Mages',
      'Weapon': 'Weapon',
      'Who': 'Who',
      'Women': 'Women',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Damaged inventory slots';

  @override
  String slotRepairBody(int count) {
    return 'This savegame holds $count inventory slots whose id no longer matches their position — in the game, dropping such an item removes a different one instead. The repair only rewrites the ids: no item is added, removed or changed. A backup is created when you save, as always.';
  }

  @override
  String get slotRepairQueued => 'Repair queued — save to apply it.';

  @override
  String get slotRepairAction => 'Repair';

  @override
  String get slotRepairDiscard => 'Discard';

  @override
  String get editorInventorySlotEditConflict =>
      'A direct edit of an inventory slot is queued together with a change that claims whole slots (repair, add or remove). The second would overwrite the first — revert one of them, then save again.';

  @override
  String get editorTraderArrayConflict =>
      'A trade change is queued together with a direct edit of the trader array. That edit renumbers the rows a trade change is addressed by, so one of the two would land on the wrong merchant — revert one of them, then save again.';

  @override
  String get backupFactFile => 'File';

  @override
  String get renameBackupTooltip => 'Name this backup';

  @override
  String get renameBackupTitle => 'Name backup';

  @override
  String get renameBackupLabel => 'Name';

  @override
  String renameBackupHelp(String fileName) {
    return 'Shown instead of the file name $fileName. Leave empty to remove the name; the file itself is not renamed.';
  }

  @override
  String get deleteBackupTooltip => 'Delete this backup';

  @override
  String get deleteBackupTitle => 'Delete backup';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Delete “$name” ($fileName)? The file is removed from disk and cannot be brought back.';
  }

  @override
  String get deleteBackupConfirm => 'Delete';

  @override
  String editorDeletedBackup(String path) {
    return 'Backup deleted: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Could not delete the backup: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Could not name the backup: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Repairing is not possible right now — this savegame cannot be written.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Backup deleted: $path — its name could not be removed: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'The repair is not available for this savegame.';
}
