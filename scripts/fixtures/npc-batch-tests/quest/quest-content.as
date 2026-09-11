// Same-module extension for GORE_TEST_A. No dialog starts/completes/fails a quest.
FText GoreBatchQuestText(const FName Text)
{
    FString Value = Text.ToString();
    return FText::FromString(Value);
}

namespace G1R::Document
{
class UDocument_GORE_BATCH_PROVISIONS : UQuestLogDocument
{
}

class UDocumentSegment_GoreProvisionsStart : UDocumentSegment
{
    default InDocument = UDocument_GORE_BATCH_PROVISIONS;
    UFUNCTION()
    void BuildSegment_Implementation(const AGothicCharacterState Reader)
    {
        this.AddParagraph(::GoreBatchQuestText(n"Der Proviantmeister braucht zwei Kaese fuer die Wache. Zuerst soll ich die Lieferung mit ihm absprechen. Fuer die vollstaendige Lieferung verspricht er 25 Erz."));
    }
}

class UDocumentSegment_GoreProvisionsAgreed : UDocumentSegment
{
    default InDocument = UDocument_GORE_BATCH_PROVISIONS;
    UFUNCTION()
    void BuildSegment_Implementation(const AGothicCharacterState Reader)
    {
        this.AddParagraph(::GoreBatchQuestText(n"Die Lieferung ist abgesprochen. Jetzt soll ich dem Proviantmeister genau zwei Kaese geben. Bis zur Uebergabe kann ich den Auftrag ohne Belohnung abbrechen."));
    }
}

class UDocumentSegment_GoreProvisionsComplete : UDocumentSegment
{
    default InDocument = UDocument_GORE_BATCH_PROVISIONS;
    UFUNCTION()
    void BuildSegment_Implementation(const AGothicCharacterState Reader)
    {
        this.AddParagraph(::GoreBatchQuestText(n"Ich habe zwei Kaese abgegeben. Der Proviantmeister hat mir die versprochenen 25 Erz ausgezahlt. Der Auftrag ist abgeschlossen."));
    }
}

class UDocumentSegment_GoreProvisionsFailed : UDocumentSegment
{
    default InDocument = UDocument_GORE_BATCH_PROVISIONS;
    UFUNCTION()
    void BuildSegment_Implementation(const AGothicCharacterState Reader)
    {
        this.AddParagraph(::GoreBatchQuestText(n"Ich habe die Lieferung vor der Uebergabe abgesagt. Der Auftrag ist gescheitert. Ich habe keinen Kaese abgegeben und keine Belohnung erhalten."));
    }
}
}

namespace G1R::Quest
{
UQuest GetGoreProvisions()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr) return nullptr;
    return Subsystem.GetQuestByClass(TSubclassOf<UQuest>(UQuest_GORE_BATCH_PROVISIONS::StaticClass()));
}

UQuest GetGoreProvisionsAgree()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr) return nullptr;
    return Subsystem.GetQuestByClass(TSubclassOf<UQuest>(UQuest_GORE_BATCH_PROVISIONS_AGREE::StaticClass()));
}

UQuest GetGoreProvisionsDeliver()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr) return nullptr;
    return Subsystem.GetQuestByClass(TSubclassOf<UQuest>(UQuest_GORE_BATCH_PROVISIONS_DELIVER::StaticClass()));
}

class UQuest_GORE_BATCH_PROVISIONS : UG1RQuest
{
    default ParentQuestClass = TSubclassOf<UQuest>(UQuest_ValleyOfMines::StaticClass());
    default QuestKind = EQuestKind::Side;
    default InvolvedCharacters.Add(n"Hero");
    default InvolvedCharacters.Add(n"GORE_TEST_A");
    default QuestGiverCharacterUniqueName = n"GORE_TEST_A";
    default NameText = ::GoreBatchQuestText(n"Proviant fuer die Wache");
    default DescriptionText = ::GoreBatchQuestText(n"Sprich die Lieferung ab und bringe dem Proviantmeister zwei Kaese. Belohnung: 25 Erz.");
    default SetQuestlogDocumentClass(G1R::Document::UDocument_GORE_BATCH_PROVISIONS);
    default bExternalAvailabilityTrigger = false;

