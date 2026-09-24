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
    bool IsVisible() const { return true; }
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
    bool IsVisible() const { return GoreObjectSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreObjectSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreObjectSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreObjectSetupStatus(this.GetSelf()) == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}
