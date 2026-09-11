// Runtime-only controls on B. Do not alter either weapon or its selection rules.
class UGoreRoleWeaponTrace : UActorComponent
{
    UPROPERTY() AGothicNPCState Subject;
    UPROPERTY() bool Armed = false;
    UPROPERTY() float Age = 0.0f;
    UPROPERTY() float Interval = 0.0f;
    UPROPERTY() int Samples = 0;
    UPROPERTY() int LastPhase = -1;
    UPROPERTY() int LastSelected = -1;
    UPROPERTY() int LastEquipped = -1;
    UPROPERTY() int LastTarget = -1;
    UPROPERTY() int LastReach = -2;
    UPROPERTY() int CombatSamples = 0;

    int Weapon(const UItemDefinition Item) const
    {
        if (Item == nullptr) return 0;
        if (Item.IsA(UItMw_1H_Sword_04_Diego_Sleeper::StaticClass())) return 1;
        if (Item.IsA(UItRw_Bow_Diego::StaticClass())) return 2;
        return 3;
    }
    void Value(int Slot, FString Field, float32 Number)
    {
        GoreRoleNote(Subject, FName("gore_role_weapon_" + Slot + "_" + Field), Number);
    }
    void Arm(AGothicNPCState NPC)
    {
        if (NPC == nullptr || NPC.GetInventory() == nullptr) return;
        Subject = NPC;
        Armed = NPC != nullptr;
        Age = 0.0f; Interval = 0.0f; Samples = 0; CombatSamples = 0;
        LastPhase = -1; LastSelected = -1; LastEquipped = -1; LastTarget = -1; LastReach = -2;
        GoreRoleNote(Subject, n"gore_role_weapon_samples", 0.0f);
        GoreRoleNote(Subject, n"gore_role_weapon_armed", Armed ? 1.0f : 0.0f);
        // Existing mixed equipment is an admission condition, never repaired here.
        GoreRoleNote(Subject, n"gore_role_weapon_bow_count", float32(Subject.GetInventory().CountItemsOfClass(UItRw_Bow_Diego)));
        GoreRoleNote(Subject, n"gore_role_weapon_sword_count", float32(Subject.GetInventory().CountItemsOfClass(UItMw_1H_Sword_04_Diego_Sleeper)));
        GoreRoleNote(Subject, n"gore_role_weapon_arrow_count", float32(Subject.GetInventory().CountItemsOfClass(UItAm_Arrow)));
        this.SetComponentTickEnabled(true);
    }
    UFUNCTION(BlueprintOverride)
    void Tick(float DeltaSeconds)
    {
        if (!Armed || Subject == nullptr) return;
        Age += DeltaSeconds; Interval += DeltaSeconds;
        if (Age > 60.0f || Samples >= 40)
        {
            Armed = false;
            GoreRoleNote(Subject, n"gore_role_weapon_armed", 0.0f);
            return;
        }
        if (Interval < 0.1f) return;
        Interval = 0.0f;
        UGameplayAbility_CharacterAI_Gothic AI = Cast<UGameplayAbility_CharacterAI_Gothic>(Subject.GetAI());
        AGothicCharacter Body = Subject.GetCharacter();
        if (AI == nullptr || Body == nullptr) return;
        int Phase = 0;
        if (::HasGameplayTag(Subject, GameplayTag::AIState_Conflict_Warning)) Phase = 1;
        if (::HasGameplayTag(Subject, GameplayTag::AIState_Conflict_Combat)) Phase = 2;
        int Selected = Weapon(AI.SelectedItemAction.ItemDefinition);
        int Equipped = Weapon(Body.GetCarryComponent().GetEquippedItemDefinition());
        AGothicCharacter Target = AI.GetCharacterOfInterest();
        int TargetKind = 0;
        int Reach = -1;
        float32 Distance = -1.0f;
        if (Target != nullptr)
        {
            TargetKind = Target.IsPlayerCharacterOrPlayerControlled() ? 1 : 2;
            Distance = Target.GetDistanceTo(Body);
            float32 Height = Body.GetSimpleCollisionHalfHeight() * 2.0f + Target.GetSimpleCollisionHalfHeight();
            Reach = Body.GetPerception().CanGroundedTargetBePotentiallyReached(Target, 100.0f, Height) ? 1 : 0;
        }
        // Record changes plus the first ten combat samples. Never select, draw,
        // update inventory, change a target, or call the scoring manager here.
        if (Phase == LastPhase && Selected == LastSelected && Equipped == LastEquipped
            && TargetKind == LastTarget && Reach == LastReach && !(Phase == 2 && CombatSamples < 10)) return;
        if (Phase == 2) ++CombatSamples;
        Value(Samples, "seconds", float32(Age));
        Value(Samples, "phase", float32(Phase));
        Value(Samples, "selected", float32(Selected));
        Value(Samples, "equipped", float32(Equipped));
        Value(Samples, "target", float32(TargetKind));
        Value(Samples, "distance", Distance);
        Value(Samples, "reachable", float32(Reach));
        Value(Samples, "close_gate", Target != nullptr && Distance < 400.0f && Reach == 1 ? 1.0f : 0.0f);
        ++Samples;
        GoreRoleNote(Subject, n"gore_role_weapon_samples", float32(Samples));
        LastPhase = Phase; LastSelected = Selected; LastEquipped = Equipped; LastTarget = TargetKind; LastReach = Reach;
    }
}

