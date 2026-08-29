/// Names of the shared game UI glyphs (`/Game/UI/Textures/Common/Icons`) and
/// the mapping from what the editor labels to the glyph the game itself draws
/// in front of that label.
///
/// Every name here must also be listed in `UI_ICON_IDS` in gore-save, which is
/// what actually extracts them; a name missing from either side simply falls
/// back to the editor's own icon, so the two lists never have to be in lockstep.
library;

import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart'
    show HeroAttributeGroup, heroAttributeGroup, heroAttributeKey;
import 'package:goresave/features/editor/domain/item_categories.dart'
    show ItemCategory;
import 'package:goresave/features/editor/domain/quest_journal.dart'
    show QuestJournalSection;

/// The game's own generic bullet — the ◆ it puts in front of a value that has
/// no icon of its own (item requirements, fall protection, spell stats). Used
/// where the editor has no better icon at all, so a row is never left ragged.
const gameIconGenericBullet = 'T_Icon_SkillPoints';

/// Groups the game never puts on its own character screen: breath, sleep,
/// booze and the raw catch-all. It has no icon for any of these values, and
/// inventing one per row only makes them look more meaningful than they are —
/// so every row in them takes the ◆ bullet the game marks a plain value with.
const _bulletOnlyGroups = <HeroAttributeGroup>{
  HeroAttributeGroup.diving,
  HeroAttributeGroup.sleep,
  HeroAttributeGroup.intoxication,
  HeroAttributeGroup.advanced,
};

/// Glyph shown in front of one gameplay attribute row, or null when the game
/// has none for it (the caller then keeps its own icon, or the ◆ bullet).
///
/// Keyed by [heroAttributeKey], so the four ids that exist in several attribute
/// sets can be told apart.
String? gameIconForAttribute(String attributeId, [String? setClass]) {
  final id = attributeId.trim();
  if (_bulletOnlyGroups.contains(heroAttributeGroup(id, setClass))) return null;
  return _attributeIcons[heroAttributeKey(id, setClass)];
}

const _attributeIcons = <String, String>{
  // Core. Level and Experience have no glyph in the game either.
  'Health': 'T_Icon_Health',
  'MaxHealth': 'T_Icon_Health',
  'Mana': 'T_Icon_Mana',
  'MaxMana': 'T_Icon_Mana',
  'Strength': 'T_Icon_Strength',
  'Dexterity': 'T_Icon_Dexterity',
  'MagicianLevel': 'T_Icon_MagicCircle',
  'SkillPoints': 'T_Icon_Book_Small',
  'Toughness': 'T_Icon_Weight',

  // Protection. The game shows a shield per damage type and the boot for the
  // fall protection it lists as "Falling" next to them.
  'Resistance_Blunt': 'T_Icon_Resistance_Blunt',
  'Resistance_Edge': 'T_Icon_Resistance_Edge',
  'Resistance_Point': 'T_Icon_Resistance_Point',
  'Resistance_Fire': 'T_Icon_Resistance_Fire',
  'Resistance_Energy': 'T_Icon_Resistance_Energy',
  'Resistance_Ice': 'T_Icon_Resistance_Ice',
  'Resistance_Wind': 'T_Icon_Resistance_Wind',
  'Resistance_Falling': 'T_Icon_Squash',

  // Combat and movement.
  'SuperArmor': 'T_Icon_Toughness',
  'MaxSuperArmor': 'T_Icon_Toughness',
  'DamageMultiplier': 'T_Icon_Weapon',
  'SpeedModifier': 'T_Icon_Acrobatics',

  // Thieving.
  'LockpickDurability': 'T_Interaction_Lockpick',
  'LockpickPrecision': 'T_Interaction_Lockpick',
  'PickPocketing': 'T_Interaction_Steal',

  // Breath, sleep, booze and everything the catch-all collects are absent on
  // purpose: see [_bulletOnlyGroups]. Adding a row here would not show it.
};

/// Glyph for one attribute sidebar group.
String? gameIconForAttributeGroup(HeroAttributeGroup group) {
  return switch (group) {
    HeroAttributeGroup.core => 'T_Icon_Health',
    HeroAttributeGroup.combat => 'T_Icon_Melee',
    HeroAttributeGroup.resistances => 'T_Icon_Resistance_Blunt',
    HeroAttributeGroup.thieving => 'T_Interaction_Steal',
    HeroAttributeGroup.diving => 'T_Icon_Dive',
    HeroAttributeGroup.sleep => 'T_Interaction_Sleep',
    HeroAttributeGroup.intoxication => 'T_Icon_Potion',
    HeroAttributeGroup.advanced => 'T_Icon_Tutorials',
  };
}

