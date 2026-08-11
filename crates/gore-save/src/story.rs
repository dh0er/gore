//! Projection and guarded writer for Gothic's persisted story-property map.
//!
//! The save does not serialize one tagged property per Angelscript field.
//! `FSingleStorySaveGameData::StoryPropertyValues` is a
//! `TMap<FName, int32>`, so every stored value has the same wire type.  The
//! semantic distinction below comes from the persisted `UPROPERTY()` schema
//! of `UStoryG1R` in the shipped `PrecompiledScript_Shipping.Cache`: 419
//! fields are declared as `int` and exactly 50 as `FInGameTime`. `Chapter` is
//! inherited from `UGameStory`. No id spelling or suffix heuristic is used.
//! The decompiled `StoryG1R.as` schema input used for this catalog has SHA-256
//! `c5e9fc15e876c21d414da6b3b2c26b5627d8e17b940bfa1b3f7d4225d4d1e07c`.

use crate::CoreError;
use crate::properties::{self, Property, PropertyValue, RootObject, StructValue};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

const STORY_CLASS: &str = "/Script/Angelscript.StoryG1R";
const DIRECT_PROPERTY_COUNT: usize = 469;
const CATALOG_PROPERTY_COUNT: usize = 470;
const INTEGER_PROPERTY_COUNT: usize = 419;

/// Compare-and-swap state supplied by the UI. Requiring the state observed
/// when the row was loaded prevents a delayed edit from overwriting a value
/// that changed after a refresh (or re-creating a value that was removed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoryExpectedValue {
    pub stored: bool,
    pub raw_value: Option<i32>,
}

/// One value-addressed mutation inside an atomic `private.story.apply` edit.
///
/// `present = false` means remove/reset the sparse map entry. `raw_value` is
/// required exactly when `present` is true. Unknown ids may always be edited
/// or removed when already stored; creating a new unknown id additionally
/// requires `allow_unknown_create`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoryChange {
    pub id: String,
    pub present: bool,
    pub raw_value: Option<i32>,
    pub expected: StoryExpectedValue,
    pub allow_unknown_create: bool,
}

/// The complete persisted `FInGameTime` field set declared by `UStoryG1R` in
/// the shipping cache. Keep this as an exact schema allow-list: names that
/// merely look time-related must remain ordinary integers.
const TIME_MARKER_PROPERTIES: [&str; 50] = [
    "Ambient_InExtremo",
    "Ambient_Orc",
    "BaalParvez_GotoSC_Day",
    "Balor_CollectDay",
    "BanditLockpick_Daily",
    "Blackmailer_Daily",
    "Blackmailer_Mad",
    "Bloodwyn_PayDay",
    "Bow_Artisan",
    "Convoy_Guard",
    "Convoy_RaidStartTime",
    "Darrion_MakeKey",
    "Darrion_MarkWeapon",
    "DefeatBully",
    "Diego_Welcome",
    "Fletcher_Timer_Night",
    "Fortuno_RationDay",
    "FourFriendsFocus",
    "GenericDiggerCrawlerQueen",
    "GenericNCAdmission",
    "GenericOCAdmission",
    "GenericSCAdmission",
    "Gorn_Timer_Battle",
    "Huno_WaitAfterTraining",
    "Lefty_WorkDay",
    "Lukor_Open_Door",
    "Melvin_Meditating",
    "Milten_Time_Troll",
    "Milten_Timer_Battle",
    "Mordrag_GotoNC_Day",
    "Mud_Path_OrcGraveyard",
    "Mud_Stay_Kalomists",
    "Myxir_Quest_Completed",
    "NovicesSleeper_Mumbling",
    "NovicesSleeper_Mumbling_Phase01",
    "NovicesSleeper_Mumbling_Phase02",
    "Orcs_Desert_Timer",
    "Pock_Forget_Time",
    "Rayk_Mad_BeforeConversation",
    "Rogue01_Robbed",
    "Rogue01_ScaredPool",
    "Rogue03_Robbed",
    "Rogue03_ScaredPool",
    "Scavenger_Preparation",
    "Snaf_RagoutDay",
    "Snaf_SyraRecipe",
    "Stone_OreArmor",
    "Stone_StartMaceCraft",
    "Viran_DeliveryDay",
    "Wolf_Crafting_Armor",
];

