FText GoreDiegoSmokeCaption(const FName Text)
{
    return FText::FromString(Text.ToString());
}

class UChoiceGoreDiegoSmoke : UTopic_Hero__OC_STT_DIEGO
{
    default Caption = GoreDiegoSmokeCaption(n"[GORE test] Dialog works");
    default PriorityRank = 2;

    UFUNCTION()
    bool IsVisible_Implementation()
    {
        return true;
    }

    UFUNCTION()
    void Act_Implementation()
    {
        this.EndConversation();
    }
}