/// Glyph for an inventory group header — the icon the game's own inventory rail
/// puts on that tab.
///
/// The bundled item stats carry the very same names next to their filter, so
/// callers that have them should prefer those; this is the fallback for when
/// they failed to load.
String? gameIconForItemCategory(ItemCategory category) {
  return switch (category) {
    ItemCategory.meleeWeapon => 'T_Icon_Melee',
    ItemCategory.rangedWeapon => 'T_Icon_Bow',
    ItemCategory.magic => 'T_Icon_Magic',
    ItemCategory.wearable => 'T_Icon_Armor',
    ItemCategory.food => 'T_Icon_Food',
    ItemCategory.potion => 'T_Icon_Potion',
    ItemCategory.material => 'T_Icon_Materials',
    ItemCategory.document => 'T_Icon_Book',
    ItemCategory.misc => 'T_Icon_Misc',
    ItemCategory.artefact => 'T_Icon_Key',
    ItemCategory.other => null,
  };
}

/// Glyph for one learnable skill, keyed by the catalog `base` the core reports
/// (`Melee_OneHanded`, `Hunting_Claw`, …). Every `Hunting_*` skill shares the
/// hunting knife, which is what the game shows for them too. Null keeps the
/// panel's own icon.
String? gameIconForSkill(String skillBase) {
  final base = skillBase.trim();
  if (base.startsWith('Hunting_')) return 'T_Icon_Hunting';
  return _skillIcons[base];
}

const _skillIcons = <String, String>{
  'Melee_OneHanded': 'T_Icon_1handed',
  'Melee_TwoHanded': 'T_Icon_2Handed',
  'Melee_Fists': 'T_Icon_Fist',
  'Melee_Orc': 'T_Icon_OrcWeapon',
  'Ranged_Bow': 'T_Icon_Bow',
  'Ranged_Crossbow': 'T_Icon_Crossbow',
  'Picklock': 'T_Interaction_Lockpick',
  'Pickpocket': 'T_Interaction_Steal',
  'Acrobatics': 'T_Icon_Acrobatics',
  'Wallclimbing': 'T_Interaction_ClimbWall',
  'Riding': 'T_Icon_Ride',
  // The game's own `T_Icon_Legs` is the trousers item icon, painted brown, so
  // it cannot take the theme's colour; its boot reads as quiet feet and can.
  'Sneak': 'T_Icon_Squash',
  'Diving': 'T_Icon_Dive',
  'Crafting_Alchemy': 'T_Interaction_Alchemy',
  'Crafting_Inscription': 'T_Interaction_Inscription',
  'Crafting_Blacksmith': 'T_Interact_Forge',
  'Mining': 'T_Interaction_Mine',
  'Mage_Circle': 'T_Icon_MagicCircle',
  'Orcish': 'T_Icon_Orchist',
};

/// Glyph the game's own quest journal puts on a section tab.
String? gameIconForQuestSection(QuestJournalSection section) {
  return switch (section) {
    QuestJournalSection.oldCamp => 'T_Icon_OldCamp',
    QuestJournalSection.newCamp => 'T_Icon_NewCamp',
    QuestJournalSection.swampCamp => 'T_Icon_SwampCamp',
    // The colony's own quests carry the game's plain quest marker.
    QuestJournalSection.colony => 'T_Icon_SideQuest',
    QuestJournalSection.completed => 'T_Icon_Commpleted',
  };
}

/// Glyph the game's own glossary puts on a category tab, by the camp the
/// editor's catalog files an entry under. `outsiders` are the people who belong
/// to no camp, which the game marks with its plain character glyph.
String? gameIconForGlossaryCamp(String camp) {
  return switch (camp) {
    'oldCamp' => 'T_Icon_OldCamp',
    'newCamp' => 'T_Icon_NewCamp',
    'swampCamp' => 'T_Icon_SwampCamp',
    'outsiders' => 'T_Icon_Characters',
    'creatures' => 'T_Icon_Creatures',
    'locations' => 'T_Icon_Location',
    'tutorials' => 'T_Icon_Tutorials',
    _ => null,
  };
}