// Only this routine permits same-identity revival. Ordinary role wait does not.
// Exact death-memory tags come from UAIState_Conflict::MemorizeConflictEnd.
class UDailyRoutine_GoreRoleRevive : UAIState_DailyRoutine_Human
{
    default Schedule(0, 0, UAIState_Stand(), Location::Anywhere, 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
    default RestoreAttributesAfterTimespan = FInGameTime::FromHours(1.0);
    default TryReviveAfterTimespan = FInGameTime::FromHours(1.0);
    default TryReviveIfDeathMemoryHasAnyOf.AddTag(GameplayTag::Memory_Conflict_Killed_TrainingFight);
    default TryReviveIfDeathMemoryHasAnyOf.AddTag(GameplayTag::Memory_Conflict_Killed_PettyFight);
    default TryReviveIfDeathMemoryHasAnyOf.AddTag(GameplayTag::Memory_Conflict_Killed_TrueFight);
    default TryReviveIfDeathMemoryHasAnyOf.AddTag(GameplayTag::Memory_Conflict_Killed_PlayerNotInvolved);
}

void GoreRoleFieldSnapshot(AGothicNPCState B, AGothicCharacterState Hero)
{
    if (B == nullptr || Hero == nullptr) return;
    GoreRoleNote(B, n"gore_role_dead", ::IsDead(B) ? 1.0f : 0.0f);
    GoreRoleNote(B, n"gore_role_defeated", ::IsDefeated(B) ? 1.0f : 0.0f);
    GoreRoleNote(B, n"gore_role_health", ::GetHealthAttribute(B));
    GoreRoleNote(B, n"gore_role_enemy", ::IsEnemyTowards(B, Hero) ? 1.0f : 0.0f);
    GoreRoleNote(B, n"gore_role_relationship", float32(int(::GetRelationshipTowards(B, Hero))));
    GoreRoleNote(B, n"gore_role_flee", ::HasGameplayTag(B, GameplayTag::AIState_Conflict_Flee) ? 1.0f : 0.0f);
    GoreRoleNote(B, n"gore_role_follow", ::HasGameplayTag(B, GameplayTag::AIState_Follow) ? 1.0f : 0.0f);
}

namespace G1R::Conversation
{
class UChoiceGoreRoleFollow : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146020;
    default Caption = FText::FromString(n"20 Begleiter: B folgen / weiterfolgen".ToString());
    default PriorityRank = 80;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::ExchangeDailyRoutineToClass(Subject(), UDailyRoutine_Generic_FollowHero);
        GoreRoleNote(Subject(), n"gore_role_follow_command", 1.0f);
        this.EndConversation();
    }
}

class UChoiceGoreRoleWait : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146021;
    default Caption = FText::FromString(n"21 Begleiter: B hier warten".ToString());
    default PriorityRank = 79;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::ExchangeDailyRoutineToClass(Subject(), UDailyRoutine_GoreRoleWait);
        GoreRoleNote(Subject(), n"gore_role_follow_command", 0.0f);
        this.EndConversation();
    }
}