    UFUNCTION()
    bool ShouldBeAvailable_Implementation()
    {
        return this.GetCharacter(n"Hero") != nullptr;
    }
    UFUNCTION()
    bool ShouldStart_Implementation()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr && Hero.Remembers(n"gore_batch_quest_accepted");
    }
    UFUNCTION()
    bool ShouldSucceed_Implementation()
    {
        UQuest Deliver = GetGoreProvisionsDeliver();
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Deliver != nullptr && Deliver.HasSucceeded() && Hero != nullptr
            && !Hero.Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    bool ShouldFail_Implementation()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr && Hero.Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void HandleQuestStarted_Implementation()
    {
        ::UnlockDocumentSegment(this.GetCharacter(n"Hero"), G1R::Document::UDocument_GORE_BATCH_PROVISIONS,
            G1R::Document::UDocumentSegment_GoreProvisionsStart);
        UQuest Agree = GetGoreProvisionsAgree();
        if (Agree != nullptr && !Agree.HasBeenStarted()) Agree.StartQuest(nullptr);
    }
    UFUNCTION()
    void HandleQuestSucceeded_Implementation()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        if (Hero == nullptr) return;
        // This callback is the only reward site. No latent call between grant and marker.
        if (!Hero.Remembers(n"gore_batch_quest_rewarded"))
        {
            ::AddItemToInventory(Hero, UItMi_Orenugget, 25, EInventoryTypes(1));
            Hero.Remember(n"gore_batch_quest_rewarded");
        }
        ::UnlockDocumentSegment(Hero, G1R::Document::UDocument_GORE_BATCH_PROVISIONS,
            G1R::Document::UDocumentSegment_GoreProvisionsComplete);
    }
    UFUNCTION()
    void HandleQuestFailed_Implementation()
    {
        UQuest Agree = GetGoreProvisionsAgree();
        UQuest Deliver = GetGoreProvisionsDeliver();
        if (Agree != nullptr && Agree.IsRunning()) Agree.FailQuest(nullptr);
        if (Deliver != nullptr && Deliver.IsRunning()) Deliver.FailQuest(nullptr);
        ::UnlockDocumentSegment(this.GetCharacter(n"Hero"), G1R::Document::UDocument_GORE_BATCH_PROVISIONS,
            G1R::Document::UDocumentSegment_GoreProvisionsFailed);
    }
}

class UQuest_GORE_BATCH_PROVISIONS_AGREE : UG1RQuest
{
    default ParentQuestClass = TSubclassOf<UQuest>(UQuest_GORE_BATCH_PROVISIONS::StaticClass());
    default QuestKind = EQuestKind::Subobjective;
    default InvolvedCharacters.Add(n"Hero");
    default NameText = ::GoreBatchQuestText(n"Sprich die Lieferung mit dem Proviantmeister ab");
    default bExternalStartTrigger = true;
    default bExternalFailTrigger = true;

    UFUNCTION()
    bool ShouldSucceed_Implementation()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr && Hero.Remembers(n"gore_batch_quest_agreed")
            && !Hero.Remembers(n"gore_batch_quest_cancelled");
    }
}

class UQuest_GORE_BATCH_PROVISIONS_DELIVER : UG1RQuest
{
    default ParentQuestClass = TSubclassOf<UQuest>(UQuest_GORE_BATCH_PROVISIONS::StaticClass());
    default QuestKind = EQuestKind::Subobjective;
    default InvolvedCharacters.Add(n"Hero");
    default NameText = ::GoreBatchQuestText(n"Gib dem Proviantmeister zwei Kaese");
    default bExternalFailTrigger = true;

    UFUNCTION()
    bool ShouldStart_Implementation()
    {
        UQuest Root = GetGoreProvisions();
        UQuest Agree = GetGoreProvisionsAgree();
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Root != nullptr && Root.IsRunning() && Agree != nullptr && Agree.HasSucceeded()
            && Hero != nullptr && !Hero.Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    bool ShouldSucceed_Implementation()
    {
        AGothicCharacterState Hero = this.GetCharacter(n"Hero");
        return Hero != nullptr && Hero.Remembers(n"gore_batch_quest_handed_in")
            && !Hero.Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void HandleQuestStarted_Implementation()
    {
        ::UnlockDocumentSegment(this.GetCharacter(n"Hero"), G1R::Document::UDocument_GORE_BATCH_PROVISIONS,
            G1R::Document::UDocumentSegment_GoreProvisionsAgreed);
    }
}
}

namespace G1R::Conversation
{
class UChoiceGoreBatchQuestAccept : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147001;
    default Caption = ::GoreBatchQuestText(n"Q1 Auftrag: Proviant fuer die Wache");
    default PriorityRank = 30;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Root = G1R::Quest::GetGoreProvisions();
        return Root != nullptr && !Root.HasBeenStarted() && this.GetHero() != nullptr
            && !this.GetHero().Remembers(n"gore_batch_quest_accepted");
    }
    UFUNCTION()
    void Act_Implementation()
    {
        if (this.IsVisible_Implementation()) this.GetHero().Remember(n"gore_batch_quest_accepted");
        this.EndConversation();
    }
}