/// The mark the game puts on a faction, by the guild tag the save records
/// crimes against.
///
/// The game's tag tree holds about forty guilds but draws a crest for only
/// three of them, the camps. Everything that belongs to a camp takes that
/// camp's crest — the guards, shadows and fire mages of the Old Camp, the
/// mercenaries, water mages and rogues of the New Camp, the novices and templars
/// of the Swamp Camp — including the bandits, whom the tag tree files beside the
/// camps rather than under one. The orcs and the undead get their own marks; the
/// rest keep the editor's own icon.
String? gameIconForGuild(String guild) {
  if (guild.contains('.OldCamp')) return 'T_Icon_OldCamp';
  if (guild.contains('.NewCamp') || guild.contains('.Bandit')) {
    return 'T_Icon_NewCamp';
  }
  if (guild.contains('.SwampCamp')) return 'T_Icon_SwampCamp';
  if (guild.startsWith('Guild.Orc')) return 'T_Icon_OrcWeapon';
  if (guild.startsWith('Guild.Undead')) return gameIconDead;
  return null;
}

/// The glyph the game marks a person with. Used where the editor needs to stand
/// in for a character it has no portrait for — the player, and every generic
/// NPC (a worker, a bandit) the glossary has no entry for.
const gameIconCharacter = 'T_Icon_Characters';

/// The glyph the game marks a creature with — every monster the glossary holds
/// no portrait of.
const gameIconCreature = 'T_Icon_Creatures';

/// The glyph the game marks a killed character with.
///
/// The game's own death badge (`T_Icon_Dead`) is a finished picture with its
/// own dark ring, which reads as a blot on a light theme. This one is plain
/// white line work, so it takes the theme's colour like every other glyph.
const gameIconDead = 'T_Interaction_Execute';

/// The glyph for a character's captured dialogue knowledge — the game's own
/// speech bubble, the same one the Knowledge tab carries.
const gameIconKnowledge = 'T_Interaction_Talk';

/// The glyph for a character who runs a shop — the game's coin pouch, the same
/// mark the Trade tab carries.
const gameIconTrade = 'T_Icon_Misc';

/// The glyph for a character who teaches a skill — the game's mastery mark.
///
/// NOT its `T_Icon_Trainer` badge: that one is a finished picture with its own
/// dark ring, so it cannot take the theme's colour. See
/// [gameIconsWithOwnColours].
const gameIconTeacher = 'T_Icon_Master';

/// The glyph for a character who makes armour — the game's own armour mark,
/// again in place of the ringed `T_Icon_Armorer` badge.
const gameIconArmorer = 'T_Icon_Armor';

/// The glyph for one of the roles the glossary files a character under, or null
/// where the game draws none that takes a colour.
String? gameIconForNpcRole(NpcGlossaryRole role) => switch (role) {
  NpcGlossaryRole.trader => gameIconTrade,
  NpcGlossaryRole.teacher => gameIconTeacher,
  NpcGlossaryRole.armorer => gameIconArmorer,
  NpcGlossaryRole.dead => gameIconDead,
  // The game's hostility and portrait marks are both ringed badges.
  NpcGlossaryRole.hostile || NpcGlossaryRole.portrait => null,
};

/// Glyphs that are NOT plain white artwork and must never be recoloured.
///
/// Most shared glyphs are white line work on transparency and have to be tinted
/// to read on either theme. These are finished pictures instead — round badges
/// with their own dark ring, skin-toned equipment slots, photographed trophies —
/// and tinting flattens each into a single blob. Determined by scanning the
/// shipped PNGs, not by eye: every visible pixel of the others is near-white
/// grey, these have colour.
const gameIconsWithOwnColours = <String>{
  'T_Icon_Angry',
  'T_Icon_Armorer',
  'T_Icon_Arrows',
  'T_Icon_Dead',
  'T_Icon_Enemy',
  'T_Icon_Friendly',
  'T_Icon_Hostile',
  'T_Icon_Legs',
  'T_Icon_MinusRed',
  'T_Icon_Shop',
  'T_Icon_Stolen',
  'T_Icon_Torso',
  'T_Icon_Trainer',
  'T_Icon_Waist',
  'T_ItemIcon_Claws',
  'T_ItemIcon_Teeth',
};
