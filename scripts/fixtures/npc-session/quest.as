// Append to the checked GORE_TEST_A conversation module for the session proof.
namespace G1R::Quest
{
FText GoreNpcSessionText(const FName Text)
{
    FString Value = Text.ToString();
    return FText::FromString(Value);
}

class UQuest_GORE_NPC_SESSION : UG1RQuest
{
    default ParentQuestClass = TSubclassOf<UQuest>(G1R::Quest::UQuest_ValleyOfMines::StaticClass());
    default QuestKind = EQuestKind::Side;
    default InvolvedCharacters.Add(n"Hero");
    default InvolvedCharacters.Add(n"GORE_TEST_A");
    default QuestGiverCharacterUniqueName = n"GORE_TEST_A";
    default NameText = GoreNpcSessionText(n"Erinnere dich an mich");
    default DescriptionText = GoreNpcSessionText(n"Sprich nach einer Pause erneut mit dem Fremden bei Xardas' Turm.");
    default bExternalStartTrigger = true;
    default bExternalSuccessTrigger = true;
}

UQuest GetGoreNpcSession()
{
    return UQuestSubsystem::Get().GetQuestByClass(TSubclassOf<UQuest>(UQuest_GORE_NPC_SESSION::StaticClass()));
}
}
