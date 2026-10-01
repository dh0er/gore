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
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(12, 0, 0.0f));
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(B, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceB::StaticClass()), n"FP_NavigationSupport393");
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(C, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceC::StaticClass()), n"FP_NavigationSupport391");
        AG1RGameState::SaveWorldFloatData(A.GetWorld(), n"gore_natural_voice_batch_v1_ready", 2.0f);
        // Finish participant movement before ending the conversation, as in the tested head setup.
        ::TeleportToSpot(Hero, n"FP_XT_WAIT_OUTSIDE");
        // Replacing the owner's routine may itself end the conversation; keep it last.
        ::TeleportToWaypointAndExchangeDailyRoutineToClass(A, TSubclassOf<UAIState_DailyRoutine>(UDailyRoutine_GoreNaturalVoiceObserver::StaticClass()), n"FP_NavigationSupport392");
        this.EndConversation();
    }
}

class UChoiceGoreNaturalVoiceNoon : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145102;
    default Caption = FText::FromString(n"02 13:59 - B wechselt seinen Platz".ToString());
    default PriorityRank = 79;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(13, 59, 0.0f));
        this.EndConversation();
    }
}

class UChoiceGoreNaturalVoiceEvening : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797145103;
    default Caption = FText::FromString(n"03 15:59 - B kehrt zurueck".ToString());
    default PriorityRank = 78;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreNaturalVoiceStatus(this.GetSelf()) == 2.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UGameTimeSubsystem::Get().AdvanceToClockTime(FClockTime(15, 59, 0.0f));
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
    default Caption = FText::FromString(n"C schon vorhanden: bitte Ausgangsspielstand 066 laden".ToString());
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
