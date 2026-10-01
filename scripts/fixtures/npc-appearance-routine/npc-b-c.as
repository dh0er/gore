// gore npc: authored character GORE_TEST_B, derived from OC_STT_Diego.
// Everything not spelled out below is inherited from that character.

class UCharacterDefinition_Human_GORE_TEST_B : UCharacterDefinition_Human_OldCamp_Shadow
{
    default m_UniqueName = n"GORE_TEST_B";
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_GORE_TEST_B::StaticClass();
    default m_CharacterType = GameplayTag::AIAgent_Human_Shadow;
    default m_LightType = 4;
    default m_InitialGuildEffect = UGE_Guild_Human_OldCamp_ShadowLeader::StaticClass();
    default m_Personality = UGothicCharacterPersonality_Brave_Archer_Patient::StaticClass();
    default m_AIAbility = UGameplayAbility_CharacterAI_Diego::StaticClass();
    default SetAttributeValue("AttributeSet_LevelProgression.Level", 100.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.Experience", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.SkillPoints", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.XPKillOrDefeatBounty", 1000.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.XPExecutedBounty", 1000.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Health.Health", 540.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Health.MaxHealth", 540.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.SuperArmor", 150.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.MaxSuperArmor", 150.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Mana.Mana", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Mana.MaxMana", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Mana.MaxMana", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Strength.Strength", 80.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Dexterity.Dexterity", 120.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Blunt", 32.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Edge", 32.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Point", 25.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Fire", 20.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Energy", 16.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Ice", 20.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Armor.Resistance_Wind", 12.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItRw_Bow_Diego::StaticClass()), 1, EInventoryTypes(4));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItAm_Arrow::StaticClass()), 100, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItFo_Potion_Health_03::StaticClass()), 99, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItFo_Potion_Wine::StaticClass()), 1, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItFo_Cheese::StaticClass()), 1, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItFo_Wineberrys_01::StaticClass()), 1, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItMi_Joint_03::StaticClass()), 1, EInventoryTypes(1));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItMw_1H_Sword_04_Diego_Sleeper::StaticClass()), 1, EInventoryTypes(3));
    default AddToInventory(TSubclassOf<UItemDefinition>(UItMi_Orenugget::StaticClass()), 18, EInventoryTypes(9));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Melee_OneHanded_Master::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Melee_Fists_Trained::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Ranged_Bow_Master::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Sneak::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Pickpocket_Skilled::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Picklock_Skilled::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Wallclimbing::StaticClass()));
}

// Explicit parameters from the shipped MO_Player asset and armor definitions.
// This model has only the Hero person option. No arbitrary NPC face is claimed.
class UCharacterVisualsDefinition_Human_GORE_TEST_B : UArmorVisualsDefinition
{
    default m_MutableAsset = "MO_Player";
    default m_MutableAssetPath = "/Game/Assets/Characters/Humans/Mutables/";
    default m_GTOAssetPath = "/Game/Assets/Characters/GTO/";
    default m_HasPreBakedSK = false;
    UPROPERTY()
    FString Person = "Hero";
    UPROPERTY()
    FString Head = "Head_01";
    UPROPERTY()
    FString Clothes = "GuardArmor";
    UPROPERTY()
    FString Shirt = "Shirt_01";
    UPROPERTY()
    FString Armor = "None";
    UPROPERTY()
    FString Shoulders = "None";
    UPROPERTY()
    FString Arms = "None";
    UPROPERTY()
    FString Belt = "Belt_02";
    UPROPERTY()
    FString Tassets = "None";
    UPROPERTY()
    FString Skirt = "Skirt_01";
    UPROPERTY()
    FString Pants = "Pants_01";
    UPROPERTY()
    FString Knees = "None";
    UPROPERTY()
    FString Boots = "Boots_02";
    UPROPERTY()
    FString Bracelet = "None";
}

