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
    default m_AIAbility = UGameplayAbility_CharacterAI_GoreBWarningRecovery::StaticClass();
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
    // C retains its original AI; recovery belongs only to B.
    default m_AIAbility = UGameplayAbility_CharacterAI_Diego::StaticClass();
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
    default Schedule(0, 0, UAIState_GoreFlexHeadProbe(), n"FP_NavigationSupport391", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

// Empirical head probe; no material-index/section-index equivalence assumed.
// No section remapping: the material IDs are already resolved from the body.
class UGoreFlexHeadProbeController : UActorComponent
{
    UPROPERTY() AGothicNPCState State;
    UPROPERTY() AGothicCharacter VisualCharacter;
    UPROPERTY() USkeletalMesh FlexHeadAsset;
    UPROPERTY() USkeletalMesh PreviousBodyAsset;
    UPROPERTY() USkeletalMeshComponent PreviousBody;
    UPROPERTY() USkeletalMeshComponent HeadComponent;
    // Preserve the existing field prefix when replacing this module.
    // Material ID, LOD, original visibility triples.
    UPROPERTY() TArray<int> OriginalVisibility;
    UPROPERTY() UPoseableMeshComponent PoseHead;
    UPROPERTY() TArray<int> FacialBones;
    UPROPERTY() bool PoseActive = false;
    UPROPERTY() bool HavePreviousPose = false;
    UPROPERTY() FVector ExpectedHead;
    UPROPERTY() FVector ExpectedNeck;
    UPROPERTY() float32 HeadCopyError = -1.0f;
    UPROPERTY() float32 NeckCopyError = -1.0f;
    UPROPERTY() int PoseFrames = 0;


    // Derived materials belong to this one visual component; shared assets remain unchanged.
    UPROPERTY() UGorePaletteMaterial PaletteMaterials;
    UPROPERTY() UPoseableMeshComponent PaletteOwner;
    // Append fields: existing saves retain the tested controller field prefix.
    UPROPERTY() UTexture2D HeroCleanTexture;
    UPROPERTY() UTexture2D FlexBeardTexture;
    UPROPERTY() bool HeroTextureAttempted = false;
    UPROPERTY() bool FlexTextureAttempted = false;
    UPROPERTY() UGoreBeardMaterial HeroFaceMaterials;
    UPROPERTY() UGoreBeardMaterial FlexFaceMaterials;

    int BeardMode()
    {
        float32 Mode = 0.0f;
        AG1RGameState::GetWorldFloatData(State.GetWorld(), n"gore_palette_beard_mode", Mode);
        return int(Mode); // 0: original for this head, 1: clean, 2: beard.
    }
    UTexture2D BeardTexture(bool Flex)
    {
        if (Flex)
        {
            if (!FlexTextureAttempted)
            {
                FlexTextureAttempted = true;
                FlexBeardTexture = Rendering::ImportFileAsTexture2D(FPaths::ConvertRelativePathToFull(
                    FPaths::ProjectContentDir() + "GoreMods/NpcBeardSwitch/T_OC_IE_Flex_Beard_D.png"));
            }
            Note(n"gore_beard_flex_texture_loaded", IsValid(FlexBeardTexture) ? 1.0f : -1.0f);
            return FlexBeardTexture;
        }
        if (!HeroTextureAttempted)
        {
            HeroTextureAttempted = true;
            HeroCleanTexture = Rendering::ImportFileAsTexture2D(FPaths::ConvertRelativePathToFull(
                FPaths::ProjectContentDir() + "GoreMods/NpcBeardSwitch/T_NH_Head_Clean_D.png"));
        }
        Note(n"gore_beard_hero_texture_loaded", IsValid(HeroCleanTexture) ? 1.0f : -1.0f);
        return HeroCleanTexture;
    }
    void ApplyBeardMaterials(UMeshComponent Mesh, bool Flex)
    {
        int Mode = BeardMode();
        bool Replace = Flex ? Mode == 2 : Mode == 1;
        UTexture2D Texture;
        if (Replace) Texture = BeardTexture(Flex);
        UGoreBeardMaterial Entries = Flex ? FlexFaceMaterials : HeroFaceMaterials;
        int Matches = 0;
        int Applied = 0;
        for (int Slot = 0; Slot < Mesh.GetNumMaterials(); ++Slot)
        {
            UMaterialInterface Original = Mesh.GetMaterial(Slot);
            if (Flex ? !NamedMaterial(Original, n"MI_OC_IE_Flex_Head_G21", n"MI_Lods_OC_IE_Flex_Head_G21")
                : !NamedMaterial(Original, n"MI_NH_Head_G21", n"MI_Lods_NH_Head_G21")) continue;
            ++Matches;
            UGoreBeardMaterial Entry = Entries;
            while (IsValid(Entry) && Entry.Slot != Slot) Entry = Entry.Next;
            if (!Replace || !IsValid(Texture))
            {
                if (IsValid(Entry) && IsValid(Entry.Original)) Mesh.SetMaterial(Slot, Entry.Original);
                continue;
            }
            if (!IsValid(Entry))
            {
                // Allocate the record before changing the component: restore always has its original.
                Entry = Cast<UGoreBeardMaterial>(NewObject(this, TSubclassOf<UObject>(UGoreBeardMaterial::StaticClass()), NAME_None, false, nullptr));
                if (!IsValid(Entry)) continue;
                Entry.Original = Original;
                Entry.Material = Mesh.CreateDynamicMaterialInstance(Slot, Original, NAME_None);
                if (!IsValid(Entry.Material)) continue;
                Entry.Slot = Slot;
                Entry.Next = Entries;
                Entries = Entry;
            }
            Entry.Material.SetTextureParameterValue(n"1 - Albedo (Alpha Cutout)", Texture);
            if (!Flex)
            {
                // Both shipped animated albedos also contain the beard. Keep their normal maps.
                Entry.Material.SetTextureParameterValue(n"5 - WM1 Albedo", Texture);
                Entry.Material.SetTextureParameterValue(n"6 - WM2 Albedo", Texture);
            }
            Mesh.SetMaterial(Slot, Entry.Material);
            ++Applied;
        }
        if (Flex) FlexFaceMaterials = Entries;
        else HeroFaceMaterials = Entries;
        Note(Flex ? n"gore_beard_flex_face_matches" : n"gore_beard_hero_face_matches", float32(Matches));
        Note(Flex ? n"gore_beard_flex_face_applied" : n"gore_beard_hero_face_applied", float32(Applied));
    }
    bool PaletteFlag(FName Key)
    {
        float32 Value = 0.0f;
        AG1RGameState::GetWorldFloatData(State.GetWorld(), Key, Value);
        return Value != 0.0f;
    }
    bool NamedMaterial(UMaterialInterface Material, FName First, FName Second = NAME_None)
    {
        for (int Depth = 0; Depth < 12 && IsValid(Material); ++Depth)
        {
            if (Material.GetName() == First || (Second != NAME_None && Material.GetName() == Second)) return true;
            UMaterialInstance Instance = Cast<UMaterialInstance>(Material);
            if (!IsValid(Instance)) return false;
            Material = Instance.Parent;
        }
        return false;
    }
    void ApplyHeroParts(USkeletalMeshComponent Body)
    {
        ApplyBeardMaterials(Body, false);
        int HairCount=0; int BeardCount=0;
        for (int M=0; M<Body.GetNumMaterials(); ++M)
        {
            UMaterialInterface Mat=Body.GetMaterial(M);
            bool Hair=NamedMaterial(Mat,n"MI_NH_Hair_V2",n"MI_HairLods_NH_UVs") || NamedMaterial(Mat,n"MI_Hair_Ties");
            bool Beard=NamedMaterial(Mat,n"MI_NH_Beard");
            if (!Hair && !Beard) continue;
            if (Hair) ++HairCount;
            if (Beard) ++BeardCount;
            bool Visible=Hair ? !PaletteFlag(n"gore_palette_hair_hidden") : BeardMode() != 1;
            for (int L=0; L<Body.GetNumLODs(); ++L) Body.ShowMaterialSection(M,-1,Visible,L);
        }
        Note(n"gore_palette_hero_hair_matches",float32(HairCount));
        Note(n"gore_palette_hero_beard_matches",float32(BeardCount));
    }
    void ApplyFlexParts()
    {
        if (!IsValid(PoseHead)) return;
        if (PaletteOwner != PoseHead)
        {
            PaletteOwner=PoseHead;
            PaletteMaterials=nullptr;
            FlexFaceMaterials=nullptr;
            for (int M=0; M<PoseHead.GetNumMaterials(); ++M)
            {
                if (!NamedMaterial(PoseHead.GetMaterial(M),n"MI_OC_IE_Flex_Hair")) continue;
                UMaterialInstanceDynamic MID=PoseHead.CreateDynamicMaterialInstance(M,PoseHead.GetMaterial(M),NAME_None);
                if (!IsValid(MID)) continue;
                UGorePaletteMaterial Entry=Cast<UGorePaletteMaterial>(NewObject(this,TSubclassOf<UObject>(UGorePaletteMaterial::StaticClass()),NAME_None,false,nullptr));
                if (!IsValid(Entry)) continue;
                Entry.Material=MID;
                Entry.RootColor=MID.GetVectorParameterValue(n"Root Color");
                Entry.TipColor=MID.GetVectorParameterValue(n"Tip Color");
                Entry.Next=PaletteMaterials;
                PaletteMaterials=Entry;
            }
        }
        int Matches=0;
        for (int M=0; M<PoseHead.GetNumMaterials(); ++M)
        {
            if (!NamedMaterial(PoseHead.GetMaterial(M),n"MI_OC_IE_Flex_Hair",n"MI_OC_IE_Flex_Haircap")) continue;
            ++Matches;
            for (int L=0; L<PoseHead.GetNumLODs(); ++L) PoseHead.ShowMaterialSection(M,-1,!PaletteFlag(n"gore_palette_hair_hidden"),L);
        }
        bool Red=PaletteFlag(n"gore_palette_hair_red");
        int Tinted=0;
        for (UGorePaletteMaterial Entry=PaletteMaterials; IsValid(Entry); Entry=Entry.Next)
        {
            if (!IsValid(Entry.Material)) continue;
            Entry.Material.SetVectorParameterValue(n"Root Color",Red ? FLinearColor(0.35f,0.008f,0.002f,1.0f) : Entry.RootColor);
            Entry.Material.SetVectorParameterValue(n"Tip Color",Red ? FLinearColor(0.9f,0.035f,0.005f,1.0f) : Entry.TipColor);
            ++Tinted;
        }
        ApplyBeardMaterials(PoseHead, true);
        Note(n"gore_palette_flex_hair_matches",float32(Matches));
        Note(n"gore_palette_tint_materials",float32(Tinted));
    }

    void Note(FName Key, float32 Value)
    {
        if (IsValid(State)) AG1RGameState::SaveWorldFloatData(State.GetWorld(), Key, Value);
    }
    bool IsTestMode()
    {
        float32 Mode = 0.0f;
        if (!AG1RGameState::GetWorldFloatData(State.GetWorld(), n"gore_head_v1_mode", Mode)) return false;
        return Mode != 0.0f;
    }
    void Initialize(AGothicNPCState NPCState)
    {
        if (!IsValid(NPCState)) return;
        if (IsValid(State)) State.OnCharacterSpawned.Unbind(this, n"OnCharacterSpawned");
        State = NPCState;
        State.OnCharacterSpawned.AddUFunction(this, n"OnCharacterSpawned");
        Refresh(State.GetCharacter());
    }
    void Apply()
    {
        Note(n"gore_head_v1_mode", 1.0f);
        if (IsValid(State)) Refresh(State.GetCharacter());
    }
    void Restore()
    {
        Note(n"gore_head_v1_mode", 0.0f);
        if (IsValid(State)) Refresh(State.GetCharacter());
    }
    UFUNCTION()
    void OnCharacterSpawned(AGothicNPCState NPCState, AGothicCharacter Character)
    {
        if (NPCState == State) Refresh(Character);
    }
    bool IsStockHeadName(FName MaterialName)
    {
        return MaterialName == n"MI_NH_Head_G21" || MaterialName == n"MI_Lods_NH_Head_G21"
            || MaterialName == n"MI_NH_Eye" || MaterialName == n"MI_NH_Mouth"
            || MaterialName == n"MI_NH_Beard" || MaterialName == n"MI_NH_Hair_V2"
            || MaterialName == n"MI_HairLods_NH_UVs" || MaterialName == n"MI_Hair_Ties"
            || MaterialName == n"MI_Eye_Meniscus_Trailer" || MaterialName == n"MI_EyeAO_01";
    }
    // Body animation runs first, then this controller writes the pose, then the
    // poseable component refreshes skinning. Facial descendants keep Flex's pose.
    UFUNCTION(BlueprintOverride)
    void Tick(float DeltaSeconds)
    {
        if (!PoseActive || !IsValid(PreviousBody) || !IsValid(PoseHead)) return;
        if (HavePreviousPose)
        {
            FVector ActualHead = PoseHead.GetBoneTransformByName(n"head", EBoneSpaces::ComponentSpace).GetLocation();
            FVector ActualNeck = PoseHead.GetBoneTransformByName(n"neck_01", EBoneSpaces::ComponentSpace).GetLocation();
            HeadCopyError = float32((ActualHead - ExpectedHead).Size());
            NeckCopyError = float32((ActualNeck - ExpectedNeck).Size());
        }
        ExpectedHead = PreviousBody.GetSocketTransform(n"head", ERelativeTransformSpace::RTS_Component).GetLocation();
        ExpectedNeck = PreviousBody.GetSocketTransform(n"neck_01", ERelativeTransformSpace::RTS_Component).GetLocation();
        PoseHead.CopyPoseFromSkeletalComponent(PreviousBody);
        for (int Index = 0; Index < FacialBones.Num(); ++Index)
            PoseHead.ResetBoneTransformByName(PoseHead.GetBoneName(FacialBones[Index]));
        HavePreviousPose = true;
        if (PoseFrames < 1000000) ++PoseFrames;
    }
    bool IsStockHeadMaterial(UMaterialInterface Material)
    {
        for (int Depth = 0; Depth < 12 && IsValid(Material); ++Depth)
        {
            if (IsStockHeadName(Material.GetName())) return true;
            UMaterialInstance Instance = Cast<UMaterialInstance>(Material);
            if (!IsValid(Instance)) return false;
            Material = Instance.Parent;
        }
        return false;
    }
    void RestoreBody(USkeletalMeshComponent Body)
    {
        PoseActive = false;
        HavePreviousPose = false;
        PoseFrames = 0;
        HeadCopyError = -1.0f;
        NeckCopyError = -1.0f;
        for (int Entry = 0; Entry + 2 < OriginalVisibility.Num(); Entry += 3)
            Body.ShowMaterialSection(OriginalVisibility[Entry], -1,
                OriginalVisibility[Entry + 2] != 0, OriginalVisibility[Entry + 1]);
        OriginalVisibility.SetNum(0);
        if (IsValid(HeadComponent)) HeadComponent.SetVisibility(false, true);
        if (IsValid(PoseHead)) PoseHead.SetVisibility(false, true);
        Note(n"gore_head_v1_hidden", 0.0f);
        Note(n"gore_head_v2_expected", 0.0f);
        Note(n"gore_head_v3_rigid", 0.0f);
        Note(n"gore_head_v4_active", 0.0f);
        Note(n"gore_head_v1_status", 0.0f);
    }
    void Refresh(AGothicCharacter Character)
    {
        if (!IsValid(State) || !IsValid(Character)) return;
        VisualCharacter = Character;
        USkeletalMeshComponent Body = Character.Mesh;
        if (!IsValid(Body) || !IsValid(Body.GetSkeletalMeshAsset())) return;
        if (PreviousBody != Body || PreviousBodyAsset != Body.GetSkeletalMeshAsset())
        {
            // BFG interpolation accepts skinned meshes but later assumes skeletal
            // components. Exclude only this visual actor before adding PoseHead.
            UBFGTickOptimizerSystem::SetTickOptimizationRuntime_Enabled(Character, false);
            PoseActive = false;
            HavePreviousPose = false;
            PoseFrames = 0;
            HeadCopyError = -1.0f;
            NeckCopyError = -1.0f;
            if (IsValid(PreviousBody)) RemoveTickPrerequisiteComponent(PreviousBody);
            if (IsValid(PoseHead))
            {
                PoseHead.SetVisibility(false, true);
                PoseHead.RemoveTickPrerequisiteComponent(this);
            }
            for (UGoreBeardMaterial Entry = HeroFaceMaterials; IsValid(Entry); Entry = Entry.Next)
            {
                if (IsValid(PreviousBody) && IsValid(Entry.Original)
                    && Entry.Slot >= 0 && Entry.Slot < PreviousBody.GetNumMaterials()
                    && PreviousBody.GetMaterial(Entry.Slot) == Entry.Material)
                    PreviousBody.SetMaterial(Entry.Slot, Entry.Original);
            }
            OriginalVisibility.SetNum(0);
            FacialBones.SetNum(0);
            PreviousBody = Body;
            PreviousBodyAsset = Body.GetSkeletalMeshAsset();
            HeroFaceMaterials = nullptr;
            HeadComponent = USkeletalMeshComponent::Get(Character, n"GoreFlexHead");
            PoseHead = nullptr;
            AddTickPrerequisiteComponent(Body);
        }
        if (!IsTestMode()) { RestoreBody(Body); ApplyHeroParts(Body); return; }
        Note(n"gore_head_v3_rigid", 0.0f);
        Note(n"gore_head_v4_active", 0.0f);
        if (!IsValid(FlexHeadAsset))
        {
            FlexHeadAsset = Cast<USkeletalMesh>(LoadObject(nullptr,
                "/Game/Assets/Characters/Humans/TierC/Flex/SK_OC_IE_Flex_Head.SK_OC_IE_Flex_Head"));
        }
        Note(n"gore_head_v1_loaded", IsValid(FlexHeadAsset) ? 1.0f : 0.0f);
        if (!IsValid(FlexHeadAsset))
        {
            if (IsValid(HeadComponent)) HeadComponent.SetVisibility(false, true);
            if (IsValid(PoseHead)) PoseHead.SetVisibility(false, true);
            PoseActive = false;
            Note(n"gore_head_v1_hidden", 0.0f);
            Note(n"gore_head_v1_status", -1.0f);
            return;
        }
        if (IsValid(HeadComponent)) HeadComponent.SetVisibility(false, true);
        PoseHead = UPoseableMeshComponent::GetOrCreate(Character, n"GoreFlexPoseHead");
        PoseHead.SetCollisionEnabled(ECollisionEnabled::NoCollision);
        if (PoseHead.GetSkinnedAsset() != FlexHeadAsset)
        {
            PoseHead.SetSkinnedAssetAndUpdate(FlexHeadAsset, true);
            FacialBones.SetNum(0);
            HavePreviousPose = false;
            PoseFrames = 0;
            HeadCopyError = -1.0f;
            NeckCopyError = -1.0f;
        }
        PoseHead.SetLeaderPoseComponent(nullptr, true, false);
        PoseHead.SetRenderStatic(false);
        PoseHead.AttachToComponent(Body, NAME_None, EAttachmentRule::SnapToTarget);
        PoseHead.SetRelativeTransform(FTransform());
        PoseHead.AddTickPrerequisiteComponent(this);
        bool BonesMatch = Body.GetBoneIndex(n"head") >= 0 && Body.GetBoneIndex(n"neck_01") >= 0
            && PoseHead.GetBoneIndex(n"head") >= 0 && PoseHead.GetBoneIndex(n"neck_01") >= 0;
        if (FacialBones.Num() == 0 && BonesMatch)
        {
            for (int BoneIndex = 0; BoneIndex < PoseHead.GetNumBones(); ++BoneIndex)
            {
                FName Bone = PoseHead.GetBoneName(BoneIndex);
                if (Body.GetBoneIndex(Bone) < 0) { BonesMatch = false; break; }
                FName Parent = PoseHead.GetParentBone(Bone);
                bool ReachedRoot = false;
                for (int Step = 0; Step < PoseHead.GetNumBones(); ++Step)
                {
                    if (Parent == NAME_None) { ReachedRoot = true; break; }
                    if (Parent == n"head") { FacialBones.Add(BoneIndex); ReachedRoot = true; break; }
                    Parent = PoseHead.GetParentBone(Parent);
                }
                if (!ReachedRoot) { BonesMatch = false; break; }
            }
        }
        BonesMatch = BonesMatch && FacialBones.Num() > 0;
        Note(n"gore_head_v1_bones", BonesMatch ? 1.0f : 0.0f);
        if (!BonesMatch)
        {
            PoseActive = false;
            FacialBones.SetNum(0);
            PoseHead.SetVisibility(false, true);
            Note(n"gore_head_v1_hidden", 0.0f);
            Note(n"gore_head_v1_status", -2.0f);
            return;
        }
        PoseActive = true;
        PoseHead.SetVisibility(true, true);
        ApplyFlexParts();
        Note(n"gore_head_v4_active", 1.0f);
        Note(n"gore_head_v4_frames", float32(PoseFrames));
        Note(n"gore_head_v4_facial", float32(FacialBones.Num()));
        Note(n"gore_head_v4_head_error", HeadCopyError);
        Note(n"gore_head_v4_neck_error", NeckCopyError);
        bool CaptureVisibility = OriginalVisibility.Num() == 0;
        int Matched = 0;
        int Hidden = 0;
        int Expected = 0;
        for (int MaterialID = 0; MaterialID < Body.GetNumMaterials(); ++MaterialID)
        {
            if (!IsStockHeadMaterial(Body.GetMaterial(MaterialID))) continue;
            ++Matched;
            for (int LOD = 0; LOD < Body.GetNumLODs(); ++LOD)
            {
                ++Expected;
                if (CaptureVisibility)
                {
                    OriginalVisibility.Add(MaterialID);
                    OriginalVisibility.Add(LOD);
                    OriginalVisibility.Add(Body.IsMaterialSectionShown(MaterialID, LOD) ? 1 : 0);
                }
                Body.ShowMaterialSection(MaterialID, -1, false, LOD);
                if (!Body.IsMaterialSectionShown(MaterialID, LOD)) ++Hidden;
            }
        }
        Note(n"gore_head_v1_matched", float32(Matched));
        Note(n"gore_head_v1_hidden", float32(Hidden));
        Note(n"gore_head_v2_expected", float32(Expected));
        Note(n"gore_head_v1_status", Matched > 0 && Hidden == Expected ? 4.0f : 3.0f);
    }
}

// Schedule only for C. State entry reconstructs the controller after loading;
// spawn callbacks and the existing two-second state loop handle visual recreation.
class UAIState_GoreFlexHeadProbe : UGothicCharacterSimulateableAIState
{
    default bSupportsSimulatedSteps = true;
    UFUNCTION(BlueprintOverride)
    void OnGracefulExitRequested()
    {
        this.bShouldExitState = true;
        this.StopWaitingAndContinueTask();
    }
    UFUNCTION(BlueprintOverride)
    void DoTask()
    {
        AGothicNPCState HeadNPC = Cast<AGothicNPCState>(this.AI.GetCharacterState());
        UGoreFlexHeadProbeController ProbeController;
        if (IsValid(HeadNPC))
        {
            ProbeController = UGoreFlexHeadProbeController::GetOrCreate(HeadNPC, n"GoreFlexHeadController");
            ProbeController.Initialize(HeadNPC);
        }
        while (!this.bShouldExitState)
        {
            if (IsValid(ProbeController)) ProbeController.Refresh(HeadNPC.GetCharacter());
            ::GotoPreferredLocation(this.AI);
            if (this.bShouldExitState) return;
            this.WaitSeconds(2.0f);
        }
    }
}

// Exact bed/alchemy pairings from Xardas' shipped bedroom routine.
// Only initial setup teleports B; scheduled phase changes use normal navigation.
class UDailyRoutine_GoreObjectActivities : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Sleep(), n"Interactive_Bed_Xardas472597", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(12, 0, UAIState_PotionAlchemy(), n"IO_XT_POTION_ALCHEMY_01", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(18, 0, UAIState_Sleep(), n"Interactive_Bed_Xardas472597", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreObjectObserver : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    // Navigation-only use; do not request the Xardas-restricted StandAround action.
    default Schedule(0, 0, UAIState_Stand(), n"FP_XT_STANDAROUND_01", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

// Daytime use of an unrestricted chair and nearby night-watch point.
class UDailyRoutine_GoreSeatGuardActivities : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Sit(), n"IO_OC_CHAIR_81", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(12, 0, UAIState_GuardWatch(), n"BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default Schedule(18, 0, UAIState_Sit(), n"IO_OC_CHAIR_81", 150.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

class UDailyRoutine_GoreSeatGuardObserver : UAIState_DailyRoutine_Human
{
    default ScheduleTimeOffsetMinutesMin = 0.0f;
    default ScheduleTimeOffsetMinutesMax = 0.0f;
    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;
    default Schedule(0, 0, UAIState_Stand(), n"WP_OC_NorthGate", 100.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

// B-only recovery for a completed threat assessment that leaves an empty
// warning state running. Keep the shipped warning, voice and escalation paths.
class UAIState_GoreBWarningRecovery : UAIState_Warning_Crime_Human_WeaponDrawn
{
    UPROPERTY()
    bool RecoveryRecorded = false;

    void BeginWarning()
    {
        Super::BeginWarning();
        AG1RGameState::SaveWorldFloatData(this.GetCharacterState().GetWorld(), n"gore_b_warning_fix_v1_active", 1.0f);
    }

    void DoInLoopIfCharacterNotStillWarning()
    {
        if (this.bShouldExitState) return;
        if (this.bEndAssessmentDone
            && this.GetCharacterOfInterest() == nullptr
            && this.GetCharactersBeingWarned().IsEmpty()
            && this.AI.GetSensedLivingEnemies(false).IsEmpty())
        {
            if (!this.RecoveryRecorded)
            {
                this.RecoveryRecorded = true;
                AG1RGameState::SaveWorldFloatData(this.GetCharacterState().GetWorld(), n"gore_b_warning_fix_v1_recovered", 1.0f);
            }
            // Use the regular crime assessment and its normal conflict cleanup.
            // The diagnostic flag must not prevent a retry if assessment waits.
            this.AssessEndWarning();
            return;
        }
        Super::DoInLoopIfCharacterNotStillWarning();
    }
}

// Diego adds only this scoring default to Human. Derive from the same supported
// parent because the shipped Diego leaf has a final defaults initializer.
class UGameplayAbility_CharacterAI_GoreBWarningRecovery : UGameplayAbility_CharacterAI_Human
{
    default AddWeightedCombatTargetScoringEntry(UAICombatTargetScoringEntry_ZombieBias(), 10000.0);
    default SetAIStateClassForType(GameplayTag::AIState_Conflict_Warning_Threat, UAIState_GoreBWarningRecovery);
}

// Per-component original colors; avoids introducing native array specializations.
class UGorePaletteMaterial : UObject
{
    UPROPERTY() UMaterialInstanceDynamic Material;
    UPROPERTY() FLinearColor RootColor;
    UPROPERTY() FLinearColor TipColor;
    UPROPERTY() UGorePaletteMaterial Next;
}

// One linked record per C-owned face slot; no mutation of the source material asset.
class UGoreBeardMaterial : UObject
{
    UPROPERTY() int Slot = -1;
    UPROPERTY() UMaterialInterface Original;
    UPROPERTY() UMaterialInstanceDynamic Material;
    UPROPERTY() UGoreBeardMaterial Next;
}