class UChoiceGoreBatchQuestAgree : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147002;
    default Caption = ::GoreBatchQuestText(n"Q2 Abgemacht: zwei Kaese fuer 25 Erz");
    default PriorityRank = 29;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Agree = G1R::Quest::GetGoreProvisionsAgree();
        return Agree != nullptr && Agree.IsRunning() && this.GetHero() != nullptr
            && !this.GetHero().Remembers(n"gore_batch_quest_agreed")
            && !this.GetHero().Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void Act_Implementation()
    {
        if (this.IsVisible_Implementation()) this.GetHero().Remember(n"gore_batch_quest_agreed");
        this.EndConversation();
    }
}

class UChoiceGoreBatchQuestSupply : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147003;
    default Caption = ::GoreBatchQuestText(n"Q3 Testvorrat: zwei Kaese nehmen (einmalig)");
    default PriorityRank = 28;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Deliver = G1R::Quest::GetGoreProvisionsDeliver();
        return Deliver != nullptr && Deliver.IsRunning() && this.GetHero() != nullptr
            && !this.GetHero().Remembers(n"gore_batch_quest_supplied")
            && !this.GetHero().Remembers(n"gore_batch_quest_handed_in")
            && !this.GetHero().Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void Act_Implementation()
    {
        if (this.IsVisible_Implementation())
        {
            ::AddItemToInventory(this.GetHero(), UItFo_Cheese, 2, EInventoryTypes(1));
            this.GetHero().Remember(n"gore_batch_quest_supplied");
        }
        this.EndConversation();
    }
}

class UChoiceGoreBatchQuestHandIn : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147004;
    default Caption = ::GoreBatchQuestText(n"Q4 Hier sind die zwei Kaese");
    default PriorityRank = 27;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Deliver = G1R::Quest::GetGoreProvisionsDeliver();
        return Deliver != nullptr && Deliver.IsRunning() && this.GetHero() != nullptr && this.GetSelf() != nullptr
            && ::HasItem(this.GetHero(), UItFo_Cheese, 2)
            && !this.GetHero().Remembers(n"gore_batch_quest_handed_in")
            && !this.GetHero().Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void Act_Implementation()
    {
        if (this.IsVisible_Implementation())
        {
            ::RemoveItemFromInventory(this.GetHero(), UItFo_Cheese, 2);
            ::AddItemToInventory(this.GetSelf(), UItFo_Cheese, 2, EInventoryTypes(1));
            this.GetHero().Remember(n"gore_batch_quest_handed_in");
        }
        this.EndConversation();
    }
}

class UChoiceGoreBatchQuestCancel : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147005;
    default Caption = ::GoreBatchQuestText(n"Q5 Ich sage die Lieferung ab (Auftrag scheitert)");
    default PriorityRank = 26;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Root = G1R::Quest::GetGoreProvisions();
        return Root != nullptr && Root.IsRunning() && this.GetHero() != nullptr
            && !this.GetHero().Remembers(n"gore_batch_quest_handed_in")
            && !this.GetHero().Remembers(n"gore_batch_quest_cancelled");
    }
    UFUNCTION()
    void Act_Implementation()
    {
        if (this.IsVisible_Implementation()) this.GetHero().Remember(n"gore_batch_quest_cancelled");
        this.EndConversation();
    }
}

class UChoiceGoreBatchQuestDone : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147006;
    default Caption = ::GoreBatchQuestText(n"Q6 Die Lieferung ist bezahlt. Danke.");
    default PriorityRank = 25;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Root = G1R::Quest::GetGoreProvisions();
        return Root != nullptr && Root.HasSucceeded();
    }
    UFUNCTION()
    void Act_Implementation() { this.EndConversation(); }
}

class UChoiceGoreBatchQuestFailed : UTopic_Hero__GORE_TEST_A
{
    default DebugId = 3409845526797147007;
    default Caption = ::GoreBatchQuestText(n"Q7 Die Lieferung bleibt abgesagt.");
    default PriorityRank = 24;
    UFUNCTION()
    bool IsVisible_Implementation() const
    {
        UQuest Root = G1R::Quest::GetGoreProvisions();
        return Root != nullptr && Root.HasFailed();
    }
    UFUNCTION()
    void Act_Implementation() { this.EndConversation(); }
}
}
