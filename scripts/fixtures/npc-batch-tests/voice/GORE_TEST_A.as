// gore-as decompiled module: AI.AIAgent.Human.Config.GORE_TEST_A.GORE_TEST_A (AI/AIAgent/Human/Config/GORE_TEST_A/GORE_TEST_A.as)
// NOTE: local names + string literals are not stored in the cache.


class UCharacterDefinition_Human_GORE_TEST_A : UCharacterDefinition_Human_OldCamp_Shadow
{
    default m_UniqueName = n"GORE_TEST_A";
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_GORE_TEST_A::StaticClass();
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

    UCharacterDefinition_Human_GORE_TEST_A()
    {
        super();
        return;
    }
}

class UCharacterVisualsDefinition_Human_GORE_TEST_A : UCharacterVisualsDefinition_Human_OC_STT_Diego
{
    default Person = "OC_STT_Diego";
    default m_PreBakedName = "OC_STT_Diego";
    default m_HasPreBakedSK = true;

    UCharacterVisualsDefinition_Human_GORE_TEST_A()
    {
        super();
        return;
    }
}

class UAIAgentConfig_Human_GORE_TEST_A : UAIAgentConfig_Human
{
    default m_CharacterDefinition = UCharacterDefinition_Human_GORE_TEST_A::StaticClass();

    UAIAgentConfig_Human_GORE_TEST_A()
    {
        super();
        return;
    }
}

class USpawnAIAgentDefinition_GORE_TEST_A : USpawnAIAgentDefinition
{
    default AIAgentConfigClass = UAIAgentConfig_Human_GORE_TEST_A::StaticClass();
    default AIAgentCharacterClass = n"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'";

    USpawnAIAgentDefinition_GORE_TEST_A()
    {
        return;
    }
}

class UConversationCharacterSettings_Ambient_GORE_TEST_A : UConversationCharacterSettings
{
    default ForCharacter = n"GORE_TEST_A";
    default VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::VoiceType_G1R_Voice05_Diego));

    UConversationCharacterSettings_Ambient_GORE_TEST_A()
    {
        return;
    }
}

class UDailyRoutine_GORE_TEST_A_Start : UAIState_DailyRoutine_Human
{
    default Schedule(0, 0, UAIState_Stand(), n"FP_XT_WAIT_OUTSIDE", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode(1);

    UDailyRoutine_GORE_TEST_A_Start()
    {
        super();
        return;
    }
}

namespace G1R::Conversation
{
class UTopic_Hero__GORE_TEST_A : UG1RDialogTopic
{
    default ForCharacter = n"GORE_TEST_A";
    default WithCharacter = n"Hero";

    UTopic_Hero__GORE_TEST_A()
    {
        super();
        return;
    }
    AGothicCharacterState GetHero() const
    {
        return this.GetCharacter(n"Hero");
    }
    AGothicCharacterState GetGORETESTA() const
    {
        return this.GetSelf();
    }
}

}
namespace G1R::Quest
{
class UQuest_GORE_NPC_SESSION : UG1RQuest
{
    default ParentQuestClass = TSubclassOf<UQuest>(G1R::Quest::UQuest_ValleyOfMines::StaticClass());
    default QuestKind = EQuestKind(2);
    default InvolvedCharacters.Add(n"Hero");
    default InvolvedCharacters.Add(n"GORE_TEST_A");
    default QuestGiverCharacterUniqueName = n"GORE_TEST_A";
    default NameText = ::G1R::Quest::GoreNpcSessionText(n"Erinnere dich an mich");
    default DescriptionText = ::G1R::Quest::GoreNpcSessionText(n"Sprich nach einer Pause erneut mit dem Fremden bei Xardas' Turm.");
    default bExternalStartTrigger = true;
    default bExternalSuccessTrigger = true;

    UQuest_GORE_NPC_SESSION()
    {
        super();
        return;
    }
}

}
namespace G1R::Conversation
{
class UChoiceGoreNpcSessionBegin : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143944;
    default Caption = FText::FromString(n"Auftrag: Erinnere dich an mich".ToString());
    default PriorityRank = 2;