/// The complete persisted `int` field set declared directly by `UStoryG1R`.
/// This list was emitted from the same shipping cache as
/// [`TIME_MARKER_PROPERTIES`]; fields in the class without `UPROPERTY()` are
/// deliberately excluded because the story save system does not persist them.
const INTEGER_PROPERTIES: [&str; INTEGER_PROPERTY_COUNT] = [
    "AfterCinematic_Nyras",
    "AfterCinematic_Sleeper",
    "AIV_GPS_BEGIN",
    "AIV_GPS_FIRSTWARN",
    "AIV_GPS_LASTWARN",
    "Armor",
    "armorInstance",
    "BaalCadar_responsive",
    "BaalCadar_Sacrilege",
    "BaalIsidro_GotDrink",
    "BaalKagan_three",
    "BaalLukor_BringParchment",
    "BaalLukor_KeyPart01",
    "BaalLukor_KeyPart02",
    "BaalNamib_responsive",
    "BaalNamib_Sacrilege",
    "BaalOrun_responsive",
    "BaalOrun_Sacrilege",
    "BaalTyon_responsive",
    "BaalTyon_Sacrilege",
    "Balor_PlayerCheating",
    "Balor_TellsNCDealer",
    "Baloro_SC_choice",
    "Baloro_SC_wantsToKnow",
    "BanditCellVisited",
    "Bartholo_flags",
    "Bartholo_guild",
    "Blackmailer_Encounter",
    "Blackmailer_Permission",
    "Blackmailer_Warning",
    "Bloodwyn_ProtectionPaid",
    "Bouncer876_GotJoint",
    "Brannok_Permission",
    "Brannok_Warning",
    "BranNoteRead",
    "BridgeGolemCombatActive",
    "BridgeStoneGolemEnemy",
    "Bullit_guild",
    "BullitDefeated",
    "CaineVanished",
    "CanUpgradeGuardArmor",
    "CanUpgradeNoviceArmor",
    "Cavalorn_BestiaryDiscovered",
    "Cavalorn_BestiaryQuestion",
    "Cavalorn_BestiaryQuestionRunning",
    "Cavalorn_FirstTime",
    "Chokta_angry",
    "Chokta_angry_counter",
    "ChromaninReaded",
    "Cipher_Trade",
    "CollapseMineNotify",
    "ConversationWithDiegoAtStoneHenge",
    "Convoy_CleanupStep",
    "Convoy_PlayerBriefed",
    "Convoy_RaidStart",
    "CorAngar_GotoOGY",
    "CorAngar_SendToNC",
    "CorKalom_BringMCQBalls",
    "Corristo_FireMagesTest",
    "Corristo_FirsTalk",
    "Counter",
    "Crw_Armor_H",
    "Damarok_GlandNegotiation",
    "Darrion_Teacher",
    "Dexter_SC",
    "Dexter_Traded",
    "DIA_Grd_216_DustyZoll_permanent",
    "Diego_After_Gamestart",
    "Diego_Follow",
    "Diego_GomezAudience",
    "Diego_Notes_DEX",
    "Diego_Notes_STR",
    "DiggerRankUpOrder",
    "Drax_CanTeach",
    "Drax_GotBeer",
    "Dusty_aivar_AIV_PARTYMEMBER",
    "Dusty_flags",
    "Dusty_guild",
    "Dusty_LetsGo",
    "EncounteredHighPriest",
    "EnteredFreeMine",
    "ExploreSunkenTower",
    "FindGolemHearts",
    "FindXardas",
    "Fingers_CanTeach",
    "Fingers_Wherecavalorn",
    "FireMagesBook",
    "FireMagesDead",
    "FireMagesPermission",
    "Fisk_ForgetSword",
    "Fisk_SellSword110",
    "Fisk_SwordSold",
    "Fletcher_foundNek",
    "Fletcher_whytalk",
    "FMHostile",
    "FMTaken",
    "ForgedRivalries2_DarrionAngry",
    "ForgedRivalries2_DarrionBanTrade",
    "ForgedRivalries2_DarrionMarkedCraft",
    "ForgedRivalries2_StoneKnowsDarrionPlanFull",
    "ForgedRivalries2_StoneKnowsDarrionPlanPartial",
    "ForgedRivalries2_StoneKnowsDesignIsGomez",
    "ForgedRivalries2_StoneMarkedCraft",
    "ForgedRivalries3_DarrionWillAcceptStone",
    "ForgedRivalries3_StoneKnowsDarrionAccept",
    "ForgedRivalries3_StoneKnowsDarrionReject",
    "ForgedRivalries3_StoneRefreshed",
    "ForgedRivalries_DarrionCraftDishonest",
    "ForgedRivalries_DarrionCraftHonest",
    "ForgedRivalries_DeliverStoneCraft",
    "ForgedRivalries_OwnCraftDishonest",
    "ForgedRivalries_StoneCraft",
    "Fortress_Inside",
    "Fortress_Outside",
    "Fortuno_HasYBerionHerbs",
    "FP_NC_PATH_41_MILTEN",
    "FP_NC_WATER_MILTEN_IN",
    "FP_OC_FREE_STONE",
    "FP_OC_NORTHGATE_GUARDPASSAGE",
    "FP_OC_RAVEN_END_GUIDE",
    "FP_OC_STAIRCASE_TOP_CHAPEL",
    "FP_OC_STANDAROUND_84_MILTEN",
    "FP_OW_29_TALAS",
    "FP_OW_DIEGO_190",
    "FP_OW_DIEGO_LOCATION_12_01",
    "FP_OW_DIEGO_WHEEL",
    "FP_OW_TALAS_BRIDGE",
    "FP_SC_MEDITATE_17",
    "FP_SC_MEDITATE_18",
    "FP_SC_START_SWAMPCAMP",
    "FP_ST_MILTEN_FORCED",
    "FP_ST_PATH_3_STONES_MILTEN",
    "Freemine_GateOpen",
    "Freemine_Recovered",
    "FriendOfUrShak",
    "Friends_SendToNC",
    "GatheredTemplars",
    "Gomez_Contacts",
    "Gomez_flags",
    "Gomez_guild",
    "GorHanis_Challenged",
    "GorHanis_Charged",
    "GorHanis_Lose",
    "GorHanis_Win",
    "gorn_aivar_AIV_FINDABLE",
    "Gorn_AloneForFM",
    "Gorn_Follow",
    "Gorn_GotoWolf",
    "Gorn_Ignite",
    "Gorn_JoinedForFM",
    "Graham_OMMapBlackmailed",
    "Graham_OMMapSold",
    "GRD_200_Thorus_ZWEIHAND1_permanent",
    "GRD_200_Thorus_ZWEIHAND2_permanent",
    "GRD_205_Scorpio_CROSSBOW2_permanent",
    "GRD_205_Scorpio_CROSSBOW_permanent",
    "Grd_260_Drake_Crawler_Okay_permanent",
    "GRD_262_Aaron_BLUFF_permanent",
    "Grim_ProtectionBully",
    "Grim_Tests",
    "Guard_Order",
    "Guard_Permission_Orc_Land",
    "GuardDistraction",
    "GuardOrcLandWarning_OC",
    "GuardPassageTavernWarning_NC",
    "GuardPassageWarning_NC",
    "GuardPassageWarning_OC",
    "GuardPassageWarning_SC",
    "GuardPassageWaterMagesWarning_NC",
    "Guild",
    "Guild_Human_NewCamp_Mercenary",
    "Guild_Human_NewCamp_Rogue",
    "Guild_Human_NewCamp_WaterMage",
    "Guild_Human_OldCamp_FireMage",
    "Guild_Human_OldCamp_Guard",
    "Guild_Human_SwampCamp_Novice",
    "Guild_Human_SwampCamp_Templar",
    "Guild_None",
    "GUR_1202_CorAngar_WANNABETPL_permanent",
    "GUR_1202_CorAngar_ZWEIHAND1_permanent",
    "GUR_1202_CorAngar_ZWEIHAND2_permanent",
    "Gur_1208_BaalCadar_KREIS1_permanent",
    "Gur_1208_BaalCadar_KREIS2_permanent",
    "Gur_1208_BaalCadar_KREIS4_permanent",
    "Haenno_Bow_Knowledge",
    "HappyFriends",
    "HasULUMULU",
    "Herek_ProtectionBully",
    "hero_aivar_AIV_GUARDPASSAGE_STATUS",
    "hero_attribute_Dexterity",
    "hero_attribute_MaxMana",
    "hero_attribute_Strength",
    "HeroInsideBanditCell",
    "HeroInsideThroneRoom",
    "Hlp_GetInstanceIDarmor",
    "Huno_LearnSmith",
    "IlegalWeedMixer_Permision",
    "InExtremoPlaying",
    "Info_Bartholo_Krautbote_permanent",
    "Info_Kalom_KrautboteBACK_permanent",
    "Info_Xardas_LOADSWORD09_permanent",
    "IntroInExtremo",
    "Jackal_ProtectionPaid",
    "Jacko_Fled",
    "JackoNoteRead",
    "Jan_Training",
    "Jeremiah_Brewer",
    "Joru_JoinSC",
    "Joru_Tips",
    "Joru_Tips_Mage",
    "Kalom_Counter",
    "Kalom_DeliveredWeed",
    "Kalom_TalkedTo",
    "KalomDead",
    "KDF_402_Corristo_HEAVYARMOR_permanent",
    "KDF_402_Corristo_KREIS1_permanent",
    "KDF_402_Corristo_KREIS2_permanent",
    "KDF_402_Corristo_KREIS3_permanent",
    "KDF_402_Corristo_KREIS4_permanent",
    "KDF_402_Corristo_WANNBEKDF_permanent",
    "KDW_600_Saturas_HEAVYARMOR_permanent",
    "KDW_600_Saturas_KREIS1_permanent",
    "KDW_600_Saturas_KREIS2_permanent",
    "KDW_600_Saturas_KREIS3_permanent",
    "KDW_600_Saturas_KREIS4_permanent",
    "KDW_600_Saturas_KREIS5_permanent",
    "Kharim_Challenged",
    "Kharim_Charged",
    "Kharim_Lose",
    "Kharim_Win",
    "Kirgo_Challenged",
    "Kirgo_Charged",
    "Kirgo_Lose",
    "Kirgo_Win",
    "Knows_GetClaws",
    "Knows_GetFur",
    "Knows_GetHide",
    "Knows_GetMCMandibles",
    "Knows_GetMCPlates",
    "Knows_GetTeeth",
    "Knows_GetUluMulu",
    "KnowStone",
    "Lares_CheatedIntoHut",
    "Lares_Permission",
    "Lee_freeminereport",
    "Lee_HeroProgression",
    "Lee_SldPossible",
    "Lefty_CarriedWater",
    "Lefty_Dead",
    "Lefty_WasBeaten",
    "Lester_Follow",
    "Lester_Guide",
    "Lester_Show",
    "LoadSword",
    "Location_AbandonedMine_AfterAmulet",
    "Location_AbandonedMine_OrcGrave",
    "Location_NewCamp_OrePile",
    "Location_OldCamp_Dungeons",
    "Location_OldMineCollapsed",
    "Location_OrcEnclave_Arena",
    "Location_SwampCamp_Temple",
    "Location_XardasTower_Bedroom",
    "LOG_OBSOLETE",
    "LOG_SUCCESS",
    "LogBaalcadarsell",
    "LogBaalcadartrain",
    "LogCavalorntrain",
    "LogDiegotrain",
    "LogGornatothfight",
    "LogGornatothtrain",
    "LogScattytrain",
    "LogScorpiocrossbow",
    "LogThorusfight",
    "LogThorustrain",
    "LogWedgelearn",
    "LogWolftrain",
    "Magician_Level",
    "MCPlatesDelivered",
    "Melvin_Preaching",
    "Milten_Follow",
    "Milten_HasLetter",
    "Milten_Sleeper_Battle",
    "MiltenAlreadyKnown",
    "Monastery_Inside",
    "Monastery_Outside",
    "MonasteryRuin_GateOpen",
    "Mordrag_Traded",
    "MordragKO_Exiled",
    "MordragKO_PlayerChoseOreBarons",
    "MordragKO_PlayerChoseThorus",
    "MordragKO_StayAtNC",
    "Mud_Follow",
    "Mud_Leave",
    "Mud_Nerve",
    "Mud_NerveRealized",
    "Mud_OrcGraveyard",
    "Myarmor",
    "NC_JointsDistributed",
    "Novice_Guide_Kalom",
    "Novice_Guide_MainGate",
    "Novice_Guide_Smithy",
    "Novice_Guide_Temple",
    "Novice_Guide_Train",
    "Novices_Mumbling",
    "Novices_Mumbling_Phase01",
    "Novices_Mumbling_Phase02",
    "NoviceSaved",
    "Novize_1_senses",
    "Novize_senses",
    "NPC_FLAG_IMMORTAL",
    "Npc_GetEquippedArmorhero",
    "Npc_GetTrueGuildhero",
    "Npc_HasItemshero_ItAt_Crawler_01",
    "Npc_HasItemshero_ItMi_Orenugget",
    "NpctypeFriend",
    "NpctypeMain",
    "Nyras_flags",
    "OC_Test",
    "OldCampAccess",
    "oldHeroGuild",
    "Orcs_Desert",
    "Ore",
    "Org_829_GotJoint",
    "Peasants_have_water",
    "PlaceholderCondition",
    "Pock_ForgetAll",
    "Points_NC",
    "Points_OC",
    "RandomDiggerPhrase_1",
    "RandomDiggerPhrase_2",
    "RandomDiggerPhrase_3",
    "RandomDiggerPhrase_4",
    "RandomDiggerPhrase_5",
    "Rayk_Mad",
    "RaykBeated",
    "RecruitedDiggers",
    "RevealedKalomists",
    "Ricelord_AskedForWater",
    "Riordian_GlandNegotiation",
    "Rogue01_Afraid",
    "Rogue01_Permission",
    "Rogue01_Warning",
    "Rogue03_Afraid",
    "Rogue03_Permission",
    "Rogue03_Warning",
    "Roscoe_aivar_AIV_PASSGATE",
    "Saturas_BringFoci",
    "SC_Walk",
    "Scorpio_Exile",
    "self_aivar_AIV_GUARDPASSAGE_STATUS",
    "self_aivar_AIV_HAS_ERPRESSED",
    "self_aivar_AIV_MISSION1",
    "self_aivar_AIV_PARTYMEMBER",
    "self_aivar_AIV_PASSGATE",
    "Self_flags",
    "self_npcType",
    "SENSE_SEESENSE_HEARSENSE_SMELL",
    "SilasFound",
    "SilasGuilty",
    "SilasRemoved",
    "Skip_guild",
    "Skip_TradeFree",
    "Sld_700_Lee_ZWEIHAND1_permanent",
    "Sld_700_Lee_ZWEIHAND2_permanent",
    "SLD_709_Cord_TRAIN_permanent",
    "SLD_709_Cord_TRAINAGAIN_permanent",
    "SLD_753_Baloro_SC_besorgt_den_Kram",
    "Snaf_FreeMBRagout",
    "StartChaptersSix",
    "Stone_guild",
    "Stone_ImprovedOreArmor",
    "Stone_Teacher",
    "StoneHenge_Inside",
    "StoneHenge_Outside",
    "StoneHengeSkeletonsDead",
    "SwampCampTemple_Permision",
    "Tarrok",
    "Tarrok_name_0",
    "Tavern_Permission",
    "TeleportToWaterMagesBlockedDone",
    "Templar_Duel",
    "Templer_1_senses",
    "Templer_senses",
    "TemplerGuardAdvice",
    "Thorus_AmuletShown",
    "Thorus_flags",
    "Thorus_MordragMageMessenger",
    "Thorus_Permission_Exterior",
    "Thorus_Permission_Interior",
    "TPL_1402_GorNaToth_TRAIN_permanent",
    "TPL_1402_GorNaToth_TRAINAGAIN_permanent",
    "Tpl_1415_Templer_ROCK_permanent",
    "Tpl_1438_Templer_TEACHZANGEN_permanent",
    "Troll_Wheel",
    "TrollCanyon_Inside",
    "TrollCanyon_Outside",
    "Tunnel_Opened",
    "UluFight",
    "UNITTEST_EXPECT_CONVERSATION_ENDED",
    "UNITTEST_SELECT_SUBDIALOG_INDEX",
    "UNITTEST_SUCCESS",
    "UrNazkrog_Permission",
    "UrNazkrog_Spores",
    "UrShak",
    "URSHAK_FRIEND",
    "Urshak_name_0",
    "UrShak_SpokeOfUluMulu",
    "VALUE_NOV_ARMOR_H",
    "VALUE_STT_ARMOR_H",
    "VLK_584_Snipes_DEAL_2_permanent",
    "VLK_585_Aleph_DIRTY_permanent",
    "VLK_585_Aleph_SCHUPPEN_permanent",
    "wache218_aivar_AIV_PASSGATE",
    "Warned_Gorn_or_Lester",
    "WaterMaguesPermission",
    "WaterMaguesTeleportBlocked",
    "Whistler_BuyMySword",
    "Whistler_BuyMySword_Day",
    "Yberion_Ashes",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SemanticType {
    TimeMarker,
    Chapter,
    Integer,
    Unknown,
}

impl SemanticType {
    fn as_str(self) -> &'static str {
        match self {
            Self::TimeMarker => "timeMarker",
            Self::Chapter => "chapter",
            Self::Integer => "integer",
            Self::Unknown => "unknown",
        }
    }

    fn declared_type(self) -> &'static str {
        match self {
            Self::TimeMarker => "FInGameTime",
            Self::Chapter | Self::Integer => "int",
            // The map's wire value is still int32, but an id absent from the
            // shipped persisted-property catalog has no source declaration we
            // can honestly report.
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug)]
struct StoryEntry {
    id: String,
    raw_value: Option<i32>,
    semantic_type: SemanticType,
    stored: bool,
    catalog_known: bool,
}

