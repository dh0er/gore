// A later native trader batch, separate from the tested OnWorldStart stock.
// This fragment contains menu additions; prepare.py adds the config defaults.
namespace G1R::Conversation
{
class UChoiceGoreRestockDeliver : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146101;
    default Caption = FText::FromString(n"02 Nachschub: +2 Kaese, +5 Pfeile, +20 Erz".ToString());
    default PriorityRank = 58;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreRoleRead(this.GetSelf(), n"gore_restock_trial_v1") == 0.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (this.GetSelf() == nullptr || UWorldPointManager::Get() == nullptr
            || GoreRoleRead(this.GetSelf(), n"gore_restock_trial_v1") != 0.0f) return;
        UWorldPointManager::Get().CallGlobalEvent(n"GoreNpcRestockTrialV1");
        GoreRoleNote(this.GetSelf(), n"gore_restock_trial_v1", 1.0f);
        GoreRoleNote(this.GetSelf(), n"gore_restock_event_calls_v1", 1.0f);
        this.EndConversation();
    }
}

class UChoiceGoreRestockRepeat : UTopic_GoreRoleControl
{
    default DebugId = 3409845526797146102;
    default Caption = FText::FromString(n"03 Test: dieselbe Lieferung erneut melden".ToString());
    default PriorityRank = 57;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return GoreRoleRead(this.GetSelf(), n"gore_restock_trial_v1") == 1.0f; }
    UFUNCTION(BlueprintOverride)
    void Act()
    {
        if (this.GetSelf() == nullptr || UWorldPointManager::Get() == nullptr
            || GoreRoleRead(this.GetSelf(), n"gore_restock_trial_v1") != 1.0f) return;
        // Deliberately bypass only our once guard to test the native event ledger.
        // A second dispatch must not count as a second delivery.
        UWorldPointManager::Get().CallGlobalEvent(n"GoreNpcRestockTrialV1");
        GoreRoleNote(this.GetSelf(), n"gore_restock_event_calls_v1",
            GoreRoleRead(this.GetSelf(), n"gore_restock_event_calls_v1") + 1.0f);
        this.EndConversation();
    }
}
}
