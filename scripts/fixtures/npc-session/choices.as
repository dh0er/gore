// Replace the generated first choice; retain its root and NPC declarations.
namespace G1R::Conversation
{
class UChoiceGoreNpcSessionBegin : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143944;
    default Caption = FText::FromString(n"Auftrag: Erinnere dich an mich".ToString());
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        return !this.GetSelf().Remembers(n"gore_npc_session_started");
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UQuest Quest = G1R::Quest::GetGoreNpcSession();
        AGothicCharacterState Npc = this.GetSelf();
        AGothicCharacterState Hero = this.GetHero();
        if (Quest != nullptr && Npc != nullptr && Hero != nullptr && !Npc.Remembers(n"gore_npc_session_started"))
        {
            Quest.StartQuest(nullptr);
            // Story has a class default Weight=1000, retained when reloaded.
            Npc.GetRelationship().AddActiveModifier(UActivePersonalRelationshipModifier_Story(Hero, ERelationship(5)));
            Npc.Remember(n"gore_npc_session_started");
        }
        this.EndConversation();
    }
}

class UChoiceGoreNpcSessionComplete : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143945;
    default Caption = FText::FromString(n"Ich bin wieder da. Du erinnerst dich an mich.".ToString());
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        return this.GetSelf().Remembers(n"gore_npc_session_started") && !this.GetSelf().Remembers(n"gore_npc_session_completed");
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        UQuest Quest = G1R::Quest::GetGoreNpcSession();
        AGothicCharacterState Npc = this.GetSelf();
        if (Quest != nullptr && Npc != nullptr && Npc.Remembers(n"gore_npc_session_started") && !Npc.Remembers(n"gore_npc_session_completed"))
        {
            Quest.SucceedQuest(nullptr);
            Npc.Remember(n"gore_npc_session_completed");
        }
        this.EndConversation();
    }
}

class UChoiceGoreNpcSessionRemembered : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797143946;
    default Caption = FText::FromString(n"Gut, dass wir Freunde geblieben sind.".ToString());
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        return this.GetSelf().Remembers(n"gore_npc_session_completed");
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        this.EndConversation();
    }
}
}
