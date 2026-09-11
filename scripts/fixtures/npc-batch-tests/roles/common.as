// Helpers before the namespace go to B; conversation declarations go to A.
void GoreRoleNote(AGothicCharacterState Context, FName Key, float32 Value)
{
    if (Context != nullptr) AG1RGameState::SaveWorldFloatData(Context.GetWorld(), Key, Value);
}

float32 GoreRoleRead(AGothicCharacterState Context, FName Key)
{
    float32 Value = 0.0f;
    if (Context != nullptr) AG1RGameState::GetWorldFloatData(Context.GetWorld(), Key, Value);
    return Value;
}

class UDailyRoutine_GoreRoleWait : UAIState_DailyRoutine_Human
{
    default Schedule(0, 0, UAIState_Stand(), Location::Anywhere, 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}

namespace G1R::Conversation
{
class UTopic_GoreRoleControl : UTopic_Hero__GORE_TEST_A
{
    default ForCharacter = n"GORE_TEST_A";
    default WithCharacter = n"Hero";
    AGothicCharacterState Hero() const { return this.GetCharacter(n"Hero"); }
    AGothicNPCState Subject() const { return FCharacterUniqueName(n"GORE_TEST_B").GetNPCState(); }
}

class UChoiceGoreRoleEnd : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146000;
    default Caption = FText::FromString(n"00 Ende".ToString());
    default PriorityRank = 0;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { this.EndConversation(); }
}
}