impl StoryEntry {
    fn path(&self) -> Vec<String> {
        if !self.stored {
            return Vec::new();
        }
        vec![
            "m_GenericData".to_string(),
            "{Story}".to_string(),
            "SaveDataByStoryClass".to_string(),
            format!("{{{STORY_CLASS}}}"),
            "StoryPropertyValues".to_string(),
            format!("{{{}}}", self.id),
        ]
    }

    fn matches(&self, terms: &[String]) -> bool {
        if terms.is_empty() {
            return true;
        }
        let raw_value = self
            .raw_value
            .map(|value| value.to_string())
            .unwrap_or_default();
        let haystack = format!(
            "{} {} {} {} {} {} {}",
            self.id,
            raw_value,
            self.semantic_type.as_str(),
            self.semantic_type.declared_type(),
            self.path().join("/"),
            if self.stored { "stored" } else { "unset" },
            if self.catalog_known {
                "catalog"
            } else {
                "unknown"
            },
        )
        .to_ascii_lowercase();
        terms.iter().all(|term| haystack.contains(term))
    }

    fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "rawValue": self.raw_value,
            "path": self.path(),
            "semanticType": self.semantic_type.as_str(),
            "declaredType": self.semantic_type.declared_type(),
            "stored": self.stored,
            "catalogKnown": self.catalog_known,
        })
    }
}

/// Query the save-backed StoryG1R map and, when requested, its authoritative
/// persisted field catalog. Stored map-entry paths are useful provenance, but
/// they are not tagged scalar addresses accepted by the generic typed writer;
/// the dedicated value-addressed `private.story.apply` operation owns writes.
/// Unset catalog entries therefore expose an empty path rather than pretending
/// that a generic writable address already exists.
pub fn query_story(
    root: &RootObject,
    query: &str,
    semantic_type: Option<&str>,
    offset: usize,
    limit: usize,
    include_unset: bool,
) -> Result<Value, CoreError> {
    let story_values = story_property_values(root)?;
    let mut entries = Vec::<StoryEntry>::with_capacity(if include_unset {
        story_values.len() + CATALOG_PROPERTY_COUNT
    } else {
        story_values.len()
    });
    let mut stored_ids = std::collections::HashSet::<String>::with_capacity(story_values.len());
    let mut known_stored_ids =
        std::collections::HashSet::<String>::with_capacity(story_values.len());
    for (key, value) in story_values {
        let id = properties::map_key_to_string(key).ok_or_else(|| {
            CoreError::Parse("StoryPropertyValues contains a non-name key".to_string())
        })?;
        let PropertyValue::Int(raw_value) = value else {
            return Err(CoreError::Parse(format!(
                "StoryPropertyValues[{id:?}] is not an int32"
            )));
        };
        let catalog_type = catalog_semantic_type(&id);
        let normalized_id = id.to_ascii_lowercase();
        stored_ids.insert(normalized_id.clone());
        if catalog_type.is_some() {
            known_stored_ids.insert(normalized_id);
        }
        entries.push(StoryEntry {
            semantic_type: catalog_type.unwrap_or(SemanticType::Unknown),
            id,
            raw_value: Some(*raw_value),
            stored: true,
            catalog_known: catalog_type.is_some(),
        });
    }

    let stored_total = entries.len();
    let unknown_stored_total = entries.iter().filter(|entry| !entry.catalog_known).count();
    let unset_total = CATALOG_PROPERTY_COUNT.saturating_sub(known_stored_ids.len());
    let stored_counts = semantic_counts(entries.iter());

    if include_unset {
        entries.extend(catalog_entries().filter_map(|(id, semantic_type)| {
            if stored_ids.contains(&id.to_ascii_lowercase()) {
                return None;
            }
            Some(StoryEntry {
                id: id.to_string(),
                raw_value: None,
                semantic_type,
                stored: false,
                catalog_known: true,
            })
        }));
    }
    entries.sort_by(|a, b| {
        a.id.to_ascii_lowercase()
            .cmp(&b.id.to_ascii_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    let terms = query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let filtered = entries
        .iter()
        .filter(|entry| {
            semantic_type.is_none_or(|filter| entry.semantic_type.as_str() == filter)
                && entry.matches(&terms)
        })
        .collect::<Vec<_>>();
    let filtered_counts = semantic_counts(filtered.iter().copied());
    let page = filtered
        .iter()
        .skip(offset)
        .take(limit)
        .map(|entry| entry.to_json())
        .collect::<Vec<_>>();

    Ok(json!({
        "section": "story",
        "query": query,
        "semanticType": semantic_type,
        "includeUnset": include_unset,
        "offset": offset,
        "limit": limit,
        "total": filtered.len(),
        "storedTotal": stored_total,
        "catalogTotal": CATALOG_PROPERTY_COUNT,
        "unsetTotal": unset_total,
        "unknownStoredTotal": unknown_stored_total,
        "count": page.len(),
        "entries": page,
        "semanticTypeCounts": filtered_counts,
        "storedSemanticTypeCounts": stored_counts,
        "catalogSemanticTypeCounts": catalog_semantic_counts(),
        "currentGameTimeSeconds": current_game_time_seconds(root),
        "writable": is_writable(root),
        "schema": {
            "storageType": "FSingleStorySaveGameData::StoryPropertyValues",
            "wireType": "TMap<FName,int32>",
            "declaredTypeSource": "UStoryG1R persisted UPROPERTY schema (PrecompiledScript_Shipping.Cache)",
            "persistedPropertyCount": CATALOG_PROPERTY_COUNT,
            "directPropertyCount": DIRECT_PROPERTY_COUNT,
            "catalogPropertyCount": CATALOG_PROPERTY_COUNT,
            "integerPropertyCount": INTEGER_PROPERTY_COUNT,
            "timeMarkerPropertyCount": TIME_MARKER_PROPERTIES.len(),
            "chapterSource": "UGameStory::Chapter (inherited)",
        },
    }))
}

/// True only when the exact StoryG1R path resolves to the wire schema the
/// dedicated writer understands. This is used to feature-gate the advertised
/// operation; a merely parseable private payload is not enough.
pub(crate) fn is_writable(root: &RootObject) -> bool {
    // Include entry-level invariants (FName/int values and no case-folded
    // duplicates), so the UI never advertises an operation guaranteed to fail.
    story_map_snapshot(root).is_ok()
}

/// Apply a complete `private.story.apply` request transactionally.
///
/// All request/CAS checks run against the same original snapshot. Mutations
/// are then made on a scratch buffer, re-parsing after every splice (and after
/// fixed-size patches), with final semantic postconditions for every id. The
/// caller's payload is replaced only after all changes pass.
pub(crate) fn apply_changes(
    payload: &mut Vec<u8>,
    changes: &[StoryChange],
) -> Result<(), CoreError> {
    if changes.is_empty() {
        return Err(CoreError::InvalidRequest(
            "private.story.apply requires at least one change".to_string(),
        ));
    }

    let root = properties::parse_private_root(payload)?;
    let original = story_map_snapshot(&root)?;
    validate_request_and_cas(changes, &original)?;

    let mut patched = payload.clone();
    for change in changes {
        apply_one_change(&mut patched, change)?;
    }

    let final_root = properties::parse_private_root(&patched).map_err(|error| {
        CoreError::Parse(format!(
            "story patch produced an inconsistent payload: {error}"
        ))
    })?;
    let final_snapshot = story_map_snapshot(&final_root)?;
    for change in changes {
        let actual = snapshot_value(&final_snapshot, &change.id)?;
        let wanted = if change.present {
            Some(
                change
                    .raw_value
                    .expect("request validation requires a present raw value"),
            )
        } else {
            None
        };
        if actual != wanted {
            return Err(CoreError::Validation(format!(
                "story value {:?} failed its postcondition: expected {}, found {}",
                change.id,
                display_state(wanted),
                display_state(actual),
            )));
        }
    }

    *payload = patched;
    Ok(())
}

#[derive(Clone, Debug)]
struct StoryMapEntry {
    id: String,
    raw_value: i32,
}

#[derive(Debug)]
struct StoryMapResolution<'a> {
    target: &'a Property,
    enclosing_size_fields: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
enum StoryMapValueShape<'a> {
    Instanced(&'a str),
    Properties,
}

/// Dedicated resolver for the one writable StoryG1R map. The generic typed
/// resolver intentionally selects the first matching property/map key, which
/// is useful for browsing but unsafe for mutation: a corrupt/modified save can
/// contain two identically-addressed branches. Every segment here must resolve
/// exactly once before the capability is advertised or any byte is patched.
fn resolve_unique_story_map(root: &RootObject) -> Result<StoryMapResolution<'_>, CoreError> {
    let generic = unique_story_property(&root.properties, "m_GenericData", "root")?;
    let mut enclosing_size_fields = vec![generic.size_field_offset()];
    let story_value = unique_story_map_value(
        generic,
        "Story",
        "m_GenericData",
        "StrProperty",
        "InstancedStruct",
        "/Script/StructUtils",
        StoryMapValueShape::Instanced("/Script/G1R.StorySaveGameData"),
    )?;
    let story_properties = story_struct_properties(
        story_value,
        "m_GenericData{Story}",
        &mut enclosing_size_fields,
    )?;

    let by_class = unique_story_property(
        story_properties,
        "SaveDataByStoryClass",
        "m_GenericData{Story}",
    )?;
    enclosing_size_fields.push(by_class.size_field_offset());
    let story_g1r = unique_story_map_value(
        by_class,
        STORY_CLASS,
        "SaveDataByStoryClass",
        "ObjectProperty",
        "SingleStorySaveGameData",
        "/Script/G1R",
        StoryMapValueShape::Properties,
    )?;
    let story_g1r_properties = story_struct_properties(
        story_g1r,
        "SaveDataByStoryClass{/Script/Angelscript.StoryG1R}",
        &mut enclosing_size_fields,
    )?;

    let target = unique_story_property(
        story_g1r_properties,
        "StoryPropertyValues",
        "SaveDataByStoryClass{/Script/Angelscript.StoryG1R}",
    )?;
    validate_story_map_property(target)?;
    Ok(StoryMapResolution {
        target,
        enclosing_size_fields,
    })
}

fn unique_story_property<'a>(
    properties: &'a [Property],
    name: &str,
    context: &str,
) -> Result<&'a Property, CoreError> {
    let mut matches = properties
        .iter()
        .filter(|property| fname_eq(&property.name, name));
    let first = matches.next().ok_or_else(|| {
        CoreError::Parse(format!(
            "story path property {name:?} not found in {context}"
        ))
    })?;
    if matches.next().is_some() {
        return Err(CoreError::Validation(format!(
            "ambiguous story path: {context} contains multiple {name:?} properties"
        )));
    }
    Ok(first)
}