    UChoiceGoreNpcSessionBegin()
    {
        super();
        return;
    }
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION()
    void Act_Implementation()
    {
        UQuest local_2 = ::G1R::Quest::GetGoreNpcSession();
        AGothicCharacterState local_6 = this.GetSelf();
        AGothicCharacterState local_10 = this.GetHero();
        if (local_2 != nullptr && (local_6 != nullptr) && (local_10 != nullptr) && !(local_6.Remembers(n"gore_npc_session_started")))
        {
            local_2.StartQuest(nullptr);
            local_6.GetRelationship().AddActiveModifier(UActivePersonalRelationshipModifier_Story(local_10, ERelationship(5)));
            local_6.Remember(n"gore_npc_session_started");
        }
        this.EndConversation();
        return;
    }
}

class UChoiceGoreNpcSessionComplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143945;
    default Caption = FText::FromString(n"Ich bin wieder da. Du erinnerst dich an mich.".ToString());
    default PriorityRank = 2;

    UChoiceGoreNpcSessionComplete()
    {
        super();
        return;
    }
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION()
    void Act_Implementation()
    {
        UQuest local_2 = ::G1R::Quest::GetGoreNpcSession();
        AGothicCharacterState local_6 = this.GetSelf();
        if (local_2 != nullptr && (local_6 != nullptr) && local_6.Remembers(n"gore_npc_session_started") && !(local_6.Remembers(n"gore_npc_session_completed")))
        {
            local_2.SucceedQuest(nullptr);
            local_6.Remember(n"gore_npc_session_completed");
        }
        this.EndConversation();
        return;
    }
}

class UChoiceGoreNpcSessionRemembered : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143946;
    default Caption = FText::FromString(n"Gut, dass wir Freunde geblieben sind.".ToString());
    default PriorityRank = 2;

    UChoiceGoreNpcSessionRemembered()
    {
        super();
        return;
    }
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION()
    void Act_Implementation()
    {
        this.EndConversation();
        return;
    }
}

}
namespace G1R::Quest
{
FText GoreNpcSessionText(const FName &inout Text)
{
    return FText::FromString(Text.ToString());
}
UQuest GetGoreNpcSession()
{
    return UQuestSubsystem::Get().GetQuestByClass(TSubclassOf<UQuest>(G1R::Quest::UQuest_GORE_NPC_SESSION::StaticClass()));
}
}

// Append to the checked GORE_TEST_A conversation; retain its NPC and quest declarations.
namespace G1R::Conversation
{
class UChoiceGoreNpcVoiceOriginal : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144101;
    default Caption = FText::FromString(n"01 Original: NPC und Held".ToString());
    default PriorityRank = 29;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("INFO_DIEGO_GAMESTART_11_00"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        ::Say(this.GetHero().GetAI(), ::LocText("INFO_DIEGO_GAMESTART_15_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceGreeting : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144102;
    default Caption = FText::FromString(n"02 Stimmtyp: Zuruf".ToString());
    default PriorityRank = 28;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), GameplayTag::Sound_Voice_Address_Call, EPerceptionNoiseLoudness(2), nullptr, false, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceMono48 : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144103;
    default Caption = FText::FromString(n"03 Neue Aufnahme: 48 kHz Mono".ToString());
    default PriorityRank = 27;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_48M_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceMono44 : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144104;
    default Caption = FText::FromString(n"04 Neue Aufnahme: 44,1 kHz Mono".ToString());
    default PriorityRank = 26;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_44M_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceStereo48 : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144105;
    default Caption = FText::FromString(n"05 Neue Aufnahme: 48 kHz Stereo".ToString());
    default PriorityRank = 25;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_48S_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceExchange : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144106;
    default Caption = FText::FromString(n"06 Neuer Dialog: NPC - Held - NPC".ToString());
    default PriorityRank = 24;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_48M_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        ::Say(this.GetHero().GetAI(), ::LocText("GORE_NPCVOICE_HERO_REPLY_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_END_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceMissing : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144107;
    default Caption = FText::FromString(n"07 Fehlende Aufnahme: Untertitel und weiter".ToString());
    default PriorityRank = 23;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_MISSING_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        ::Say(this.GetSelf().GetAI(), ::LocText("GORE_NPCVOICE_A_END_01"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);
        this.EndConversation();
    }
}

class UChoiceGoreNpcVoiceMumble : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144108;
    default Caption = FText::FromString(n"08 Stimmtyp: zwei Alltagszeilen".ToString());
    default PriorityRank = 22;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), GameplayTag::Sound_Voice_Conversation_DailyRoutine_Mumble, EPerceptionNoiseLoudness(2), nullptr, false, FGameplayTag::Empty);
        ::Say(this.GetSelf().GetAI(), GameplayTag::Sound_Voice_Conversation_DailyRoutine_Mumble, EPerceptionNoiseLoudness(2), nullptr, false, FGameplayTag::Empty);
        this.EndConversation();
    }
}

}

