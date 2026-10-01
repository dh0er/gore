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
    bool IsVisible() const { return true; }

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
    bool IsVisible() const { return GoreAppearanceSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreAppearanceSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreAppearanceSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreAppearanceSetupStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

class UChoiceGoreSetupIncomplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144206;
    default Caption = FText::FromString(n"13 Aufbau konnte nicht vorbereitet werden".ToString());
    default PriorityRank = 41;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreAppearanceSetupStatus(this.GetSelf()) == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}

}
