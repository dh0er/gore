// Append to the checked GORE_TEST_A conversation; retain its NPC and quest declarations.
namespace G1R::Conversation
{
class UChoiceGoreNpcVoiceOriginal : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797144101;
    default Caption = FText::FromString(n"01 Original: NPC und Held".ToString());
    default PriorityRank = 29;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

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
    bool IsVisible() const
    {
        return true;
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        ::Say(this.GetSelf().GetAI(), GameplayTag::Sound_Voice_Conversation_DailyRoutine_Mumble, EPerceptionNoiseLoudness(2), nullptr, false, FGameplayTag::Empty);
        ::Say(this.GetSelf().GetAI(), GameplayTag::Sound_Voice_Conversation_DailyRoutine_Mumble, EPerceptionNoiseLoudness(2), nullptr, false, FGameplayTag::Empty);
        this.EndConversation();
    }
}

}