// Test setup explicitly places actors once. Phase controls change only game time.
namespace G1R::Conversation
{
// One saved status per attempt, so a previous attempt cannot hide a later failure.
float32 GoreAppearanceSetupStatus(AGothicCharacterState CharacterState)
{
    float32 Status = 0.0f;
    AG1RGameState::GetWorldFloatData(CharacterState.GetWorld(), n"gore_npc_setup_status_v2", Status);
    return Status;
}

void GoreSetAppearanceSetupStatus(AGothicCharacterState CharacterState, float32 Status)
{
    AG1RGameState::SaveWorldFloatData(CharacterState.GetWorld(), n"gore_npc_setup_status_v2", Status);
}

class UChoiceGoreAppearanceSetup : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144201;
    default Caption = FText::FromString(n"09 Aufbau (Lesen und Trinken)".ToString());
    default PriorityRank = 40;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        GoreSetAppearanceSetupStatus(this.GetSelf(), 1.0f);
        AGothicNPCState A = Cast<AGothicNPCState>(this.GetSelf());
        AGothicNPCState B = FCharacterUniqueName(n"GORE_TEST_B").GetNPCState();
        AGothicNPCState C = FCharacterUniqueName(n"GORE_TEST_C").GetNPCState();
        FInteractionSpotHandle CSpot = FInteractionSpotHandle(n"FP_NavigationSupport391");
        if (A == nullptr || B == nullptr || !CSpot.IsValid())
        {
            this.EndConversation();
            return;
        }
        if (C == nullptr)
        {
            FTransform SpawnAt = CSpot.GetTransform();
            C = MagicScript::SpawnAIAgentDefinition(TSubclassOf<UGameplayEffect>(nullptr), TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_GORE_TEST_C::StaticClass()), SpawnAt.GetLocation(), SpawnAt.GetRotation().Rotator(), TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GORE_TEST_C_Start::StaticClass()), this.GetSelf().GetCharacter());
        }
        if (C == nullptr)
        {
            this.EndConversation();
            return;
        }
        AGothicCharacterState Hero = this.GetHero();
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(C, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GORE_TEST_C_Start::StaticClass()), n"FP_NavigationSupport391");
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(B, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GORE_TEST_B_Start::StaticClass()), n"FP_NavigationSupport393");
        // B/C preparation must finish before touching the conversation participants.
        A.Remember(n"gore_npc_appearance_routine_ready");
        GoreSetAppearanceSetupStatus(A, 2.0f);
        ::TeleportToSpot(Hero, n"FP_XT_WAIT_OUTSIDE");
        // Replacing the owner's AI routine can itself finish the conversation.
        // Keep all required side effects before this final participant operation.
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(A, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreAppearanceControl::StaticClass()), n"FP_NavigationSupport392");
        this.EndConversation();
    }
}

