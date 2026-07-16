/// Conservative source-derived meaning of one persisted G1R integer field.
///
/// The evidence scan covered 7,305 decompiled AngelScript files. The direct
/// declaration input (`StoryG1R.as`) has SHA-256
/// `c5e9fc15e876c21d414da6b3b2c26b5627d8e17b940bfa1b3f7d4225d4d1e07c`.
///
/// This deliberately excludes the 50 `FInGameTime` fields and inherited
/// `Chapter`: both already have stronger declared types. The remaining 419
/// fields are all serialized as `int32`, so these categories guide the UI but
/// never narrow the raw storage domain.
enum StoryIntegerKind {
  binaryFlag,
  finiteState,
  counterOrScore,
  calendarDay,
  derivedOrOpaqueInteger,
  readOnlyInSourceInteger,
  dormantOrLegacyInteger,
}

class StoryIntegerSemantics {
  const StoryIntegerSemantics({
    required this.id,
    required this.kind,
    required this.confidence,
    this.knownValues = const [],
  });

  final String id;
  final StoryIntegerKind kind;

  /// Short source-confidence token suitable for mapping to localized UI text.
  final String confidence;

  /// Values evidenced by shipped source/defaults or the researched saves.
  ///
  /// They are suggestions, not an exhaustive validation range. Native code,
  /// migrations, game updates, and mods may use additional `int32` values.
  final List<int> knownValues;
}

/// The exact 419-field direct-integer catalog from the researched shipping
/// cache, in stable kind/source order.
final List<StoryIntegerSemantics> storyIntegerSemanticsCatalog =
    List<StoryIntegerSemantics>.unmodifiable(
      _storyIntegerIdsByKind.entries.expand(
        (group) => _splitStoryIds(group.value).map(
          (id) => StoryIntegerSemantics(
            id: id,
            kind: group.key,
            confidence: _confidenceFor(group.key),
            knownValues:
                _knownValuesById[id] ??
                (group.key == StoryIntegerKind.binaryFlag
                    ? _binaryValues
                    : const []),
          ),
        ),
      ),
    );

final Map<String, StoryIntegerSemantics> _storyIntegerSemanticsById = {
  for (final value in storyIntegerSemanticsCatalog)
    value.id.toLowerCase(): value,
};

/// Looks up one source-known integer field using ASCII case-insensitive ID
/// matching. Unknown/modded IDs, time markers, and `Chapter` return `null`.
StoryIntegerSemantics? storyIntegerSemantics(String id) =>
    _storyIntegerSemanticsById[id.trim().toLowerCase()];

const _binaryValues = <int>[0, 1];

String _confidenceFor(StoryIntegerKind kind) {
  switch (kind) {
    case StoryIntegerKind.binaryFlag:
    case StoryIntegerKind.finiteState:
    case StoryIntegerKind.counterOrScore:
    case StoryIntegerKind.calendarDay:
      return 'high-source-evidence';
    case StoryIntegerKind.derivedOrOpaqueInteger:
      return 'medium-source-evidence';
    case StoryIntegerKind.readOnlyInSourceInteger:
      return 'medium-no-script-write';
    case StoryIntegerKind.dormantOrLegacyInteger:
      return 'no-live-script-reference';
  }
}

Iterable<String> _splitStoryIds(String ids) =>
    ids.trim().split(RegExp(r'\s+')).where((id) => id.isNotEmpty);