fn unique_story_map_value<'a>(
    property: &'a Property,
    key: &str,
    context: &str,
    expected_key_type: &str,
    expected_value_struct: &str,
    expected_value_package: &str,
    expected_value_shape: StoryMapValueShape<'_>,
) -> Result<&'a PropertyValue, CoreError> {
    if property.type_name != "MapProperty" {
        return Err(CoreError::Parse(format!(
            "story path {context} is {}, expected MapProperty",
            property.type_name
        )));
    }
    let PropertyValue::Map { entries, .. } = &property.value else {
        return Err(CoreError::Parse(format!(
            "story path {context} did not parse as a map"
        )));
    };
    let (key_descriptor, value_descriptor) =
        property.descriptor.map.as_deref().ok_or_else(|| {
            CoreError::Parse(format!("story path {context} map descriptor is missing"))
        })?;
    if key_descriptor.type_name != expected_key_type
        || key_descriptor.struct_type.is_some()
        || key_descriptor.enum_type.is_some()
        || value_descriptor.type_name != "StructProperty"
        || !value_descriptor
            .struct_type
            .as_deref()
            .is_some_and(|(name, package)| {
                name == expected_value_struct && package == expected_value_package
            })
        || value_descriptor.enum_type.is_some()
    {
        let actual_struct = value_descriptor
            .struct_type
            .as_deref()
            .map(|(name, package)| format!("{name}@{package}"))
            .unwrap_or_else(|| "<missing>".to_string());
        return Err(CoreError::Parse(format!(
            "story path {context} must be TMap<{expected_key_type},StructProperty({expected_value_struct}@{expected_value_package})>, got TMap<{},{}({actual_struct})>",
            key_descriptor.type_name, value_descriptor.type_name
        )));
    }
    let key_matches = |candidate: &PropertyValue| match (expected_key_type, candidate) {
        ("StrProperty", PropertyValue::Str(candidate)) => candidate == key,
        // Both FName and UObject/class paths are FName-backed identities in
        // Unreal. Case-only variants therefore count as the same key and must
        // be rejected as ambiguous rather than selecting the first spelling.
        ("NameProperty", PropertyValue::Name(candidate))
        | ("ObjectProperty", PropertyValue::Object(candidate)) => fname_eq(candidate, key),
        _ => false,
    };
    let mut matches = entries
        .iter()
        .filter(|(candidate, _)| key_matches(candidate));
    let (_, first) = matches.next().ok_or_else(|| {
        CoreError::Parse(format!("story path map key {key:?} not found in {context}"))
    })?;
    if matches.next().is_some() {
        return Err(CoreError::Validation(format!(
            "ambiguous story path: {context} contains multiple map keys {key:?}"
        )));
    }
    match (expected_value_shape, first) {
        (
            StoryMapValueShape::Instanced(expected_actual_type),
            PropertyValue::Struct(StructValue::Instanced(Some(instance))),
        ) if instance.actual_type == expected_actual_type => {}
        (StoryMapValueShape::Properties, PropertyValue::Struct(StructValue::Properties(_))) => {}
        (StoryMapValueShape::Instanced(expected_actual_type), actual) => {
            return Err(CoreError::Parse(format!(
                "story path {context} map value {key:?} must be populated InstancedStruct {expected_actual_type:?}, got {actual:?}"
            )));
        }
        (StoryMapValueShape::Properties, actual) => {
            return Err(CoreError::Parse(format!(
                "story path {context} map value {key:?} must be a tagged property struct, got {actual:?}"
            )));
        }
    }
    Ok(first)
}

fn story_struct_properties<'a>(
    value: &'a PropertyValue,
    context: &str,
    enclosing_size_fields: &mut Vec<usize>,
) -> Result<&'a [Property], CoreError> {
    match value {
        PropertyValue::Struct(StructValue::Properties(properties)) => Ok(properties),
        PropertyValue::Struct(StructValue::Instanced(Some(instance))) => {
            enclosing_size_fields.push(instance.data_size_offset);
            Ok(&instance.properties)
        }
        _ => Err(CoreError::Parse(format!(
            "story path {context} is not a populated property struct"
        ))),
    }
}

fn story_map_snapshot(root: &RootObject) -> Result<Vec<StoryMapEntry>, CoreError> {
    let resolved = resolve_unique_story_map(root)?;
    let property = resolved.target;
    let PropertyValue::Map { entries, .. } = &property.value else {
        unreachable!("resolve_unique_story_map validated the parsed value")
    };
    let mut snapshot = Vec::with_capacity(entries.len());
    let mut seen = HashMap::<String, String>::with_capacity(entries.len());
    for (key, value) in entries {
        let PropertyValue::Name(id) = key else {
            return Err(CoreError::Parse(
                "StoryPropertyValues contains a non-FName key".to_string(),
            ));
        };
        let PropertyValue::Int(raw_value) = value else {
            return Err(CoreError::Parse(format!(
                "StoryPropertyValues[{id:?}] is not an int32"
            )));
        };
        let folded = fold_fname(id);
        if let Some(previous) = seen.insert(folded, id.clone()) {
            return Err(CoreError::Validation(format!(
                "StoryPropertyValues contains duplicate FName ids {previous:?} and {id:?}"
            )));
        }
        snapshot.push(StoryMapEntry {
            id: id.clone(),
            raw_value: *raw_value,
        });
    }
    Ok(snapshot)
}

fn validate_request_and_cas(
    changes: &[StoryChange],
    original: &[StoryMapEntry],
) -> Result<(), CoreError> {
    let mut requested = HashSet::<String>::with_capacity(changes.len());
    for change in changes {
        validate_change_shape(change)?;
        let folded = fold_fname(&change.id);
        if !requested.insert(folded) {
            return Err(CoreError::InvalidRequest(format!(
                "private.story.apply contains duplicate id {:?} (FName comparison is case-insensitive)",
                change.id
            )));
        }

        let actual = snapshot_value(original, &change.id)?;
        let expected = if change.expected.stored {
            Some(
                change
                    .expected
                    .raw_value
                    .expect("request validation requires a stored expected value"),
            )
        } else {
            None
        };
        if actual != expected {
            return Err(CoreError::Validation(format!(
                "story value {:?} changed since it was loaded: expected {}, found {}",
                change.id,
                display_state(expected),
                display_state(actual),
            )));
        }

        if actual.is_none()
            && change.present
            && catalog_semantic_type(&change.id).is_none()
            && !change.allow_unknown_create
        {
            return Err(CoreError::UnsupportedEdit(format!(
                "cannot create unknown story id {:?} without allowUnknownCreate=true",
                change.id
            )));
        }
    }
    Ok(())
}

