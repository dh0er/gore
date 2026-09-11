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
    bool IsVisible() const { return true; }
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
    bool IsVisible() const { return GoreSeatGuardSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreSeatGuardSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreSeatGuardSetupStatus(this.GetSelf()) == 2.0f; }
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
    bool IsVisible() const { return GoreSeatGuardSetupStatus(this.GetSelf()) == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}