class UChoiceGoreRoutineNoon : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144202;
    default Caption = FText::FromString(n"10 Uhr auf 11:59 - zum Trinken".ToString());
    default PriorityRank = 39;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(11, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreRoutineEvening : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144203;
    default Caption = FText::FromString(n"11 Uhr auf 17:59 - zurueck zum Lesen".ToString());
    default PriorityRank = 38;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(17, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreRoutineMorning : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144204;
    default Caption = FText::FromString(n"12 Uhr auf 08:00 - naechster Morgen".ToString());
    default PriorityRank = 37;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreSetupPrepared : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144205;
    default Caption = FText::FromString(n"13 Test vorbereitet - Figuren pruefen".ToString());
    default PriorityRank = 41;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

class UChoiceGoreSetupIncomplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144206;
    default Caption = FText::FromString(n"13 Aufbau konnte nicht vorbereitet werden".ToString());
    default PriorityRank = 41;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

}

namespace G1R::Conversation
{
void GoreSetHeadProbeMode(AGothicCharacterState Speaker, float32 Mode)
{
    AGothicNPCState C = FCharacterUniqueName(n"GORE_TEST_C").GetNPCState();
    if (C == nullptr) return;
    AG1RGameState::SaveWorldFloatData(Speaker.GetWorld(), n"gore_head_v1_mode", Mode);
    UGoreFlexHeadProbeController Controller = UGoreFlexHeadProbeController::GetOrCreate(C, n"GoreFlexHeadController");
    Controller.Initialize(C);
}
class UChoiceGoreHeadApply : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144301;
    default Caption = FText::FromString(n"14 C: anderen Kopf anlegen".ToString());
    default PriorityRank = 36;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        GoreSetHeadProbeMode(this.GetSelf(), 1.0f);
        this.EndConversation();
    }
}
class UChoiceGoreHeadRestore : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144302;
    default Caption = FText::FromString(n"15 C: Heldenkopf wiederherstellen".ToString());
    default PriorityRank = 35;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        GoreSetHeadProbeMode(this.GetSelf(), 0.0f);
        this.EndConversation();
    }
}
}

namespace G1R::Conversation
{
float32 GoreObjectSetupStatus(AGothicCharacterState CharacterState)
{
    float32 Status = 0.0f;
    AG1RGameState::GetWorldFloatData(CharacterState.GetWorld(), n"gore_npc_objects_setup_v1", Status);
    return Status;
}

class UChoiceGoreObjectsSetup : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144401;
    default Caption = FText::FromString(n"16 Objekttest: Bett und Alchemie".ToString());
    default PriorityRank = 50;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        AG1RGameState::SaveWorldFloatData(this.GetSelf().GetWorld(), n"gore_npc_objects_setup_v1", 1.0f);
        AGothicNPCState A = Cast<AGothicNPCState>(this.GetSelf());
        AGothicNPCState B = FCharacterUniqueName(n"GORE_TEST_B").GetNPCState();
        AGothicCharacterState Hero = this.GetHero();
        if (A == nullptr || B == nullptr || Hero == nullptr
            || !FInteractionSpotHandle(n"Interactive_Bed_Xardas472597").IsValid()
            || !FInteractionSpotHandle(n"IO_XT_POTION_ALCHEMY_01").IsValid()
            || !FInteractionSpotHandle(n"IO_XT_TAKENOTES_02").IsValid()
            || !FInteractionSpotHandle(n"WP_OW_TP_XARDASTOWER_BEDROOM").IsValid()
            || !FInteractionSpotHandle(n"FP_XT_STANDAROUND_01").IsValid())
        {
            this.EndConversation();
            return;
        }
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(B, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreObjectActivities::StaticClass()), n"IO_XT_TAKENOTES_02");
        AG1RGameState::SaveWorldFloatData(A.GetWorld(), n"gore_npc_objects_setup_v1", 2.0f);
        ::TeleportToSpot(Hero, n"WP_OW_TP_XARDASTOWER_BEDROOM");
        // Keep required setup before replacing the conversation owner's routine.
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(A, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreObjectObserver::StaticClass()), n"FP_XT_STANDAROUND_01");
        this.EndConversation();
    }
}

