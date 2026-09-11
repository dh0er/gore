// gore-as decompiled module: AI.AIAgent.Human.Config.OC_STT_Diego.CharacterDefinition_OC_STT_Diego (AI/AIAgent/Human/Config/OC_STT_Diego/CharacterDefinition_OC_STT_Diego.as)
// NOTE: local names + string literals are not stored in the cache.


class UCharacterDefinition_Human_OC_STT_Diego : UCharacterDefinition_Human_OldCamp_Shadow
{
    default m_CharacterType = GameplayTag::AIAgent_Human_Shadow;
    default m_LightType = 4;
    default SetAttributeValue("AttributeSet_LevelProgression.Level", 100.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.Experience", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.SkillPoints", 0.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.XPKillOrDefeatBounty", 1000.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_LevelProgression.XPExecutedBounty", 1000.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Health.Health", 1234.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default SetAttributeValue("AttributeSet_Health.MaxHealth", 1234.0f, TSubclassOf<UDifficultySettings>(nullptr));
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
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_OC_STT_Diego::StaticClass();
    default m_InitialGuildEffect = UGE_Guild_Human_OldCamp_ShadowLeader::StaticClass();
    default m_Personality = UGothicCharacterPersonality_Brave_Archer_Patient::StaticClass();
    default m_AIAbility = UGameplayAbility_CharacterAI_Diego::StaticClass();
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Melee_OneHanded_Master::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Melee_Fists_Trained::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Ranged_Bow_Master::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Sneak::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Pickpocket_Skilled::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Picklock_Skilled::StaticClass()));
    default m_Skills.Add(TSubclassOf<UScriptGameplayEffect>(UGE_Skill_Wallclimbing::StaticClass()));
    default m_UniqueName = n"OC_STT_Diego";

    UCharacterDefinition_Human_OC_STT_Diego()
    {
        super();
        return;
    }
}