fn validate_change_shape(change: &StoryChange) -> Result<(), CoreError> {
    if change.id.is_empty() || change.id.trim() != change.id {
        return Err(CoreError::InvalidRequest(
            "private.story.apply change.id must be non-empty and have no surrounding whitespace"
                .to_string(),
        ));
    }
    if change.id.len() > 1024 || change.id.contains('\0') {
        return Err(CoreError::InvalidRequest(format!(
            "private.story.apply id {:?} is not a valid bounded FString",
            change.id
        )));
    }
    if change.present != change.raw_value.is_some() {
        return Err(CoreError::InvalidRequest(format!(
            "private.story.apply change {:?} must provide rawValue exactly when present=true",
            change.id
        )));
    }
    if change.expected.stored != change.expected.raw_value.is_some() {
        return Err(CoreError::InvalidRequest(format!(
            "private.story.apply change {:?} must provide expected.rawValue exactly when expected.stored=true",
            change.id
        )));
    }
    Ok(())
}

fn apply_one_change(payload: &mut Vec<u8>, change: &StoryChange) -> Result<(), CoreError> {
    // Re-parse for every change. A preceding insert/remove invalidates every
    // stored byte offset in the tree, while the FName id remains stable.
    let root = properties::parse_private_root(payload)?;
    let resolved = resolve_unique_story_map(&root)?;
    let snapshot = story_map_snapshot(&root)?;
    let matching_index = snapshot
        .iter()
        .position(|entry| fname_eq(&entry.id, &change.id));

    match (matching_index, change.present) {
        (Some(index), true) => {
            let new_value = change
                .raw_value
                .expect("request validation requires a present raw value");
            if snapshot[index].raw_value != new_value {
                patch_inline_i32(
                    payload,
                    resolved.target,
                    index,
                    snapshot[index].raw_value,
                    new_value,
                )?;
            }
        }
        (Some(index), false) => {
            let target = resolved.target.clone();
            let enclosing = resolved.enclosing_size_fields.clone();
            properties::patch_container(
                payload,
                &target,
                &enclosing,
                &properties::ContainerEdit::MapRemove { entry_index: index },
            )?;
        }
        (None, true) => {
            let stored_id = catalog_canonical_id(&change.id).unwrap_or(&change.id);
            let mut entry_bytes = properties::encode_fstring_value(stored_id);
            entry_bytes.extend_from_slice(
                &change
                    .raw_value
                    .expect("request validation requires a present raw value")
                    .to_le_bytes(),
            );
            let target = resolved.target.clone();
            let enclosing = resolved.enclosing_size_fields.clone();
            properties::patch_container(
                payload,
                &target,
                &enclosing,
                &properties::ContainerEdit::MapInsert { entry_bytes },
            )?;
        }
        (None, false) => {}
    }

    let reparsed = properties::parse_private_root(payload).map_err(|error| {
        CoreError::Parse(format!(
            "story change {:?} produced an inconsistent payload: {error}",
            change.id
        ))
    })?;
    let after = story_map_snapshot(&reparsed)?;
    let actual = snapshot_value(&after, &change.id)?;
    let wanted = if change.present {
        Some(
            change
                .raw_value
                .expect("request validation requires a present raw value"),
        )
    } else {
        None
    };
    if actual != wanted {
        return Err(CoreError::Validation(format!(
            "story change {:?} failed its immediate postcondition: expected {}, found {}",
            change.id,
            display_state(wanted),
            display_state(actual),
        )));
    }
    Ok(())
}

fn validate_story_map_property(property: &Property) -> Result<(), CoreError> {
    // Reuse the exact descriptor validation without relying on a second path
    // lookup. Building a tiny borrowed-root wrapper would obscure errors, so
    // keep the checks local and explicit here.
    if property.type_name != "MapProperty" {
        return Err(CoreError::Parse(
            "StoryPropertyValues is not a MapProperty".to_string(),
        ));
    }
    let (key, value) =
        property.descriptor.map.as_deref().ok_or_else(|| {
            CoreError::Parse("StoryPropertyValues map descriptor is missing".into())
        })?;
    if key.type_name != "NameProperty"
        || key.struct_type.is_some()
        || key.enum_type.is_some()
        || value.type_name != "IntProperty"
        || value.struct_type.is_some()
        || value.enum_type.is_some()
    {
        return Err(CoreError::Parse(format!(
            "StoryPropertyValues must be TMap<FName,int32>, got TMap<{},{}>",
            key.type_name, value.type_name
        )));
    }
    Ok(())
}

fn patch_inline_i32(
    payload: &mut [u8],
    property: &Property,
    entry_index: usize,
    expected_old: i32,
    new_value: i32,
) -> Result<(), CoreError> {
    let layout = properties::map_layout(payload, property)?;
    let range = layout.entry_ranges.get(entry_index).ok_or_else(|| {
        CoreError::Validation(format!(
            "StoryPropertyValues entry index {entry_index} disappeared before patching"
        ))
    })?;
    let value_offset = range.end.checked_sub(4).ok_or_else(|| {
        CoreError::Parse("StoryPropertyValues entry is shorter than an int32".to_string())
    })?;
    if value_offset < range.start || range.end > payload.len() {
        return Err(CoreError::Parse(
            "StoryPropertyValues int32 range is out of bounds".to_string(),
        ));
    }
    let on_disk = i32::from_le_bytes(
        payload[value_offset..range.end]
            .try_into()
            .expect("validated four-byte range"),
    );
    if on_disk != expected_old {
        return Err(CoreError::Validation(format!(
            "StoryPropertyValues inline int32 differs from parsed value: expected {expected_old}, found {on_disk}"
        )));
    }
    payload[value_offset..range.end].copy_from_slice(&new_value.to_le_bytes());
    Ok(())
}

fn snapshot_value(snapshot: &[StoryMapEntry], id: &str) -> Result<Option<i32>, CoreError> {
    let mut matching = snapshot.iter().filter(|entry| fname_eq(&entry.id, id));
    let value = matching.next().map(|entry| entry.raw_value);
    if matching.next().is_some() {
        return Err(CoreError::Validation(format!(
            "StoryPropertyValues id {id:?} is ambiguous under FName case folding"
        )));
    }
    Ok(value)
}

fn display_state(value: Option<i32>) -> String {
    value.map_or_else(|| "unset".to_string(), |raw| format!("stored({raw})"))
}

fn fold_fname(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn fname_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn catalog_semantic_type(id: &str) -> Option<SemanticType> {
    // FName comparison is case-insensitive; preserve that contract while
    // still requiring an exact schema name (no prefix/suffix heuristics).
    if TIME_MARKER_PROPERTIES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(id))
    {
        Some(SemanticType::TimeMarker)
    } else if id.eq_ignore_ascii_case("Chapter") {
        Some(SemanticType::Chapter)
    } else if INTEGER_PROPERTIES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(id))
    {
        Some(SemanticType::Integer)
    } else {
        None
    }
}

fn catalog_canonical_id(id: &str) -> Option<&'static str> {
    catalog_entries()
        .find(|(known, _)| known.eq_ignore_ascii_case(id))
        .map(|(known, _)| known)
}

fn catalog_entries() -> impl Iterator<Item = (&'static str, SemanticType)> {
    INTEGER_PROPERTIES
        .iter()
        .copied()
        .map(|id| (id, SemanticType::Integer))
        .chain(
            TIME_MARKER_PROPERTIES
                .iter()
                .copied()
                .map(|id| (id, SemanticType::TimeMarker)),
        )
        .chain(std::iter::once(("Chapter", SemanticType::Chapter)))
}

fn catalog_semantic_counts() -> Value {
    json!({
        "timeMarker": TIME_MARKER_PROPERTIES.len(),
        "chapter": 1,
        "integer": INTEGER_PROPERTIES.len(),
        "unknown": 0,
    })
}

fn semantic_counts<'a>(entries: impl Iterator<Item = &'a StoryEntry>) -> Value {
    let mut time_markers = 0usize;
    let mut chapters = 0usize;
    let mut integers = 0usize;
    let mut unknown = 0usize;
    for entry in entries {
        match entry.semantic_type {
            SemanticType::TimeMarker => time_markers += 1,
            SemanticType::Chapter => chapters += 1,
            SemanticType::Integer => integers += 1,
            SemanticType::Unknown => unknown += 1,
        }
    }
    json!({
        "timeMarker": time_markers,
        "chapter": chapters,
        "integer": integers,
        "unknown": unknown,
    })
}

fn story_property_values(
    root: &RootObject,
) -> Result<&[(PropertyValue, PropertyValue)], CoreError> {
    let generic = named_property(&root.properties, "m_GenericData")?;
    let story = named_map_value(&generic.value, "Story", "m_GenericData")?;
    let story_props = property_list(story, "m_GenericData{Story}")?;
    let by_class = named_property(story_props, "SaveDataByStoryClass")?;
    let story_g1r = named_map_value(&by_class.value, STORY_CLASS, "SaveDataByStoryClass")?;
    let story_g1r_props = property_list(story_g1r, "SaveDataByStoryClass{StoryG1R}")?;
    let values = named_property(story_g1r_props, "StoryPropertyValues")?;
    match &values.value {
        PropertyValue::Map { entries, .. } => Ok(entries),
        _ => Err(CoreError::Parse(
            "StoryPropertyValues is not a TMap<FName,int32>".to_string(),
        )),
    }
}

fn current_game_time_seconds(root: &RootObject) -> Option<f64> {
    let generic = root.properties.iter().find(|p| p.name == "m_GenericData")?;
    let game_time = optional_named_map_value(&generic.value, "GameTime")?;
    let game_time_props = optional_property_list(game_time)?;
    let current_time = game_time_props.iter().find(|p| p.name == "CurrentTime")?;
    let current_time_props = optional_property_list(&current_time.value)?;
    let total_seconds = current_time_props
        .iter()
        .find(|p| p.name == "TotalSeconds")?;
    match total_seconds.value {
        PropertyValue::Double(value) => Some(value),
        PropertyValue::Float(value) => Some(value as f64),
        PropertyValue::Int(value) => Some(value as f64),
        PropertyValue::Int64(value) => Some(value as f64),
        _ => None,
    }
}