class UChoiceGoreObjectsNoon : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144402;
    default Caption = FText::FromString(n"17 Objekttest: 11:59 - zur Alchemie".ToString());
    default PriorityRank = 49;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(11, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreObjectsEvening : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144403;
    default Caption = FText::FromString(n"18 Objekttest: 17:59 - zurueck ins Bett".ToString());
    default PriorityRank = 48;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(17, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreObjectsMorning : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144404;
    default Caption = FText::FromString(n"19 Objekttest: 08:00 - naechster Morgen".ToString());
    default PriorityRank = 47;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreObjectsIncomplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144405;
    default Caption = FText::FromString(n"20 Objekttest: Aufbau fehlgeschlagen".ToString());
    default PriorityRank = 51;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}

namespace G1R::Conversation
{
float32 GoreSeatGuardSetupStatus(AGothicCharacterState CharacterState)
{
    float32 Status = 0.0f;
    AG1RGameState::GetWorldFloatData(CharacterState.GetWorld(), n"gore_npc_seat_guard_setup_v1", Status);
    return Status;
}

class UChoiceGoreSeatGuardSetup : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144411;
    default Caption = FText::FromString(n"21 Objekttest: Sitzen und Wache".ToString());
    default PriorityRank = 46;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        AG1RGameState::SaveWorldFloatData(this.GetSelf().GetWorld(), n"gore_npc_seat_guard_setup_v1", 1.0f);
        AGothicNPCState A = Cast<AGothicNPCState>(this.GetSelf());
        AGothicNPCState B = FCharacterUniqueName(n"GORE_TEST_B").GetNPCState();
        AGothicCharacterState Hero = this.GetHero();
        if (A == nullptr || B == nullptr || Hero == nullptr
            || !FInteractionSpotHandle(n"IO_OC_CHAIR_81").IsValid()
            || !FInteractionSpotHandle(n"BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5").IsValid()
            || !FInteractionSpotHandle(n"WP_Guide_Path_FromTo_SwampCamp_2").IsValid()
            || !FInteractionSpotHandle(n"WP_OC_NorthGate").IsValid())
        {
            this.EndConversation();
            return;
        }
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(B, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreSeatGuardActivities::StaticClass()), n"BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5");
        AG1RGameState::SaveWorldFloatData(A.GetWorld(), n"gore_npc_seat_guard_setup_v1", 2.0f);
        ::TeleportToSpot(Hero, n"WP_Guide_Path_FromTo_SwampCamp_2");
        // Keep required setup before replacing the conversation owner's routine.
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(A, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreSeatGuardObserver::StaticClass()), n"WP_OC_NorthGate");
        this.EndConversation();
    }
}