class UChoiceGoreRoleTraining : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146022;
    default Caption = FText::FromString(n"22 Kampf: B Trainingskampf (Niederlage, kein Todesziel)".ToString());
    default PriorityRank = 78;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Subject().GetAI() == nullptr || Hero() == nullptr) return;
        ::ExchangeDailyRoutineToClass(Subject(), UDailyRoutine_GoreRoleWait);
        ::StartTrainingFight(Subject().GetAI(), Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleEnemy : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146023;
    default Caption = FText::FromString(n"23 Feindschaft: B Feind bis zur Niederlage".ToString());
    default PriorityRank = 77;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Hero() == nullptr || Subject().GetAI() == nullptr) return;
        ::SetRelationshipUntilDefeat(Subject(), Hero(), ERelationship::Enemy);
        ::ForceHearingPerception(Subject().GetAI(), Hero());
        GoreRoleFieldSnapshot(Subject(), Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleGuild : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146024;
    default Caption = FText::FromString(n"24 Fraktion: B gildenlos (nur Test-B)".ToString());
    default PriorityRank = 76;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        Subject().SetTrueGuild(UGE_Guild_None);
        GoreRoleNote(Subject(), n"gore_role_guild_none", 1.0f);
        GoreRoleFieldSnapshot(Subject(), Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleGuildRestore : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146025;
    default Caption = FText::FromString(n"25 Fraktion: B Schatten-Anfuehrer wiederherstellen".ToString());
    default PriorityRank = 75;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        Subject().SetTrueGuild(UGE_Guild_Human_OldCamp_ShadowLeader);
        GoreRoleNote(Subject(), n"gore_role_guild_none", 0.0f);
        GoreRoleFieldSnapshot(Subject(), Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleFlee : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146026;
    default Caption = FText::FromString(n"26 Flucht: B immer fliehen lassen (danach 27)".ToString());
    default PriorityRank = 74;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameplayAbility_CharacterAI_Gothic AI = Cast<UGameplayAbility_CharacterAI_Gothic>(Subject().GetAI());
        if (AI == nullptr || Hero() == nullptr) return;
        if (GoreRoleRead(Subject(), n"gore_role_flee_override") == 0.0f)
            GoreRoleNote(Subject(), n"gore_role_old_flee_mode", float32(int(AI.ModeOfFleeOnUnfavorableCombat)));
        AI.ModeOfFleeOnUnfavorableCombat = EFleeOnUnfavorableCombatMode::Always;
        GoreRoleNote(Subject(), n"gore_role_flee_override", 1.0f);
        ::SetRelationshipUntilDefeat(Subject(), Hero(), ERelationship::Enemy);
        ::ForceHearingPerception(AI, Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleFleeRestore : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146027;
    default Caption = FText::FromString(n"27 Fluchtregel wiederherstellen".ToString());
    default PriorityRank = 73;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && GoreRoleRead(Subject(), n"gore_role_flee_override") == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameplayAbility_CharacterAI_Gothic AI = Cast<UGameplayAbility_CharacterAI_Gothic>(Subject().GetAI());
        if (AI == nullptr) return;
        AI.ModeOfFleeOnUnfavorableCombat = EFleeOnUnfavorableCombatMode(int(GoreRoleRead(Subject(), n"gore_role_old_flee_mode")));
        GoreRoleNote(Subject(), n"gore_role_flee_override", 0.0f);
        this.EndConversation();
    }
}

class UChoiceGoreRoleMortal : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146028;
    default Caption = FText::FromString(n"28 Todtest: B Trainingskampf mit Todesziel".ToString());
    default PriorityRank = 72;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Subject().GetAI() == nullptr || Hero() == nullptr) return;
        ::ExchangeDailyRoutineToClass(Subject(), UDailyRoutine_GoreRoleWait);
        GoreRoleNote(Subject(), n"gore_role_revive_enabled", 0.0f);
        ::StartTrainingFightToDeath(Subject().GetAI(), Hero());
        this.EndConversation();
    }
}

class UChoiceGoreRoleRevive : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146029;
    default Caption = FText::FromString(n"29 Wiederkehr: nativen Respawn fuer toten B freigeben".ToString());
    default PriorityRank = 71;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && ::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        GoreRoleFieldSnapshot(Subject(), Hero());
        Subject().ExchangeDailyRoutineToClass(UDailyRoutine_GoreRoleRevive);
        GoreRoleNote(Subject(), n"gore_role_revive_enabled", 1.0f);
        // No global chapter event, extra actor, identity replacement or tag removal.
        this.EndConversation();
    }
}

class UChoiceGoreRoleWeaponArm : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146030;
    default Caption = FText::FromString(n"30 Bogen/Schwert: 60s Diagnose starten, beide behalten".ToString());
    default PriorityRank = 70;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr && !::IsDead(Subject()); }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (Subject().GetInventory() == nullptr || Subject().GetCharacter() == nullptr) return;
        if (!::HasItem(Subject(), UItRw_Bow_Diego) || !::HasItem(Subject(), UItMw_1H_Sword_04_Diego_Sleeper))
        {
            GoreRoleNote(Subject(), n"gore_role_weapon_armed", -1.0f);
            return;
        }
        UGoreRoleWeaponTrace::GetOrCreate(this.GetSelf(), n"GoreRoleWeaponTrace").Arm(Subject());
        this.EndConversation();
    }
}

class UChoiceGoreRoleFieldRead : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146031;
    default Caption = FText::FromString(n"31 Rollen: Zustand protokollieren".ToString());
    default PriorityRank = 69;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return Subject() != nullptr; }
    UFUNCTION(BlueprintOverride)
    void Act() { GoreRoleFieldSnapshot(Subject(), Hero()); this.EndConversation(); }
}
}