const Map<StoryIntegerKind, String> _storyIntegerIdsByKind = {
  StoryIntegerKind.binaryFlag: '''
AfterCinematic_Nyras AfterCinematic_Sleeper BaalCadar_responsive BaalCadar_Sacrilege
BaalIsidro_GotDrink BaalLukor_KeyPart01 BaalLukor_KeyPart02 BaalNamib_responsive BaalNamib_Sacrilege
BaalOrun_responsive BaalOrun_Sacrilege BaalTyon_responsive BaalTyon_Sacrilege Balor_PlayerCheating
Balor_TellsNCDealer Baloro_SC_choice Baloro_SC_wantsToKnow Bloodwyn_ProtectionPaid
Bouncer876_GotJoint Brannok_Permission BranNoteRead BridgeGolemCombatActive BridgeStoneGolemEnemy
BullitDefeated CaineVanished CanUpgradeGuardArmor CanUpgradeNoviceArmor Cavalorn_BestiaryDiscovered
Cavalorn_BestiaryQuestionRunning Chokta_angry Cipher_Trade CollapseMineNotify
ConversationWithDiegoAtStoneHenge Convoy_PlayerBriefed CorAngar_GotoOGY Corristo_FirsTalk
Damarok_GlandNegotiation Darrion_Teacher Dexter_SC Dexter_Traded Diego_After_Gamestart
Diego_GomezAudience Drax_CanTeach Drax_GotBeer Dusty_flags EnteredFreeMine Fingers_CanTeach
FireMagesPermission Fisk_ForgetSword Fisk_SellSword110 Fisk_SwordSold Fletcher_foundNek
Fletcher_whytalk FMHostile ForgedRivalries2_DarrionAngry ForgedRivalries2_DarrionBanTrade
ForgedRivalries2_StoneKnowsDarrionPlanFull ForgedRivalries2_StoneKnowsDarrionPlanPartial
ForgedRivalries2_StoneKnowsDesignIsGomez ForgedRivalries2_StoneMarkedCraft
ForgedRivalries3_DarrionWillAcceptStone ForgedRivalries3_StoneKnowsDarrionAccept
ForgedRivalries3_StoneKnowsDarrionReject ForgedRivalries3_StoneRefreshed
ForgedRivalries_DarrionCraftDishonest ForgedRivalries_DarrionCraftHonest
ForgedRivalries_DeliverStoneCraft ForgedRivalries_OwnCraftDishonest ForgedRivalries_StoneCraft
Fortress_Inside Fortress_Outside Freemine_Recovered FriendOfUrShak Friends_SendToNC GorHanis_Win
Gorn_AloneForFM Gorn_GotoWolf Gorn_JoinedForFM Graham_OMMapSold Grim_ProtectionBully Grim_Tests
Guard_Permission_Orc_Land Haenno_Bow_Knowledge HappyFriends HasULUMULU Herek_ProtectionBully
HeroInsideBanditCell HeroInsideThroneRoom IlegalWeedMixer_Permision
Info_Bartholo_Krautbote_permanent Info_Xardas_LOADSWORD09_permanent IntroInExtremo
Jackal_ProtectionPaid JackoNoteRead Jan_Training Jeremiah_Brewer Joru_JoinSC Joru_Tips
Joru_Tips_Mage Kalom_DeliveredWeed Kalom_TalkedTo KDW_600_Saturas_HEAVYARMOR_permanent
Kharim_Challenged Kharim_Lose Kharim_Win Kirgo_Challenged Kirgo_Lose Kirgo_Win Knows_GetMCPlates
KnowStone Lares_Permission Lee_freeminereport Lee_SldPossible Lefty_CarriedWater Lefty_WasBeaten
Lester_Show Location_AbandonedMine_AfterAmulet Location_AbandonedMine_OrcGrave
Location_NewCamp_OrePile Location_OldMineCollapsed Location_OrcEnclave_Arena
Location_SwampCamp_Temple Location_XardasTower_Bedroom LogBaalcadarsell LogBaalcadartrain
LogCavalorntrain LogDiegotrain LogGornatothfight LogGornatothtrain LogScattytrain LogScorpiocrossbow
LogThorustrain LogWedgelearn Magician_Level MCPlatesDelivered Milten_HasLetter Monastery_Inside
Monastery_Outside Mordrag_Traded MordragKO_Exiled MordragKO_PlayerChoseOreBarons
MordragKO_PlayerChoseThorus MordragKO_StayAtNC Mud_Follow Mud_NerveRealized Mud_OrcGraveyard
Novice_Guide_Kalom Novice_Guide_MainGate Novice_Guide_Smithy Novice_Guide_Temple Novice_Guide_Train
NoviceSaved OldCampAccess Orcs_Desert Org_829_GotJoint RandomDiggerPhrase_1 RandomDiggerPhrase_2
RandomDiggerPhrase_3 RandomDiggerPhrase_4 RandomDiggerPhrase_5 RaykBeated RevealedKalomists
Ricelord_AskedForWater Riordian_GlandNegotiation Rogue01_Warning Rogue03_Warning SC_Walk
Scorpio_Exile SilasFound SilasGuilty Skip_TradeFree Snaf_FreeMBRagout Stone_ImprovedOreArmor
Stone_Teacher SwampCampTemple_Permision Tavern_Permission TeleportToWaterMagesBlockedDone
Templar_Duel TemplerGuardAdvice Thorus_AmuletShown Thorus_MordragMageMessenger
Thorus_Permission_Exterior Thorus_Permission_Interior UNITTEST_EXPECT_CONVERSATION_ENDED
UNITTEST_SELECT_SUBDIALOG_INDEX UrNazkrog_Permission UrNazkrog_Spores UrShak_SpokeOfUluMulu
VLK_584_Snipes_DEAL_2_permanent WaterMaguesPermission WaterMaguesTeleportBlocked Whistler_BuyMySword
''',
  StoryIntegerKind.finiteState: '''
BaalLukor_BringParchment BanditCellVisited Blackmailer_Permission Brannok_Warning
Cavalorn_BestiaryQuestion ChromaninReaded Convoy_CleanupStep Corristo_FireMagesTest
DiggerRankUpOrder GuardOrcLandWarning_OC GuardPassageTavernWarning_NC GuardPassageWarning_NC
GuardPassageWarning_OC GuardPassageWarning_SC GuardPassageWaterMagesWarning_NC Jacko_Fled
Lester_Guide Mud_Nerve Rogue01_Permission Rogue03_Permission Saturas_BringFoci UNITTEST_SUCCESS
Yberion_Ashes
''',
  StoryIntegerKind.counterOrScore: '''
BaalKagan_three Blackmailer_Encounter Blackmailer_Warning Chokta_angry_counter Counter
FindGolemHearts GatheredTemplars Gomez_Contacts GorHanis_Charged Gorn_Ignite Guard_Order
GuardDistraction Kalom_Counter Kharim_Charged Kirgo_Charged Lee_HeroProgression Melvin_Preaching
Milten_Sleeper_Battle Mud_Leave NC_JointsDistributed Novices_Mumbling Novices_Mumbling_Phase01
Novices_Mumbling_Phase02 OC_Test Peasants_have_water Points_NC Points_OC Rayk_Mad RecruitedDiggers
Rogue01_Afraid Rogue03_Afraid
''',
  StoryIntegerKind.calendarDay: 'Whistler_BuyMySword_Day',
  StoryIntegerKind.derivedOrOpaqueInteger:
      'Diego_Notes_DEX Diego_Notes_STR Urshak_name_0',
  StoryIntegerKind.readOnlyInSourceInteger: '''
Convoy_RaidStart CorKalom_BringMCQBalls ExploreSunkenTower FireMagesBook FireMagesDead FMTaken
ForgedRivalries2_DarrionMarkedCraft GorHanis_Challenged GorHanis_Lose hero_attribute_Dexterity
hero_attribute_Strength Huno_LearnSmith Location_OldCamp_Dungeons MiltenAlreadyKnown
PlaceholderCondition Pock_ForgetAll self_aivar_AIV_PASSGATE SilasRemoved Tunnel_Opened UrShak
''',
  StoryIntegerKind.dormantOrLegacyInteger: '''
AIV_GPS_BEGIN AIV_GPS_FIRSTWARN AIV_GPS_LASTWARN Armor armorInstance Bartholo_flags Bartholo_guild
Bullit_guild Cavalorn_FirstTime CorAngar_SendToNC Crw_Armor_H DIA_Grd_216_DustyZoll_permanent
Diego_Follow Dusty_aivar_AIV_PARTYMEMBER Dusty_guild Dusty_LetsGo EncounteredHighPriest FindXardas
Fingers_Wherecavalorn Fortuno_HasYBerionHerbs FP_NC_PATH_41_MILTEN FP_NC_WATER_MILTEN_IN
FP_OC_FREE_STONE FP_OC_NORTHGATE_GUARDPASSAGE FP_OC_RAVEN_END_GUIDE FP_OC_STAIRCASE_TOP_CHAPEL
FP_OC_STANDAROUND_84_MILTEN FP_OW_29_TALAS FP_OW_DIEGO_190 FP_OW_DIEGO_LOCATION_12_01
FP_OW_DIEGO_WHEEL FP_OW_TALAS_BRIDGE FP_SC_MEDITATE_17 FP_SC_MEDITATE_18 FP_SC_START_SWAMPCAMP
FP_ST_MILTEN_FORCED FP_ST_PATH_3_STONES_MILTEN Freemine_GateOpen Gomez_flags Gomez_guild
gorn_aivar_AIV_FINDABLE Gorn_Follow Graham_OMMapBlackmailed GRD_200_Thorus_ZWEIHAND1_permanent
GRD_200_Thorus_ZWEIHAND2_permanent GRD_205_Scorpio_CROSSBOW2_permanent
GRD_205_Scorpio_CROSSBOW_permanent Grd_260_Drake_Crawler_Okay_permanent
GRD_262_Aaron_BLUFF_permanent Guild Guild_Human_NewCamp_Mercenary Guild_Human_NewCamp_Rogue
Guild_Human_NewCamp_WaterMage Guild_Human_OldCamp_FireMage Guild_Human_OldCamp_Guard
Guild_Human_SwampCamp_Novice Guild_Human_SwampCamp_Templar Guild_None
GUR_1202_CorAngar_WANNABETPL_permanent GUR_1202_CorAngar_ZWEIHAND1_permanent
GUR_1202_CorAngar_ZWEIHAND2_permanent Gur_1208_BaalCadar_KREIS1_permanent
Gur_1208_BaalCadar_KREIS2_permanent Gur_1208_BaalCadar_KREIS4_permanent
hero_aivar_AIV_GUARDPASSAGE_STATUS hero_attribute_MaxMana Hlp_GetInstanceIDarmor InExtremoPlaying
Info_Kalom_KrautboteBACK_permanent KalomDead KDF_402_Corristo_HEAVYARMOR_permanent
KDF_402_Corristo_KREIS1_permanent KDF_402_Corristo_KREIS2_permanent
KDF_402_Corristo_KREIS3_permanent KDF_402_Corristo_KREIS4_permanent
KDF_402_Corristo_WANNBEKDF_permanent KDW_600_Saturas_KREIS1_permanent
KDW_600_Saturas_KREIS2_permanent KDW_600_Saturas_KREIS3_permanent KDW_600_Saturas_KREIS4_permanent
KDW_600_Saturas_KREIS5_permanent Knows_GetClaws Knows_GetFur Knows_GetHide Knows_GetMCMandibles
Knows_GetTeeth Knows_GetUluMulu Lares_CheatedIntoHut Lefty_Dead Lester_Follow LoadSword LOG_OBSOLETE
LOG_SUCCESS LogThorusfight LogWolftrain Milten_Follow MonasteryRuin_GateOpen Myarmor Novize_1_senses
Novize_senses NPC_FLAG_IMMORTAL Npc_GetEquippedArmorhero Npc_GetTrueGuildhero
Npc_HasItemshero_ItAt_Crawler_01 Npc_HasItemshero_ItMi_Orenugget NpctypeFriend NpctypeMain
Nyras_flags oldHeroGuild Ore Roscoe_aivar_AIV_PASSGATE self_aivar_AIV_GUARDPASSAGE_STATUS
self_aivar_AIV_HAS_ERPRESSED self_aivar_AIV_MISSION1 self_aivar_AIV_PARTYMEMBER Self_flags
self_npcType SENSE_SEESENSE_HEARSENSE_SMELL Skip_guild Sld_700_Lee_ZWEIHAND1_permanent
Sld_700_Lee_ZWEIHAND2_permanent SLD_709_Cord_TRAIN_permanent SLD_709_Cord_TRAINAGAIN_permanent
SLD_753_Baloro_SC_besorgt_den_Kram StartChaptersSix Stone_guild StoneHenge_Inside StoneHenge_Outside
StoneHengeSkeletonsDead Tarrok Tarrok_name_0 Templer_1_senses Templer_senses Thorus_flags
TPL_1402_GorNaToth_TRAIN_permanent TPL_1402_GorNaToth_TRAINAGAIN_permanent
Tpl_1415_Templer_ROCK_permanent Tpl_1438_Templer_TEACHZANGEN_permanent Troll_Wheel
TrollCanyon_Inside TrollCanyon_Outside UluFight URSHAK_FRIEND VALUE_NOV_ARMOR_H VALUE_STT_ARMOR_H
VLK_585_Aleph_DIRTY_permanent VLK_585_Aleph_SCHUPPEN_permanent wache218_aivar_AIV_PASSGATE
Warned_Gorn_or_Lester
''',
};