fn named_property<'a>(properties: &'a [Property], name: &str) -> Result<&'a Property, CoreError> {
    properties
        .iter()
        .find(|property| property.name == name)
        .ok_or_else(|| CoreError::Parse(format!("story path property {name:?} not found")))
}

fn named_map_value<'a>(
    value: &'a PropertyValue,
    key: &str,
    context: &str,
) -> Result<&'a PropertyValue, CoreError> {
    optional_named_map_value(value, key).ok_or_else(|| {
        CoreError::Parse(format!("story path map key {key:?} not found in {context}"))
    })
}

fn optional_named_map_value<'a>(value: &'a PropertyValue, key: &str) -> Option<&'a PropertyValue> {
    let PropertyValue::Map { entries, .. } = value else {
        return None;
    };
    entries
        .iter()
        .find(|(candidate, _)| properties::map_key_to_string(candidate).as_deref() == Some(key))
        .map(|(_, value)| value)
}

fn property_list<'a>(value: &'a PropertyValue, context: &str) -> Result<&'a [Property], CoreError> {
    optional_property_list(value)
        .ok_or_else(|| CoreError::Parse(format!("{context} is not a populated property struct")))
}

fn optional_property_list(value: &PropertyValue) -> Option<&[Property]> {
    match value {
        PropertyValue::Struct(StructValue::Properties(properties)) => Some(properties),
        PropertyValue::Struct(StructValue::Instanced(Some(instance))) => Some(&instance.properties),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::{Descriptor, Property};

    fn property(name: &str, type_name: &str, value: PropertyValue) -> Property {
        Property {
            name: name.into(),
            type_name: type_name.into(),
            descriptor: Descriptor::default(),
            array_index: 0,
            tag_flags: 0,
            value_offset: 5,
            value_size: 0,
            value,
        }
    }

    fn props(properties: Vec<Property>) -> PropertyValue {
        PropertyValue::Struct(StructValue::Properties(properties))
    }

    fn story_root() -> RootObject {
        let values = property(
            "StoryPropertyValues",
            "MapProperty",
            PropertyValue::Map {
                num_to_remove: 0,
                entries: vec![
                    (
                        PropertyValue::Name("Stone_OreArmor".to_string()),
                        PropertyValue::Int(1_767_047),
                    ),
                    (
                        PropertyValue::Name("Chapter".to_string()),
                        PropertyValue::Int(2),
                    ),
                    (
                        PropertyValue::Name("Unknown_Timer_Name".to_string()),
                        PropertyValue::Int(17),
                    ),
                ],
            },
        );
        let by_class = property(
            "SaveDataByStoryClass",
            "MapProperty",
            PropertyValue::Map {
                num_to_remove: 0,
                entries: vec![(
                    PropertyValue::Object(STORY_CLASS.to_string()),
                    props(vec![values]),
                )],
            },
        );
        let current_time = property(
            "CurrentTime",
            "StructProperty",
            props(vec![property(
                "TotalSeconds",
                "DoubleProperty",
                PropertyValue::Double(1_875_587.943_7),
            )]),
        );
        let generic = property(
            "m_GenericData",
            "MapProperty",
            PropertyValue::Map {
                num_to_remove: 0,
                entries: vec![
                    (
                        PropertyValue::Name("Story".to_string()),
                        props(vec![by_class]),
                    ),
                    (
                        PropertyValue::Name("GameTime".to_string()),
                        props(vec![current_time]),
                    ),
                ],
            },
        );
        RootObject {
            class: "/Script/Test.Save".to_string(),
            flag: 0,
            properties: vec![generic],
            footer: 0,
            consumed: 0,
        }
    }

    fn fstring(value: &str) -> Vec<u8> {
        properties::encode_fstring_value(value)
    }

    fn name_int_map_with_key_type(key_type: &str, entries: &[(&str, i32)]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value) in entries {
            body.extend_from_slice(&fstring(key));
            body.extend_from_slice(&value.to_le_bytes());
        }
        let mut out = fstring("StoryPropertyValues");
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&fstring(key_type));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&fstring("IntProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&body);
        out
    }

    fn struct_map(
        name: &str,
        key_type: &str,
        struct_type: &str,
        entries: &[(&str, Vec<u8>)],
    ) -> Vec<u8> {
        struct_map_with_package(name, key_type, struct_type, "/Script/G1R", entries)
    }

    fn struct_map_with_package(
        name: &str,
        key_type: &str,
        struct_type: &str,
        package: &str,
        entries: &[(&str, Vec<u8>)],
    ) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, properties) in entries {
            body.extend_from_slice(&fstring(key));
            body.extend_from_slice(properties);
            body.extend_from_slice(&fstring("None"));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&fstring(key_type));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(package));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&body);
        out
    }

    fn instanced_struct_map(
        name: &str,
        key_type: &str,
        descriptor_struct: &str,
        descriptor_package: &str,
        actual_type: &str,
        entries: &[(&str, Vec<u8>)],
    ) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, properties) in entries {
            body.extend_from_slice(&fstring(key));
            let mut instance_body = properties.clone();
            instance_body.extend_from_slice(&fstring("None"));
            body.extend_from_slice(&fstring(actual_type));
            body.extend_from_slice(&(instance_body.len() as u32).to_le_bytes());
            body.extend_from_slice(&instance_body);
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&fstring(key_type));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(descriptor_struct));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(descriptor_package));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&body);
        out
    }

    fn story_outer(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        instanced_struct_map(
            "m_GenericData",
            "StrProperty",
            "InstancedStruct",
            "/Script/StructUtils",
            "/Script/G1R.StorySaveGameData",
            entries,
        )
    }

    fn story_payload_with_key_type(key_type: &str, entries: &[(&str, i32)]) -> Vec<u8> {
        let values = name_int_map_with_key_type(key_type, entries);
        let by_class = struct_map(
            "SaveDataByStoryClass",
            "ObjectProperty",
            "SingleStorySaveGameData",
            &[(STORY_CLASS, values)],
        );
        let generic = story_outer(&[("Story", by_class)]);
        let mut payload = fstring("/Script/Angelscript.GothicFinalDataGame");
        payload.push(0);
        payload.extend_from_slice(&generic);
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload
    }

    fn private_root_with_properties(properties: &[Vec<u8>]) -> Vec<u8> {
        let mut payload = fstring("/Script/Angelscript.GothicFinalDataGame");
        payload.push(0);
        for property in properties {
            payload.extend_from_slice(property);
        }
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload
    }

    fn expected(stored: bool, raw_value: Option<i32>) -> StoryExpectedValue {
        StoryExpectedValue { stored, raw_value }
    }

    fn change(
        id: &str,
        present: bool,
        raw_value: Option<i32>,
        expected: StoryExpectedValue,
    ) -> StoryChange {
        StoryChange {
            id: id.to_string(),
            present,
            raw_value,
            expected,
            allow_unknown_create: false,
        }
    }

    #[test]
    fn story_query_classifies_from_exact_cache_schema_and_keeps_unknowns() {
        let root = story_root();
        let all = query_story(&root, "", None, 0, 100, false).unwrap();

        assert_eq!(all["storedTotal"], 3);
        assert_eq!(all["catalogTotal"], 470);
        assert_eq!(all["unsetTotal"], 468);
        assert_eq!(all["unknownStoredTotal"], 1);
        assert_eq!(all["includeUnset"], false);
        assert_eq!(all["total"], 3);
        assert_eq!(all["semanticTypeCounts"]["timeMarker"], 1);
        assert_eq!(all["semanticTypeCounts"]["chapter"], 1);
        assert_eq!(all["semanticTypeCounts"]["integer"], 0);
        assert_eq!(all["semanticTypeCounts"]["unknown"], 1);
        assert_eq!(all["currentGameTimeSeconds"], 1_875_587.943_7);

        let entries = all["entries"].as_array().unwrap();
        let stone = entries
            .iter()
            .find(|entry| entry["id"] == "Stone_OreArmor")
            .unwrap();
        assert_eq!(stone["rawValue"], 1_767_047);
        assert_eq!(stone["stored"], true);
        assert_eq!(stone["catalogKnown"], true);
        assert_eq!(stone["semanticType"], "timeMarker");
        assert_eq!(stone["declaredType"], "FInGameTime");
        assert_eq!(
            stone["path"],
            json!([
                "m_GenericData",
                "{Story}",
                "SaveDataByStoryClass",
                "{/Script/Angelscript.StoryG1R}",
                "StoryPropertyValues",
                "{Stone_OreArmor}",
            ])
        );

        let chapter = entries
            .iter()
            .find(|entry| entry["id"] == "Chapter")
            .unwrap();
        assert_eq!(chapter["semanticType"], "chapter");
        assert_eq!(chapter["declaredType"], "int");

        let unknown = entries
            .iter()
            .find(|entry| entry["id"] == "Unknown_Timer_Name")
            .unwrap();
        assert_eq!(unknown["semanticType"], "unknown");
        assert_eq!(unknown["declaredType"], "unknown");
        assert_eq!(unknown["stored"], true);
        assert_eq!(unknown["catalogKnown"], false);
    }

    #[test]
    fn story_query_filters_and_pages_without_changing_stored_counts() {
        let root = story_root();
        let filtered = query_story(&root, "stone timeMarker", None, 0, 1, false).unwrap();
        assert_eq!(filtered["total"], 1);
        assert_eq!(filtered["count"], 1);
        assert_eq!(filtered["storedTotal"], 3);
        assert_eq!(filtered["entries"][0]["id"], "Stone_OreArmor");
        assert_eq!(filtered["semanticTypeCounts"]["timeMarker"], 1);
        assert_eq!(filtered["storedSemanticTypeCounts"]["integer"], 0);
        assert_eq!(filtered["storedSemanticTypeCounts"]["unknown"], 1);

        let empty_page = query_story(&root, "", None, 2, 1, false).unwrap();
        assert_eq!(empty_page["total"], 3);
        assert_eq!(empty_page["count"], 1);
        assert_eq!(empty_page["entries"][0]["id"], "Unknown_Timer_Name");
    }

    #[test]
    fn story_query_filters_by_exact_semantic_type() {
        let root = story_root();

        let time_markers = query_story(&root, "", Some("timeMarker"), 0, 100, false).unwrap();
        assert_eq!(time_markers["semanticType"], "timeMarker");
        assert_eq!(time_markers["total"], 1);
        assert_eq!(time_markers["entries"][0]["id"], "Stone_OreArmor");
        assert_eq!(time_markers["semanticTypeCounts"]["timeMarker"], 1);
        assert_eq!(time_markers["semanticTypeCounts"]["unknown"], 0);

        let unknown = query_story(&root, "", Some("unknown"), 0, 100, false).unwrap();
        assert_eq!(unknown["total"], 1);
        assert_eq!(unknown["entries"][0]["id"], "Unknown_Timer_Name");
        assert_eq!(unknown["entries"][0]["semanticType"], "unknown");
        assert_eq!(unknown["semanticTypeCounts"]["unknown"], 1);

        let integers = query_story(&root, "", Some("integer"), 0, 1000, true).unwrap();
        assert_eq!(integers["total"], INTEGER_PROPERTY_COUNT);
        assert!(
            integers["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["semanticType"] == "integer")
        );

        // Semantic-type facets are API identifiers, not fuzzy search terms.
        let wrong_case = query_story(&root, "", Some("timemarker"), 0, 100, false).unwrap();
        assert_eq!(wrong_case["total"], 0);
    }

    #[test]
    fn include_unset_returns_catalog_union_and_marks_prospective_entries() {
        let root = story_root();
        let all = query_story(&root, "", None, 0, 1000, true).unwrap();

        assert_eq!(all["includeUnset"], true);
        assert_eq!(all["storedTotal"], 3);
        assert_eq!(all["catalogTotal"], 470);
        assert_eq!(all["unsetTotal"], 468);
        assert_eq!(all["unknownStoredTotal"], 1);
        assert_eq!(all["total"], 471);
        assert_eq!(all["count"], 471);
        assert_eq!(all["catalogSemanticTypeCounts"]["integer"], 419);
        assert_eq!(all["catalogSemanticTypeCounts"]["timeMarker"], 50);
        assert_eq!(all["catalogSemanticTypeCounts"]["chapter"], 1);
        assert_eq!(all["catalogSemanticTypeCounts"]["unknown"], 0);

        let entries = all["entries"].as_array().unwrap();
        let unset = entries
            .iter()
            .find(|entry| entry["id"] == "AfterCinematic_Nyras")
            .unwrap();
        assert_eq!(unset["rawValue"], Value::Null);
        assert_eq!(unset["stored"], false);
        assert_eq!(unset["catalogKnown"], true);
        assert_eq!(unset["path"], json!([]));
        assert_eq!(unset["semanticType"], "integer");

        let unknown = entries
            .iter()
            .find(|entry| entry["id"] == "Unknown_Timer_Name")
            .unwrap();
        assert_eq!(unknown["stored"], true);
        assert_eq!(unknown["catalogKnown"], false);
        assert_eq!(unknown["semanticType"], "unknown");
        assert_eq!(unknown["declaredType"], "unknown");

        let filtered = query_story(&root, "AfterCinematic_Nyras unset", None, 0, 10, true).unwrap();
        assert_eq!(filtered["total"], 1);
        assert_eq!(filtered["entries"][0]["id"], "AfterCinematic_Nyras");
    }

    #[test]
    fn time_marker_schema_is_exact_and_duplicate_free() {
        let time_markers = TIME_MARKER_PROPERTIES
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let integers = INTEGER_PROPERTIES
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let catalog = catalog_entries()
            .map(|(id, _)| id.to_ascii_lowercase())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(TIME_MARKER_PROPERTIES.len(), 50);
        assert_eq!(time_markers.len(), TIME_MARKER_PROPERTIES.len());
        assert_eq!(INTEGER_PROPERTIES.len(), 419);
        assert_eq!(integers.len(), INTEGER_PROPERTIES.len());
        assert_eq!(catalog.len(), 470);
        assert_eq!(
            catalog_semantic_type("Stone_OreArmor"),
            Some(SemanticType::TimeMarker)
        );
        assert_eq!(
            catalog_semantic_type("AfterCinematic_Nyras"),
            Some(SemanticType::Integer)
        );
        assert_eq!(
            catalog_semantic_type("Chapter"),
            Some(SemanticType::Chapter)
        );
        assert_eq!(catalog_semantic_type("Stone_OreArmor_Day"), None);
        // Declared in the class but not annotated with UPROPERTY in the cache.
        assert_eq!(catalog_semantic_type("DurstigeBauern"), None);
    }

    #[test]
    fn apply_changes_edits_inserts_removes_and_preserves_full_i32_range() {
        let mut payload = story_payload_with_key_type(
            "NameProperty",
            &[
                ("Stone_OreArmor", 1_767_047),
                ("Chapter", 2),
                ("Unknown_Timer_Name", 17),
            ],
        );
        let changes = vec![
            // FName lookup is case-insensitive and keeps the stored spelling.
            change(
                "stone_orearmor",
                true,
                Some(i32::MIN),
                expected(true, Some(1_767_047)),
            ),
            // Missing known ids are created from the exact catalog spelling.
            change(
                "aftercinematic_nyras",
                true,
                Some(i32::MAX),
                expected(false, None),
            ),
            change("CHAPTER", false, None, expected(true, Some(2))),
            // Already-stored unknown/modded ids remain fully editable.
            change(
                "unknown_timer_name",
                true,
                Some(-42),
                expected(true, Some(17)),
            ),
        ];

        apply_changes(&mut payload, &changes).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len());
        let snapshot = story_map_snapshot(&root).unwrap();
        assert_eq!(
            snapshot_value(&snapshot, "Stone_OreArmor").unwrap(),
            Some(i32::MIN)
        );
        assert_eq!(
            snapshot_value(&snapshot, "AfterCinematic_Nyras").unwrap(),
            Some(i32::MAX)
        );
        assert_eq!(snapshot_value(&snapshot, "Chapter").unwrap(), None);
        assert_eq!(
            snapshot_value(&snapshot, "Unknown_Timer_Name").unwrap(),
            Some(-42)
        );
        assert!(
            snapshot
                .iter()
                .any(|entry| entry.id == "AfterCinematic_Nyras"),
            "known creation must use the canonical catalog spelling"
        );
    }

    #[test]
    fn every_catalog_id_can_be_created_and_reset_in_one_atomic_batch() {
        let mut payload = story_payload_with_key_type("NameProperty", &[]);
        let add = catalog_entries()
            .enumerate()
            .map(|(index, (id, _))| change(id, true, Some(index as i32), expected(false, None)))
            .collect::<Vec<_>>();
        assert_eq!(add.len(), CATALOG_PROPERTY_COUNT);
        apply_changes(&mut payload, &add).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len());
        let snapshot = story_map_snapshot(&root).unwrap();
        assert_eq!(snapshot.len(), CATALOG_PROPERTY_COUNT);
        assert!(catalog_entries().all(|(id, _)| snapshot_value(&snapshot, id).unwrap().is_some()));

        let reset = add
            .iter()
            .map(|added| change(&added.id, false, None, expected(true, added.raw_value)))
            .collect::<Vec<_>>();
        apply_changes(&mut payload, &reset).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len());
        assert!(story_map_snapshot(&root).unwrap().is_empty());
    }

    #[test]
    fn apply_changes_guards_unknown_creation_but_allows_explicit_override() {
        let mut payload = story_payload_with_key_type("NameProperty", &[("Chapter", 2)]);
        let original = payload.clone();
        let mut create = change("Mod_CustomFlag", true, Some(123), expected(false, None));
        let error = apply_changes(&mut payload, &[create.clone()]).unwrap_err();
        assert!(matches!(error, CoreError::UnsupportedEdit(_)));
        assert!(error.to_string().contains("allowUnknownCreate"));
        assert_eq!(payload, original, "policy failure must be atomic");

        create.allow_unknown_create = true;
        apply_changes(&mut payload, &[create]).unwrap();
        let root = properties::parse_private_root(&payload).unwrap();
        let snapshot = story_map_snapshot(&root).unwrap();
        assert_eq!(
            snapshot_value(&snapshot, "mod_customflag").unwrap(),
            Some(123)
        );
    }

    #[test]
    fn apply_changes_rejects_stale_cas_and_case_folded_duplicate_requests_atomically() {
        let mut payload = story_payload_with_key_type("NameProperty", &[("Chapter", 2)]);
        let original = payload.clone();
        let stale = change("Chapter", true, Some(3), expected(true, Some(1)));
        let error = apply_changes(&mut payload, &[stale]).unwrap_err();
        assert!(matches!(error, CoreError::Validation(_)));
        assert!(error.to_string().contains("changed since it was loaded"));
        assert_eq!(payload, original);

        let duplicate = [
            change("Chapter", true, Some(3), expected(true, Some(2))),
            change("chapter", false, None, expected(true, Some(2))),
        ];
        let error = apply_changes(&mut payload, &duplicate).unwrap_err();
        assert!(matches!(error, CoreError::InvalidRequest(_)));
        assert!(error.to_string().contains("duplicate id"));
        assert_eq!(payload, original);
    }

    #[test]
    fn apply_changes_rejects_wrong_map_descriptor_and_duplicate_stored_fnames() {
        let mut wrong_descriptor = story_payload_with_key_type("StrProperty", &[("Chapter", 2)]);
        let original = wrong_descriptor.clone();
        let root = properties::parse_private_root(&wrong_descriptor).unwrap();
        assert!(!is_writable(&root));
        let edit = change("Chapter", true, Some(3), expected(true, Some(2)));
        let error = apply_changes(&mut wrong_descriptor, &[edit]).unwrap_err();
        assert!(matches!(error, CoreError::Parse(_)));
        assert!(error.to_string().contains("TMap<FName,int32>"));
        assert_eq!(wrong_descriptor, original);

        let mut duplicates =
            story_payload_with_key_type("NameProperty", &[("Chapter", 2), ("chapter", 3)]);
        let original = duplicates.clone();
        let root = properties::parse_private_root(&duplicates).unwrap();
        assert!(!is_writable(&root));
        let edit = change("Chapter", true, Some(4), expected(true, Some(2)));
        let error = apply_changes(&mut duplicates, &[edit]).unwrap_err();
        assert!(matches!(error, CoreError::Validation(_)));
        assert!(error.to_string().contains("duplicate FName ids"));
        assert_eq!(duplicates, original);
    }

    #[test]
    fn writable_and_apply_reject_ambiguity_at_every_story_path_segment() {
        let values = name_int_map_with_key_type("NameProperty", &[("Chapter", 2)]);
        let by_class = struct_map(
            "SaveDataByStoryClass",
            "ObjectProperty",
            "SingleStorySaveGameData",
            &[(STORY_CLASS, values.clone())],
        );
        let generic = story_outer(&[("Story", by_class.clone())]);

        let duplicate_generic = private_root_with_properties(&[generic.clone(), generic.clone()]);
        let duplicate_story_key = private_root_with_properties(&[story_outer(&[
            ("Story", by_class.clone()),
            ("Story", by_class.clone()),
        ])]);
        let duplicate_by_class_property = private_root_with_properties(&[story_outer(&[(
            "Story",
            [by_class.clone(), by_class.clone()].concat(),
        )])]);
        let duplicate_class_key = {
            let duplicate_by_class = struct_map(
                "SaveDataByStoryClass",
                "ObjectProperty",
                "SingleStorySaveGameData",
                &[(STORY_CLASS, values.clone()), (STORY_CLASS, values.clone())],
            );
            private_root_with_properties(&[story_outer(&[("Story", duplicate_by_class)])])
        };
        let duplicate_values_property = {
            let duplicate_values = [values.clone(), values].concat();
            let duplicate_by_class = struct_map(
                "SaveDataByStoryClass",
                "ObjectProperty",
                "SingleStorySaveGameData",
                &[(STORY_CLASS, duplicate_values)],
            );
            private_root_with_properties(&[story_outer(&[("Story", duplicate_by_class)])])
        };

        for (segment, mut payload) in [
            ("m_GenericData", duplicate_generic),
            ("Story", duplicate_story_key),
            ("SaveDataByStoryClass", duplicate_by_class_property),
            (STORY_CLASS, duplicate_class_key),
            ("StoryPropertyValues", duplicate_values_property),
        ] {
            let root = properties::parse_private_root(&payload).unwrap();
            assert!(!is_writable(&root), "{segment} ambiguity was advertised");
            let original = payload.clone();
            let error = apply_changes(
                &mut payload,
                &[change("Chapter", true, Some(3), expected(true, Some(2)))],
            )
            .unwrap_err();
            assert!(
                matches!(error, CoreError::Validation(_)),
                "{segment}: {error}"
            );
            assert!(
                error.to_string().contains("ambiguous story path"),
                "{segment}: {error}"
            );
            assert_eq!(payload, original, "{segment} ambiguity mutated bytes");
        }
    }

    #[test]
    fn writable_and_apply_reject_wrong_outer_story_map_schema() {
        let values = name_int_map_with_key_type("NameProperty", &[("Chapter", 2)]);
        let valid_by_class = struct_map(
            "SaveDataByStoryClass",
            "ObjectProperty",
            "SingleStorySaveGameData",
            &[(STORY_CLASS, values.clone())],
        );

        let wrap_generic = |by_class: Vec<u8>| {
            private_root_with_properties(&[story_outer(&[("Story", by_class)])])
        };
        let cases = [
            (
                "m_GenericData key",
                private_root_with_properties(&[instanced_struct_map(
                    "m_GenericData",
                    "NameProperty",
                    "InstancedStruct",
                    "/Script/StructUtils",
                    "/Script/G1R.StorySaveGameData",
                    &[("Story", valid_by_class.clone())],
                )]),
            ),
            (
                "m_GenericData value struct",
                private_root_with_properties(&[struct_map(
                    "m_GenericData",
                    "StrProperty",
                    "GenericData",
                    &[("Story", valid_by_class.clone())],
                )]),
            ),
            (
                "m_GenericData value package",
                private_root_with_properties(&[instanced_struct_map(
                    "m_GenericData",
                    "StrProperty",
                    "InstancedStruct",
                    "/Script/Other",
                    "/Script/G1R.StorySaveGameData",
                    &[("Story", valid_by_class.clone())],
                )]),
            ),
            (
                "m_GenericData actual type",
                private_root_with_properties(&[instanced_struct_map(
                    "m_GenericData",
                    "StrProperty",
                    "InstancedStruct",
                    "/Script/StructUtils",
                    "/Script/G1R.WrongStorySaveGameData",
                    &[("Story", valid_by_class.clone())],
                )]),
            ),
            (
                "SaveDataByStoryClass key",
                wrap_generic(struct_map(
                    "SaveDataByStoryClass",
                    "NameProperty",
                    "SingleStorySaveGameData",
                    &[(STORY_CLASS, values.clone())],
                )),
            ),
            (
                "SaveDataByStoryClass value struct",
                wrap_generic(struct_map(
                    "SaveDataByStoryClass",
                    "ObjectProperty",
                    "WrongStorySaveGameData",
                    &[(STORY_CLASS, values.clone())],
                )),
            ),
            (
                "SaveDataByStoryClass value package",
                wrap_generic(struct_map_with_package(
                    "SaveDataByStoryClass",
                    "ObjectProperty",
                    "SingleStorySaveGameData",
                    "/Script/Other",
                    &[(STORY_CLASS, values.clone())],
                )),
            ),
        ];

        for (schema, mut payload) in cases {
            let root = properties::parse_private_root(&payload).unwrap();
            assert!(!is_writable(&root), "{schema} was advertised writable");
            let original = payload.clone();
            let error = apply_changes(
                &mut payload,
                &[change("Chapter", true, Some(3), expected(true, Some(2)))],
            )
            .unwrap_err();
            assert!(matches!(error, CoreError::Parse(_)), "{schema}: {error}");
            assert!(
                error.to_string().contains("story path"),
                "{schema}: {error}"
            );
            assert_eq!(payload, original, "{schema} mutated bytes");
        }
    }

    #[test]
    fn outer_string_story_key_is_exact_and_case_sensitive() {
        let values = name_int_map_with_key_type("NameProperty", &[("Chapter", 2)]);
        let by_class = struct_map(
            "SaveDataByStoryClass",
            "ObjectProperty",
            "SingleStorySaveGameData",
            &[(STORY_CLASS, values)],
        );
        let mut with_case_variant = private_root_with_properties(&[story_outer(&[
            ("story", by_class.clone()),
            ("Story", by_class.clone()),
        ])]);
        let root = properties::parse_private_root(&with_case_variant).unwrap();
        assert!(
            is_writable(&root),
            "case-distinct FString keys are not an ambiguity"
        );
        apply_changes(
            &mut with_case_variant,
            &[change("Chapter", true, Some(3), expected(true, Some(2)))],
        )
        .unwrap();
        let root = properties::parse_private_root(&with_case_variant).unwrap();
        assert_eq!(
            snapshot_value(&story_map_snapshot(&root).unwrap(), "Chapter").unwrap(),
            Some(3)
        );

        let mut lowercase_only =
            private_root_with_properties(&[story_outer(&[("story", by_class)])]);
        let root = properties::parse_private_root(&lowercase_only).unwrap();
        assert!(!is_writable(&root));
        let original = lowercase_only.clone();
        let error = apply_changes(
            &mut lowercase_only,
            &[change("Chapter", true, Some(3), expected(true, Some(2)))],
        )
        .unwrap_err();
        assert!(matches!(error, CoreError::Parse(_)));
        assert_eq!(lowercase_only, original);
    }

    #[test]
    fn object_path_case_variants_are_an_ambiguous_story_class_key() {
        let values = name_int_map_with_key_type("NameProperty", &[("Chapter", 2)]);
        let case_variant = STORY_CLASS.to_ascii_lowercase();
        let by_class = struct_map(
            "SaveDataByStoryClass",
            "ObjectProperty",
            "SingleStorySaveGameData",
            &[(STORY_CLASS, values.clone()), (&case_variant, values)],
        );
        let mut payload = private_root_with_properties(&[story_outer(&[("Story", by_class)])]);
        let root = properties::parse_private_root(&payload).unwrap();
        assert!(!is_writable(&root));
        let original = payload.clone();
        let error = apply_changes(
            &mut payload,
            &[change("Chapter", true, Some(3), expected(true, Some(2)))],
        )
        .unwrap_err();
        assert!(matches!(error, CoreError::Validation(_)));
        assert!(error.to_string().contains("ambiguous story path"));
        assert_eq!(payload, original);
    }

    #[test]
    fn apply_changes_validates_internal_change_shape_before_mutation() {
        let mut payload = story_payload_with_key_type("NameProperty", &[("Chapter", 2)]);
        let original = payload.clone();
        for invalid in [
            change("Chapter", true, None, expected(true, Some(2))),
            change("Chapter", false, Some(2), expected(true, Some(2))),
            change("Chapter", true, Some(3), expected(true, None)),
            change(" Chapter", true, Some(3), expected(true, Some(2))),
        ] {
            assert!(matches!(
                apply_changes(&mut payload, &[invalid]),
                Err(CoreError::InvalidRequest(_))
            ));
            assert_eq!(payload, original);
        }
    }
}