class UChoiceGoreSeatGuardNoon : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144412;
    default Caption = FText::FromString(n"22 Objekttest: 11:59 - zur Wache".ToString());
    default PriorityRank = 45;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(11, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreSeatGuardEvening : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144413;
    default Caption = FText::FromString(n"23 Objekttest: 17:59 - zurueck zum Stuhl".ToString());
    default PriorityRank = 44;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(17, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreSeatGuardMorning : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144414;
    default Caption = FText::FromString(n"24 Objekttest: 08:00 - naechster Morgen".ToString());
    default PriorityRank = 43;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreSeatGuardIncomplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144415;
    default Caption = FText::FromString(n"25 Objekttest: Sitz-Wache-Aufbau fehlgeschlagen".ToString());
    default PriorityRank = 47;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return false; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}

namespace G1R::Conversation
{
float32 GoreNaturalVoiceStatus(AGothicCharacterState State)
{
    float32 Value = 0.0f;
    AG1RGameState::GetWorldFloatData(State.GetWorld(), n"gore_natural_voice_batch_v1_ready", Value);
    return Value;
}

class UChoiceGoreNaturalVoiceSetup : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145101;
    default Caption = FText::FromString(n"01 Natuerliche Stimmen: Aufbau mit B und C".ToString());
    default PriorityRank = 80;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        AGothicNPCState A = Cast<AGothicNPCState>(this.GetSelf());
        AGothicNPCState B = FCharacterUniqueName(n"GORE_TEST_B").GetNPCState();
        AGothicNPCState C = FCharacterUniqueName(n"GORE_TEST_C").GetNPCState();
        AGothicCharacterState Hero = this.GetHero();
        float32 PreviousStatus = GoreNaturalVoiceStatus(this.GetSelf());
        AG1RGameState::SaveWorldFloatData(this.GetSelf().GetWorld(), n"gore_natural_voice_batch_v1_ready", 1.0f);
        if (A == nullptr || B == nullptr || Hero == nullptr
            || !FInteractionSpotHandle(n"FP_NavigationSupport391").IsValid()
            || !FInteractionSpotHandle(n"FP_NavigationSupport392").IsValid()
            || !FInteractionSpotHandle(n"FP_NavigationSupport393").IsValid()
            || !FInteractionSpotHandle(n"FP_XT_WAIT_OUTSIDE").IsValid())
        {
            this.EndConversation();
            return;
        }
        // Start from save030: a fresh C removes uncertainty about a voice type
        // already initialized in an older save. Reuse our own saved C afterwards.
        if (C != nullptr && PreviousStatus != 2.0f)
        {
            AG1RGameState::SaveWorldFloatData(A.GetWorld(), n"gore_natural_voice_batch_v1_ready", 3.0f);
            this.EndConversation();
            return;
        }
        if (C == nullptr)
        {
            FTransform SpawnAt = FInteractionSpotHandle(n"FP_NavigationSupport391").GetTransform();
            C = MagicScript::SpawnAIAgentDefinition(TSubclassOf<UGameplayEffect>(nullptr),
                TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_GORE_TEST_C::StaticClass()),
                SpawnAt.GetLocation(), SpawnAt.GetRotation().Rotator(),
                TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceC::StaticClass()), A.GetCharacter());
        }
        if (C == nullptr) { this.EndConversation(); return; }
        // This sets the relationship only. The later player sight perception,
        // not the menu, supplies the greeting event and selects the voice line.
        ::SetRelationshipTowards(B, Hero, ERelationship(5));
        ::SetRelationshipTowards(C, Hero, ERelationship(5));
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(8, 0, 0.0f));
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(B, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceB::StaticClass()), n"FP_NavigationSupport393");
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(C, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceC::StaticClass()), n"FP_NavigationSupport391");
        AG1RGameState::SaveWorldFloatData(A.GetWorld(), n"gore_natural_voice_batch_v1_ready", 2.0f);
        this.EndConversation();
        ::TeleportToSpot(Hero, n"FP_XT_WAIT_OUTSIDE");
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(A, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceObserver::StaticClass()), n"FP_NavigationSupport392");
    }
}

class UChoiceGoreNaturalVoiceNoon : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145102;
    default Caption = FText::FromString(n"02 11:59 - B wechselt seinen Platz".ToString());
    default PriorityRank = 79;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(11, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreNaturalVoiceEvening : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145103;
    default Caption = FText::FromString(n"03 17:59 - B kehrt zurueck".ToString());
    default PriorityRank = 78;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(17, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreNaturalVoiceReady : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145104;
    default Caption = FText::FromString(n"04 Bereit: B Diego, C Lares - ohne Gespraech annaehern".ToString());
    default PriorityRank = 77;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

class UChoiceGoreNaturalVoiceOldC : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145105;
    default Caption = FText::FromString(n"C schon vorhanden: bitte Ausgangsspielstand 030 laden".ToString());
    default PriorityRank = 81;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 3.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

class UChoiceGoreNaturalVoiceIncomplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145106;
    default Caption = FText::FromString(n"Aufbau fehlgeschlagen: Spielstand fuer Diagnose speichern".ToString());
    default PriorityRank = 81;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}