// Source writes/comparisons and values observed in the researched saves. These
// are hints only; they are intentionally not validation constraints.
const Map<String, List<int>> _knownValuesById = {
  'BaalKagan_three': [3],
  'BaalLukor_BringParchment': [0, 1, 2, 4],
  'BanditCellVisited': [0, 1, 2],
  'Blackmailer_Encounter': [0, 1],
  'Blackmailer_Permission': [0, 1, 2, 3],
  'Blackmailer_Warning': [0, 1, 2],
  'Brannok_Warning': [0, 1, 2],
  'Cavalorn_BestiaryQuestion': [0, 1, 2, 3],
  'Chokta_angry_counter': [2, 3],
  'ChromaninReaded': [1, 2, 3, 4, 5, 6],
  'Convoy_CleanupStep': [1, 2],
  'Convoy_RaidStart': [0, 1],
  'CorKalom_BringMCQBalls': [0],
  'Corristo_FireMagesTest': [1, 4, 5, 6],
  'Counter': [0, 4],
  'DiggerRankUpOrder': [1, 2, 3, 4, 5],
  'ExploreSunkenTower': [0],
  'FindGolemHearts': [1, 2, 3, 4],
  'FireMagesDead': [0],
  'FMTaken': [0],
  'GatheredTemplars': [3],
  'Gomez_Contacts': [0, 3, 4],
  'GorHanis_Challenged': [0],
  'GorHanis_Charged': [0, 1],
  'GorHanis_Lose': [0],
  'Gorn_Ignite': [0, 1, 2],
  'Guard_Order': [0, 1],
  'GuardDistraction': [1, 3],
  'GuardOrcLandWarning_OC': [0, 1, 2],
  'GuardPassageTavernWarning_NC': [0, 1, 2],
  'GuardPassageWarning_NC': [0, 1, 2],
  'GuardPassageWarning_OC': [0, 1, 2],
  'GuardPassageWarning_SC': [0, 1, 2],
  'GuardPassageWaterMagesWarning_NC': [0, 1, 2],
  'Jacko_Fled': [0, 1, 2],
  'Kalom_Counter': [2, 5],
  'Kharim_Charged': [0, 1],
  'Kirgo_Charged': [0, 1],
  'Lee_HeroProgression': [0, 1, 2],
  'Lester_Guide': [0, 1, 2, 3, 4],
  'Melvin_Preaching': [0, 1],
  'Milten_Sleeper_Battle': [0, 1, 2],
  'MiltenAlreadyKnown': [0],
  'Mud_Leave': [0, 1, 2, 3, 4, 5],
  'Mud_Nerve': [
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
  ],
  'NC_JointsDistributed': [10, 22],
  'Novices_Mumbling': [0, 1, 2, 3],
  'Novices_Mumbling_Phase01': [0, 1, 2, 3],
  'Novices_Mumbling_Phase02': [0, 1, 2, 3],
  'OC_Test': [3, 5, 8],
  'Peasants_have_water': [0, 2, 5],
  'PlaceholderCondition': [0, 1, 5, 6, 8],
  'Points_NC': [10, 30, 35, 45, 60],
  'Points_OC': [0, 10, 15, 17, 27],
  'Rayk_Mad': [0, 1],
  'RecruitedDiggers': [3],
  'Rogue01_Afraid': [0, 1, 2],
  'Rogue01_Permission': [0, 1, 2],
  'Rogue03_Afraid': [0, 1, 2],
  'Rogue03_Permission': [0, 1, 2],
  'Saturas_BringFoci': [0, 1, 2, 3, 4, 5],
  'self_aivar_AIV_PASSGATE': [0],
  'Tunnel_Opened': [0, 1],
  'UNITTEST_SUCCESS': [1, 2, 3],
  'Whistler_BuyMySword_Day': [1, 2],
  'Yberion_Ashes': [0, 1, 2],
};