class UAIAgentConfig_Human_GORE_TEST_B : UAIAgentConfig_Human
{
    default m_CharacterDefinition = UCharacterDefinition_Human_GORE_TEST_B::StaticClass();
}

class USpawnAIAgentDefinition_GORE_TEST_B : USpawnAIAgentDefinition
{
    default AIAgentConfigClass = UAIAgentConfig_Human_GORE_TEST_B::StaticClass();
    default AIAgentCharacterClass = n"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'";
}

class UConversationCharacterSettings_Ambient_GORE_TEST_B : UConversationCharacterSettings
{
    default ForCharacter = n"GORE_TEST_B";
    default VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::VoiceType_G1R_Voice05_Diego));
}

// Direct activities use shipped bPossibleAnywhere interactions. They do not
// require action tags on the route's navigation-only/restricted freepoints.
class UAIState_GoreScheduledActivity : UGothicCharacterSimulateableAIState
{
    default bSupportsSimulatedSteps = true;
    UPROPERTY()
    FGameplayTag ActionTag;

    UFUNCTION(BlueprintOverride)
    void OnGracefulExitRequested()
    {
        this.bShouldExitState = true;
        this.StopWaitingAndContinueTask();
    }

    UFUNCTION(BlueprintOverride)
    void DoTask()
    {
        while (!this.bShouldExitState)
        {
            ::GotoPreferredLocation(this.AI);
            if (this.bShouldExitState) { return; }
            ::TryInteractionWithoutSpot(this.AI, this.ActionTag, 20.0f);
            if (this.bShouldExitState) { return; }
            this.WaitSeconds(2.0f);
        }
    }
}

class UAIState_GoreScheduledRead : UAIState_GoreScheduledActivity
{
    default ActionTag = GameplayTag::Action_Conversation_ReadBook;
}

class UAIState_GoreScheduledDrink : UAIState_GoreScheduledActivity
{
    default ActionTag = GameplayTag::Action_Conversation_Drink;
}

class UDailyRoutine_GORE_TEST_B_Start : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_GoreScheduledRead(), n"FP_NavigationSupport393", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(12, 0, UAIState_GoreScheduledDrink(), n"FP_XT_WAIT_OUTSIDE", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(18, 0, UAIState_GoreScheduledRead(), n"FP_NavigationSupport393", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UCharacterDefinition_Human_GORE_TEST_C : UCharacterDefinition_Human_GORE_TEST_B
{
    default m_UniqueName = n"GORE_TEST_C";
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_GORE_TEST_C::StaticClass();
}

class UCharacterVisualsDefinition_Human_GORE_TEST_C : UCharacterVisualsDefinition_Human_GORE_TEST_B
{
    default Clothes = "NoviceArmor";
    default Shirt = "None";
    default Belt = "Belt_01";
    default Boots = "None";
    UPROPERTY()
    FString Forearm_L = "None";
    UPROPERTY()
    FString Forearm_R = "None";
    UPROPERTY()
    FString Poncho = "None";
}

class UAIAgentConfig_Human_GORE_TEST_C : UAIAgentConfig_Human
{
    default m_CharacterDefinition = UCharacterDefinition_Human_GORE_TEST_C::StaticClass();
}

class USpawnAIAgentDefinition_GORE_TEST_C : USpawnAIAgentDefinition
{
    default AIAgentConfigClass = UAIAgentConfig_Human_GORE_TEST_C::StaticClass();
    default AIAgentCharacterClass = n"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'";
}

class UConversationCharacterSettings_Ambient_GORE_TEST_C : UConversationCharacterSettings
{
    default ForCharacter = n"GORE_TEST_C";
    default VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::VoiceType_G1R_Voice05_Diego));
}

class UDailyRoutine_GoreAppearanceControl : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Stand(), n"FP_NavigationSupport392", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GORE_TEST_C_Start : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Stand(), n"FP_NavigationSupport391", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}
