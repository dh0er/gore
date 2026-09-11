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
            OriginalVisibility.SetNum(0);
            FacialBones.SetNum(0);
            PreviousBody = Body;
            PreviousBodyAsset = Body.GetSkeletalMeshAsset();
            HeadComponent = USkeletalMeshComponent::Get(Character, n"GoreFlexHead");
            PoseHead = nullptr;
            AddTickPrerequisiteComponent(Body);
        }
        if (!IsTestMode()) { RestoreBody(Body); return; }
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
